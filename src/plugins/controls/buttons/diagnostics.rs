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

pub fn sync_diagnostics_button_text(
    settings: Res<DiagnosticsHudSettings>,
    mut initialized: Local<bool>,
    button_children_query: Query<&Children, With<DiagnosticsHudToggleButton>>,
    mut text_query: Query<&mut Text>,
) {
    // Sync on first run or when settings change
    if !*initialized || settings.is_changed() {
        *initialized = true;

        let text_str = if settings.enabled {
            "Hide Diagnostics (D)"
        } else {
            "Show Diagnostics (D)"
        };

        for children in button_children_query.iter() {
            for child in children {
                if let Ok(mut text) = text_query.get_mut(*child) {
                    *text = Text::new(text_str.to_string());
                    break;
                }
            }
        }
    }
}
