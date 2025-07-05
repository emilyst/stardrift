//! Simulation diagnostics module.
//!
//! This module provides diagnostic tracking capabilities for the N-body simulation.
//! It collects and monitors key simulation metrics including:
//!
//! - **Barycenter position**: Tracks the X, Y, and Z coordinates of the system's center of mass
//!
//! The diagnostics are collected at regular intervals and can be consumed by other systems
//! for display, logging, or analysis purposes. The module integrates with Bevy's diagnostic
//! system to provide standardized metric collection and history tracking.
//!
//! # Main Components
//!
//! - [`SimulationDiagnosticsPlugin`]: The main plugin that registers diagnostic paths and
//!   systems for data collection
//! - [`SimulationDiagnosticsState`]: Resource that manages the update timing for diagnostics
//!
//! # Usage
//!
//! Add the plugin to your Bevy app to enable simulation diagnostics:
//!
//! ```rust,ignore
//! app.add_plugins(SimulationDiagnosticsPlugin::default());
//! ```

use crate::states::AppState;
use bevy::diagnostic::DEFAULT_MAX_HISTORY_LENGTH;
use bevy::diagnostic::Diagnostic;
use bevy::diagnostic::DiagnosticPath;
use bevy::diagnostic::RegisterDiagnostic;
use bevy::prelude::*;
use core::time::Duration;

#[derive(Resource)]
pub struct SimulationDiagnosticsState {
    update_timer: Timer,
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
    const DIAGNOSTIC_PATHS: &'static [DiagnosticPath] = &[];

    fn register_diagnostics(&self, app: &mut App) {
        for path in Self::DIAGNOSTIC_PATHS {
            app.register_diagnostic(
                Diagnostic::new(path.clone())
                    .with_max_history_length(self.max_history_length)
                    .with_smoothing_factor(self.smoothing_factor),
            );
        }
    }

    fn update_timer_ticks(mut state: ResMut<SimulationDiagnosticsState>, time: Res<Time>) {
        state.update_timer.tick(time.delta());
    }
}

impl Plugin for SimulationDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimulationDiagnosticsState {
            update_timer: Timer::new(self.update_interval, TimerMode::Repeating),
        });

        self.register_diagnostics(app);

        app.add_systems(
            FixedPostUpdate,
            Self::update_timer_ticks.run_if(in_state(AppState::Running)),
        );
    }
}
