use crate::prelude::*;
use clap::ValueEnum;
use config::{Config, ConfigError, File};
#[cfg(not(target_arch = "wasm32"))]
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Color scheme selection for celestial bodies
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default, ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub enum ColorScheme {
    /// Physics-based black body radiation colors
    #[default]
    BlackBody,
    /// Random vibrant colors using full spectrum
    Rainbow,

    // Colorblind-safe palettes
    /// Optimized for red-green colorblindness (most common)
    DeuteranopiaSafe,
    /// Optimized for red-blindness
    ProtanopiaSafe,
    /// Optimized for blue-yellow colorblindness
    TritanopiaSafe,
    /// High contrast for maximum distinguishability
    HighContrast,

    // Scientific colormaps
    /// Purple-blue-green-yellow gradient (perceptually uniform)
    Viridis,
    /// Magenta-purple-pink-yellow gradient
    Plasma,
    /// Black-red-yellow-white heat map
    Inferno,
    /// Improved rainbow (perceptually better than standard rainbow)
    Turbo,

    // Aesthetic themes
    /// Soft, low-saturation colors
    Pastel,
    /// High saturation cyberpunk-style colors
    Neon,
    /// Grayscale variations
    Monochrome,
    /// Vaporwave aesthetic with pink-purple-cyan palette
    Vaporwave,

    // Pride flag color schemes
    /// Bisexual pride flag colors (pink-purple-blue gradient)
    Bisexual,
    /// Transgender pride flag colors (light blue-pink-white)
    Transgender,
    /// Lesbian pride flag colors (orange to pink gradient)
    Lesbian,
    /// Pansexual pride flag colors (pink-yellow-blue)
    Pansexual,
    /// Non-binary pride flag colors (yellow-white-purple-black)
    Nonbinary,
    /// Asexual pride flag colors (black-gray-white-purple)
    Asexual,
    /// Genderfluid pride flag colors (pink-white-purple-black-blue)
    Genderfluid,
    /// Aromantic pride flag colors (green-white-gray-black)
    Aromantic,
    /// Agender pride flag colors (black-gray-white-green symmetric)
    Agender,
}

#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct SimulationConfig {
    pub physics: PhysicsConfig,
    pub rendering: RenderingConfig,
    pub trails: TrailConfig,
    pub screenshots: ScreenshotConfig,
    pub system: SystemConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct PhysicsConfig {
    pub gravitational_constant: Scalar,
    pub body_count: usize,
    pub octree_theta: Scalar,
    pub octree_leaf_threshold: usize,
    pub body_distribution_sphere_radius_multiplier: f32,
    pub body_distribution_min_distance: f32,
    pub min_body_radius: f32,
    pub max_body_radius: f32,
    pub force_calculation_min_distance: Scalar,
    pub force_calculation_max_force: Scalar,
    pub initial_seed: Option<u64>,
    pub initial_velocity: InitialVelocityConfig,
    #[serde(default)]
    pub integrator: IntegratorConfig,
    pub barycentric_drift_correction: bool,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravitational_constant: 100.0,
            body_count: 25,
            octree_theta: 0.5,
            octree_leaf_threshold: 1,
            body_distribution_sphere_radius_multiplier: 500.0,
            body_distribution_min_distance: 0.001,
            min_body_radius: 2.0,
            max_body_radius: 4.0,
            force_calculation_min_distance: 1.0,
            force_calculation_max_force: 1e5,
            initial_seed: None,
            initial_velocity: InitialVelocityConfig::default(),
            integrator: IntegratorConfig::default(),
            barycentric_drift_correction: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct InitialVelocityConfig {
    pub enabled: bool,
    pub min_speed: Scalar,
    pub max_speed: Scalar,
    pub velocity_mode: VelocityMode,
    pub tangential_bias: Scalar,
}

impl Default for InitialVelocityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_speed: 5.0,
            max_speed: 5.0,
            velocity_mode: VelocityMode::Orbital,
            tangential_bias: 0.7,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum VelocityMode {
    Random,
    Orbital,
    Tangential,
    Radial,
}

impl Default for VelocityMode {
    fn default() -> Self {
        VelocityMode::Orbital
    }
}

/// Flexible integrator configuration
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct IntegratorConfig {
    /// Type of integrator (e.g., "velocity_verlet")
    #[serde(rename = "type")]
    pub integrator_type: String,
}

impl Default for IntegratorConfig {
    fn default() -> Self {
        Self {
            integrator_type: "velocity_verlet".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct RenderingConfig {
    pub color_scheme: ColorScheme,
    pub min_temperature: f32,
    pub max_temperature: f32,
    pub bloom_intensity: f32,
    pub saturation_intensity: f32,
    pub camera_radius_multiplier: f32,
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            color_scheme: ColorScheme::Rainbow,
            min_temperature: 3000.0,
            max_temperature: 15000.0,
            bloom_intensity: 250.0,
            saturation_intensity: 3.0,
            camera_radius_multiplier: 3.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct TrailConfig {
    // Length & Timing
    pub trail_length_seconds: f32,
    pub update_interval_seconds: f32,
    pub max_points_per_trail: usize,

    // Visual Appearance
    pub base_width: f32,
    pub width_relative_to_body: bool,
    pub body_size_multiplier: f32,

    // Fading & Transparency
    pub enable_fading: bool,
    pub fade_curve: FadeCurve,
    pub min_alpha: f32,
    pub max_alpha: f32,

    // Width Tapering
    pub enable_tapering: bool,
    pub taper_curve: TaperCurve,
    pub min_width_ratio: f32,

    // Bloom Effect
    pub bloom_factor: f32,
    pub use_additive_blending: bool,
}

impl Default for TrailConfig {
    fn default() -> Self {
        Self {
            // Length & Timing
            trail_length_seconds: 60.0,
            update_interval_seconds: 1.0 / 30.0,
            max_points_per_trail: 10000,

            // Visual Appearance
            base_width: 1.0,
            width_relative_to_body: true,
            body_size_multiplier: 2.0,

            // Fading & Transparency
            enable_fading: true,
            fade_curve: FadeCurve::Exponential,
            min_alpha: 0.0,
            max_alpha: 0.3333,

            // Width Tapering
            enable_tapering: true,
            taper_curve: TaperCurve::Linear,
            min_width_ratio: 0.2,

            // Bloom Effect
            bloom_factor: 1.0,
            use_additive_blending: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum FadeCurve {
    Linear,
    Exponential,
    SmoothStep,
    EaseInOut,
}

impl Default for FadeCurve {
    fn default() -> Self {
        FadeCurve::Exponential
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TaperCurve {
    Linear,
    Exponential,
    SmoothStep,
}

impl Default for TaperCurve {
    fn default() -> Self {
        TaperCurve::Linear
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ScreenshotConfig {
    pub directory: Option<String>,
    pub filename_prefix: String,
    pub include_timestamp: bool,
    pub notification_enabled: bool,
    pub hide_ui_frame_delay: u32,
}

impl Default for ScreenshotConfig {
    fn default() -> Self {
        Self {
            directory: None,
            filename_prefix: "stardrift_screenshot".to_string(),
            include_timestamp: true,
            notification_enabled: true,
            hide_ui_frame_delay: 2,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SystemConfig {
    pub prevent_screen_sleep: bool,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            prevent_screen_sleep: true,
        }
    }
}

impl SimulationConfig {
    fn get_config_path() -> Result<PathBuf, ConfigError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(proj_dirs) = ProjectDirs::from("", "", "Stardrift") {
                let config_dir = proj_dirs.config_dir();
                std::fs::create_dir_all(config_dir).map_err(|e| {
                    ConfigError::Message(format!("Failed to create config dir: {e}"))
                })?;
                Ok(config_dir.join("config.toml"))
            } else {
                Err(ConfigError::Message(
                    "Failed to determine config directory".into(),
                ))
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            Err(ConfigError::Message(
                "Config not supported on WebAssembly".into(),
            ))
        }
    }

    fn load_config_with_source(source: File<config::FileSourceFile, config::FileFormat>) -> Self {
        let config_result = Config::builder()
            .add_source(config::File::from_str(
                &toml::to_string(&Self::default()).unwrap(),
                config::FileFormat::Toml,
            ))
            .add_source(source)
            .build();

        match config_result {
            Ok(config) => match config.try_deserialize::<Self>() {
                Ok(sim_config) => {
                    info!("Configuration loaded successfully");
                    sim_config
                }
                Err(e) => {
                    warn!("Failed to deserialize config: {}. Using defaults.", e);
                    Self::default()
                }
            },
            Err(e) => {
                warn!("Failed to load config: {}. Using defaults.", e);
                Self::default()
            }
        }
    }

    pub fn load_from_user_config() -> Self {
        match Self::get_config_path() {
            Ok(path) => {
                info!("Using configuration path: {}", path.display());
                let file_exists = path.exists();
                if !file_exists {
                    warn!("Configuration file not found, will use defaults");
                }
                Self::load_config_with_source(File::from(path).required(false))
            }
            Err(e) => {
                warn!("Failed to determine configuration path: {}", e);
                Self::default()
            }
        }
    }

    pub fn load_or_default(path: &str) -> Self {
        info!("Attempting to load configuration from: {}", path);
        match std::fs::read_to_string(path) {
            Ok(content) => {
                info!("Configuration file exists and was read successfully");
                match toml::from_str::<toml::Value>(&content) {
                    Ok(_) => Self::load_config_with_source(File::with_name(path).required(false)),
                    Err(e) => {
                        warn!("Failed to parse config file {path}: {e}. Using defaults.",);
                        Self::default()
                    }
                }
            }
            Err(_) => {
                warn!("Config file {path} not found. Using defaults.");
                Self::default()
            }
        }
    }

    #[allow(dead_code)]
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    #[allow(dead_code)]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_to_user_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path =
            Self::get_config_path().map_err(|e| format!("Failed to get config path: {e}"))?;

        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    #[allow(dead_code)]
    #[cfg(target_arch = "wasm32")]
    pub fn save_to_user_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        Err("Configuration saving not supported on WebAssembly".into())
    }
}
