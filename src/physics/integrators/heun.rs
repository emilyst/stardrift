//! Heun's method (Improved Euler) integration
//!
//! A classical second-order predictor-corrector method that achieves improved
//! accuracy over Euler methods through slope averaging. While not symplectic,
//! Heun's method provides a good balance of simplicity and accuracy for
//! short-duration simulations where energy conservation is not critical.

use super::{AccelerationField, Integrator};
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
        // Proper Heun's method with acceleration evaluation
        // This is a predictor-corrector method that achieves 2nd order accuracy

        // Stage 1: Evaluate at current position (predictor)
        let k1_x = *velocity;
        let k1_v = field.at(*position);

        // Stage 2: Evaluate at predicted endpoint
        let pos_predicted = *position + k1_x * dt;
        let vel_predicted = *velocity + k1_v * dt;
        let k2_x = vel_predicted;
        let k2_v = field.at(pos_predicted);

        // Average the slopes (corrector)
        *position += (k1_x + k2_x) * (dt * 0.5);
        *velocity += (k1_v + k2_v) * (dt * 0.5);
    }

    fn convergence_order(&self) -> usize {
        2
    }

    fn name(&self) -> &'static str {
        "heun"
    }

    fn aliases(&self) -> Vec<&'static str> {
        vec!["improved_euler"]
    }
}
