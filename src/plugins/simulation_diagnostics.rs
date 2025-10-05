//! Simulation diagnostics plugin - Self-contained plugin pattern
//!
//! This plugin follows the self-contained pattern for collecting and tracking
//! simulation-specific performance metrics. Currently a placeholder for future
//! diagnostics implementation such as octree build time, force calculation
//! performance, and physics step timing.

use crate::physics::components::{Mass, PhysicsBody, Velocity};
use crate::physics::math::Scalar;
use crate::states::AppState;
use bevy::diagnostic::DEFAULT_MAX_HISTORY_LENGTH;
use bevy::diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic};
use bevy::prelude::*;
use core::time::Duration;

/// Lightweight resource for tracking simulation metrics
#[derive(Resource, Default)]
pub struct SimulationMetrics {
    /// Last calculated kinetic energy
    pub kinetic_energy: Scalar,
}

#[derive(Resource)]
pub struct SimulationDiagnosticsState {
    pub update_timer: Timer,
}

pub struct SimulationDiagnosticsPlugin {
    max_history_length: usize,
    smoothing_factor: f64,
    update_interval: Duration,
}

impl Default for SimulationDiagnosticsPlugin {
    fn default() -> Self {
        Self {
            max_history_length: DEFAULT_MAX_HISTORY_LENGTH,
            smoothing_factor: 0.0,
            update_interval: Duration::from_secs_f64(1_f64 / 60_f64),
        }
    }
}

impl SimulationDiagnosticsPlugin {
    /// Diagnostic path for kinetic energy
    pub const KINETIC_ENERGY: DiagnosticPath =
        DiagnosticPath::const_new("simulation/energy/kinetic");

    fn register_diagnostics(&self, app: &mut App) {
        // Register kinetic energy diagnostic with units
        app.register_diagnostic(
            Diagnostic::new(Self::KINETIC_ENERGY)
                .with_max_history_length(self.max_history_length)
                .with_smoothing_factor(self.smoothing_factor)
                .with_suffix("J"), // Add units for clarity
        );
    }

    fn update_timer_ticks(mut state: ResMut<SimulationDiagnosticsState>, time: Res<Time>) {
        state.update_timer.tick(time.delta());
    }

    /// Calculate total kinetic energy of the system
    ///
    /// Kinetic energy = ½ Σ(m·v²) for all bodies
    fn calculate_kinetic_energy(
        bodies: Query<(&Velocity, &Mass), With<PhysicsBody>>,
        mut metrics: ResMut<SimulationMetrics>,
        mut diagnostics: Diagnostics,
        state: Res<SimulationDiagnosticsState>,
    ) {
        // Only update when timer is ready to avoid excessive computation
        if !state.update_timer.is_finished() {
            return;
        }

        let kinetic_energy: Scalar = bodies
            .iter()
            .map(|(velocity, mass)| {
                let v_squared = velocity.value().length_squared();
                0.5 * mass.value() * v_squared
            })
            .sum();

        metrics.kinetic_energy = kinetic_energy;
        diagnostics.add_measurement(&Self::KINETIC_ENERGY, || kinetic_energy as f64);
    }
}

impl Plugin for SimulationDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimulationDiagnosticsState {
            update_timer: Timer::new(self.update_interval, TimerMode::Repeating),
        })
        .init_resource::<SimulationMetrics>();

        self.register_diagnostics(app);

        app.add_systems(
            FixedPostUpdate,
            (Self::update_timer_ticks, Self::calculate_kinetic_energy)
                .run_if(in_state(AppState::Running)),
        );
    }
}
