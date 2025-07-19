//! Stardrift prelude module
//!
//! This module re-exports the most commonly used types, traits, and functions
//! across the Stardrift application to reduce import boilerplate.

// External crate re-exports
pub use avian3d::math::{Scalar, Vector};
pub use avian3d::prelude::*;
pub use bevy::prelude::*;
pub use rand::Rng;

// Internal re-exports - Config
pub use crate::config::SimulationConfig;

// Internal re-exports - States
pub use crate::states::{AppState, LoadingState};

// Internal re-exports - Resources (most commonly used)
pub use crate::resources::{
    Barycenter, BarycenterGizmoVisibility, BodyCount, GravitationalConstant, GravitationalOctree,
    OctreeVisualizationSettings, SharedRng,
};

// Internal re-exports - Components
pub use crate::components::BodyBundle;

// Internal re-exports - Physics
pub use crate::physics::octree::{Octree, OctreeBody};

// Internal re-exports - Simulation Action Events
pub use crate::systems::simulation_actions::{
    RestartSimulationEvent, ToggleBarycenterGizmoVisibilityEvent, ToggleOctreeVisualizationEvent,
    TogglePauseSimulationEvent,
};

// Note: Utility functions are kept crate-private and not re-exported in prelude
