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

pub fn initialize_octree_button_text(
    settings: Res<OctreeVisualizationSettings>,
    mut query: Query<(&OctreeToggleButton, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    let text_str = if settings.enabled {
        "Hide Octree (O)"
    } else {
        "Show Octree (O)"
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

pub fn update_octree_button_text(
    settings: Res<OctreeVisualizationSettings>,
    mut query: Query<(&OctreeToggleButton, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    if settings.is_changed() {
        let text_str = if settings.enabled {
            "Hide Octree (O)"
        } else {
            "Show Octree (O)"
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
