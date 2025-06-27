use crate::config::SimulationConfig;
use crate::resources::*;
use crate::systems::physics::spawn_simulation_bodies;
use avian3d::math::Vector;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;

pub fn quit_on_escape(keys: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write_default();
    }
}

pub fn restart_simulation_on_r(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    simulation_bodies: Query<Entity, With<RigidBody>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<SharedRng>,
    body_count: Res<BodyCount>,
    mut current_barycenter: ResMut<CurrentBarycenter>,
    mut previous_barycenter: ResMut<PreviousBarycenter>,
    octree: ResMut<GravitationalOctree>,
    mut pan_orbit_camera: Query<&mut PanOrbitCamera>,
    config: Res<SimulationConfig>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        for entity in simulation_bodies.iter() {
            commands.entity(entity).despawn();
        }

        **current_barycenter = Vector::ZERO;
        **previous_barycenter = Vector::ZERO;
        if let Ok(mut octree_guard) = octree.0.write() {
            octree_guard.build(vec![]); // Reset octree with empty body list
        }

        if let Ok(mut camera) = pan_orbit_camera.single_mut() {
            camera.target_focus = Vec3::ZERO;
            camera.force_update = true;
        }

        *rng = SharedRng::default(); // This will create a new random seed

        spawn_simulation_bodies(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut rng,
            **body_count,
            &config,
        );
    }
}

pub fn pause_physics_on_space(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    enabled_rigid_bodies: Query<Entity, (With<RigidBody>, Without<RigidBodyDisabled>)>,
    disabled_rigid_bodies: Query<Entity, (With<RigidBody>, With<RigidBodyDisabled>)>,
) {
    if keys.just_pressed(KeyCode::Space) {
        for entity in &enabled_rigid_bodies {
            commands.entity(entity).insert(RigidBodyDisabled);
        }

        for entity in &disabled_rigid_bodies {
            commands.entity(entity).remove::<RigidBodyDisabled>();
        }
    }
}

pub fn toggle_octree_visualization(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<OctreeVisualizationSettings>,
) {
    for &keycode in keys.get_just_pressed() {
        match keycode {
            KeyCode::KeyO => settings.enabled = !settings.enabled,
            KeyCode::Digit0 => settings.max_depth = None,
            KeyCode::Digit1 => settings.max_depth = Some(1),
            KeyCode::Digit2 => settings.max_depth = Some(2),
            KeyCode::Digit3 => settings.max_depth = Some(3),
            KeyCode::Digit4 => settings.max_depth = Some(4),
            KeyCode::Digit5 => settings.max_depth = Some(5),
            KeyCode::Digit6 => settings.max_depth = Some(6),
            KeyCode::Digit7 => settings.max_depth = Some(7),
            KeyCode::Digit8 => settings.max_depth = Some(8),
            KeyCode::Digit9 => settings.max_depth = Some(9),
            _ => {}
        }
    }
}
