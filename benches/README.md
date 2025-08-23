# Stardrift Benchmarks

## Overview

This directory contains comprehensive benchmarks for the Stardrift N-body simulation. The benchmarks measure both
performance and quality characteristics of various components.

## Benchmark Suites

### 1. Integrator Benchmarks (`integrators.rs`)

Comprehensive testing of numerical integrators across multiple dimensions:

#### Performance Group

- **Raw throughput**: How fast each integrator completes a single step
- Tested integrators: Symplectic Euler, Velocity Verlet, Heun, RK2 Midpoint, RK4

#### Accuracy Group

- **Harmonic oscillator accuracy**: Error vs analytical solution over one period
- **Convergence order**: Verifies each integrator achieves its theoretical order of accuracy
    - Symplectic Euler: 1st order
    - Velocity Verlet: 2nd order
    - Heun: 2nd order
    - RK2 Midpoint: 2nd order
    - RK4: 4th order

#### Stability Group

- **Energy conservation**: Energy drift over 10,000 steps
- **Kepler orbits**: Conservation of energy and angular momentum in orbital mechanics
- Expected results:
    - Velocity Verlet: Excellent energy conservation (<0.1% drift)
    - Symplectic Euler: Good energy conservation (<5% drift)
    - RK4: Very low energy drift (<0.01%)
    - Heun/RK2: May show significant drift (non-symplectic)

#### Work-Precision Group

- **Accuracy vs computation cost**: Tests different timesteps to find optimal accuracy/speed tradeoff
- Helps determine the best integrator for specific accuracy requirements

#### Realistic N-Body Group

- **Real octree forces**: Tests integrators with actual N-body gravitational forces
- Uses 100-body cluster with Barnes-Hut octree acceleration

### 2. Octree Benchmarks (`octree.rs`)

Comprehensive Barnes-Hut octree testing:

#### Construction Group

- **Scaling**: Verifies O(n log n) construction time
- **Memory efficiency**: Tests different leaf thresholds

#### Physics Group

- **Force calculation scaling**: Should be O(log n) per body
- **Theta accuracy tradeoff**: Accuracy vs performance with different theta values

#### Real-World Group

- **60 FPS target**: Tests ability to maintain 60 FPS with various body counts

#### Characteristics Group

- **Stats overhead**: Measures the cost of collecting octree statistics

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark suite
cargo bench --bench integrators
cargo bench --bench octree

# Run specific benchmark group
cargo bench --bench integrators -- performance
cargo bench --bench integrators -- accuracy
cargo bench --bench integrators -- stability

# Generate HTML report (requires gnuplot)
cargo bench --bench integrators -- --save-baseline my_baseline
```

## Interpreting Results

### Choosing an Integrator

Based on benchmark results, choose integrators as follows:

1. **For interactive/gaming (60+ FPS)**:
    - Symplectic Euler: Fastest, decent stability
    - Velocity Verlet: Good balance of speed and conservation

2. **For long-term simulations**:
    - Velocity Verlet: Best energy conservation
    - RK4: Best overall accuracy (but slower)

3. **For high accuracy requirements**:
    - RK4: 4th order accuracy, best for precise trajectories
    - RK2/Heun: Good middle ground

4. **For educational/visualization**:
    - Any integrator with appropriate timestep
    - Consider accuracy vs visual smoothness tradeoff

### Performance Targets

- **60 FPS**: Complete physics cycle in <16.67ms
- **30 FPS**: Complete physics cycle in <33.33ms
- **Interactive**: User-perceivable lag starts at ~100ms

### Typical Results

With ~100 bodies on modern hardware:

- Symplectic Euler: ~0.5ms per frame
- Velocity Verlet: ~0.8ms per frame
- Heun/RK2: ~1.0ms per frame
- RK4: ~2.0ms per frame

## Benchmark Development

To add new benchmarks:

1. Create a new function following the naming pattern `bench_*`
2. Use `criterion::Criterion` for the benchmark harness
3. Group related benchmarks using `benchmark_group`
4. Add to appropriate `criterion_group!` macro
5. Update this README with description

## Dependencies

- `criterion`: Benchmark harness with statistical analysis
- `gnuplot` (optional): For generating comparison plots