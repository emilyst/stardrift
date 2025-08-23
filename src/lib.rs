//! Stardrift library
//!
//! This provides the core functionality of stardrift as a library
//! to enable integration testing.

pub mod config;
pub mod events;
pub mod physics;
pub mod plugins;
pub mod prelude;
pub mod resources;
pub mod states;
pub mod utils;

#[cfg(test)]
pub mod test_utils;

// Re-export commonly used items
pub use config::SimulationConfig;
pub use events::*;
pub use physics::{
    components::{Mass, Position, Velocity},
    integrators,
    math::{Scalar, Vector},
};
pub use plugins::{
    camera::CameraPlugin, controls::ControlsPlugin, diagnostics_hud::DiagnosticsHudPlugin,
    simulation::SimulationPlugin, trails::TrailsPlugin, visualization::VisualizationPlugin,
};
pub use prelude::*;
pub use states::AppState;
