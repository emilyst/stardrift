//! Comprehensive integrator benchmarks
//!
//! This benchmark suite tests various aspects of numerical integrators:
//! - Performance (speed/throughput) - Lower time is better
//! - Accuracy (error vs analytical solutions) - Lower values are better
//! - Convergence order (error reduction with smaller timesteps) - Lower deviation is better
//! - Stability (energy conservation over long simulations) - Lower drift is better
//! - Work-precision (accuracy for different timesteps) - Lower error is better
//! - Real N-body scenarios (performance with octree) - Lower time is better
//!
//! Note: Accuracy benchmarks report error values as durations (scaled by 1e9)
//! to work with Criterion's framework. Lower values indicate better accuracy.

use criterion::{BenchmarkId, Criterion, PlotConfiguration, criterion_group, criterion_main};
use std::hint::black_box;

extern crate stardrift;
use stardrift::physics::integrators::{AccelerationField, Integrator};
use stardrift::physics::math::{Scalar, Vector};
use stardrift::physics::octree::{Octree, OctreeBody};

const PI: Scalar = std::f64::consts::PI;

// =============================================================================
// Acceleration Functions for Benchmarks
// =============================================================================

/// Harmonic oscillator / Spring force: a = -k * x
/// where k is the spring constant (or omega^2 for oscillators)
struct HarmonicOscillator {
    pub k: Scalar,
}

impl HarmonicOscillator {
    /// Create from angular frequency omega (k = omega^2)
    pub fn from_omega(omega: Scalar) -> Self {
        Self { k: omega * omega }
    }
}

impl AccelerationField for HarmonicOscillator {
    fn at(&self, position: Vector) -> Vector {
        -self.k * position
    }
}

/// Central force problem (e.g., Kepler orbits): a = -μ/r³ * r_vec
/// where μ = GM for gravitational problems
struct CentralForce {
    pub mu: Scalar,
}

impl AccelerationField for CentralForce {
    fn at(&self, position: Vector) -> Vector {
        let r = position.length();
        if r > 1e-10 {
            -position * (self.mu / (r * r * r))
        } else {
            Vector::ZERO
        }
    }
}

/// N-body acceleration field using octree for efficient force calculation
/// Useful for testing integrators with realistic many-body forces
struct NBodyAcceleration<'a> {
    pub octree: &'a Octree,
    pub entity: bevy::ecs::entity::Entity,
    pub mass: Scalar,
    pub g: Scalar,
}

impl<'a> AccelerationField for NBodyAcceleration<'a> {
    fn at(&self, position: Vector) -> Vector {
        let force =
            self.octree
                .calculate_force_at_position(position, self.mass, self.entity, self.g);
        force / self.mass
    }
}

// =============================================================================
// Helper Functions and Test Scenarios
// =============================================================================

/// Get all integrators to test using the registry
fn get_integrators() -> Vec<(String, Box<dyn Integrator>)> {
    use stardrift::physics::integrators::registry::IntegratorRegistry;
    let registry = IntegratorRegistry::new().with_standard_integrators();
    let mut integrators = Vec::new();

    // Get all available integrators from the registry
    for name in registry.list_available() {
        if let Ok(integrator) = registry.create(&name) {
            integrators.push((name, integrator));
        }
    }

    integrators
}

/// Get integrators with their expected convergence orders
fn get_integrators_with_order() -> Vec<(String, Box<dyn Integrator>, usize)> {
    use stardrift::physics::integrators::registry::IntegratorRegistry;
    let registry = IntegratorRegistry::new().with_standard_integrators();
    let mut integrators = Vec::new();

    for name in registry.list_available() {
        if let Ok(integrator) = registry.create(&name) {
            let order = integrator.convergence_order();
            integrators.push((name, integrator, order));
        }
    }

    integrators
}

// =============================================================================
// Performance Benchmarks (Raw Speed)
// =============================================================================

fn bench_integrator_performance(c: &mut Criterion) {
    // Measures raw computational speed of integrators (time per step)
    let mut group = c.benchmark_group("integrator_performance");
    group
        .plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

    let integrators = get_integrators();
    let oscillator = HarmonicOscillator::from_omega(1.0);

    for (name, integrator) in &integrators {
        group.bench_function(name.as_str(), |b| {
            b.iter(|| {
                let mut position = black_box(Vector::new(1.0, 0.0, 0.0));
                let mut velocity = black_box(Vector::new(0.0, 1.0, 0.0));
                let dt = black_box(0.01);

                integrator.step(&mut position, &mut velocity, &oscillator, dt);
                black_box(position);
                black_box(velocity);
            });
        });
    }

    group.finish();
}

// =============================================================================
// Accuracy Benchmarks
// =============================================================================

fn bench_integrator_accuracy(c: &mut Criterion) {
    // Measures position error after one period of harmonic oscillation
    let mut group = c.benchmark_group("integrator_accuracy");
    // Configure plot for error values (log scale is useful for errors)
    group
        .plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

    let integrators = get_integrators();

    // Test with harmonic oscillator (has analytical solution)
    let oscillator = HarmonicOscillator::from_omega(2.0 * PI);

    for (name, integrator) in &integrators {
        group.bench_function(BenchmarkId::new("harmonic", name.as_str()), |b| {
            b.iter_custom(|iters| {
                let mut total_error = 0.0;

                for _ in 0..iters {
                    let mut position = Vector::new(1.0, 0.0, 0.0);
                    let mut velocity = Vector::new(0.0, 0.0, 0.0);
                    let dt = 0.01;
                    let steps = 100; // One period (T = 2π/ω = 2π/(2π) = 1.0 seconds)

                    for _ in 0..steps {
                        integrator.step(&mut position, &mut velocity, &oscillator, dt);
                    }

                    // Compare with analytical solution
                    let t = dt * steps as Scalar;
                    // For harmonic oscillator: omega = sqrt(k)
                    let omega = oscillator.k.sqrt();
                    let exact_pos = Vector::new((omega * t).cos(), 0.0, 0.0);
                    let error = (position - exact_pos).length();
                    total_error += error;
                }

                // Return average error as a Duration (nanoseconds as proxy for error magnitude)
                // Scale up by 1e9 to avoid zero durations for very small errors
                // This allows Criterion to handle it properly while showing error values
                let avg_error = total_error / iters as f64;
                std::time::Duration::from_nanos((avg_error * 1e9) as u64)
            });
        });
    }

    group.finish();
}

fn bench_convergence_order(c: &mut Criterion) {
    // Verifies that integrators achieve their theoretical convergence order
    // (how error decreases as timestep decreases)
    let mut group = c.benchmark_group("convergence_order");
    group.sample_size(10);
    // Log scale for error visualization
    group
        .plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

    let integrators = get_integrators_with_order();
    let oscillator = HarmonicOscillator::from_omega(1.0);
    let timesteps = vec![0.1, 0.05, 0.025, 0.0125];

    for (name, integrator, expected_order) in &integrators {
        group.bench_function(name.as_str(), |b| {
            b.iter_custom(|iters| {
                let mut total_order_error = 0.0;

                for _ in 0..iters {
                    let mut errors = Vec::new();

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

                    // Calculate convergence order (since timesteps halve each time)
                    let mut orders = Vec::new();
                    for i in 1..errors.len() {
                        if errors[i] > 1e-10 && errors[i - 1] > 1e-10 {
                            // Order = log(error_ratio) / log(timestep_ratio)
                            // Since timesteps halve: log2(error_ratio)
                            let order = (errors[i - 1] / errors[i]).log2();
                            orders.push(order);
                        }
                    }

                    // Check if we're close to expected order
                    if !orders.is_empty() {
                        let avg_order = orders.iter().sum::<Scalar>() / orders.len() as Scalar;
                        let order_error = (avg_order - *expected_order as Scalar).abs();
                        total_order_error += order_error;
                    } else {
                        // If we can't calculate order (due to very small errors), assume good convergence
                        total_order_error += 0.01;
                    }
                }

                // Return average convergence order error as Duration
                let avg_order_error = total_order_error / iters as f64;
                std::time::Duration::from_nanos((avg_order_error * 1e9) as u64)
            });
        });
    }

    group.finish();
}

// =============================================================================
// Stability Benchmarks (Conservation Properties)
// =============================================================================

fn bench_integrator_stability(c: &mut Criterion) {
    // Measures energy conservation over long simulations (10,000 steps)
    // Symplectic integrators should show better long-term stability
    let mut group = c.benchmark_group("integrator_stability");
    group.sample_size(10); // Reduce sample size for long-running benchmarks
    // Log scale for energy drift visualization
    group
        .plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

    let integrators = get_integrators();
    let oscillator = HarmonicOscillator::from_omega(2.0 * PI);

    for (name, integrator) in &integrators {
        group.bench_function(BenchmarkId::new("energy_drift", name.as_str()), |b| {
            b.iter_custom(|iters| {
                let mut total_energy_drift = 0.0;

                for _ in 0..iters {
                    let mut position = Vector::new(1.0, 0.0, 0.0);
                    let mut velocity = Vector::new(0.0, 0.0, 0.0);
                    let dt = 0.01;
                    let steps = 10000; // Long simulation

                    // Initial total energy: E = 0.5*k*x^2 + 0.5*v^2 (with x=1, v=0)
                    let initial_energy = 0.5 * oscillator.k * 1.0;

                    for _ in 0..steps {
                        integrator.step(&mut position, &mut velocity, &oscillator, dt);
                    }

                    let final_energy = 0.5 * velocity.length_squared()
                        + 0.5 * oscillator.k * position.length_squared();
                    let energy_drift = ((final_energy - initial_energy) / initial_energy).abs();

                    total_energy_drift += energy_drift;
                }

                // Return average energy drift as Duration
                let avg_drift = total_energy_drift / iters as f64;
                std::time::Duration::from_nanos((avg_drift * 1e9) as u64)
            });
        });
    }

    group.finish();
}

fn bench_kepler_orbit(c: &mut Criterion) {
    // Tests conservation of energy and angular momentum in circular orbit
    // Critical for astronomical simulations
    let mut group = c.benchmark_group("kepler_orbit");
    group.sample_size(10);
    // Log scale for error visualization
    group
        .plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

    let integrators = get_integrators();

    // Circular orbit parameters
    let mu: Scalar = 1.0; // GM
    let radius: Scalar = 1.0;
    let orbital_velocity = (mu / radius).sqrt();
    let kepler = CentralForce { mu };

    for (name, integrator) in &integrators {
        group.bench_function(name.as_str(), |b| {
            b.iter_custom(|iters| {
                let mut total_conservation_error = 0.0;

                for _ in 0..iters {
                    let mut position = Vector::new(radius, 0.0, 0.0);
                    let mut velocity = Vector::new(0.0, orbital_velocity, 0.0);
                    let dt = 0.01;
                    let orbital_period: Scalar = 2.0 * PI * (radius.powi(3) / mu).sqrt();
                    let steps = (orbital_period / dt) as usize;

                    // Specific orbital energy for circular orbit: E = -μ/(2r)
                    let initial_energy = -mu / (2.0 * radius);
                    let initial_angular_momentum = radius * orbital_velocity;

                    for _ in 0..steps {
                        integrator.step(&mut position, &mut velocity, &kepler, dt);
                    }

                    // Check conservation
                    let r = position.length();
                    let v = velocity.length();
                    let final_energy = 0.5 * v * v - mu / r;
                    let final_angular_momentum = position.cross(velocity).length();

                    let energy_error =
                        ((final_energy - initial_energy) / initial_energy.abs()).abs();
                    let angular_momentum_error = ((final_angular_momentum
                        - initial_angular_momentum)
                        / initial_angular_momentum)
                        .abs();

                    // Combine both errors (could also report them separately)
                    total_conservation_error += energy_error + angular_momentum_error;
                }

                // Return average conservation error as Duration
                let avg_error = total_conservation_error / iters as f64;
                std::time::Duration::from_nanos((avg_error * 1e9) as u64)
            });
        });
    }

    group.finish();
}

// =============================================================================
// Work-Precision Benchmarks (Cost vs Accuracy)
// =============================================================================

fn bench_work_precision(c: &mut Criterion) {
    // Shows accuracy achieved at different timesteps
    // Helps choose optimal timestep for desired accuracy
    let mut group = c.benchmark_group("work_precision");
    group.sample_size(20);
    // Log scale for error visualization
    group
        .plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

    // Test different timesteps to show accuracy vs computation tradeoff
    let timesteps = vec![0.1, 0.05, 0.01, 0.005, 0.001];
    let integrators = get_integrators();

    let oscillator = HarmonicOscillator::from_omega(1.0);
    let final_time = 10.0; // Simulate for 10 time units

    for (name, integrator) in &integrators {
        for &dt in &timesteps {
            let steps = (final_time / dt) as usize;

            group.bench_function(
                BenchmarkId::new(name.as_str(), format!("dt_{:.3}", dt)),
                |b| {
                    b.iter_custom(|iters| {
                        let mut total_error = 0.0;

                        for _ in 0..iters {
                            let mut position = Vector::new(1.0, 0.0, 0.0);
                            let mut velocity = Vector::new(0.0, 0.0, 0.0);

                            for _ in 0..steps {
                                integrator.step(&mut position, &mut velocity, &oscillator, dt);
                            }

                            // Calculate error
                            let exact_pos = Vector::new(final_time.cos(), 0.0, 0.0);
                            let error = (position - exact_pos).length();
                            total_error += error;
                        }

                        // Return average position error as Duration
                        let avg_error = total_error / iters as f64;
                        std::time::Duration::from_nanos((avg_error * 1e9) as u64)
                    });
                },
            );
        }
    }

    group.finish();
}

// =============================================================================
// Real N-Body Benchmarks
// =============================================================================

fn bench_nbody_realistic(c: &mut Criterion) {
    // Tests integrator performance with realistic N-body forces from octree
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
        group.bench_function(name.as_str(), |b| {
            // Test on middle body
            let test_body = &bodies[body_count / 2];
            let field = NBodyAcceleration {
                octree: &octree,
                entity: test_body.entity,
                mass: test_body.mass,
                g: 1.0,
            };

            b.iter(|| {
                let mut position = black_box(test_body.position);
                let mut velocity = black_box(Vector::new(0.1, 0.0, 0.0)); // Small initial velocity
                let dt = black_box(0.01);

                for _ in 0..10 {
                    integrator.step(&mut position, &mut velocity, &field, dt);
                }

                black_box(position);
                black_box(velocity);
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
