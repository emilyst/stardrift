//! Position-Extended Forest-Ruth-Like (PEFRL) integration method
//!
//! A state-of-the-art 4th-order symplectic integrator optimized for long-term
//! stability in Hamiltonian systems. PEFRL combines high accuracy with exact
//! preservation of phase space structure, making it ideal for orbital mechanics
//! and N-body simulations where energy conservation over millions of timesteps
//! is critical.

use super::{Integrator, StageQuery, StepState};
use crate::physics::math::{Scalar, Vector};

/// PEFRL integrator - a 4th order symplectic integrator
///
/// This is a fourth-order symplectic integrator that achieves exceptional long-term
/// stability through a carefully optimized 9-stage symmetric composition. Unlike
/// traditional Runge-Kutta methods, PEFRL preserves the geometric structure of
/// phase space, preventing the artificial energy drift that plagues non-symplectic
/// integrators in conservative systems.
///
/// # Algorithm Structure
///
/// PEFRL uses a palindromic (time-reversible) sequence of position and velocity updates
/// with specially optimized coefficients that minimize the leading error term:
///
/// ```text
/// Stage 1: x += ξ * v * dt                    (ξ = 0.1786178958448091)
/// Stage 2: v += (1-2λ)/2 * a(x) * dt          (λ = -0.2123418310626054)
/// Stage 3: x += χ * v * dt                    (χ = -0.0662645826698185)
/// Stage 4: v += λ * a(x) * dt
/// Stage 5: x += (1-2(χ+ξ)) * v * dt           (middle stage)
/// Stage 6: v += λ * a(x) * dt
/// Stage 7: x += χ * v * dt
/// Stage 8: v += (1-2λ)/2 * a(x) * dt
/// Stage 9: x += ξ * v * dt
/// ```
///
/// The symmetric structure (reading identically forward and backward) ensures
/// time-reversibility and causes odd-order error terms to cancel, achieving
/// 4th-order accuracy with only 4 force evaluations.
///
/// # Symplectic Properties
///
/// **Phase Space Preservation**: Each stage is a shear transformation with
/// Jacobian determinant = 1. The composition preserves the symplectic 2-form
/// ω = dp ∧ dq, ensuring:
/// - Exact conservation of phase space volume (Liouville's theorem)
/// - Bounded energy oscillations without secular drift
/// - Preservation of Poincaré invariants
/// - Long-term stability of orbital parameters
///
/// **Modified Hamiltonian**: PEFRL exactly conserves a modified Hamiltonian
/// H̃ = H + O(dt⁴), where the modification is small and bounded. This contrasts
/// with non-symplectic methods where energy error grows unboundedly.
///
/// # Mathematical Properties
///
/// - **Order of accuracy**: O(dt⁴) local truncation error
/// - **Force evaluations**: 4 per timestep (stages 2, 4, 6, 8)
/// - **Symplectic**: Yes - preserves phase space volume exactly
/// - **Time-reversible**: Yes - palindromic structure ensures reversibility
/// - **Stability**: Excellent for Hamiltonian systems
///
/// # Coefficient Optimization
///
/// The coefficients (ξ, λ, χ) were determined through numerical optimization
/// to minimize the coefficient of the dt⁵ error term while maintaining the
/// symplectic constraint that position and velocity coefficients each sum to 1.
/// These specific values provide approximately 100× better accuracy than
/// unoptimized symmetric compositions of the same order.
///
/// # Energy Behavior
///
/// - **Energy conservation**: Bounded oscillations without secular drift
/// - **Modified Hamiltonian**: Conserves H̃ = H + O(dt⁴) exactly
/// - **Long-term stability**: Suitable for millions of timesteps
/// - **Phase space**: All Poincaré invariants preserved
///
/// **In the simulation these are theta = 0 statements.** At theta > 0 the
/// Barnes-Hut field is not the gradient of any potential (the acceptance
/// criterion is not symmetric between a pair of bodies), so exact
/// symplecticity and the modified-Hamiltonian guarantee do not apply; the
/// long-term drift is then set by the Barnes-Hut error, not this method.
/// The palindromic structure remains exactly time-reversible at any theta.
/// See docs/integration.md.
///
/// # Computational Cost
///
/// - **Force evaluations**: 4 per timestep
/// - **Cost comparison**: ~2× more expensive than Velocity Verlet per step
/// - **Efficiency gain**: Often allows larger timesteps than lower-order methods
/// - **Memory usage**: Minimal - only current state stored
/// - **Parallelization**: Limited - stages must be computed sequentially
///
/// # Comparison with Other Methods
///
/// | Integrator      | Order | Symplectic | Force Evals | Energy Behavior            |
/// |-----------------|-------|------------|-------------|----------------------------|
/// | PEFRL           | 4     | Yes        | 4           | Bounded oscillation        |
/// | RK4             | 4     | No         | 4           | Secular drift              |
/// | Velocity Verlet | 2     | Yes        | 2           | Bounded oscillation        |
/// | Yoshida4        | 4     | Yes        | 3           | Bounded (larger amplitude) |
///
/// # Use Cases
///
/// **Ideal for:**
/// - Solar system dynamics and asteroid trajectory calculations
/// - Long-term stability studies in celestial mechanics
/// - Molecular dynamics with conservative forces
/// - Symplectic integration benchmarks
/// - Any Hamiltonian system requiring both high accuracy and conservation
///
/// **Consider alternatives:**
/// - Use Velocity Verlet for most applications (simpler, often sufficient)
/// - Use RK4 for short simulations where energy drift is acceptable
/// - Use higher-order composition methods if even better accuracy needed
///
/// # Implementation Notes
///
/// The implementation precomputes the derived coefficients (1-2λ)/2 and 1-2(χ+ξ)
/// for efficiency. Both COEFF_A and COEFF_B are computed as constants at compile
/// time, eliminating redundant arithmetic during integration.
///
/// # Historical Note
///
/// PEFRL represents the culmination of decades of research into symplectic
/// integration. The Forest-Ruth method (1990) pioneered 4th-order symplectic
/// integration, while this optimized variant (2002) achieves superior accuracy
/// through coefficient optimization.
///
/// # Reference
///
/// Omelyan, Mryglod, Folk (2002) "Optimized Forest-Ruth- and Suzuki-like algorithms
/// for integration of motion in many-body systems", Computer Physics Communications
/// 146(2), 188-202. DOI: 10.1016/S0010-4655(02)00451-4
#[derive(Debug, Copy, Clone, Default)]
pub struct Pefrl;

impl Pefrl {
    const XI: Scalar = 0.178_617_895_844_809_1;
    const LAMBDA: Scalar = -0.212_341_831_062_605_4;
    const CHI: Scalar = -0.066_264_582_669_818_5;
    const COEFF_A: Scalar = 0.5 * (1.0 - 2.0 * Pefrl::LAMBDA);
    const COEFF_B: Scalar = 1.0 - 2.0 * (Pefrl::CHI + Pefrl::XI);

    /// Drift coefficient preceding each of the four kicks. The trailing
    /// xi-drift that completes the palindrome is applied in `finish`.
    const DRIFT_COEFFS: [Scalar; 4] = [Pefrl::XI, Pefrl::CHI, Pefrl::COEFF_B, Pefrl::CHI];
    /// Kick coefficient for each of the four field evaluations.
    const KICK_COEFFS: [Scalar; 4] = [Pefrl::COEFF_A, Pefrl::LAMBDA, Pefrl::LAMBDA, Pefrl::COEFF_A];
}

impl Integrator for Pefrl {
    fn clone_box(&self) -> Box<dyn Integrator> {
        Box::new(*self)
    }

    fn next_query(&self, state: &StepState, _scratch: &[Vector], dt: Scalar) -> Option<StageQuery> {
        // Each kick is preceded by a drift; the query previews the drifted
        // position as a pure function of the working state, and apply_stage
        // commits the same drift before kicking.
        Self::DRIFT_COEFFS
            .get(state.stage)
            .map(|&drift| StageQuery {
                position: state.position + state.velocity * (drift * dt),
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
        // Commit the drift previewed by next_query, then kick.
        state.position += state.velocity * (Self::DRIFT_COEFFS[state.stage] * dt);
        state.velocity += accel * (Self::KICK_COEFFS[state.stage] * dt);
        state.stage += 1;
    }

    fn finish(&self, state: &StepState, _scratch: &[Vector], dt: Scalar) -> (Vector, Vector) {
        // Trailing xi-drift with the POST-final-kick velocity completes the
        // palindrome; a pure read here would silently drop the last drift.
        (
            state.position + state.velocity * (Pefrl::XI * dt),
            state.velocity,
        )
    }

    fn convergence_order(&self) -> usize {
        4
    }

    fn name(&self) -> &'static str {
        "pefrl"
    }

    fn aliases(&self) -> Vec<&'static str> {
        vec!["forest_ruth"]
    }
}
