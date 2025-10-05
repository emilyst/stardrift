//! Stardrift prelude module
//!
//! This module re-exports the most commonly used types, traits, and functions
//! across the Stardrift application to reduce import boilerplate.

// External crate re-exports
pub use bevy::prelude::*;
pub use rand::prelude::*;

// Physics math types
pub use crate::physics::math::{Scalar, Vector};

// Internal re-exports - Config
pub use crate::config::SimulationConfig;

// Internal re-exports - States
pub use crate::states::AppState;

// Internal re-exports - Resources (most commonly used)
pub use crate::resources::{
    Barycenter, BodyCount, GravitationalConstant, GravitationalOctree, RenderingRng, SharedRng,
};

// Internal re-exports - Visualization
pub use crate::plugins::visualization::{
    BarycenterGizmoVisibility, OctreeVisualizationSettings, TrailsVisualizationSettings,
};

// Internal re-exports - Physics
pub use crate::physics::octree::Octree;

// Internal re-exports - Events
pub use crate::messages::SimulationCommand;

// Note: Utility functions are kept crate-private and not re-exported in prelude
