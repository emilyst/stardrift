use crate::config::SimulationConfig;
use crate::resources::*;
use crate::utils::math;
use avian3d::math::Scalar;
use avian3d::prelude::RigidBody;
use bevy::color::palettes::css;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::ClusterConfig;
use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, TouchControls, TrackpadBehavior};

pub fn spawn_camera(
    mut commands: Commands,
    body_count: Res<BodyCount>,
    config: Res<SimulationConfig>,
) {
    // TODO: calculate distance at which min sphere radius subtends camera frustum
    let body_distribution_sphere_radius = math::min_sphere_radius_for_surface_distribution(
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
        Tonemapping::TonyMcMapface,
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

pub fn shift_bodies_to_initial_barycenter(
    mut bodies: Query<&mut Transform, With<RigidBody>>,
    current_barycenter: Res<CurrentBarycenter>,
    initial_barycenter: Res<InitialBarycenter>,
) {
    if !current_barycenter.is_finite() {
        return;
    }

    // If initial barycenter hasn't been set yet, don't shift anything
    let Some(initial_barycenter_value) = **initial_barycenter else {
        return;
    };

    if !initial_barycenter_value.is_finite() {
        return;
    }

    // Calculate the shift needed to keep barycenter at its initial position
    let barycenter_drift = **current_barycenter - initial_barycenter_value;

    // If there's no significant drift, don't update anything
    if barycenter_drift.length_squared() < 1e-12 {
        return;
    }

    // Shift all bodies by the negative of the barycenter drift
    // This keeps the barycenter at its initial position
    let position_shift = -barycenter_drift;

    for mut transform in &mut bodies {
        transform.translation += position_shift.as_vec3();
    }
}

pub fn update_camera_focus_to_initial_barycenter(
    mut camera_query: Query<&mut PanOrbitCamera>,
    initial_barycenter: Res<InitialBarycenter>,
) {
    // Only update camera focus if initial barycenter has been set
    if let Some(initial_barycenter_value) = **initial_barycenter {
        if initial_barycenter_value.is_finite()
            && initial_barycenter_value != avian3d::math::Vector::ZERO
        {
            if let Ok(mut camera) = camera_query.single_mut() {
                // Only update if the camera is still focused on the origin (hasn't been moved by user)
                if camera.focus == Vec3::ZERO {
                    camera.focus = initial_barycenter_value.as_vec3();
                    camera.force_update = true;
                }
            }
        }
    }
}

pub fn draw_barycenter_gizmo(
    mut gizmos: Gizmos,
    body_count: Res<BodyCount>,
    barycenter_gizmo_visibility: Res<BarycenterGizmoVisibility>,
    initial_barycenter: Res<InitialBarycenter>,
) {
    if barycenter_gizmo_visibility.enabled {
        if let Some(initial_barycenter_value) = **initial_barycenter {
            if initial_barycenter_value.is_finite() {
                // Draw the gizmo at the initial barycenter position since we're keeping the barycenter there
                let gizmo_position = if initial_barycenter_value == avian3d::math::Vector::ZERO {
                    Vec3::ZERO // If initial barycenter is at origin, draw at origin
                } else {
                    initial_barycenter_value.as_vec3()
                };

                gizmos.cross(
                    gizmo_position,
                    libm::cbrt(**body_count as Scalar * **body_count as Scalar / 3.0) as f32,
                    css::WHITE,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use avian3d::math::Vector;
    use avian3d::prelude::RigidBody;
    use bevy::ecs::system::SystemState;

    #[test]
    fn test_shift_bodies_to_initial_barycenter() {
        let mut world = World::new();
        world.init_resource::<CurrentBarycenter>();
        world.init_resource::<InitialBarycenter>();

        // Set up initial and current barycenter positions
        let initial_barycenter = Vector::new(2.0, 1.0, -1.0);
        let current_barycenter = Vector::new(7.0, 4.0, -3.0);

        **world.resource_mut::<InitialBarycenter>() = Some(initial_barycenter);
        **world.resource_mut::<CurrentBarycenter>() = current_barycenter;

        // Spawn some test bodies
        let body1_initial_pos = Vec3::new(10.0, 0.0, 0.0);
        let body2_initial_pos = Vec3::new(-5.0, 8.0, 3.0);

        world.spawn((
            Transform::from_translation(body1_initial_pos),
            RigidBody::Dynamic,
        ));

        world.spawn((
            Transform::from_translation(body2_initial_pos),
            RigidBody::Dynamic,
        ));

        // Calculate expected shift before calling the function
        let expected_shift = -(current_barycenter - initial_barycenter);
        let expected_body1_pos = body1_initial_pos + expected_shift.as_vec3();
        let expected_body2_pos = body2_initial_pos + expected_shift.as_vec3();

        // Create system state
        let mut system_state: SystemState<(
            Query<&mut Transform, With<RigidBody>>,
            Res<CurrentBarycenter>,
            Res<InitialBarycenter>,
        )> = SystemState::new(&mut world);

        // Run the shift_bodies_to_initial_barycenter function
        let (bodies, current_barycenter, initial_barycenter) = system_state.get_mut(&mut world);
        shift_bodies_to_initial_barycenter(bodies, current_barycenter, initial_barycenter);

        // Get the updated transforms
        let transforms: Vec<Vec3> = world
            .query::<&Transform>()
            .iter(&world)
            .map(|t| t.translation)
            .collect();

        assert_eq!(transforms.len(), 2);
        assert!((transforms[0] - expected_body1_pos).length() < 1e-6);
        assert!((transforms[1] - expected_body2_pos).length() < 1e-6);
    }

    #[test]
    fn test_shift_bodies_no_change_when_barycenter_at_initial() {
        let mut world = World::new();
        world.init_resource::<CurrentBarycenter>();
        world.init_resource::<InitialBarycenter>();

        // Set up identical initial and current barycenter positions (no shift needed)
        let barycenter = Vector::new(2.0, 1.0, -1.0);
        **world.resource_mut::<InitialBarycenter>() = Some(barycenter);
        **world.resource_mut::<CurrentBarycenter>() = barycenter;

        // Spawn a test body
        let initial_pos = Vec3::new(10.0, 5.0, -3.0);
        world.spawn((Transform::from_translation(initial_pos), RigidBody::Dynamic));

        // Create system state
        let mut system_state: SystemState<(
            Query<&mut Transform, With<RigidBody>>,
            Res<CurrentBarycenter>,
            Res<InitialBarycenter>,
        )> = SystemState::new(&mut world);

        // Run the shift_bodies_to_initial_barycenter function
        let (bodies, current_barycenter, initial_barycenter) = system_state.get_mut(&mut world);
        shift_bodies_to_initial_barycenter(bodies, current_barycenter, initial_barycenter);

        // Check that body position hasn't changed
        let final_pos = world
            .query::<&Transform>()
            .iter(&world)
            .next()
            .unwrap()
            .translation;

        assert_eq!(final_pos, initial_pos);
    }
}
