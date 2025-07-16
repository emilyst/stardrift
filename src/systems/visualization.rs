use crate::physics;
use crate::resources;
use bevy::color::palettes::css;
use bevy::prelude::*;

pub fn visualize_octree(
    mut gizmos: Gizmos,
    octree: Res<resources::GravitationalOctree>,
    settings: Res<resources::OctreeVisualizationSettings>,
) {
    if !settings.enabled {
        return;
    }

    let bounds = octree.bounds(settings.max_depth);

    for aabb in bounds {
        draw_bounding_box_wireframe_gizmo(&mut gizmos, &aabb, css::WHITE);
    }
}

pub fn draw_bounding_box_wireframe_gizmo(
    gizmos: &mut Gizmos,
    aabb: &physics::aabb3d::Aabb3d,
    color: impl Into<Color>,
) {
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
