//! Physics resources for simulation

use super::integrators::Integrator;
use crate::physics::math::Scalar;
use bevy::prelude::*;

/// Resource holding the currently active integrator
#[derive(Resource)]
pub struct CurrentIntegrator(pub Box<dyn Integrator + Send + Sync>);

impl Default for CurrentIntegrator {
    fn default() -> Self {
        Self(Box::new(super::integrators::SymplecticEuler))
    }
}

/// Resource for physics timestep control
#[derive(Resource, Debug, Clone)]
pub struct PhysicsTime {
    /// Timestep for physics simulation
    pub dt: Scalar,
    /// Whether physics is paused
    pub paused: bool,
    /// Accumulated time for fixed timestep
    pub accumulator: Scalar,
}

impl Default for PhysicsTime {
    fn default() -> Self {
        Self {
            dt: 1.0 / 60.0, // 60 Hz default // TODO: base on FixedUpdate schedule?
            paused: false,
            accumulator: 0.0,
        }
    }
}

impl PhysicsTime {
    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn unpause(&mut self) {
        self.paused = false;
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn reset_accumulator(&mut self) {
        self.accumulator = 0.0;
    }
}
