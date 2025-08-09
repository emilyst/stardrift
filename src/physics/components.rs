//! Physics components for n-body simulation

use crate::physics::math::{Scalar, Vector};
use bevy::prelude::*;

/// Mass component for physics bodies
#[derive(Component, Debug, Clone, Copy)]
pub struct Mass(pub Scalar);

impl Mass {
    pub fn new(mass: Scalar) -> Self {
        Self(mass)
    }

    #[inline]
    pub fn value(&self) -> Scalar {
        self.0
    }
}

/// Velocity component for physics bodies
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Velocity(pub Vector);

impl Velocity {
    pub fn new(velocity: Vector) -> Self {
        Self(velocity)
    }

    #[inline]
    pub fn value(&self) -> Vector {
        self.0
    }

    #[inline]
    pub fn value_mut(&mut self) -> &mut Vector {
        &mut self.0
    }
}

/// Acceleration component for physics bodies (computed from forces)
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Acceleration(pub Vector);

impl Acceleration {
    pub fn new(acceleration: Vector) -> Self {
        Self(acceleration)
    }

    #[inline]
    pub fn value(&self) -> Vector {
        self.0
    }

    #[inline]
    pub fn value_mut(&mut self) -> &mut Vector {
        &mut self.0
    }
}

/// Radius component for physics bodies (used for rendering and trails)
#[derive(Component, Debug, Clone, Copy)]
pub struct Radius(pub Scalar);

impl Radius {
    pub fn new(radius: Scalar) -> Self {
        Self(radius)
    }

    #[inline]
    pub fn value(&self) -> Scalar {
        self.0
    }
}

/// High-precision position for physics calculations
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Position(pub Vector);

impl Position {
    pub fn new(position: Vector) -> Self {
        Self(position)
    }

    #[inline]
    pub fn value(&self) -> Vector {
        self.0
    }

    #[inline]
    pub fn value_mut(&mut self) -> &mut Vector {
        &mut self.0
    }

    /// Check if Transform needs updating based on precision threshold
    #[inline]
    pub fn needs_transform_update(&self, transform: &Transform) -> bool {
        let current_pos = Vector::new(
            transform.translation.x as Scalar,
            transform.translation.y as Scalar,
            transform.translation.z as Scalar,
        );
        (self.0 - current_pos).length_squared() > 1e-6 // ~0.001 units
    }
}

/// Marker component for physics bodies that should be simulated
#[derive(Component, Debug, Default)]
pub struct PhysicsBody;

/// Complete kinematic state at a single point in time
///
/// Represents the full motion state of a body, including position,
/// velocity, and acceleration. Used by integrators to store and
/// retrieve historical states for multi-step integration methods.
#[derive(Debug, Clone, Copy, Default)]
pub struct KinematicState {
    /// Position vector in world space
    pub position: Vector,
    /// Velocity vector (rate of change of position)
    pub velocity: Vector,
    /// Acceleration vector (rate of change of velocity)
    pub acceleration: Vector,
}

impl KinematicState {
    /// Create a new kinematic state with the given components
    pub fn new(position: Vector, velocity: Vector, acceleration: Vector) -> Self {
        Self {
            position,
            velocity,
            acceleration,
        }
    }

    /// Create a kinematic state from ECS component references
    ///
    /// Useful for capturing the current state of a physics body
    /// from its individual component values.
    pub fn from_components(pos: &Position, vel: &Velocity, acc: &Acceleration) -> Self {
        Self {
            position: pos.value(),
            velocity: vel.value(),
            acceleration: acc.value(),
        }
    }

    /// Apply this kinematic state to ECS components
    ///
    /// Updates the given component references with the values
    /// stored in this state. Useful for restoring a previous
    /// state or applying predictions.
    pub fn apply_to_components(
        &self,
        pos: &mut Position,
        vel: &mut Velocity,
        acc: &mut Acceleration,
    ) {
        *pos.value_mut() = self.position;
        *vel.value_mut() = self.velocity;
        *acc.value_mut() = self.acceleration;
    }
}

/// Fixed-size circular buffer for storing kinematic history
///
/// Maintains a rolling history of 8 kinematic states for use by multi-step
/// integration methods (e.g., Adams-Bashforth, Runge-Kutta). This size is
/// sufficient for most practical integration methods:
/// - 4 states: 4th-order Runge-Kutta
/// - 5 states: 5th-order Adams-Bashforth
/// - 8 states: Provides headroom for higher-order methods
///
/// # Implementation Details
/// - Fixed size of 8 to simplify ECS queries
/// - Implements a circular buffer where oldest states are overwritten
/// - Tracks the number of valid entries during warm-up period
/// - Provides O(1) insertion and O(1) access by age
///
/// # Example
/// ```ignore
/// // Create a history buffer
/// let mut history = KinematicHistory::new(initial_state);
///
/// // Add new states as simulation progresses
/// history.push(new_state);
///
/// // Access previous states by age
/// let previous = history.get(1);  // One step ago
/// let two_ago = history.get(2);   // Two steps ago
/// ```
#[derive(Component, Debug, Clone)]
pub struct KinematicHistory {
    /// Circular buffer of states (fixed size 8)
    states: [KinematicState; 8],
    /// Index of oldest entry (next to be overwritten)
    head: usize,
    /// Number of valid entries (for warm-up period)
    count: usize,
}

impl KinematicHistory {
    /// Create a new history buffer with an initial state
    ///
    /// The buffer is initialized with the given state in all positions,
    /// but `count` starts at 0, indicating no valid history yet.
    pub fn new(initial: KinematicState) -> Self {
        Self {
            states: [initial; 8],
            head: 0,
            count: 0,
        }
    }

    /// Push a new state into the history, overwriting the oldest if full
    ///
    /// States are stored in a circular fashion. Once the buffer is full,
    /// each new state overwrites the oldest one.
    pub fn push(&mut self, state: KinematicState) {
        self.states[self.head] = state;
        self.head = (self.head + 1) % 8;
        self.count = self.count.saturating_add(1).min(8);
    }

    /// Get a historical state by its age
    ///
    /// # Arguments
    /// * `age` - How many steps back (0 = current, 1 = previous, etc.)
    ///
    /// # Returns
    /// * `Some(&KinematicState)` if the state exists
    /// * `None` if age exceeds available history
    pub fn get(&self, age: usize) -> Option<&KinematicState> {
        if age >= self.count {
            return None;
        }
        let idx = (self.head + 8 - 1 - age) % 8;
        Some(&self.states[idx])
    }

    /// Get most recent state
    pub fn current(&self) -> Option<&KinematicState> {
        self.get(0)
    }

    /// Check if the buffer has enough states for the required order
    ///
    /// Used by multi-step integrators to verify they have sufficient
    /// historical data before attempting integration.
    pub fn is_ready(&self, required_order: usize) -> bool {
        self.count >= required_order
    }

    /// Iterate over all states in chronological order (oldest to newest)
    ///
    /// Useful for algorithms that need to process states sequentially
    /// from oldest to most recent.
    pub fn iter_chronological(&self) -> impl Iterator<Item = &KinematicState> {
        (0..self.count).rev().filter_map(move |age| self.get(age))
    }
}

/// Component bundle for spawning physics bodies
#[derive(Bundle)]
pub struct PhysicsBodyBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub position: Position,
    pub mass: Mass,
    pub velocity: Velocity,
    pub acceleration: Acceleration,
    pub radius: Radius,
    pub physics_body: PhysicsBody,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
}

impl PhysicsBodyBundle {
    pub fn new(position: Vector, mass: f32, radius: f32, velocity: Vector) -> Self {
        Self {
            transform: Transform::from_translation(position.as_vec3()),
            global_transform: GlobalTransform::default(),
            position: Position::new(position),
            mass: Mass::new(mass.into()),
            velocity: Velocity::new(velocity),
            acceleration: Acceleration::default(),
            radius: Radius::new(radius.into()),
            physics_body: PhysicsBody,
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            view_visibility: ViewVisibility::default(),
        }
    }
}
