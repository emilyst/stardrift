//! Full physics-step benchmark: the production FixedUpdate pipeline
//! (`integrate_motions` → `detect_and_merge_collisions` →
//! `counteract_barycentric_drift`) on a minimal World.
//!
//! The collisions-enabled vs collisions-disabled delta is the steady-state
//! cost of the collision pass (gather, sweep-and-prune, narrow phase) when
//! nothing is touching — the case every frame pays. Merge steps additionally
//! pay a despawn, but at most N−1 merges can ever happen in a run, so the
//! per-frame number is the one that matters.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::hint::black_box;

use bevy::prelude::*;
use bevy::tasks::{ComputeTaskPool, TaskPoolBuilder};
use stardrift::config::SimulationConfig;
use stardrift::physics::components::PhysicsBodyBundle;
use stardrift::physics::integrators::Integrator;
use stardrift::physics::integrators::registry::IntegratorRegistry;
use stardrift::physics::math::{Scalar, Vector, mass_for_radius};
use stardrift::physics::octree::Octree;
use stardrift::physics::resources::{CurrentIntegrator, PhysicsTime};
use stardrift::plugins::simulation::collisions::detect_and_merge_collisions;
use stardrift::plugins::simulation::physics::{counteract_barycentric_drift, integrate_motions};
use stardrift::resources::{Barycenter, GravitationalConstant, GravitationalOctree};

fn build_sim(count: usize, collisions_enabled: bool) -> (World, Schedule) {
    // The parallel acceleration path dispatches on the compute task pool at
    // N >= 128; initialize it once so those sizes bench the production path.
    ComputeTaskPool::get_or_init(|| TaskPoolBuilder::new().build());

    let registry = IntegratorRegistry::new().with_standard_integrators();
    let integrator: Box<dyn Integrator + Send + Sync> = registry.create("velocity_verlet").unwrap();

    let mut config = SimulationConfig::default();
    config.physics.collisions.enabled = collisions_enabled;

    let mut world = World::new();
    world.insert_resource(GravitationalOctree::new(
        Octree::new(0.5, 1.0, 1e6).with_leaf_threshold(1),
    ));
    world.insert_resource(CurrentIntegrator(integrator));
    world.insert_resource(PhysicsTime {
        dt: 1.0 / 60.0,
        paused: false,
    });
    world.insert_resource(GravitationalConstant(100.0));
    world.insert_resource(Barycenter::default());
    world.insert_resource(config);

    // Spherical shell at constant surface density (radius ∝ sqrt(N)) with a
    // minimum spawn separation, so the scene stays contact-free for the
    // whole measured window at every N and the on/off delta isolates the
    // collision pass itself. This is deliberate benchmark hygiene, learned
    // the hard way: volume-matched shells (cbrt), normalized-cube direction
    // sampling (density concentrates toward the cube diagonals), and the
    // shearing tangential velocity field each caused merges during the
    // window — and every merge step forfeits the FSAL cache and pays a
    // second octree-build-and-force pass, which dominated the delta and
    // masqueraded as superlinear collision-pass cost. Merge cost is real
    // physics, but it belongs to the merge rate, not to this steady-state
    // number. The separation must beat not just contact distance (8) plus
    // shear drift (~10) but mutual two-body free-fall at G = 100: a pair
    // 25 apart falls to contact within the measured second, so the
    // separation is sized for a free-fall time comfortably beyond it.
    const MIN_SEPARATION: Scalar = 60.0;
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let shell_radius = 750.0 * (count as Scalar / 25.0).sqrt();
    let mut accepted: Vec<Vector> = Vec::with_capacity(count);
    while accepted.len() < count {
        // Uniform on the sphere: rejection-sample the unit ball, normalize.
        let direction = loop {
            let v = Vector::new(
                rng.random_range(-1.0..=1.0),
                rng.random_range(-1.0..=1.0),
                rng.random_range(-1.0..=1.0),
            );
            let len = v.length();
            if len > 1e-3 && len <= 1.0 {
                break v / len;
            }
        };
        let position = direction * shell_radius;
        if accepted
            .iter()
            .any(|p| p.distance_squared(position) < MIN_SEPARATION * MIN_SEPARATION)
        {
            continue;
        }
        accepted.push(position);
        let radius: Scalar = rng.random_range(2.0..=4.0);
        let tangent = direction.cross(Vector::Z).normalize_or_zero();
        let velocity = if tangent.length_squared() > 0.5 {
            tangent * 5.0
        } else {
            Vector::new(5.0, 0.0, 0.0)
        };
        world.spawn(PhysicsBodyBundle::new(
            position,
            mass_for_radius(radius),
            radius as f32,
            velocity,
        ));
    }

    let mut schedule = Schedule::default();
    schedule.add_systems(
        (
            integrate_motions,
            detect_and_merge_collisions,
            counteract_barycentric_drift,
        )
            .chain(),
    );

    (world, schedule)
}

/// Steps per measured iteration. Fresh world each iteration: a persistent
/// world would evolve across criterion's hundreds of thousands of samples
/// (the scene collapses, and with collisions on the population merges down),
/// so on/off would measure different scenes. One second of simulation from
/// the spread-out initial state keeps the scene contact-free and comparable.
const STEPS_PER_ITER: usize = 60;

fn bench_full_step(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_step");
    group.sample_size(20);
    let body_counts = [50, 100, 300, 1000, 3000];

    for &count in &body_counts {
        group.throughput(Throughput::Elements((count * STEPS_PER_ITER) as u64));
        for (label, enabled) in [("collisions_on", true), ("collisions_off", false)] {
            group.bench_with_input(BenchmarkId::new(label, count), &count, |bencher, &count| {
                bencher.iter_batched_ref(
                    || build_sim(count, enabled),
                    |(world, schedule)| {
                        for _ in 0..STEPS_PER_ITER {
                            schedule.run(world);
                        }
                        black_box(world);
                    },
                    criterion::BatchSize::LargeInput,
                );
            });
        }
    }
    group.finish();
}

/// The collision pass alone (no integration; nothing moves), isolating its
/// cost from the rest of the pipeline at the same body counts.
fn bench_collision_pass_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("collision_pass_only");
    let body_counts = [300, 1000, 3000];

    for &count in &body_counts {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("bodies", count),
            &count,
            |bencher, &count| {
                let (mut world, _) = build_sim(count, true);
                let mut schedule = Schedule::default();
                schedule.add_systems(detect_and_merge_collisions);
                // First run resolves any spawn-overlap merges so the measured
                // steady state is contact-free.
                schedule.run(&mut world);
                bencher.iter(|| {
                    schedule.run(&mut world);
                    black_box(&world);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(full_step, bench_full_step);
criterion_group!(collision_pass, bench_collision_pass_only);
criterion_main!(full_step, collision_pass);
