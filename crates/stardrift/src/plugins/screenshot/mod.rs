//! Screenshot plugin
//!
//! This plugin provides both manual and automated screenshot capture functionality.
//! Manual screenshots hide UI elements for clean captures, while automated screenshots
//! preserve UI visibility for validation purposes.

use crate::prelude::*;

pub mod automated;
pub mod manual;

pub use automated::{AutomatedScreenshotNaming, AutomatedScreenshotSchedule};
pub use manual::ScreenshotState;

use automated::process_automated_screenshots;
use manual::{handle_take_screenshot_event, process_screenshot_capture};

pub struct ScreenshotPlugin;

impl Plugin for ScreenshotPlugin {
    fn build(&self, app: &mut App) {
        // Initialize resources
        app.init_resource::<ScreenshotState>();

        // Manual screenshot systems
        app.add_systems(
            Update,
            (handle_take_screenshot_event, process_screenshot_capture).chain(),
        );

        // Automated screenshot system - runs only if resource exists
        app.add_systems(
            Update,
            process_automated_screenshots.run_if(resource_exists::<AutomatedScreenshotSchedule>),
        );
    }
}
