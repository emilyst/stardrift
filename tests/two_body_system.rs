//! System-level two-body test: measures the convergence order of the *actual*
//! production integration scheme, not the integrators in isolation.
//!
//! `integrate_motions` advances all bodies stage-synchronized: each stage of
//! the active integrator gathers every body's stage query, rebuilds the
//! octree from that consistent snapshot, and evaluates all accelerations
//! against it before any body advances. These tests pin the properties that
//! the synchronization delivers, at theta = 0 (exact pairwise forces):
//!
//! - every integrator recovers its nominal convergence order through the real
//!   pipeline (1, 1, 2, 2, 2, 4, 4);
//! - total linear momentum is conserved to roundoff by ALL methods,
//!   multi-stage included — each stage's pairwise forces are evaluated from
//!   one consistent configuration, so action equals reaction exactly;
//! - angular momentum is conserved to roundoff by the splitting (drift-kick)
//!   methods, and drifts at its predicted secular magnitude for the others;
//! - the palindromic methods (velocity Verlet, PEFRL) are exactly
//!   time-reversible through the full pipeline, and the non-symmetric methods
//!   measurably are not;
//! - the single-evaluation methods (explicit and symplectic Euler) produce
//!   bitwise-identical trajectories to the pre-restructure pipeline, anchoring
//!   the driver rewrite.
//!
//! An order or conservation regression here means the driver no longer keeps
//! stages globally synchronized (or the octree stopped being exact at
//! theta = 0). See docs/integration.md for the design.
//!
//! To re-derive measured values:
//! `cargo test --test two_body_system characterize -- --ignored --nocapture`

use bevy::prelude::*;
use stardrift::physics::components::{Mass, Position, Velocity};
use stardrift::physics::integrators::Integrator;
use stardrift::physics::integrators::registry::IntegratorRegistry;
use stardrift::physics::math::{Scalar, Vector};
use stardrift::physics::octree::Octree;
use stardrift::physics::resources::{CurrentIntegrator, PhysicsTime};
use stardrift::plugins::simulation::physics::integrate_motions;
use stardrift::resources::{GravitationalConstant, GravitationalOctree};
use std::f64::consts::PI;

// =============================================================================
// Scenario: unequal-mass two-body problem, eccentric relative orbit
// =============================================================================

const G: Scalar = 1.0;
/// Unequal masses (3:1) so no equal-mass symmetry can cancel the
/// frozen-field error terms.
const MASS_1: Scalar = 3.0;
const MASS_2: Scalar = 1.0;
const TOTAL_MASS: Scalar = MASS_1 + MASS_2;
/// The relative coordinate follows a Kepler orbit with mu = G * (m1 + m2).
const MU: Scalar = G * TOTAL_MASS;
const REL_A: Scalar = 1.0;
const REL_E: Scalar = 0.5;
/// Period of the relative orbit: 2 pi sqrt(a^3 / mu) = pi for these values.
const REL_T: Scalar = PI;

/// Relative-coordinate perihelion start: r = a(1-e) along x,
/// v = sqrt(mu (1+e) / (a (1-e))) along y.
fn relative_perihelion_state() -> (Vector, Vector) {
    let r_peri = REL_A * (1.0 - REL_E);
    let v_peri = (MU * (1.0 + REL_E) / r_peri).sqrt();
    (Vector::new(r_peri, 0.0, 0.0), Vector::new(0.0, v_peri, 0.0))
}

/// Exact relative state at time `t` after perihelion, via Newton iteration on
/// Kepler's equation M = E - e sin E. (Same solver as the integrator
/// correctness suite; duplicated to keep both test binaries self-contained.)
fn kepler_exact(
    semi_major: Scalar,
    eccentricity: Scalar,
    gm: Scalar,
    t: Scalar,
) -> (Vector, Vector) {
    let mean_motion = (gm / semi_major.powi(3)).sqrt();
    let mean_anomaly = mean_motion * t;

    let mut ecc_anomaly = mean_anomaly;
    for _ in 0..64 {
        let f = ecc_anomaly - eccentricity * ecc_anomaly.sin() - mean_anomaly;
        let f_prime = 1.0 - eccentricity * ecc_anomaly.cos();
        let delta = f / f_prime;
        ecc_anomaly -= delta;
        if delta.abs() < 1e-15 {
            break;
        }
    }

    let (sin_e, cos_e) = ecc_anomaly.sin_cos();
    let semi_minor = semi_major * (1.0 - eccentricity * eccentricity).sqrt();
    let position = Vector::new(semi_major * (cos_e - eccentricity), semi_minor * sin_e, 0.0);
    let ecc_anomaly_rate = mean_motion / (1.0 - eccentricity * cos_e);
    let velocity = Vector::new(
        -semi_major * sin_e * ecc_anomaly_rate,
        semi_minor * cos_e * ecc_anomaly_rate,
        0.0,
    );
    (position, velocity)
}

fn phase_space_error(dx: Vector, dv: Vector) -> Scalar {
    (dx.length_squared() + dv.length_squared()).sqrt()
}

/// Least-squares fit of log(err) = log(C) + p * log(h); returns (p, R^2).
fn fit_order(step_sizes: &[Scalar], errors: &[Scalar]) -> (Scalar, Scalar) {
    let n = step_sizes.len() as Scalar;
    let xs: Vec<Scalar> = step_sizes.iter().map(|h| h.ln()).collect();
    let ys: Vec<Scalar> = errors.iter().map(|e| e.ln()).collect();
    let x_mean = xs.iter().sum::<Scalar>() / n;
    let y_mean = ys.iter().sum::<Scalar>() / n;
    let ss_xy: Scalar = xs
        .iter()
        .zip(&ys)
        .map(|(x, y)| (x - x_mean) * (y - y_mean))
        .sum();
    let ss_xx: Scalar = xs.iter().map(|x| (x - x_mean).powi(2)).sum();
    let ss_yy: Scalar = ys.iter().map(|y| (y - y_mean).powi(2)).sum();
    let slope = ss_xy / ss_xx;
    let r_squared = (ss_xy * ss_xy) / (ss_xx * ss_yy);
    (slope, r_squared)
}

// =============================================================================
// Simulation driver: the real production systems on a minimal World
// =============================================================================

struct TwoBodySim {
    world: World,
    schedule: Schedule,
    body_1: Entity,
    body_2: Entity,
}

impl TwoBodySim {
    fn new(integrator_name: &str, dt: Scalar) -> Self {
        // theta = 0: exact pairwise forces, so any momentum or order defect
        // measured here comes from the integration scheme, not the octree
        // approximation.
        Self::with_theta(integrator_name, dt, 0.0)
    }

    fn with_theta(integrator_name: &str, dt: Scalar, theta: Scalar) -> Self {
        // At N = 2 the driver's acceleration pass runs sequentially, so no
        // compute task pool is needed.
        let registry = IntegratorRegistry::new().with_standard_integrators();
        let integrator: Box<dyn Integrator + Send + Sync> = registry
            .create(integrator_name)
            .unwrap_or_else(|e| panic!("{e}"));

        let mut world = World::new();
        // min_distance far below perihelion separation (0.5).
        world.insert_resource(GravitationalOctree::new(Octree::new(theta, 1e-6, 1e12)));
        world.insert_resource(CurrentIntegrator(integrator));
        world.insert_resource(PhysicsTime { dt, paused: false });
        world.insert_resource(GravitationalConstant(G));

        let (rel_pos, rel_vel) = relative_perihelion_state();
        let body_1 = world
            .spawn((
                Position::new(-rel_pos * (MASS_2 / TOTAL_MASS)),
                Velocity::new(-rel_vel * (MASS_2 / TOTAL_MASS)),
                Mass::new(MASS_1),
            ))
            .id();
        let body_2 = world
            .spawn((
                Position::new(rel_pos * (MASS_1 / TOTAL_MASS)),
                Velocity::new(rel_vel * (MASS_1 / TOTAL_MASS)),
                Mass::new(MASS_2),
            ))
            .id();

        // The production driver builds the octree internally, once per
        // integrator stage.
        let mut schedule = Schedule::default();
        schedule.add_systems(integrate_motions);

        Self {
            world,
            schedule,
            body_1,
            body_2,
        }
    }

    fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.schedule.run(&mut self.world);
        }
    }

    fn body_state(&self, entity: Entity) -> (Vector, Vector) {
        let position = self.world.get::<Position>(entity).unwrap().value();
        let velocity = self.world.get::<Velocity>(entity).unwrap().value();
        (position, velocity)
    }

    fn relative_state(&self) -> (Vector, Vector) {
        let (x1, v1) = self.body_state(self.body_1);
        let (x2, v2) = self.body_state(self.body_2);
        (x2 - x1, v2 - v1)
    }

    fn total_momentum(&self) -> Vector {
        let (_, v1) = self.body_state(self.body_1);
        let (_, v2) = self.body_state(self.body_2);
        v1 * MASS_1 + v2 * MASS_2
    }

    fn total_angular_momentum(&self) -> Vector {
        let (x1, v1) = self.body_state(self.body_1);
        let (x2, v2) = self.body_state(self.body_2);
        x1.cross(v1) * MASS_1 + x2.cross(v2) * MASS_2
    }

    fn total_energy(&self) -> Scalar {
        let (x1, v1) = self.body_state(self.body_1);
        let (x2, v2) = self.body_state(self.body_2);
        let kinetic = 0.5 * MASS_1 * v1.length_squared() + 0.5 * MASS_2 * v2.length_squared();
        kinetic - G * MASS_1 * MASS_2 / (x2 - x1).length()
    }

    fn negate_velocities(&mut self) {
        for entity in [self.body_1, self.body_2] {
            let mut velocity = self.world.get_mut::<Velocity>(entity).unwrap();
            let negated = -velocity.value();
            *velocity.value_mut() = negated;
        }
    }
}

// =============================================================================
// Measurements
// =============================================================================

/// Steps per relative-orbit period for the order study. The endpoint is 5/8 of
/// a period — deliberately not a whole period, where phase errors can cancel.
const ORDER_STEPS_PER_PERIOD: &[usize] = &[200, 400, 800, 1600];

fn measure_system_order(name: &str) -> (Scalar, Scalar, Vec<Scalar>) {
    let t_end = REL_T * 5.0 / 8.0;
    let (exact_pos, exact_vel) = kepler_exact(REL_A, REL_E, MU, t_end);

    let (step_sizes, errors): (Vec<Scalar>, Vec<Scalar>) = ORDER_STEPS_PER_PERIOD
        .iter()
        .map(|&n| {
            let dt = REL_T / n as Scalar;
            let mut sim = TwoBodySim::new(name, dt);
            sim.step_n(n * 5 / 8);
            let (rel_pos, rel_vel) = sim.relative_state();
            let error = phase_space_error(rel_pos - exact_pos, rel_vel - exact_vel);
            (dt, error)
        })
        .unzip();

    let (order, r_squared) = fit_order(&step_sizes, &errors);
    (order, r_squared, errors)
}

const MOMENTUM_STEPS_PER_PERIOD: usize = 1000;

/// Running max of |total momentum| over 5/8 period, relative to the
/// characteristic momentum scale of the orbit. Initial total momentum is
/// exactly zero by construction.
fn measure_momentum_drift(name: &str) -> Scalar {
    let dt = REL_T / MOMENTUM_STEPS_PER_PERIOD as Scalar;
    let mut sim = TwoBodySim::new(name, dt);
    let momentum_scale = MASS_2 * relative_perihelion_state().1.length();
    let mut max_drift = 0.0_f64;
    for _ in 0..MOMENTUM_STEPS_PER_PERIOD * 5 / 8 {
        sim.step_n(1);
        max_drift = max_drift.max(sim.total_momentum().length() / momentum_scale);
    }
    max_drift
}

// =============================================================================
// Tests
// =============================================================================

/// Measured convergence order of the full production pipeline, per integrator
/// (characterized 2026-08-25, post stage-synchronization). Every method
/// recovers its nominal order at theta = 0. The two first-order methods
/// measure slightly below 1 (pre-asymptotic at this dt range, trending toward
/// 1 at finer steps); their bands account for that.
const SYSTEM_ORDER_BANDS: &[(&str, (Scalar, Scalar))] = &[
    ("explicit_euler", (0.6, 1.1)),                     // measured 0.82
    ("symplectic_euler", (0.65, 1.15)),                 // measured 0.89
    ("velocity_verlet", (1.75, 2.25)),                  // measured 2.00
    ("heun", (1.75, 2.3)),                              // measured 2.03
    ("runge_kutta_second_order_midpoint", (1.65, 2.2)), // measured 1.90
    ("runge_kutta_fourth_order", (3.7, 4.45)),          // measured 4.10
    ("pefrl", (3.75, 4.25)),                            // measured 4.00
];

#[test]
fn system_convergence_order_matches_nominal_order() {
    for (name, (lo, hi)) in SYSTEM_ORDER_BANDS {
        let (order, r_squared, errors) = measure_system_order(name);
        assert!(
            order > *lo && order < *hi,
            "{name}: system-level convergence order {order:.3} outside ({lo}, {hi}) \
             (R^2 = {r_squared:.5}, errors: {errors:?}).\n\
             An order below nominal means integrate_motions no longer keeps \
             stages globally synchronized — re-characterize this suite."
        );
    }
}

#[test]
fn all_methods_conserve_momentum_to_roundoff() {
    // Every stage's pairwise forces are evaluated from one consistent
    // configuration, so action equals reaction exactly at theta = 0 and total
    // momentum survives to roundoff — for multi-stage methods too.
    let registry = IntegratorRegistry::new().with_standard_integrators();
    for name in registry.list_available() {
        let drift = measure_momentum_drift(&name);
        assert!(
            drift < 1e-12,
            "{name}: momentum drift {drift:.3e} above roundoff.\n\
             Secular drift means a stage evaluated forces from an inconsistent \
             (stale-partner) configuration — re-characterize this suite."
        );
    }
}

/// Angular-momentum drift over 5/8 period at dt = T/1000, theta = 0
/// (characterized 2026-08-25). Only the splitting (drift-kick) methods
/// conserve L exactly: every drift and every kick against a central pairwise
/// force preserves x cross v regardless of coefficients. The Runge-Kutta
/// family accumulates a secular O(dt^(p+1))-per-step error, and explicit
/// Euler grows L by the closed form L' = L(1 + dt^2 mu / r^3) each step —
/// their two-sided bands pin those magnitudes.
const ANGULAR_MOMENTUM_BANDS: &[(&str, Option<(Scalar, Scalar)>)] = &[
    ("explicit_euler", Some((1e-2, 1e-1))), // measured 3.07e-2
    ("symplectic_euler", None),
    ("velocity_verlet", None),
    ("heun", Some((3e-7, 6e-6))), // measured 1.27e-6
    ("runge_kutta_second_order_midpoint", Some((2e-5, 4e-4))), // measured 7.53e-5
    ("runge_kutta_fourth_order", Some((2e-12, 3e-10))), // measured 2.32e-11
    ("pefrl", None),
];

#[test]
fn splitting_methods_conserve_angular_momentum_to_roundoff() {
    let dt = REL_T / MOMENTUM_STEPS_PER_PERIOD as Scalar;
    for (name, band) in ANGULAR_MOMENTUM_BANDS {
        let mut sim = TwoBodySim::new(name, dt);
        let initial = sim.total_angular_momentum();
        let scale = initial.length();
        sim.step_n(MOMENTUM_STEPS_PER_PERIOD * 5 / 8);
        let drift = sim.total_angular_momentum() - initial;
        match band {
            None => {
                // Componentwise, so an out-of-plane leak (a z-symmetry bug)
                // is visible even though the orbit is planar.
                for (axis, component) in [("x", drift.x), ("y", drift.y), ("z", drift.z)] {
                    assert!(
                        component.abs() / scale < 1e-12,
                        "{name}: angular momentum {axis}-component drift {component:.3e} \
                         (relative to |L| = {scale:.3e}) above roundoff — the drift-kick \
                         structure no longer conserves L exactly"
                    );
                }
            }
            Some((lo, hi)) => {
                let relative = drift.length() / scale;
                assert!(
                    relative > *lo && relative < *hi,
                    "{name}: angular momentum drift {relative:.3e} outside measured \
                     band ({lo:.0e}, {hi:.0e}) — re-characterize"
                );
            }
        }
    }
}

/// Forward-backward residual through the full pipeline at theta = 0.5:
/// integrate forward, negate velocities, integrate back, compare to the
/// initial state. The Barnes-Hut field depends only on positions and the tree
/// build is deterministic, so the palindromic methods (velocity Verlet,
/// PEFRL) retrace their steps to roundoff at ANY theta — this is the one
/// structural property that survives the octree approximation. Non-symmetric
/// methods leave an O(dt^p+1)-per-step residual, orders of magnitude larger.
/// The residual gap is dt-dependent — RK4's per-step asymmetry is O(dt^5), so
/// a fine dt would shrink it below any fixed floor — hence the deliberately
/// coarse dt = T/200 here, where the measured gap is eight orders of
/// magnitude (characterized 2026-08-25: velocity_verlet 8.7e-16, pefrl
/// 1.9e-15; rk4 4.6e-7, heun/rk2 9.2e-4, symplectic_euler 1.3e-1,
/// explicit_euler 1.0e0).
const REVERSIBILITY_STEPS: usize = 100;
const REVERSIBILITY_BANDS: &[(&str, bool)] = &[
    ("explicit_euler", false),
    ("symplectic_euler", false),
    ("velocity_verlet", true),
    ("heun", false),
    ("runge_kutta_second_order_midpoint", false),
    ("runge_kutta_fourth_order", false),
    ("pefrl", true),
];

#[test]
fn palindromic_methods_are_exactly_reversible_at_nonzero_theta() {
    let dt = REL_T / 200.0;
    for (name, reversible) in REVERSIBILITY_BANDS {
        let mut sim = TwoBodySim::with_theta(name, dt, 0.5);
        let (x1_0, v1_0) = sim.body_state(sim.body_1);
        let (x2_0, v2_0) = sim.body_state(sim.body_2);
        sim.step_n(REVERSIBILITY_STEPS);
        sim.negate_velocities();
        sim.step_n(REVERSIBILITY_STEPS);
        let (x1, v1) = sim.body_state(sim.body_1);
        let (x2, v2) = sim.body_state(sim.body_2);
        // Position and velocity residuals both matter: a velocity asymmetry
        // alone must fail the reversible band.
        let residual =
            phase_space_error(x1 - x1_0, v1 + v1_0).max(phase_space_error(x2 - x2_0, v2 + v2_0));
        if *reversible {
            assert!(
                residual < 1e-11,
                "{name}: forward-backward residual {residual:.3e} above roundoff — \
                 palindromic structure broken (or the tree build became \
                 nondeterministic)"
            );
        } else {
            assert!(
                residual > 1e-7,
                "{name}: forward-backward residual {residual:.3e} suspiciously small \
                 for a non-symmetric method — it may have accidentally become \
                 time-symmetric (compare velocity_verlet); re-characterize"
            );
        }
    }
}

/// Bitwise anchors for the single-evaluation methods, captured from the
/// pre-restructure pipeline (commit cd30c30: one octree build per step via
/// rebuild_octree, then independent per-body stepping). For these methods the
/// staged driver performs the identical arithmetic in the identical order, so
/// the trajectories must match bit for bit. A mismatch means the driver
/// rewrite perturbed the single-evaluation path. If an intentional change
/// breaks these (e.g. reordering force accumulation), re-capture via the
/// commented recipe below.
///
/// Recipe: run 100 steps at dt = REL_T/1000 from the standard scenario and
/// print `to_bits()` of both bodies' position and velocity components.
const BITWISE_ANCHORS: &[(&str, [[u64; 3]; 4])] = &[
    (
        "explicit_euler",
        [
            // body 1 position, body 1 velocity, body 2 position, body 2 velocity
            [0x3f6baed43ce6dea0, 0xbfc8a05d8a4bf08d, 0x0000000000000000],
            [0x3fe287fd8ae48ab1, 0xbfd27fb5116e20a9, 0x0000000000000000],
            [0xbf84c31f2dad26a0, 0x3fe2784627b8f46f, 0x0000000000000000],
            [0xbffbcbfc5056d008, 0x3febbf8f9a253112, 0x0000000000000000],
        ],
    ),
    (
        "symplectic_euler",
        [
            [0x3f7a295f3cba8e31, 0xbfc822333313f526, 0x0000000000000000],
            [0x3fe2aaefafba7007, 0xbfd16752671aaa11, 0x0000000000000000],
            [0xbf939f076d8beacc, 0x3fe219a6664ef7dc, 0x0000000000000000],
            [0xbffc00678797a80c, 0x3fea1afb9aa7ff25, 0x0000000000000000],
        ],
    ),
];

#[test]
fn single_eval_methods_match_pre_restructure_trajectories_bitwise() {
    for (name, anchor) in BITWISE_ANCHORS {
        let mut sim = TwoBodySim::new(name, REL_T / 1000.0);
        sim.step_n(100);
        let (x1, v1) = sim.body_state(sim.body_1);
        let (x2, v2) = sim.body_state(sim.body_2);
        let actual = [x1, v1, x2, v2];
        let labels = ["x1", "v1", "x2", "v2"];
        for ((vector, expected_bits), label) in actual.iter().zip(anchor).zip(labels) {
            let actual_bits = [vector.x.to_bits(), vector.y.to_bits(), vector.z.to_bits()];
            assert!(
                actual_bits == *expected_bits,
                "{name}: {label} = {vector:?} (bits {actual_bits:#018x?}) differs from \
                 the pre-restructure anchor {expected_bits:#018x?}.\n\
                 The driver rewrite perturbed the single-evaluation path; if the \
                 change is intentional, re-capture the anchors."
            );
        }
    }
}

// =============================================================================
// Characterization helper
// =============================================================================

/// Energy-drift character across theta, for the symplectic methods. Not an
/// assertion: at theta > 0 the Barnes-Hut field is not a gradient field, so
/// the drift is a property of the approximation, not the integrator. This
/// prints the measured max |dE/E| over 20 relative orbits per theta so the
/// docs' claims can be checked from measurement.
/// `cargo test --test two_body_system characterize_theta -- --ignored --nocapture`
#[test]
#[ignore = "measurement helper, not an assertion"]
fn characterize_theta_sweep() {
    let dt = REL_T / MOMENTUM_STEPS_PER_PERIOD as Scalar;
    let orbits = 20;
    for name in ["symplectic_euler", "velocity_verlet", "pefrl"] {
        for theta in [0.0, 0.25, 0.5, 1.0] {
            let mut sim = TwoBodySim::with_theta(name, dt, theta);
            let initial_energy = sim.total_energy();
            let mut max_drift = 0.0_f64;
            for _ in 0..MOMENTUM_STEPS_PER_PERIOD * orbits {
                sim.step_n(1);
                let drift = ((sim.total_energy() - initial_energy) / initial_energy).abs();
                max_drift = max_drift.max(drift);
            }
            println!("{name} theta={theta}: max |dE/E| over {orbits} orbits = {max_drift:.3e}");
        }
    }
}

/// Prints measured system-level orders and momentum drifts.
/// `cargo test --test two_body_system characterize -- --ignored --nocapture`
#[test]
#[ignore = "measurement helper, not an assertion"]
fn characterize() {
    let registry = IntegratorRegistry::new().with_standard_integrators();
    for name in registry.list_available() {
        let (order, r_squared, errors) = measure_system_order(&name);
        let drift = measure_momentum_drift(&name);
        let formatted: Vec<String> = errors.iter().map(|e| format!("{e:.3e}")).collect();
        println!(
            "{name}: order {order:.4} (r2 {r_squared:.6}) errors [{}] momentum drift {drift:.3e}",
            formatted.join(", ")
        );
    }
}
