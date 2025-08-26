//! Heun's method (Improved Euler) integration
//!
//! A classical second-order predictor-corrector method that achieves improved
//! accuracy over Euler methods through slope averaging. While not symplectic,
//! Heun's method provides a good balance of simplicity and accuracy for
//! short-duration simulations where energy conservation is not critical.

use super::{ForceEvaluator, Integrator};
use crate::physics::math::{Scalar, Vector};

/// Heun's method (Improved Euler method)
///
/// A second-order accurate predictor-corrector method that improves upon
/// basic Euler by averaging slopes at the beginning and predicted endpoint
/// of each timestep. This reduces local truncation error from O(dt) to O(dt²).
///
/// # Algorithm
///
/// The method uses a two-stage approach:
///
/// ```text
/// Stage 1 (Predictor):
///   k1_x = v(t)
///   k1_v = a(x(t))
///
/// Stage 2 (Evaluate at predicted point):
///   x_pred = x(t) + k1_x * dt
///   v_pred = v(t) + k1_v * dt
///   k2_x = v_pred
///   k2_v = a(x_pred)
///
/// Final update (Average):
///   x(t+dt) = x(t) + (k1_x + k2_x) * dt/2
///   v(t+dt) = v(t) + (k1_v + k2_v) * dt/2
/// ```
///
/// # Energy Behavior
///
/// - **Non-symplectic**: Does not preserve phase space volume
/// - **Energy drift**: Exhibits gradual energy drift in conservative systems
/// - **Stability**: Conditionally stable with larger stability region than basic Euler
/// - **Accuracy**: Second-order accurate O(dt²) for smooth problems
///
/// # Computational Cost
///
/// - Requires 2 force evaluations per timestep
/// - More expensive than symplectic Euler (1 evaluation) but more accurate
/// - Less expensive than RK4 (4 evaluations) but lower accuracy
///
/// # Mathematical Properties
///
/// - **Order of accuracy**: O(dt²) local truncation error
/// - **Force evaluations**: 2 per timestep
/// - **Stability**: Conditionally stable (larger region than Euler)
/// - **Time-reversible**: No
/// - **Symplectic**: No - does not preserve phase space volume
///
/// # Comparison with Other Methods
///
/// | Property      | Heun                | RK2 Midpoint | Velocity Verlet | Symplectic Euler |
/// |---------------|---------------------|--------------|-----------------|------------------|
/// | Order         | 2                   | 2            | 2               | 1                |
/// | Force evals   | 2                   | 2            | 1               | 1                |
/// | Symplectic    | No                  | No           | Yes             | Yes              |
/// | Energy drift  | Linear              | Linear       | Bounded         | Bounded          |
/// | Method type   | Predictor-corrector | Midpoint     | Symplectic      | Symplectic       |
///
/// # Use Cases
///
/// **Ideal for:**
/// - General-purpose integration where moderate accuracy suffices
/// - Systems with natural dissipation where energy drift is acceptable
/// - Short to medium duration simulations
/// - Educational demonstrations of predictor-corrector concepts
///
/// **Consider alternatives:**
/// - Use Velocity Verlet for energy conservation with similar order
/// - Use RK4 for higher accuracy in short simulations
/// - Use symplectic methods for long-term conservative dynamics
///
/// # Historical Note
///
/// Named after Karl Heun (1900), also known as the modified Euler method,
/// improved Euler method, or explicit trapezoidal rule (distinct from the
/// implicit trapezoidal rule which IS symplectic).
#[derive(Debug, Clone, Copy, Default)]
pub struct Heun;

impl Integrator for Heun {
    fn step(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        evaluator: &dyn ForceEvaluator,
        dt: Scalar,
    ) {
        // Proper Heun's method with force evaluation
        // This is a predictor-corrector method that achieves 2nd order accuracy

        // Stage 1: Evaluate at current position (predictor)
        let k1_x = *velocity;
        let k1_v = evaluator.calc_acceleration(*position);

        // Stage 2: Evaluate at predicted endpoint
        let pos_predicted = *position + k1_x * dt;
        let vel_predicted = *velocity + k1_v * dt;
        let k2_x = vel_predicted;
        let k2_v = evaluator.calc_acceleration(pos_predicted);

        // Average the slopes (corrector)
        *position += (k1_x + k2_x) * (dt * 0.5);
        *velocity += (k1_v + k2_v) * (dt * 0.5);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heun_basic_step() {
        let integrator = Heun;
        let mut position = Vector::new(0.0, 0.0, 0.0);
        let mut velocity = Vector::new(1.0, 0.0, 0.0);
        let dt = 0.01;

        // Test evaluator with constant acceleration
        struct TestEvaluator;
        impl ForceEvaluator for TestEvaluator {
            fn calc_acceleration(&self, _position: Vector) -> Vector {
                Vector::new(0.0, -9.81, 0.0)
            }
        }
        let evaluator = TestEvaluator;

        integrator.step(&mut position, &mut velocity, &evaluator, dt);

        // Verify movement occurred
        assert!(position.x > 0.0);
        assert!(velocity.y < 0.0);

        // With constant acceleration, Heun should give same velocity as Euler
        // but slightly different position due to averaging
        assert_eq!(velocity.y, -9.81 * dt);
    }

    #[test]
    fn test_heun_energy_drift() {
        // Test that Heun exhibits energy drift (non-symplectic behavior)
        let integrator = Heun;

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

        // Track energy over time
        let mut max_energy = initial_energy;
        let mut min_energy = initial_energy;

        // Simulate for many steps
        for _ in 0..5000 {
            integrator.step(&mut position, &mut velocity, &evaluator, dt);
            let energy = 0.5 * velocity.length_squared() + 0.5 * k * position.length_squared();
            max_energy = max_energy.max(energy);
            min_energy = min_energy.min(energy);
        }

        // For non-symplectic methods, we expect systematic drift
        // The energy doesn't stay bounded within a small range
        let final_energy = 0.5 * velocity.length_squared() + 0.5 * k * position.length_squared();

        // Check that energy has changed (either growth or decay)
        let final_error = (final_energy - initial_energy).abs() / initial_energy;

        // For Heun with dt=0.01 and 5000 steps, even small drift accumulates
        // The test verifies non-symplectic behavior (energy not conserved)
        assert!(
            final_error > 1e-5 || (max_energy - min_energy) / initial_energy > 1e-5,
            "Heun is non-symplectic and should not conserve energy perfectly. Error: {}, Range: {}",
            final_error,
            (max_energy - min_energy) / initial_energy
        );
    }

    #[test]
    fn test_heun_second_order_convergence() {
        // Verify second-order convergence
        let integrator = Heun;

        // Test with harmonic oscillator
        struct SpringEvaluator {
            k: Scalar,
        }
        impl ForceEvaluator for SpringEvaluator {
            fn calc_acceleration(&self, position: Vector) -> Vector {
                position * (-self.k)
            }
        }
        let evaluator = SpringEvaluator { k: 1.0 };

        // Test with two different timesteps
        let dt1 = 0.02;
        let dt2 = 0.01; // Half the timestep

        let initial_pos = Vector::new(1.0, 0.0, 0.0);
        let initial_vel = Vector::new(0.0, 0.0, 0.0);
        let final_time = 0.5; // Shorter time to avoid excessive drift

        // Integrate with dt1
        let mut pos1 = initial_pos;
        let mut vel1 = initial_vel;
        let steps1 = (final_time / dt1) as usize;
        for _ in 0..steps1 {
            integrator.step(&mut pos1, &mut vel1, &evaluator, dt1);
        }

        // Integrate with dt2
        let mut pos2 = initial_pos;
        let mut vel2 = initial_vel;
        let steps2 = (final_time / dt2) as usize;
        for _ in 0..steps2 {
            integrator.step(&mut pos2, &mut vel2, &evaluator, dt2);
        }

        // Analytical solution: x = cos(t), v = -sin(t)
        let exact_pos = Vector::new(final_time.cos(), 0.0, 0.0);

        let error1 = (pos1 - exact_pos).length();
        let error2 = (pos2 - exact_pos).length();

        // For second-order method, error should scale as O(dt²)
        // error2/error1 should be approximately (dt2/dt1)² = 0.25
        let error_ratio = error2 / error1;
        assert!(
            (error_ratio - 0.25).abs() < 0.1, // Some tolerance
            "Second-order convergence not satisfied. Error ratio: {}, expected ~0.25",
            error_ratio
        );
    }
}
