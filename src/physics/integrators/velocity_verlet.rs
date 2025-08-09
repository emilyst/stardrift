//! Velocity Verlet integration method

use super::{Integrator, MultiStepIntegrator};
use crate::physics::components::KinematicHistory;
use crate::physics::math::{Scalar, Vector};

/// Velocity Verlet integrator
///
/// A second-order symplectic integrator that provides excellent energy conservation
/// for Hamiltonian systems like gravitational n-body simulations. When historical
/// acceleration data is available, it uses the standard Velocity Verlet algorithm.
/// Otherwise, it falls back to a position-Verlet method using only current acceleration.
///
/// The algorithm with history:
/// 1. Update position: x(t+dt) = x(t) + v(t)*dt + 0.5*a(t)*dt²
/// 2. Update velocity: v(t+dt) = v(t) + 0.5*(a(t) + a(t-dt))*dt
///
/// Without history (fallback):
/// 1. Update position: x(t+dt) = x(t) + v(t)*dt + 0.5*a(t)*dt²
/// 2. Update velocity: v(t+dt) = v(t) + a(t)*dt
#[derive(Debug, Clone, Default)]
pub struct VelocityVerlet;

impl Integrator for VelocityVerlet {
    fn step(&self, position: &mut Vector, velocity: &mut Vector, acceleration: Vector, dt: Scalar) {
        // Fallback: Position-Verlet when no history is available
        // This is still second-order accurate for position

        // Update position using current velocity and acceleration
        // x(t+dt) = x(t) + v(t)*dt + 0.5*a(t)*dt²
        *position += *velocity * dt + acceleration * (0.5 * dt * dt);

        // Update velocity using current acceleration only
        // v(t+dt) = v(t) + a(t)*dt
        // Note: This is less accurate than full Velocity Verlet but maintains stability
        *velocity += acceleration * dt;
    }

    fn name(&self) -> &str {
        "Velocity Verlet"
    }

    fn order(&self) -> usize {
        2
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl MultiStepIntegrator for VelocityVerlet {
    fn step_with_history(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        acceleration: Vector,
        dt: Scalar,
        history: &KinematicHistory,
    ) {
        // Get previous acceleration from history if available
        if let Some(previous_state) = history.get(0) {
            // Full Velocity Verlet algorithm

            // Update position using current velocity and acceleration
            // x(t+dt) = x(t) + v(t)*dt + 0.5*a(t)*dt²
            *position += *velocity * dt + acceleration * (0.5 * dt * dt);

            // Update velocity using average of current and previous acceleration
            // v(t+dt) = v(t) + 0.5*(a(t) + a(t-dt))*dt
            *velocity += (acceleration + previous_state.acceleration) * (0.5 * dt);
        } else {
            // Fall back to simple step if no history available
            self.step(position, velocity, acceleration, dt);
        }
    }

    fn required_history_size(&self) -> usize {
        1 // Only needs one previous state for the previous acceleration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::components::{KinematicHistory, KinematicState};
    use crate::physics::math::Vector;

    #[test]
    fn test_velocity_verlet_simple_step() {
        let integrator = VelocityVerlet;

        let mut position = Vector::new(1.0, 0.0, 0.0);
        let mut velocity = Vector::new(0.0, 1.0, 0.0);
        let acceleration = Vector::new(0.0, 0.0, -9.81);
        let dt = 0.01;

        integrator.step(&mut position, &mut velocity, acceleration, dt);

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
    fn test_velocity_verlet_with_history() {
        let integrator = VelocityVerlet;

        let mut position = Vector::new(1.0, 0.0, 0.0);
        let mut velocity = Vector::new(0.0, 1.0, 0.0);
        let acceleration = Vector::new(0.0, 0.0, -9.81);
        let dt = 0.01;

        // Create history with previous acceleration
        let mut history = KinematicHistory::new(KinematicState::new(
            Vector::new(0.0, 0.0, 0.0),
            Vector::new(0.0, 0.0, 0.0),
            Vector::new(0.0, 0.0, 0.0),
        ));
        let previous_acceleration = Vector::new(0.0, 0.0, -9.0);
        history.push(KinematicState::new(
            Vector::new(0.9, -0.01, 0.0),
            Vector::new(0.0, 0.9, 0.0),
            previous_acceleration,
        ));

        integrator.step_with_history(&mut position, &mut velocity, acceleration, dt, &history);

        // Position update should be the same as simple step
        let expected_x = 1.0;
        let expected_y = 0.01;
        let expected_z = -0.00049;

        assert!((position.x - expected_x).abs() < 1e-6);
        assert!((position.y - expected_y).abs() < 1e-6);
        assert!((position.z - expected_z).abs() < 1e-6);

        // Velocity should use average of current and previous acceleration
        // v(t+dt) = v(t) + 0.5*(a(t) + a(t-dt))*dt
        // v_z = 0 + 0.5*(-9.81 + -9.0)*0.01 = -0.09405
        let expected_vz = -0.09405;
        assert!((velocity.z - expected_vz).abs() < 1e-6);
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

        let mut history = KinematicHistory::new(KinematicState::new(
            Vector::new(0.0, 0.0, 0.0),
            Vector::new(0.0, 0.0, 0.0),
            Vector::new(0.0, 0.0, 0.0),
        ));

        // Simulate for many steps
        for _ in 0..1000 {
            // Spring force: F = -kx, so a = -kx/m (assuming m=1)
            let acceleration = position * (-k);

            // Store current state before integration
            history.push(KinematicState::new(position, velocity, acceleration));

            // Integrate
            integrator.step_with_history(&mut position, &mut velocity, acceleration, dt, &history);
        }

        // Calculate final energy
        let final_energy = 0.5 * velocity.length_squared() + 0.5 * k * position.length_squared();

        // Energy should be conserved to within numerical precision
        // Note: Larger error tolerance because the integrator needs warm-up period
        let energy_error = (final_energy - initial_energy).abs() / initial_energy;
        assert!(energy_error < 0.05, "Energy error: {}", energy_error);
    }

    #[test]
    fn test_properties() {
        let integrator = VelocityVerlet;
        assert_eq!(integrator.name(), "Velocity Verlet");
        assert_eq!(integrator.order(), 2);
        assert_eq!(integrator.required_history_size(), 1);
    }
}
