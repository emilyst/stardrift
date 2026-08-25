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

/// Physics tick rate. Feeds both the `FixedUpdate` schedule rate and
/// [`PhysicsTime::dt`] so wall-clock pacing and the integration step can
/// never disagree (Bevy's `FixedUpdate` otherwise defaults to 64 Hz).
pub const PHYSICS_TICK_HZ: Scalar = 60.0;

/// Resource for physics timestep control
#[derive(Resource, Debug, Clone)]
pub struct PhysicsTime {
    /// Timestep for physics simulation
    pub dt: Scalar,
    /// Whether physics is paused
    pub paused: bool,
}

impl Default for PhysicsTime {
    fn default() -> Self {
        Self {
            dt: 1.0 / PHYSICS_TICK_HZ,
            paused: false,
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
}
