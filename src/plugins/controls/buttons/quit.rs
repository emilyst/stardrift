//! Quit button component (non-WASM platforms only)

use crate::plugins::controls::ButtonWithLabel;
use crate::prelude::*;
use bevy::prelude::*;

#[derive(Component, Default)]
pub struct QuitButton;

impl ButtonWithLabel for QuitButton {
    fn command() -> SimulationCommand {
        SimulationCommand::Quit
    }

    fn marker() -> Self {
        Self
    }

    fn base_text() -> &'static str {
        "Quit"
    }

    fn shortcut() -> &'static str {
        "Q"
    }
}
