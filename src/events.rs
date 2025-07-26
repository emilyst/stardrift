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
