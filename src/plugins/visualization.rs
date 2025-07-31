//! Visualization plugin - Self-contained plugin pattern
//!
//! This plugin handles debug visualization features including octree wireframe
//! rendering and barycenter gizmo display. It responds to SimulationCommand
//! events to toggle visualization states.

use crate::physics::aabb3d::Aabb3d;
use crate::prelude::*;
use bevy::color::palettes::css;

/// Plugin that provides debug visualization features
pub struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        // Initialize resources
        app.insert_resource(OctreeVisualizationSettings {
            enabled: false,
            ..default()
        });
        app.init_resource::<BarycenterGizmoVisibility>();
        app.insert_resource(TrailsVisualizationSettings { enabled: true });

        // Add systems
        app.add_systems(
            Update,
            (
                handle_visualization_commands,
                visualize_octree.run_if(resource_exists_and_equals(OctreeVisualizationSettings {
                    enabled: true,
                    max_depth: None,
                })),
                draw_barycenter_gizmo.run_if(resource_exists_and_equals(
                    BarycenterGizmoVisibility { enabled: true },
                )),
                update_trail_visibility,
            ),
        );
    }
}

/// Handles SimulationCommand events for visualization features
fn handle_visualization_commands(
    mut commands: EventReader<SimulationCommand>,
    mut octree_settings: ResMut<OctreeVisualizationSettings>,
    mut barycenter_visibility: ResMut<BarycenterGizmoVisibility>,
    mut trails_settings: ResMut<TrailsVisualizationSettings>,
) {
    for command in commands.read() {
        match command {
            SimulationCommand::ToggleOctreeVisualization => {
                octree_settings.enabled = !octree_settings.enabled;
                info!(
                    "Octree visualization {}",
                    if octree_settings.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
            }
            SimulationCommand::ToggleBarycenterGizmo => {
                barycenter_visibility.enabled = !barycenter_visibility.enabled;
                info!(
                    "Barycenter gizmo {}",
                    if barycenter_visibility.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
            }
            SimulationCommand::ToggleTrailsVisualization => {
                trails_settings.enabled = !trails_settings.enabled;
                info!(
                    "Trails visualization {}",
                    if trails_settings.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
            }
            SimulationCommand::SetOctreeMaxDepth(depth) => {
                octree_settings.max_depth = depth.map(|d| d as usize);
                info!(
                    "Octree max depth set to {}",
                    depth.map_or("all".to_string(), |d| d.to_string())
                );
            }
            _ => {} // Ignore other commands
        }
    }
}

/// Visualizes the octree structure using debug gizmos
fn visualize_octree(
    mut gizmos: Gizmos,
    octree: Res<GravitationalOctree>,
    settings: Res<OctreeVisualizationSettings>,
) {
    if !settings.enabled {
        return;
    }

    // Explicit dereference to avoid false positives in IDE code analysis
    let bounds = octree.as_ref().bounds(settings.max_depth);

    for aabb in bounds {
        draw_bounding_box_wireframe_gizmo(&mut gizmos, &aabb, css::WHITE);
    }
}

/// Draws a wireframe bounding box using gizmos
fn draw_bounding_box_wireframe_gizmo(gizmos: &mut Gizmos, aabb: &Aabb3d, color: impl Into<Color>) {
    let min = aabb.min.as_vec3();
    let max = aabb.max.as_vec3();
    let color = color.into();

    let corners = [
        Vec3::new(min.x, min.y, min.z), // 0: min corner
        Vec3::new(max.x, min.y, min.z), // 1: +x
        Vec3::new(max.x, max.y, min.z), // 2: +x+y
        Vec3::new(min.x, max.y, min.z), // 3: +y
        Vec3::new(min.x, min.y, max.z), // 4: +z
        Vec3::new(max.x, min.y, max.z), // 5: +x+z
        Vec3::new(max.x, max.y, max.z), // 6: max corner
        Vec3::new(min.x, max.y, max.z), // 7: +y+z
    ];

    // Bottom face (z = min)
    gizmos.line(corners[0], corners[1], color); // min to +x
    gizmos.line(corners[1], corners[2], color); // +x to +x+y
    gizmos.line(corners[2], corners[3], color); // +x+y to +y
    gizmos.line(corners[3], corners[0], color); // +y to min

    // Top face (z = max)
    gizmos.line(corners[4], corners[5], color); // +z to +x+z
    gizmos.line(corners[5], corners[6], color); // +x+z to max
    gizmos.line(corners[6], corners[7], color); // max to +y+z
    gizmos.line(corners[7], corners[4], color); // +y+z to +z

    // Vertical edges
    gizmos.line(corners[0], corners[4], color); // min to +z
    gizmos.line(corners[1], corners[5], color); // +x to +x+z
    gizmos.line(corners[2], corners[6], color); // +x+y to max
    gizmos.line(corners[3], corners[7], color); // +y to +y+z
}

/// Draws a cross gizmo at the barycenter position
fn draw_barycenter_gizmo(
    mut gizmos: Gizmos,
    body_count: Res<BodyCount>,
    barycenter_gizmo_visibility: Res<BarycenterGizmoVisibility>,
    barycenter: Res<Barycenter>,
) {
    if barycenter_gizmo_visibility.enabled {
        if let Some(barycenter) = **barycenter {
            if barycenter.is_finite() {
                gizmos.cross(
                    barycenter.as_vec3(),
                    libm::cbrt(**body_count as Scalar * **body_count as Scalar / 3.0) as f32,
                    css::WHITE,
                );
            }
        }
    }
}

/// Resource to control octree visualization settings
#[derive(Resource, Default)]
pub struct OctreeVisualizationSettings {
    pub enabled: bool,
    pub max_depth: Option<usize>, // None means show all levels
}

impl PartialEq for OctreeVisualizationSettings {
    fn eq(&self, other: &Self) -> bool {
        self.enabled == other.enabled
    }
}

/// Resource to control barycenter gizmo visibility
#[derive(Resource, Default)]
pub struct BarycenterGizmoVisibility {
    pub enabled: bool,
}

impl PartialEq for BarycenterGizmoVisibility {
    fn eq(&self, other: &Self) -> bool {
        self.enabled == other.enabled
    }
}

/// Resource to control trails visibility
#[derive(Resource, Default)]
pub struct TrailsVisualizationSettings {
    pub enabled: bool,
}

impl PartialEq for TrailsVisualizationSettings {
    fn eq(&self, other: &Self) -> bool {
        self.enabled == other.enabled
    }
}

#[cfg(feature = "trails")]
fn update_trail_visibility(
    trails_settings: Res<TrailsVisualizationSettings>,
    mut trail_query: Query<&mut Visibility, With<crate::plugins::trails::TrailRenderer>>,
) {
    if !trails_settings.is_changed() {
        return;
    }

    for mut visibility in &mut trail_query {
        *visibility = if trails_settings.enabled {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Stub function when trails feature is disabled
#[cfg(not(feature = "trails"))]
fn update_trail_visibility() {}
