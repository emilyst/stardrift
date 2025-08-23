//! Centralized event definitions
//!
//! All events in the stardrift application are defined in this module to maintain
//! clear boundaries between systems and improve discoverability. Events are the
//! primary mechanism for cross-system communication in the ECS architecture.

use bevy::prelude::*;

#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationCommand {
    Restart,
    TogglePause,
    ToggleOctreeVisualization,
    ToggleBarycenterGizmo,
    ToggleTrailsVisualization,
    ToggleDiagnosticsHud,
    TakeScreenshot,
    SetOctreeMaxDepth(Option<u8>),
    #[cfg(not(target_arch = "wasm32"))]
    Quit,
}
