//! Numerical integration methods for n-body simulation

use crate::physics::components::KinematicHistory;
use crate::physics::math::{Scalar, Vector};
use std::any::Any;

pub mod symplectic_euler;

pub use symplectic_euler::SymplecticEuler;

/// Base trait for all integrators
pub trait Integrator: Send + Sync {
    /// Advance a single body's state by one time step
    ///
    /// This is the simplest form of integration, used when no history is available.
    /// Multi-step integrators should implement a reasonable fallback here
    /// (typically first-order Euler).
    ///
    /// # Arguments
    /// * `position` - Mutable reference to position
    /// * `velocity` - Mutable reference to velocity vector
    /// * `acceleration` - Current acceleration
    /// * `dt` - Time step
    fn step(&self, position: &mut Vector, velocity: &mut Vector, acceleration: Vector, dt: Scalar);

    /// Get the name of this integrator
    fn name(&self) -> &str;

    /// Get the order of this integrator
    fn order(&self) -> usize;

    /// Get self as Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Extended trait for integrators that can use historical states
///
/// Multi-step integrators implement this trait to provide enhanced
/// accuracy when historical data is available. They must also implement
/// the base `Integrator` trait with a fallback method for when history
/// is not available.
pub trait MultiStepIntegrator: Integrator {
    /// Advance state using historical data for improved accuracy
    ///
    /// # Arguments
    /// * `position` - Mutable reference to position
    /// * `velocity` - Mutable reference to velocity vector
    /// * `acceleration` - Current acceleration
    /// * `dt` - Time step
    /// * `history` - Historical states for multi-step integration
    fn step_with_history(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        acceleration: Vector,
        dt: Scalar,
        history: &KinematicHistory,
    );

    /// Minimum number of historical states needed for full accuracy
    fn required_history_size(&self) -> usize;
}
