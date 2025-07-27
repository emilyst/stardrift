use avian3d::math::{Scalar, Vector};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use stardrift::config::SimulationConfig;
use stardrift::physics::octree::{Octree, OctreeBody};
use std::f64::consts;
use std::hint::black_box;
use std::time::Duration;

/// Generate test bodies with proper spherical distribution matching the actual simulation
fn generate_test_bodies_spherical(count: usize, seed: u64, radius: Scalar) -> Vec<OctreeBody> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut bodies = Vec::with_capacity(count);

    for _ in 0..count {
        // Proper spherical coordinate generation (matching utils::math::random_unit_vector)
        let theta = rng.random_range(0.0..=2.0 * consts::PI);
        let phi = libm::acos(rng.random_range(-1.0..=1.0));
        let r = rng.random_range(0.0..radius);

        let position = Vector::new(
            r * libm::sin(phi) * libm::cos(theta),
            r * libm::sin(phi) * libm::sin(theta),
            r * libm::cos(phi),
        );

        let mass = rng.random_range(1.0..100.0);
        bodies.push(OctreeBody { position, mass });
    }

    bodies
}

/// Generate bodies in a clustered arrangement to test cache efficiency
fn generate_clustered_bodies(count: usize, seed: u64, clusters: usize) -> Vec<OctreeBody> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut bodies = Vec::with_capacity(count);

    let bodies_per_cluster = count / clusters;
    let cluster_radius = 50.0;
    let cluster_spread = 300.0;

    for cluster in 0..clusters {
        // Generate cluster center
        let theta = rng.random_range(0.0..=2.0 * consts::PI);
        let phi = libm::acos(rng.random_range(-1.0..=1.0));
        let r = rng.random_range(100.0..cluster_spread);

        let cluster_center = Vector::new(
            r * libm::sin(phi) * libm::cos(theta),
            r * libm::sin(phi) * libm::sin(theta),
            r * libm::cos(phi),
        );

        // Generate bodies around cluster center
        let cluster_bodies = if cluster == clusters - 1 {
            count - bodies.len() // Handle remainder
        } else {
            bodies_per_cluster
        };

        for _ in 0..cluster_bodies {
            let offset_theta = rng.random_range(0.0..=2.0 * consts::PI);
            let offset_phi = libm::acos(rng.random_range(-1.0..=1.0));
            let offset_r = rng.random_range(0.0..cluster_radius);

            let offset = Vector::new(
                offset_r * libm::sin(offset_phi) * libm::cos(offset_theta),
                offset_r * libm::sin(offset_phi) * libm::sin(offset_theta),
                offset_r * libm::cos(offset_phi),
            );

            let position = cluster_center + offset;
            let mass = rng.random_range(1.0..100.0);
            bodies.push(OctreeBody { position, mass });
        }
    }

    bodies
}

/// Load configuration from file
fn load_config(profile: &str) -> SimulationConfig {
    let config_path = format!("configs/benchmark_profiles/{profile}.toml");
    SimulationConfig::load_or_default(&config_path)
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
                let force = octree.calculate_force(
                    black_box(test_body),
                    None,
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
                        let force = octree.calculate_force(black_box(body), None, g);
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
                    let force = octree.calculate_force(body, None, physics.gravitational_constant);
                    forces.push(force);
                }

                black_box((octree, forces));
            });
        });
    }

    group.finish();
}

fn bench_extreme_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("extreme_scenarios");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    let body_counts = [1_000, 5_000, 10_000, 50_000];
    let config = load_config("stress_test");
    let physics = &config.physics;

    for &count in &body_counts {
        let bodies = generate_test_bodies_spherical(count, 42, 1000.0);

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::new("bodies", count), &count, |b, _| {
            b.iter(|| {
                let mut octree = Octree::new(
                    physics.octree_theta,
                    physics.force_calculation_min_distance,
                    physics.force_calculation_max_force,
                );
                octree.build(black_box(bodies.iter().copied()));

                // Sample force calculations (not all bodies for extreme counts)
                let sample_size = (count / 10).min(100);
                let mut total_force = Vector::ZERO;
                for i in 0..sample_size {
                    let body = &bodies[i * 10];
                    let force = octree.calculate_force(body, None, physics.gravitational_constant);
                    total_force += force;
                }

                black_box((octree, total_force));
            });
        });
    }

    group.finish();
}

// =============================================================================
// Configuration Profile Benchmarks
// =============================================================================

fn bench_configuration_profiles(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_profiles");

    let profiles = ["fast_inaccurate", "balanced", "high_accuracy"];
    let body_count = 1_000;
    let bodies = generate_test_bodies_spherical(body_count, 42, 300.0);

    for profile in &profiles {
        let config = load_config(profile);
        let physics = &config.physics;

        group.throughput(Throughput::Elements(body_count as u64));
        group.bench_with_input(BenchmarkId::new("profile", profile), profile, |b, _| {
            b.iter(|| {
                let mut octree = Octree::new(
                    physics.octree_theta,
                    physics.force_calculation_min_distance,
                    physics.force_calculation_max_force,
                )
                .with_leaf_threshold(physics.octree_leaf_threshold);

                octree.build(black_box(bodies.iter().copied()));

                let mut total_force = Vector::ZERO;
                for body in &bodies {
                    let force = octree.calculate_force(body, None, physics.gravitational_constant);
                    total_force += force;
                }

                black_box((octree, total_force));
            });
        });
    }

    group.finish();
}

// =============================================================================
// Special Performance Characteristics
// =============================================================================

fn bench_spatial_arrangements(c: &mut Criterion) {
    let mut group = c.benchmark_group("spatial_arrangements");

    let body_count = 1_000;
    let arrangements = [
        (
            "uniform",
            generate_test_bodies_spherical(body_count, 42, 500.0),
        ),
        ("clustered_4", generate_clustered_bodies(body_count, 42, 4)),
        (
            "clustered_10",
            generate_clustered_bodies(body_count, 42, 10),
        ),
        (
            "clustered_50",
            generate_clustered_bodies(body_count, 42, 50),
        ),
    ];

    for (name, bodies) in &arrangements {
        group.throughput(Throughput::Elements(body_count as u64));
        group.bench_with_input(BenchmarkId::new("arrangement", name), name, |b, _| {
            b.iter(|| {
                let mut octree = Octree::new(0.5, 10.0, 1e4);
                octree.build(black_box(bodies.iter().copied()));

                let mut total_force = Vector::ZERO;
                for body in bodies.iter().take(100) {
                    // Sample 100 bodies
                    let force = octree.calculate_force(body, None, 10.0);
                    total_force += force;
                }

                black_box((octree, total_force));
            });
        });
    }

    group.finish();
}

fn bench_rebuild_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("rebuild_patterns");

    let body_count = 1_000;
    let base_bodies = generate_test_bodies_spherical(body_count, 42, 300.0);

    // Test different movement patterns
    let patterns = [
        ("static", 0.0),
        ("small_drift", 1.0),
        ("moderate_movement", 10.0),
        ("large_movement", 100.0),
    ];

    for (name, movement) in &patterns {
        group.throughput(Throughput::Elements(body_count as u64 * 5)); // 5 timesteps
        group.bench_with_input(
            BenchmarkId::new("pattern", name),
            movement,
            |b, &movement_scale| {
                let mut octree = Octree::new(0.5, 10.0, 1e4);
                let mut rng = ChaCha8Rng::seed_from_u64(123);

                b.iter(|| {
                    let mut bodies = base_bodies.clone();

                    // Simulate 5 timesteps
                    for _step in 0..5 {
                        // Move bodies
                        if movement_scale > 0.0 {
                            for body in &mut bodies {
                                let drift = Vector::new(
                                    rng.random_range(-movement_scale..movement_scale),
                                    rng.random_range(-movement_scale..movement_scale),
                                    rng.random_range(-movement_scale..movement_scale),
                                );
                                body.position += drift;
                            }
                        }

                        octree.build(black_box(bodies.iter().copied()));
                    }

                    black_box(&octree);
                });
            },
        );
    }

    group.finish();
}

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

criterion_group!(
    realworld,
    bench_realworld_60fps_target,
    bench_extreme_scenarios
);

criterion_group!(configurations, bench_configuration_profiles);

criterion_group!(
    characteristics,
    bench_spatial_arrangements,
    bench_rebuild_patterns,
    bench_stats_overhead
);

criterion_main!(
    construction,
    physics,
    realworld,
    configurations,
    characteristics
);
