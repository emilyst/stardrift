//! Diagnostic: per-system, per-step timing of the physics pipeline at high
//! body counts, with body-count tracking to correlate cost spikes with
//! collision merges. Not a benchmark — a one-off investigation tool.
//!
//! Run: cargo run --release --example step_timing [N]

use bevy::prelude::*;
use bevy::tasks::{ComputeTaskPool, TaskPoolBuilder};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::time::Instant;

use stardrift::config::SimulationConfig;
use stardrift::physics::components::{PhysicsBody, PhysicsBodyBundle};
use stardrift::physics::integrators::Integrator;
use stardrift::physics::integrators::registry::IntegratorRegistry;
use stardrift::physics::math::{Scalar, Vector, mass_for_radius};
use stardrift::physics::octree::Octree;
use stardrift::physics::resources::{CurrentIntegrator, PhysicsTime};
use stardrift::plugins::simulation::collisions::detect_and_merge_collisions;
use stardrift::plugins::simulation::physics::{counteract_barycentric_drift, integrate_motions};
use stardrift::resources::{Barycenter, GravitationalConstant, GravitationalOctree};

fn build_world(count: usize, collisions_enabled: bool) -> World {
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

    // Contact-free scene matching the benchmark: constant surface density
    // (radius ∝ sqrt(N)), uniform sphere sampling, minimum spawn separation.
    const MIN_SEPARATION: Scalar = 60.0;
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let shell_radius = 750.0 * (count as Scalar / 25.0).sqrt();
    let mut accepted: Vec<Vector> = Vec::with_capacity(count);
    while accepted.len() < count {
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
    world
}

fn run(count: usize, collisions_enabled: bool) {
    let mut world = build_world(count, collisions_enabled);

    let mut s_integrate = Schedule::default();
    s_integrate.add_systems(integrate_motions);
    let mut s_collide = Schedule::default();
    s_collide.add_systems(detect_and_merge_collisions);
    let mut s_drift = Schedule::default();
    s_drift.add_systems(counteract_barycentric_drift);

    let mut count_query = world.query_filtered::<(), With<PhysicsBody>>();
    let mut prev_bodies = count_query.iter(&world).count();
    let (mut t_int_total, mut t_col_total) = (0.0f64, 0.0f64);
    let mut merge_steps = 0usize;
    let mut merged_away = 0usize;

    for _step in 1..=60 {
        let t0 = Instant::now();
        s_integrate.run(&mut world);
        let t1 = Instant::now();
        s_collide.run(&mut world);
        let t2 = Instant::now();
        s_drift.run(&mut world);

        t_int_total += t1.duration_since(t0).as_secs_f64() * 1e6;
        t_col_total += t2.duration_since(t1).as_secs_f64() * 1e6;

        let bodies = count_query.iter(&world).count();
        if bodies != prev_bodies {
            merge_steps += 1;
            merged_away += prev_bodies - bodies;
        }
        prev_bodies = bodies;
    }
    println!(
        "N={count} collisions={}: integrate {:.1}us/step  collide {:.1}us/step  merge-steps {merge_steps}  merged-away {merged_away}",
        if collisions_enabled { "ON " } else { "OFF" },
        t_int_total / 60.0,
        t_col_total / 60.0,
    );
}

fn main() {
    let count: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(3000);
    run(count, false);
    run(count, true);
    // Second round to expose warmup effects.
    run(count, false);
    run(count, true);
}
