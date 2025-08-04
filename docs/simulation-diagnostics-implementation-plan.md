# Simulation Diagnostics Implementation Plan

**Created**: 2025-08-04  
**Last Updated**: 2025-08-04  
**Note**: This document was generated automatically with AI assistance.

## Overview

This document outlines a comprehensive plan for implementing physics and performance diagnostics in the Stardrift N-body
gravity simulation. The implementation will extend the existing `SimulationDiagnosticsPlugin` to provide real-time
monitoring of simulation accuracy and performance.

## Goals

1. **Physics Accuracy Monitoring**
    - Track conservation laws (energy, momentum, angular momentum)
    - Monitor numerical stability and error accumulation
    - Enable comparison between different symplectic integrators
    - Measure deviation from true symplectic behavior

2. **Performance Profiling**
    - Measure octree construction time
    - Track force calculation duration
    - Monitor overall physics step timing
    - Analyze memory usage patterns

3. **System Behavior Analysis**
    - Track statistical properties (velocity distributions, virial ratio)
    - Monitor collision statistics and close encounters
    - Analyze chaos indicators and predictability

4. **Integration Requirements**
    - Minimal performance overhead
    - Clean integration with existing systems
    - Extensible for future diagnostics
    - User-friendly accuracy indicators

## Diagnostic Metrics (Priority Order)

### Priority 1: Core Conservation Laws

#### 1.1 Hamiltonian (Total Energy)

**Priority: CRITICAL** - Fundamental test of integrator quality

The Hamiltonian H = T + V represents total system energy:

- **Kinetic Energy (T)**: ½Σ(m·v²) for all bodies
- **Potential Energy (V)**: -½Σ(G·m₁·m₂/r) for all pairs

**Implementation Strategy:**

- Calculate kinetic energy by iterating over all bodies
- Accumulate potential energy during octree force calculations
- Use atomic operations for thread-safe accumulation in parallel systems
- Track both instantaneous value and drift rate (dH/dt)

#### 1.2 Angular Momentum Conservation

**Priority: CRITICAL** - More sensitive than energy to asymmetric errors

Total angular momentum L = Σ(r × mv) should be exactly conserved.

**Implementation Strategy:**

- Calculate L = Σ(position × mass × velocity) for all bodies
- Track vector components (Lx, Ly, Lz) separately
- Monitor magnitude |L| and direction changes
- Compute in barycentric frame to avoid drift artifacts

**Implementation Notes:**

```rust
// Use DVec3 for precision
let angular_momentum: DVec3 = bodies
.iter()
.map( | (transform, velocity, mass)| {
let r = transform.translation.as_dvec3() - barycenter;
let v = velocity.0;
r.cross(v) * mass.value()
})
.sum();
```

### Priority 2: Integration Quality Metrics

#### 2.1 Phase Space Volume (Symplecticity)

**Priority: HIGH** - Direct measure of symplectic property preservation

Symplectic integrators preserve phase space volume (Liouville's theorem).

**Implementation Strategy:**

- Track a small ensemble of test particles around each body
- Compute Jacobian determinant of phase space mapping
- Monitor volume preservation over time
- Use as integrator quality metric

**Implementation Notes:**

- Start with 6-8 test particles in small sphere around each body
- Track their phase space volume evolution
- Expensive computation - sample every N timesteps

#### 2.2 Virial Ratio

**Priority: HIGH** - System equilibrium indicator

For gravitationally bound systems: 2K/|V| ≈ 1.0 at equilibrium

**Implementation Strategy:**

- Calculate from existing kinetic and potential energy
- Track time-averaged value
- Monitor oscillations around equilibrium
- Detect numerical heating/cooling

### Priority 3: Performance Metrics

#### 3.1 Octree Build Time

**Priority: HIGH** - Key performance bottleneck

**Implementation Strategy:**

- Start timer before `rebuild_octree` system
- End timer after completion
- Track both average and worst-case times
- Correlate with body count

#### 3.2 Force Calculation Time

**Priority: HIGH** - Main computational cost

**Implementation Strategy:**

- Time the `apply_gravitation_octree` system
- Include parallel computation overhead
- Track time per body
- Monitor scaling with N

#### 3.3 Physics Step Timing

**Priority: MEDIUM** - Overall performance

**Implementation Strategy:**

- Measure entire FixedUpdate schedule duration
- Break down by major phases
- Identify bottlenecks

### Priority 4: Collision and Stability Metrics

#### 4.1 Collision/Near-Miss Statistics

**Priority: HIGH** - Critical for future collision implementation

Track close encounters between bodies:

- Minimum approach distance per timestep
- Number of encounters below threshold distances
- Two-body encounter velocity statistics

**Implementation Strategy:**

- During force calculation, track minimum distances
- Maintain histogram of approach distances
- Flag potential collisions for future implementation
- Use for adaptive timestep recommendations

**Implementation Notes:**

```rust
// Track during octree traversal
if distance < 5.0 * (radius1 + radius2) {
metrics.record_close_encounter(distance, relative_velocity);
}
```

#### 4.2 Integration Error Metrics

**Priority: MEDIUM** - Accuracy assessment

**Implementation Strategy:**

- Backward error analysis: integrate forward then backward
- Local truncation error estimates
- Energy drift rate (dH/dt) tracking
- Comparative metrics between integrators

### Priority 5: Statistical Analysis

#### 5.1 Velocity Distribution

**Priority: MEDIUM** - System thermodynamics

Track velocity distribution statistics:

- Mean, variance, skewness
- Comparison to Maxwell-Boltzmann
- Evolution over time

**Implementation Strategy:**

- Compute velocity magnitude distribution
- Track percentiles (10%, 50%, 90%)
- Monitor distribution shape evolution

#### 5.2 Spatial Distribution Metrics

**Priority: MEDIUM** - System structure

**Implementation Strategy:**

- Lagrangian radii (radii containing 10%, 50%, 90% of mass)
- Core/halo density ratio
- Clustering coefficients

### Priority 6: Advanced Diagnostics

#### 6.1 Chaos Indicators

**Priority: LOW** - Advanced analysis

Lyapunov exponents measure sensitivity to initial conditions.

**Implementation Strategy:**

- Track separation of nearby trajectory pairs
- Compute largest Lyapunov exponent
- Indicate predictability timescale
- Expensive - optional diagnostic

#### 6.2 Memory Metrics

**Priority: LOW** - System monitoring

Track memory usage patterns:

- Octree node count
- Body count impact
- Memory allocation patterns

## Architecture Design

### Diagnostic Paths

```rust
// Priority 1: Core Conservation Laws
pub const HAMILTONIAN_TOTAL: DiagnosticPath =
    DiagnosticPath::const_new("simulation/hamiltonian/total");
pub const HAMILTONIAN_KINETIC: DiagnosticPath =
    DiagnosticPath::const_new("simulation/hamiltonian/kinetic");
pub const HAMILTONIAN_POTENTIAL: DiagnosticPath =
    DiagnosticPath::const_new("simulation/hamiltonian/potential");
pub const HAMILTONIAN_DRIFT_RATE: DiagnosticPath =
    DiagnosticPath::const_new("simulation/hamiltonian/drift_rate");

pub const ANGULAR_MOMENTUM_X: DiagnosticPath =
    DiagnosticPath::const_new("simulation/angular_momentum/x");
pub const ANGULAR_MOMENTUM_Y: DiagnosticPath =
    DiagnosticPath::const_new("simulation/angular_momentum/y");
pub const ANGULAR_MOMENTUM_Z: DiagnosticPath =
    DiagnosticPath::const_new("simulation/angular_momentum/z");
pub const ANGULAR_MOMENTUM_MAG: DiagnosticPath =
    DiagnosticPath::const_new("simulation/angular_momentum/magnitude");

// Priority 2: Integration Quality
pub const PHASE_SPACE_VOLUME: DiagnosticPath =
    DiagnosticPath::const_new("simulation/symplectic/phase_space_volume");
pub const VIRIAL_RATIO: DiagnosticPath =
    DiagnosticPath::const_new("simulation/virial_ratio");

// Priority 3: Performance
pub const OCTREE_BUILD_TIME: DiagnosticPath =
    DiagnosticPath::const_new("simulation/octree/build_time_ms");
pub const FORCE_CALC_TIME: DiagnosticPath =
    DiagnosticPath::const_new("simulation/physics/force_calc_time_ms");
pub const PHYSICS_STEP_TIME: DiagnosticPath =
    DiagnosticPath::const_new("simulation/physics/step_time_ms");

// Priority 4: Collision Statistics
pub const MIN_APPROACH_DISTANCE: DiagnosticPath =
    DiagnosticPath::const_new("simulation/collisions/min_distance");
pub const CLOSE_ENCOUNTER_COUNT: DiagnosticPath =
    DiagnosticPath::const_new("simulation/collisions/close_encounters");

// Priority 5: Statistical Measures
pub const VELOCITY_MEAN: DiagnosticPath =
    DiagnosticPath::const_new("simulation/statistics/velocity_mean");
pub const VELOCITY_VARIANCE: DiagnosticPath =
    DiagnosticPath::const_new("simulation/statistics/velocity_variance");
pub const LAGRANGIAN_RADIUS_50: DiagnosticPath =
    DiagnosticPath::const_new("simulation/statistics/lagrangian_r50");

// Priority 6: Advanced
pub const OCTREE_NODE_COUNT: DiagnosticPath =
    DiagnosticPath::const_new("simulation/octree/node_count");
pub const LYAPUNOV_EXPONENT: DiagnosticPath =
    DiagnosticPath::const_new("simulation/chaos/lyapunov_exponent");
```

### Resource Structure

```rust
#[derive(Resource, Default)]
pub struct SimulationMetrics {
    // Timing
    pub octree_build_start: Option<Instant>,
    pub force_calc_start: Option<Instant>,
    pub physics_step_start: Option<Instant>,

    // Energy tracking
    pub last_potential_energy: Scalar,
    pub last_kinetic_energy: Scalar,
    pub initial_hamiltonian: Option<Scalar>,

    // Angular momentum tracking
    pub last_angular_momentum: DVec3,
    pub initial_angular_momentum: Option<DVec3>,

    // Collision tracking
    pub min_approach_distance: Scalar,
    pub close_encounter_pairs: Vec<(Entity, Entity, Scalar)>,

    // Statistical tracking
    pub velocity_samples: Vec<Scalar>,

    // Phase space tracking (for symplecticity)
    pub test_particles: HashMap<Entity, Vec<PhaseSpacePoint>>,
}

#[derive(Clone, Copy)]
pub struct PhaseSpacePoint {
    pub position: DVec3,
    pub velocity: DVec3,
}
```

### System Organization

1. **Timing Systems**: Wrap existing physics systems with timing measurements
2. **Calculation Systems**: Dedicated systems for complex calculations (Hamiltonian)
3. **Collection Systems**: Gather and update diagnostic stores

## Implementation Phases

### Phase 1: Core Infrastructure & Conservation Laws (Session 1)

**Priority 1 Diagnostics**

1. Define all diagnostic paths (complete list)
2. Create expanded `SimulationMetrics` resource
3. Implement Hamiltonian calculation:
    - Kinetic energy system
    - Modify octree for potential energy accumulation
    - Energy drift rate tracking
4. Implement angular momentum calculation:
    - Vector components tracking
    - Barycentric frame computation
    - Conservation monitoring

### Phase 2: Integration Quality Metrics (Session 2)

**Priority 2 Diagnostics**

1. Implement virial ratio calculation
2. Design phase space volume tracking:
    - Test particle system
    - Jacobian computation framework
    - Volume preservation metrics
3. Add symplecticity diagnostics
4. Create integrator comparison framework

### Phase 3: Performance & Collision Metrics (Session 3)

**Priority 3 & 4 Diagnostics**

1. Implement timing systems:
    - Octree build timing
    - Force calculation timing
    - Physics step duration
2. Add collision statistics:
    - Close encounter detection
    - Minimum distance tracking
    - Encounter histogram
3. Create adaptive timestep recommendations

### Phase 4: Statistical Analysis (Session 4)

**Priority 5 Diagnostics**

1. Implement velocity distribution analysis
2. Add spatial distribution metrics:
    - Lagrangian radii
    - Density profiles
3. Create statistical evolution tracking
4. Add thermodynamic indicators

### Phase 5: Visualization & Advanced Features (Session 5)

**Priority 6 & Integration**

1. Extend diagnostics HUD for all metrics
2. Create diagnostic dashboard:
    - Real-time conservation law monitoring
    - Performance profiling views
    - Statistical distribution plots
3. Implement chaos indicators (optional)
4. Add export functionality for analysis

### Phase 6: Testing and Documentation (Session 6)

1. Benchmark diagnostic overhead
2. Verify conservation law accuracy
3. Create diagnostic usage guide
4. Add example configurations for different use cases

## Technical Considerations

### Potential Energy Calculation

The naive O(N²) approach for potential energy is prohibitive. Instead:

1. **Integrate with Force Calculation**: Accumulate potential energy during existing octree traversal
2. **Use Barnes-Hut Approximation**: Same multipole approximations used for forces
3. **Atomic Accumulation**: Thread-safe accumulation for parallel calculations

### Performance Impact

Target: <1% overhead for diagnostics

- Use conditional compilation for expensive diagnostics
- Implement sampling for high-frequency measurements
- Cache calculations where possible

### Future Extensions

The architecture should support:

- Angular momentum tracking
- Temperature/velocity distribution analysis
- Collision event monitoring
- Orbital element analysis
- Phase space visualization

## Testing Strategy

1. **Unit Tests**
    - Energy calculation accuracy
    - Timing measurement precision
    - Thread safety of accumulators

2. **Integration Tests**
    - Verify energy conservation with known systems
    - Compare with analytical solutions
    - Stress test with large body counts

3. **Benchmarks**
    - Measure diagnostic overhead
    - Profile memory usage
    - Analyze scaling behavior

## Success Criteria

1. **Conservation Laws**
    - Hamiltonian conservation within 0.01% over 1000 timesteps
    - Angular momentum conservation within machine precision
    - Clear drift rate indicators for integrator comparison

2. **Performance Impact**
    - Total diagnostic overhead < 1% of frame time
    - Negligible impact on simulation scaling
    - Efficient parallel computation preservation

3. **Usability**
    - All metrics update at configurable intervals
    - Clear accuracy indicators for non-experts
    - Real-time feedback on simulation quality

4. **Architecture**
    - Clean integration without modifying core physics
    - Extensible for future diagnostics
    - Plugin remains self-contained

5. **Scientific Validity**
    - Accurate potential energy calculation with Barnes-Hut
    - Proper barycentric frame calculations
    - Meaningful statistical measures

## References

- Bevy Diagnostics Documentation
- Barnes-Hut Algorithm (1986)
- Symplectic Integration Methods
- Energy Conservation in N-body Simulations

---

*This plan will be updated as implementation progresses. Each phase completion will include lessons learned and
adjustments for subsequent phases.*