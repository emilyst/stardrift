//! Semi-implicit Euler integration method (symplectic Euler)

use super::Integrator;
use crate::physics::math::{Scalar, Vector};

/// Semi-implicit Euler integrator (also known as symplectic Euler)
///
/// This is a first-order symplectic integrator that updates velocities
/// before positions, providing better energy conservation than explicit Euler.
#[derive(Debug, Clone, Default)]
pub struct SemiImplicitEuler;

impl Integrator for SemiImplicitEuler {
    fn integrate_single(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        acceleration: Vector,
        dt: Scalar,
    ) {
        // Update velocity first: v(t+dt) = v(t) + a(t) * dt
        *velocity += acceleration * dt;

        // Then update position using new velocity: x(t+dt) = x(t) + v(t+dt) * dt
        *position += *velocity * dt;
    }

    fn name(&self) -> &str {
        "Semi-implicit Euler"
    }

    fn order(&self) -> usize {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::math::Vector;

    #[test]
    fn test_semi_implicit_euler_integrate_single() {
        let integrator = SemiImplicitEuler;

        let mut position = Vector::new(1.0, 0.0, 0.0);
        let mut velocity = Vector::new(0.0, 1.0, 0.0);
        let acceleration = Vector::new(0.0, 0.0, -9.81);
        let dt = 0.01;

        integrator.integrate_single(&mut position, &mut velocity, acceleration, dt);

        // Velocity should be updated first
        assert_eq!(velocity, Vector::new(0.0, 1.0, -0.0981));

        // Position should use the new velocity
        let expected_position = Vector::new(1.0, 0.01, -0.000981);
        assert!((position - expected_position).length() < 1e-6);
    }

    #[test]
    fn test_properties() {
        let integrator = SemiImplicitEuler;
        assert_eq!(integrator.name(), "Semi-implicit Euler");
        assert_eq!(integrator.order(), 1);
    }
}
