//! Velocity Verlet integration method

use super::{ForceEvaluator, Integrator};
use crate::physics::math::{Scalar, Vector};

/// Velocity Verlet integrator
///
/// A second-order symplectic integrator that provides excellent energy conservation
/// for Hamiltonian systems like gravitational n-body simulations. This implementation
/// uses the standard Velocity Verlet algorithm with force recalculation.
///
/// The algorithm:
/// 1. Calculate acceleration at current position: a(t)
/// 2. Update position: x(t+dt) = x(t) + v(t)*dt + 0.5*a(t)*dt²
/// 3. Calculate acceleration at new position: a(t+dt)
/// 4. Update velocity: v(t+dt) = v(t) + 0.5*(a(t) + a(t+dt))*dt
#[derive(Debug, Clone, Default)]
pub struct VelocityVerlet;

impl Integrator for VelocityVerlet {
    fn step(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        evaluator: &dyn ForceEvaluator,
        dt: Scalar,
    ) {
        // Proper Velocity Verlet with force recalculation
        // This is the mathematically correct implementation that conserves energy

        // Calculate acceleration at current position
        let accel_old = evaluator.calc_acceleration(*position);

        // Update position using current velocity and acceleration
        // x(t+dt) = x(t) + v(t)*dt + 0.5*a(t)*dt²
        *position += *velocity * dt + accel_old * (0.5 * dt * dt);

        // Calculate acceleration at new position
        let accel_new = evaluator.calc_acceleration(*position);

        // Update velocity using average of old and new acceleration
        // v(t+dt) = v(t) + 0.5*(a(t) + a(t+dt))*dt
        *velocity += (accel_old + accel_new) * (0.5 * dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::math::Vector;

    #[test]
    fn test_velocity_verlet_simple_step() {
        let integrator = VelocityVerlet;

        let mut position = Vector::new(1.0, 0.0, 0.0);
        let mut velocity = Vector::new(0.0, 1.0, 0.0);
        let dt = 0.01;

        // Test evaluator
        struct TestEvaluator;
        impl ForceEvaluator for TestEvaluator {
            fn calc_acceleration(&self, _position: Vector) -> Vector {
                Vector::new(0.0, 0.0, -9.81)
            }
        }
        let evaluator = TestEvaluator;

        integrator.step(&mut position, &mut velocity, &evaluator, dt);

        // Position should be updated with velocity and half acceleration
        let expected_x = 1.0;
        let expected_y = 0.01; // v*dt = 1.0 * 0.01
        let expected_z = -0.00049; // 0.5*a*dt² = 0.5 * -9.81 * 0.01²

        assert!((position.x - expected_x).abs() < 1e-6);
        assert!((position.y - expected_y).abs() < 1e-6);
        assert!((position.z - expected_z).abs() < 1e-6);

        // Velocity should be updated with acceleration
        assert_eq!(velocity, Vector::new(0.0, 1.0, -0.0981));
    }

    #[test]
    fn test_energy_conservation() {
        // Test with a simple harmonic oscillator to verify energy conservation
        let integrator = VelocityVerlet;

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

        // Simulate for many steps
        for _ in 0..1000 {
            // Integrate
            integrator.step(&mut position, &mut velocity, &evaluator, dt);
        }

        // Calculate final energy
        let final_energy = 0.5 * velocity.length_squared() + 0.5 * k * position.length_squared();

        // Energy should be conserved to within numerical precision
        // With proper Velocity Verlet, energy conservation is excellent
        let energy_error = (final_energy - initial_energy).abs() / initial_energy;
        assert!(energy_error < 0.001, "Energy error: {}", energy_error);
    }
}
