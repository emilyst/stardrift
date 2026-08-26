# Integration design

How Stardrift advances the N-body system in time: the staged integrator
protocol, what it guarantees at which settings, and the trade-offs that were
accepted deliberately. Written alongside the 2026-08 restructure of
`integrate_motions`; the test evidence lives in `tests/two_body_system.rs`
and `tests/integrator_correctness.rs`.

## The staged protocol

A numerical integrator advances one body's `(position, velocity)` by one
timestep, evaluating the gravitational acceleration at one or more
intermediate states along the way. The naive N-body driver — build the octree
once, then step every body independently — is subtly wrong for any
multi-stage method: intermediate evaluations see the *other* bodies at their
pre-step positions. That staleness injects an O(dt), velocity-correlated
acceleration error which caps every method at first-order global accuracy,
breaks Newton's third law (secular momentum drift even with exact forces),
and destroys the symplectic and time-reversal structure the better
integrators are chosen for. Measured before the restructure: velocity
Verlet, RK4, and PEFRL all converged at order ~1 *on the same error curve* —
PEFRL bought 4x the force evaluations of symplectic Euler for essentially
identical trajectories.

The fix is stage synchronization, and the `Integrator` trait expresses it as
a query/apply conversation (`src/physics/integrators/mod.rs`):

- `next_query(&state, scratch, dt) -> Option<StageQuery>` — the next field
  evaluation the method needs, as a pure function of the per-body
  `StepState`; `None` means the step is complete.
- `apply_stage(&mut state, scratch, accel, dt)` — fold the evaluated
  acceleration in.
- `finish(&state, scratch, dt) -> (position, velocity)` — the committed
  end-of-step state (not necessarily a pure read: PEFRL applies its trailing
  drift here).
- `step(...)` — provided method driving the same protocol against an analytic
  field; the unit-level and N-body behaviors come from one implementation.

The driver (`integrate_motions` in `src/plugins/simulation/physics.rs`) runs
each stage as a global pass: gather every body's stage query into one
snapshot, rebuild the octree from that snapshot, evaluate every body's
acceleration against it, and only then let any body advance. The snapshot
buffer doubles as the octree's build input, which makes "all evaluations in a
stage see one consistent configuration" structural rather than a discipline.
Committed positions and velocities are written back exactly once per step.

Design choices worth knowing about:

- **The protocol is iterator-shaped, not indexed.** Drivers loop on
  `next_query` until `None` and must not assume a fixed stage count or
  monotonically increasing indices. This leaves room for implicit methods
  (repeated queries per stage while iterating to convergence) and adaptive
  methods (restart from the retained initial state) without another protocol
  change.
- **Queries carry velocity** (`StageQuery { position, velocity }`). The
  gravitational field ignores it; a future velocity-dependent force (drag,
  radiation pressure) will not need a trait change.
- **FSAL** (`reuses_final_stage`): velocity Verlet's final field evaluation
  is at the committed end-of-step position, which is exactly the next step's
  first query. The driver caches the final stage's accelerations and skips
  one octree build per step for such methods — so the default integrator
  costs one build per step, the same as before the restructure. The cache
  survives barycentric drift correction because accelerations are
  translation-invariant and the octree build is translation-equivariant (see
  the comment on `Octree::build`; do not break that property).
- **Scratch is driver-owned, flat, and sized per method**
  (`scratch_len()` slots per body). The RK family records its stage
  accelerations tableau-generally; the splitting methods use none.

## What it buys, honestly

All figures measured through the real pipeline on an eccentric (e = 0.5),
unequal-mass (3:1) two-body problem; see `tests/two_body_system.rs`.

At **theta = 0** (exact pairwise forces) the pipeline delivers everything the
integrators' docs claim:

- Nominal convergence orders, measured: explicit Euler 0.82, symplectic
  Euler 0.89 (both trending to 1 at finer dt), velocity Verlet 2.00,
  Heun 2.03, RK2 1.90, RK4 4.10, PEFRL 4.00.
- Linear momentum conserved to roundoff (~1e-15) by **all seven** methods.
- Angular momentum conserved to roundoff by the splitting methods
  (symplectic Euler, velocity Verlet, PEFRL) — a structural property of
  drift-kick compositions against central pairwise forces.
- The force law's clamps are conservative (the `min_distance` clamp is the
  interior-of-a-uniform-sphere law; the `max_force` clamp is a constant-
  magnitude radial force), so the system is genuinely Hamiltonian with a C1
  pair potential and the splitting integrators are exactly symplectic. The
  *bounded-energy-error* guarantee additionally assumes no pair sits inside
  the clamp radii. With merge-on-contact collisions enabled (the default),
  pair separations are bounded below by the contact distance, so bodies
  never visually overlap, both clamps are inactive for pairs, and — by the
  shell theorem — the point-mass force is *exact* for the non-overlapping
  uniform spheres the bodies represent. With collisions disabled,
  overlapping bodies void the bounded-energy theorem (not the
  symplecticity).
- Collision detection sweeps each body's per-step segment
  (`PreviousPosition` → `Position`, both written by the driver) between
  integration and drift correction. Linearizing the step is valid for
  dt ≪ sqrt(6/(πGρ)) ≈ 0.138 s at defaults — the 60 Hz step clears it 8x —
  and errs only toward slightly-early merges (bounded by (π/6)Gρdt², ~1.5%
  of contact distance worst case), never missed ones. Merges conserve mass,
  linear momentum, and the mass-weighted position sum to roundoff; kinetic
  energy and the pair's internal angular momentum are physically lost
  (inelastic collision — the latter would be the merged body's spin, which
  is not modeled). Energy and angular-momentum diagnostics must expect
  discrete downward steps at merges.

At **theta > 0** the Barnes-Hut acceptance criterion is not symmetric between
a pair of bodies, so the approximate field is not the gradient of any
potential. The scheme is **not** symplectic there, momentum is conserved only
to O(epsilon_BH), and the acceptance discontinuities cap the *measured*
convergence order near 1 for any practical dt. What the restructure delivers
at production theta is a large drop in the error *constant* — the dominant
velocity-correlated staleness term (a numerical friction) is gone — and one
exact structural property: the palindromic methods (velocity Verlet, PEFRL)
are exactly **time-reversible at any theta**, because the field depends only
on positions and the tree build is deterministic. Measured forward-backward
residuals through the pipeline: ~1e-15 for the palindromic methods, 4.6e-7
to 1.0 for the rest (two-body scene, so Barnes-Hut acceptance itself is not
exercised — the reversibility of acceptance flips follows from the same
determinism argument but awaits an N-body measurement). Reversibility eliminates the integrator's contribution to
secular energy drift; the residual drift is a property of the Barnes-Hut
approximation and shrinks with theta.

Practical guidance that follows: velocity Verlet remains the right default
(second order, symplectic at theta = 0, reversible at any theta, one octree
build per step via FSAL). PEFRL now genuinely earns its 4 builds per step
when accuracy matters and theta is small. RK4 is competitive only for short
runs; the Euler methods remain educational.

## Accepted losses

Named here so they are decisions, not accidents:

- **Multistep and Hermite methods** (Adams-Bashforth/Moulton; the Hermite
  predictor-corrector standard in collisional N-body). These need persistent
  per-body history across steps and — for Hermite — a jerk-computing octree
  traversal, neither of which exists. `StepState` is deliberately per-step.
- **Individual or block timesteps** (Aarseth-style). Fundamentally
  incompatible with the globally synchronized stage snapshot; a real-time
  visualizer wants uniform time anyway.

## Deferred, with reasons

- **Quantized root box.** Anchoring the octree's root to a power-of-two grid
  would reduce tree-topology churn (the current root AABB derives from the
  extremal bodies, so one body's motion shifts every octant boundary), but it
  breaks exact translation equivariance — which both barycentric drift
  correction and the FSAL cache silently depend on. Revisit only as a
  package deal.
- **epsilon_BH / topology-churn instrumentation.** Two cheap `--verbose`
  numbers (max relative acceleration error vs direct summation; bodies whose
  octant assignment changed per build) would turn the theta > 0 claims above
  from estimates into per-configuration facts.
- **Symmetric (dual-tree) traversal**, falcON-style: the known route to exact
  momentum conservation at theta > 0, at the cost of replacing the per-body
  traversal wholesale.
- **Adaptive global dt.** The retained initial state in `StepState` makes
  reject-and-retry cheap; what is missing is an embedded error estimate and a
  global controller (the dt must stay uniform across bodies). This is the
  natural answer to close encounters, which otherwise force a small fixed dt.

## Regression surface

- `tests/two_body_system.rs` — the pipeline: order recovery, momentum and
  angular momentum, reversibility at theta = 0.5, and bitwise anchors pinning
  the single-evaluation methods to the pre-restructure trajectories.
- `tests/integrator_correctness.rs` — the methods in isolation: trajectory
  accuracy, order and error constant, conservation character, reversibility
  classification, force-evaluation structure, golden values, registry
  integrity.

If `integrate_motions` changes shape again, the intended failure signal is
those suites' two-sided bands — better-than-expected results fail too, and
mean the expectations should be re-derived, not that the change is free.
