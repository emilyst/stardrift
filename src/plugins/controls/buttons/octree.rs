//! Octree visualization toggle button component

use crate::plugins::controls::ButtonWithLabel;
use crate::prelude::*;
use bevy::prelude::*;

#[derive(Component, Default)]
pub struct OctreeToggleButton;

impl ButtonWithLabel for OctreeToggleButton {
    fn command() -> SimulationCommand {
        SimulationCommand::ToggleOctreeVisualization
    }

    fn marker() -> Self {
        Self
    }

    fn base_text() -> &'static str {
        "Show Octree"
    }

    fn shortcut() -> &'static str {
        "O"
    }
}

pub fn sync_octree_button_text(
    settings: Res<OctreeVisualizationSettings>,
    mut initialized: Local<bool>,
    button_children_query: Query<&Children, With<OctreeToggleButton>>,
    mut text_query: Query<&mut Text>,
) {
    if !*initialized || settings.is_changed() {
        *initialized = true;

        let text_str = if settings.enabled {
            "Hide Octree (O)"
        } else {
            "Show Octree (O)"
        };

        for children in button_children_query.iter() {
            for child in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    *text = Text::new(text_str.to_string());
                    break;
                }
            }
        }
    }
}
