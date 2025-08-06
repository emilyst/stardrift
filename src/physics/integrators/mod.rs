//! Numerical integration methods for n-body simulation

use crate::physics::math::{Scalar, Vector};

pub mod semi_implicit_euler;

pub use semi_implicit_euler::SemiImplicitEuler;

pub trait SymplecticIntegrator: Send + Sync {
    /// Integrate a single body's state
    ///
    /// Updates the position and velocity of a body based on its current
    /// acceleration and the time step. This method is designed to work
    /// directly with ECS component data, avoiding unnecessary allocations.
    ///
    /// # Arguments
    /// * `position` - Mutable reference to position
    /// * `velocity` - Mutable reference to velocity vector
    /// * `acceleration` - Current acceleration
    /// * `dt` - Time step
    fn integrate_single(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        acceleration: Vector,
        dt: Scalar,
    );

    /// Get the name of this integrator
    fn name(&self) -> &str;

    /// Get the order of this integrator
    fn order(&self) -> usize;
}
