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

    #[cfg(test)]
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

/// Component bundle for spawning physics bodies
#[derive(Bundle)]
pub struct PhysicsBodyBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub position: Position,
    pub mass: Mass,
    pub velocity: Velocity,
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
            radius: Radius::new(radius.into()),
            physics_body: PhysicsBody,
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            view_visibility: ViewVisibility::default(),
        }
    }
}
