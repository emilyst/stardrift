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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::math::Vector;
    use crate::test_utils::physics::acceleration_functions::{
        ConstantAcceleration, HarmonicOscillator,
    };

    #[test]
    fn test_symplectic_euler_integrate_single() {
        let integrator = SymplecticEuler;
        let test_field = ConstantAcceleration::default(); // Uses default gravity

        let mut position = Vector::new(1.0, 0.0, 0.0);
        let mut velocity = Vector::new(0.0, 1.0, 0.0);
        let dt = 0.01;

        integrator.step(&mut position, &mut velocity, &test_field, dt);

        // Velocity should be updated first
        assert_eq!(velocity, Vector::new(0.0, 1.0, -0.0981));

        // Position should use the new velocity
        let expected_position = Vector::new(1.0, 0.01, -0.000981);
        assert!((position - expected_position).length() < 1e-6);
    }

    #[test]
    fn test_symplectic_euler_energy_conservation() {
        // Test with harmonic oscillator - energy should remain bounded
        let integrator = SymplecticEuler;

        let mut position = Vector::new(1.0, 0.0, 0.0);
        let mut velocity = Vector::new(0.0, 0.0, 0.0);
        let k = 1.0; // Spring constant
        let dt = 0.01;

        let initial_energy = 0.5 * k * position.length_squared();

        let spring_field = HarmonicOscillator { k };

        // Simulate for many steps
        for _ in 0..1000 {
            integrator.step(&mut position, &mut velocity, &spring_field, dt);
        }

        // Calculate final energy
        let final_energy = 0.5 * velocity.length_squared() + 0.5 * k * position.length_squared();

        // Energy should be bounded (not necessarily conserved to high precision due to O(dt) error)
        // For symplectic Euler, we expect larger error than Velocity Verlet but still bounded
        let energy_error = (final_energy - initial_energy).abs() / initial_energy;
        assert!(
            energy_error < 0.1, // 10% error is acceptable for first-order method
            "Energy error should be bounded: {}",
            energy_error
        );
    }

    #[test]
    fn test_symplectic_euler_order_verification() {
        // Verify first-order convergence
        let integrator = SymplecticEuler;

        // Test with harmonic oscillator (known analytical solution)
        let spring_field = HarmonicOscillator { k: 1.0 };

        // Test with two different timesteps
        let dt1 = 0.1;
        let dt2 = 0.05; // Half the timestep

        let initial_pos = Vector::new(1.0, 0.0, 0.0);
        let initial_vel = Vector::new(0.0, 0.0, 0.0);
        let final_time = 1.0;

        // Integrate with dt1
        let mut pos1 = initial_pos;
        let mut vel1 = initial_vel;
        let steps1 = (final_time / dt1) as usize;
        for _ in 0..steps1 {
            integrator.step(&mut pos1, &mut vel1, &spring_field, dt1);
        }

        // Integrate with dt2
        let mut pos2 = initial_pos;
        let mut vel2 = initial_vel;
        let steps2 = (final_time / dt2) as usize;
        for _ in 0..steps2 {
            integrator.step(&mut pos2, &mut vel2, &spring_field, dt2);
        }

        // Analytical solution at t=1: x = cos(t), v = -sin(t)
        let exact_pos = Vector::new(final_time.cos(), 0.0, 0.0);
        // Note: exact_vel would be Vector::new(-final_time.sin(), 0.0, 0.0)
        // but we only test position convergence for this order verification

        let error1 = (pos1 - exact_pos).length();
        let error2 = (pos2 - exact_pos).length();

        // For first-order method, error should scale as O(dt)
        // error2/error1 should be approximately dt2/dt1 = 0.5
        let error_ratio = error2 / error1;
        assert!(
            (error_ratio - 0.5).abs() < 0.15, // Some tolerance for numerical errors
            "First-order convergence not satisfied. Error ratio: {}, expected ~0.5",
            error_ratio
        );
    }
}
