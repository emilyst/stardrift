# Numerical Integrators Guide

This guide describes the numerical integration methods available in Stardrift
and helps you choose one. Run `stardrift --list-integrators` for the accepted
names and aliases; for the design of the integration pipeline itself, see
[Integration design](integration.md).

## Background

An N-body simulation advances bodies through time in discrete steps, and the
integration method determines how each step approximates the true motion. The
choice affects accuracy per step, long-term stability, whether total energy
drifts, and computational cost.

The integrators fall into two families:

- **Symplectic** methods (`symplectic_euler`, `velocity_verlet`, `pefrl`)
  preserve the geometric structure of the underlying physics. Their energy
  error oscillates but does not systematically drift, which is what you want
  for orbital mechanics.
- **Explicit** methods (`explicit_euler`, `heun`, `rk2`, `rk4`) can be more
  accurate per step but accumulate energy drift over time. They suit short
  runs and comparisons, not long-term evolution.

One caveat: symplecticity holds exactly only when forces are exact
(`octree_theta = 0`). At the default theta the Barnes-Hut approximation
breaks it, though `velocity_verlet` and `pefrl` remain exactly
time-reversible at any theta, which still eliminates the integrator's
contribution to secular drift. [Integration design](integration.md) covers
this in detail.

## Quick Recommendations

| Use Case | Recommended Integrator |
|----------|----------------------|
| General use | `velocity_verlet` (the default) |
| Long runs where drift matters | `pefrl` |
| Cheapest acceptable | `symplectic_euler` |
| High accuracy, short runs | `runge_kutta_fourth_order` |
| Seeing what goes wrong | `explicit_euler` |

## Available Integrators

### Velocity Verlet (2nd order, symplectic) — default

**Name**: `velocity_verlet` | **Alias**: `verlet`

The workhorse of N-body simulation: symplectic, time-reversible, and a good
balance of speed and accuracy.

```
x(t+dt) = x(t) + v(t) * dt + 0.5 * a(t) * dt²
a(t+dt) = compute_acceleration(x(t+dt))
v(t+dt) = v(t) + 0.5 * (a(t) + a(t+dt)) * dt
```

Nominally two force evaluations per step, but the final evaluation is reused
as the next step's first (FSAL), so in practice it costs one octree build per
step — the same as the Euler methods.

### PEFRL (4th order, symplectic)

**Name**: `pefrl` | **Alias**: `forest_ruth`

Position-Extended Forest-Ruth-Like: a fourth-order symplectic method with
coefficients optimized to minimize the error constant. Four force evaluations
per step. Worth the cost for long runs at small theta where accuracy and
energy behavior matter most; overkill otherwise.

### Symplectic Euler (1st order, symplectic)

**Name**: `symplectic_euler` | **Aliases**: `euler`, `semi_implicit_euler`

The simplest symplectic method: update velocity first, then position with the
new velocity. Cheap and drift-free, but first-order accuracy means visible
trajectory error unless the timestep is small.

```
v(t+dt) = v(t) + a(t) * dt
x(t+dt) = x(t) + v(t+dt) * dt
```

### Explicit Euler (1st order)

**Name**: `explicit_euler` | **Alias**: `forward_euler`

The textbook first method, kept for education and comparison. Energy drifts
severely — orbits spiral in or out within seconds. Not for actual use.

```
x(t+dt) = x(t) + v(t) * dt
v(t+dt) = v(t) + a(t) * dt
```

### Heun's Method (2nd order)

**Name**: `heun` | **Alias**: `improved_euler`

A predictor-corrector: take a full Euler step, then average the derivatives
at the start and predicted end. More accurate than explicit Euler, still
drifts over long runs.

### RK2 Midpoint (2nd order)

**Name**: `runge_kutta_second_order_midpoint` | **Aliases**: `rk2`, `midpoint`

Second-order Runge-Kutta evaluating the derivative at the midpoint of the
step. Comparable to Heun in cost and behavior.

### RK4 (4th order)

**Name**: `runge_kutta_fourth_order` | **Alias**: `rk4`

The classic fourth-order Runge-Kutta method. Highly accurate per step and
well understood, but not symplectic — best for short, accuracy-critical runs.

## Comparison

| Integrator | Order | Force Evals/Step | Symplectic | Energy Behavior |
|------------|-------|------------------|------------|-----------------|
| `explicit_euler` | 1 | 1 | No | Severe drift |
| `symplectic_euler` | 1 | 1 | Yes | Bounded |
| `heun` | 2 | 2 | No | Drifts |
| `rk2` | 2 | 2 | No | Drifts |
| `velocity_verlet` | 2 | 2 (1 with FSAL) | Yes | Bounded |
| `rk4` | 4 | 4 | No | Drifts |
| `pefrl` | 4 | 4 | Yes | Bounded, smallest error |

("Bounded" assumes exact forces; see the theta caveat above.)

## Configuration

Set the integrator in your config file:

```toml
[physics.integrator]
type = "velocity_verlet"
```

Or via the command line:

```bash
stardrift --integrator pefrl
```

## Further Reading

- [Leapfrog integration](https://en.wikipedia.org/wiki/Leapfrog_integration) - Wikipedia article on Verlet methods
- [Symplectic integrator](https://en.wikipedia.org/wiki/Symplectic_integrator) - Why symplectic methods matter
- [N-body simulation](https://en.wikipedia.org/wiki/N-body_simulation) - Overview of the problem domain

## See Also

- [Integration design](integration.md) - The staged integration pipeline and its guarantees
- [Configuration Reference](configuration.md) - Full configuration options
- [Architecture](architecture.md) - How the physics engine is implemented
