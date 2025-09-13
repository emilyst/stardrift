//! Explicit Euler integration method (forward Euler)
//!
//! WARNING: This integrator is provided primarily for educational and comparison purposes.
//! It exhibits poor energy conservation in conservative systems, with energy typically
//! drifting exponentially over time.

use super::{AccelerationField, Integrator};
use crate::physics::math::{Scalar, Vector};

/// Explicit Euler integrator (forward Euler method)
///
/// The simplest possible numerical integration method, updating position
/// before velocity using current state values. While computationally minimal,
/// it fails to preserve the symplectic structure of Hamiltonian systems and
/// exhibits exponential energy drift, making it unsuitable for conservative systems.
///
/// # Algorithm
///
/// The position-first update scheme that causes energy drift:
///
/// ```text
/// Stage 1: Position update using CURRENT velocity
///   x(t+dt) = x(t) + v(t) * dt
///
/// Stage 2: Velocity update using ORIGINAL position
///   a(t) = F(x(t))/m
///   v(t+dt) = v(t) + a(t) * dt
/// ```
///
/// Note the critical difference from symplectic Euler: position is updated
/// BEFORE velocity, using the old velocity value.
///
/// # Mathematical Properties
///
/// - **Order of accuracy**: O(dt) local truncation error
/// - **Force evaluations**: 1 per timestep
/// - **Stability**: Conditionally stable (small stability region)
/// - **Time-reversible**: No
/// - **Symplectic**: No - destroys phase space structure
///
/// # Energy Behavior
///
/// - **Non-symplectic**: Phase space volume not preserved (det J ≠ 1)
/// - **Energy drift**: Exponential growth or decay
/// - **Orbital behavior**: Stable orbits spiral outward or inward
/// - **Error growth**: Energy error ~ exp(λt) for some λ > 0
/// - **Catastrophic failure**: Systems become unphysical over time
///
/// The energy drift is not just poor accuracy - it's a systematic bias that
/// compounds exponentially, making long simulations meaningless.
///
/// # Computational Cost
///
/// Minimal computational requirements:
/// - 1 force evaluation per timestep
/// - Simplest possible implementation
/// - Same cost as symplectic Euler
/// - No advantage over symplectic Euler
///
/// # Comparison with Other Methods
///
/// | Property      | Explicit Euler | Symplectic Euler | Heun        | Velocity Verlet |
/// |---------------|----------------|------------------|-------------|-----------------|
/// | Order         | 1              | 1                | 2           | 2               |
/// | Force evals   | 1              | 1                | 2           | 1               |
/// | Symplectic    | No             | Yes              | No          | Yes             |
/// | Energy drift  | Exponential    | Bounded          | Linear      | Bounded         |
/// | Use case      | Educational    | Simple problems  | Short sims  | Long sims       |
///
/// # Use Cases
///
/// **Valid uses (limited):**
/// - Educational demonstrations showing importance of symplectic methods
/// - Comparison benchmarks to highlight energy drift
/// - Systems with strong dissipation where energy loss is physical
/// - Debugging and algorithm verification
///
/// **Never use for:**
/// - Orbital mechanics or celestial dynamics
/// - Molecular dynamics simulations
/// - Any conservative Hamiltonian system
/// - Scientific physics simulations
/// - Long-duration integrations
///
/// # Historical Note
///
/// Named after Leonhard Euler (1768), this was the first numerical method for
/// differential equations. Its limitations led to development of improved methods,
/// particularly symplectic integrators for Hamiltonian systems.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExplicitEuler;

impl Integrator for ExplicitEuler {
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
        // Store the current velocity for position update
        let current_velocity = *velocity;

        // Calculate acceleration at current position
        let acceleration = field.at(*position);

        // Update position first using CURRENT velocity: x(t+dt) = x(t) + v(t) * dt
        *position += current_velocity * dt;

        // Then update velocity: v(t+dt) = v(t) + a(t) * dt
        *velocity += acceleration * dt;
    }

    fn convergence_order(&self) -> usize {
        1
    }

    fn name(&self) -> &'static str {
        "explicit_euler"
    }

    fn aliases(&self) -> Vec<&'static str> {
        vec!["forward_euler"]
    }
}
