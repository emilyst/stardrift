//! Barycenter gizmo toggle button component

use crate::plugins::controls::ButtonWithLabel;
use crate::prelude::*;
use bevy::prelude::*;

#[derive(Component, Default)]
pub struct BarycenterGizmoToggleButton;

impl ButtonWithLabel for BarycenterGizmoToggleButton {
    fn command() -> SimulationCommand {
        SimulationCommand::ToggleBarycenterGizmo
    }

    fn marker() -> Self {
        Self
    }

    fn base_text() -> &'static str {
        "Show Barycenter"
    }

    fn shortcut() -> &'static str {
        "C"
    }
}

pub fn initialize_barycenter_button_text(
    settings: Res<BarycenterGizmoVisibility>,
    mut query: Query<(&BarycenterGizmoToggleButton, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    let text_str = if settings.enabled {
        "Hide Barycenter (C)"
    } else {
        "Show Barycenter (C)"
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

pub fn update_barycenter_button_text(
    settings: Res<BarycenterGizmoVisibility>,
    mut query: Query<(&BarycenterGizmoToggleButton, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    if settings.is_changed() {
        let text_str = if settings.enabled {
            "Hide Barycenter (C)"
        } else {
            "Show Barycenter (C)"
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
