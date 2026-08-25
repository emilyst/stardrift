//! Registry-driven correctness harness for all numerical integrators.
//!
//! Every integrator in the standard registry is characterized by an entry in
//! [`EXPECTATIONS`]. A newly registered integrator fails the registry test
//! until an entry is added, and the entry's values must come from measurement
//! (see the `characterize` test below).
//!
//! Test families and what they catch:
//!
//! 1. **Trajectory accuracy** (3-D anisotropic oscillator vs. closed form):
//!    gross formula errors. Bands are *two-sided* — a method that becomes
//!    accidentally more accurate has turned into a different method, which is
//!    also a bug.
//! 2. **Convergence order and error constant** (eccentric Kepler orbit vs.
//!    Kepler's equation): order loss from refactors, and — via the fitted
//!    error constant — wrong-but-order-preserving coefficient changes.
//! 3. **Conservation** (inclined eccentric Kepler orbit): broken symplectic
//!    structure. Asserts drift *character* (bounded vs. secular) in addition
//!    to magnitude. Angular momentum is conserved exactly by any
//!    drift-kick splitting on a central force, so the splitting methods are
//!    held to roundoff; explicit Euler has an exact closed-form growth law.
//! 4. **Reversibility classification**: broken palindromic structure. Run for
//!    all integrators — the non-reversible ones must *fail* to reverse by
//!    their expected margin.
//! 5. **Stage structure** (spy field): wrong force-evaluation count or a
//!    stage that ignores its intermediate position.
//! 6. **Golden values** (10 Kepler steps vs. hard-coded reference): pins
//!    every coefficient and stage ordering to ~1e-12.
//! 7. **Registry integrity**: name/alias collisions, uncharacterized
//!    integrators, wrong claimed order.
//!
//! To re-derive measured values after an intentional change:
//! `cargo test --test integrator_correctness characterize -- --ignored --nocapture`

use stardrift::physics::integrators::registry::IntegratorRegistry;
use stardrift::physics::integrators::{AccelerationField, Integrator};
use stardrift::physics::math::{Scalar, Vector};
use std::f64::consts::PI;
use std::sync::Mutex;

// =============================================================================
// Acceleration fields
// =============================================================================

/// Anisotropic 3-D harmonic oscillator: a_i = -omega_i^2 * x_i.
///
/// Anisotropy ensures a bug confined to one component (dropped z, swapped
/// axes) changes the trajectory. A 1-D field embedded in 3-D cannot see those.
struct AnisotropicOscillator {
    omega_squared: Vector,
}

impl AccelerationField for AnisotropicOscillator {
    fn at(&self, position: Vector) -> Vector {
        -(self.omega_squared * position)
    }
}

/// Inverse-square central force: a = -GM * x / |x|^3.
///
/// Nonlinear, so it distinguishes methods that coincide on linear problems
/// (Heun and RK2 midpoint are identical on any linear field).
struct InverseSquare {
    gm: Scalar,
}

impl AccelerationField for InverseSquare {
    fn at(&self, position: Vector) -> Vector {
        let r_squared = position.length_squared();
        let r = r_squared.sqrt();
        -position * (self.gm / (r_squared * r))
    }
}

/// Wraps another field and records every query position.
struct SpyField<'a> {
    inner: &'a dyn AccelerationField,
    queries: Mutex<Vec<Vector>>,
}

impl AccelerationField for SpyField<'_> {
    fn at(&self, position: Vector) -> Vector {
        self.queries.lock().unwrap().push(position);
        self.inner.at(position)
    }
}

// =============================================================================
// Exact solutions
// =============================================================================

/// Exact state of the anisotropic oscillator at time `t` (componentwise).
fn oscillator_exact(omega: Vector, x0: Vector, v0: Vector, t: Scalar) -> (Vector, Vector) {
    let mut position = Vector::ZERO;
    let mut velocity = Vector::ZERO;
    for i in 0..3 {
        let (sin_wt, cos_wt) = (omega[i] * t).sin_cos();
        position[i] = x0[i] * cos_wt + v0[i] / omega[i] * sin_wt;
        velocity[i] = -x0[i] * omega[i] * sin_wt + v0[i] * cos_wt;
    }
    (position, velocity)
}

/// Exact state of a planar Kepler orbit at time `t` after perihelion passage,
/// via Newton iteration on Kepler's equation M = E - e sin E.
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

// =============================================================================
// Shared configuration
// =============================================================================

const KEPLER_GM: Scalar = 1.0;
const KEPLER_A: Scalar = 1.0;
const KEPLER_E: Scalar = 0.5;
/// Orbital period for GM = a = 1.
const KEPLER_T: Scalar = 2.0 * PI;

fn oscillator_omega() -> Vector {
    Vector::new(1.0, std::f64::consts::SQRT_2, 3.0_f64.sqrt())
}

/// Non-degenerate oscillator initial conditions: both x0 and v0 nonzero and
/// off-axis, so no component or symmetry is trivially zero.
fn oscillator_ics() -> (Vector, Vector) {
    (Vector::new(1.0, 0.7, -0.4), Vector::new(0.3, -0.5, 0.6))
}

/// Planar Kepler perihelion start: r = a(1-e), v = sqrt(GM(1+e)/(a(1-e))).
fn kepler_planar_ics() -> (Vector, Vector) {
    let r_peri = KEPLER_A * (1.0 - KEPLER_E);
    let v_peri = (KEPLER_GM * (1.0 + KEPLER_E) / r_peri).sqrt();
    (Vector::new(r_peri, 0.0, 0.0), Vector::new(0.0, v_peri, 0.0))
}

/// The planar orbit rotated by Rz(0.5) * Rx(0.4) so all three components of
/// angular momentum are nonzero — a z-leak or orientation bug is visible.
fn kepler_inclined_ics() -> (Vector, Vector) {
    let rotate = |p: Vector| {
        let (sin_a, cos_a) = 0.4_f64.sin_cos();
        let rx = Vector::new(p.x, p.y * cos_a - p.z * sin_a, p.y * sin_a + p.z * cos_a);
        let (sin_b, cos_b) = 0.5_f64.sin_cos();
        Vector::new(
            rx.x * cos_b - rx.y * sin_b,
            rx.x * sin_b + rx.y * cos_b,
            rx.z,
        )
    };
    let (x0, v0) = kepler_planar_ics();
    (rotate(x0), rotate(v0))
}

fn kepler_energy(position: Vector, velocity: Vector) -> Scalar {
    0.5 * velocity.length_squared() - KEPLER_GM / position.length()
}

/// Phase-space error norm. Includes velocity so the error cannot vanish at a
/// turning point (where position error is second-order in phase error).
/// Positions and velocities are O(1) in the natural units used throughout.
fn phase_space_error(dx: Vector, dv: Vector) -> Scalar {
    (dx.length_squared() + dv.length_squared()).sqrt()
}

/// Least-squares fit of log(err) = log(C) + p * log(h).
/// Returns (order p, constant C, R^2).
fn fit_power_law(step_sizes: &[Scalar], errors: &[Scalar]) -> (Scalar, Scalar, Scalar) {
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
    let intercept = y_mean - slope * x_mean;
    let r_squared = (ss_xy * ss_xy) / (ss_xx * ss_yy);
    (slope, intercept.exp(), r_squared)
}

fn registry() -> IntegratorRegistry {
    IntegratorRegistry::new().with_standard_integrators()
}

fn make(name: &str) -> Box<dyn Integrator> {
    registry().create(name).unwrap_or_else(|e| panic!("{e}"))
}

// =============================================================================
// Expectation table
// =============================================================================

/// How angular momentum should behave on a central-force orbit.
enum AngularMomentum {
    /// Drift-kick splittings conserve L exactly by structure (every drift and
    /// every kick preserves x cross v when a is parallel to x). Held to a
    /// roundoff-level cap, componentwise and signed.
    ExactToRoundoff { cap: Scalar },
    /// Explicit Euler multiplies L by exactly (1 + h^2 GM / r^3) each step.
    /// Checked against that closed form along the numerical trajectory.
    ClosedFormExplicitEuler,
    /// Non-symplectic multi-stage methods drift secularly. Two-sided band on
    /// |L_end - L_0| / |L_0| at the end of the conservation run.
    SecularBand { band: (Scalar, Scalar) },
}

struct Expectation {
    name: &'static str,
    aliases: &'static [&'static str],
    order: usize,
    force_evals_per_step: usize,
    symplectic: bool,
    reversible: bool,
    /// Two-sided band on the endpoint phase-space error of the trajectory
    /// test (oscillator, dt = 0.02, t = 10).
    trajectory_error: (Scalar, Scalar),
    /// Steps-per-period values for the order study (dt = T / n).
    order_steps_per_period: &'static [usize],
    /// Two-sided band on the fitted error constant C in err = C * h^p.
    order_constant: (Scalar, Scalar),
    /// Orbits for the conservation run; the run continues to 2x this to
    /// measure drift character.
    conservation_orbits: usize,
    /// Two-sided band on running-max |dE|/|E0| at `conservation_orbits`.
    energy_drift: (Scalar, Scalar),
    /// Band on (running max at 2t) / (running max at t): ~1 for bounded
    /// (symplectic), ~2 for secular. None for explicit Euler, whose orbit
    /// unbinds rather than drifting politely.
    drift_character: Option<(Scalar, Scalar)>,
    angular_momentum: AngularMomentum,
    /// Two-sided band on the forward-backward residual (phase-space norm).
    reversibility_residual: (Scalar, Scalar),
    /// (position, velocity) after 10 Kepler steps at dt = T/200 from the
    /// planar perihelion start. Golden values pin every coefficient and the
    /// stage ordering; re-derive deliberately (see module docs) if an
    /// integrator is intentionally changed.
    golden: ([Scalar; 3], [Scalar; 3]),
}

const STANDARD_ORDER_STEPS: &[usize] = &[200, 400, 800, 1600];
/// First-order methods need much smaller h to be in the asymptotic regime on
/// this orbit (their error at T/200 is O(1)).
const FINE_ORDER_STEPS: &[usize] = &[3200, 6400, 12800, 25600];

// Bands below are two-sided with roughly 3x headroom around values measured
// by the `characterize` test on 2026-08-25. Sanity checks against theory:
// Heun and RK2-midpoint are bitwise-identical on the (linear) oscillator and
// differ only on Kepler; velocity Verlet's oscillator error is the predicted
// (omega h)^2 phase error; the splitting methods conserve L to roundoff;
// PEFRL's Kepler error constant is ~3x below RK4's (Omelyan et al.'s
// optimized coefficients — a swap to plain Forest-Ruth coefficients would
// land far outside its band).

static EXPECTATIONS: &[Expectation] = &[
    Expectation {
        name: "explicit_euler",
        aliases: &["forward_euler"],
        order: 1,
        force_evals_per_step: 1,
        symplectic: false,
        reversible: false,
        trajectory_error: (1e-1, 8e-1), // measured 2.76e-1 (amplitude growth)
        order_steps_per_period: FINE_ORDER_STEPS,
        order_constant: (2e1, 1.8e2), // measured 6.03e1
        conservation_orbits: 5,
        energy_drift: (2e-1, 8e-1), // measured 4.35e-1 — unbinds if run longer
        drift_character: None,
        angular_momentum: AngularMomentum::ClosedFormExplicitEuler,
        reversibility_residual: (3e-1, 3.0), // measured 1.02
        golden: (
            [0.34276158605451185, 0.5014490719141789, 0.0],
            [-0.9646261786465633, 1.285929180009859, 0.0],
        ),
    },
    Expectation {
        name: "symplectic_euler",
        aliases: &["euler", "semi_implicit_euler"],
        order: 1,
        force_evals_per_step: 1,
        symplectic: true,
        reversible: false,
        trajectory_error: (5e-3, 5e-2), // measured 1.57e-2
        order_steps_per_period: FINE_ORDER_STEPS,
        order_constant: (1.5, 1.3e1), // measured 4.38
        conservation_orbits: 50,
        energy_drift: (3e-3, 3e-2),         // measured 9.08e-3
        drift_character: Some((0.95, 1.5)), // bounded; measured 1.0000
        angular_momentum: AngularMomentum::ExactToRoundoff { cap: 1e-12 },
        reversibility_residual: (2e-2, 3e-1), // measured 8.09e-2
        golden: (
            [0.3058630070350699, 0.4824244951234116, 0.0],
            [-1.0102282302784296, 1.2380266692059696, 0.0],
        ),
    },
    Expectation {
        name: "velocity_verlet",
        aliases: &["verlet"],
        order: 2,
        force_evals_per_step: 2,
        symplectic: true,
        reversible: true,
        trajectory_error: (2.5e-4, 2e-3), // measured 7.07e-4
        order_steps_per_period: STANDARD_ORDER_STEPS,
        order_constant: (6.0, 6e1), // measured 1.90e1
        conservation_orbits: 50,
        energy_drift: (4e-5, 3e-4),         // measured 1.07e-4
        drift_character: Some((0.95, 1.5)), // bounded; measured 1.0000
        angular_momentum: AngularMomentum::ExactToRoundoff { cap: 1e-12 },
        reversibility_residual: (0.0, 1e-11), // measured 4.68e-16
        golden: (
            [0.3285810827021065, 0.4852173550070203, 0.0],
            [-0.954274589575545, 1.226469911676383, 0.0],
        ),
    },
    Expectation {
        name: "heun",
        aliases: &["improved_euler"],
        order: 2,
        force_evals_per_step: 2,
        symplectic: false,
        reversible: false,
        trajectory_error: (8e-4, 7e-3), // measured 2.46e-3 (== rk2: linear problem)
        order_steps_per_period: STANDARD_ORDER_STEPS,
        order_constant: (2e1, 1.8e2), // measured 6.09e1 (!= rk2: nonlinear problem)
        conservation_orbits: 50,
        energy_drift: (3e-4, 3e-3),        // measured 9.97e-4
        drift_character: Some((1.4, 2.2)), // secular; measured 1.75
        angular_momentum: AngularMomentum::SecularBand { band: (8e-5, 8e-4) }, // measured 2.51e-4
        reversibility_residual: (4e-4, 4e-3), // measured 1.16e-3
        golden: (
            [0.32916999525807067, 0.4854934592416741, 0.0],
            [-0.9507719370092144, 1.2289393587577577, 0.0],
        ),
    },
    Expectation {
        name: "runge_kutta_second_order_midpoint",
        aliases: &["rk2", "midpoint"],
        order: 2,
        force_evals_per_step: 2,
        symplectic: false,
        reversible: false,
        trajectory_error: (8e-4, 7e-3), // measured 2.46e-3 (== heun: linear problem)
        order_steps_per_period: STANDARD_ORDER_STEPS,
        order_constant: (2.0, 2e1), // measured 6.15 (!= heun: nonlinear problem)
        conservation_orbits: 50,
        energy_drift: (1e-4, 1e-3),        // measured 3.33e-4
        drift_character: Some((1.6, 2.4)), // secular; measured 1.98
        angular_momentum: AngularMomentum::SecularBand {
            band: (1.6e-5, 1.6e-4),
        }, // measured 4.93e-5
        reversibility_residual: (4e-4, 4e-3), // measured 1.16e-3
        golden: (
            [0.32858683687019075, 0.48510728999624936, 0.0],
            [-0.9537965807617615, 1.2256433901588297, 0.0],
        ),
    },
    Expectation {
        name: "runge_kutta_fourth_order",
        aliases: &["rk4"],
        order: 4,
        force_evals_per_step: 4,
        symplectic: false,
        reversible: false,
        trajectory_error: (4e-8, 4e-7), // measured 1.26e-7
        order_steps_per_period: STANDARD_ORDER_STEPS,
        order_constant: (6.0, 5.5e1), // measured 1.77e1
        conservation_orbits: 50,
        energy_drift: (5e-9, 5e-8),        // measured 1.65e-8
        drift_character: Some((1.5, 2.3)), // secular; measured 1.90
        angular_momentum: AngularMomentum::SecularBand {
            band: (1.5e-9, 1.5e-8),
        }, // measured 4.59e-9
        reversibility_residual: (1.5e-7, 1.5e-6), // measured 4.48e-7
        golden: (
            [0.32870905897024605, 0.4846963399830578, 0.0],
            [-0.9556640001094447, 1.2254560506563958, 0.0],
        ),
    },
    Expectation {
        name: "pefrl",
        aliases: &["forest_ruth"],
        order: 4,
        force_evals_per_step: 4,
        symplectic: true,
        reversible: true,
        trajectory_error: (2e-10, 2e-9), // measured 6.03e-10
        order_steps_per_period: STANDARD_ORDER_STEPS,
        order_constant: (2.0, 1.8e1), // measured 5.88
        conservation_orbits: 50,
        energy_drift: (4e-10, 4e-9),        // measured 1.26e-9
        drift_character: Some((0.95, 1.5)), // bounded; measured 1.0000
        angular_momentum: AngularMomentum::ExactToRoundoff { cap: 1e-12 },
        reversibility_residual: (0.0, 1e-11), // measured 7.32e-16
        golden: (
            [0.32870908085659284, 0.48469684397583723, 0.0],
            [-0.955662278522325, 1.2254572110639348, 0.0],
        ),
    },
];

fn expectation(name: &str) -> &'static Expectation {
    EXPECTATIONS
        .iter()
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("no expectation entry for integrator '{name}'"))
}

// =============================================================================
// Measurement machinery (shared by tests and the characterize helper)
// =============================================================================

const TRAJECTORY_DT: Scalar = 0.02;
const TRAJECTORY_STEPS: usize = 500; // t = 10

/// Endpoint phase-space error on the anisotropic oscillator.
fn measure_trajectory_error(integrator: &dyn Integrator) -> Scalar {
    let omega = oscillator_omega();
    // Largest omega * dt must stay well inside the asymptotic regime.
    assert!(omega.z * TRAJECTORY_DT <= 0.05);
    let field = AnisotropicOscillator {
        omega_squared: omega * omega,
    };
    let (mut position, mut velocity) = oscillator_ics();
    for _ in 0..TRAJECTORY_STEPS {
        integrator.step(&mut position, &mut velocity, &field, TRAJECTORY_DT);
    }
    let t = TRAJECTORY_DT * TRAJECTORY_STEPS as Scalar;
    let (x0, v0) = oscillator_ics();
    let (exact_position, exact_velocity) = oscillator_exact(omega, x0, v0, t);
    // Scale velocity error per-component by 1/omega for a proper phase-space norm.
    let dv = (velocity - exact_velocity) / omega;
    phase_space_error(position - exact_position, dv)
}

/// Integrate 5/8 of a Kepler period at each step count; returns (h, error)
/// pairs. The endpoint is deliberately *not* a whole period: at t = T the
/// first-order fixed phase offset of symplectic Euler cancels and it measures
/// as second-order (and turning-point endpoints hide phase error generally).
fn measure_order_errors(
    integrator: &dyn Integrator,
    steps_per_period: &[usize],
) -> Vec<(Scalar, Scalar)> {
    let field = InverseSquare { gm: KEPLER_GM };
    let t_end = KEPLER_T * 5.0 / 8.0;
    let (exact_position, exact_velocity) = kepler_exact(KEPLER_A, KEPLER_E, KEPLER_GM, t_end);
    steps_per_period
        .iter()
        .map(|&n| {
            assert!(n % 8 == 0, "steps per period must be divisible by 8");
            let dt = KEPLER_T / n as Scalar;
            let (mut position, mut velocity) = kepler_planar_ics();
            for _ in 0..n * 5 / 8 {
                integrator.step(&mut position, &mut velocity, &field, dt);
            }
            let error = phase_space_error(position - exact_position, velocity - exact_velocity);
            (dt, error)
        })
        .collect()
}

struct ConservationOutcome {
    /// Running max of |dE|/|E0| at `conservation_orbits`.
    max_drift_halfway: Scalar,
    /// Running max of |dE|/|E0| at 2x `conservation_orbits`.
    max_drift_full: Scalar,
    /// L at the end of the full run.
    final_angular_momentum: Vector,
    initial_angular_momentum: Vector,
    /// Product of (1 + h^2 GM / r^3) along the trajectory (explicit Euler's
    /// exact per-step L growth factor).
    euler_growth_product: Scalar,
}

const CONSERVATION_STEPS_PER_ORBIT: usize = 1000;

fn measure_conservation(integrator: &dyn Integrator, orbits: usize) -> ConservationOutcome {
    let field = InverseSquare { gm: KEPLER_GM };
    let dt = KEPLER_T / CONSERVATION_STEPS_PER_ORBIT as Scalar;
    let (mut position, mut velocity) = kepler_inclined_ics();

    let initial_energy = kepler_energy(position, velocity);
    let initial_angular_momentum = position.cross(velocity);

    let halfway = orbits * CONSERVATION_STEPS_PER_ORBIT;
    let total = 2 * halfway;
    let mut max_drift = 0.0_f64;
    let mut max_drift_halfway = 0.0;
    let mut euler_growth_product = 1.0;

    for step in 0..total {
        let r_cubed = position.length().powi(3);
        euler_growth_product *= 1.0 + dt * dt * KEPLER_GM / r_cubed;
        integrator.step(&mut position, &mut velocity, &field, dt);
        let energy = kepler_energy(position, velocity);
        let drift = ((energy - initial_energy) / initial_energy).abs();
        max_drift = max_drift.max(drift);
        if step + 1 == halfway {
            max_drift_halfway = max_drift;
        }
    }

    ConservationOutcome {
        max_drift_halfway,
        max_drift_full: max_drift,
        final_angular_momentum: position.cross(velocity),
        initial_angular_momentum,
        euler_growth_product,
    }
}

const REVERSIBILITY_DT: Scalar = 0.05;
const REVERSIBILITY_STEPS: usize = 100;

/// Forward N steps, negate velocity, N more steps, negate again; residual
/// against the initial state in the phase-space norm.
fn measure_reversibility(integrator: &dyn Integrator) -> Scalar {
    let omega = oscillator_omega();
    let field = AnisotropicOscillator {
        omega_squared: omega * omega,
    };
    let (x0, v0) = oscillator_ics();
    let (mut position, mut velocity) = (x0, v0);
    for _ in 0..REVERSIBILITY_STEPS {
        integrator.step(&mut position, &mut velocity, &field, REVERSIBILITY_DT);
    }
    velocity = -velocity;
    for _ in 0..REVERSIBILITY_STEPS {
        integrator.step(&mut position, &mut velocity, &field, REVERSIBILITY_DT);
    }
    velocity = -velocity;
    phase_space_error(position - x0, velocity - v0)
}

const GOLDEN_STEPS: usize = 10;

/// Ten Kepler steps at dt = T/200 from the planar perihelion start.
fn measure_golden(integrator: &dyn Integrator) -> (Vector, Vector) {
    let field = InverseSquare { gm: KEPLER_GM };
    let dt = KEPLER_T / 200.0;
    let (mut position, mut velocity) = kepler_planar_ics();
    for _ in 0..GOLDEN_STEPS {
        integrator.step(&mut position, &mut velocity, &field, dt);
    }
    (position, velocity)
}

// =============================================================================
// 1. Trajectory accuracy
// =============================================================================

#[test]
fn trajectory_accuracy() {
    for exp in EXPECTATIONS {
        let error = measure_trajectory_error(make(exp.name).as_ref());
        let (lo, hi) = exp.trajectory_error;
        assert!(
            error > lo && error < hi,
            "{}: trajectory error {error:.3e} outside band ({lo:.3e}, {hi:.3e})",
            exp.name
        );
    }
}

// =============================================================================
// 2. Convergence order and error constant
// =============================================================================

#[test]
fn convergence_order_and_constant() {
    for exp in EXPECTATIONS {
        let samples = measure_order_errors(make(exp.name).as_ref(), exp.order_steps_per_period);
        let (step_sizes, errors): (Vec<_>, Vec<_>) = samples.into_iter().unzip();

        let min_error = errors.iter().cloned().fold(f64::INFINITY, f64::min);
        assert!(
            min_error > 1e4 * f64::EPSILON,
            "{}: smallest order-study error {min_error:.3e} is near roundoff; \
             shrink the dt range (larger steps) so the measurement is meaningful",
            exp.name
        );

        let (order, constant, r_squared) = fit_power_law(&step_sizes, &errors);
        assert!(
            (order - exp.order as Scalar).abs() < 0.25,
            "{}: measured convergence order {order:.3} deviates from claimed {} \
             (errors: {errors:?})",
            exp.name,
            exp.order
        );
        assert!(
            r_squared > 0.999,
            "{}: order fit R^2 = {r_squared:.6} — errors do not follow a clean \
             power law (straddling regimes?): {errors:?}",
            exp.name
        );
        let (lo, hi) = exp.order_constant;
        assert!(
            constant > lo && constant < hi,
            "{}: fitted error constant {constant:.3e} outside band ({lo:.3e}, {hi:.3e}) — \
             order is intact but the error coefficient changed",
            exp.name
        );
    }
}

// =============================================================================
// 3. Conservation
// =============================================================================

fn run_conservation_test(name: &str) {
    let exp = expectation(name);
    let outcome = measure_conservation(make(name).as_ref(), exp.conservation_orbits);

    let (lo, hi) = exp.energy_drift;
    assert!(
        outcome.max_drift_halfway > lo && outcome.max_drift_halfway < hi,
        "{name}: max energy drift {:.3e} over {} orbits outside band ({lo:.3e}, {hi:.3e})",
        outcome.max_drift_halfway,
        exp.conservation_orbits
    );

    if let Some((ratio_lo, ratio_hi)) = exp.drift_character {
        let ratio = outcome.max_drift_full / outcome.max_drift_halfway;
        let character = if exp.symplectic { "bounded" } else { "secular" };
        assert!(
            ratio > ratio_lo && ratio < ratio_hi,
            "{name}: drift ratio (2t vs t) = {ratio:.3}, expected {character} \
             behavior in ({ratio_lo}, {ratio_hi})"
        );
    }

    let l0 = outcome.initial_angular_momentum;
    let l_final = outcome.final_angular_momentum;
    let l_scale = l0.length();
    match &exp.angular_momentum {
        AngularMomentum::ExactToRoundoff { cap } => {
            // Componentwise and signed: exact by drift-kick structure.
            for i in 0..3 {
                let rel = (l_final[i] - l0[i]).abs() / l_scale;
                assert!(
                    rel < *cap,
                    "{name}: angular momentum component {i} drifted by {rel:.3e} \
                     (cap {cap:.1e}); splitting structure is broken"
                );
            }
        }
        AngularMomentum::ClosedFormExplicitEuler => {
            // L' = L * (1 + h^2 GM / r^3) exactly, per step; direction unchanged.
            let expected = l_scale * outcome.euler_growth_product;
            let rel = (l_final.length() - expected).abs() / expected;
            assert!(
                rel < 1e-12,
                "{name}: |L| growth deviates from closed form by {rel:.3e}"
            );
            let alignment = l_final.normalize().dot(l0.normalize());
            assert!(
                alignment > 1.0 - 1e-12,
                "{name}: angular momentum direction changed (dot = {alignment})"
            );
        }
        AngularMomentum::SecularBand { band: (lo, hi) } => {
            let rel = (l_final - l0).length() / l_scale;
            assert!(
                rel > *lo && rel < *hi,
                "{name}: angular momentum drift {rel:.3e} outside band ({lo:.3e}, {hi:.3e})"
            );
        }
    }
}

macro_rules! conservation_tests {
    ($($test_name:ident => $integrator:literal),* $(,)?) => {
        $(
            #[test]
            fn $test_name() {
                run_conservation_test($integrator);
            }
        )*
    };
}

conservation_tests! {
    conservation_explicit_euler => "explicit_euler",
    conservation_symplectic_euler => "symplectic_euler",
    conservation_velocity_verlet => "velocity_verlet",
    conservation_heun => "heun",
    conservation_rk2_midpoint => "runge_kutta_second_order_midpoint",
    conservation_rk4 => "runge_kutta_fourth_order",
    conservation_pefrl => "pefrl",
}

/// Long soak: the symplectic methods must show *bounded* energy error over
/// 800 orbits — no secular drift. Run with `cargo test -- --ignored`.
#[test]
#[ignore = "long soak; run explicitly with --ignored (ideally --release)"]
fn conservation_soak_800_orbits() {
    for exp in EXPECTATIONS.iter().filter(|e| e.symplectic) {
        let outcome = measure_conservation(make(exp.name).as_ref(), 400);
        // Bounded means the 800-orbit max stays within a small multiple of
        // the 50-orbit band ceiling.
        let cap = exp.energy_drift.1 * 3.0;
        assert!(
            outcome.max_drift_full < cap,
            "{}: energy drift {:.3e} over 800 orbits exceeds {cap:.3e}; \
             secular drift in a symplectic method",
            exp.name,
            outcome.max_drift_full
        );
    }
}

// =============================================================================
// 4. Reversibility classification
// =============================================================================

#[test]
fn reversibility_classification() {
    for exp in EXPECTATIONS {
        let residual = measure_reversibility(make(exp.name).as_ref());
        let (lo, hi) = exp.reversibility_residual;
        let class = if exp.reversible {
            "reversible"
        } else {
            "non-reversible"
        };
        assert!(
            residual >= lo && residual < hi,
            "{}: forward-backward residual {residual:.3e} outside {class} band \
             ({lo:.3e}, {hi:.3e})",
            exp.name
        );
    }
}

// =============================================================================
// 5. Stage structure
// =============================================================================

#[test]
fn stage_structure() {
    let omega = oscillator_omega();
    let inner = AnisotropicOscillator {
        omega_squared: omega * omega,
    };

    for exp in EXPECTATIONS {
        let spy = SpyField {
            inner: &inner,
            queries: Mutex::new(Vec::new()),
        };
        let (mut position, mut velocity) = oscillator_ics();
        make(exp.name).step(&mut position, &mut velocity, &spy, 0.01);

        let queries = spy.queries.into_inner().unwrap();
        assert_eq!(
            queries.len(),
            exp.force_evals_per_step,
            "{}: expected {} force evaluations per step, saw {}",
            exp.name,
            exp.force_evals_per_step,
            queries.len()
        );
        // Consecutive stages of a multi-stage method must query *different*
        // positions; identical queries mean a stage ignores its intermediate
        // state (the frozen-field failure mode).
        for pair in queries.windows(2) {
            assert!(
                (pair[0] - pair[1]).length() > 1e-9,
                "{}: consecutive stage queries at identical positions {:?}",
                exp.name,
                pair[0]
            );
        }
    }
}

// =============================================================================
// 6. Golden values
// =============================================================================

#[test]
fn golden_values() {
    for exp in EXPECTATIONS {
        let (position, velocity) = measure_golden(make(exp.name).as_ref());
        let expected_position = Vector::from_array(exp.golden.0);
        let expected_velocity = Vector::from_array(exp.golden.1);
        let scale = expected_position.length().max(expected_velocity.length());
        let deviation =
            phase_space_error(position - expected_position, velocity - expected_velocity) / scale;
        assert!(
            deviation < 1e-12,
            "{}: state after {GOLDEN_STEPS} Kepler steps deviates from golden \
             reference by {deviation:.3e} (relative). If the integrator was \
             changed intentionally, re-derive via the characterize test.",
            exp.name
        );
    }
}

// =============================================================================
// 7. Registry integrity
// =============================================================================

#[test]
fn registry_integrity() {
    let registry = registry();

    let mut expected_names: Vec<String> = EXPECTATIONS.iter().map(|e| e.name.to_string()).collect();
    expected_names.sort();
    assert_eq!(
        registry.list_available(),
        expected_names,
        "registry contents do not match the expectation table; \
         characterize new integrators before registering them"
    );

    for exp in EXPECTATIONS {
        let integrator = registry.create(exp.name).unwrap();
        assert_eq!(integrator.name(), exp.name);
        assert_eq!(
            integrator.convergence_order(),
            exp.order,
            "{}: claimed convergence order changed",
            exp.name
        );
        for alias in exp.aliases {
            let via_alias = registry
                .create(alias)
                .unwrap_or_else(|e| panic!("alias '{alias}' failed to resolve: {e}"));
            assert_eq!(
                via_alias.name(),
                exp.name,
                "alias '{alias}' resolves to '{}', expected '{}' — alias collision",
                via_alias.name(),
                exp.name
            );
        }
    }

    // No alias may shadow a canonical name or another integrator's alias:
    // insertion overwrites silently, so a collision shows up as a short count.
    let expected_alias_total: usize = EXPECTATIONS.iter().map(|e| e.aliases.len()).sum();
    assert_eq!(
        registry.list_aliases().len(),
        expected_alias_total,
        "registry alias list does not match expectation table (collision or drop)"
    );
}

// =============================================================================
// Characterization helper
// =============================================================================

/// Prints every measured quantity the expectation table encodes. Run after an
/// intentional integrator change to re-derive bands and golden values:
/// `cargo test --test integrator_correctness characterize -- --ignored --nocapture`
#[test]
#[ignore = "measurement helper, not an assertion"]
fn characterize() {
    for exp in EXPECTATIONS {
        let integrator = make(exp.name);
        println!("== {} ==", exp.name);

        let trajectory = measure_trajectory_error(integrator.as_ref());
        println!("  trajectory_error: {trajectory:e}");

        let samples = measure_order_errors(integrator.as_ref(), exp.order_steps_per_period);
        let (step_sizes, errors): (Vec<_>, Vec<_>) = samples.into_iter().unzip();
        let (order, constant, r_squared) = fit_power_law(&step_sizes, &errors);
        println!("  order: {order:.4}  constant: {constant:e}  r2: {r_squared:.7}");
        let formatted: Vec<String> = errors.iter().map(|e| format!("{e:e}")).collect();
        println!("  order errors: [{}]", formatted.join(", "));

        let outcome = measure_conservation(integrator.as_ref(), exp.conservation_orbits);
        println!(
            "  energy drift @{} orbits: {:e}  @{} orbits: {:e}  ratio: {:.4}",
            exp.conservation_orbits,
            outcome.max_drift_halfway,
            2 * exp.conservation_orbits,
            outcome.max_drift_full,
            outcome.max_drift_full / outcome.max_drift_halfway
        );
        let dl = outcome.final_angular_momentum - outcome.initial_angular_momentum;
        println!(
            "  |dL|/|L|: {:e}  componentwise: [{:e}, {:e}, {:e}]",
            dl.length() / outcome.initial_angular_momentum.length(),
            dl.x,
            dl.y,
            dl.z
        );
        println!("  euler growth product: {:?}", outcome.euler_growth_product);

        let residual = measure_reversibility(integrator.as_ref());
        println!("  reversibility residual: {residual:e}");

        let (position, velocity) = measure_golden(integrator.as_ref());
        println!(
            "  golden: ([{:?}, {:?}, {:?}], [{:?}, {:?}, {:?}])",
            position.x, position.y, position.z, velocity.x, velocity.y, velocity.z
        );
    }
}
