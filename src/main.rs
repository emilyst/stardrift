#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use clap::Parser;

use bevy::diagnostic::{
    EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
    SystemInformationDiagnosticsPlugin,
};
use bevy::log::{Level, LogPlugin};
use bevy::window::{MonitorSelection, PresentMode, WindowMode};
use bevy::{app::TaskPoolThreadAssignmentPolicy, tasks::available_parallelism};
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use stardrift::cli;
use stardrift::config::WindowModeConfig;
use stardrift::plugins::keep_awake::KeepAwakePlugin;
use stardrift::plugins::loading_screen::{LoadingScreenPlugin, PostLoadingState};
use stardrift::plugins::screenshot::ScreenshotPlugin;
use stardrift::plugins::trails::TrailsPlugin;
use stardrift::plugins::{
    attribution::AttributionPlugin, bodies::BodiesPlugin, camera::CameraPlugin,
    controls::ControlsPlugin, embedded_assets::EmbeddedAssetsPlugin, simulation::SimulationPlugin,
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

    // Handle print-default-config flag
    if args.print_default_config {
        cli::handle_print_default_config();
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

    // Benchmark mode takes the whole screen and drops vsync so frame times
    // reflect the workload rather than the display's refresh rate. Windowed
    // macOS pins to refresh regardless of present mode; borderless fullscreen
    // with Immediate is the combination that actually uncaps there.
    //
    // WASM always runs windowed: the canvas already fills its parent, and
    // browser fullscreen can only be entered from a user gesture.
    let fullscreen = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
    let (window_mode, present_mode) = if args.bench_mode {
        (fullscreen, PresentMode::Immediate)
    } else {
        let mode = match config.system.window_mode {
            _ if cfg!(target_arch = "wasm32") => WindowMode::Windowed,
            WindowModeConfig::BorderlessFullscreen => fullscreen,
            WindowModeConfig::Windowed => WindowMode::Windowed,
        };
        (mode, PresentMode::Fifo)
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
                    mode: window_mode,
                    prevent_default_event_handling: true,
                    present_mode,
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
            // Benchmark mode surfaces frame-time diagnostics at info level
            // so they land in the default log output.
            debug: !args.bench_mode,
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
        BodiesPlugin,
        CameraPlugin,
        ControlsPlugin,
        VisualizationPlugin,
        AttributionPlugin,
        TrailsPlugin,
        ScreenshotPlugin,
        KeepAwakePlugin,
        LoadingScreenPlugin,
    ));

    // Initialize app states after DefaultPlugins (which includes StatesPlugin)
    app.init_state::<AppState>();

    // The app starts in AppState::Loading; this is where it goes afterwards.
    app.insert_resource(PostLoadingState(if args.paused {
        AppState::Paused
    } else {
        AppState::Running
    }));

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
