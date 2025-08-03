//! Trails plugin - Self-contained plugin pattern
//!
//! This plugin follows the self-contained pattern where all systems, components,
//! and rendering logic are defined within the plugin module. This pattern is ideal
//! for independent, feature-gated functionality that can be completely removed
//! without affecting the core simulation.

use crate::prelude::*;
use crate::states::AppState;
use bevy::render::mesh::{MeshAabb, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::view::NoFrustumCulling;

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

#[derive(Bundle)]
struct TrailBundle {
    renderer: TrailRenderer,
    tracked: TrackedBody,
    trail: Trail,
    material: MeshMaterial3d<StandardMaterial>,
    transform: Transform,
    visibility: Visibility,
}

impl TrailBundle {
    fn new(tracked_body: Entity, trail: Trail, material: Handle<StandardMaterial>) -> Self {
        Self {
            renderer: TrailRenderer,
            tracked: TrackedBody(tracked_body),
            trail,
            material: MeshMaterial3d(material),
            transform: Transform::default(),
            visibility: Visibility::default(),
        }
    }
}

use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct TrailPoint {
    pub position: Vec3,
    pub age: f32,
}

pub struct TrailRenderParams<'a> {
    pub camera_pos: Option<Vec3>,
    pub base_width: f32,
    pub width_relative_to_body: bool,
    pub body_radius: Option<f32>,
    pub body_size_multiplier: f32,
    pub enable_tapering: bool,
    pub taper_curve: &'a crate::config::TaperCurve,
    pub min_width_ratio: f32,
}

#[derive(Component, Debug)]
pub struct Trail {
    pub points: VecDeque<TrailPoint>,
    pub color: Color,
    pub body_radius: f32,
    pub last_update: f32,
    pub pause_time: Option<f32>,
    pub total_pause_duration: f32,
}

impl Trail {
    pub fn new(color: Color, body_radius: f32) -> Self {
        Self {
            points: VecDeque::new(),
            color,
            body_radius,
            last_update: 0.0,
            pause_time: None,
            total_pause_duration: 0.0,
        }
    }

    pub fn add_point(&mut self, position: Vec3, current_time: f32) {
        self.points.push_front(TrailPoint {
            position,
            age: current_time,
        });

        self.last_update = current_time;
    }

    pub fn cleanup_old_points(&mut self, current_time: f32, max_age: f32, max_points: usize) {
        let effective_time = self.effective_time(current_time);

        self.points
            .retain(|point| effective_time - point.age <= max_age);

        // Also enforce max_points limit for performance
        if self.points.len() > max_points {
            self.points.truncate(max_points);
        }

        // Don't update last_update here - that should only happen when adding new points!
    }

    pub fn should_update(&self, current_time: f32, update_interval: f32) -> bool {
        current_time - self.last_update >= update_interval
    }

    pub fn pause(&mut self, current_time: f32) {
        if self.pause_time.is_none() {
            self.pause_time = Some(current_time);
        }
    }

    pub fn unpause(&mut self, current_time: f32) {
        if let Some(pause_start) = self.pause_time {
            self.total_pause_duration += current_time - pause_start;
            self.pause_time = None;
        }
    }

    pub fn is_paused(&self) -> bool {
        self.pause_time.is_some()
    }

    pub fn effective_time(&self, current_time: f32) -> f32 {
        if let Some(pause_start) = self.pause_time {
            pause_start - self.total_pause_duration
        } else {
            current_time - self.total_pause_duration
        }
    }

    pub fn calculate_point_alpha(
        &self,
        point: &TrailPoint,
        current_time: f32,
        fade_config: &crate::config::TrailConfig,
    ) -> f32 {
        if !fade_config.enable_fading {
            return fade_config.max_alpha as f32;
        }

        let effective_time = self.effective_time(current_time);
        let age = effective_time - point.age;
        let max_age = fade_config.trail_length_seconds as f32;

        if age <= 0.0 || max_age <= 0.0 {
            return fade_config.max_alpha as f32;
        }

        let fade_ratio = (age / max_age).clamp(0.0, 1.0);

        let curved_fade = match fade_config.fade_curve {
            crate::config::FadeCurve::Linear => fade_ratio,
            crate::config::FadeCurve::Exponential => fade_ratio * fade_ratio,
            crate::config::FadeCurve::SmoothStep => {
                // Smooth step function: 3t² - 2t³
                let t = fade_ratio;
                3.0 * t * t - 2.0 * t * t * t
            }
            crate::config::FadeCurve::EaseInOut => {
                // Ease in-out using cosine interpolation
                let t = fade_ratio;
                0.5 * (1.0 - (t * std::f32::consts::PI).cos())
            }
        };

        let min_alpha = fade_config.min_alpha as f32;
        let max_alpha = fade_config.max_alpha as f32;

        max_alpha - curved_fade * (max_alpha - min_alpha)
    }

    /// Each trail vertex becomes two vertices forming a strip with configurable width
    pub fn get_triangle_strip_vertices(&self, params: &TrailRenderParams) -> Vec<Vec3> {
        if self.points.len() < 2 {
            return Vec::new();
        }

        let mut vertices = Vec::with_capacity(self.points.len() * 2);

        for i in 0..self.points.len() {
            let current_pos = self.points[i].position;

            // Calculate direction consistently along the trail
            let direction = if i + 1 < self.points.len() {
                // Use direction to next point
                (self.points[i + 1].position - current_pos).normalize_or_zero()
            } else if i > 0 {
                // For last point, use same direction as previous segment
                (current_pos - self.points[i - 1].position).normalize_or_zero()
            } else {
                // Single point fallback
                Vec3::X
            };

            let mut perpendicular = if let Some(cam_pos) = params.camera_pos {
                // Camera-facing width: cross product gives us width perpendicular to both
                // trail direction and camera-to-point vector
                let to_camera = (cam_pos - current_pos).normalize_or_zero();
                let cross = direction.cross(to_camera).normalize_or_zero();

                if cross.length_squared() > 0.001 {
                    cross
                } else {
                    // Camera aligned with trail, use world-up fallback
                    direction.cross(Vec3::Y).normalize_or_zero()
                }
            } else {
                // Fallback to world-up perpendicular
                direction.cross(Vec3::Y).normalize_or_zero()
            };

            if perpendicular == Vec3::ZERO {
                // Final fallback if direction is vertical
                perpendicular = Vec3::X;
            }

            // Calculate trail width based on configuration
            let base_trail_width = if params.width_relative_to_body {
                params.body_radius.unwrap_or(1.0) * params.body_size_multiplier
            } else {
                params.base_width
            };

            // Apply tapering if enabled
            let width = if params.enable_tapering {
                // Calculate position along trail (0.0 at head, 1.0 at tail)
                let position_ratio = i as f32 / (self.points.len() - 1) as f32;

                // Apply taper curve
                let taper_factor = match params.taper_curve {
                    crate::config::TaperCurve::Linear => {
                        1.0 - position_ratio * (1.0 - params.min_width_ratio)
                    }
                    crate::config::TaperCurve::Exponential => {
                        // Exponential tapering (more aggressive at the end)
                        let t = 1.0 - position_ratio;
                        params.min_width_ratio + (1.0 - params.min_width_ratio) * (t * t)
                    }
                    crate::config::TaperCurve::SmoothStep => {
                        // Smooth step tapering
                        let t = 1.0 - position_ratio;
                        let smooth = 3.0 * t * t - 2.0 * t * t * t;
                        params.min_width_ratio + (1.0 - params.min_width_ratio) * smooth
                    }
                };

                base_trail_width * taper_factor
            } else {
                base_trail_width
            };

            let half_width = width / 2.0;
            let left = current_pos - perpendicular * half_width;
            let right = current_pos + perpendicular * half_width;

            vertices.push(left);
            vertices.push(right);
        }

        vertices
    }
}

pub struct TrailsPlugin;

impl Plugin for TrailsPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (TrailSet::Initialize, TrailSet::Update, TrailSet::Render).chain(),
        );

        app.add_systems(
            Update,
            (
                Self::initialize_trails.in_set(TrailSet::Initialize),
                Self::update_trails.in_set(TrailSet::Update),
                Self::render_trails.in_set(TrailSet::Render),
            )
                .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
        );

        // Trail systems run in Update while physics runs in FixedUpdate.
        // This is intentional - Bevy ensures Update runs after any pending
        // FixedUpdate steps, so trails always see the latest physics positions.
    }
}

impl TrailsPlugin {
    fn update_trails(
        mut trail_query: Query<(&mut Trail, &TrackedBody), With<TrailRenderer>>,
        body_query: Query<&Transform, With<RigidBody>>,
        time: Res<Time>,
        config: Res<SimulationConfig>,
        app_state: Res<State<AppState>>,
    ) {
        let current_time = time.elapsed_secs();
        let is_paused = matches!(app_state.get(), AppState::Paused);

        for (mut trail, tracked_body) in trail_query.iter_mut() {
            if is_paused {
                trail.pause(current_time);
                continue;
            }

            if trail.is_paused() {
                trail.unpause(current_time);
            }

            // Only add new points if we're tracking an active body
            if let Ok(transform) = body_query.get(tracked_body.0) {
                if trail.should_update(current_time, config.trails.update_interval_seconds as f32) {
                    trail.add_point(transform.translation, current_time);
                }
            }

            // Always cleanup old points, even for orphaned trails
            trail.cleanup_old_points(
                current_time,
                config.trails.trail_length_seconds as f32,
                config.trails.max_points_per_trail,
            );
        }
    }

    /// This system should run after bodies are spawned
    #[allow(clippy::type_complexity)]
    fn initialize_trails(
        mut commands: Commands,
        // Only process newly added bodies - eliminates O(n²) check
        query: Query<
            (Entity, &MeshMaterial3d<StandardMaterial>, Option<&Collider>),
            Added<RigidBody>,
        >,
        mut materials: ResMut<Assets<StandardMaterial>>,
        app_state: Res<State<AppState>>,
        time: Res<Time>,
        config: Res<SimulationConfig>,
    ) {
        let is_paused = matches!(app_state.get(), AppState::Paused);
        let current_time = time.elapsed_secs();

        for (entity, mesh_material, collider) in query.iter() {
            // Extract color from the body's material
            let color = if let Some(material) = materials.get(&mesh_material.0) {
                material.base_color
            } else {
                Color::WHITE
            };

            // Extract radius from collider if available
            let body_radius = collider
                .and_then(|c| {
                    let shape = c.shape();
                    shape.as_ball().map(|ball| ball.radius as f32)
                })
                .unwrap_or(1.0);

            let mut trail = Trail::new(color, body_radius);

            if is_paused {
                trail.pause(current_time);
            }

            // Choose blending mode based on configuration
            // Additive: Strong bloom but no transparency
            // Premultiplied: True transparency with bloom compensation
            let alpha_mode = if config.trails.use_additive_blending {
                AlphaMode::Add
            } else {
                AlphaMode::Premultiplied
            };

            let trail_material = materials.add(StandardMaterial {
                base_color: color,
                unlit: true,
                alpha_mode,
                emissive: color.into(), // Add emissive for bloom effect
                ..default()
            });

            commands.spawn((
                TrailBundle::new(entity, trail, trail_material),
                // TEMPORARY: Disable frustum culling for trails
                // This prevents trails from being culled incorrectly while we debug bounding volume issues
                // Frustum culling is still having problems with dynamic trail geometry despite AABB computation
                // TODO: Remove this once we solve the underlying bounding volume calculation
                NoFrustumCulling,
            ));
        }
    }

    fn render_trails(
        mut commands: Commands,
        mut trail_meshes: ResMut<Assets<Mesh>>,
        mut renderer_query: Query<(Entity, &Trail, Option<&Mesh3d>), With<TrailRenderer>>,
        camera_query: Query<&Transform, With<Camera>>,
        time: Res<Time>,
        config: Res<SimulationConfig>,
    ) {
        // Get camera position for camera-facing trails
        let camera_pos = camera_query
            .single()
            .ok()
            .map(|transform| transform.translation);

        let current_time = time.elapsed_secs();

        for (renderer_entity, trail, mesh_handle) in renderer_query.iter_mut() {
            if trail.points.len() >= 2 {
                let body_radius = Some(trail.body_radius);

                match mesh_handle {
                    Some(mesh_handle) => {
                        // Update existing mesh
                        if let Some(mesh) = trail_meshes.get_mut(&mesh_handle.0) {
                            Self::update_trail_mesh(
                                mesh,
                                trail,
                                camera_pos,
                                current_time,
                                &config.trails,
                                Some(trail.body_radius),
                            );
                        } else {
                            warn!("Trail mesh handle exists but mesh not found in assets!");
                        }
                    }
                    None => {
                        // Create new mesh and add it to the entity
                        let trail_mesh = Self::create_trail_mesh_with_data(
                            &mut trail_meshes,
                            trail,
                            camera_pos,
                            current_time,
                            &config.trails,
                            body_radius,
                        );
                        commands.entity(renderer_entity).insert(Mesh3d(trail_mesh));
                    }
                }
            }
        }
    }

    fn create_trail_mesh_with_data(
        meshes: &mut Assets<Mesh>,
        trail: &Trail,
        camera_pos: Option<Vec3>,
        current_time: f32,
        trail_config: &crate::config::TrailConfig,
        body_radius: Option<f32>,
    ) -> Handle<Mesh> {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleStrip,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        Self::update_trail_mesh(
            &mut mesh,
            trail,
            camera_pos,
            current_time,
            trail_config,
            body_radius,
        );
        meshes.add(mesh)
    }

    fn update_trail_mesh(
        mesh: &mut Mesh,
        trail: &Trail,
        camera_pos: Option<Vec3>,
        current_time: f32,
        trail_config: &crate::config::TrailConfig,
        body_radius: Option<f32>,
    ) {
        let params = TrailRenderParams {
            camera_pos,
            base_width: trail_config.base_width as f32,
            width_relative_to_body: trail_config.width_relative_to_body,
            body_radius,
            body_size_multiplier: trail_config.body_size_multiplier as f32,
            enable_tapering: trail_config.enable_tapering,
            taper_curve: &trail_config.taper_curve,
            min_width_ratio: trail_config.min_width_ratio as f32,
        };
        let strip_vertices = trail.get_triangle_strip_vertices(&params);

        if strip_vertices.is_empty() {
            // Create a minimal degenerate triangle strip to avoid empty buffer issues in WebGL
            // This creates 4 vertices all at the origin with zero alpha (invisible)
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0, 0.0, 0.0]; 4]);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4]);
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.0, 0.0, 0.0, 0.0]; 4]);
            mesh.remove_indices();

            // Still compute bounds to prevent culling issues
            if mesh.compute_aabb().is_none() {
                warn!("Failed to compute AABB for empty trail mesh");
            }
            return;
        }

        let vertex_count = strip_vertices.len();

        // Pre-allocate vectors with exact capacity to avoid reallocations
        let mut positions = Vec::with_capacity(vertex_count);
        let mut normals = Vec::with_capacity(vertex_count);
        let mut colors = Vec::with_capacity(vertex_count);

        // Get base color once for efficiency
        let base_color = trail.color.to_srgba();
        let bloom_intensity = trail_config.bloom_factor;

        for (i, vertex) in strip_vertices.iter().enumerate() {
            positions.push([vertex.x, vertex.y, vertex.z]);
            normals.push([0.0, 1.0, 0.0]);

            // Each trail point generates 2 vertices (left and right side of strip)
            // Note: VecDeque has newest points at index 0, so point_index matches directly
            let point_index = i / 2; // Convert vertex index to point index

            if point_index < trail.points.len() {
                let point = &trail.points[point_index];
                let alpha = trail.calculate_point_alpha(point, current_time, trail_config);

                // Apply bloom intensification using luminance-based scaling
                // This matches how celestial bodies create their bloom effect
                let r = base_color.red;
                let g = base_color.green;
                let b = base_color.blue;

                // Calculate luminance using ITU-R BT.601 formula
                let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
                let scale_factor = bloom_intensity as f32 * luminance + 1.0;

                // For premultiplied alpha, we need to handle colors differently
                // The bloom buffer sees: color * alpha, so we boost the color to compensate
                // This allows both transparency AND bloom effect
                let bloom_compensation = if !trail_config.use_additive_blending && alpha > 0.01 {
                    // Only compensate for premultiplied mode
                    // At alpha=1.0, no boost needed. At alpha=0.1, 10x boost
                    1.0 / alpha.sqrt()
                } else {
                    1.0 // No compensation for additive mode
                };

                // Apply the scaling to create HDR colors for bloom
                let bloomed_r = r * scale_factor * bloom_compensation;
                let bloomed_g = g * scale_factor * bloom_compensation;
                let bloomed_b = b * scale_factor * bloom_compensation;

                colors.push([bloomed_r, bloomed_g, bloomed_b, alpha]);
            } else {
                // Fallback for edge cases
                colors.push([1.0, 1.0, 1.0, 1.0]);
            }
        }

        // For triangle strips, no manual indices needed - Bevy will automatically
        // connect consecutive vertices: (0,1,2), (1,2,3), (2,3,4), etc.
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh.remove_indices();

        // IMPORTANT: Compute bounds to prevent incorrect frustum culling
        // Without proper bounds, trails may be culled too aggressively
        if mesh.compute_aabb().is_none() {
            warn!("Failed to compute AABB for trail mesh");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::AssetPlugin;

    #[test]
    fn test_trail_mesh_creation_with_data() {
        // Test that we can create a mesh with trail data
        let mut app = App::new();
        app.add_plugins(AssetPlugin::default());
        app.init_resource::<Assets<Mesh>>();

        let mut trail = Trail::new(Color::WHITE, 1.0);
        trail.add_point(Vec3::new(0.0, 0.0, 0.0), 0.0);
        trail.add_point(Vec3::new(1.0, 0.0, 0.0), 1.0);

        let trail_config = crate::config::TrailConfig::default();
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        let handle = TrailsPlugin::create_trail_mesh_with_data(
            &mut meshes,
            &trail,
            None,
            1.0,
            &trail_config,
            Some(1.0),
        );

        // Should have a valid handle
        assert!(meshes.get(&handle).is_some());
    }

    #[test]
    fn test_update_trail_mesh_empty() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleStrip,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        let trail = Trail::new(Color::WHITE, 1.0);
        let trail_config = crate::config::TrailConfig::default();

        // Empty trail should create a minimal degenerate triangle strip to avoid empty buffer issues
        TrailsPlugin::update_trail_mesh(&mut mesh, &trail, None, 0.0, &trail_config, Some(1.0));

        // Should have 4 vertices for the degenerate triangle strip
        if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(pos_vec)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            assert_eq!(pos_vec.len(), 4);
            // All vertices should be at origin
            for pos in pos_vec {
                assert_eq!(pos[0], 0.0);
                assert_eq!(pos[1], 0.0);
                assert_eq!(pos[2], 0.0);
            }
        }

        // Should have colors with zero alpha (invisible)
        if let Some(bevy::render::mesh::VertexAttributeValues::Float32x4(color_vec)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        {
            assert_eq!(color_vec.len(), 4);
            // All colors should have zero alpha
            for color in color_vec {
                assert_eq!(color[3], 0.0);
            }
        }
    }

    #[test]
    fn test_update_trail_mesh_with_data() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleStrip,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        let mut trail = Trail::new(Color::WHITE, 1.0);
        let trail_config = crate::config::TrailConfig::default();

        // Add some trail points
        trail.add_point(Vec3::new(0.0, 0.0, 0.0), 0.0);
        trail.add_point(Vec3::new(1.0, 0.0, 0.0), 1.0);
        trail.add_point(Vec3::new(2.0, 0.0, 0.0), 2.0);

        // Update mesh with trail data
        TrailsPlugin::update_trail_mesh(&mut mesh, &trail, None, 2.0, &trail_config, Some(1.0));

        // Should have position data
        if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(pos_vec)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            assert!(!pos_vec.is_empty());
            assert_eq!(pos_vec.len() % 2, 0); // Should be pairs (triangle strip)
        }

        // Should have color data with alpha
        if let Some(bevy::render::mesh::VertexAttributeValues::Float32x4(color_vec)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        {
            assert!(!color_vec.is_empty());
            // Colors should match position count
            if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(pos_vec)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            {
                assert_eq!(color_vec.len(), pos_vec.len());
            }
        }
    }

    #[test]
    fn test_trail_creation() {
        let color = Color::srgb(1.0, 0.0, 0.0);
        let trail = Trail::new(color, 1.0);

        assert_eq!(trail.points.len(), 0);
        assert_eq!(trail.color, color);
        assert_eq!(trail.body_radius, 1.0);
        assert_eq!(trail.last_update, 0.0);
        assert_eq!(trail.pause_time, None);
        assert_eq!(trail.total_pause_duration, 0.0);
    }

    #[test]
    fn test_trail_add_point() {
        let mut trail = Trail::new(Color::WHITE, 1.0);
        let pos1 = Vec3::new(0.0, 0.0, 0.0);
        let pos2 = Vec3::new(1.0, 0.0, 0.0);

        trail.add_point(pos1, 1.0);
        assert_eq!(trail.points.len(), 1);
        assert_eq!(trail.points[0].position, pos1);

        trail.add_point(pos2, 2.0);
        assert_eq!(trail.points.len(), 2);
        assert_eq!(trail.points[0].position, pos2); // Newest point first
        assert_eq!(trail.points[1].position, pos1);
    }

    #[test]
    fn test_trail_cleanup_old_points() {
        let mut trail = Trail::new(Color::WHITE, 1.0);

        // Add points at different times
        trail.add_point(Vec3::new(0.0, 0.0, 0.0), 1.0);
        trail.add_point(Vec3::new(1.0, 0.0, 0.0), 2.0);
        trail.add_point(Vec3::new(2.0, 0.0, 0.0), 3.0);
        trail.add_point(Vec3::new(3.0, 0.0, 0.0), 10.0);

        assert_eq!(trail.points.len(), 4);

        // Clean up points older than 5 seconds from time 10.0
        trail.cleanup_old_points(10.0, 5.0, 1000);

        // Points at times 1.0, 2.0, 3.0 should be removed (older than 5 seconds from 10.0)
        // Only point at time 10.0 should remain
        assert_eq!(trail.points.len(), 1);
        assert_eq!(trail.points[0].age, 10.0);
    }

    #[test]
    fn test_trail_max_points_limit() {
        let mut trail = Trail::new(Color::WHITE, 1.0);

        // Add more points than our limit
        for i in 0..10 {
            trail.add_point(Vec3::new(i as f32, 0.0, 0.0), i as f32);
        }

        // Apply max_points limit of 5
        trail.cleanup_old_points(100.0, 100.0, 5);

        assert_eq!(trail.points.len(), 5);
    }

    #[test]
    fn test_alpha_calculation() {
        let trail = Trail::new(Color::WHITE, 1.0);
        let config = crate::config::TrailConfig {
            trail_length_seconds: 10.0,
            min_alpha: 0.0,
            max_alpha: 1.0,
            enable_fading: true,
            ..Default::default()
        };

        // Create a test point
        let point = TrailPoint {
            position: Vec3::ZERO,
            age: 5.0,
        };

        // Test at different times
        let alpha_new = trail.calculate_point_alpha(&point, 5.0, &config); // Same age as point
        let alpha_mid = trail.calculate_point_alpha(&point, 10.0, &config); // 5 seconds old
        let alpha_old = trail.calculate_point_alpha(&point, 15.0, &config); // 10 seconds old (max age)

        // New point should be max alpha
        assert!((alpha_new - 1.0).abs() < 0.001);

        // Middle-aged point should be partially faded
        assert!(alpha_mid > 0.0 && alpha_mid < 1.0);

        // Old point should be min alpha
        assert!(alpha_old.abs() < 0.001);
    }

    #[test]
    fn test_alpha_calculation_disabled() {
        let trail = Trail::new(Color::WHITE, 1.0);
        let config = crate::config::TrailConfig {
            enable_fading: false,
            max_alpha: 0.8,
            ..Default::default()
        };

        let point = TrailPoint {
            position: Vec3::ZERO,
            age: 0.0,
        };

        // When fading is disabled, should always return max_alpha
        let alpha = trail.calculate_point_alpha(&point, 10.0, &config);
        assert!((alpha - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_should_update() {
        let mut trail = Trail::new(Color::WHITE, 1.0);
        trail.last_update = 5.0;

        assert!(!trail.should_update(5.5, 1.0)); // Too soon
        assert!(trail.should_update(6.0, 1.0)); // Time to update
        assert!(trail.should_update(7.0, 1.0)); // Definitely time to update
    }

    #[test]
    fn test_pause_unpause() {
        let mut trail = Trail::new(Color::WHITE, 1.0);

        assert!(!trail.is_paused());
        assert_eq!(trail.pause_time, None);
        assert_eq!(trail.total_pause_duration, 0.0);

        // Pause at time 10.0
        trail.pause(10.0);
        assert!(trail.is_paused());
        assert_eq!(trail.pause_time, Some(10.0));

        // Unpause at time 15.0 (5 seconds paused)
        trail.unpause(15.0);
        assert!(!trail.is_paused());
        assert_eq!(trail.pause_time, None);
        assert_eq!(trail.total_pause_duration, 5.0);

        // Pause again at time 20.0
        trail.pause(20.0);
        assert!(trail.is_paused());

        // Unpause at time 23.0 (3 more seconds paused)
        trail.unpause(23.0);
        assert_eq!(trail.total_pause_duration, 8.0); // 5 + 3
    }

    #[test]
    fn test_effective_time() {
        let mut trail = Trail::new(Color::WHITE, 1.0);

        // No pause, effective time equals current time
        assert_eq!(trail.effective_time(10.0), 10.0);

        // Pause at time 10.0
        trail.pause(10.0);
        // While paused, effective time should stay at 10.0
        assert_eq!(trail.effective_time(15.0), 10.0);
        assert_eq!(trail.effective_time(20.0), 10.0);

        // Unpause at time 20.0 (10 seconds paused)
        trail.unpause(20.0);
        // Now effective time should be current time minus pause duration
        assert_eq!(trail.effective_time(25.0), 15.0); // 25 - 10 = 15
        assert_eq!(trail.effective_time(30.0), 20.0); // 30 - 10 = 20
    }

    #[test]
    fn test_triangle_strip_vertices() {
        let mut trail = Trail::new(Color::WHITE, 1.0);

        // Empty trail should return no vertices
        let params = TrailRenderParams {
            camera_pos: None,
            base_width: 1.0,
            width_relative_to_body: false,
            body_radius: None,
            body_size_multiplier: 1.0,
            enable_tapering: false,
            taper_curve: &crate::config::TaperCurve::Linear,
            min_width_ratio: 0.1,
        };
        assert_eq!(trail.get_triangle_strip_vertices(&params).len(), 0);

        // Single point should return no vertices (need at least 2 for direction)
        trail.add_point(Vec3::ZERO, 0.0);
        assert_eq!(trail.get_triangle_strip_vertices(&params).len(), 0);

        // Two points should return 4 vertices (2 per point)
        trail.add_point(Vec3::new(1.0, 0.0, 0.0), 1.0);
        let vertices = trail.get_triangle_strip_vertices(&params);
        assert_eq!(vertices.len(), 4);

        // Vertices should form a strip
        assert_ne!(vertices[0], vertices[1]); // Left and right should be different
        assert_ne!(vertices[2], vertices[3]); // Left and right should be different
    }

    #[test]
    fn test_configurable_width() {
        let mut trail = Trail::new(Color::WHITE, 1.0);
        trail.add_point(Vec3::new(0.0, 0.0, 0.0), 0.0);
        trail.add_point(Vec3::new(1.0, 0.0, 0.0), 1.0);

        // Test absolute width
        let mut params = TrailRenderParams {
            camera_pos: None,
            base_width: 0.5,
            width_relative_to_body: false,
            body_radius: None,
            body_size_multiplier: 1.0,
            enable_tapering: false,
            taper_curve: &crate::config::TaperCurve::Linear,
            min_width_ratio: 0.1,
        };
        let vertices_narrow = trail.get_triangle_strip_vertices(&params);

        params.base_width = 2.0;
        let vertices_wide = trail.get_triangle_strip_vertices(&params);

        // Calculate widths by measuring distance between left and right vertices
        let width_narrow = (vertices_narrow[0] - vertices_narrow[1]).length();
        let width_wide = (vertices_wide[0] - vertices_wide[1]).length();

        assert!((width_narrow - 0.5).abs() < 0.01);
        assert!((width_wide - 2.0).abs() < 0.01);

        // Test relative width to body
        params.base_width = 1.0;
        params.width_relative_to_body = true;
        params.body_radius = Some(3.0);
        params.body_size_multiplier = 2.0;
        let vertices_relative = trail.get_triangle_strip_vertices(&params);
        let width_relative = (vertices_relative[0] - vertices_relative[1]).length();

        // Should be body_radius (3.0) * multiplier (2.0) = 6.0
        assert!((width_relative - 6.0).abs() < 0.01);
    }

    #[test]
    fn test_trail_persistence_on_body_despawn() {
        // Test that trails persist as "ghosts" when their tracked body is despawned
        use bevy::state::app::StatesPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), StatesPlugin));
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<SimulationConfig>();
        app.init_state::<AppState>();
        app.add_systems(Update, TrailsPlugin::update_trails);

        // Create a body entity with Transform component
        let body_entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                RigidBody::Dynamic,
            ))
            .id();

        // Create a trail tracking this body
        let mut trail = Trail::new(Color::WHITE, 1.0);
        trail.add_point(Vec3::new(0.0, 0.0, 0.0), 0.0);
        trail.add_point(Vec3::new(1.0, 0.0, 0.0), 1.0);

        // Create a material for the trail
        let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
        let trail_material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            ..default()
        });

        let trail_entity = app
            .world_mut()
            .spawn(TrailBundle::new(body_entity, trail, trail_material))
            .id();

        // Verify trail is tracking the body
        let tracked = app.world().get::<TrackedBody>(trail_entity).unwrap();
        assert_eq!(tracked.0, body_entity);

        // Despawn the body
        app.world_mut().entity_mut(body_entity).despawn();

        // Run update to detect the missing body
        app.update();

        // Trail entity should still exist
        assert!(app.world().get_entity(trail_entity).is_ok());

        // Trail should still have its data
        let trail_data = app.world().get::<Trail>(trail_entity).unwrap();
        assert_eq!(trail_data.points.len(), 2);

        // TrackedBody should still reference the despawned entity
        let tracked_after = app.world().get::<TrackedBody>(trail_entity).unwrap();
        assert_eq!(tracked_after.0, body_entity);
    }

    #[test]
    fn test_width_tapering() {
        let mut trail = Trail::new(Color::WHITE, 1.0);

        // Create a trail with multiple points to test tapering
        for i in 0..5 {
            trail.add_point(Vec3::new(i as f32, 0.0, 0.0), i as f32);
        }

        // Test linear tapering
        let mut params = TrailRenderParams {
            camera_pos: None,
            base_width: 2.0,
            width_relative_to_body: false,
            body_radius: None,
            body_size_multiplier: 1.0,
            enable_tapering: true,
            taper_curve: &crate::config::TaperCurve::Linear,
            min_width_ratio: 0.2,
        };
        let vertices_linear = trail.get_triangle_strip_vertices(&params);

        // Check that width decreases linearly from head to tail
        let width_head = (vertices_linear[0] - vertices_linear[1]).length();
        let width_tail = (vertices_linear[8] - vertices_linear[9]).length();

        assert!((width_head - 2.0).abs() < 0.01, "Head width should be 2.0");
        assert!(
            (width_tail - 0.4).abs() < 0.01,
            "Tail width should be 2.0 * 0.2 = 0.4"
        );

        // Test exponential tapering
        params.taper_curve = &crate::config::TaperCurve::Exponential;
        let vertices_exp = trail.get_triangle_strip_vertices(&params);

        let width_exp_head = (vertices_exp[0] - vertices_exp[1]).length();
        let width_exp_tail = (vertices_exp[8] - vertices_exp[9]).length();

        assert!(
            (width_exp_head - 2.0).abs() < 0.01,
            "Exponential head width should be 2.0"
        );
        assert!(
            (width_exp_tail - 0.4).abs() < 0.01,
            "Exponential tail width should be 0.4"
        );

        // Test disabled tapering
        params.enable_tapering = false;
        params.taper_curve = &crate::config::TaperCurve::Linear;
        let vertices_no_taper = trail.get_triangle_strip_vertices(&params);

        let width_no_taper_head = (vertices_no_taper[0] - vertices_no_taper[1]).length();
        let width_no_taper_tail = (vertices_no_taper[8] - vertices_no_taper[9]).length();

        assert!(
            (width_no_taper_head - 2.0).abs() < 0.01,
            "No taper head width should be 2.0"
        );
        assert!(
            (width_no_taper_tail - 2.0).abs() < 0.01,
            "No taper tail width should also be 2.0"
        );
    }
}
