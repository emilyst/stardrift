//! Explicit Euler integration method (forward Euler)
//!
//! WARNING: This integrator is provided primarily for educational and comparison purposes.
//! It exhibits poor energy conservation in conservative systems, with energy typically
//! drifting exponentially over time.

use super::{ForceEvaluator, Integrator};
use crate::physics::math::{Scalar, Vector};

/// Explicit Euler integrator (forward Euler method)
///
/// This is the simplest numerical integration method, updating position
/// before velocity using the current state values. While computationally
/// efficient, it does not preserve the symplectic structure of Hamiltonian
/// systems and exhibits poor long-term energy behavior.
///
/// # Algorithm
///
/// ```text
/// x(t+dt) = x(t) + v(t) * dt
/// v(t+dt) = v(t) + a(x(t)) * dt
/// ```
///
/// # Energy Behavior
///
/// - **Non-symplectic**: Does not preserve phase space volume
/// - **Energy drift**: Exhibits secular (unbounded) energy drift
/// - **Orbital decay/growth**: Stable orbits spiral inward or outward
/// - **Not recommended for**: Long-duration simulations, orbital mechanics
///
/// # Use Cases
///
/// - Educational demonstrations of integration methods
/// - Comparison benchmarks
/// - Short-duration simulations where energy conservation is not critical
/// - Systems with natural dissipation where energy loss is expected
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
}
