//! Position-Extended Forest-Ruth-Like (PEFRL) integration method

use super::{ForceEvaluator, Integrator};
use crate::physics::math::{Scalar, Vector};

/// PEFRL integrator - a 4th order symplectic integrator
///
/// This is a fourth-order symplectic integrator optimized for Hamiltonian systems.
/// It provides excellent long-term energy conservation while maintaining 4th order accuracy.
/// The coefficients have been optimized to minimize the leading error term.
///
/// The algorithm uses a symmetric composition of position and velocity updates:
/// 1. x += ξ * v * dt
/// 2. v += (1-2λ)/2 * a(x) * dt
/// 3. x += χ * v * dt
/// 4. v += λ * a(x) * dt
/// 5. x += (1-2(χ+ξ)) * v * dt
/// 6. v += λ * a(x) * dt
/// 7. x += χ * v * dt
/// 8. v += (1-2λ)/2 * a(x) * dt
/// 9. x += ξ * v * dt
///
/// Reference: Omelyan, Mryglod, Folk (2002) "Optimized Forest-Ruth- and Suzuki-like algorithms
/// for integration of motion in many-body systems"
#[derive(Debug, Clone, Default)]
pub struct Pefrl;

impl Pefrl {
    /// Optimized coefficients for minimal error
    const XI: Scalar = 0.178_617_895_844_809_1;
    const LAMBDA: Scalar = -0.212_341_831_062_605_4;
    const CHI: Scalar = -0.066_264_582_669_818_5;
}

impl Integrator for Pefrl {
    fn step(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        evaluator: &dyn ForceEvaluator,
        dt: Scalar,
    ) {
        // Precompute commonly used values
        const COEFF_A: Scalar = 0.5 * (1.0 - 2.0 * Pefrl::LAMBDA); // (1-2λ)/2
        let coeff_b = 1.0 - 2.0 * (Pefrl::CHI + Pefrl::XI); // 1-2(χ+ξ)

        // Stage 1: Position update
        *position += *velocity * (Pefrl::XI * dt);

        // Stage 2: Velocity update with first acceleration
        let accel_1 = evaluator.calc_acceleration(*position);
        *velocity += accel_1 * (COEFF_A * dt);

        // Stage 3: Position update
        *position += *velocity * (Pefrl::CHI * dt);

        // Stage 4: Velocity update with second acceleration
        let accel_2 = evaluator.calc_acceleration(*position);
        *velocity += accel_2 * (Pefrl::LAMBDA * dt);

        // Stage 5: Position update (middle stage)
        *position += *velocity * (coeff_b * dt);

        // Stage 6: Velocity update with third acceleration
        let accel_3 = evaluator.calc_acceleration(*position);
        *velocity += accel_3 * (Pefrl::LAMBDA * dt);

        // Stage 7: Position update
        *position += *velocity * (Pefrl::CHI * dt);

        // Stage 8: Velocity update with fourth acceleration
        let accel_4 = evaluator.calc_acceleration(*position);
        *velocity += accel_4 * (COEFF_A * dt);

        // Stage 9: Final position update
        *position += *velocity * (Pefrl::XI * dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::math::Vector;

    #[test]
    fn test_pefrl_simple_step() {
        let integrator = Pefrl;

        let mut position = Vector::new(1.0, 0.0, 0.0);
        let mut velocity = Vector::new(1.0, 1.0, 0.0); // Give x velocity too
        let dt = 0.01;

        // Test evaluator with constant acceleration
        struct TestEvaluator;
        impl ForceEvaluator for TestEvaluator {
            fn calc_acceleration(&self, _position: Vector) -> Vector {
                Vector::new(0.0, 0.0, -9.81)
            }
        }
        let evaluator = TestEvaluator;

        let initial_x = position.x;
        let initial_y = position.y;

        integrator.step(&mut position, &mut velocity, &evaluator, dt);

        // Verify movement occurred
        assert!(position.x > initial_x, "X position should have increased");
        assert!(position.y > initial_y, "Y position should have increased");
        assert!(velocity.z < 0.0, "Z velocity should be negative");
    }

    #[test]
    fn test_pefrl_energy_conservation() {
        // Test with a simple harmonic oscillator to verify energy conservation
        let integrator = Pefrl;

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
            integrator.step(&mut position, &mut velocity, &evaluator, dt);
        }

        // Calculate final energy
        let final_energy = 0.5 * velocity.length_squared() + 0.5 * k * position.length_squared();

        // Energy should be conserved to within numerical precision
        // PEFRL should have excellent energy conservation
        let energy_error = (final_energy - initial_energy).abs() / initial_energy;
        assert!(
            energy_error < 1e-8,
            "Energy error should be very small: {}",
            energy_error
        );
    }

    #[test]
    fn test_pefrl_fourth_order_accuracy() {
        // Test that PEFRL achieves 4th order accuracy
        let integrator = Pefrl;

        // Test with simple harmonic oscillator
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
        let dt1 = 0.01;
        let dt2 = 0.005; // Half the timestep

        let initial_pos = Vector::new(1.0, 0.0, 0.0);
        let initial_vel = Vector::new(0.0, 0.0, 0.0);

        // Integrate with dt1
        let mut pos1 = initial_pos;
        let mut vel1 = initial_vel;
        for _ in 0..10 {
            integrator.step(&mut pos1, &mut vel1, &evaluator, dt1);
        }

        // Integrate with dt2 (twice as many steps)
        let mut pos2 = initial_pos;
        let mut vel2 = initial_vel;
        for _ in 0..20 {
            integrator.step(&mut pos2, &mut vel2, &evaluator, dt2);
        }

        // The error should scale as O(dt^4) for a 4th order method
        // This is a simplified test - just verify results are close
        let pos_diff = (pos1 - pos2).length();
        assert!(
            pos_diff < 1e-7,
            "Position difference between timesteps: {}",
            pos_diff
        );
    }
}
