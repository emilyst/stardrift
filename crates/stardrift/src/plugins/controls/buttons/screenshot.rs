//! Screenshot button component

use crate::plugins::controls::ButtonWithLabel;
use crate::prelude::*;
use bevy::prelude::*;

#[derive(Component, Default)]
pub struct ScreenshotButton;

impl ButtonWithLabel for ScreenshotButton {
    fn command() -> SimulationCommand {
        SimulationCommand::TakeScreenshot
    }

    fn marker() -> Self {
        Self
    }

    fn base_text() -> &'static str {
        "Screenshot"
    }

    fn shortcut() -> &'static str {
        "S"
    }
}
