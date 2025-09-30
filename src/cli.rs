//! Command line interface for Stardrift

use clap::Parser;
use std::fmt;

use crate::config::{ColorScheme, IntegratorConfig, SimulationConfig};
use crate::physics::integrators::registry::IntegratorRegistry;
use crate::plugins::screenshot::{AutomatedScreenshotNaming, AutomatedScreenshotSchedule};

/// CLI-specific errors
#[derive(Debug)]
pub enum CliError {
    /// Configuration file could not be loaded
    ConfigLoad(String),
    /// Invalid integrator name provided
    InvalidIntegrator(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::ConfigLoad(msg) => write!(f, "Failed to load configuration: {msg}"),
            CliError::InvalidIntegrator(msg) => write!(f, "Invalid integrator: {msg}"),
        }
    }
}

impl std::error::Error for CliError {}

/// Stardrift - N-body gravity simulation
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to configuration file (TOML format)
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<String>,

    /// Number of bodies to simulate (overrides config file)
    #[arg(short = 'n', long, value_name = "COUNT")]
    pub bodies: Option<usize>,

    /// Gravitational constant (overrides config file)
    #[arg(short = 'g', long, value_name = "VALUE")]
    pub gravity: Option<f64>,

    /// Integrator type (e.g., velocity_verlet, rk4, heun)
    #[arg(short = 'i', long, value_name = "TYPE")]
    pub integrator: Option<String>,

    /// Random seed for body generation
    #[arg(short = 's', long, value_name = "SEED")]
    pub seed: Option<u64>,

    /// Color scheme for bodies (e.g., black_body, viridis, rainbow)
    #[arg(long, value_name = "SCHEME")]
    pub color_scheme: Option<ColorScheme>,

    /// Start paused
    #[arg(short = 'p', long)]
    pub paused: bool,

    /// Enable verbose logging
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// List available integrators and exit
    #[arg(long)]
    pub list_integrators: bool,

    /// Take screenshot after N seconds (can be fractional)
    #[arg(long, value_name = "SECONDS")]
    pub screenshot_after: Option<f32>,

    /// Take multiple screenshots at intervals (seconds between shots)
    #[arg(long, value_name = "SECONDS")]
    pub screenshot_interval: Option<f32>,

    /// Number of screenshots to take (requires --screenshot-interval)
    #[arg(long, value_name = "COUNT", default_value = "1")]
    pub screenshot_count: usize,

    /// Use frame-based timing instead of time-based
    #[arg(long)]
    pub screenshot_use_frames: bool,

    /// Directory for automated screenshots (creates if needed)
    #[arg(long, value_name = "PATH")]
    pub screenshot_dir: Option<String>,

    /// Base filename for screenshots (without extension)
    #[arg(long, value_name = "NAME")]
    pub screenshot_name: Option<String>,

    /// Disable timestamp in filenames for predictable names
    #[arg(long)]
    pub screenshot_no_timestamp: bool,

    /// Use sequential numbering instead of timestamps
    #[arg(long)]
    pub screenshot_sequential: bool,

    /// Output full paths of created screenshots to stdout
    #[arg(long)]
    pub screenshot_list_paths: bool,

    /// Exit after taking all screenshots
    #[arg(long)]
    pub exit_after_screenshots: bool,
}

/// Handles the --list-integrators flag by printing available integrators and exiting
pub fn handle_list_integrators() {
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
}

/// Loads configuration from file or defaults, then applies command-line overrides
pub fn load_and_apply_config(args: &Args) -> Result<SimulationConfig, CliError> {
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

    if let Some(integrator_type) = &args.integrator {
        // Validate integrator name against registry
        let registry = IntegratorRegistry::new().with_standard_integrators();
        registry
            .create(integrator_type)
            .map_err(|err| CliError::InvalidIntegrator(err))?;

        println!("Using integrator: {integrator_type}");
        config.physics.integrator = IntegratorConfig {
            integrator_type: integrator_type.clone(),
        };
    }

    if let Some(seed) = args.seed {
        println!("Using random seed: {seed}");
        config.physics.initial_seed = Some(seed);
    }

    if let Some(color_scheme) = args.color_scheme {
        println!("Using color scheme: {color_scheme:?}");
        config.rendering.color_scheme = color_scheme;
    }

    Ok(config)
}

/// Creates screenshot schedule and naming resources based on CLI arguments
pub fn create_screenshot_resources(
    args: &Args,
    config: &SimulationConfig,
) -> (
    Option<AutomatedScreenshotSchedule>,
    Option<AutomatedScreenshotNaming>,
) {
    let schedule = AutomatedScreenshotSchedule::new(
        args.screenshot_after,
        args.screenshot_interval,
        args.screenshot_count,
        args.screenshot_use_frames,
        args.exit_after_screenshots,
    );

    let naming = if schedule.is_some()
        && (args.screenshot_dir.is_some()
            || args.screenshot_name.is_some()
            || args.screenshot_no_timestamp
            || args.screenshot_sequential
            || args.screenshot_list_paths)
    {
        Some(AutomatedScreenshotNaming::new(
            args.screenshot_dir.clone(),
            args.screenshot_name.clone(),
            args.screenshot_no_timestamp,
            args.screenshot_sequential,
            args.screenshot_list_paths,
            config,
        ))
    } else {
        None
    };

    (schedule, naming)
}
