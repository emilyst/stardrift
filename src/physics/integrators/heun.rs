use super::{ForceEvaluator, Integrator};
use crate::physics::math::{Scalar, Vector};

/// Heun's method (Improved Euler method)
///
/// This is a 2-stage, 2nd order accurate integrator that averages
/// the derivatives at the beginning and end of the timestep.
///
/// Algorithm:
/// - k1 = f(t, y)  
/// - k2 = f(t + dt, y + k1*dt)
/// - y_new = y + (k1 + k2)*dt/2
///
/// Also known as the trapezoidal rule or improved Euler method.
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
    fn test_heun_step() {
        let integrator = Heun;
        let mut position = Vector::new(0.0, 0.0, 0.0);
        let mut velocity = Vector::new(1.0, 0.0, 0.0);
        let _acceleration = Vector::new(0.0, -9.81, 0.0);
        let dt = 0.01;

        // Test evaluator
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
}
