mod components;
mod config;
mod physics;
mod plugins;
mod resources;
mod states;
mod systems;
mod utils;

use crate::plugins::diagnostics_hud::DiagnosticsHudPlugin;
use crate::plugins::embedded_assets::EmbeddedAssetsPlugin;
use crate::plugins::simulation::SimulationPlugin;
use crate::plugins::simulation_diagnostics::SimulationDiagnosticsPlugin;
use crate::states::AppState;
use crate::states::LoadingState;
use avian3d::prelude::*;
use bevy::app::TaskPoolThreadAssignmentPolicy;
use bevy::diagnostic::EntityCountDiagnosticsPlugin;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::diagnostic::SystemInformationDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::tasks::available_parallelism;
use bevy::window::WindowMode;
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
                    // keep the defaults for everything else
                    ..default()
                },
            }),
        EmbeddedAssetsPlugin,
        DiagnosticsHudPlugin,
        EntityCountDiagnosticsPlugin,
        FrameTimeDiagnosticsPlugin::default(),
        PanOrbitCameraPlugin,
        PhysicsPlugins::default(),
        SimulationDiagnosticsPlugin::default(),
        SystemInformationDiagnosticsPlugin,
        SimulationPlugin,
    ));

    // Initialize app states after DefaultPlugins (which includes StatesPlugin)
    app.init_state::<AppState>();
    app.add_sub_state::<LoadingState>();

    app.run();
}
