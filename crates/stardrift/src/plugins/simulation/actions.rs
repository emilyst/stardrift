//! Action handlers for simulation commands
//!
//! This module contains handlers for SimulationCommand events including
//! restart and pause/resume functionality.

use super::physics::spawn_bodies;
use crate::physics::components::PhysicsBody;
use crate::physics::resources::PhysicsTime;
use crate::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;

#[allow(clippy::too_many_arguments)]
pub fn handle_restart_simulation_event(
    mut commands_reader: EventReader<SimulationCommand>,
    mut commands: Commands,
    simulation_bodies: Query<Entity, With<PhysicsBody>>,
    trail_renderers: Query<Entity, With<crate::plugins::trails::TrailRenderer>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<SharedRng>,
    body_count: Res<BodyCount>,
    mut barycenter: ResMut<Barycenter>,
    mut octree: ResMut<GravitationalOctree>,
    mut pan_orbit_camera: Single<&mut PanOrbitCamera>,
    config: Res<SimulationConfig>,
) {
    for command in commands_reader.read() {
        if !matches!(command, SimulationCommand::Restart) {
            continue;
        }
        // Despawn all bodies
        simulation_bodies.iter().for_each(|entity| {
            commands.entity(entity).despawn();
        });

        // Despawn all trail renderers
        trail_renderers.iter().for_each(|entity| {
            commands.entity(entity).despawn();
        });

        **barycenter = None;

        octree.build(vec![]);

        pan_orbit_camera.target_focus = Vec3::ZERO;
        pan_orbit_camera.force_update = true;

        *rng = SharedRng::default();

        spawn_bodies(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut rng,
            **body_count,
            &config,
        );
    }
}

pub fn handle_toggle_pause_simulation_event(
    mut commands_reader: EventReader<SimulationCommand>,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut physics_time: ResMut<PhysicsTime>,
) {
    for command in commands_reader.read() {
        if !matches!(command, SimulationCommand::TogglePause) {
            continue;
        }
        match current_state.get() {
            AppState::Running => {
                next_state.set(AppState::Paused);
                physics_time.pause();
            }
            AppState::Paused => {
                next_state.set(AppState::Running);
                physics_time.unpause();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_app;

    #[test]
    fn test_pause_toggle_physics_time() {
        let mut app = create_test_app();
        app.add_plugins(bevy_panorbit_camera::PanOrbitCameraPlugin);
        app.add_plugins(crate::plugins::simulation::SimulationPlugin::new());

        app.world_mut()
            .insert_resource(NextState::Pending(AppState::Running));
        app.update();

        // Verify physics time is not paused initially
        let physics_time = app.world().resource::<PhysicsTime>();
        assert!(!physics_time.is_paused());

        // Send pause command
        app.world_mut().send_event(SimulationCommand::TogglePause);
        app.update();

        // Verify physics time is now paused
        let physics_time = app.world().resource::<PhysicsTime>();
        assert!(physics_time.is_paused());

        // Toggle again
        app.world_mut().send_event(SimulationCommand::TogglePause);
        app.update();

        // Verify physics time is unpaused
        let physics_time = app.world().resource::<PhysicsTime>();
        assert!(!physics_time.is_paused());
    }
}
