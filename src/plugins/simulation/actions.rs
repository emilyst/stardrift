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
    mut commands_reader: MessageReader<SimulationCommand>,
    mut commands: Commands,
    simulation_bodies: Query<Entity, With<PhysicsBody>>,
    mut physics_rng: ResMut<SharedRng>,
    mut rendering_rng: ResMut<RenderingRng>,
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
        // Despawn all bodies; the trails plugin handles its own renderers
        // in response to the same Restart command.
        simulation_bodies.iter().for_each(|entity| {
            commands.entity(entity).despawn();
        });

        **barycenter = None;

        octree.build(std::iter::empty());

        pan_orbit_camera.target_focus = Vec3::ZERO;
        pan_orbit_camera.force_update = true;

        *physics_rng = SharedRng::default();
        *rendering_rng = RenderingRng::default();

        spawn_bodies(
            &mut commands,
            &mut physics_rng,
            &mut rendering_rng,
            **body_count,
            &config,
        );
    }
}

pub fn handle_toggle_pause_simulation_event(
    mut commands_reader: MessageReader<SimulationCommand>,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for command in commands_reader.read() {
        if !matches!(command, SimulationCommand::TogglePause) {
            continue;
        }
        match current_state.get() {
            // Startup hold; the loading screen releases it.
            AppState::Loading => {}
            AppState::Running => next_state.set(AppState::Paused),
            AppState::Paused => next_state.set(AppState::Running),
        }
    }
}

/// `PhysicsTime` follows `AppState`: held in `Loading` and `Paused`, released
/// in `Running`. Runs in `OnEnter`, which precedes the frame's fixed steps.
pub fn pause_physics(mut physics_time: ResMut<PhysicsTime>) {
    physics_time.pause();
}

pub fn unpause_physics(mut physics_time: ResMut<PhysicsTime>) {
    physics_time.unpause();
}
