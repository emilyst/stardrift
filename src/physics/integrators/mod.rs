//! Numerical integration methods for n-body simulation
//!
//! This module provides various numerical integrators for solving the equations
//! of motion in gravitational n-body simulations. Each integrator implements
//! the `Integrator` trait, which requires self-description of its properties
//! (name, aliases, convergence order) and the core integration step.

use crate::physics::math::{Scalar, Vector};

pub mod explicit_euler;
pub mod heun;
pub mod pefrl;
pub mod registry;
pub mod runge_kutta;
pub mod symplectic_euler;
pub mod velocity_verlet;

pub use explicit_euler::ExplicitEuler;
pub use heun::Heun;
pub use pefrl::Pefrl;
pub use runge_kutta::RungeKuttaFourthOrder;
pub use runge_kutta::RungeKuttaSecondOrderMidpoint;
pub use symplectic_euler::SymplecticEuler;
pub use velocity_verlet::VelocityVerlet;

/// Acceleration field trait for calculating accelerations at arbitrary positions
///
/// This trait allows integrators to evaluate accelerations at intermediate positions
/// during multi-stage integration methods (e.g., RK4, Velocity Verlet).
/// The field is passed to integrators to enable accurate acceleration calculations
/// without rebuilding the octree at each intermediate step.
pub trait AccelerationField: Send + Sync {
    /// Calculate acceleration at a given position
    ///
    /// # Arguments
    /// * `position` - The position at which to evaluate the acceleration
    ///
    /// # Returns
    /// The acceleration vector at the given position
    fn at(&self, position: Vector) -> Vector;
}

/// A single field-evaluation request issued by an integrator mid-step.
///
/// The query carries the velocity at the same stage so that a future
/// velocity-dependent field (drag, radiation pressure) can be supported
/// without another protocol change; a purely positional field ignores it.
#[derive(Debug, Copy, Clone)]
pub struct StageQuery {
    /// Position at which the field must be evaluated
    pub position: Vector,
    /// Velocity associated with the same stage
    pub velocity: Vector,
}

/// Per-body state threaded through one step of the staged protocol.
///
/// The initial state is retained separately from the working state so that a
/// future adaptive method can reject a step and retry from `initial_*` at a
/// smaller `dt` without re-reading the ECS.
#[derive(Debug, Copy, Clone)]
pub struct StepState {
    /// Position at the start of the step
    pub initial_position: Vector,
    /// Velocity at the start of the step
    pub initial_velocity: Vector,
    /// Working position; meaning between stages is integrator-defined
    pub position: Vector,
    /// Working velocity; meaning between stages is integrator-defined
    pub velocity: Vector,
    /// Number of stage accelerations applied so far
    pub stage: usize,
}

impl StepState {
    pub fn new(position: Vector, velocity: Vector) -> Self {
        Self {
            initial_position: position,
            initial_velocity: velocity,
            position,
            velocity,
            stage: 0,
        }
    }
}

/// Upper bound on [`Integrator::scratch_len`] supported by the provided
/// [`Integrator::step`] convenience method (which uses a stack buffer).
pub const MAX_SCRATCH_LEN: usize = 8;

/// Base trait for all integrators with capability discovery
///
/// Integrators are self-describing, providing their name, aliases, and
/// mathematical properties. This enables the registry to discover and
/// manage integrators without hardcoded knowledge.
///
/// # Staged protocol
///
/// Integration is expressed as a query/apply conversation so that a driver
/// advancing many mutually interacting bodies can keep them synchronized:
///
/// 1. [`Integrator::next_query`] returns the next field evaluation the method
///    needs (or `None` when the step is complete),
/// 2. the caller evaluates the acceleration — for an N-body driver, against a
///    field rebuilt from *all* bodies' current stage queries,
/// 3. [`Integrator::apply_stage`] folds the result into the state,
/// 4. when `next_query` returns `None`, [`Integrator::finish`] produces the
///    committed end-of-step state.
///
/// Drivers must not assume stage indices advance monotonically or that the
/// number of queries is fixed up front: a future implicit method may issue
/// repeated queries for the same stage while iterating to convergence, and an
/// adaptive method may restart from the initial state. Loop on `next_query`
/// until it returns `None`.
///
/// For a single body against an analytic field the provided [`Integrator::step`]
/// drives the same protocol, so unit-level behavior and N-body behavior come
/// from one implementation.
pub trait Integrator: Send + Sync {
    /// Create a boxed clone of this integrator
    ///
    /// This enables the registry to create new instances without knowing
    /// the concrete type, supporting true generic discovery.
    fn clone_box(&self) -> Box<dyn Integrator>;

    /// Number of acceleration scratch slots one step of this method may
    /// record via `scratch` (0 for methods that fold each acceleration into
    /// the working state immediately). Must not exceed [`MAX_SCRATCH_LEN`].
    fn scratch_len(&self) -> usize {
        0
    }

    /// The next field evaluation this method needs, or `None` when the step
    /// is complete and [`Integrator::finish`] may be called.
    ///
    /// Must be a pure function of the state: it is called before the field
    /// exists for that stage and must not mutate anything.
    fn next_query(&self, state: &StepState, scratch: &[Vector], dt: Scalar) -> Option<StageQuery>;

    /// Fold the acceleration evaluated at the most recent query into the
    /// state, advancing `state.stage`.
    fn apply_stage(&self, state: &mut StepState, scratch: &mut [Vector], accel: Vector, dt: Scalar);

    /// Produce the committed end-of-step `(position, velocity)`.
    ///
    /// Called once `next_query` returns `None`. Note this is not necessarily
    /// a pure read of the working state: PEFRL applies its trailing drift
    /// here.
    fn finish(&self, state: &StepState, scratch: &[Vector], dt: Scalar) -> (Vector, Vector);

    /// Whether this method's final field evaluation is at the committed
    /// end-of-step position, making it reusable as the *next* step's first
    /// evaluation (FSAL — "first same as last"). A driver may then skip one
    /// field rebuild per step. Velocity Verlet is the canonical example.
    fn reuses_final_stage(&self) -> bool {
        false
    }

    /// Advance a single body's state by one time step using an acceleration field
    ///
    /// Provided method: drives the staged protocol against the given field,
    /// evaluating each query immediately. For an analytic field this is the
    /// exact single-body reduction of the synchronized N-body scheme.
    ///
    /// # Arguments
    /// * `position` - Mutable reference to position
    /// * `velocity` - Mutable reference to velocity vector
    /// * `field` - Acceleration field for calculating acceleration at arbitrary positions
    /// * `dt` - Time step
    fn step(
        &self,
        position: &mut Vector,
        velocity: &mut Vector,
        field: &dyn AccelerationField,
        dt: Scalar,
    ) {
        debug_assert!(self.scratch_len() <= MAX_SCRATCH_LEN);
        let mut scratch = [Vector::ZERO; MAX_SCRATCH_LEN];
        let scratch = &mut scratch[..self.scratch_len()];
        let mut state = StepState::new(*position, *velocity);
        while let Some(query) = self.next_query(&state, scratch, dt) {
            let accel = field.at(query.position);
            self.apply_stage(&mut state, scratch, accel, dt);
        }
        let (new_position, new_velocity) = self.finish(&state, scratch, dt);
        *position = new_position;
        *velocity = new_velocity;
    }

    /// Returns the convergence order of this integration method
    ///
    /// The convergence order indicates how the error scales with timestep:
    /// - Order 1: Error ~ O(dt)
    /// - Order 2: Error ~ O(dt²)
    /// - Order 4: Error ~ O(dt⁴)
    fn convergence_order(&self) -> usize;

    /// Returns the canonical name of this integrator
    ///
    /// This is the primary identifier used in configuration files
    fn name(&self) -> &'static str;

    /// Returns alternative names/aliases for this integrator
    ///
    /// These provide convenient shortcuts for users
    fn aliases(&self) -> Vec<&'static str> {
        Vec::new() // Default: no aliases
    }
}
