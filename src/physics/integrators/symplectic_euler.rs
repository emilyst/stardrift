//! Symplectic Euler integration method
//!
//! The simplest symplectic integrator, providing a foundation for understanding
//! how symplectic methods preserve phase space structure. Despite its first-order
//! accuracy, it often outperforms higher-order non-symplectic methods in
//! long-term energy conservation.

use super::{AccelerationField, Integrator};
use crate::physics::math::{Scalar, Vector};

/// Symplectic Euler integrator (also known as semi-implicit Euler)
///
/// The simplest member of the symplectic integrator family, this first-order
/// method achieves phase space preservation through a velocity-first update
/// scheme. Despite its low order, it provides bounded energy oscillations
/// that make it superior to explicit Euler for conservative systems.
///
/// # Algorithm
///
/// The key insight is updating velocity before position:
///
/// ```text
/// Stage 1: Velocity update using current position
///   a(t) = F(x(t))/m
///   v(t+dt) = v(t) + a(t) * dt
///
/// Stage 2: Position update using NEW velocity
///   x(t+dt) = x(t) + v(t+dt) * dt
/// ```
///
/// This ordering (velocity-first) is crucial for symplecticity. The reverse
/// ordering (position-first) gives explicit Euler, which is NOT symplectic.
///
/// # Mathematical Properties
///
/// - **Order of accuracy**: O(dt) local truncation error
/// - **Symplectic**: Preserves phase space volume (det J = 1)
/// - **Force evaluations**: 1 per timestep
/// - **Stability**: Conditionally stable
/// - **Time-reversible**: No (but symplectic structure preserved)
///
/// # Symplectic Structure
///
/// The transformation can be written as a composition of shear maps:
/// ```text
/// (x, v) → (x, v + a(x)*dt) → (x + v*dt, v)
/// ```
///
/// Each shear has Jacobian determinant = 1, preserving phase space volume.
/// This ensures Liouville's theorem is satisfied and the symplectic 2-form
/// ω = dp ∧ dq is preserved.
///
/// # Energy Behavior
///
/// - **Energy conservation**: Bounded oscillations, no secular drift
/// - **Modified Hamiltonian**: Conserves H̃ = H + O(dt)
/// - **Error magnitude**: Larger than Velocity Verlet but still bounded
/// - **Long-term stability**: Suitable for million-step simulations
///
/// Unlike explicit Euler which exhibits exponential energy drift, symplectic
/// Euler maintains bounded energy error indefinitely.
///
/// # Computational Cost
///
/// Minimal computational requirements:
/// - 1 force evaluation per timestep
/// - No intermediate storage needed
/// - Simplest possible implementation
/// - Same cost as explicit Euler but with conservation properties
///
/// # Comparison with Other Methods
///
/// | Property      | Symplectic Euler | Explicit Euler | Velocity Verlet | RK4      |
/// |---------------|------------------|----------------|-----------------|----------|
/// | Order         | 1                | 1              | 2               | 4        |
/// | Force evals   | 1                | 1              | 1               | 4        |
/// | Symplectic    | Yes              | No             | Yes             | No       |
/// | Energy drift  | Bounded          | Exponential    | Bounded         | Linear   |
/// | Complexity    | Minimal          | Minimal        | Low             | Moderate |
///
/// # Use Cases
///
/// **Ideal for:**
/// - Educational demonstrations of symplectic integration
/// - Quick prototyping and testing
/// - Systems where speed matters more than accuracy
/// - Real-time simulations with tight performance constraints
/// - Establishing baseline energy conservation behavior
///
/// **Consider alternatives:**
/// - Use Velocity Verlet for better accuracy, same cost
/// - Use PEFRL for high-precision orbital mechanics
/// - Use explicit Euler only for dissipative systems
///
/// # Historical Note
///
/// Also called semi-implicit Euler or Euler-Cromer method. The symplectic
/// property was recognized later, leading to its adoption in computational
/// physics despite its low order of accuracy.
#[derive(Debug, Copy, Clone, Default)]
pub struct SymplecticEuler;

impl Integrator for SymplecticEuler {
    fn clone_box(&self) -> Box<dyn Integrator> {
        Box::new(*self)
    }

    fn step(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        field: &dyn AccelerationField,
        dt: Scalar,
    ) {
        // Calculate acceleration at current position
        let acceleration = field.at(*position);

        // Update velocity first: v(t+dt) = v(t) + a(t) * dt
        *velocity += acceleration * dt;

        // Then update position using new velocity: x(t+dt) = x(t) + v(t+dt) * dt
        *position += *velocity * dt;
    }

    fn convergence_order(&self) -> usize {
        1
    }

    fn name(&self) -> &'static str {
        "symplectic_euler"
    }

    fn aliases(&self) -> Vec<&'static str> {
        vec!["euler", "semi_implicit_euler"]
    }
}
