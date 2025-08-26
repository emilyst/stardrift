//! Explicit Euler integration method (forward Euler)
//!
//! WARNING: This integrator is provided primarily for educational and comparison purposes.
//! It exhibits poor energy conservation in conservative systems, with energy typically
//! drifting exponentially over time.

use super::{ForceEvaluator, Integrator};
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
/// | Use case      | Educational    | Simple problems  | Short sims  | Production      |
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
/// - Production physics simulations
/// - Long-duration integrations
///
/// # Historical Note
///
/// Named after Leonhard Euler (1768), this was the first numerical method for
/// differential equations. Its limitations led to development of improved methods,
/// particularly symplectic integrators for Hamiltonian systems.
#[derive(Debug, Clone, Default)]
pub struct ExplicitEuler;

impl Integrator for ExplicitEuler {
    fn step(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        evaluator: &dyn ForceEvaluator,
        dt: Scalar,
    ) {
        // Store the current velocity for position update
        let current_velocity = *velocity;

        // Calculate acceleration at current position
        let acceleration = evaluator.calc_acceleration(*position);

        // Update position first using CURRENT velocity: x(t+dt) = x(t) + v(t) * dt
        *position += current_velocity * dt;

        // Then update velocity: v(t+dt) = v(t) + a(t) * dt
        *velocity += acceleration * dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::math::Vector;

    #[test]
    fn test_explicit_euler_integrate_single() {
        // Simple test evaluator that returns constant acceleration (gravity)
        struct TestEvaluator;
        impl ForceEvaluator for TestEvaluator {
            fn calc_acceleration(&self, _position: Vector) -> Vector {
                Vector::new(0.0, 0.0, -9.81)
            }
        }

        let integrator = ExplicitEuler;
        let evaluator = TestEvaluator;

        let mut position = Vector::new(1.0, 0.0, 0.0);
        let mut velocity = Vector::new(0.0, 1.0, 0.0);
        let dt = 0.01;

        integrator.step(&mut position, &mut velocity, &evaluator, dt);

        // Position should be updated with OLD velocity
        let expected_position = Vector::new(1.0, 0.01, 0.0);
        assert!((position - expected_position).length() < 1e-6);

        // Velocity should be updated after position
        assert_eq!(velocity, Vector::new(0.0, 1.0, -0.0981));
    }

    #[test]
    fn test_explicit_euler_order_of_operations() {
        // Test evaluator that makes acceleration depend on position
        // This helps verify the correct order of operations
        struct PositionDependentEvaluator;
        impl ForceEvaluator for PositionDependentEvaluator {
            fn calc_acceleration(&self, position: Vector) -> Vector {
                // Simple spring force: a = -k * x (with k=1 for simplicity)
                -position
            }
        }

        let integrator = ExplicitEuler;
        let evaluator = PositionDependentEvaluator;

        let mut position = Vector::new(1.0, 0.0, 0.0);
        let mut velocity = Vector::new(0.0, 0.0, 0.0);
        let dt = 0.1;

        integrator.step(&mut position, &mut velocity, &evaluator, dt);

        // Position uses OLD velocity (which was zero)
        assert_eq!(position, Vector::new(1.0, 0.0, 0.0));

        // Velocity uses acceleration from ORIGINAL position
        // a = -[1, 0, 0], so v = [0, 0, 0] + [-1, 0, 0] * 0.1 = [-0.1, 0, 0]
        assert_eq!(velocity, Vector::new(-0.1, 0.0, 0.0));
    }

    #[test]
    fn test_explicit_euler_exponential_energy_drift() {
        // Test that explicit Euler exhibits exponential energy drift
        let integrator = ExplicitEuler;

        let mut position = Vector::new(1.0, 0.0, 0.0);
        let mut velocity = Vector::new(0.0, 0.0, 0.0);
        let k = 1.0; // Spring constant
        let dt = 0.01;

        let initial_energy = 0.5 * k * position.length_squared();

        // Spring force evaluator
        struct SpringEvaluator {
            k: Scalar,
        }
        impl ForceEvaluator for SpringEvaluator {
            fn calc_acceleration(&self, position: Vector) -> Vector {
                position * (-self.k)
            }
        }
        let evaluator = SpringEvaluator { k };

        // Track energy growth
        let mut previous_energy = initial_energy;
        let mut drift_increasing = true;

        // Simulate and check for exponential growth pattern
        for i in 1..=1000 {
            integrator.step(&mut position, &mut velocity, &evaluator, dt);

            if i % 100 == 0 {
                let current_energy =
                    0.5 * velocity.length_squared() + 0.5 * k * position.length_squared();
                let current_drift = (current_energy - initial_energy).abs();
                let previous_drift = (previous_energy - initial_energy).abs();

                // Drift should be growing (exponential pattern)
                if current_drift <= previous_drift {
                    drift_increasing = false;
                }

                previous_energy = current_energy;
            }
        }

        let final_energy = 0.5 * velocity.length_squared() + 0.5 * k * position.length_squared();
        let energy_error = (final_energy - initial_energy).abs() / initial_energy;

        // Explicit Euler should show significant drift
        assert!(
            energy_error > 0.1, // More than 10% error expected
            "Explicit Euler should exhibit large energy drift. Error: {}",
            energy_error
        );

        // The drift should generally be increasing (exponential)
        assert!(
            drift_increasing,
            "Energy drift should show exponential growth pattern"
        );
    }
}
