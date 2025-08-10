//! Runge-Kutta integration methods

use super::{ForceEvaluator, Integrator};
use crate::physics::math::{Scalar, Vector};

/// Second-order Runge-Kutta method (Midpoint method)
///
/// This is a 2-stage, 2nd order accurate integrator that evaluates
/// the derivative at the midpoint of the timestep.
///
/// Algorithm:
/// - k1 = f(t, y)
/// - k2 = f(t + dt/2, y + k1*dt/2)
/// - y_new = y + k2*dt
#[derive(Debug, Clone, Copy, Default)]
pub struct RungeKuttaSecondOrderMidpoint;

impl Integrator for RungeKuttaSecondOrderMidpoint {
    fn step(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        evaluator: &dyn ForceEvaluator,
        dt: Scalar,
    ) {
        // Proper RK2 Midpoint method with force evaluation
        // Achieves true 2nd order accuracy by evaluating at the midpoint

        // Stage 1: Evaluate at current position
        let k1_x = *velocity;
        let k1_v = evaluator.calc_acceleration(*position);

        // Stage 2: Evaluate at midpoint
        let pos_mid = *position + k1_x * (dt * 0.5);
        let vel_mid = *velocity + k1_v * (dt * 0.5);
        let k2_x = vel_mid;
        let k2_v = evaluator.calc_acceleration(pos_mid);

        // Update using midpoint derivative
        *position += k2_x * dt;
        *velocity += k2_v * dt;
    }
}

/// Fourth-order Runge-Kutta integrator (RK4)
///
/// A classic multi-stage integrator that provides fourth-order accuracy
/// by combining four intermediate evaluations of the derivative.
///
/// The RK4 algorithm:
/// 1. k1 = f(t, y)
/// 2. k2 = f(t + dt/2, y + k1*dt/2)
/// 3. k3 = f(t + dt/2, y + k2*dt/2)
/// 4. k4 = f(t + dt, y + k3*dt)
/// 5. y(t+dt) = y(t) + dt/6 * (k1 + 2*k2 + 2*k3 + k4)
#[derive(Debug, Clone, Default)]
pub struct RungeKuttaFourthOrder;

impl Integrator for RungeKuttaFourthOrder {
    fn step(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        evaluator: &dyn ForceEvaluator,
        dt: Scalar,
    ) {
        // Proper RK4 with force evaluation at each stage
        // This achieves true 4th order accuracy

        // Stage 1: k1 at current position
        let k1_x = *velocity;
        let k1_v = evaluator.calc_acceleration(*position);

        // Stage 2: k2 at midpoint using k1
        let pos_k2 = *position + k1_x * (dt * 0.5);
        let vel_k2 = *velocity + k1_v * (dt * 0.5);
        let k2_x = vel_k2;
        let k2_v = evaluator.calc_acceleration(pos_k2);

        // Stage 3: k3 at midpoint using k2
        let pos_k3 = *position + k2_x * (dt * 0.5);
        let vel_k3 = *velocity + k2_v * (dt * 0.5);
        let k3_x = vel_k3;
        let k3_v = evaluator.calc_acceleration(pos_k3);

        // Stage 4: k4 at endpoint using k3
        let pos_k4 = *position + k3_x * dt;
        let vel_k4 = *velocity + k3_v * dt;
        let k4_x = vel_k4;
        let k4_v = evaluator.calc_acceleration(pos_k4);

        // Combine stages using RK4 weights: y_n+1 = y_n + dt/6 * (k1 + 2*k2 + 2*k3 + k4)
        *position += (k1_x + k2_x * 2.0 + k3_x * 2.0 + k4_x) * (dt / 6.0);
        *velocity += (k1_v + k2_v * 2.0 + k3_v * 2.0 + k4_v) * (dt / 6.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rk4_fallback() {
        let rk4 = RungeKuttaFourthOrder;
        let mut position = Vector::new(1.0, 0.0, 0.0);
        let mut velocity = Vector::new(0.0, 1.0, 0.0);
        let _acceleration = Vector::new(0.0, 0.0, -9.81);
        let dt = 0.01;

        // Test fallback step method
        // Test evaluator
        struct TestEvaluator;
        impl ForceEvaluator for TestEvaluator {
            fn calc_acceleration(&self, _position: Vector) -> Vector {
                Vector::new(0.0, 0.0, -9.81)
            }
        }
        let evaluator = TestEvaluator;

        rk4.step(&mut position, &mut velocity, &evaluator, dt);

        // With RK4 (even simplified with constant acceleration),
        // the result is different from simple Euler
        // Velocity uses RK4 combination: v += (k1 + 2*k2 + 2*k3 + k4) * dt / 6
        // Where all k values are the same (acceleration) for constant acceleration
        // So v += acceleration * dt (same as Euler for constant acceleration)
        assert_eq!(velocity, Vector::new(0.0, 1.0, -0.0981));

        // Position uses RK4 combination with velocity stages
        // The position will be different from Euler due to the RK4 weighting
        // Expected: around (1.0, 0.01, -0.0004905) based on RK4 formula
        let expected_pos = Vector::new(1.0, 0.01, -0.0004905);
        assert!(
            (position - expected_pos).length() < 1e-10,
            "Position should match RK4 result, got {:?}",
            position
        );
    }

    #[test]
    fn test_rk2_midpoint_step() {
        let integrator = RungeKuttaSecondOrderMidpoint;
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
    }
}
