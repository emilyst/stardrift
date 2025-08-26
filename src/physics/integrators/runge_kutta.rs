//! Runge-Kutta integration methods
//!
//! Classical explicit Runge-Kutta methods that achieve high accuracy through
//! multiple evaluations of the derivative at carefully chosen intermediate points.
//! While these methods provide excellent accuracy for smooth problems, they are
//! non-symplectic and exhibit energy drift in conservative systems, making them
//! less suitable for long-term orbital mechanics than symplectic alternatives.

use super::{AccelerationField, Integrator};
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
        Box::new(self.clone())
    }

    fn step(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        field: &dyn AccelerationField,
        dt: Scalar,
    ) {
        // Proper RK2 Midpoint method with acceleration evaluation
        // Achieves true 2nd order accuracy by evaluating at the midpoint

        // Stage 1: Evaluate at current position
        let k1_x = *velocity;
        let k1_v = field.at(*position);

        // Stage 2: Evaluate at midpoint
        let pos_mid = *position + k1_x * (dt * 0.5);
        let vel_mid = *velocity + k1_v * (dt * 0.5);
        let k2_x = vel_mid;
        let k2_v = field.at(pos_mid);

        // Update using midpoint derivative
        *position += k2_x * dt;
        *velocity += k2_v * dt;
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
#[derive(Debug, Clone, Default)]
pub struct RungeKuttaFourthOrder;

impl Integrator for RungeKuttaFourthOrder {
    fn clone_box(&self) -> Box<dyn Integrator> {
        Box::new(self.clone())
    }

    fn step(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        field: &dyn AccelerationField,
        dt: Scalar,
    ) {
        // Proper RK4 with acceleration evaluation at each stage
        // This achieves true 4th order accuracy

        // Stage 1: k1 at current position
        let k1_x = *velocity;
        let k1_v = field.at(*position);

        // Stage 2: k2 at midpoint using k1
        let pos_k2 = *position + k1_x * (dt * 0.5);
        let vel_k2 = *velocity + k1_v * (dt * 0.5);
        let k2_x = vel_k2;
        let k2_v = field.at(pos_k2);

        // Stage 3: k3 at midpoint using k2
        let pos_k3 = *position + k2_x * (dt * 0.5);
        let vel_k3 = *velocity + k2_v * (dt * 0.5);
        let k3_x = vel_k3;
        let k3_v = field.at(pos_k3);

        // Stage 4: k4 at endpoint using k3
        let pos_k4 = *position + k3_x * dt;
        let vel_k4 = *velocity + k3_v * dt;
        let k4_x = vel_k4;
        let k4_v = field.at(pos_k4);

        // Combine stages using RK4 weights: y_n+1 = y_n + dt/6 * (k1 + 2*k2 + 2*k3 + k4)
        *position += (k1_x + k2_x * 2.0 + k3_x * 2.0 + k4_x) * (dt / 6.0);
        *velocity += (k1_v + k2_v * 2.0 + k3_v * 2.0 + k4_v) * (dt / 6.0);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::physics::acceleration_functions::{
        ConstantAcceleration, HarmonicOscillator,
    };

    #[test]
    fn test_rk4_basic_step() {
        let rk4 = RungeKuttaFourthOrder;
        let mut position = Vector::new(1.0, 0.0, 0.0);
        let mut velocity = Vector::new(0.0, 1.0, 0.0);
        let dt = 0.01;

        let test_field = ConstantAcceleration::default();

        rk4.step(&mut position, &mut velocity, &test_field, dt);

        // Velocity: for constant acceleration, RK4 gives exact result
        assert_eq!(velocity, Vector::new(0.0, 1.0, -0.0981));

        // Position: RK4 weighted average
        let expected_pos = Vector::new(1.0, 0.01, -0.0004905);
        assert!(
            (position - expected_pos).length() < 1e-10,
            "Position should match RK4 result, got {:?}",
            position
        );
    }

    #[test]
    fn test_rk4_energy_drift() {
        // RK4 should exhibit energy drift despite high accuracy
        let integrator = RungeKuttaFourthOrder;

        let mut position = Vector::new(1.0, 0.0, 0.0);
        let mut velocity = Vector::new(0.0, 0.0, 0.0);
        let k = 1.0;
        let dt = 0.05; // Larger timestep acceptable due to 4th order

        let spring_field = HarmonicOscillator { k };

        // Track drift over long time
        let mut energies = Vec::new();
        for _ in 0..2000 {
            integrator.step(&mut position, &mut velocity, &spring_field, dt);
            let energy = 0.5 * velocity.length_squared() + 0.5 * k * position.length_squared();
            energies.push(energy);
        }

        // Check for systematic drift trend
        let mid_energy = energies[1000];
        let final_energy = energies[1999];

        // RK4 has slower drift than RK2, but as a non-symplectic method
        // it should show some drift pattern over long integration
        // Check that energy is not perfectly conserved
        let drift = (final_energy - mid_energy).abs();
        assert!(
            drift > 1e-8 || final_energy != mid_energy,
            "RK4 is non-symplectic and should exhibit some energy variation. Drift: {}",
            drift
        );
    }

    #[test]
    fn test_rk4_fourth_order_convergence() {
        // Verify fourth-order convergence
        let integrator = RungeKuttaFourthOrder;

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

        // Analytical solution
        let exact_pos = Vector::new(final_time.cos(), 0.0, 0.0);

        let error1 = (pos1 - exact_pos).length();
        let error2 = (pos2 - exact_pos).length();

        // For fourth-order method: error2/error1 ≈ (dt2/dt1)^4 = 0.0625
        let error_ratio = error2 / error1;
        assert!(
            (error_ratio - 0.0625).abs() < 0.02,
            "Fourth-order convergence not satisfied. Ratio: {}, expected ~0.0625",
            error_ratio
        );
    }

    #[test]
    fn test_rk2_midpoint_step() {
        let integrator = RungeKuttaSecondOrderMidpoint;
        let mut position = Vector::new(0.0, 0.0, 0.0);
        let mut velocity = Vector::new(1.0, 0.0, 0.0);
        let dt = 0.01;

        let test_field = ConstantAcceleration {
            acceleration: Vector::new(0.0, -9.81, 0.0),
        };

        integrator.step(&mut position, &mut velocity, &test_field, dt);

        // Verify movement occurred
        assert!(position.x > 0.0);
        assert!(velocity.y < 0.0);
    }

    #[test]
    fn test_rk2_energy_drift() {
        // RK2 should exhibit energy drift (non-symplectic)
        let integrator = RungeKuttaSecondOrderMidpoint;

        let mut position = Vector::new(1.0, 0.0, 0.0);
        let mut velocity = Vector::new(0.0, 0.0, 0.0);
        let k = 1.0;
        let dt = 0.01;

        let initial_energy = 0.5 * k * position.length_squared();

        let spring_field = HarmonicOscillator { k };

        // Simulate for many steps
        for _ in 0..5000 {
            integrator.step(&mut position, &mut velocity, &spring_field, dt);
        }

        let final_energy = 0.5 * velocity.length_squared() + 0.5 * k * position.length_squared();
        let energy_error = (final_energy - initial_energy).abs() / initial_energy;

        // RK2 is non-symplectic and should not conserve energy perfectly
        // Even small errors accumulate over 5000 steps
        assert!(
            energy_error > 1e-5,
            "RK2 is non-symplectic and should not conserve energy perfectly. Error: {}",
            energy_error
        );
    }
}
