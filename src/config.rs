use crate::prelude::*;
use config::{Config, ConfigError, File};
#[cfg(not(target_arch = "wasm32"))]
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct SimulationConfig {
    pub physics: PhysicsConfig,
    pub rendering: RenderingConfig,
    pub trails: TrailConfig,
    pub screenshots: ScreenshotConfig,
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
            min_speed: 50.0,
            max_speed: 100.0,
            velocity_mode: VelocityMode::Random,
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

/// Flexible integrator configuration
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct IntegratorConfig {
    /// Type of integrator (e.g., "velocity_verlet")
    #[serde(rename = "type")]
    pub integrator_type: String,

    /// Integrator-specific parameters
    #[serde(default)]
    pub params: crate::physics::integrators::registry::IntegratorParams,
}

impl Default for IntegratorConfig {
    fn default() -> Self {
        Self {
            integrator_type: "velocity_verlet".to_string(),
            params: Default::default(),
        }
    }
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravitational_constant: 0.001,
            body_count: 100,
            octree_theta: 0.5,
            octree_leaf_threshold: 1,
            body_distribution_sphere_radius_multiplier: 100.0,
            body_distribution_min_distance: 0.001,
            min_body_radius: 2.0,
            max_body_radius: 4.0,
            force_calculation_min_distance: 1.0,
            force_calculation_max_force: 1e6,
            initial_seed: None,
            initial_velocity: InitialVelocityConfig::default(),
            integrator: IntegratorConfig::default(),
            barycentric_drift_correction: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct RenderingConfig {
    pub min_temperature: f32,
    pub max_temperature: f32,
    pub bloom_intensity: f32,
    pub saturation_intensity: f32,
    pub camera_radius_multiplier: f32,
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            min_temperature: 3000.0,
            max_temperature: 15000.0,
            bloom_intensity: 250.0,
            saturation_intensity: 3.0,
            camera_radius_multiplier: 4.0,
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
            trail_length_seconds: 10.0,          // 10 second trails
            update_interval_seconds: 1.0 / 30.0, // 30 FPS updates
            max_points_per_trail: 10000,         // Reasonable limit
            base_width: 1.0,                     // Matches current behavior
            width_relative_to_body: true,        // Start with relative sizing
            body_size_multiplier: 2.0,           // 2x body radius when enabled
            enable_fading: true,                 // Enable trail fade-out effect
            fade_curve: FadeCurve::Exponential,  // Aggressively fade out
            min_alpha: 0.0,                      // Fully transparent at tail
            max_alpha: 0.3333,                   // Translucent at head
            enable_tapering: true,               // Taper by default
            taper_curve: TaperCurve::Linear,     // Linear tapering
            min_width_ratio: 0.2,                // Tail is 20% of base width
            bloom_factor: 1.0,                   // Disable bloom by default
            use_additive_blending: true,         // Use additive blending by default
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TaperCurve {
    Linear,
    Exponential,
    SmoothStep,
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
            directory: None, // None means current working directory
            filename_prefix: "stardrift_screenshot".to_string(),
            include_timestamp: true,
            notification_enabled: true,
            hide_ui_frame_delay: 2,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_config_path_structure() {
        let path = SimulationConfig::get_config_path();
        let binding = path.unwrap();
        let path_str = binding.to_string_lossy();

        assert!(path_str.ends_with("config.toml"));
        // On different platforms, the directory name might be lowercased or modified
        // Check that it contains either "Stardrift" or "stardrift"
        let path_lower = path_str.to_lowercase();
        assert!(
            path_lower.contains("stardrift"),
            "Config path should contain 'stardrift' (case-insensitive): {path_str}"
        );
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_wasm_config_path_error() {
        let path_result = SimulationConfig::get_config_path();
        assert!(path_result.is_err());

        let error_message = path_result.unwrap_err().to_string();
        assert!(error_message.contains("Config not supported on WebAssembly"));
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_wasm_save_to_user_config_error() {
        let config = SimulationConfig::default();
        let save_result = config.save_to_user_config();

        assert!(save_result.is_err());
        let error_message = save_result.unwrap_err().to_string();
        assert!(error_message.contains("Configuration saving not supported on WebAssembly"));
    }

    #[test]
    fn test_save_and_load_config() {
        use std::fs;

        let mut config = SimulationConfig::default();
        config.physics.gravitational_constant = 42.0;
        config.physics.body_count = 123;
        config.rendering.bloom_intensity = 999.0;

        let temp_path = "test_config_temp.toml";
        config.save(temp_path).expect("Failed to save test config");

        let loaded_config = SimulationConfig::load_or_default(temp_path);

        assert_eq!(loaded_config.physics.gravitational_constant, 42.0);
        assert_eq!(loaded_config.physics.body_count, 123);
        assert_eq!(loaded_config.rendering.bloom_intensity, 999.0);

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_config_without_version_loads_correctly() {
        use std::fs;

        // Create a config file without version field (should load correctly now)
        let config_content = r#"
[physics]
gravitational_constant = 99.0
body_count = 999

[rendering]
bloom_intensity = 888.0
"#;

        let temp_path = "test_config_no_version.toml";
        fs::write(temp_path, config_content).expect("Failed to write test config");

        let loaded_config = SimulationConfig::load_or_default(temp_path);

        // Should load the custom values now
        assert_eq!(loaded_config.physics.gravitational_constant, 99.0);
        assert_eq!(loaded_config.physics.body_count, 999);
        assert_eq!(loaded_config.rendering.bloom_intensity, 888.0);

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_config_with_legacy_version_field() {
        use std::fs;

        // Create a config file with legacy version field (should be ignored but still load)
        let config_content = r#"version = 4

[physics]
gravitational_constant = 99.0
body_count = 999
octree_theta = 0.7
octree_leaf_threshold = 8
body_distribution_sphere_radius_multiplier = 250.0
body_distribution_min_distance = 0.002
min_body_radius = 6.0
max_body_radius = 12.0
force_calculation_min_distance = 15.0
force_calculation_max_force = 2000.0

[rendering]
min_temperature = 3000.0
max_temperature = 12000.0
bloom_intensity = 888.0
saturation_intensity = 4.0
camera_radius_multiplier = 3.0
"#;

        let temp_path = "test_config_with_version.toml";
        fs::write(temp_path, config_content).expect("Failed to write test config");

        let loaded_config = SimulationConfig::load_or_default(temp_path);

        // Should load the custom values even with legacy version field
        assert_eq!(loaded_config.physics.gravitational_constant, 99.0);
        assert_eq!(loaded_config.physics.body_count, 999);
        assert_eq!(loaded_config.physics.octree_theta, 0.7);
        assert_eq!(loaded_config.physics.octree_leaf_threshold, 8);
        assert_eq!(loaded_config.rendering.bloom_intensity, 888.0);

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_trail_config_serialization() {
        use std::fs;

        let mut config = SimulationConfig::default();
        config.trails.trail_length_seconds = 15.0;
        config.trails.base_width = 2.5;
        config.trails.enable_tapering = true;
        config.trails.fade_curve = FadeCurve::Exponential;

        let temp_path = "test_trail_config.toml";
        config
            .save(temp_path)
            .expect("Failed to save trail config test");

        let loaded_config = SimulationConfig::load_or_default(temp_path);

        assert_eq!(loaded_config.trails.trail_length_seconds, 15.0);
        assert_eq!(loaded_config.trails.base_width, 2.5);
        assert!(loaded_config.trails.enable_tapering);
        assert!(matches!(
            loaded_config.trails.fade_curve,
            FadeCurve::Exponential
        ));

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_snake_case_enum_parsing() {
        use std::fs;

        // Test that snake_case values work for all enums
        let config_content = r#"

[physics]
gravitational_constant = 500.0
body_count = 100
octree_theta = 0.5
octree_leaf_threshold = 2
body_distribution_sphere_radius_multiplier = 100.0
body_distribution_min_distance = 0.001
min_body_radius = 1.0
max_body_radius = 2.0
force_calculation_min_distance = 2.0
force_calculation_max_force = 10000.0

[physics.integrator]
type = "symplectic_euler"

[physics.initial_velocity]
enabled = true
min_speed = 5.0
max_speed = 20.0
velocity_mode = "tangential"
tangential_bias = 0.7

[rendering]
min_temperature = 3000.0
max_temperature = 15000.0
bloom_intensity = 250.0
saturation_intensity = 3.0
camera_radius_multiplier = 4.0

[trails]
trail_length_seconds = 10.0
update_interval_seconds = 0.033333333
max_points_per_trail = 10000
base_width = 1.0
width_relative_to_body = true
body_size_multiplier = 2.0
enable_fading = true
fade_curve = "smooth_step"
min_alpha = 0.0
max_alpha = 0.3333
enable_tapering = true
taper_curve = "exponential"
min_width_ratio = 0.2
bloom_factor = 1.0
use_additive_blending = true

[screenshots]
filename_prefix = "stardrift_screenshot"
include_timestamp = true
notification_enabled = true
hide_ui_frame_delay = 2
"#;

        let temp_path = "test_snake_case_enums.toml";
        fs::write(temp_path, config_content).expect("Failed to write test config");

        let loaded_config = SimulationConfig::load_or_default(temp_path);

        // Verify the config loaded correctly with snake_case values
        assert_eq!(
            loaded_config.physics.integrator.integrator_type,
            "symplectic_euler"
        );
        assert!(matches!(
            loaded_config.physics.initial_velocity.velocity_mode,
            VelocityMode::Tangential
        ));
        assert!(matches!(
            loaded_config.trails.fade_curve,
            FadeCurve::SmoothStep
        ));
        assert!(matches!(
            loaded_config.trails.taper_curve,
            TaperCurve::Exponential
        ));

        let _ = fs::remove_file(temp_path);
    }
}
