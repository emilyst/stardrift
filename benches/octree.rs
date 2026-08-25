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
            entity: bevy::ecs::entity::Entity::from_raw_u32(i as u32).unwrap(),
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
// Benchmark Groups
// =============================================================================

criterion_group!(construction, bench_construction_scaling);

criterion_group!(physics, bench_force_calculation_scaling);

criterion_group!(realworld, bench_realworld_60fps_target);

criterion_main!(construction, physics, realworld);
