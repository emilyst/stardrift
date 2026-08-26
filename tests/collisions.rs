//! System-level collision tests: the real `integrate_motions` →
//! `detect_and_merge_collisions` pipeline on a minimal World.
//!
//! These pin the properties the merge design promises:
//! - mass, linear momentum, and the mass-weighted position sum are conserved
//!   through a merge to roundoff (so barycentric drift correction sees
//!   nothing);
//! - detection is swept, not discrete: a body crossing another entirely
//!   within one step still merges (the test that guards against a future
//!   "simplification" to end-of-step overlap testing);
//! - simultaneous multi-body contacts resolve as one connected component,
//!   independent of spawn order up to rounding;
//! - the merged radius follows the shared density relation exactly;
//! - a paused or disabled simulation never merges.

use bevy::prelude::*;
use stardrift::config::SimulationConfig;
use stardrift::physics::components::{Mass, PhysicsBody, PhysicsBodyBundle, Position, Velocity};
use stardrift::physics::integrators::Integrator;
use stardrift::physics::integrators::registry::IntegratorRegistry;
use stardrift::physics::math::{Scalar, Vector, mass_for_radius, radius_for_mass};
use stardrift::physics::octree::Octree;
use stardrift::physics::resources::{CurrentIntegrator, PhysicsTime};
use stardrift::plugins::simulation::collisions::detect_and_merge_collisions;
use stardrift::plugins::simulation::physics::{counteract_barycentric_drift, integrate_motions};
use stardrift::resources::{Barycenter, GravitationalConstant, GravitationalOctree};

const DT: Scalar = 1.0 / 60.0;

struct CollisionSim {
    world: World,
    schedule: Schedule,
}

impl CollisionSim {
    /// Pipeline with the production systems in production order. G = 0 for
    /// kinematic tests (bodies coast in straight lines); pass a nonzero G to
    /// exercise gravity through the merge.
    fn new(g: Scalar, config: SimulationConfig) -> Self {
        let registry = IntegratorRegistry::new().with_standard_integrators();
        let integrator: Box<dyn Integrator + Send + Sync> =
            registry.create("velocity_verlet").unwrap();

        let mut world = World::new();
        world.insert_resource(GravitationalOctree::new(Octree::new(0.0, 1e-6, 1e12)));
        world.insert_resource(CurrentIntegrator(integrator));
        world.insert_resource(PhysicsTime {
            dt: DT,
            paused: false,
        });
        world.insert_resource(GravitationalConstant(g));
        world.insert_resource(Barycenter::default());
        world.insert_resource(config);

        let mut schedule = Schedule::default();
        schedule.add_systems(
            (
                integrate_motions,
                detect_and_merge_collisions,
                counteract_barycentric_drift,
            )
                .chain(),
        );

        Self { world, schedule }
    }

    fn spawn_body(&mut self, position: Vector, radius: Scalar, velocity: Vector) -> Entity {
        self.world
            .spawn(PhysicsBodyBundle::new(
                position,
                mass_for_radius(radius),
                radius as f32,
                velocity,
            ))
            .id()
    }

    fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.schedule.run(&mut self.world);
        }
    }

    fn bodies(&mut self) -> Vec<(Vector, Vector, Scalar)> {
        self.world
            .query_filtered::<(&Position, &Velocity, &Mass), With<PhysicsBody>>()
            .iter(&self.world)
            .map(|(p, v, m)| (p.value(), v.value(), m.value()))
            .collect()
    }

    fn totals(&mut self) -> (Scalar, Vector, Vector) {
        self.bodies().iter().fold(
            (0.0, Vector::ZERO, Vector::ZERO),
            |(m_acc, p_acc, x_acc), (x, v, m)| (m_acc + m, p_acc + *v * *m, x_acc + *x * *m),
        )
    }
}

fn default_config() -> SimulationConfig {
    SimulationConfig::default()
}

#[test]
fn head_on_merge_conserves_mass_momentum_and_mass_weighted_position() {
    let mut sim = CollisionSim::new(0.0, default_config());
    // Zero net momentum: m1 v1 = -m2 v2, and m1 x1 + m2 x2 = 0, so the
    // mass-weighted position sum is a conserved zero through the merge.
    let r1 = 2.0;
    let r2 = 3.0;
    let (m1, m2) = (mass_for_radius(r1), mass_for_radius(r2));
    sim.spawn_body(
        Vector::new(-10.0, 0.0, 0.0),
        r1,
        Vector::new(30.0, 0.0, 0.0),
    );
    sim.spawn_body(
        Vector::new(10.0 * m1 / m2, 0.0, 0.0),
        r2,
        Vector::new(-30.0 * m1 / m2, 0.0, 0.0),
    );
    let (mass_before, momentum_before, weighted_before) = sim.totals();
    assert!(momentum_before.length() < 1e-12);
    assert!(weighted_before.length() < 1e-12);

    sim.step_n(120);

    let bodies = sim.bodies();
    assert_eq!(bodies.len(), 1, "bodies on a collision course must merge");
    let (mass_after, momentum_after, weighted_after) = sim.totals();
    assert_eq!(mass_after, mass_before);
    assert!(momentum_after.length() < 1e-12 * mass_before * 30.0);
    assert!(weighted_after.length() < 1e-12 * mass_before * 10.0);
}

#[test]
fn tunneling_body_merges_within_one_step() {
    let mut sim = CollisionSim::new(0.0, default_config());
    // 6000 units/s covers 100 units in one 60 Hz step — the moving body's
    // endpoints are both ~50 units from the stationary one, far outside the
    // 4-unit contact distance. Only a swept test can see this contact.
    sim.spawn_body(Vector::ZERO, 2.0, Vector::ZERO);
    sim.spawn_body(
        Vector::new(-50.0, 0.1, 0.0),
        2.0,
        Vector::new(6000.0, 0.0, 0.0),
    );

    sim.step_n(1);

    assert_eq!(
        sim.bodies().len(),
        1,
        "pass-through within a single step must still be detected"
    );
}

#[test]
fn simultaneous_triangle_contact_merges_as_one_component() {
    // Equilateral triangle, side 3, radii 2: all three pairs are inside
    // contact distance (4) simultaneously. One component, one survivor, and
    // the merged state is spawn-order independent up to rounding.
    let side = 3.0;
    let vertices = [
        Vector::new(0.0, 0.0, 0.0),
        Vector::new(side, 0.0, 0.0),
        Vector::new(side / 2.0, side * (3.0_f64).sqrt() / 2.0, 0.0),
    ];
    let radii = [2.0, 2.5, 3.0];
    let velocities = [
        Vector::new(1.0, -2.0, 0.5),
        Vector::new(-0.5, 1.0, 2.0),
        Vector::new(0.0, 0.5, -1.0),
    ];

    let mut merged_states = Vec::new();
    for order in [[0, 1, 2], [2, 0, 1], [1, 2, 0]] {
        let mut sim = CollisionSim::new(0.0, default_config());
        for &i in &order {
            sim.spawn_body(vertices[i], radii[i], velocities[i]);
        }
        let (mass_before, momentum_before, _) = sim.totals();

        sim.step_n(1);

        let bodies = sim.bodies();
        assert_eq!(bodies.len(), 1, "all three must merge in one step");
        let (x, v, m) = bodies[0];
        // The merge accumulates in entity order, totals() in query order, so
        // the sums agree to rounding, not bitwise.
        assert!((m - mass_before).abs() < 1e-12 * mass_before);
        assert!((v * m - momentum_before).length() < 1e-12 * momentum_before.length().max(1.0));
        merged_states.push((x, v, m));
    }

    // Spawn order changes the (entity-sorted) accumulation order, so results
    // agree only up to rounding — a few ulp, not bitwise.
    let (x0, v0, m0) = merged_states[0];
    for &(x, v, m) in &merged_states[1..] {
        assert!((m - m0).abs() < 1e-12 * m0);
        assert!((x - x0).length() < 1e-13);
        assert!((v - v0).length() < 1e-13);
    }
}

#[test]
fn merged_radius_follows_density_relation() {
    let mut sim = CollisionSim::new(0.0, default_config());
    let r1 = 2.0;
    let r2 = 3.5;
    sim.spawn_body(Vector::new(-3.0, 0.0, 0.0), r1, Vector::new(10.0, 0.0, 0.0));
    sim.spawn_body(Vector::new(3.0, 0.0, 0.0), r2, Vector::new(-10.0, 0.0, 0.0));

    sim.step_n(60);

    let mut query = sim
        .world
        .query_filtered::<&stardrift::physics::components::Radius, With<PhysicsBody>>();
    let radii: Vec<Scalar> = query.iter(&sim.world).map(|r| r.value()).collect();
    assert_eq!(radii.len(), 1);
    let expected = radius_for_mass(mass_for_radius(r1) + mass_for_radius(r2));
    assert_eq!(radii[0], expected);
    // Which equals volume conservation under one global density.
    assert!((radii[0] - (r1.powi(3) + r2.powi(3)).cbrt()).abs() < 1e-12);
}

#[test]
fn paused_simulation_never_merges() {
    let mut sim = CollisionSim::new(0.0, default_config());
    // Deeply overlapping bodies in a frozen scene must not merge.
    sim.spawn_body(Vector::ZERO, 3.0, Vector::ZERO);
    sim.spawn_body(Vector::new(1.0, 0.0, 0.0), 3.0, Vector::ZERO);
    sim.world.resource_mut::<PhysicsTime>().paused = true;

    sim.step_n(10);

    assert_eq!(sim.bodies().len(), 2);
}

#[test]
fn disabled_collisions_never_merge() {
    let mut config = default_config();
    config.physics.collisions.enabled = false;
    let mut sim = CollisionSim::new(0.0, config);
    sim.spawn_body(Vector::ZERO, 3.0, Vector::ZERO);
    sim.spawn_body(Vector::new(1.0, 0.0, 0.0), 3.0, Vector::ZERO);

    sim.step_n(10);

    assert_eq!(sim.bodies().len(), 2);
}

#[test]
fn merge_does_not_translate_scene_under_drift_correction() {
    // Regression guard for merge placement: the survivor sits at the
    // end-of-step mass-weighted centroid, so the mass-weighted position sum
    // is unchanged and an enabled drift corrector applies no translation on
    // the merge step. Placing the survivor anywhere else (larger body's
    // position, contact-time centroid) fails this by jolting the witness.
    let mut config = default_config();
    config.physics.barycentric_drift_correction = true;
    let mut sim = CollisionSim::new(100.0, config);

    // Colliding pair with zero net momentum, plus a distant slow witness.
    let r = 2.5;
    sim.spawn_body(Vector::new(-8.0, 0.0, 0.0), r, Vector::new(25.0, 0.0, 0.0));
    sim.spawn_body(Vector::new(8.0, 0.0, 0.0), r, Vector::new(-25.0, 0.0, 0.0));
    let witness = sim.spawn_body(Vector::new(0.0, 4000.0, 0.0), 2.0, Vector::ZERO);

    let witness_before = sim.world.get::<Position>(witness).unwrap().value();
    sim.step_n(120);
    let witness_after = sim.world.get::<Position>(witness).unwrap().value();

    assert_eq!(sim.bodies().len(), 2, "the pair must have merged");
    // The witness falls toward the pair under gravity; over 2 s at
    // a ~ G*M/d² ≈ 100·2·65/4000² ≈ 8e-4 that's ~2e-3 units of legitimate
    // motion. A drift-corrector jolt from a mis-placed merge would move it
    // by a macroscopic fraction of the pair separation instead.
    assert!(
        (witness_after - witness_before).length() < 0.01,
        "witness moved {} — drift corrector translated the scene on merge",
        (witness_after - witness_before).length()
    );
}
