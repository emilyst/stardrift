//! Trails visualization toggle button component

use crate::plugins::controls::ButtonWithLabel;
use crate::prelude::*;
use bevy::prelude::*;

#[derive(Component, Default)]
pub struct TrailsToggleButton;

impl ButtonWithLabel for TrailsToggleButton {
    fn command() -> SimulationCommand {
        SimulationCommand::ToggleTrailsVisualization
    }

    fn marker() -> Self {
        Self
    }

    fn base_text() -> &'static str {
        "Show Trails"
    }

    fn shortcut() -> &'static str {
        "T"
    }
}

pub fn sync_trails_button_text(
    settings: Res<TrailsVisualizationSettings>,
    mut initialized: Local<bool>,
    button_children_query: Query<&Children, With<TrailsToggleButton>>,
    mut text_query: Query<&mut Text>,
) {
    // Sync on first run or when settings change
    if !*initialized || settings.is_changed() {
        *initialized = true;

        let text_str = if settings.enabled {
            "Hide Trails (T)"
        } else {
            "Show Trails (T)"
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
