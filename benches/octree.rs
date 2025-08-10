use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use stardrift::config::SimulationConfig;
use stardrift::physics::math::{Scalar, Vector};
use stardrift::physics::octree::{Octree, OctreeBody};
use std::f64::consts;
use std::hint::black_box;

/// Generate test bodies with proper spherical distribution matching the actual simulation
fn generate_test_bodies_spherical(count: usize, seed: u64, radius: Scalar) -> Vec<OctreeBody> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut bodies = Vec::with_capacity(count);

    for i in 0..count {
        // Proper spherical coordinate generation (matching physics::math::random_unit_vector)
        let theta = rng.random_range(0.0..=2.0 * consts::PI);
        let phi = libm::acos(rng.random_range(-1.0..=1.0));
        let r = rng.random_range(0.0..radius);

        let position = Vector::new(
            r * libm::sin(phi) * libm::cos(theta),
            r * libm::sin(phi) * libm::sin(theta),
            r * libm::cos(phi),
        );

        let mass = rng.random_range(1.0..100.0);
        bodies.push(OctreeBody {
            position,
            mass,
            entity: bevy::ecs::entity::Entity::from_raw(i as u32),
        });
    }

    bodies
}

// =============================================================================
// Construction Performance Benchmarks
// =============================================================================

fn bench_construction_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("construction_scaling");

    // Test O(n log n) scaling with powers of 10
    let body_counts = [10, 100, 1_000, 10_000, 100_000];
    let config = SimulationConfig::default();
    let physics = &config.physics;

    for &count in &body_counts {
        let bodies = generate_test_bodies_spherical(count, 42, 500.0);

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::new("bodies", count), &count, |b, _| {
            b.iter(|| {
                let mut octree = Octree::new(
                    physics.octree_theta,
                    physics.force_calculation_min_distance,
                    physics.force_calculation_max_force,
                );
                octree.build(black_box(bodies.iter().copied()));
                black_box(octree);
            });
        });
    }

    group.finish();
}

fn bench_construction_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("construction_memory");

    // Test memory efficiency with different leaf thresholds
    let leaf_thresholds = [1, 4, 10, 20, 50, 100];
    let body_count = 10_000;
    let bodies = generate_test_bodies_spherical(body_count, 42, 500.0);

    for &threshold in &leaf_thresholds {
        group.throughput(Throughput::Elements(body_count as u64));
        group.bench_with_input(
            BenchmarkId::new("leaf_threshold", threshold),
            &threshold,
            |b, &threshold_val| {
                b.iter(|| {
                    let mut octree = Octree::new(0.5, 10.0, 1e4).with_leaf_threshold(threshold_val);
                    octree.build(black_box(bodies.iter().copied()));
                    black_box(octree);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Force Calculation Performance Benchmarks
// =============================================================================

fn bench_force_calculation_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("force_calculation_scaling");

    let body_counts = [10, 100, 1_000, 10_000];
    let config = SimulationConfig::default();
    let physics = &config.physics;

    for &count in &body_counts {
        let bodies = generate_test_bodies_spherical(count, 42, 500.0);
        let mut octree = Octree::new(
            physics.octree_theta,
            physics.force_calculation_min_distance,
            physics.force_calculation_max_force,
        );
        octree.build(bodies.iter().copied());

        // Measure force calculation per body (should be O(log n))
        group.throughput(Throughput::Elements(1)); // Per-body throughput
        group.bench_with_input(BenchmarkId::new("bodies", count), &count, |b, _| {
            let test_body = &bodies[count / 2]; // Middle body
            b.iter(|| {
                let force = octree.calculate_force_at_position(
                    black_box(test_body.position),
                    black_box(test_body.mass),
                    black_box(test_body.entity),
                    physics.gravitational_constant,
                );
                black_box(force);
            });
        });
    }

    group.finish();
}

fn bench_theta_accuracy_tradeoff(c: &mut Criterion) {
    let mut group = c.benchmark_group("theta_accuracy_tradeoff");

    // Test accuracy vs performance trade-off
    let theta_values = [0.1, 0.3, 0.5, 0.8, 1.0, 1.5, 2.0];
    let body_count = 5_000;
    let bodies = generate_test_bodies_spherical(body_count, 42, 500.0);
    let g = 10.0;

    for &theta in &theta_values {
        let mut octree = Octree::new(theta, 10.0, 1e4);
        octree.build(bodies.iter().copied());

        group.throughput(Throughput::Elements(body_count as u64));
        group.bench_with_input(
            BenchmarkId::new("theta", (theta * 100.0) as u32),
            &theta,
            |b, _| {
                b.iter(|| {
                    let mut total_force = Vector::ZERO;
                    for body in &bodies {
                        let force = octree.calculate_force_at_position(
                            black_box(body.position),
                            black_box(body.mass),
                            black_box(body.entity),
                            g,
                        );
                        total_force += force;
                    }
                    black_box(total_force);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Real-World Scenario Benchmarks
// =============================================================================

fn bench_realworld_60fps_target(c: &mut Criterion) {
    let mut group = c.benchmark_group("realworld_60fps");

    // Target: Complete physics cycle in < 16.67ms
    let body_counts = [50, 100, 150, 200, 300];
    let config = SimulationConfig::default();
    let physics = &config.physics;

    for &count in &body_counts {
        let bodies = generate_test_bodies_spherical(count, 42, 300.0);

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::new("bodies", count), &count, |b, _| {
            b.iter(|| {
                // Full physics cycle
                let mut octree = Octree::new(
                    physics.octree_theta,
                    physics.force_calculation_min_distance,
                    physics.force_calculation_max_force,
                );
                octree.build(black_box(bodies.iter().copied()));

                let mut forces = Vec::with_capacity(bodies.len());
                for body in &bodies {
                    let force = octree.calculate_force_at_position(
                        body.position,
                        body.mass,
                        body.entity,
                        physics.gravitational_constant,
                    );
                    forces.push(force);
                }

                black_box((octree, forces));
            });
        });
    }

    group.finish();
}

// =============================================================================
// Special Performance Characteristics
// =============================================================================

fn bench_stats_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("stats_overhead");

    let body_counts = [100, 1_000, 10_000];

    for &count in &body_counts {
        let bodies = generate_test_bodies_spherical(count, 42, 500.0);
        let mut octree = Octree::new(0.5, 10.0, 1e4);
        octree.build(bodies.iter().copied());

        group.bench_with_input(BenchmarkId::new("bodies", count), &count, |b, _| {
            b.iter(|| {
                let stats = octree.octree_stats();
                black_box(stats);
            });
        });
    }

    group.finish();
}

// =============================================================================
// Benchmark Groups
// =============================================================================

criterion_group!(
    construction,
    bench_construction_scaling,
    bench_construction_memory
);

criterion_group!(
    physics,
    bench_force_calculation_scaling,
    bench_theta_accuracy_tradeoff
);

criterion_group!(realworld, bench_realworld_60fps_target);

criterion_group!(characteristics, bench_stats_overhead);

criterion_main!(construction, physics, realworld, characteristics);
