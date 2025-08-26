#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use clap::Parser;

use bevy::diagnostic::{
    EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
    SystemInformationDiagnosticsPlugin,
};
use bevy::{app::TaskPoolThreadAssignmentPolicy, tasks::available_parallelism, window::WindowMode};
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use stardrift::plugins::screenshot::{
    AutomatedScreenshotNaming, AutomatedScreenshotSchedule, ScreenshotPlugin,
};
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

/// Stardrift - N-body gravity simulation
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to configuration file (TOML format)
    #[arg(short, long, value_name = "FILE")]
    config: Option<String>,

    /// Number of bodies to simulate (overrides config file)
    #[arg(short = 'n', long, value_name = "COUNT")]
    bodies: Option<usize>,

    /// Gravitational constant (overrides config file)
    #[arg(short = 'g', long, value_name = "VALUE")]
    gravity: Option<f64>,

    /// Integrator type (e.g., velocity_verlet, rk4, heun)
    #[arg(short = 'i', long, value_name = "TYPE")]
    integrator: Option<String>,

    /// Random seed for body generation
    #[arg(short = 's', long, value_name = "SEED")]
    seed: Option<u64>,

    /// Start paused
    #[arg(short = 'p', long)]
    paused: bool,

    /// Enable verbose logging
    #[arg(short = 'v', long)]
    verbose: bool,

    /// List available integrators and exit
    #[arg(long)]
    list_integrators: bool,

    /// Take screenshot after N seconds (can be fractional)
    #[arg(long, value_name = "SECONDS")]
    screenshot_after: Option<f32>,

    /// Take multiple screenshots at intervals (seconds between shots)
    #[arg(long, value_name = "SECONDS")]
    screenshot_interval: Option<f32>,

    /// Number of screenshots to take (requires --screenshot-interval)
    #[arg(long, value_name = "COUNT", default_value = "1")]
    screenshot_count: usize,

    /// Use frame-based timing instead of time-based
    #[arg(long)]
    screenshot_use_frames: bool,

    /// Directory for automated screenshots (creates if needed)
    #[arg(long, value_name = "PATH")]
    screenshot_dir: Option<String>,

    /// Base filename for screenshots (without extension)
    #[arg(long, value_name = "NAME")]
    screenshot_name: Option<String>,

    /// Disable timestamp in filenames for predictable names
    #[arg(long)]
    screenshot_no_timestamp: bool,

    /// Use sequential numbering instead of timestamps
    #[arg(long)]
    screenshot_sequential: bool,

    /// Output full paths of created screenshots to stdout
    #[arg(long)]
    screenshot_list_paths: bool,

    /// Exit after taking all screenshots
    #[arg(long)]
    exit_after_screenshots: bool,
}

fn main() {
    let args = Args::parse();

    // Handle list-integrators flag
    if args.list_integrators {
        use stardrift::physics::integrators::registry::IntegratorRegistry;
        let registry = IntegratorRegistry::new().with_standard_integrators();
        println!("Available integrators:");
        for name in registry.list_available() {
            println!("  - {name}");
        }

        let aliases = registry.list_aliases();
        if !aliases.is_empty() {
            println!("\nAliases:");
            for (alias, target) in aliases {
                println!("  - {alias} -> {target}");
            }
        }
        return;
    }

    // Load configuration
    let mut config = if let Some(config_path) = &args.config {
        println!("Loading configuration from: {config_path}");
        SimulationConfig::load_or_default(config_path)
    } else {
        SimulationConfig::load_from_user_config()
    };

    // Apply command-line overrides
    if let Some(body_count) = args.bodies {
        println!("Overriding body count to: {body_count}");
        config.physics.body_count = body_count;
    }

    if let Some(gravity) = args.gravity {
        println!("Overriding gravitational constant to: {gravity}");
        config.physics.gravitational_constant = gravity;
    }

    if let Some(integrator_type) = args.integrator {
        println!("Using integrator: {integrator_type}");
        config.physics.integrator = stardrift::config::IntegratorConfig { integrator_type };
    }

    if let Some(seed) = args.seed {
        println!("Using random seed: {seed}");
        config.physics.initial_seed = Some(seed);
    }

    // Set up logging
    if args.verbose {
        unsafe {
            std::env::set_var("RUST_LOG", "stardrift=debug,bevy=info");
        }
    }

    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    fit_canvas_to_parent: true,
                    fullsize_content_view: true,
                    mode: WindowMode::Windowed,
                    prevent_default_event_handling: true,
                    present_mode: bevy::window::PresentMode::AutoVsync,
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
        EntityCountDiagnosticsPlugin,
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
    ));

    // Initialize app states after DefaultPlugins (which includes StatesPlugin)
    app.init_state::<AppState>();

    // Start paused if requested
    if args.paused {
        app.insert_resource(NextState::Pending(AppState::Paused));
    }

    // Set up automated screenshots if requested
    if let Some(schedule) = AutomatedScreenshotSchedule::new(
        args.screenshot_after,
        args.screenshot_interval,
        args.screenshot_count,
        args.screenshot_use_frames,
        args.exit_after_screenshots,
    ) {
        app.insert_resource(schedule);

        // Also set up naming if any naming options are provided
        if args.screenshot_dir.is_some()
            || args.screenshot_name.is_some()
            || args.screenshot_no_timestamp
            || args.screenshot_sequential
            || args.screenshot_list_paths
        {
            // Need to get config from the app world since we've already moved it
            let config = app.world().resource::<SimulationConfig>().clone();
            let naming = AutomatedScreenshotNaming::new(
                args.screenshot_dir,
                args.screenshot_name,
                args.screenshot_no_timestamp,
                args.screenshot_sequential,
                args.screenshot_list_paths,
                &config,
            );
            app.insert_resource(naming);
        }
    }

    app.run();
}
