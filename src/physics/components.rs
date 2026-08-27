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

/// The body's display color, fixed at spawn (saturation already applied).
/// The source of truth for both renderers: the bodies plugin packs it into
/// `MeshTag`, the trails plugin into its own tag encoding.
#[derive(Component, Debug, Clone, Copy)]
pub struct BodyColor(pub Color);

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

/// The body's committed position at the start of the current step, written by
/// the integration driver just before it commits the new position. Collision
/// detection sweeps the segment from this to `Position`; the pair joins
/// consecutive steps' segments exactly, leaving no gap for a contact to fall
/// through.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PreviousPosition(pub Vector);

impl PreviousPosition {
    #[inline]
    pub fn value(&self) -> Vector {
        self.0
    }
}

/// Marker component for physics bodies that should be simulated.
///
/// The `Added<PhysicsBody>` window is load-bearing for rendering: both the
/// bodies plugin (mesh/material/tag attach) and the trails plugin (trail
/// renderer spawn) react to it from `Update`, after `SimulationSet::Input`.
/// A spawn path added outside that ordering must still flush its commands
/// before those systems run, or the body comes up invisible and trail-less.
///
/// Requires `PreviousPosition` so no spawn path can create a body that
/// gravitates but is invisible to collision detection (the collision query
/// hard-requires the component). A required-default `PreviousPosition` of
/// zero never reaches the collision system: the integration driver's
/// write-back overwrites it with the true start-of-step position before
/// collisions run.
#[derive(Component, Debug, Default)]
#[require(PreviousPosition)]
pub struct PhysicsBody;

/// Component bundle for spawning physics bodies
#[derive(Bundle)]
pub struct PhysicsBodyBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub position: Position,
    pub previous_position: PreviousPosition,
    pub mass: Mass,
    pub velocity: Velocity,
    pub radius: Radius,
    pub physics_body: PhysicsBody,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
}

impl PhysicsBodyBundle {
    pub fn new(position: Vector, mass: Scalar, radius: f32, velocity: Vector) -> Self {
        Self {
            transform: Transform::from_translation(position.as_vec3())
                .with_scale(Vec3::splat(radius)),
            global_transform: GlobalTransform::default(),
            position: Position::new(position),
            previous_position: PreviousPosition(position),
            mass: Mass::new(mass),
            velocity: Velocity::new(velocity),
            radius: Radius::new(radius.into()),
            physics_body: PhysicsBody,
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            view_visibility: ViewVisibility::default(),
        }
    }
}
