//! Action handlers for simulation commands
//!
//! This module contains handlers for SimulationCommand events including
//! restart, pause/resume, and screenshot functionality.

use super::physics::spawn_bodies;
use crate::physics::components::PhysicsBody;
use crate::physics::resources::PhysicsTime;
use crate::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy_panorbit_camera::PanOrbitCamera;
use chrono::Local;
use std::path::PathBuf;

#[derive(Resource, Default)]
pub struct ScreenshotState {
    pub pending: bool,
    pub frame_count: u32,
}

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

pub fn handle_take_screenshot_event(
    mut commands_reader: EventReader<SimulationCommand>,
    mut screenshot_state: ResMut<ScreenshotState>,
    mut ui_query: Query<&mut Visibility, With<crate::plugins::controls::UIRoot>>,
    mut hud_query: Query<
        &mut Visibility,
        (
            With<crate::plugins::diagnostics_hud::DiagnosticsHudRoot>,
            Without<crate::plugins::controls::UIRoot>,
        ),
    >,
) {
    for command in commands_reader.read() {
        if !matches!(command, SimulationCommand::TakeScreenshot) {
            continue;
        }
        // Hide UI
        for mut visibility in &mut ui_query {
            *visibility = Visibility::Hidden;
        }

        // Hide diagnostics HUD
        for mut visibility in &mut hud_query {
            *visibility = Visibility::Hidden;
        }

        // Set screenshot pending
        screenshot_state.pending = true;
        screenshot_state.frame_count = 0;
    }
}

pub fn process_screenshot_capture(
    mut commands: Commands,
    mut screenshot_state: ResMut<ScreenshotState>,
    mut ui_query: Query<&mut Visibility, With<crate::plugins::controls::UIRoot>>,
    mut hud_query: Query<
        &mut Visibility,
        (
            With<crate::plugins::diagnostics_hud::DiagnosticsHudRoot>,
            Without<crate::plugins::controls::UIRoot>,
        ),
    >,
    config: Res<SimulationConfig>,
) {
    if !screenshot_state.pending {
        return;
    }

    screenshot_state.frame_count += 1;

    // Wait configured frames to ensure UI is hidden
    if screenshot_state.frame_count == config.screenshots.hide_ui_frame_delay {
        // Build filename
        let filename = if config.screenshots.include_timestamp {
            let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S%.3f");
            format!("{}_{timestamp}.png", config.screenshots.filename_prefix)
        } else {
            format!("{}.png", config.screenshots.filename_prefix)
        };

        // Build full path
        let full_path = if let Some(ref dir) = config.screenshots.directory {
            let dir_path = PathBuf::from(dir);

            // Create directory if it doesn't exist
            if !dir_path.exists() {
                if let Err(e) = std::fs::create_dir_all(&dir_path) {
                    error!("Failed to create screenshot directory: {}", e);
                    // Fallback to current directory
                    PathBuf::from(filename)
                } else {
                    dir_path.join(filename)
                }
            } else {
                dir_path.join(filename)
            }
        } else {
            PathBuf::from(filename)
        };

        // Convert to string for save_to_disk
        let path_string = full_path.to_string_lossy().to_string();

        if config.screenshots.notification_enabled {
            info!("Taking screenshot: {}", path_string);
        }

        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path_string));
    }

    // Restore UI after one more frame
    if screenshot_state.frame_count > config.screenshots.hide_ui_frame_delay {
        for mut visibility in &mut ui_query {
            *visibility = Visibility::Visible;
        }
        for mut visibility in &mut hud_query {
            *visibility = Visibility::Visible;
        }
        screenshot_state.pending = false;
        screenshot_state.frame_count = 0;
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

    #[test]
    fn test_screenshot_command_with_ui_hiding() {
        let mut app = create_test_app();
        app.add_plugins(bevy_panorbit_camera::PanOrbitCameraPlugin);
        app.add_plugins(crate::plugins::simulation::SimulationPlugin::new());

        // Add screenshot config and state
        let mut config = SimulationConfig::default();
        config.screenshots.hide_ui_frame_delay = 2;
        app.insert_resource(config);
        app.insert_resource(ScreenshotState::default());

        // Create some UI entities
        use bevy::ui::Node;
        let ui_entity = app
            .world_mut()
            .spawn((Node::default(), crate::plugins::controls::UIRoot))
            .id();

        app.world_mut()
            .insert_resource(NextState::Pending(AppState::Running));
        app.update();

        // Send screenshot command
        app.world_mut()
            .send_event(SimulationCommand::TakeScreenshot);
        app.update();

        // Check that UI was hidden
        let visibility = app.world().entity(ui_entity).get::<Visibility>();
        assert!(visibility.is_some());
        assert_eq!(*visibility.unwrap(), Visibility::Hidden);

        // Check that screenshot state was set
        let screenshot_state = app.world().resource::<ScreenshotState>();
        assert!(screenshot_state.pending);
    }
}
