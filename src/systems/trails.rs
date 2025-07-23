use crate::components::Trail;
use crate::prelude::*;
use bevy::render::mesh::{MeshAabb, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::view::NoFrustumCulling;

#[derive(Component)]
pub struct TrailRenderer {
    pub body_entity: Entity,
}

pub fn update_trails(
    mut trail_query: Query<(&mut Trail, &Transform)>,
    time: Res<Time>,
    config: Res<SimulationConfig>,
) {
    let current_time = time.elapsed_secs();

    for (mut trail, transform) in trail_query.iter_mut() {
        if trail.should_update(current_time, config.trails.update_interval_seconds as f32) {
            trail.add_point(transform.translation, current_time);

            trail.cleanup_old_points(
                current_time,
                config.trails.trail_length_seconds as f32,
                config.trails.max_points_per_trail,
            );
        }
    }
}

/// This system should run after bodies are spawned
pub fn initialize_trails(
    mut commands: Commands,
    query: Query<(Entity, &MeshMaterial3d<StandardMaterial>), (With<RigidBody>, Without<Trail>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    _meshes: ResMut<Assets<Mesh>>,
) {
    for (entity, mesh_material) in query.iter() {
        // Extract color from the body's material
        let color = if let Some(material) = materials.get(&mesh_material.0) {
            material.base_color
        } else {
            Color::WHITE
        };

        commands.entity(entity).insert(Trail::new(color));

        let trail_material = materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            alpha_mode: AlphaMode::Add,
            ..default()
        });

        commands.spawn((
            TrailRenderer {
                body_entity: entity,
            },
            MeshMaterial3d(trail_material),
            Transform::default(),
            // Add visibility bundle to ensure proper culling behavior
            Visibility::default(),
            // TEMPORARY: Disable frustum culling for trails
            // This prevents trails from being culled incorrectly while we debug bounding volume issues
            // Frustum culling is still having problems with dynamic trail geometry despite AABB computation
            // TODO: Remove this once we solve the underlying bounding volume calculation
            NoFrustumCulling,
        ));
    }
}

pub fn render_trails(
    mut commands: Commands,
    mut trail_meshes: ResMut<Assets<Mesh>>,
    trail_query: Query<(&Trail, Option<&Collider>)>,
    mut renderer_query: Query<(Entity, &TrailRenderer, Option<&Mesh3d>)>,
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

    for (renderer_entity, renderer, mesh_handle) in renderer_query.iter_mut() {
        if let Ok((trail, collider)) = trail_query.get(renderer.body_entity) {
            if !trail.points.is_empty() {
                // Extract body radius from collider if available
                let body_radius = collider
                    .and_then(|c| {
                        // Get the underlying shape from the collider
                        let shape = c.shape();

                        // Try to downcast to a Ball shape and get its radius
                        shape.as_ball().map(|ball| ball.radius as f32)
                    })
                    .or(Some(1.0)); // Default radius if we can't extract it

                match mesh_handle {
                    Some(mesh_handle) => {
                        // Update existing mesh
                        if let Some(mesh) = trail_meshes.get_mut(&mesh_handle.0) {
                            update_trail_mesh(
                                mesh,
                                trail,
                                camera_pos,
                                current_time,
                                &config.trails,
                                body_radius,
                            );
                        }
                    }
                    None => {
                        // Create new mesh and add it to the entity
                        let trail_mesh = create_trail_mesh_with_data(
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
    update_trail_mesh(
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
    let strip_vertices = trail.get_triangle_strip_vertices(
        camera_pos,
        trail_config.base_width as f32,
        trail_config.width_relative_to_body,
        body_radius,
        trail_config.body_size_multiplier as f32,
        trail_config.enable_tapering,
        &trail_config.taper_curve,
        trail_config.min_width_ratio as f32,
    );

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

    for (i, vertex) in strip_vertices.iter().enumerate() {
        positions.push([vertex.x, vertex.y, vertex.z]);
        normals.push([0.0, 1.0, 0.0]);

        // Each trail point generates 2 vertices (left and right side of strip)
        // Note: VecDeque has newest points at index 0, so point_index matches directly
        let point_index = i / 2; // Convert vertex index to point index

        if point_index < trail.points.len() {
            let point = &trail.points[point_index];
            let alpha = trail.calculate_point_alpha(point, current_time, trail_config);
            colors.push([base_color.red, base_color.green, base_color.blue, alpha]);
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

    // IMPORTANT: Compute bounds to prevent incorrect frustum culling
    // Without proper bounds, trails may be culled too aggressively
    if mesh.compute_aabb().is_none() {
        warn!("Failed to compute AABB for trail mesh");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::AssetPlugin;

    #[test]
    fn test_trail_renderer_component() {
        let entity = Entity::from_raw(1);
        let renderer = TrailRenderer {
            body_entity: entity,
        };

        assert_eq!(renderer.body_entity, entity);
    }

    #[test]
    fn test_trail_mesh_creation_with_data() {
        // Test that we can create a mesh with trail data
        let mut app = App::new();
        app.add_plugins(AssetPlugin::default());
        app.init_resource::<Assets<Mesh>>();

        let mut trail = Trail::new(Color::WHITE);
        trail.add_point(Vec3::new(0.0, 0.0, 0.0), 0.0);
        trail.add_point(Vec3::new(1.0, 0.0, 0.0), 1.0);

        let trail_config = crate::config::TrailConfig::default();
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        let handle =
            create_trail_mesh_with_data(&mut meshes, &trail, None, 1.0, &trail_config, Some(1.0));

        // Should have a valid handle
        assert!(meshes.get(&handle).is_some());
    }

    #[test]
    fn test_update_trail_mesh_empty() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleStrip,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        let trail = Trail::new(Color::WHITE);
        let trail_config = crate::config::TrailConfig::default();

        // Empty trail should clear the mesh
        update_trail_mesh(&mut mesh, &trail, None, 0.0, &trail_config, Some(1.0));

        // Should have empty position attributes
        if let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            if let bevy::render::mesh::VertexAttributeValues::Float32x3(pos_vec) = positions {
                assert_eq!(pos_vec.len(), 0);
            }
        }
    }

    #[test]
    fn test_update_trail_mesh_with_data() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleStrip,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        let mut trail = Trail::new(Color::WHITE);
        let trail_config = crate::config::TrailConfig::default();

        // Add some trail points
        trail.add_point(Vec3::new(0.0, 0.0, 0.0), 0.0);
        trail.add_point(Vec3::new(1.0, 0.0, 0.0), 1.0);
        trail.add_point(Vec3::new(2.0, 0.0, 0.0), 2.0);

        // Update mesh with trail data
        update_trail_mesh(&mut mesh, &trail, None, 2.0, &trail_config, Some(1.0));

        // Should have position data
        if let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            if let bevy::render::mesh::VertexAttributeValues::Float32x3(pos_vec) = positions {
                assert!(pos_vec.len() > 0);
                assert_eq!(pos_vec.len() % 2, 0); // Should be pairs (triangle strip)
            }
        }

        // Should have color data with alpha
        if let Some(colors) = mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
            if let bevy::render::mesh::VertexAttributeValues::Float32x4(color_vec) = colors {
                assert!(color_vec.len() > 0);
                // Colors should match position count
                if let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                    if let bevy::render::mesh::VertexAttributeValues::Float32x3(pos_vec) = positions
                    {
                        assert_eq!(color_vec.len(), pos_vec.len());
                    }
                }
            }
        }
    }
}
