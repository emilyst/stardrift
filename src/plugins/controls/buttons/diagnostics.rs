//! Diagnostics HUD toggle button component

use crate::plugins::controls::ButtonWithLabel;
use crate::plugins::diagnostics_hud::DiagnosticsHudSettings;
use crate::prelude::*;
use bevy::prelude::*;

#[derive(Component, Default)]
pub struct DiagnosticsHudToggleButton;

impl ButtonWithLabel for DiagnosticsHudToggleButton {
    fn command() -> SimulationCommand {
        SimulationCommand::ToggleDiagnosticsHud
    }

    fn marker() -> Self {
        Self
    }

    fn base_text() -> &'static str {
        "Show Diagnostics"
    }

    fn shortcut() -> &'static str {
        "D"
    }
}

pub fn initialize_diagnostics_button_text(
    settings: Res<DiagnosticsHudSettings>,
    mut query: Query<(&DiagnosticsHudToggleButton, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    let text_str = if settings.enabled {
        "Hide Diagnostics (D)"
    } else {
        "Show Diagnostics (D)"
    };
    for (_, children) in &mut query {
        for child in children {
            if let Ok(mut text) = text_query.get_mut(*child) {
                *text = Text::new(text_str.to_string());
                break;
            }
        }
    }
}

pub fn update_diagnostics_button_text(
    settings: Res<DiagnosticsHudSettings>,
    mut query: Query<(&DiagnosticsHudToggleButton, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    if settings.is_changed() {
        let text_str = if settings.enabled {
            "Hide Diagnostics (D)"
        } else {
            "Show Diagnostics (D)"
        };
        for (_, children) in &mut query {
            for child in children {
                if let Ok(mut text) = text_query.get_mut(*child) {
                    *text = Text::new(text_str.to_string());
                    break;
                }
            }
        }
    }
}
