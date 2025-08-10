//! Comprehensive integrator benchmarks
//!
//! This benchmark suite tests various aspects of numerical integrators:
//! - Performance (speed/throughput)
//! - Accuracy (error vs analytical solutions, convergence order)
//! - Stability (energy conservation, orbital mechanics)
//! - Work-precision (accuracy vs computational cost)
//! - Real N-body scenarios

use criterion::{BenchmarkId, Criterion, PlotConfiguration, criterion_group, criterion_main};
use std::hint::black_box;

extern crate stardrift;
use stardrift::physics::integrators::{
    ForceEvaluator, Heun, Integrator, RungeKuttaFourthOrder, RungeKuttaSecondOrderMidpoint,
    SymplecticEuler, VelocityVerlet,
};
use stardrift::physics::math::{Scalar, Vector};
use stardrift::physics::octree::{Octree, OctreeBody};

const PI: Scalar = std::f64::consts::PI;

// =============================================================================
// Helper Functions and Test Scenarios
// =============================================================================

/// Get all integrators to test
fn get_integrators() -> Vec<(&'static str, Box<dyn Integrator>)> {
    vec![
        ("symplectic_euler", Box::new(SymplecticEuler)),
        ("velocity_verlet", Box::new(VelocityVerlet)),
        ("heun", Box::new(Heun)),
        ("rk2_midpoint", Box::new(RungeKuttaSecondOrderMidpoint)),
        ("rk4", Box::new(RungeKuttaFourthOrder)),
    ]
}

/// Get integrators with their expected convergence orders
fn get_integrators_with_order() -> Vec<(&'static str, Box<dyn Integrator>, usize)> {
    vec![
        ("symplectic_euler", Box::new(SymplecticEuler), 1),
        ("velocity_verlet", Box::new(VelocityVerlet), 2),
        ("heun", Box::new(Heun), 2),
        ("rk2_midpoint", Box::new(RungeKuttaSecondOrderMidpoint), 2),
        ("rk4", Box::new(RungeKuttaFourthOrder), 4),
    ]
}

/// Harmonic oscillator for accuracy testing
struct HarmonicOscillator {
    omega: Scalar,
}

impl ForceEvaluator for HarmonicOscillator {
    fn calc_acceleration(&self, position: Vector) -> Vector {
        -self.omega * self.omega * position
    }
}

/// Kepler problem (central force)
struct KeplerProblem {
    mu: Scalar, // GM for central body
}

impl ForceEvaluator for KeplerProblem {
    fn calc_acceleration(&self, position: Vector) -> Vector {
        let r = position.length();
        if r > 1e-10 {
            -position * (self.mu / (r * r * r))
        } else {
            Vector::ZERO
        }
    }
}

/// N-body evaluator using octree
struct NBodyEvaluator<'a> {
    octree: &'a Octree,
    entity: bevy::ecs::entity::Entity,
    mass: Scalar,
    g: Scalar,
}

impl<'a> ForceEvaluator for NBodyEvaluator<'a> {
    fn calc_acceleration(&self, position: Vector) -> Vector {
        let force =
            self.octree
                .calculate_force_at_position(position, self.mass, self.entity, self.g);
        force / self.mass
    }
}

// =============================================================================
// Performance Benchmarks (Raw Speed)
// =============================================================================

fn bench_integrator_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("integrator_performance");
    group
        .plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

    let integrators = get_integrators();
    let evaluator = HarmonicOscillator { omega: 1.0 };

    for (name, integrator) in &integrators {
        group.bench_function(*name, |b| {
            let mut position = Vector::new(1.0, 0.0, 0.0);
            let mut velocity = Vector::new(0.0, 1.0, 0.0);
            let dt = 0.01;

            b.iter(|| {
                integrator.step(&mut position, &mut velocity, &evaluator, dt);
                black_box((position, velocity));
            });
        });
    }

    group.finish();
}

// =============================================================================
// Accuracy Benchmarks
// =============================================================================

fn bench_integrator_accuracy(c: &mut Criterion) {
    let mut group = c.benchmark_group("integrator_accuracy");

    let integrators = get_integrators();

    // Test with harmonic oscillator (has analytical solution)
    let oscillator = HarmonicOscillator { omega: 2.0 * PI };

    for (name, integrator) in &integrators {
        group.bench_function(BenchmarkId::new("harmonic", *name), |b| {
            b.iter_custom(|iters| {
                let mut total_duration = std::time::Duration::from_secs(0);
                let mut _total_error = 0.0;

                for _ in 0..iters {
                    let mut position = Vector::new(1.0, 0.0, 0.0);
                    let mut velocity = Vector::new(0.0, 0.0, 0.0);
                    let dt = 0.01;
                    let steps = 100; // One period

                    let start = std::time::Instant::now();
                    for _ in 0..steps {
                        integrator.step(&mut position, &mut velocity, &oscillator, dt);
                    }
                    total_duration += start.elapsed();

                    // Compare with analytical solution
                    let t = dt * steps as Scalar;
                    let exact_pos = Vector::new((oscillator.omega * t).cos(), 0.0, 0.0);
                    let error = (position - exact_pos).length();
                    _total_error += error;
                }

                // Return time taken (criterion will compute statistics)
                total_duration
            });
        });
    }

    group.finish();
}

fn bench_convergence_order(c: &mut Criterion) {
    let mut group = c.benchmark_group("convergence_order");
    group.sample_size(10);

    let integrators = get_integrators_with_order();
    let oscillator = HarmonicOscillator { omega: 1.0 };
    let timesteps = vec![0.1, 0.05, 0.025, 0.0125];

    for (name, integrator, expected_order) in &integrators {
        group.bench_function(*name, |b| {
            b.iter_custom(|iters| {
                let mut total_duration = std::time::Duration::from_secs(0);

                for _ in 0..iters {
                    let mut errors = Vec::new();

                    let start = std::time::Instant::now();
                    for &dt in &timesteps {
                        let mut position = Vector::new(1.0, 0.0, 0.0);
                        let mut velocity = Vector::new(0.0, 0.0, 0.0);
                        let steps = (1.0 / dt) as usize; // Simulate for 1 time unit

                        for _ in 0..steps {
                            integrator.step(&mut position, &mut velocity, &oscillator, dt);
                        }

                        let exact_pos = Vector::new(1.0_f64.cos(), 0.0, 0.0);
                        let error = (position - exact_pos).length();
                        errors.push(error);
                    }
                    total_duration += start.elapsed();

                    // Calculate convergence order
                    let mut orders = Vec::new();
                    for i in 1..errors.len() {
                        if errors[i] > 1e-10 {
                            let order = (errors[i - 1] / errors[i]).log2();
                            orders.push(order);
                        }
                    }

                    // Check if we're close to expected order
                    if !orders.is_empty() {
                        let avg_order = orders.iter().sum::<Scalar>() / orders.len() as Scalar;
                        let _order_error = (avg_order - *expected_order as Scalar).abs();
                        black_box(_order_error);
                    }
                }

                total_duration
            });
        });
    }

    group.finish();
}

// =============================================================================
// Stability Benchmarks (Conservation Properties)
// =============================================================================

fn bench_integrator_stability(c: &mut Criterion) {
    let mut group = c.benchmark_group("integrator_stability");
    group.sample_size(10); // Reduce sample size for long-running benchmarks

    let integrators = get_integrators();
    let oscillator = HarmonicOscillator { omega: 2.0 * PI };

    for (name, integrator) in &integrators {
        group.bench_function(BenchmarkId::new("energy_drift", *name), |b| {
            b.iter_custom(|iters| {
                let mut total_duration = std::time::Duration::from_secs(0);

                for _ in 0..iters {
                    let mut position = Vector::new(1.0, 0.0, 0.0);
                    let mut velocity = Vector::new(0.0, 0.0, 0.0);
                    let dt = 0.01;
                    let steps = 10000; // Long simulation

                    let initial_energy = 0.5 * oscillator.omega * oscillator.omega;

                    let start = std::time::Instant::now();
                    for _ in 0..steps {
                        integrator.step(&mut position, &mut velocity, &oscillator, dt);
                    }
                    total_duration += start.elapsed();

                    let final_energy = 0.5 * velocity.length_squared()
                        + 0.5 * oscillator.omega * oscillator.omega * position.length_squared();
                    let _energy_drift = ((final_energy - initial_energy) / initial_energy).abs();

                    black_box(_energy_drift);
                }

                total_duration
            });
        });
    }

    group.finish();
}

fn bench_kepler_orbit(c: &mut Criterion) {
    let mut group = c.benchmark_group("kepler_orbit");
    group.sample_size(10);

    let integrators = get_integrators();

    // Circular orbit parameters
    let mu: Scalar = 1.0; // GM
    let radius: Scalar = 1.0;
    let orbital_velocity = (mu / radius).sqrt();
    let kepler = KeplerProblem { mu };

    for (name, integrator) in &integrators {
        group.bench_function(*name, |b| {
            b.iter_custom(|iters| {
                let mut total_duration = std::time::Duration::from_secs(0);

                for _ in 0..iters {
                    let mut position = Vector::new(radius, 0.0, 0.0);
                    let mut velocity = Vector::new(0.0, orbital_velocity, 0.0);
                    let dt = 0.01;
                    let orbital_period: Scalar = 2.0 * PI * (radius.powi(3) / mu).sqrt();
                    let steps = (orbital_period / dt) as usize;

                    let initial_energy = -mu / (2.0 * radius); // Specific orbital energy
                    let initial_angular_momentum = radius * orbital_velocity;

                    let start = std::time::Instant::now();
                    for _ in 0..steps {
                        integrator.step(&mut position, &mut velocity, &kepler, dt);
                    }
                    total_duration += start.elapsed();

                    // Check conservation
                    let r = position.length();
                    let v = velocity.length();
                    let final_energy = 0.5 * v * v - mu / r;
                    let final_angular_momentum = position.cross(velocity).length();

                    let _energy_error =
                        ((final_energy - initial_energy) / initial_energy.abs()).abs();
                    let _angular_momentum_error = ((final_angular_momentum
                        - initial_angular_momentum)
                        / initial_angular_momentum)
                        .abs();

                    black_box((_energy_error, _angular_momentum_error));
                }

                total_duration
            });
        });
    }

    group.finish();
}

// =============================================================================
// Work-Precision Benchmarks (Cost vs Accuracy)
// =============================================================================

fn bench_work_precision(c: &mut Criterion) {
    let mut group = c.benchmark_group("work_precision");
    group.sample_size(20);

    // Test different timesteps to show accuracy vs computation tradeoff
    let timesteps = vec![0.1, 0.05, 0.01, 0.005, 0.001];
    let integrators = get_integrators();

    let oscillator = HarmonicOscillator { omega: 1.0 };
    let final_time = 10.0; // Simulate for 10 time units

    for (name, integrator) in &integrators {
        for &dt in &timesteps {
            let steps = (final_time / dt) as usize;

            group.bench_function(BenchmarkId::new(*name, format!("dt_{:.3}", dt)), |b| {
                b.iter_custom(|iters| {
                    let mut total_duration = std::time::Duration::from_secs(0);

                    for _ in 0..iters {
                        let mut position = Vector::new(1.0, 0.0, 0.0);
                        let mut velocity = Vector::new(0.0, 0.0, 0.0);

                        let start = std::time::Instant::now();
                        for _ in 0..steps {
                            integrator.step(&mut position, &mut velocity, &oscillator, dt);
                        }
                        total_duration += start.elapsed();

                        // Calculate error
                        let exact_pos = Vector::new(final_time.cos(), 0.0, 0.0);
                        let _error = (position - exact_pos).length();
                        black_box(_error);
                    }

                    total_duration
                });
            });
        }
    }

    group.finish();
}

// =============================================================================
// Real N-Body Benchmarks
// =============================================================================

fn bench_nbody_realistic(c: &mut Criterion) {
    let mut group = c.benchmark_group("nbody_realistic");
    group.sample_size(10);

    // Generate a small cluster of bodies
    let body_count = 100;
    let mut bodies = Vec::with_capacity(body_count);

    // Create a simple cluster with deterministic positions
    for i in 0..body_count {
        let angle = (i as Scalar) * 2.0 * PI / (body_count as Scalar);
        let radius = 10.0 + (i as Scalar) * 0.4; // Spread from 10 to 50
        let z = ((i as Scalar) - 50.0) * 0.1; // Spread from -5 to 5

        bodies.push(OctreeBody {
            position: Vector::new(radius * angle.cos(), radius * angle.sin(), z),
            mass: 1.0 + (i as Scalar) * 0.015, // Mass from 0.5 to 2.0
            entity: bevy::ecs::entity::Entity::from_raw(i as u32),
        });
    }

    // Build octree
    let mut octree = Octree::new(0.5, 0.01, 1e6);
    octree.build(bodies.iter().copied());

    let integrators = get_integrators();

    for (name, integrator) in &integrators {
        group.bench_function(*name, |b| {
            // Test on middle body
            let test_body = &bodies[body_count / 2];
            let evaluator = NBodyEvaluator {
                octree: &octree,
                entity: test_body.entity,
                mass: test_body.mass,
                g: 1.0,
            };

            b.iter(|| {
                let mut position = test_body.position;
                let mut velocity = Vector::new(0.1, 0.0, 0.0); // Small initial velocity
                let dt = 0.01;

                for _ in 0..10 {
                    integrator.step(&mut position, &mut velocity, &evaluator, dt);
                }

                black_box((position, velocity));
            });
        });
    }

    group.finish();
}

// =============================================================================
// Benchmark Groups
// =============================================================================

criterion_group!(performance, bench_integrator_performance);

criterion_group!(accuracy, bench_integrator_accuracy, bench_convergence_order);

criterion_group!(stability, bench_integrator_stability, bench_kepler_orbit);

criterion_group!(precision, bench_work_precision);

criterion_group!(realistic, bench_nbody_realistic);

criterion_main!(performance, accuracy, stability, precision, realistic);
