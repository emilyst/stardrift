//! Camera plugin - Self-contained plugin pattern
//!
//! This plugin handles camera setup and positioning. While the camera positioning
//! is calculated based on physics parameters (body count and distribution), the
//! camera itself is conceptually separate from the physics simulation.

use crate::config::SimulationConfig;
use crate::physics::math::min_sphere_radius_for_surface_distribution;
use crate::prelude::*;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::render::view::Hdr;
use bevy_panorbit_camera::PanOrbitCamera;
use bevy_panorbit_camera::TouchControls;
use bevy_panorbit_camera::TrackpadBehavior;

/// Plugin that handles camera setup and control
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera);
    }
}

/// Spawns the main camera with appropriate positioning based on simulation parameters
fn spawn_camera(mut commands: Commands, body_count: Res<BodyCount>, config: Res<SimulationConfig>) {
    // TODO: calculate distance at which min sphere radius subtends camera frustum
    let body_distribution_sphere_radius = min_sphere_radius_for_surface_distribution(
        **body_count,
        config.physics.body_distribution_sphere_radius_multiplier,
        config.physics.body_distribution_min_distance,
    );

    commands.spawn((
        Name::new("Main Camera"),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Camera3d::default(),
        Hdr,
        Tonemapping::AcesFitted,
        Bloom::NATURAL,
        Msaa::Sample4,
        PanOrbitCamera {
            allow_upside_down: true,
            focus: Vec3::ZERO,
            pan_smoothness: 0.0,
            radius: Some(
                body_distribution_sphere_radius * config.rendering.camera_radius_multiplier,
            ),
            touch_enabled: true,
            touch_controls: TouchControls::OneFingerOrbit,
            trackpad_behavior: TrackpadBehavior::blender_default(),
            trackpad_pinch_to_zoom_enabled: true,
            ..default()
        },
    ));
}
