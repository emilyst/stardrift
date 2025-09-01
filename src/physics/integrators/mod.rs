//! Numerical integration methods for n-body simulation
//!
//! This module provides various numerical integrators for solving the equations
//! of motion in gravitational n-body simulations. Each integrator implements
//! the `Integrator` trait, which requires self-description of its properties
//! (name, aliases, convergence order) and the core integration step.

use crate::physics::math::{Scalar, Vector};

pub mod explicit_euler;
pub mod heun;
pub mod pefrl;
pub mod registry;
pub mod runge_kutta;
pub mod symplectic_euler;
pub mod velocity_verlet;

pub use explicit_euler::ExplicitEuler;
pub use heun::Heun;
pub use pefrl::Pefrl;
pub use runge_kutta::RungeKuttaFourthOrder;
pub use runge_kutta::RungeKuttaSecondOrderMidpoint;
pub use symplectic_euler::SymplecticEuler;
pub use velocity_verlet::VelocityVerlet;

/// Acceleration field trait for calculating accelerations at arbitrary positions
///
/// This trait allows integrators to evaluate accelerations at intermediate positions
/// during multi-stage integration methods (e.g., RK4, Velocity Verlet).
/// The field is passed to integrators to enable accurate acceleration calculations
/// without rebuilding the octree at each intermediate step.
pub trait AccelerationField: Send + Sync {
    /// Calculate acceleration at a given position
    ///
    /// # Arguments
    /// * `position` - The position at which to evaluate the acceleration
    ///
    /// # Returns
    /// The acceleration vector at the given position
    fn at(&self, position: Vector) -> Vector;
}

/// Base trait for all integrators with capability discovery
///
/// Integrators are self-describing, providing their name, aliases, and
/// mathematical properties. This enables the registry to discover and
/// manage integrators without hardcoded knowledge.
pub trait Integrator: Send + Sync {
    /// Create a boxed clone of this integrator
    ///
    /// This enables the registry to create new instances without knowing
    /// the concrete type, supporting true generic discovery.
    fn clone_box(&self) -> Box<dyn Integrator>;

    /// Advance a single body's state by one time step using an acceleration field
    ///
    /// This method calculates accelerations at intermediate positions as needed for
    /// accurate multi-stage integration methods.
    ///
    /// # Arguments
    /// * `position` - Mutable reference to position
    /// * `velocity` - Mutable reference to velocity vector
    /// * `field` - Acceleration field for calculating acceleration at arbitrary positions
    /// * `dt` - Time step
    fn step(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        field: &dyn AccelerationField,
        dt: Scalar,
    );

    /// Returns the convergence order of this integration method
    ///
    /// The convergence order indicates how the error scales with timestep:
    /// - Order 1: Error ~ O(dt)
    /// - Order 2: Error ~ O(dt²)
    /// - Order 4: Error ~ O(dt⁴)
    fn convergence_order(&self) -> usize;

    /// Returns the canonical name of this integrator
    ///
    /// This is the primary identifier used in configuration files
    fn name(&self) -> &'static str;

    /// Returns alternative names/aliases for this integrator
    ///
    /// These provide convenient shortcuts for users
    fn aliases(&self) -> Vec<&'static str> {
        Vec::new() // Default: no aliases
    }
}
