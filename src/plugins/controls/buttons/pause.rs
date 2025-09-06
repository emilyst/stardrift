//! Pause/resume button component

use crate::plugins::controls::ButtonWithLabel;
use crate::prelude::*;
use bevy::prelude::*;

#[derive(Component, Default)]
pub struct PauseButton;

impl ButtonWithLabel for PauseButton {
    fn command() -> SimulationCommand {
        SimulationCommand::TogglePause
    }

    fn marker() -> Self {
        Self
    }

    fn base_text() -> &'static str {
        "Pause"
    }

    fn shortcut() -> &'static str {
        "Space"
    }
}

pub fn sync_pause_button_text(
    state: Res<State<AppState>>,
    mut initialized: Local<bool>,
    mut button_children_query: Query<&Children, With<PauseButton>>,
    mut text_query: Query<&mut Text>,
) {
    // Sync on first run or when state changes
    if !*initialized || state.is_changed() {
        *initialized = true;

        for children in button_children_query.iter_mut() {
            let dynamic_text = match state.get() {
                AppState::Running => "Pause (Space)",
                AppState::Paused => "Resume (Space)",
            };

            for child in children {
                if let Ok(mut text) = text_query.get_mut(*child) {
                    *text = Text::new(dynamic_text.to_string());
                    break;
                }
            }
        }
    }
}
