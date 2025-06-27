use crate::resources::*;
use crate::utils::math;
use avian3d::math::Scalar;
use bevy::color::palettes::css;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, TouchControls, TrackpadBehavior};

pub fn spawn_camera(mut commands: Commands, body_count: Res<BodyCount>) {
    // TODO: calculate distance at which min sphere radius subtends camera frustum
    let body_distribution_sphere_radius =
        math::min_sphere_radius_for_surface_distribution(**body_count, 200.0, 0.001);

    commands.spawn((
        Name::new("Main Camera"),
        Camera {
            hdr: true,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Camera3d::default(),
        Tonemapping::TonyMcMapface,
        Bloom::NATURAL,
        Msaa::default(),
        PanOrbitCamera {
            focus: Vec3::ZERO,
            pan_smoothness: 0.0,
            radius: Some((body_distribution_sphere_radius * 2.0) as f32),
            touch_controls: TouchControls::OneFingerOrbit,
            trackpad_behavior: TrackpadBehavior::blender_default(),
            trackpad_pinch_to_zoom_enabled: true,
            ..default()
        },
    ));
}

pub fn follow_barycenter(
    mut pan_orbit_camera: Single<&mut PanOrbitCamera>,
    mut gizmos: Gizmos,
    current_barycenter: Res<CurrentBarycenter>,
    body_count: Res<BodyCount>,
) {
    if current_barycenter.is_finite() {
        pan_orbit_camera.force_update = true;
        pan_orbit_camera.target_focus = current_barycenter.clone().as_vec3();
        gizmos.cross(
            current_barycenter.as_vec3(),
            libm::cbrt(**body_count as Scalar * **body_count as Scalar / 3.0) as f32,
            css::WHITE,
        );
    }
}
