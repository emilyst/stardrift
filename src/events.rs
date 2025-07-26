//! Centralized event definitions
//!
//! All events in the stardrift application are defined in this module to maintain
//! clear boundaries between systems and improve discoverability. Events are the
//! primary mechanism for cross-system communication in the ECS architecture.
//!
//! Events are organized by category:
//! - Simulation control events
//! - Visualization toggle events  
//! - UI update events

use bevy::prelude::*;
use std::marker::PhantomData;

// Simulation control events
#[derive(Event)]
pub struct RestartSimulationEvent;

#[derive(Event)]
pub struct TogglePauseSimulationEvent;

// Visualization toggle events
#[derive(Event)]
pub struct ToggleOctreeVisualizationEvent;

#[derive(Event)]
pub struct ToggleBarycenterGizmoVisibilityEvent;

// Screenshot event
#[derive(Event)]
pub struct TakeScreenshotEvent;

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
