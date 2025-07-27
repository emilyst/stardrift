//! Centralized event definitions
//!
//! All events in the stardrift application are defined in this module to maintain
//! clear boundaries between systems and improve discoverability. Events are the
//! primary mechanism for cross-system communication in the ECS architecture.
//!
//! Events are organized by category:
//! - Simulation command pattern
//! - UI update events

use bevy::prelude::*;
use std::marker::PhantomData;

// Unified simulation command pattern
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationCommand {
    Restart,
    TogglePause,
    ToggleOctreeVisualization,
    ToggleBarycenterGizmo,
    TakeScreenshot,
    Quit,
    SetOctreeMaxDepth(Option<u8>),
}

// UI update events
#[derive(Event)]
pub struct UpdateButtonTextEvent<T: Component> {
    pub new_text: String,
    _marker: PhantomData<T>,
}

impl<T: Component> UpdateButtonTextEvent<T> {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            new_text: text.into(),
            _marker: PhantomData,
        }
    }
}
