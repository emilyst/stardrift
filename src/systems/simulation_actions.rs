use crate::prelude::*;
use crate::systems::physics::spawn_simulation_bodies;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy_panorbit_camera::PanOrbitCamera;
use chrono::Local;
use std::path::PathBuf;

#[derive(Event)]
pub struct RestartSimulationEvent;

#[derive(Event)]
pub struct ToggleOctreeVisualizationEvent;

#[derive(Event)]
pub struct ToggleBarycenterGizmoVisibilityEvent;

#[derive(Event)]
pub struct TogglePauseSimulationEvent;

#[derive(Event)]
pub struct TakeScreenshotEvent;

#[derive(Resource, Default)]
pub struct ScreenshotState {
    pub pending: bool,
    pub frame_count: u32,
}

#[allow(clippy::too_many_arguments)]
pub fn handle_restart_simulation_event(
    mut restart_events: EventReader<RestartSimulationEvent>,
    mut commands: Commands,
    simulation_bodies: Query<Entity, With<RigidBody>>,
    #[cfg(feature = "trails")] trail_renderers: Query<
        Entity,
        With<crate::systems::trails::TrailRenderer>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<SharedRng>,
    body_count: Res<BodyCount>,
    mut barycenter: ResMut<Barycenter>,
    mut octree: ResMut<GravitationalOctree>,
    mut pan_orbit_camera: Single<&mut PanOrbitCamera>,
    config: Res<SimulationConfig>,
) {
    restart_events.read().for_each(|_| {
        // Despawn all bodies
        simulation_bodies.iter().for_each(|entity| {
            commands.entity(entity).despawn();
        });

        // Despawn all trail renderers
        #[cfg(feature = "trails")]
        trail_renderers.iter().for_each(|entity| {
            commands.entity(entity).despawn();
        });

        **barycenter = None;

        octree.build(vec![]);

        pan_orbit_camera.target_focus = Vec3::ZERO;
        pan_orbit_camera.force_update = true;

        *rng = SharedRng::default();

        spawn_simulation_bodies(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut rng,
            **body_count,
            &config,
        );
    });
}

pub fn handle_toggle_octree_visualization_event(
    mut octree_events: EventReader<ToggleOctreeVisualizationEvent>,
    mut settings: ResMut<OctreeVisualizationSettings>,
) {
    octree_events.read().for_each(|_| {
        settings.enabled = !settings.enabled;
    });
}

pub fn handle_toggle_barycenter_gizmo_visibility_event(
    mut barycenter_events: EventReader<ToggleBarycenterGizmoVisibilityEvent>,
    mut settings: ResMut<BarycenterGizmoVisibility>,
) {
    barycenter_events.read().for_each(|_| {
        settings.enabled = !settings.enabled;
    });
}

pub fn handle_toggle_pause_simulation_event(
    mut pause_events: EventReader<TogglePauseSimulationEvent>,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut time: ResMut<Time<Physics>>,
) {
    pause_events.read().for_each(|_| {
        match current_state.get() {
            AppState::Running => {
                next_state.set(AppState::Paused);
                time.pause();
            }
            AppState::Paused => {
                next_state.set(AppState::Running);
                time.unpause();
            }
            _ => {} // ignore Loading state
        }
    });
}

pub fn handle_take_screenshot_event(
    mut screenshot_events: EventReader<TakeScreenshotEvent>,
    mut screenshot_state: ResMut<ScreenshotState>,
    mut ui_query: Query<&mut Visibility, With<crate::systems::ui::UIRoot>>,
    mut hud_query: Query<
        &mut Visibility,
        (
            With<crate::plugins::diagnostics_hud::DiagnosticsHudRoot>,
            Without<crate::systems::ui::UIRoot>,
        ),
    >,
) {
    screenshot_events.read().for_each(|_| {
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
    });
}

pub fn process_screenshot_capture(
    mut commands: Commands,
    mut screenshot_state: ResMut<ScreenshotState>,
    mut ui_query: Query<&mut Visibility, With<crate::systems::ui::UIRoot>>,
    mut hud_query: Query<
        &mut Visibility,
        (
            With<crate::plugins::diagnostics_hud::DiagnosticsHudRoot>,
            Without<crate::systems::ui::UIRoot>,
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
