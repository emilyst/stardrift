//! Numerical integration methods for n-body simulation

use crate::physics::math::{Scalar, Vector};

pub mod heun;
pub mod pefrl;
pub mod registry;
pub mod runge_kutta;
pub mod symplectic_euler;
pub mod velocity_verlet;

pub use heun::Heun;
pub use pefrl::Pefrl;
pub use runge_kutta::RungeKuttaFourthOrder;
pub use runge_kutta::RungeKuttaSecondOrderMidpoint;
pub use symplectic_euler::SymplecticEuler;
pub use velocity_verlet::VelocityVerlet;

/// Force evaluator trait for calculating forces at arbitrary positions
///
/// This trait allows integrators to evaluate forces at intermediate positions
/// during multi-stage integration methods (e.g., RK4, Velocity Verlet).
/// The evaluator is passed to integrators to enable accurate force calculations
/// without rebuilding the octree at each intermediate step.
pub trait ForceEvaluator: Send + Sync {
    /// Calculate acceleration at a given position
    ///
    /// # Arguments
    /// * `position` - The position at which to evaluate the force
    ///
    /// # Returns
    /// The acceleration vector at the given position
    fn calc_acceleration(&self, position: Vector) -> Vector;
}

/// Base trait for all integrators with capability discovery
pub trait Integrator: Send + Sync {
    /// Advance a single body's state by one time step using a force evaluator
    ///
    /// This method calculates forces at intermediate positions as needed for
    /// accurate multi-stage integration methods.
    ///
    /// # Arguments
    /// * `position` - Mutable reference to position
    /// * `velocity` - Mutable reference to velocity vector
    /// * `evaluator` - Force evaluator for calculating acceleration at arbitrary positions
    /// * `dt` - Time step
    fn step(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        evaluator: &dyn ForceEvaluator,
        dt: Scalar,
    );
}
