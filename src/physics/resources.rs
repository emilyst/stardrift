//! Physics resources for simulation

use super::bh_probe::BhSample;
use super::integrators::Integrator;
use crate::config::BhProbeConfig;
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

/// Barnes-Hut probe switch, sampling counters, and the latest observation.
///
/// Written by the integration driver (which alone holds the stage snapshot
/// the probe needs), read by the diagnostics and HUD plugins. Optional in
/// the driver's signature so bare test worlds need not insert it.
#[derive(Resource, Debug, Clone)]
pub struct BhProbeState {
    pub enabled: bool,
    /// Probe the first tree build of every Nth non-paused step
    pub sample_every_steps: u32,
    /// Non-paused steps seen while enabled
    pub steps: u64,
    /// Increments with every observation; readers use it to detect a new
    /// sample rather than re-reporting `latest`
    pub sample_id: u64,
    pub latest: Option<BhSample>,
}

impl BhProbeState {
    pub fn from_config(config: &BhProbeConfig) -> Self {
        Self {
            enabled: config.enabled,
            sample_every_steps: config.sample_every_steps.max(1),
            steps: 0,
            sample_id: 0,
            latest: None,
        }
    }
}
