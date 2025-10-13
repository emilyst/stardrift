#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use clap::Parser;

use bevy::diagnostic::{
    EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
    SystemInformationDiagnosticsPlugin,
};
use bevy::log::{Level, LogPlugin};
use bevy::{app::TaskPoolThreadAssignmentPolicy, tasks::available_parallelism, window::WindowMode};
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use stardrift::cli;
use stardrift::plugins::keep_awake::KeepAwakePlugin;
use stardrift::plugins::screenshot::ScreenshotPlugin;
use stardrift::plugins::trails::TrailsPlugin;
use stardrift::plugins::{
    attribution::AttributionPlugin, camera::CameraPlugin, controls::ControlsPlugin,
    embedded_assets::EmbeddedAssetsPlugin, simulation::SimulationPlugin,
    visualization::VisualizationPlugin,
};
use stardrift::plugins::{
    diagnostics_hud::DiagnosticsHudPlugin, simulation_diagnostics::SimulationDiagnosticsPlugin,
};
use stardrift::prelude::*;

fn main() {
    let args = cli::Args::parse();

    // Handle list-integrators flag
    if args.list_integrators {
        cli::handle_list_integrators();
        return;
    }

    // Load configuration and apply CLI overrides
    let config = match cli::load_and_apply_config(&args) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Error: {err}");
            std::process::exit(1);
        }
    };

    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(LogPlugin {
                filter: if args.verbose {
                    "stardrift=debug,bevy=debug".to_string()
                } else {
                    "stardrift=info,bevy=info".to_string()
                },
                level: Level::INFO,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    fit_canvas_to_parent: true,
                    fullsize_content_view: true,
                    mode: WindowMode::Windowed,
                    prevent_default_event_handling: true,
                    present_mode: bevy::window::PresentMode::Fifo,
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
    );

    app.add_plugins((
        EmbeddedAssetsPlugin,
        DiagnosticsHudPlugin,
        LogDiagnosticsPlugin {
            debug: true,
            ..default()
        },
        EntityCountDiagnosticsPlugin::default(),
        FrameTimeDiagnosticsPlugin::default(),
        PanOrbitCameraPlugin,
        SimulationDiagnosticsPlugin::default(),
        SystemInformationDiagnosticsPlugin,
    ));

    app.add_plugins((
        SimulationPlugin::with_config(config),
        CameraPlugin,
        ControlsPlugin,
        VisualizationPlugin,
        AttributionPlugin,
        TrailsPlugin,
        ScreenshotPlugin,
        KeepAwakePlugin,
    ));

    // Initialize app states after DefaultPlugins (which includes StatesPlugin)
    app.init_state::<AppState>();

    // Start paused if requested
    if args.paused {
        app.insert_resource(NextState::Pending(AppState::Paused));
    }

    // Set up automated screenshots if requested
    let (schedule, naming) = cli::create_screenshot_resources(&args, app.world().resource());
    if let Some(schedule) = schedule {
        app.insert_resource(schedule);
    }
    if let Some(naming) = naming {
        app.insert_resource(naming);
    }

    app.run();
}
