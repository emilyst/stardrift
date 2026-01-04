# Numerical Integrators Guide

This guide explains the numerical integration methods available in Stardrift and helps you choose the right one for your use case.

## What is a Numerical Integrator?

In an N-body simulation, we need to solve differential equations that describe how bodies move under gravitational forces. Since these equations generally can't be solved analytically, we use numerical integration to approximate the solution by taking discrete time steps.

The choice of integrator significantly affects:
- **Accuracy**: How close the simulation is to the true physical behavior
- **Stability**: Whether the simulation remains well-behaved over time
- **Energy conservation**: Whether total system energy drifts over time
- **Performance**: Computational cost per time step

## Quick Recommendations

| Use Case | Recommended Integrator |
|----------|----------------------|
| General use | `velocity_verlet` |
| Long-term simulations | `pefrl` |
| Performance-critical | `symplectic_euler` (with smaller timestep) |
| Educational/comparison | `explicit_euler` |
| High accuracy, short-term | `runge_kutta_fourth_order` |

## Integrator Categories

### Symplectic Integrators

Symplectic integrators preserve the geometric structure of Hamiltonian systems, which means they exhibit excellent long-term energy conservation. For gravitational simulations, this is crucial—non-symplectic methods will see energy slowly grow or decay, leading to unphysical behavior over time.

**Key property**: Energy oscillates around the true value but doesn't systematically drift.

### Explicit (Non-Symplectic) Integrators

Explicit integrators are general-purpose methods that can achieve high accuracy per step but don't preserve energy. They're suitable for short simulations or systems where energy conservation isn't critical.

**Key property**: Higher-order methods are more accurate per step, but energy drifts over time.

## Available Integrators

### Symplectic Euler (1st Order)

**Config name**: `"symplectic_euler"` | **Aliases**: `"euler"`, `"semi_implicit_euler"`

The simplest symplectic integrator. Updates velocity before position, which preserves phase space volume.

```
v(t+dt) = v(t) + a(t) * dt
x(t+dt) = x(t) + v(t+dt) * dt
```

| Aspect | Rating |
|--------|--------|
| Speed | Fastest |
| Accuracy | Low |
| Energy Conservation | Good |
| Force Evaluations | 1 per step |

**Pros:**
- Very fast computation
- Symplectic (no energy drift)
- Simple to understand and debug

**Cons:**
- Low accuracy requires smaller timesteps
- First-order convergence

**Best for**: Quick visualizations, real-time applications where performance matters more than precision.

---

### Velocity Verlet (2nd Order) — RECOMMENDED

**Config name**: `"velocity_verlet"` | **Alias**: `"verlet"`

The workhorse of N-body simulation. Uses the average of accelerations at the start and end of each timestep, making it time-reversible and symplectic.

```
x(t+dt) = x(t) + v(t) * dt + 0.5 * a(t) * dt²
a(t+dt) = compute_acceleration(x(t+dt))
v(t+dt) = v(t) + 0.5 * (a(t) + a(t+dt)) * dt
```

| Aspect | Rating |
|--------|--------|
| Speed | Fast |
| Accuracy | Good |
| Energy Conservation | Excellent |
| Force Evaluations | 2 per step |

**Pros:**
- Excellent energy conservation
- Time-reversible (important for physical accuracy)
- Good balance of speed and accuracy
- Second-order convergence

**Cons:**
- Requires two force evaluations per step
- Not as accurate as RK4 for smooth problems

**Best for**: Most N-body simulations. This is the default and recommended choice.

---

### PEFRL (4th Order)

**Config name**: `"pefrl"` | **Alias**: `"forest_ruth"`

Position Extended Forest-Ruth Like integrator. A fourth-order symplectic method with optimized coefficients that minimize error.

| Aspect | Rating |
|--------|--------|
| Speed | Slow |
| Accuracy | Very High |
| Energy Conservation | Superior |
| Force Evaluations | 4 per step |

**Pros:**
- Superior long-term energy conservation
- Fourth-order accuracy
- Symplectic
- Optimized error coefficients

**Cons:**
- Most expensive (4 force evaluations per step)
- Overkill for short simulations

**Best for**: Scientific simulations, long-term orbital mechanics, situations where energy conservation is paramount.

---

### Explicit Euler (1st Order)

**Config name**: `"explicit_euler"` | **Alias**: `"forward_euler"`

The simplest possible integrator. Updates position before velocity using only current state values.

```
x(t+dt) = x(t) + v(t) * dt
v(t+dt) = v(t) + a(t) * dt
```

| Aspect | Rating |
|--------|--------|
| Speed | Fastest |
| Accuracy | Very Low |
| Energy Conservation | Very Poor |
| Force Evaluations | 1 per step |

**WARNING**: Energy grows or decays exponentially. Unsuitable for orbital mechanics or any simulation you want to run for more than a few seconds.

**Pros:**
- Simplest possible implementation
- Fastest computation

**Cons:**
- Severe energy drift
- Unstable for oscillatory systems
- Orbits will spiral in or out

**Best for**: Educational purposes, comparing against better methods, debugging. **Not for actual simulations.**

---

### Heun's Method (2nd Order)

**Config name**: `"heun"` | **Alias**: `"improved_euler"`

A predictor-corrector method that averages derivatives at the start and predicted endpoint.

```
x_predicted = x(t) + v(t) * dt
v_predicted = v(t) + a(t) * dt
a_predicted = compute_acceleration(x_predicted)
x(t+dt) = x(t) + 0.5 * (v(t) + v_predicted) * dt
v(t+dt) = v(t) + 0.5 * (a(t) + a_predicted) * dt
```

| Aspect | Rating |
|--------|--------|
| Speed | Fast |
| Accuracy | Medium |
| Energy Conservation | Poor |
| Force Evaluations | 2 per step |

**Pros:**
- Better accuracy than explicit Euler
- Simple predictor-corrector approach

**Cons:**
- Energy drift in long simulations
- Not symplectic

**Best for**: Short-term simulations with smooth forces, non-Hamiltonian systems.

---

### RK2 Midpoint (2nd Order)

**Config name**: `"runge_kutta_second_order_midpoint"` | **Aliases**: `"rk2"`, `"midpoint"`

Second-order Runge-Kutta using the midpoint rule. Evaluates the derivative at the middle of the timestep.

| Aspect | Rating |
|--------|--------|
| Speed | Fast |
| Accuracy | Medium |
| Energy Conservation | Poor |
| Force Evaluations | 2 per step |

**Pros:**
- Good accuracy for smooth problems
- Classic, well-understood method

**Cons:**
- Energy drift
- Not suitable for long-term simulations

**Best for**: Non-Hamiltonian systems, short integration periods, educational use.

---

### RK4 (4th Order)

**Config name**: `"runge_kutta_fourth_order"` | **Alias**: `"rk4"`

The classic fourth-order Runge-Kutta method. Uses a weighted average of four derivative evaluations.

| Aspect | Rating |
|--------|--------|
| Speed | Slow |
| Accuracy | Very High |
| Energy Conservation | Poor |
| Force Evaluations | 4 per step |

**Pros:**
- High accuracy per step
- Well-understood error behavior
- Good for smooth functions

**Cons:**
- Energy drift over long simulations
- Expensive (4 evaluations per step)
- Not symplectic

**Best for**: High-accuracy requirements over short timeframes, non-Hamiltonian systems, benchmarking.

## Comparison Table

| Integrator | Order | Force Evals | Energy Conservation | Relative Speed |
|------------|-------|-------------|---------------------|----------------|
| `explicit_euler` | 1 | 1 | Very Poor | Fastest |
| `symplectic_euler` | 1 | 1 | Good | Fastest |
| `heun` | 2 | 2 | Poor | Fast |
| `rk2` | 2 | 2 | Poor | Fast |
| `velocity_verlet` | 2 | 2 | Excellent | Fast |
| `rk4` | 4 | 4 | Poor | Slow |
| `pefrl` | 4 | 4 | Superior | Slow |

## Choosing Guidelines

### 1. For Most Users

Use **`velocity_verlet`** (the default). It provides an excellent balance of speed, accuracy, and energy conservation.

### 2. For Long-Term Stability

Use **`pefrl`** if you're running simulations for extended periods and need the energy to stay bounded. The extra computational cost is worth it for simulations lasting thousands of timesteps.

### 3. For Real-Time Performance

Use **`symplectic_euler`** with a smaller timestep. It's the fastest option that still maintains energy conservation.

### 4. When Energy Conservation Matters

**Always choose a symplectic integrator** (`symplectic_euler`, `velocity_verlet`, or `pefrl`). The explicit methods (Euler, Heun, RK2, RK4) will show energy drift that compounds over time.

### 5. For Educational Purposes

Try **`explicit_euler`** to see how badly things can go wrong, then compare with **`velocity_verlet`** to appreciate the importance of symplectic methods.

## Configuration

Set the integrator in your config file:

```toml
[physics.integrator]
type = "velocity_verlet"
```

Or via command line:

```bash
stardrift --integrator pefrl
```

List all available integrators:

```bash
stardrift --list-integrators
```

## Further Reading

- [Leapfrog integration](https://en.wikipedia.org/wiki/Leapfrog_integration) - Wikipedia article on Verlet methods
- [Symplectic integrator](https://en.wikipedia.org/wiki/Symplectic_integrator) - Why symplectic methods matter
- [N-body simulation](https://en.wikipedia.org/wiki/N-body_simulation) - Overview of the problem domain

## See Also

- [Configuration Reference](configuration.md) - Full configuration options
- [Architecture](architecture.md) - How the physics engine is implemented
