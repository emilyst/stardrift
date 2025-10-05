//! Centralized message definitions
//!
//! All messages in Stardrift are defined in this module to maintain
//! clear boundaries between systems and improve discoverability. Messages are the
//! primary mechanism for cross-system communication in the ECS architecture.

use bevy::prelude::*;

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationCommand {
    Restart,
    TogglePause,
    ToggleOctreeVisualization,
    ToggleBarycenterGizmo,
    ToggleTrailsVisualization,
    ToggleDiagnosticsHud,
    TakeScreenshot,
    #[cfg(not(target_arch = "wasm32"))]
    Quit,
}
