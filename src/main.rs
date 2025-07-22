#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
#![allow(dead_code)]

mod components;
mod config;
mod physics;
mod plugins;
mod prelude;
mod resources;
mod states;
mod systems;
mod utils;

#[cfg(feature = "trails")]
use crate::plugins::trails::TrailsPlugin;
#[cfg(feature = "diagnostics")]
use crate::plugins::{
    diagnostics_hud::DiagnosticsHudPlugin, simulation_diagnostics::SimulationDiagnosticsPlugin,
};
use crate::plugins::{embedded_assets::EmbeddedAssetsPlugin, simulation::SimulationPlugin};
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
                    fullsize_content_view: true,
                    mode: WindowMode::Windowed,
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
        #[cfg(feature = "trails")]
        TrailsPlugin,
    ));

    // Initialize app states after DefaultPlugins (which includes StatesPlugin)
    app.init_state::<AppState>();
    app.add_sub_state::<LoadingState>();

    app.run();
}
