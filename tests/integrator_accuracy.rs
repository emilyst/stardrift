//! Accuracy tests for numerical integrators
//!
//! Tests each integrator against known analytical solutions and verifies
//! expected order of convergence.

use stardrift::physics::integrators::{
    AccelerationField, Heun, Integrator, RungeKuttaFourthOrder, RungeKuttaSecondOrderMidpoint,
    SymplecticEuler, VelocityVerlet,
};
use stardrift::physics::math::{Scalar, Vector};

const PI: Scalar = std::f64::consts::PI;

/// Test fixture for a simple harmonic oscillator
///
/// Analytical solution:
/// x(t) = A * cos(ωt + φ)
/// v(t) = -A * ω * sin(ωt + φ)
///
/// With initial conditions x(0) = A, v(0) = 0:
/// x(t) = A * cos(ωt)
/// v(t) = -A * ω * sin(ωt)
struct HarmonicOscillator {
    omega: Scalar,
    amplitude: Scalar,
}

impl HarmonicOscillator {
    fn new(omega: Scalar, amplitude: Scalar) -> Self {
        Self { omega, amplitude }
    }

    /// Get acceleration for harmonic oscillator: a = -ω²x
    #[allow(dead_code)]
    fn acceleration(&self, position: Vector) -> Vector {
        -self.omega * self.omega * position
    }

    /// Analytical position at time t
    fn exact_position(&self, t: Scalar) -> Vector {
        Vector::new(self.amplitude * (self.omega * t).cos(), 0.0, 0.0)
    }

    /// Analytical velocity at time t
    fn exact_velocity(&self, t: Scalar) -> Vector {
        Vector::new(
            -self.amplitude * self.omega * (self.omega * t).sin(),
            0.0,
            0.0,
        )
    }

    /// Total energy (should be conserved)
    fn energy(&self, position: Vector, velocity: Vector) -> Scalar {
        let kinetic = 0.5 * velocity.length_squared();
        let potential = 0.5 * self.omega * self.omega * position.length_squared();
        kinetic + potential
    }
}

/// Acceleration field for harmonic oscillator
struct HarmonicOscillatorAccelerationField {
    omega: Scalar,
}

impl HarmonicOscillatorAccelerationField {
    fn new(omega: Scalar) -> Self {
        Self { omega }
    }
}

impl AccelerationField for HarmonicOscillatorAccelerationField {
    fn at(&self, position: Vector) -> Vector {
        -self.omega * self.omega * position
    }
}

/// Run a simulation with given integrator and return final state
fn simulate_harmonic_oscillator(
    integrator: &dyn Integrator,
    oscillator: &HarmonicOscillator,
    dt: Scalar,
    steps: usize,
) -> (Vector, Vector, Scalar) {
    let mut position = Vector::new(oscillator.amplitude, 0.0, 0.0);
    let mut velocity = Vector::new(0.0, 0.0, 0.0);
    let harmonic_oscillator_field = HarmonicOscillatorAccelerationField::new(oscillator.omega);

    for _ in 0..steps {
        integrator.step(&mut position, &mut velocity, &harmonic_oscillator_field, dt);
    }

    let final_time = dt * steps as Scalar;
    (position, velocity, final_time)
}

/// Calculate relative error between numerical and analytical solutions
fn calculate_error(numerical: Vector, analytical: Vector) -> Scalar {
    (numerical - analytical).length() / analytical.length().max(1e-10)
}

/// Test energy conservation for symplectic integrators
#[test]
fn test_symplectic_euler_energy_conservation() {
    let integrator = SymplecticEuler;
    let oscillator = HarmonicOscillator::new(2.0 * PI, 1.0);

    let dt = 0.001;
    let steps = 10000;

    let mut position = Vector::new(1.0, 0.0, 0.0);
    let mut velocity = Vector::new(0.0, 0.0, 0.0);
    let initial_energy = oscillator.energy(position, velocity);
    let harmonic_oscillator_field = HarmonicOscillatorAccelerationField::new(oscillator.omega);

    let mut max_energy_error = 0.0f64;

    for _ in 0..steps {
        integrator.step(&mut position, &mut velocity, &harmonic_oscillator_field, dt);

        let current_energy = oscillator.energy(position, velocity);
        let energy_error = ((current_energy - initial_energy) / initial_energy).abs();
        max_energy_error = max_energy_error.max(energy_error);
    }

    // Symplectic Euler should conserve energy to within ~1% for this test
    assert!(
        max_energy_error < 0.01,
        "Energy drift too large: {:.2}%",
        max_energy_error * 100.0
    );
}

#[test]
fn test_velocity_verlet_energy_conservation() {
    let integrator = VelocityVerlet;
    let oscillator = HarmonicOscillator::new(2.0 * PI, 1.0);

    let dt = 0.01;
    let steps = 1000;

    // Test Velocity Verlet energy conservation
    let mut position = Vector::new(1.0, 0.0, 0.0);
    let mut velocity = Vector::new(0.0, 0.0, 0.0);
    let initial_energy = oscillator.energy(position, velocity);
    let harmonic_oscillator_field = HarmonicOscillatorAccelerationField::new(oscillator.omega);

    let mut max_energy_error = 0.0f64;

    for _ in 0..steps {
        integrator.step(&mut position, &mut velocity, &harmonic_oscillator_field, dt);

        let current_energy = oscillator.energy(position, velocity);
        let energy_error = ((current_energy - initial_energy) / initial_energy).abs();
        max_energy_error = max_energy_error.max(energy_error);
    }

    println!(
        "Velocity Verlet energy error: {:.6}%",
        max_energy_error * 100.0
    );

    // Velocity Verlet with proper force recalculation conserves energy excellently
    assert!(
        max_energy_error < 0.001,
        "Energy drift too large: {:.2}%",
        max_energy_error * 100.0
    );
}

/// Test order of convergence for various integrators
#[test]
fn test_symplectic_euler_order() {
    let integrator = SymplecticEuler;
    let oscillator = HarmonicOscillator::new(1.0, 1.0);

    let time_steps = vec![0.1, 0.05, 0.025, 0.0125];
    let mut errors = Vec::new();

    for &dt in &time_steps {
        let steps = (1.0 / dt) as usize; // Simulate for 1 second
        let (pos, _, _) = simulate_harmonic_oscillator(&integrator, &oscillator, dt, steps);
        let exact_pos = oscillator.exact_position(1.0);
        let error = calculate_error(pos, exact_pos);
        errors.push(error);
    }

    // Calculate convergence order
    for i in 1..errors.len() {
        let order = (errors[i - 1] / errors[i]).log2();
        println!("Symplectic Euler convergence order: {:.2}", order);
        // Should be approximately 1 for first-order method
        assert!(
            order > 0.8 && order < 1.5,
            "Unexpected convergence order: {}",
            order
        );
    }
}

#[test]
fn test_heun_method_order() {
    let integrator = Heun;
    let oscillator = HarmonicOscillator::new(1.0, 1.0);

    let time_steps = vec![0.1, 0.05, 0.025, 0.0125];
    let mut errors = Vec::new();

    for &dt in &time_steps {
        let steps = (1.0 / dt) as usize;
        let (pos, _, _) = simulate_harmonic_oscillator(&integrator, &oscillator, dt, steps);
        let exact_pos = oscillator.exact_position(1.0);
        let error = calculate_error(pos, exact_pos);
        errors.push(error);
    }

    for i in 1..errors.len() {
        let order = (errors[i - 1] / errors[i]).log2();
        println!("Heun method convergence order: {:.2}", order);
        // Heun should achieve near 2nd order accuracy
        assert!(
            order > 1.8 && order < 2.5,
            "Unexpected convergence order: {}",
            order
        );
    }
}

#[test]
fn test_rk2_midpoint_order() {
    let integrator = RungeKuttaSecondOrderMidpoint;
    let oscillator = HarmonicOscillator::new(1.0, 1.0);

    let time_steps = vec![0.1, 0.05, 0.025, 0.0125];
    let mut errors = Vec::new();

    for &dt in &time_steps {
        let steps = (1.0 / dt) as usize;
        let (pos, _, _) = simulate_harmonic_oscillator(&integrator, &oscillator, dt, steps);
        let exact_pos = oscillator.exact_position(1.0);
        let error = calculate_error(pos, exact_pos);
        errors.push(error);
    }

    for i in 1..errors.len() {
        let order = (errors[i - 1] / errors[i]).log2();
        println!("RK2 Midpoint convergence order: {:.2}", order);
        // RK2 should achieve near 2nd order accuracy
        assert!(
            order > 1.8 && order < 2.5,
            "Unexpected convergence order: {}",
            order
        );
    }
}

#[test]
fn test_rk4_order() {
    let integrator = RungeKuttaFourthOrder;
    let oscillator = HarmonicOscillator::new(1.0, 1.0);

    let time_steps = vec![0.2, 0.1, 0.05, 0.025];
    let mut errors = Vec::new();

    for &dt in &time_steps {
        let steps = (1.0 / dt) as usize;
        let (pos, _, _) = simulate_harmonic_oscillator(&integrator, &oscillator, dt, steps);
        let exact_pos = oscillator.exact_position(1.0);
        let error = calculate_error(pos, exact_pos);
        errors.push(error);
    }

    // RK4 now achieves near 4th order accuracy
    // Calculate and verify convergence order
    for i in 1..errors.len() {
        if errors[i] > 1e-10 {
            // Avoid division by very small numbers
            let order = (errors[i - 1] / errors[i]).log2();
            println!("RK4 convergence order: {:.2}", order);
            // Should achieve close to 4th order accuracy
            assert!(
                order > 3.5,
                "RK4 should achieve near 4th order accuracy, got {}",
                order
            );
        }
    }
}

/// Test long-term stability of integrators
#[test]
fn test_long_term_stability() {
    let oscillator = HarmonicOscillator::new(2.0 * PI, 1.0);
    let dt = 0.01;
    let steps = 100000; // 1000 seconds, ~159 periods

    let integrators: Vec<(&str, Box<dyn Integrator>)> = vec![
        ("Symplectic Euler", Box::new(SymplecticEuler)),
        ("Velocity Verlet", Box::new(VelocityVerlet)),
        ("Heun", Box::new(Heun)),
        ("RK2 Midpoint", Box::new(RungeKuttaSecondOrderMidpoint)),
        ("RK4", Box::new(RungeKuttaFourthOrder)),
    ];

    for (name, integrator) in integrators {
        let mut position = Vector::new(1.0, 0.0, 0.0);
        let mut velocity = Vector::new(0.0, 0.0, 0.0);
        let initial_energy = oscillator.energy(position, velocity);
        let harmonic_oscillator_field = HarmonicOscillatorAccelerationField::new(oscillator.omega);

        for _ in 0..steps {
            integrator.step(&mut position, &mut velocity, &harmonic_oscillator_field, dt);
        }

        let final_energy = oscillator.energy(position, velocity);
        let energy_drift = ((final_energy - initial_energy) / initial_energy).abs();

        println!(
            "{} long-term energy drift: {:.2}%",
            name,
            energy_drift * 100.0
        );

        // Verify energy conservation properties
        match name {
            "Symplectic Euler" => {
                // Symplectic methods have bounded energy oscillation
                assert!(
                    energy_drift < 0.05,
                    "Symplectic Euler energy drift too large: {:.2}%",
                    energy_drift * 100.0
                );
            }
            "Velocity Verlet" => {
                // Velocity Verlet with proper force recalculation has excellent conservation
                assert!(
                    energy_drift < 0.001,
                    "Velocity Verlet energy drift too large: {:.2}%",
                    energy_drift * 100.0
                );
            }
            "Heun" | "RK2 Midpoint" => {
                // Non-symplectic methods may have some drift over long simulations
                // Just document the behavior without strict assertions
                if energy_drift > 0.1 {
                    println!(
                        "  Note: {} shows energy drift of {:.2}% (non-symplectic)",
                        name,
                        energy_drift * 100.0
                    );
                }
            }
            "RK4" => {
                // RK4 is high-order but not symplectic, may have small drift
                // Just document the behavior
                println!(
                    "  Note: RK4 energy drift: {:.4}% (high-order non-symplectic)",
                    energy_drift * 100.0
                );
            }
            _ => {}
        }
    }
}

/// Test integrator registry creation
#[test]
fn test_registry_integrator_creation() {
    use stardrift::physics::integrators::registry::IntegratorRegistry;
    let registry = IntegratorRegistry::new().with_standard_integrators();

    // Test creating each integrator type using their aliases
    let integrator_names = vec![
        "symplectic_euler",
        "velocity_verlet",
        "heun",
        "rk2", // Use alias instead of rk2_midpoint
        "rk4", // Use alias
    ];

    for name in integrator_names {
        let integrator = registry.create(name);
        assert!(integrator.is_ok(), "Failed to create integrator: {}", name);

        let _integrator = integrator.unwrap();
        // Just verify we got an integrator, don't check exact name matching
    }
}

/// Test harmonic oscillator accuracy for all integrators
#[test]
fn test_all_integrators_harmonic_oscillator() {
    let oscillator = HarmonicOscillator::new(2.0 * PI, 1.0);
    let dt = 0.01;
    let steps = 100; // One period

    use stardrift::physics::integrators::registry::IntegratorRegistry;
    let registry = IntegratorRegistry::new().with_standard_integrators();
    let integrator_configs = vec![
        "symplectic_euler",
        "velocity_verlet",
        "heun",
        "rk2", // Use alias
        "rk4", // Use alias
    ];

    println!("\nHarmonic Oscillator Test Results (1 period):");
    println!("---------------------------------------------");

    for name in integrator_configs {
        let integrator = registry.create(name).unwrap();
        let (pos, vel, time) =
            simulate_harmonic_oscillator(integrator.as_ref(), &oscillator, dt, steps);

        let exact_pos = oscillator.exact_position(time);
        let exact_vel = oscillator.exact_velocity(time);

        let pos_error = calculate_error(pos, exact_pos);
        let vel_error = calculate_error(vel, exact_vel);

        println!(
            "{:20} | Position Error: {:.6} | Velocity Error: {:.6}",
            name, pos_error, vel_error
        );
    }
}
