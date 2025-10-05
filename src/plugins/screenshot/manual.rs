//! Manual screenshot functionality
//!
//! This module handles user-triggered screenshots via keyboard shortcuts
//! or UI buttons, with automatic UI hiding for clean captures.

use crate::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use chrono::Local;
use std::path::PathBuf;

#[derive(Resource, Default)]
pub struct ScreenshotState {
    pub pending: bool,
    pub frame_count: u32,
}

pub fn handle_take_screenshot_event(
    mut commands_reader: MessageReader<SimulationCommand>,
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
