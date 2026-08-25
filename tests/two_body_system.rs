//! System-level two-body test: measures the convergence order of the *actual*
//! production integration scheme, not the integrators in isolation.
//!
//! `integrate_motions` builds one octree per step (via `rebuild_octree`) and
//! then advances every body independently against that frozen snapshot. A
//! multi-stage integrator therefore evaluates its intermediate accelerations
//! with the *other* bodies still at their pre-step positions, which injects an
//! O(dt) error into the acceleration and caps the whole scheme at first-order
//! global accuracy — regardless of the integrator's nominal order. The
//! single-evaluation methods (explicit and symplectic Euler) query the field
//! once, at pre-step positions, which is exactly the synchronized form of
//! those schemes; their genuine order is 1, so they lose nothing.
//!
//! These tests pin the *current* behavior:
//!
//! - every integrator, PEFRL included, measures order ~1 through the real
//!   `rebuild_octree` + `integrate_motions` pipeline;
//! - total linear momentum is exactly conserved (to roundoff) by the
//!   single-evaluation methods but drifts secularly for multi-stage methods,
//!   because the stale-partner evaluations break Newton's third law even with
//!   theta = 0 (exact pairwise forces).
//!
//! If `integrate_motions` is ever restructured into stage-synchronized passes
//! (rebuilding or re-evaluating the field between stages for all bodies), the
//! multi-stage rows here SHOULD start failing with *better* measured orders —
//! that failure is the desired signal. Re-characterize and update both this
//! file's expectations and the integrator-selection advice in the docs.
//!
//! To re-derive measured values:
//! `cargo test --test two_body_system characterize -- --ignored --nocapture`

use bevy::prelude::*;
use bevy::tasks::{ComputeTaskPool, TaskPool};
use stardrift::physics::components::{Mass, Position, Velocity};
use stardrift::physics::integrators::Integrator;
use stardrift::physics::integrators::registry::IntegratorRegistry;
use stardrift::physics::math::{Scalar, Vector};
use stardrift::physics::octree::Octree;
use stardrift::physics::resources::{CurrentIntegrator, PhysicsTime};
use stardrift::plugins::simulation::physics::{integrate_motions, rebuild_octree};
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
        // integrate_motions uses par_iter_mut, which needs the task pool.
        ComputeTaskPool::get_or_init(TaskPool::default);

        let registry = IntegratorRegistry::new().with_standard_integrators();
        let integrator: Box<dyn Integrator + Send + Sync> = registry
            .create(integrator_name)
            .unwrap_or_else(|e| panic!("{e}"));

        let mut world = World::new();
        // theta = 0: exact pairwise forces, so any momentum or order defect
        // measured here comes from the integration scheme, not the octree
        // approximation. min_distance far below perihelion separation (0.5).
        world.insert_resource(GravitationalOctree::new(Octree::new(0.0, 1e-6, 1e12)));
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

        // Production ordering: PhysicsSet::BuildOctree then IntegrateMotions.
        let mut schedule = Schedule::default();
        schedule.add_systems((rebuild_octree, integrate_motions).chain());

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
/// (characterized 2026-08-25). All are ~1: the frozen octree field caps every
/// multi-stage method at first order. The multi-stage methods do not merely
/// share the order — they collapse onto the *same* error curve (velocity
/// Verlet, RK4, and PEFRL agree to three digits), because the frozen-field
/// O(dt) term dominates whatever the integrator itself contributes.
const SYSTEM_ORDER_BANDS: &[(&str, (Scalar, Scalar))] = &[
    ("explicit_euler", (0.7, 1.4)),                    // measured 0.82
    ("symplectic_euler", (0.7, 1.4)),                  // measured 0.89
    ("velocity_verlet", (0.7, 1.4)),                   // measured 0.96
    ("heun", (0.7, 1.4)),                              // measured 0.98
    ("runge_kutta_second_order_midpoint", (0.7, 1.4)), // measured 0.95
    ("runge_kutta_fourth_order", (0.7, 1.4)),          // measured 0.95
    ("pefrl", (0.7, 1.4)),                             // measured 0.95
];

#[test]
fn system_convergence_order_is_first_order_for_all_integrators() {
    for (name, (lo, hi)) in SYSTEM_ORDER_BANDS {
        let (order, r_squared, errors) = measure_system_order(name);
        assert!(
            order > *lo && order < *hi,
            "{name}: system-level convergence order {order:.3} outside ({lo}, {hi}) \
             (R^2 = {r_squared:.5}, errors: {errors:?}).\n\
             If this integrator now measures *higher* than first order, the \
             frozen-field limitation in integrate_motions has changed — \
             re-characterize this suite and update the module docs."
        );
    }
}

#[test]
fn single_eval_methods_conserve_momentum_to_roundoff() {
    // One field evaluation at pre-step positions: pairwise forces are exactly
    // antisymmetric, so total momentum survives to roundoff.
    for name in ["explicit_euler", "symplectic_euler"] {
        let drift = measure_momentum_drift(name);
        assert!(
            drift < 1e-12,
            "{name}: single-evaluation momentum drift {drift:.3e} above roundoff"
        );
    }
}

#[test]
fn multi_stage_methods_leak_momentum_through_frozen_field() {
    // Intermediate stages see the partner at its stale pre-step position;
    // action does not equal reaction, and momentum drifts secularly even with
    // exact (theta = 0) forces. All five multi-stage methods measured ~3.1e-3
    // over 5/8 period at dt = T/1000.
    for name in [
        "velocity_verlet",
        "heun",
        "runge_kutta_second_order_midpoint",
        "runge_kutta_fourth_order",
        "pefrl",
    ] {
        let drift = measure_momentum_drift(name);
        let (lo, hi) = (3e-4, 3e-2);
        assert!(
            drift > lo && drift < hi,
            "{name}: momentum drift {drift:.3e} outside measured band \
             ({lo:.0e}, {hi:.0e}).\n\
             A drift at roundoff level means integrate_motions no longer \
             freezes the field between stages — re-characterize this suite."
        );
    }
}

// =============================================================================
// Characterization helper
// =============================================================================

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
