//! Simulation diagnostics plugin - Self-contained plugin pattern
//!
//! This plugin follows the self-contained pattern for collecting and tracking
//! simulation-specific performance metrics. Currently a placeholder for future
//! diagnostics implementation such as octree build time, force calculation
//! performance, and physics step timing.

use crate::physics::components::{Mass, PhysicsBody, Velocity};
use crate::physics::math::Scalar;
use crate::physics::resources::BhProbeState;
use crate::states::AppState;
use bevy::diagnostic::DEFAULT_MAX_HISTORY_LENGTH;
use bevy::diagnostic::{
    Diagnostic, DiagnosticPath, Diagnostics, DiagnosticsStore, RegisterDiagnostic,
};
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

    /// Barnes-Hut probe: `sqrt(Σ|a_bh − a_ref|² / Σ|a_ref|²)`
    pub const BH_ACCEL_ERROR_L2: DiagnosticPath =
        DiagnosticPath::const_new("simulation/barnes_hut/accel_error_l2");
    /// Barnes-Hut probe: floored per-body relative maximum
    pub const BH_ACCEL_ERROR_MAX: DiagnosticPath =
        DiagnosticPath::const_new("simulation/barnes_hut/accel_error_max");
    /// Barnes-Hut probe: fraction of surviving bodies whose octant path
    /// changed since the previous sample
    pub const BH_TOPOLOGY_CHURN: DiagnosticPath =
        DiagnosticPath::const_new("simulation/barnes_hut/topology_churn");

    const BH_PATHS: [DiagnosticPath; 3] = [
        Self::BH_ACCEL_ERROR_L2,
        Self::BH_ACCEL_ERROR_MAX,
        Self::BH_TOPOLOGY_CHURN,
    ];

    /// Probe samples arrive a few times per second; a short history keeps
    /// the `LogDiagnosticsPlugin { debug: true }` dump readable.
    const BH_HISTORY_LENGTH: usize = 16;

    fn register_diagnostics(&self, app: &mut App) {
        // Register kinetic energy diagnostic with units
        app.register_diagnostic(
            Diagnostic::new(Self::KINETIC_ENERGY)
                .with_max_history_length(self.max_history_length)
                .with_smoothing_factor(self.smoothing_factor)
                .with_suffix("J"), // Add units for clarity
        );
        // Unsmoothed so readers see the latest sample, not an average
        for path in Self::BH_PATHS {
            app.register_diagnostic(
                Diagnostic::new(path)
                    .with_max_history_length(Self::BH_HISTORY_LENGTH)
                    .with_smoothing_factor(0.0),
            );
        }
    }

    /// Enable the probe diagnostics only when the probe runs: disabled
    /// diagnostics accept no measurements and are skipped by the log
    /// plugin, so an idle probe leaves no trace in the diagnostics dump.
    /// Runs at `Startup` because `BhProbeState` is inserted by the
    /// simulation plugin, which may be added after this one.
    fn enable_bh_diagnostics(mut store: ResMut<DiagnosticsStore>, probe: Res<BhProbeState>) {
        for path in Self::BH_PATHS {
            if let Some(diagnostic) = store.get_mut(&path) {
                diagnostic.is_enabled = probe.enabled;
            }
        }
    }

    /// Forward each new probe observation to the diagnostics store and the
    /// log. Gated on `sample_id` so nothing is re-emitted between samples.
    fn report_bh_probe(
        probe: Res<BhProbeState>,
        mut diagnostics: Diagnostics,
        mut last_sample_id: Local<u64>,
    ) {
        if probe.sample_id == *last_sample_id {
            return;
        }
        *last_sample_id = probe.sample_id;
        let Some(sample) = probe.latest else {
            return;
        };
        diagnostics.add_measurement(&Self::BH_ACCEL_ERROR_L2, || sample.accel_error_l2);
        diagnostics.add_measurement(&Self::BH_ACCEL_ERROR_MAX, || sample.accel_error_max);
        if let Some(churn) = sample.topology_churn {
            diagnostics.add_measurement(&Self::BH_TOPOLOGY_CHURN, || churn);
        }
        match sample.topology_churn {
            Some(churn) => info!(
                target: "stardrift::bh_probe",
                "bh_probe sample={} bodies={} accel_error_l2={:.3e} accel_error_max={:.3e} topology_churn={:.4}",
                probe.sample_id, sample.bodies, sample.accel_error_l2, sample.accel_error_max, churn
            ),
            None => info!(
                target: "stardrift::bh_probe",
                "bh_probe sample={} bodies={} accel_error_l2={:.3e} accel_error_max={:.3e} topology_churn=n/a",
                probe.sample_id, sample.bodies, sample.accel_error_l2, sample.accel_error_max
            ),
        }
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

        app.add_systems(Startup, Self::enable_bh_diagnostics);
        app.add_systems(
            FixedPostUpdate,
            (
                Self::update_timer_ticks,
                Self::calculate_kinetic_energy,
                Self::report_bh_probe,
            )
                .run_if(in_state(AppState::Running)),
        );
    }
}
