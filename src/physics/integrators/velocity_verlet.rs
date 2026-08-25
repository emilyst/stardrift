//! Velocity Verlet integration method
//!
//! The gold standard for molecular dynamics and orbital mechanics, providing
//! an optimal balance between computational efficiency, accuracy, and energy
//! conservation. This second-order symplectic integrator is widely regarded
//! as the best general-purpose method for Hamiltonian systems.

use super::{Integrator, StageQuery, StepState};
use crate::physics::math::{Scalar, Vector};

/// Velocity Verlet integrator
///
/// A second-order symplectic integrator that provides excellent energy conservation
/// for Hamiltonian systems. Widely used in molecular dynamics and orbital mechanics,
/// it offers the best balance between computational cost and conservation properties
/// for most applications.
///
/// # Algorithm
///
/// The Velocity Verlet algorithm uses a clever splitting that maintains symplecticity:
///
/// ```text
/// Stage 1: Half-step velocity update
///   a(t) = F(x(t))/m
///   v(t+dt/2) = v(t) + a(t) * dt/2
///
/// Stage 2: Full-step position update
///   x(t+dt) = x(t) + v(t+dt/2) * dt
///
/// Stage 3: Complete velocity update
///   a(t+dt) = F(x(t+dt))/m
///   v(t+dt) = v(t+dt/2) + a(t+dt) * dt/2
/// ```
///
/// This implementation uses the kick-drift-kick (KDK) form directly, which
/// makes the palindromic K(dt/2) D(dt) K(dt/2) structure — and hence
/// time-reversibility — explicit in the code. The final kick's field
/// evaluation is at the committed end-of-step position, so it doubles as the
/// next step's first evaluation (FSAL): a driver that caches it performs only
/// one fresh field evaluation per step.
///
/// # Mathematical Properties
///
/// - **Order of accuracy**: O(dt²) local truncation error
/// - **Symplectic**: Preserves phase space volume exactly (det J = 1)
/// - **Time-reversible**: Forward and backward integration are symmetric
/// - **Force evaluations**: 2 per timestep (start and end of step)
/// - **Self-starting**: No previous values needed
///
/// # Energy Behavior
///
/// - **Energy conservation**: Excellent - bounded oscillations without drift
/// - **Modified Hamiltonian**: Conserves H̃ = H + O(dt²) exactly
/// - **Long-term stability**: No secular drift in energy or angular momentum
/// - **Symplectic structure**: Preserves all Poincaré invariants
///
/// The energy error remains bounded for exponentially long times, making
/// this ideal for long-duration simulations of conservative systems.
///
/// **In the simulation these are theta = 0 statements.** At theta > 0 the
/// Barnes-Hut acceptance is not symmetric between a pair of bodies, so the
/// approximate force field is not the gradient of any potential: the scheme
/// is not symplectic there and momentum is conserved only to the Barnes-Hut
/// error. What survives at any theta is exact time-reversibility of this
/// method's palindromic structure, which eliminates the integrator's own
/// contribution to secular energy drift; the residual is a property of the
/// Barnes-Hut approximation (reduce theta to reduce it). See
/// docs/integration.md.
///
/// # Computational Cost
///
/// This implementation performs 2 force evaluations per timestep:
/// - Twice the cost of Symplectic Euler, but with O(dt²) vs O(dt) accuracy
/// - 2× cheaper than RK4 or PEFRL
/// - In the simulation the FSAL property (see above) reduces the cost to one
///   fresh field evaluation and one octree build per step
///
/// # Comparison with Other Methods
///
/// | Property      | Velocity Verlet | RK4         | PEFRL       | Symplectic Euler |
/// |---------------|-----------------|-------------|-------------|------------------|
/// | Order         | 2               | 4           | 4           | 1                |
/// | Force evals   | 2               | 4           | 4           | 1                |
/// | Symplectic    | Yes             | No          | Yes         | Yes              |
/// | Energy drift  | Bounded         | Linear      | Bounded     | Bounded          |
/// | Best for      | General purpose | Short sims  | High accuracy| Simple problems |
///
/// # Use Cases
///
/// **Ideal for:**
/// - Molecular dynamics simulations
/// - Gravitational N-body problems
/// - Solar system and asteroid dynamics
/// - Any Hamiltonian system where energy conservation matters
/// - Real-time physics simulations (games, visualizations)
///
/// **Consider alternatives:**
/// - Use PEFRL when 4th-order accuracy is essential
/// - Use RK4 for non-conservative systems or short simulations
/// - Use Symplectic Euler for educational purposes or extreme simplicity
///
/// # Implementation Notes
///
/// The integration driver caches the final stage's accelerations between
/// steps (FSAL), so this method performs one fresh field evaluation per step
/// in the simulation. The standalone `step` path evaluates twice.
#[derive(Debug, Copy, Clone, Default)]
pub struct VelocityVerlet;

impl Integrator for VelocityVerlet {
    fn clone_box(&self) -> Box<dyn Integrator> {
        Box::new(*self)
    }

    fn next_query(
        &self,
        state: &StepState,
        _scratch: &[Vector],
        _dt: Scalar,
    ) -> Option<StageQuery> {
        // Stage 0 queries the start-of-step state; stage 1 queries the
        // drifted position with the half-kicked velocity.
        (state.stage < 2).then_some(StageQuery {
            position: state.position,
            velocity: state.velocity,
        })
    }

    fn apply_stage(
        &self,
        state: &mut StepState,
        _scratch: &mut [Vector],
        accel: Vector,
        dt: Scalar,
    ) {
        match state.stage {
            0 => {
                // Kick: v(t+dt/2) = v(t) + a(t)*dt/2
                state.velocity += accel * (0.5 * dt);
                // Drift: x(t+dt) = x(t) + v(t+dt/2)*dt
                state.position += state.velocity * dt;
            }
            _ => {
                // Kick: v(t+dt) = v(t+dt/2) + a(t+dt)*dt/2
                state.velocity += accel * (0.5 * dt);
            }
        }
        state.stage += 1;
    }

    fn finish(&self, state: &StepState, _scratch: &[Vector], _dt: Scalar) -> (Vector, Vector) {
        (state.position, state.velocity)
    }

    fn reuses_final_stage(&self) -> bool {
        // The final field evaluation is at the committed end-of-step
        // position, which is exactly the next step's stage-0 query (FSAL).
        true
    }

    fn convergence_order(&self) -> usize {
        2
    }

    fn name(&self) -> &'static str {
        "velocity_verlet"
    }

    fn aliases(&self) -> Vec<&'static str> {
        vec!["verlet"]
    }
}
