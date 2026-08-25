//! Runge-Kutta integration methods
//!
//! Classical explicit Runge-Kutta methods that achieve high accuracy through
//! multiple evaluations of the derivative at carefully chosen intermediate points.
//! While these methods provide excellent accuracy for smooth problems, they are
//! non-symplectic and exhibit energy drift in conservative systems, making them
//! less suitable for long-term orbital mechanics than symplectic alternatives.

use super::{Integrator, StageQuery, StepState};
use crate::physics::math::{Scalar, Vector};

/// Second-order Runge-Kutta method (Midpoint method)
///
/// A 2nd-order accurate integrator that achieves improved accuracy over Euler
/// methods by evaluating the derivative at the midpoint of each timestep.
/// This method provides a good balance between computational cost and accuracy
/// for short to medium duration simulations.
///
/// # Algorithm
///
/// The midpoint method uses a two-stage approach:
///
/// ```text
/// Stage 1: Evaluate at current state
///   k1_x = v(t)
///   k1_v = a(x(t))
///
/// Stage 2: Evaluate at midpoint
///   x_mid = x(t) + k1_x * dt/2
///   v_mid = v(t) + k1_v * dt/2
///   k2_x = v_mid
///   k2_v = a(x_mid)
///
/// Final update: Use only midpoint derivatives
///   x(t+dt) = x(t) + k2_x * dt
///   v(t+dt) = v(t) + k2_v * dt
/// ```
///
/// # Mathematical Properties
///
/// - **Order of accuracy**: O(dt²) local truncation error
/// - **Force evaluations**: 2 per timestep
/// - **Stability**: Conditionally stable (larger stability region than Euler)
/// - **Time reversibility**: Not time-reversible
/// - **Implicit version**: The implicit midpoint method IS symplectic
///
/// # Energy Behavior
///
/// - **Non-symplectic**: Does not preserve phase space volume
/// - **Energy drift**: Exhibits secular drift in conservative systems
/// - **Error growth**: Energy error accumulates as O(t * dt²)
/// - **Not suitable for**: Long-term orbital mechanics or energy-conserving simulations
///
/// # Computational Cost
///
/// Requires 2 force evaluations per timestep:
/// - More expensive than symplectic Euler (1 evaluation)
/// - Same cost as Heun's method
/// - Less expensive than RK4 (4 evaluations)
///
/// # Use Cases
///
/// **Good for:**
/// - Short to medium duration simulations
/// - Systems with dissipation where energy conservation isn't critical
/// - Initial value problems requiring moderate accuracy
/// - Educational demonstrations of Runge-Kutta concepts
///
/// **Consider alternatives:**
/// - Use `VelocityVerlet` for similar cost but with energy conservation
/// - Use `RK4` when higher accuracy is needed
/// - Use symplectic methods for long-term conservative dynamics
#[derive(Debug, Clone, Copy, Default)]
pub struct RungeKuttaSecondOrderMidpoint;

impl Integrator for RungeKuttaSecondOrderMidpoint {
    fn clone_box(&self) -> Box<dyn Integrator> {
        Box::new(*self)
    }

    fn scratch_len(&self) -> usize {
        2
    }

    fn next_query(&self, state: &StepState, scratch: &[Vector], dt: Scalar) -> Option<StageQuery> {
        match state.stage {
            // Stage 1: evaluate at current position
            0 => Some(StageQuery {
                position: state.initial_position,
                velocity: state.initial_velocity,
            }),
            // Stage 2: evaluate at midpoint
            1 => Some(StageQuery {
                position: state.initial_position + state.initial_velocity * (dt * 0.5),
                velocity: state.initial_velocity + scratch[0] * (dt * 0.5),
            }),
            _ => None,
        }
    }

    fn apply_stage(
        &self,
        state: &mut StepState,
        scratch: &mut [Vector],
        accel: Vector,
        _dt: Scalar,
    ) {
        scratch[state.stage] = accel;
        state.stage += 1;
    }

    fn finish(&self, state: &StepState, scratch: &[Vector], dt: Scalar) -> (Vector, Vector) {
        // Update using the midpoint derivative
        let k2_x = state.initial_velocity + scratch[0] * (dt * 0.5);
        let k2_v = scratch[1];
        (
            state.initial_position + k2_x * dt,
            state.initial_velocity + k2_v * dt,
        )
    }

    fn convergence_order(&self) -> usize {
        2
    }

    fn name(&self) -> &'static str {
        "runge_kutta_second_order_midpoint"
    }

    fn aliases(&self) -> Vec<&'static str> {
        vec!["rk2", "midpoint"]
    }
}

/// Fourth-order Runge-Kutta integrator (RK4)
///
/// The classic "workhorse" of numerical integration, RK4 provides an excellent
/// balance between accuracy and computational cost for many applications.
/// It achieves 4th-order accuracy through a weighted average of four derivative
/// evaluations at carefully chosen points within each timestep.
///
/// # Algorithm
///
/// RK4 uses four stages with the following structure:
///
/// ```text
/// Stage 1: k1 at current state
///   k1_x = v(t)
///   k1_v = a(x(t))
///
/// Stage 2: k2 at midpoint using k1
///   x_2 = x(t) + k1_x * dt/2
///   v_2 = v(t) + k1_v * dt/2
///   k2_x = v_2
///   k2_v = a(x_2)
///
/// Stage 3: k3 at midpoint using k2
///   x_3 = x(t) + k2_x * dt/2
///   v_3 = v(t) + k2_v * dt/2
///   k3_x = v_3
///   k3_v = a(x_3)
///
/// Stage 4: k4 at endpoint using k3
///   x_4 = x(t) + k3_x * dt
///   v_4 = v(t) + k3_v * dt
///   k4_x = v_4
///   k4_v = a(x_4)
///
/// Final update: Weighted average
///   x(t+dt) = x(t) + dt/6 * (k1_x + 2*k2_x + 2*k3_x + k4_x)
///   v(t+dt) = v(t) + dt/6 * (k1_v + 2*k2_v + 2*k3_v + k4_v)
/// ```
///
/// The weights (1, 2, 2, 1)/6 come from Simpson's rule for numerical integration.
///
/// # Mathematical Properties
///
/// - **Order of accuracy**: O(dt⁴) local truncation error
/// - **Force evaluations**: 4 per timestep
/// - **Stability**: Conditionally stable with moderate stability region
/// - **Self-starting**: No previous values needed beyond initial conditions
/// - **Butcher tableau**: Classical 4th-order weights
///
/// # Energy Behavior
///
/// - **Non-symplectic**: Does not preserve phase space volume or symplectic structure
/// - **Energy drift**: Systematic energy error grows linearly with time
/// - **Error accumulation**: Energy error ~ O(t * dt⁴)
/// - **Phase space**: Jacobian determinant ≠ 1, causing artificial dissipation or growth
///
/// Despite its high accuracy, RK4 is unsuitable for long-term integration of
/// conservative systems where energy preservation is critical.
///
/// # Performance Characteristics
///
/// - **Computational cost**: 4 force evaluations per timestep
/// - **Memory efficient**: Only stores current state (self-starting)
/// - **Step size**: Often allows larger dt than lower-order methods for same accuracy
/// - **Parallelization**: Stages 2 and 3 cannot be computed in parallel
///
/// # Comparison with Symplectic Methods
///
/// | Property      | RK4         | PEFRL        | Velocity Verlet     |
/// |---------------|-------------|--------------|---------------------|
/// | Order         | 4           | 4            | 2                   |
/// | Force evals   | 4           | 4            | 1                   |
/// | Symplectic    | No          | Yes          | Yes                 |
/// | Energy drift  | Linear      | Bounded      | Bounded             |
/// | Best for      | Short sims  | Long orbits  | Energy conservation |
///
/// # Use Cases
///
/// **Ideal for:**
/// - Trajectory calculations where high accuracy matters more than energy conservation
/// - Short-duration simulations of any dynamical system
/// - Non-conservative systems with dissipation
/// - Boundary value problems and shooting methods
/// - Systems where the Hamiltonian structure isn't important
///
/// **Avoid for:**
/// - Long-term orbital mechanics (use PEFRL or VelocityVerlet)
/// - Molecular dynamics requiring energy conservation
/// - Hamiltonian systems where phase space structure matters
/// - Very long integration times in conservative systems
///
/// # Historical Note
///
/// Developed by German mathematicians C. Runge and M.W. Kutta around 1900,
/// RK4 became the standard integration method before the importance of
/// symplectic integration for Hamiltonian systems was recognized.
#[derive(Debug, Clone, Copy, Default)]
pub struct RungeKuttaFourthOrder;

impl Integrator for RungeKuttaFourthOrder {
    fn clone_box(&self) -> Box<dyn Integrator> {
        Box::new(*self)
    }

    fn scratch_len(&self) -> usize {
        4
    }

    fn next_query(&self, state: &StepState, scratch: &[Vector], dt: Scalar) -> Option<StageQuery> {
        let x0 = state.initial_position;
        let v0 = state.initial_velocity;
        match state.stage {
            // Stage 1: k1 at current position
            0 => Some(StageQuery {
                position: x0,
                velocity: v0,
            }),
            // Stage 2: k2 at midpoint using k1
            1 => Some(StageQuery {
                position: x0 + v0 * (dt * 0.5),
                velocity: v0 + scratch[0] * (dt * 0.5),
            }),
            // Stage 3: k3 at midpoint using k2 (k2_x = v0 + k1_v*dt/2)
            2 => Some(StageQuery {
                position: x0 + (v0 + scratch[0] * (dt * 0.5)) * (dt * 0.5),
                velocity: v0 + scratch[1] * (dt * 0.5),
            }),
            // Stage 4: k4 at endpoint using k3 (k3_x = v0 + k2_v*dt/2)
            3 => Some(StageQuery {
                position: x0 + (v0 + scratch[1] * (dt * 0.5)) * dt,
                velocity: v0 + scratch[2] * dt,
            }),
            _ => None,
        }
    }

    fn apply_stage(
        &self,
        state: &mut StepState,
        scratch: &mut [Vector],
        accel: Vector,
        _dt: Scalar,
    ) {
        scratch[state.stage] = accel;
        state.stage += 1;
    }

    fn finish(&self, state: &StepState, scratch: &[Vector], dt: Scalar) -> (Vector, Vector) {
        let v0 = state.initial_velocity;
        let &[k1_v, k2_v, k3_v, k4_v] = &scratch[..4] else {
            unreachable!("RK4 records exactly 4 stage accelerations");
        };
        let k1_x = v0;
        let k2_x = v0 + k1_v * (dt * 0.5);
        let k3_x = v0 + k2_v * (dt * 0.5);
        let k4_x = v0 + k3_v * dt;

        // Combine stages using RK4 weights: y_n+1 = y_n + dt/6 * (k1 + 2*k2 + 2*k3 + k4)
        (
            state.initial_position + (k1_x + k2_x * 2.0 + k3_x * 2.0 + k4_x) * (dt / 6.0),
            state.initial_velocity + (k1_v + k2_v * 2.0 + k3_v * 2.0 + k4_v) * (dt / 6.0),
        )
    }

    fn convergence_order(&self) -> usize {
        4
    }

    fn name(&self) -> &'static str {
        "runge_kutta_fourth_order"
    }

    fn aliases(&self) -> Vec<&'static str> {
        vec!["rk4"]
    }
}
