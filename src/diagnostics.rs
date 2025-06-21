use crate::CurrentBarycenter;
use bevy::diagnostic::Diagnostic;
use bevy::diagnostic::DiagnosticPath;
use bevy::diagnostic::Diagnostics;
use bevy::diagnostic::RegisterDiagnostic;
use bevy::diagnostic::DEFAULT_MAX_HISTORY_LENGTH;
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
            smoothing_factor: 0.1,
            update_interval: Duration::from_secs_f64(1_f64 / 60_f64),
        }
    }
}

impl SimulationDiagnosticsPlugin {
    pub const BARYCENTER_X_PATH: DiagnosticPath = DiagnosticPath::const_new("barycenter/x");
    pub const BARYCENTER_Y_PATH: DiagnosticPath = DiagnosticPath::const_new("barycenter/y");
    pub const BARYCENTER_Z_PATH: DiagnosticPath = DiagnosticPath::const_new("barycenter/z");

    pub const CAMERA_X_PATH: DiagnosticPath = DiagnosticPath::const_new("camera/x");
    pub const CAMERA_Y_PATH: DiagnosticPath = DiagnosticPath::const_new("camera/y");
    pub const CAMERA_Z_PATH: DiagnosticPath = DiagnosticPath::const_new("camera/z");

    const DIAGNOSTIC_PATHS: &'static [DiagnosticPath] = &[
        Self::BARYCENTER_X_PATH,
        Self::BARYCENTER_Y_PATH,
        Self::BARYCENTER_Z_PATH,
        Self::CAMERA_X_PATH,
        Self::CAMERA_Y_PATH,
        Self::CAMERA_Z_PATH,
    ];

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

    fn update_barycenter_diagnostics(
        barycenter: Res<CurrentBarycenter>,
        mut diagnostics: Diagnostics,
        state: ResMut<SimulationDiagnosticsState>,
    ) {
        if state.update_timer.finished() {
            diagnostics.add_measurement(&Self::BARYCENTER_X_PATH, || barycenter.x);
            diagnostics.add_measurement(&Self::BARYCENTER_Y_PATH, || barycenter.y);
            diagnostics.add_measurement(&Self::BARYCENTER_Z_PATH, || barycenter.z);
        }
    }

    fn update_camera_diagnostics(
        camera_transform: Single<&Transform, With<Camera>>,
        mut diagnostics: Diagnostics,
        state: ResMut<SimulationDiagnosticsState>,
    ) {
        if state.update_timer.finished() {
            diagnostics.add_measurement(&Self::CAMERA_X_PATH, || {
                camera_transform.translation.x as f64
            });
            diagnostics.add_measurement(&Self::CAMERA_Y_PATH, || {
                camera_transform.translation.y as f64
            });
            diagnostics.add_measurement(&Self::CAMERA_Z_PATH, || {
                camera_transform.translation.z as f64
            });
        }
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
            (
                Self::update_timer_ticks,
                Self::update_barycenter_diagnostics,
                Self::update_camera_diagnostics,
            )
                .chain(),
        );
    }
}
