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
  pair separations are bounded below by the contact distance in the steady
  state, so — by the shell theorem — the point-mass force is exact for the
  non-overlapping uniform spheres the bodies represent, up to one caveat:
  merges are applied after the step, so on the merge step itself the force
  evaluations (including velocity Verlet's cached final stage) see the
  pre-merge, possibly overlapping configuration for that one step. With
  collisions disabled, persistently overlapping bodies void the
  bounded-energy theorem (not the symplecticity).
- Collision detection sweeps each body's per-step segment
  (`PreviousPosition` → `Position`, both written by the driver) between
  integration and drift correction. Linearizing the step is valid for
  dt ≪ sqrt(6/(πGρ)) ≈ 0.138 s at defaults — the 60 Hz step clears it 8x.
  Pair curvature errs only toward slightly-early merges (bounded by
  (π/6)Gρdt², ~1.5% of contact distance worst case); third-body tidal
  bending near a massive merged clump can locally exceed the pair term and
  in principle miss a grazing encounter by a hair, with still-converging
  pairs caught on a later step (see the module doc in
  `src/plugins/simulation/collisions.rs`). Merges conserve mass,
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

## Instrumented: what theta costs

`--bh-probe` (or `[physics.bh_probe] enabled = true`) measures the
Barnes-Hut field against exact pairwise summation on the live stage snapshot,
through the same force law, so the numbers isolate pure approximation error
(`src/physics/bh_probe.rs`; tests in `tests/bh_probe.rs`). Three figures,
sampled on the first octree build of every 15th step by default, logged
under `stardrift::bh_probe` and shown in the diagnostics HUD:

- `accel_error_l2` — `sqrt(Σ|a_bh − a_ref|² / Σ|a_ref|²)`, the standard
  scale-free treecode figure; the headline.
- `accel_error_max` — largest per-body relative error, with the denominator
  floored at 1e-3 of the RMS exact acceleration so field nulls cannot
  dominate it.
- `topology_churn` — fraction of bodies (among those present in both
  samples) whose root-to-leaf octant path changed since the previous sample.
  The build is translation- and scale-equivariant, so uniform drift and pure
  dispersal do not register; relative motion does, and so does a hull body
  shifting the root box, which can rekey the whole population at once. That
  amplification is what the quantized-root-box item below is about.

Measured 2026-09-09, seed 42, velocity Verlet, all other settings default
(collisions on, so the body count declines — the last column is the count
at the end), five minutes of wall time per run, 1200 samples each. Mean
over all samples with the per-sample maximum in parentheses:

| n | theta | `accel_error_l2` | `accel_error_max` | `topology_churn` | bodies |
|---|-------|------------------|-------------------|------------------|--------|
| 25 | 0.0 | 1.5e-16 (3.8e-16) | 3.2e-16 (2.2e-15) | 6.0 % (57 %) | 12 |
| 25 | 0.5 | 1.2e-4 (2.0e-3) | 4.1e-3 (8.8e-2) | 7.3 % (47 %) | 12 |
| 25 | 1.0 | 1.5e-3 (3.1e-2) | 1.8e-2 (2.0e-1) | 5.8 % (69 %) | 9 |
| 1000 | 0.0 | 9.4e-16 (5.0e-15) | 3.6e-15 (7.5e-15) | 0.32 % (1.6 %) | 870 |
| 1000 | 0.5 | 1.3e-4 (8.8e-4) | 1.5e-2 (3.9e-2) | 0.31 % (1.5 %) | 870 |
| 1000 | 1.0 | 9.4e-4 (8.2e-3) | 9.9e-2 (2.3e-1) | 0.32 % (1.4 %) | 870 |
| 5000 | 0.5 | 8.4e-5 (4.0e-4) | 2.5e-2 (4.4e-2) | 0.11 % (0.4 %) | 4796 |

Reading it: theta = 0 sits at roundoff, which is the probe's own sanity
check. The default theta = 0.5 costs about one part in 10⁴ in the L2 sense
at every body count, while the worst single body is off by a percent or
two at any moment. Theta = 1.0 is an order of magnitude worse in L2 and
lets individual bodies (near a field null, or where a coarse node covering
a close pair is accepted) run 10–20 % off.

The error is not stationary. The **opening 30 s is the worst window for
L2** — the fresh spawn shell presents every body with many equidistant
groups that pass the acceptance test, and the L2 figure then falls by
4–10× over the first two minutes as bodies merge, mass concentrates, and
each body's field comes to be dominated by near neighbours that are always
evaluated exactly (n = 1000, theta = 0.5: 2.3e-4 in the first 30 s,
1.1e-4 at 2–3 min, 9.5e-5 at 4–5 min). The per-body maximum does **not**
fall with it (same run: 1.0e-2 → 1.8e-2 → 1.4e-2): as the field gets
lumpier, some body is always next to an accepted node that misrepresents
it. A short run therefore overstates the typical error and understates
the worst case.

Churn is a property of the trajectory, not of theta, and it grows as the
system evolves: at n = 25 it climbs from ~2 % per sample interval in the
opening window to 6–7 % later, with bursts above 30 % whenever a hull body
or a merge rekeys a cluster; the theta runs only separate once their
trajectories have diverged (they are identical for the first 30 s). At
n = 1000 it stays near 0.3 % with 1.5 % bursts, and at n = 5000 near
0.1 %. Small and bursty at every size — evidence against, not for,
spending effort on a quantized root box.

Cost: one extra O(N²) pass per sample, parallelised across the compute
task pool — measured at 0.2 ms per sample at n = 1000 and 3 ms at n = 5000
on an 18-thread machine, four samples per second. Off by default; with it
off the driver pays one branch per build and seeded frame-based
screenshots are byte-identical.

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
