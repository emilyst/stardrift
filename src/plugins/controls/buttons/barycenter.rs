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

pub fn sync_barycenter_button_text(
    settings: Res<BarycenterGizmoVisibility>,
    mut initialized: Local<bool>,
    mut button_children_query: Query<&Children, With<BarycenterGizmoToggleButton>>,
    mut text_query: Query<&mut Text>,
) {
    // Sync on first run or when settings change
    if !*initialized || settings.is_changed() {
        *initialized = true;

        let text_str = if settings.enabled {
            "Hide Barycenter (C)"
        } else {
            "Show Barycenter (C)"
        };

        for children in button_children_query.iter_mut() {
            for child in children {
                if let Ok(mut text) = text_query.get_mut(*child) {
                    *text = Text::new(text_str.to_string());
                    break;
                }
            }
        }
    }
}
