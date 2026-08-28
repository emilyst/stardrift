//! Trails plugin - Self-contained plugin pattern
//!
//! Renders fading, tapering ribbons behind moving bodies. Point recording is
//! CPU-side (`trail.rs`); ribbon expansion, camera-facing width, fade, and
//! expiry all run in the vertex shader (`trail.wgsl`), so trail meshes are
//! re-uploaded only when a point is recorded (~the record interval), not
//! every rendered frame. Expired points linger invisibly in the buffers
//! until a coarse (~1 Hz) CPU trim drops them.

mod material;
#[cfg(not(target_arch = "wasm32"))]
mod render;
mod segment;
mod trail;

pub use material::{TrailMaterial, TrailMaterialHandle};
pub use segment::{TrailSegment, TrailSegmentQueue};
pub use trail::{Trail, TrailClock, TrailPoint};

pub use crate::plugins::visualization::TrailsVisualizationSettings;

use crate::physics::components::{BodyColor, PhysicsBody, Radius};
use crate::prelude::*;
use crate::states::AppState;
#[cfg(target_arch = "wasm32")]
use bevy::asset::RenderAssetUsages;
use bevy::asset::embedded_asset;
#[cfg(target_arch = "wasm32")]
use bevy::camera::visibility::NoFrustumCulling;
use bevy::diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic};
#[cfg(target_arch = "wasm32")]
use bevy::mesh::{MeshTag, PrimitiveTopology};
use bevy::pbr::MaterialPlugin;
use material::pack_trail_color;
#[cfg(target_arch = "wasm32")]
use material::{ATTRIBUTE_TRAIL_BIRTH, ATTRIBUTE_TRAIL_OFFSET, ATTRIBUTE_TRAIL_TANGENT};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TrailSet {
    Initialize,
    Update,
    Render,
}

#[derive(Component)]
pub struct TrailRenderer;

#[derive(Component)]
pub struct TrackedBody(pub Entity);

#[cfg(target_arch = "wasm32")]
#[derive(Bundle)]
struct TrailBundle {
    renderer: TrailRenderer,
    tracked: TrackedBody,
    trail: Trail,
    mesh: Mesh3d,
    material: MeshMaterial3d<TrailMaterial>,
    tag: MeshTag,
    transform: Transform,
    visibility: Visibility,
}

pub struct TrailsPlugin;

impl Plugin for TrailsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "trail.wgsl");
        app.add_plugins(MaterialPlugin::<TrailMaterial>::default());
        app.init_resource::<TrailClock>();
        app.init_resource::<TrailMaterialHandle>();
        app.init_resource::<TrailSegmentQueue>();

        // The instanced ring renderer owns trail visuals on native; the
        // legacy per-trail mesh path stays wasm-only until the port.
        #[cfg(not(target_arch = "wasm32"))]
        app.add_plugins(render::TrailRenderPlugin);

        app.register_diagnostic(Diagnostic::new(Self::TRAIL_POINTS).with_smoothing_factor(0.0));
        app.register_diagnostic(Diagnostic::new(Self::TRAIL_SEGMENT_UPLOADS));
        #[cfg(target_arch = "wasm32")]
        app.register_diagnostic(Diagnostic::new(Self::TRAIL_MESH_REBUILDS));

        app.configure_sets(
            Update,
            (TrailSet::Initialize, TrailSet::Update, TrailSet::Render).chain(),
        );
        // Restart despawns and respawns bodies from SimulationSet::Input;
        // this edge (and the sync point Bevy inserts for it) guarantees
        // initialize_trails sees the fresh bodies' Added<PhysicsBody> the
        // same frame instead of relying on incidental schedule topology.
        app.configure_sets(
            Update,
            crate::plugins::simulation::SimulationSet::Input.before(TrailSet::Initialize),
        );

        app.add_systems(
            Update,
            (
                Self::initialize_trails.in_set(TrailSet::Initialize),
                // Explicitly before initialize_trails: the restart wipe must
                // never be able to despawn renderers created for the fresh
                // body set. Today's sync-point topology happens to guarantee
                // this without the edge; the edge makes it structural.
                Self::handle_restart
                    .in_set(TrailSet::Initialize)
                    .before(Self::initialize_trails),
                // The clock must settle before recording so every trail sees
                // one effective time per frame.
                Self::update_trail_clock
                    .in_set(TrailSet::Update)
                    .before(Self::update_trails),
                Self::update_trails.in_set(TrailSet::Update),
                // Explicitly before despawn_orphaned_trails: both touch
                // `Trail` after update_trails and would otherwise be
                // ambiguously ordered, and "orphaned and fully decayed" must
                // be evaluated against this tick's trim.
                Self::trim_expired_points
                    .in_set(TrailSet::Update)
                    .after(Self::update_trails)
                    .before(Self::despawn_orphaned_trails),
                Self::despawn_orphaned_trails
                    .in_set(TrailSet::Update)
                    .after(Self::update_trails),
                Self::record_trail_diagnostics.in_set(TrailSet::Render),
            )
                .run_if(in_state(AppState::Running).or_else(in_state(AppState::Paused))),
        );

        // The legacy renderer's mesh maintenance, wasm-only.
        #[cfg(target_arch = "wasm32")]
        app.add_systems(
            Update,
            (
                // Before rebuild_trail_meshes: `dirty` flags are consumed by
                // the rebuild, so the diagnostics sampling above must not be
                // reordered after it either (both live in TrailSet::Render;
                // the explicit edge keeps the count honest).
                (Self::rebuild_trail_meshes, Self::sync_trail_material)
                    .in_set(TrailSet::Render)
                    .after(Self::record_trail_diagnostics),
            )
                .run_if(in_state(AppState::Running).or_else(in_state(AppState::Paused))),
        );

        // Trail systems run in Update while physics runs in FixedUpdate.
        // This is intentional - Bevy ensures Update runs after any pending
        // FixedUpdate steps, so trails always see the latest physics positions.
    }
}

impl TrailsPlugin {
    /// Total recorded points across all live trails. Trail CPU cost scales
    /// with this, not with body count alone. Includes expired points still
    /// awaiting the coarse trim (up to ~1 s of record rate per trail), so it
    /// measures buffer load, not visible length.
    pub const TRAIL_POINTS: DiagnosticPath = DiagnosticPath::const_new("trails/points");
    /// Segments queued for upload into the ring this frame (nonzero on
    /// record ticks; the whole batch is one `write_buffer`).
    pub const TRAIL_SEGMENT_UPLOADS: DiagnosticPath =
        DiagnosticPath::const_new("trails/segment_uploads");
    /// Trails whose meshes will be rebuilt (re-uploaded) this frame.
    /// Legacy renderer only, hence wasm-only.
    #[cfg(target_arch = "wasm32")]
    pub const TRAIL_MESH_REBUILDS: DiagnosticPath =
        DiagnosticPath::const_new("trails/mesh_rebuilds");

    fn record_trail_diagnostics(
        trails: Query<&Trail, With<TrailRenderer>>,
        segments: Res<TrailSegmentQueue>,
        mut diagnostics: Diagnostics,
    ) {
        let mut points = 0;
        let mut dirty = 0;
        for trail in &trails {
            points += trail.points.len();
            if trail.dirty {
                dirty += 1;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = dirty;
        diagnostics.add_measurement(&Self::TRAIL_POINTS, || points as f64);
        diagnostics.add_measurement(&Self::TRAIL_SEGMENT_UPLOADS, || {
            (segments.fresh.len() + segments.rewrite.len()) as f64
        });
        #[cfg(target_arch = "wasm32")]
        diagnostics.add_measurement(&Self::TRAIL_MESH_REBUILDS, || dirty as f64);
    }

    fn update_trail_clock(
        mut clock: ResMut<TrailClock>,
        app_state: Res<State<AppState>>,
        time: Res<Time>,
    ) {
        let now = time.elapsed_secs();
        match app_state.get() {
            AppState::Paused => clock.pause(now),
            AppState::Running => clock.unpause(now),
        }
    }

    fn update_trails(
        mut trail_query: Query<(&mut Trail, &TrackedBody), With<TrailRenderer>>,
        body_query: Query<(&Transform, Option<&Radius>, Option<&BodyColor>), With<PhysicsBody>>,
        time: Res<Time>,
        config: Res<SimulationConfig>,
        clock: Res<TrailClock>,
        mut segments: ResMut<TrailSegmentQueue>,
    ) {
        // Effective time is frozen while paused: nothing records, nothing
        // expires, and the shader's fade input holds still.
        if clock.is_paused() {
            return;
        }
        let now = clock.effective_time(time.elapsed_secs());

        // One shared record tick for every trail — this is what keeps each
        // tick's batch contiguous in the ring, and it doubles as the moment
        // the previous batch rotates into the rewrite slot to receive its
        // outgoing-tangent fix-ups (mitered joints need the successor
        // direction, which only exists one tick later).
        #[cfg(not(target_arch = "wasm32"))]
        let due = {
            let tick = segments.should_tick(now, config.trails.update_interval_seconds);
            if tick {
                segments.begin_tick();
            }
            tick
        };

        for (mut trail, tracked_body) in trail_query.iter_mut() {
            #[cfg(target_arch = "wasm32")]
            let due = trail.should_update(now, config.trails.update_interval_seconds);

            // Only add new points if we're tracking an active body
            if let Ok((transform, radius, color)) = body_query.get(tracked_body.0)
                && due
            {
                // Radius at record time: collision merges grow the body, and
                // the trail width follows per point from here on.
                let radius = radius.map(|r| r.value() as f32).unwrap_or(1.0);
                let position = transform.translation;

                // Feed the instanced renderer: the segment from the previous
                // sample to this one. Color is sampled at record time, so a
                // body recolored by a merge leaves its old color in older
                // segments. Wasm has no ring renderer draining this queue,
                // so it must not fill it.
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let prev = trail.points.front().map(|p| (p.position, p.birth));
                    let second = trail.points.get(1).map(|p| p.position);
                    if let Some((prev_position, prev_birth)) = prev {
                        // The previous segment's true successor endpoint is
                        // this new sample; both quads at the joint then
                        // project the exact same points and their mitered
                        // edges agree by arithmetic.
                        segments.fix_up_prev(tracked_body.0, position.to_array());
                        // Smooth the record-time speed: raw per-segment
                        // speed jitters (positions quantize to physics
                        // fixed-steps while birth stamps ride frame time),
                        // and the long-exposure energy law turns that
                        // jitter into beading.
                        let dt = (now - prev_birth).max(1e-5);
                        let raw_speed = position.distance(prev_position) / dt;
                        trail.speed_ema = if trail.speed_ema <= 0.0 {
                            raw_speed
                        } else {
                            0.7 * trail.speed_ema + 0.3 * raw_speed
                        };
                        // The point before p0 is the second-newest sample,
                        // or a straight backward extrapolation for a
                        // trail's first segment (an unrotated end edge).
                        let p_prev = second.unwrap_or(prev_position + (prev_position - position));
                        let index = segments.fresh.len();
                        segments.fresh.push(TrailSegment {
                            p0: prev_position.to_array(),
                            p1: position.to_array(),
                            birth0: prev_birth,
                            birth1: now,
                            radius,
                            color: pack_trail_color(color.map(|c| c.0).unwrap_or(Color::WHITE)),
                            p_prev: p_prev.to_array(),
                            // Straight extrapolation; corrected one tick
                            // later via fix_up_prev.
                            p_next: (position + (position - prev_position)).to_array(),
                            speed: trail.speed_ema,
                        });
                        segments.fresh_index.insert(tracked_body.0, index);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                let _ = (&segments, color);

                trail.add_point(position, radius, now, config.trails.max_points_per_trail);
            }
        }
    }

    /// Interval between CPU trims of expired points. Expiry is visually
    /// instant (shader-side); this only bounds how much invisible data
    /// lingers in the buffers, so it can be very coarse.
    const TRIM_INTERVAL_SECONDS: f32 = 1.0;

    /// Drop expired points at a coarse cadence. Doing this per frame marked
    /// every trail dirty ~every frame at steady state, re-uploading every
    /// trail mesh at frame rate — the dominant frame cost at high body
    /// counts. The trim never marks trails dirty: live trails fold removals
    /// into their next record rebuild, and orphaned trails stop uploading
    /// entirely during fade-out. The gate lives in effective time, so pause
    /// freezes it along with the ages it checks.
    fn trim_expired_points(
        mut trail_query: Query<&mut Trail, With<TrailRenderer>>,
        time: Res<Time>,
        config: Res<SimulationConfig>,
        clock: Res<TrailClock>,
        mut last_trim: Local<f32>,
    ) {
        if clock.is_paused() {
            return;
        }
        let now = clock.effective_time(time.elapsed_secs());
        if now - *last_trim < Self::TRIM_INTERVAL_SECONDS {
            return;
        }
        // Unconditionally, not only when something was removed: an idle tick
        // must close the gate too, or it stays open forever.
        *last_trim = now;

        for mut trail in trail_query.iter_mut() {
            trail.trim_expired(now, config.trails.trail_length_seconds);
        }
    }

    /// This system should run after bodies are spawned
    fn initialize_trails(
        mut commands: Commands,
        // Only process newly added bodies - eliminates O(n²) check
        query: Query<(Entity, Option<&BodyColor>), Added<PhysicsBody>>,
        mut meshes: ResMut<Assets<Mesh>>,
        material: Res<TrailMaterialHandle>,
        config: Res<SimulationConfig>,
        trails_visible: Res<TrailsVisualizationSettings>,
    ) {
        for (entity, color) in query.iter() {
            let color = color.map(|c| c.0).unwrap_or(Color::WHITE);
            let trail = Trail::new();

            // Native renders trails through the instanced ring renderer
            // (`render/`): the trail entity is pure recording state, with no
            // render components at all. The legacy per-trail mesh path in
            // the wasm block remains for the WASM build until the ring
            // renderer is ported.
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (&color, &mut meshes, &material, &config, &trails_visible);
                commands.spawn((TrailRenderer, TrackedBody(entity), trail));
            }

            #[cfg(target_arch = "wasm32")]
            {
                // Create the mesh up front (degenerate and invisible until
                // two points exist) so the rebuild path never has to branch
                // on a missing Mesh3d.
                let mut mesh = Mesh::new(
                    PrimitiveTopology::TriangleStrip,
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                );
                // The trail is empty here, so the degenerate branch runs and
                // no taper is evaluated; the timestamp is irrelevant.
                write_trail_mesh(&mut mesh, &trail, &config.trails, 0.0);

                let visibility = if trails_visible.enabled {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };

                commands.spawn((
                    TrailBundle {
                        renderer: TrailRenderer,
                        tracked: TrackedBody(entity),
                        trail,
                        mesh: Mesh3d(meshes.add(mesh)),
                        material: MeshMaterial3d(material.clone()),
                        tag: MeshTag(pack_trail_color(color)),
                        transform: Transform::default(),
                        visibility,
                    },
                    // Deliberate, not a workaround: a CPU-side AABB cannot
                    // describe geometry that is expanded in the vertex
                    // shader, and Bevy's dynamic AABB updating would
                    // otherwise recompute bounds over every trail whenever
                    // the shared material is touched (which is every frame,
                    // for the time uniform). Nothing here needs the bounds:
                    // the scene has no lights or shadow culling, transparent
                    // sorting uses the render mesh's stored center, and the
                    // camera frames the whole system. Shader-side expiry
                    // makes this more load-bearing still: meshes now go
                    // un-mutated for up to a second (or an entire orphan
                    // fade-out), so mutation-driven bounds would also be
                    // badly stale.
                    NoFrustumCulling,
                ));
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    /// Upload changed trail geometry. Gated on `Trail::dirty`, so uploads
    /// track the point-record rate, not the frame rate.
    fn rebuild_trail_meshes(
        mut meshes: ResMut<Assets<Mesh>>,
        mut query: Query<(&mut Trail, &Mesh3d), With<TrailRenderer>>,
        config: Res<SimulationConfig>,
        clock: Res<TrailClock>,
        time: Res<Time>,
    ) {
        let now = clock.effective_time(time.elapsed_secs());
        for (mut trail, mesh_handle) in query.iter_mut() {
            if !trail.dirty {
                continue;
            }
            trail.dirty = false;

            if let Some(mut mesh) = meshes.get_mut(&mesh_handle.0) {
                write_trail_mesh(&mut mesh, &trail, &config.trails, now);
            } else {
                warn!("Trail mesh handle exists but mesh not found in assets!");
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    /// Push the pause-aware clock into the shared material once per frame.
    /// Skipped while the value is unchanged (pause) so the material asset is
    /// not marked modified for nothing.
    fn sync_trail_material(
        mut materials: ResMut<Assets<TrailMaterial>>,
        handle: Res<TrailMaterialHandle>,
        clock: Res<TrailClock>,
        time: Res<Time>,
    ) {
        let effective_time = clock.effective_time(time.elapsed_secs());
        let stale = materials
            .get(&**handle)
            .is_some_and(|m| m.params.effective_time != effective_time);
        if stale && let Some(mut material) = materials.get_mut(&**handle) {
            material.params.effective_time = effective_time;
        }
    }

    /// Despawn trail renderers whose tracked body no longer exists once
    /// their points have fully decayed. Bodies absorbed by collision merges
    /// orphan their trails; update_trails stops feeding them and this
    /// reclaims the renderer entity (and its mesh asset) after the fade-out
    /// completes — the point buffer empties at the trim tick following full
    /// decay, so reclamation lags by up to the trim interval. Query::get on
    /// a despawned entity returns Err via the generation bump, so a recycled
    /// index can never false-match.
    fn despawn_orphaned_trails(
        mut commands: Commands,
        trail_query: Query<(Entity, &TrackedBody, &Trail), With<TrailRenderer>>,
        body_query: Query<(), With<PhysicsBody>>,
    ) {
        for (entity, tracked_body, trail) in trail_query.iter() {
            if body_query.get(tracked_body.0).is_err() && trail.points.is_empty() {
                commands.entity(entity).despawn();
            }
        }
    }

    /// Despawn all trail renderers on restart. Owned by this plugin so the
    /// simulation plugin does not have to reach across the boundary to clean
    /// up trail entities.
    fn handle_restart(
        mut commands_reader: MessageReader<SimulationCommand>,
        mut commands: Commands,
        trail_renderers: Query<Entity, With<TrailRenderer>>,
        mut segments: ResMut<TrailSegmentQueue>,
    ) {
        for command in commands_reader.read() {
            if !matches!(command, SimulationCommand::Restart) {
                continue;
            }
            trail_renderers.iter().for_each(|entity| {
                commands.entity(entity).despawn();
            });
            // Wipe the instanced renderer's ring: drop all staged batches
            // and signal the render world via the generation.
            segments.clear_for_restart();
        }
    }
}

#[cfg(target_arch = "wasm32")]
/// Write the trail's point set into its mesh: two coincident vertices per
/// point whose signed offsets carry side, per-point width, and taper. The
/// vertex shader does the rest.
fn write_trail_mesh(
    mesh: &mut Mesh,
    trail: &Trail,
    config: &crate::config::TrailConfig,
    effective_now: f32,
) {
    let n = trail.points.len();

    if n < 2 {
        // Degenerate 4-vertex strip instead of empty buffers, which WebGL2
        // rejects. Zero offsets make it invisible; every attribute must be
        // present with matching length.
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32; 3]; 4]);
        mesh.insert_attribute(ATTRIBUTE_TRAIL_TANGENT, vec![[0.0f32; 3]; 4]);
        mesh.insert_attribute(ATTRIBUTE_TRAIL_BIRTH, vec![0.0f32; 4]);
        mesh.insert_attribute(ATTRIBUTE_TRAIL_OFFSET, vec![0.0f32; 4]);
        mesh.remove_indices();
        return;
    }

    let mut positions = Vec::with_capacity(n * 2);
    let mut tangents = Vec::with_capacity(n * 2);
    let mut births = Vec::with_capacity(n * 2);
    let mut offsets = Vec::with_capacity(n * 2);

    for point in trail.points.iter() {
        let base_width = if config.width_relative_to_body {
            point.radius * config.body_size_multiplier
        } else {
            config.base_width
        };

        let taper = if config.enable_tapering {
            // Age toward expiry, not index position: a young trail is a
            // uniform-width ribbon and thinning appears only as the tail
            // genuinely ages out (index-based taper compressed the full
            // fat-to-point range into short young trails — the "tadpole").
            // Baked at rebuild time, so it animates at the record rate for
            // live trails and freezes for orphans mid-fade; the foundation
            // renderer will evaluate it in the shader instead.
            let age_ratio = if config.trail_length_seconds > 0.0 {
                ((effective_now - point.birth) / config.trail_length_seconds).clamp(0.0, 1.0)
            } else {
                0.0
            };
            taper_factor(&config.taper_curve, age_ratio, config.min_width_ratio)
        } else {
            1.0
        };

        // A stationary body records a zero tangent; collapse the pair to
        // zero width rather than rendering an arbitrarily oriented quad.
        let half_width = if point.tangent == Vec3::ZERO {
            0.0
        } else {
            0.5 * base_width * taper
        };

        for offset in [-half_width, half_width] {
            positions.push(point.position.to_array());
            tangents.push(point.tangent.to_array());
            births.push(point.birth);
            offsets.push(offset);
        }
    }

    // For triangle strips, no manual indices needed - Bevy will automatically
    // connect consecutive vertices: (0,1,2), (1,2,3), (2,3,4), etc.
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(ATTRIBUTE_TRAIL_TANGENT, tangents);
    mesh.insert_attribute(ATTRIBUTE_TRAIL_BIRTH, births);
    mesh.insert_attribute(ATTRIBUTE_TRAIL_OFFSET, offsets);
    mesh.remove_indices();
}

#[cfg(target_arch = "wasm32")]
fn taper_factor(
    curve: &crate::config::TaperCurve,
    position_ratio: f32,
    min_width_ratio: f32,
) -> f32 {
    match curve {
        crate::config::TaperCurve::Linear => 1.0 - position_ratio * (1.0 - min_width_ratio),
        crate::config::TaperCurve::Exponential => {
            // Exponential tapering (more aggressive at the end)
            let t = 1.0 - position_ratio;
            min_width_ratio + (1.0 - min_width_ratio) * (t * t)
        }
        crate::config::TaperCurve::SmoothStep => {
            // Smooth step tapering
            let t = 1.0 - position_ratio;
            let smooth = 3.0 * t * t - 2.0 * t * t * t;
            min_width_ratio + (1.0 - min_width_ratio) * smooth
        }
    }
}
