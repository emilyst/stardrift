#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
#![allow(dead_code)]

mod components;
mod config;
mod events;
mod physics;
mod plugins;
mod prelude;
mod resources;
mod states;
mod utils;

#[cfg(feature = "trails")]
use crate::plugins::trails::TrailsPlugin;
use crate::plugins::{
    camera::CameraPlugin, controls::ControlsPlugin, embedded_assets::EmbeddedAssetsPlugin,
    simulation::SimulationPlugin, visualization::VisualizationPlugin,
};
#[cfg(feature = "diagnostics")]
use crate::plugins::{
    diagnostics_hud::DiagnosticsHudPlugin, simulation_diagnostics::SimulationDiagnosticsPlugin,
};
use crate::prelude::*;
#[cfg(feature = "diagnostics")]
use bevy::diagnostic::{
    EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, SystemInformationDiagnosticsPlugin,
};
use bevy::{app::TaskPoolThreadAssignmentPolicy, tasks::available_parallelism, window::WindowMode};
use bevy_panorbit_camera::PanOrbitCameraPlugin;

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    fit_canvas_to_parent: true,
                    fullsize_content_view: true,
                    mode: WindowMode::Windowed,
                    prevent_default_event_handling: true,
                    title: "Stardrift".to_string(),
                    titlebar_transparent: true,
                    ..default()
                }),
                ..default()
            })
            .set(TaskPoolPlugin {
                task_pool_options: TaskPoolOptions {
                    compute: TaskPoolThreadAssignmentPolicy {
                        // set the minimum # of compute threads
                        // to the total number of available threads
                        min_threads: available_parallelism(),
                        max_threads: usize::MAX, // unlimited max threads
                        percent: 1.0,            // this value is irrelevant in this case
                        on_thread_spawn: None,
                        on_thread_destroy: None,
                    },
                    ..default()
                },
            }),
        EmbeddedAssetsPlugin,
        #[cfg(feature = "diagnostics")]
        DiagnosticsHudPlugin,
        #[cfg(feature = "diagnostics")]
        EntityCountDiagnosticsPlugin,
        #[cfg(feature = "diagnostics")]
        FrameTimeDiagnosticsPlugin::default(),
        PanOrbitCameraPlugin,
        PhysicsPlugins::default(),
        #[cfg(feature = "diagnostics")]
        SimulationDiagnosticsPlugin::default(),
        #[cfg(feature = "diagnostics")]
        SystemInformationDiagnosticsPlugin,
        SimulationPlugin,
        CameraPlugin,
        ControlsPlugin,
        VisualizationPlugin,
        #[cfg(feature = "trails")]
        TrailsPlugin,
    ));

    // Initialize app states after DefaultPlugins (which includes StatesPlugin)
    app.init_state::<AppState>();

    app.run();
}
