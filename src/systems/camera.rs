use crate::config;
use crate::resources;
use crate::utils;
use avian3d::math::Scalar;
use bevy::color::palettes::css;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::ClusterConfig;
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;
use bevy_panorbit_camera::TouchControls;
use bevy_panorbit_camera::TrackpadBehavior;

pub fn spawn_camera(
    mut commands: Commands,
    body_count: Res<resources::BodyCount>,
    config: Res<config::SimulationConfig>,
) {
    // TODO: calculate distance at which min sphere radius subtends camera frustum
    let body_distribution_sphere_radius = utils::math::min_sphere_radius_for_surface_distribution(
        **body_count,
        config.physics.body_distribution_sphere_radius_multiplier,
        config.physics.body_distribution_min_distance,
    );

    commands.spawn((
        Name::new("Main Camera"),
        Camera {
            hdr: true,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Camera3d::default(),
        ClusterConfig::None,
        Tonemapping::AcesFitted,
        Bloom::NATURAL,
        Msaa::Off,
        PanOrbitCamera {
            allow_upside_down: true,
            focus: Vec3::ZERO,
            pan_smoothness: 0.0,
            radius: Some(
                (body_distribution_sphere_radius * config.rendering.camera_radius_multiplier)
                    as f32,
            ),
            touch_controls: TouchControls::OneFingerOrbit,
            trackpad_behavior: TrackpadBehavior::blender_default(),
            trackpad_pinch_to_zoom_enabled: true,
            ..default()
        },
    ));
}

pub fn draw_barycenter_gizmo(
    mut gizmos: Gizmos,
    body_count: Res<resources::BodyCount>,
    barycenter_gizmo_visibility: Res<resources::BarycenterGizmoVisibility>,
    barycenter: Res<resources::Barycenter>,
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
