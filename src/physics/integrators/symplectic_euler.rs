//! Symplectic Euler integration method

use super::{ForceEvaluator, Integrator};
use crate::physics::math::{Scalar, Vector};

/// Symplectic Euler integrator (also known as semi-implicit Euler)
///
/// This is a first-order symplectic integrator that updates velocities
/// before positions, preserving the symplectic structure of Hamiltonian systems
/// and providing better energy conservation than explicit Euler.
#[derive(Debug, Clone, Default)]
pub struct SymplecticEuler;

impl Integrator for SymplecticEuler {
    fn step(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        evaluator: &dyn ForceEvaluator,
        dt: Scalar,
    ) {
        // Calculate acceleration at current position
        let acceleration = evaluator.calc_acceleration(*position);

        // Update velocity first: v(t+dt) = v(t) + a(t) * dt
        *velocity += acceleration * dt;

        // Then update position using new velocity: x(t+dt) = x(t) + v(t+dt) * dt
        *position += *velocity * dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::math::Vector;

    #[test]
    fn test_symplectic_euler_integrate_single() {
        // Simple test evaluator that returns constant acceleration
        struct TestEvaluator;
        impl ForceEvaluator for TestEvaluator {
            fn calc_acceleration(&self, _position: Vector) -> Vector {
                Vector::new(0.0, 0.0, -9.81)
            }
        }

        let integrator = SymplecticEuler;
        let evaluator = TestEvaluator;

        let mut position = Vector::new(1.0, 0.0, 0.0);
        let mut velocity = Vector::new(0.0, 1.0, 0.0);
        let dt = 0.01;

        integrator.step(&mut position, &mut velocity, &evaluator, dt);

        // Velocity should be updated first
        assert_eq!(velocity, Vector::new(0.0, 1.0, -0.0981));

        // Position should use the new velocity
        let expected_position = Vector::new(1.0, 0.01, -0.000981);
        assert!((position - expected_position).length() < 1e-6);
    }
}
