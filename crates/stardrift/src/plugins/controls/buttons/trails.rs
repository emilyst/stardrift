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

pub fn initialize_trails_button_text(
    settings: Res<TrailsVisualizationSettings>,
    mut query: Query<(&TrailsToggleButton, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    let text_str = if settings.enabled {
        "Hide Trails (T)"
    } else {
        "Show Trails (T)"
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

pub fn update_trails_button_text(
    settings: Res<TrailsVisualizationSettings>,
    mut query: Query<(&TrailsToggleButton, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    if settings.is_changed() {
        let text_str = if settings.enabled {
            "Hide Trails (T)"
        } else {
            "Show Trails (T)"
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
