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

pub fn initialize_pause_button_text(
    state: Res<State<AppState>>,
    mut query: Query<(&PauseButton, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    for (_, children) in &mut query {
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

pub fn update_pause_button_text(
    state: Res<State<AppState>>,
    mut query: Query<(&PauseButton, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    if state.is_changed() {
        for (_, children) in &mut query {
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
