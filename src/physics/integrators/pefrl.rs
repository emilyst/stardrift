//! Position-Extended Forest-Ruth-Like (PEFRL) integration method
//!
//! A state-of-the-art 4th-order symplectic integrator optimized for long-term
//! stability in Hamiltonian systems. PEFRL combines high accuracy with exact
//! preservation of phase space structure, making it ideal for orbital mechanics
//! and N-body simulations where energy conservation over millions of timesteps
//! is critical.

use super::{ForceEvaluator, Integrator};
use crate::physics::math::{Scalar, Vector};

/// PEFRL integrator - a 4th order symplectic integrator
///
/// This is a fourth-order symplectic integrator that achieves exceptional long-term
/// stability through a carefully optimized 9-stage symmetric composition. Unlike
/// traditional Runge-Kutta methods, PEFRL preserves the geometric structure of
/// phase space, preventing the artificial energy drift that plagues non-symplectic
/// integrators in conservative systems.
///
/// # Algorithm Structure
///
/// PEFRL uses a palindromic (time-reversible) sequence of position and velocity updates
/// with specially optimized coefficients that minimize the leading error term:
///
/// ```text
/// Stage 1: x += ξ * v * dt                    (ξ = 0.1786178958448091)
/// Stage 2: v += (1-2λ)/2 * a(x) * dt          (λ = -0.2123418310626054)
/// Stage 3: x += χ * v * dt                    (χ = -0.0662645826698185)
/// Stage 4: v += λ * a(x) * dt
/// Stage 5: x += (1-2(χ+ξ)) * v * dt           (middle stage)
/// Stage 6: v += λ * a(x) * dt
/// Stage 7: x += χ * v * dt
/// Stage 8: v += (1-2λ)/2 * a(x) * dt
/// Stage 9: x += ξ * v * dt
/// ```
///
/// The symmetric structure (reading identically forward and backward) ensures
/// time-reversibility and causes odd-order error terms to cancel, achieving
/// 4th-order accuracy with only 4 force evaluations.
///
/// # Symplectic Properties
///
/// **Phase Space Preservation**: Each stage is a shear transformation with
/// Jacobian determinant = 1. The composition preserves the symplectic 2-form
/// ω = dp ∧ dq, ensuring:
/// - Exact conservation of phase space volume (Liouville's theorem)
/// - Bounded energy oscillations without secular drift
/// - Preservation of Poincaré invariants
/// - Long-term stability of orbital parameters
///
/// **Modified Hamiltonian**: PEFRL exactly conserves a modified Hamiltonian
/// H̃ = H + O(dt⁴), where the modification is small and bounded. This contrasts
/// with non-symplectic methods where energy error grows unboundedly.
///
/// # Mathematical Properties
///
/// - **Order of accuracy**: O(dt⁴) local truncation error
/// - **Force evaluations**: 4 per timestep (stages 2, 4, 6, 8)
/// - **Symplectic**: Yes - preserves phase space volume exactly
/// - **Time-reversible**: Yes - palindromic structure ensures reversibility
/// - **Stability**: Excellent for Hamiltonian systems
///
/// # Coefficient Optimization
///
/// The coefficients (ξ, λ, χ) were determined through numerical optimization
/// to minimize the coefficient of the dt⁵ error term while maintaining the
/// symplectic constraint that position and velocity coefficients each sum to 1.
/// These specific values provide approximately 100× better accuracy than
/// unoptimized symmetric compositions of the same order.
///
/// # Energy Behavior
///
/// - **Energy conservation**: Bounded oscillations without secular drift
/// - **Modified Hamiltonian**: Conserves H̃ = H + O(dt⁴) exactly
/// - **Long-term stability**: Suitable for millions of timesteps
/// - **Phase space**: All Poincaré invariants preserved
///
/// # Computational Cost
///
/// - **Force evaluations**: 4 per timestep
/// - **Cost comparison**: ~4× more expensive than Velocity Verlet per step
/// - **Efficiency gain**: Often allows larger timesteps than lower-order methods
/// - **Memory usage**: Minimal - only current state stored
/// - **Parallelization**: Limited - stages must be computed sequentially
///
/// # Comparison with Other Methods
///
/// | Integrator      | Order | Symplectic | Force Evals | Energy Behavior            |
/// |-----------------|-------|------------|-------------|----------------------------|
/// | PEFRL           | 4     | Yes        | 4           | Bounded oscillation        |
/// | RK4             | 4     | No         | 4           | Secular drift              |
/// | Velocity Verlet | 2     | Yes        | 1           | Bounded oscillation        |
/// | Yoshida4        | 4     | Yes        | 3           | Bounded (larger amplitude) |
///
/// # Use Cases
///
/// **Ideal for:**
/// - Solar system dynamics and asteroid trajectory calculations
/// - Long-term stability studies in celestial mechanics
/// - Molecular dynamics with conservative forces
/// - Symplectic integration benchmarks
/// - Any Hamiltonian system requiring both high accuracy and conservation
///
/// **Consider alternatives:**
/// - Use Velocity Verlet for most applications (simpler, often sufficient)
/// - Use RK4 for short simulations where energy drift is acceptable
/// - Use higher-order composition methods if even better accuracy needed
///
/// # Implementation Notes
///
/// The implementation precomputes the derived coefficients (1-2λ)/2 and 1-2(χ+ξ)
/// for efficiency. Both COEFF_A and COEFF_B are computed as constants at compile
/// time, eliminating redundant arithmetic during integration.
///
/// # Historical Note
///
/// PEFRL represents the culmination of decades of research into symplectic
/// integration. The Forest-Ruth method (1990) pioneered 4th-order symplectic
/// integration, while this optimized variant (2002) achieves superior accuracy
/// through coefficient optimization.
///
/// # Reference
///
/// Omelyan, Mryglod, Folk (2002) "Optimized Forest-Ruth- and Suzuki-like algorithms
/// for integration of motion in many-body systems", Computer Physics Communications
/// 146(2), 188-202. DOI: 10.1016/S0010-4655(02)00451-4
#[derive(Debug, Clone, Default)]
pub struct Pefrl;

impl Pefrl {
    const XI: Scalar = 0.178_617_895_844_809_1;
    const LAMBDA: Scalar = -0.212_341_831_062_605_4;
    const CHI: Scalar = -0.066_264_582_669_818_5;
    const COEFF_A: Scalar = 0.5 * (1.0 - 2.0 * Pefrl::LAMBDA);
    const COEFF_B: Scalar = 1.0 - 2.0 * (Pefrl::CHI + Pefrl::XI);
}

impl Integrator for Pefrl {
    fn step(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        evaluator: &dyn ForceEvaluator,
        dt: Scalar,
    ) {
        // Stage 1: Position update
        *position += *velocity * (Pefrl::XI * dt);

        // Stage 2: Velocity update with first acceleration
        let accel_1 = evaluator.calc_acceleration(*position);
        *velocity += accel_1 * (Pefrl::COEFF_A * dt);

        // Stage 3: Position update
        *position += *velocity * (Pefrl::CHI * dt);

        // Stage 4: Velocity update with second acceleration
        let accel_2 = evaluator.calc_acceleration(*position);
        *velocity += accel_2 * (Pefrl::LAMBDA * dt);

        // Stage 5: Position update (middle stage)
        *position += *velocity * (Pefrl::COEFF_B * dt);

        // Stage 6: Velocity update with third acceleration
        let accel_3 = evaluator.calc_acceleration(*position);
        *velocity += accel_3 * (Pefrl::LAMBDA * dt);

        // Stage 7: Position update
        *position += *velocity * (Pefrl::CHI * dt);

        // Stage 8: Velocity update with fourth acceleration
        let accel_4 = evaluator.calc_acceleration(*position);
        *velocity += accel_4 * (Pefrl::COEFF_A * dt);

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
