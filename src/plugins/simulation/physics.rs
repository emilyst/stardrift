use crate::config::SimulationConfig;
use crate::physics::integrators::StepState;
use crate::physics::math::{Scalar, Vector};
use crate::physics::{
    components::{Mass, PhysicsBody, PhysicsBodyBundle, Position, Velocity},
    octree::OctreeBody,
    resources::{CurrentIntegrator, PhysicsTime},
};
use crate::resources::{
    Barycenter, GravitationalConstant, GravitationalOctree, RenderingRng, SharedRng,
};
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::Mesh3d;
use bevy::prelude::*;
use bevy::tasks::{ComputeTaskPool, ParallelSliceMut};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PhysicsSet {
    IntegrateMotions,
    CorrectBarycentricDrift,
    SyncTransforms,
}

/// Below this body count the acceleration pass runs sequentially: task-pool
/// dispatch costs more than the work at small N, and small worlds (including
/// the physics tests) then never touch the compute task pool at all.
const PARALLEL_ACCEL_THRESHOLD: usize = 128;

/// Driver-private buffers reused across steps.
#[derive(Default)]
pub struct StepBuffers {
    entities: Vec<Entity>,
    masses: Vec<Scalar>,
    states: Vec<StepState>,
    /// Per-body integrator scratch, `scratch_len` slots each, flat SoA layout
    scratch: Vec<Vector>,
    /// Stage snapshot: every body at its current stage-query position. This
    /// one buffer is both the octree's build input and the force-evaluation
    /// input, which is what makes the per-stage field globally consistent.
    snapshot: Vec<OctreeBody>,
    accels: Vec<Vector>,
    /// FSAL cache: the final stage's accelerations from the previous step
    /// (see `reuses_final_stage`), plus the conditions it was recorded under.
    fsal_accels: Vec<Vector>,
    fsal_entities: Vec<Entity>,
    fsal_integrator: &'static str,
    fsal_valid: bool,
}

/// Integrate positions and velocities for all bodies, stage-synchronized.
///
/// Each stage of the active integrator runs as a global pass: every body's
/// stage query is gathered, the octree is rebuilt from that consistent
/// snapshot, accelerations are evaluated against it, and only then does any
/// body advance to the next stage. Multi-stage integrators therefore see all
/// bodies at the same intermediate time, which is what preserves their
/// nominal order and (for the splitting methods at theta = 0) symplecticity
/// and momentum conservation. The committed positions and velocities are
/// written back exactly once per step.
pub fn integrate_motions(
    mut query: Query<(Entity, &mut Position, &mut Velocity, &Mass)>,
    integrator: Res<CurrentIntegrator>,
    physics_time: Res<PhysicsTime>,
    mut octree: ResMut<GravitationalOctree>,
    g: Res<GravitationalConstant>,
    mut buffers: Local<StepBuffers>,
) {
    // Early-return before any mutable component access so a paused
    // simulation marks nothing changed.
    if physics_time.is_paused() {
        return;
    }

    let dt = physics_time.dt;
    let g = **g;
    let integrator = &*integrator.0;
    let buffers = &mut *buffers;

    buffers.entities.clear();
    buffers.masses.clear();
    buffers.states.clear();
    for (entity, position, velocity, mass) in query.iter() {
        buffers.entities.push(entity);
        buffers.masses.push(mass.value());
        buffers
            .states
            .push(StepState::new(position.value(), velocity.value()));
    }

    let n = buffers.entities.len();
    if n == 0 {
        return;
    }

    let scratch_len = integrator.scratch_len();
    // Scratch is not zeroed between steps: each step's stage counter restarts
    // at zero, so slots are always written before they are read.
    buffers.scratch.resize(n * scratch_len, Vector::ZERO);
    buffers.snapshot.resize(
        n,
        OctreeBody {
            position: Vector::ZERO,
            mass: 0.0,
            entity: Entity::PLACEHOLDER,
        },
    );
    buffers.accels.resize(n, Vector::ZERO);

    // The FSAL cache is valid only if the previous step committed exactly
    // this entity set with this integrator. Positions may have been uniformly
    // translated in between (barycentric drift correction): accelerations are
    // translation-invariant and the octree build is translation-equivariant
    // (see Octree::build), so the cached values still apply.
    let use_fsal_cache = integrator.reuses_final_stage()
        && buffers.fsal_valid
        && buffers.fsal_integrator == integrator.name()
        && buffers.fsal_entities == buffers.entities;

    let mut first_stage = true;
    loop {
        // All bodies share one integrator and advance in lockstep, so the
        // first body's completion is everyone's completion.
        let mut complete = false;
        for i in 0..n {
            let scratch = &buffers.scratch[i * scratch_len..(i + 1) * scratch_len];
            match integrator.next_query(&buffers.states[i], scratch, dt) {
                Some(stage_query) => {
                    buffers.snapshot[i] = OctreeBody {
                        position: stage_query.position,
                        mass: buffers.masses[i],
                        entity: buffers.entities[i],
                    };
                }
                None => {
                    complete = true;
                    break;
                }
            }
        }
        if complete {
            break;
        }

        if first_stage && use_fsal_cache {
            // First stage reuses the previous step's final-stage
            // accelerations; the queries are at the same configuration.
            buffers.accels.copy_from_slice(&buffers.fsal_accels);
        } else {
            octree.build_from_slice(&buffers.snapshot);
            let octree = &**octree;
            let snapshot = &buffers.snapshot;
            let evaluate = |global_index: usize, accel: &mut Vector| {
                let body = &snapshot[global_index];
                let force =
                    octree.calculate_force_at_position(body.position, body.mass, body.entity, g);
                *accel = force / body.mass;
            };
            if n < PARALLEL_ACCEL_THRESHOLD {
                for (i, accel) in buffers.accels.iter_mut().enumerate() {
                    evaluate(i, accel);
                }
            } else {
                let task_pool = ComputeTaskPool::get();
                let chunk_size = (n / (task_pool.thread_num() * 4)).max(32);
                buffers
                    .accels
                    .par_chunk_map_mut(task_pool, chunk_size, |chunk_index, chunk| {
                        for (offset, accel) in chunk.iter_mut().enumerate() {
                            evaluate(chunk_index * chunk_size + offset, accel);
                        }
                    });
            }
        }
        first_stage = false;

        for i in 0..n {
            let scratch = &mut buffers.scratch[i * scratch_len..(i + 1) * scratch_len];
            integrator.apply_stage(&mut buffers.states[i], scratch, buffers.accels[i], dt);
        }
    }

    if integrator.reuses_final_stage() {
        // `accels` holds the final stage's evaluations at this point.
        buffers.fsal_accels.clear();
        buffers.fsal_accels.extend_from_slice(&buffers.accels);
        buffers.fsal_entities.clear();
        buffers.fsal_entities.extend_from_slice(&buffers.entities);
        buffers.fsal_integrator = integrator.name();
        buffers.fsal_valid = true;
    } else {
        buffers.fsal_valid = false;
    }

    for i in 0..n {
        let scratch = &buffers.scratch[i * scratch_len..(i + 1) * scratch_len];
        let (position, velocity) = integrator.finish(&buffers.states[i], scratch, dt);
        if let Ok((_, mut position_component, mut velocity_component, _)) =
            query.get_mut(buffers.entities[i])
        {
            *position_component.value_mut() = position;
            *velocity_component.value_mut() = velocity;
        }
    }
}

/// Synchronize Transform components from high-precision Position components
pub fn sync_transform_from_position(
    mut query: Query<(&Position, &mut Transform), (With<PhysicsBody>, Changed<Position>)>,
    camera_query: Query<&Transform, (With<Camera>, Without<PhysicsBody>)>,
) {
    let camera_translation = camera_query
        .iter()
        .next()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    for (position, mut transform) in query.iter_mut() {
        let position_as_vec3 = position.value().as_vec3();
        let distance_to_camera = (position_as_vec3 - camera_translation).length();

        if position.needs_transform_update(&transform) || distance_to_camera < 100.0 {
            transform.translation = position_as_vec3;
        }
    }
}

/// Counteract barycentric drift to keep simulation centered
pub fn counteract_barycentric_drift(
    mut bodies: Query<(&mut Position, &Mass)>,
    mut barycenter: ResMut<Barycenter>,
    config: Res<SimulationConfig>,
    physics_time: Res<PhysicsTime>,
) {
    // A physics operation: while paused, positions must not move.
    if physics_time.is_paused() {
        return;
    }

    let (weighted_positions, total_mass): (Vector, Scalar) = bodies
        .iter()
        .map(|(position, mass)| (position.value(), mass.value()))
        .fold((Vector::ZERO, 0.0), |(pos_acc, mass_acc), (pos, mass)| {
            (pos_acc + pos * mass, mass_acc + mass)
        });

    if total_mass.abs() <= Scalar::EPSILON {
        return;
    }

    let updated_barycenter = weighted_positions / total_mass;

    if !updated_barycenter.is_finite() {
        return;
    }

    let Some(previous_barycenter) = **barycenter else {
        **barycenter = Some(updated_barycenter);
        return;
    };

    if !config.physics.barycentric_drift_correction {
        **barycenter = Some(updated_barycenter);
        return;
    }

    let barycentric_drift = updated_barycenter - previous_barycenter;

    if barycentric_drift.length_squared().abs() <= Scalar::EPSILON {
        return;
    }

    // When correcting drift, we move bodies back so the barycenter stays at the previous position
    bodies.par_iter_mut().for_each(|(mut position, _)| {
        *position.value_mut() += -barycentric_drift;
    });

    // After correction, the barycenter remains at the previous position
    // So we don't update the stored barycenter value
}

/// Helper function to spawn bodies with the given parameters
pub fn spawn_bodies(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    physics_rng: &mut ResMut<SharedRng>,
    rendering_rng: &mut ResMut<RenderingRng>,
    body_count: usize,
    config: &SimulationConfig,
) {
    use super::components::factory;
    use crate::config::ColorScheme;
    use crate::utils::color::*;

    let mut pending: Vec<(
        Vector,
        f32,
        Vector,
        Handle<StandardMaterial>,
        Handle<Mesh>,
        f32,
    )> = Vec::with_capacity(body_count);

    for _ in 0..body_count {
        // Use physics RNG for position, radius, and velocity (physics determinism)
        let position = factory::random_position(physics_rng, body_count, config);
        let radius = factory::random_radius(physics_rng, config);
        let velocity = factory::random_velocity(physics_rng, position, config);

        // Use rendering RNG for color generation (visual determinism, independent of physics)
        let color = match config.rendering.color_scheme {
            ColorScheme::BlackBody => {
                let temperature = factory::calculate_temperature(radius, config);
                rgb_for_temp(temperature)
            }
            ColorScheme::Rainbow => random_rainbow_color(rendering_rng),
            // Colorblind-safe palettes
            ColorScheme::DeuteranopiaSafe => deuteranopia_safe_color(rendering_rng),
            ColorScheme::ProtanopiaSafe => protanopia_safe_color(rendering_rng),
            ColorScheme::TritanopiaSafe => tritanopia_safe_color(rendering_rng),
            ColorScheme::HighContrast => high_contrast_color(rendering_rng),
            // Scientific colormaps
            ColorScheme::Viridis => viridis_color(rendering_rng),
            ColorScheme::Plasma => plasma_color(rendering_rng),
            ColorScheme::Inferno => inferno_color(rendering_rng),
            ColorScheme::Turbo => turbo_color(rendering_rng),
            // Aesthetic themes
            ColorScheme::Pastel => pastel_color(rendering_rng),
            ColorScheme::Neon => neon_color(rendering_rng),
            ColorScheme::Monochrome => monochrome_color(rendering_rng),
            ColorScheme::Vaporwave => vaporwave_color(rendering_rng),
            // Pride flag color schemes
            ColorScheme::Bisexual => bisexual_pride_color(rendering_rng),
            ColorScheme::Transgender => transgender_pride_color(rendering_rng),
            ColorScheme::Lesbian => lesbian_pride_color(rendering_rng),
            ColorScheme::Pansexual => pansexual_pride_color(rendering_rng),
            ColorScheme::Nonbinary => nonbinary_pride_color(rendering_rng),
            ColorScheme::Asexual => asexual_pride_color(rendering_rng),
            ColorScheme::Genderfluid => genderfluid_pride_color(rendering_rng),
            ColorScheme::Aromantic => aromantic_pride_color(rendering_rng),
            ColorScheme::Agender => agender_pride_color(rendering_rng),
        };

        // Create material from color (single API path)
        let material = create_emissive_material(
            materials,
            color,
            config.rendering.bloom_intensity,
            config.rendering.saturation_intensity,
        );

        let mesh = factory::create_detailed_mesh(meshes, radius);

        // Mass proportional to volume (r³) with default density
        let density = 1.0; // Default density, could be made configurable
        let mass = density * 4.0 / 3.0 * std::f32::consts::PI * radius.powi(3);

        pending.push((
            Vector::from(position),
            mass,
            Vector::from(velocity),
            material,
            mesh,
            radius,
        ));
    }

    // Boost into the center-of-momentum frame: random spawn velocities leave a
    // nonzero net momentum, which would translate the whole system indefinitely.
    // A single Galilean boost (and recenter) at spawn removes it as an initial
    // condition, so the barycenter starts at the origin with zero velocity.
    let (weighted_pos, momentum, total_mass) = pending.iter().fold(
        (Vector::ZERO, Vector::ZERO, 0.0 as Scalar),
        |(x_acc, p_acc, m_acc), (position, mass, velocity, ..)| {
            let m = *mass as Scalar;
            (x_acc + *position * m, p_acc + *velocity * m, m_acc + m)
        },
    );

    let (barycenter, barycenter_velocity) = if total_mass > Scalar::EPSILON {
        (weighted_pos / total_mass, momentum / total_mass)
    } else {
        (Vector::ZERO, Vector::ZERO)
    };

    for (position, mass, velocity, material, mesh, radius) in pending {
        commands.spawn((
            PhysicsBodyBundle::new(
                position - barycenter,
                mass,
                radius,
                velocity - barycenter_velocity,
            ),
            MeshMaterial3d(material),
            Mesh3d(mesh),
        ));
    }
}

/// Bevy system to spawn simulation bodies at startup
pub fn spawn_simulation_bodies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut physics_rng: ResMut<SharedRng>,
    mut rendering_rng: ResMut<RenderingRng>,
    body_count: Res<crate::resources::BodyCount>,
    config: Res<SimulationConfig>,
) {
    spawn_bodies(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut physics_rng,
        &mut rendering_rng,
        **body_count,
        &config,
    );
}
