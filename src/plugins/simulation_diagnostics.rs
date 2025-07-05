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
