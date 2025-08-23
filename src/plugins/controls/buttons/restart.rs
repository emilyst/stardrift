//! Restart simulation button component

use crate::plugins::controls::ButtonWithLabel;
use crate::prelude::*;
use bevy::prelude::*;

#[derive(Component, Default)]
pub struct RestartSimulationButton;

impl ButtonWithLabel for RestartSimulationButton {
    fn command() -> SimulationCommand {
        SimulationCommand::Restart
    }

    fn marker() -> Self {
        Self
    }

    fn base_text() -> &'static str {
        "New Simulation"
    }

    fn shortcut() -> &'static str {
        "N"
    }
}
