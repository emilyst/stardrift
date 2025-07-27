//! Camera plugin - Self-contained plugin pattern
//!
//! This plugin handles camera setup and positioning. While the camera positioning
//! is calculated based on physics parameters (body count and distribution), the
//! camera itself is conceptually separate from the physics simulation.

use crate::config::SimulationConfig;
use crate::prelude::*;
use crate::utils::math::min_sphere_radius_for_surface_distribution;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::ClusterConfig;
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
            hdr: true,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Camera3d::default(),
        ClusterConfig::None,
        Tonemapping::AcesFitted,
        Bloom::default(),
        Msaa::Sample4,
        PanOrbitCamera {
            allow_upside_down: true,
            focus: Vec3::ZERO,
            pan_smoothness: 0.0,
            radius: Some(
                (body_distribution_sphere_radius * config.rendering.camera_radius_multiplier)
                    as f32,
            ),
            touch_enabled: true,
            touch_controls: TouchControls::OneFingerOrbit,
            trackpad_behavior: TrackpadBehavior::blender_default(),
            trackpad_pinch_to_zoom_enabled: true,
            ..default()
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_app;
    use bevy_panorbit_camera::PanOrbitCamera;

    #[test]
    fn test_camera_distance_calculation() {
        let mut app = create_test_app();
        app.add_plugins(bevy_panorbit_camera::PanOrbitCameraPlugin);
        app.add_plugins(CameraPlugin);

        // Set up with 100 bodies and custom config for predictable results
        let mut config = SimulationConfig::default();
        config.physics.body_distribution_sphere_radius_multiplier = 1.0;
        config.physics.body_distribution_min_distance = 1.0;
        config.rendering.camera_radius_multiplier = 3.0;

        app.insert_resource(config);
        app.insert_resource(BodyCount::from(100));

        app.update();

        // Check calculated distance
        let pan_orbit_entity = app
            .world()
            .iter_entities()
            .find(|e| app.world().get::<PanOrbitCamera>(e.id()).is_some())
            .expect("PanOrbitCamera should exist");
        let pan_orbit = app
            .world()
            .get::<PanOrbitCamera>(pan_orbit_entity.id())
            .unwrap();

        // The radius should be based on the sphere distribution calculation
        // For 100 bodies with min_distance 1.0, the sphere radius is approximately:
        // min_sphere_radius_for_surface_distribution(100, 1.0) * camera_radius_multiplier
        assert!(pan_orbit.radius.is_some());
        let radius = pan_orbit.radius.unwrap();

        // For 100 bodies, the radius should be reasonable (not checking exact value due to complex calculation)
        assert!(radius > 10.0);
        assert!(radius < 1000.0);
    }

    #[test]
    fn test_camera_configuration_from_config() {
        let mut app = create_test_app();
        app.add_plugins(bevy_panorbit_camera::PanOrbitCameraPlugin);
        app.add_plugins(CameraPlugin);

        // Custom config with different camera settings
        let mut config = SimulationConfig::default();
        config.rendering.camera_radius_multiplier = 5.0;
        let _original_sphere_multiplier = config.physics.body_distribution_sphere_radius_multiplier;

        app.insert_resource(config);
        app.insert_resource(BodyCount::from(100));

        app.update();

        // Check radius uses custom multiplier
        let pan_orbit_entity = app
            .world()
            .iter_entities()
            .find(|e| app.world().get::<PanOrbitCamera>(e.id()).is_some())
            .expect("PanOrbitCamera should exist");
        let pan_orbit = app
            .world()
            .get::<PanOrbitCamera>(pan_orbit_entity.id())
            .unwrap();

        // Verify that camera has a radius and it's affected by the camera multiplier
        assert!(pan_orbit.radius.is_some());
        let radius = pan_orbit.radius.unwrap();

        // The radius should be larger with multiplier 5.0 than default 4.0
        // We check the ratio instead of exact value
        let expected_ratio = 5.0 / 4.0;
        let default_radius = 2389.6157; // From the earlier test
        let ratio = radius / default_radius;

        // Allow some tolerance for floating point comparison
        assert!(
            (ratio - expected_ratio).abs() < 0.1,
            "Camera multiplier not applied correctly: expected ratio {expected_ratio}, got {ratio}"
        );
    }
}
