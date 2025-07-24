use crate::prelude::*;
use config::{Config, ConfigError, File};
#[cfg(not(target_arch = "wasm32"))]
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
pub struct SimulationConfig {
    pub version: u32,
    pub physics: PhysicsConfig,
    pub rendering: RenderingConfig,
    #[cfg(feature = "trails")]
    pub trails: TrailConfig,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            version: 5,
            physics: PhysicsConfig::default(),
            rendering: RenderingConfig::default(),
            #[cfg(feature = "trails")]
            trails: TrailConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PhysicsConfig {
    pub gravitational_constant: Scalar,
    pub body_count: usize,
    pub octree_theta: Scalar,
    pub octree_leaf_threshold: usize,
    pub body_distribution_sphere_radius_multiplier: Scalar,
    pub body_distribution_min_distance: Scalar,
    pub min_body_radius: Scalar,
    pub max_body_radius: Scalar,
    pub force_calculation_min_distance: Scalar,
    pub force_calculation_max_force: Scalar,
    pub initial_seed: Option<u64>,
    pub collision_restitution: Scalar,
    pub collision_friction: Scalar,
    pub initial_velocity: InitialVelocityConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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
            max_speed: 20.0,
            velocity_mode: VelocityMode::Random,
            tangential_bias: 0.7,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum VelocityMode {
    Random,
    Orbital,
    Tangential,
    Radial,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravitational_constant: 2e2,
            body_count: 10,
            octree_theta: 0.0,
            octree_leaf_threshold: 1,
            body_distribution_sphere_radius_multiplier: 100.0,
            body_distribution_min_distance: 0.001,
            min_body_radius: 1.0,
            max_body_radius: 2.0,
            force_calculation_min_distance: 2.0,
            force_calculation_max_force: 1e5,
            initial_seed: None,
            collision_restitution: 0.8,
            collision_friction: 0.5,
            initial_velocity: InitialVelocityConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RenderingConfig {
    pub min_temperature: Scalar,
    pub max_temperature: Scalar,
    pub bloom_intensity: Scalar,
    pub saturation_intensity: Scalar,
    pub camera_radius_multiplier: Scalar,
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            min_temperature: 3000.0,
            max_temperature: 15000.0,
            bloom_intensity: 100.0,
            saturation_intensity: 3.0,
            camera_radius_multiplier: 4.0,
        }
    }
}

#[cfg(feature = "trails")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrailConfig {
    // Length & Timing
    pub trail_length_seconds: Scalar,
    pub update_interval_seconds: Scalar,
    pub max_points_per_trail: usize,

    // Visual Appearance
    pub base_width: Scalar,
    pub width_relative_to_body: bool,
    pub body_size_multiplier: Scalar,

    // Fading & Transparency
    pub enable_fading: bool,
    pub fade_curve: FadeCurve,
    pub min_alpha: Scalar,
    pub max_alpha: Scalar,

    // Width Tapering
    pub enable_tapering: bool,
    pub taper_curve: TaperCurve,
    pub min_width_ratio: Scalar,

    // Bloom Effect
    pub bloom_factor: Scalar,
    pub use_additive_blending: bool,
}

#[cfg(feature = "trails")]
impl Default for TrailConfig {
    fn default() -> Self {
        Self {
            trail_length_seconds: 60.0,          // 60 second trails
            update_interval_seconds: 1.0 / 30.0, // 30 FPS updates (current)
            max_points_per_trail: 10000,         // Reasonable limit
            base_width: 1.0,                     // Matches current behavior
            width_relative_to_body: false,       // Start with absolute sizing
            body_size_multiplier: 2.0,           // 2x body radius when enabled
            enable_fading: true,                 // Enable trail fade-out effect
            fade_curve: FadeCurve::Exponential,  // Aggressively fade out
            min_alpha: 0.0,                      // Fully transparent at tail
            max_alpha: 1.0,                      // Fully opaque at head
            enable_tapering: true,               // Taper by default
            taper_curve: TaperCurve::Linear,     // Simple tapering
            min_width_ratio: 0.2,                // Tail is 20% of base width
            bloom_factor: 1.0,                   // Disable bloom by default
            use_additive_blending: true,         // Use additive blending by default
        }
    }
}

#[cfg(feature = "trails")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FadeCurve {
    Linear,
    Exponential,
    SmoothStep,
    EaseInOut,
}

#[cfg(feature = "trails")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TaperCurve {
    Linear,
    Exponential,
    SmoothStep,
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
                    if sim_config.version < Self::default().version {
                        warn!(
                            "Config version {} is outdated. Using defaults.",
                            sim_config.version
                        );
                        warn!("Using default configuration due to outdated version");
                        Self::default()
                    } else {
                        info!("Configuration loaded successfully");
                        sim_config
                    }
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
                    Ok(value) => {
                        if value.get("version").is_none() {
                            warn!("Config file {path} missing version field. Using defaults.");
                            Self::default()
                        } else {
                            Self::load_config_with_source(File::with_name(path).required(false))
                        }
                    }
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

    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_to_user_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path =
            Self::get_config_path().map_err(|e| format!("Failed to get config path: {e}"))?;

        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

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
        assert!(path_str.contains("Stardrift"));
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

        let mut config = SimulationConfig {
            version: 5,
            ..Default::default()
        };
        config.physics.gravitational_constant = 42.0;
        config.physics.body_count = 123;
        config.rendering.bloom_intensity = 999.0;

        let temp_path = "test_config_temp.toml";
        config.save(temp_path).expect("Failed to save test config");

        let loaded_config = SimulationConfig::load_or_default(temp_path);

        assert_eq!(loaded_config.version, 5);
        assert_eq!(loaded_config.physics.gravitational_constant, 42.0);
        assert_eq!(loaded_config.physics.body_count, 123);
        assert_eq!(loaded_config.rendering.bloom_intensity, 999.0);

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_config_without_version_ignored() {
        use std::fs;

        // Create a config file without version field
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

        // Should use defaults since version is missing
        let default_config = SimulationConfig::default();
        assert_eq!(loaded_config.version, default_config.version);
        assert_eq!(
            loaded_config.physics.gravitational_constant,
            default_config.physics.gravitational_constant
        );
        assert_eq!(
            loaded_config.physics.body_count,
            default_config.physics.body_count
        );
        assert_eq!(
            loaded_config.rendering.bloom_intensity,
            default_config.rendering.bloom_intensity
        );

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_config_with_version_loaded() {
        use std::fs;

        // Create a config file with outdated version (4 < 5)
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

        // Should use defaults since version 4 is less than current version 5
        let default_config = SimulationConfig::default();
        assert_eq!(loaded_config.version, default_config.version);
        assert_eq!(
            loaded_config.physics.gravitational_constant,
            default_config.physics.gravitational_constant
        );
        assert_eq!(
            loaded_config.physics.body_count,
            default_config.physics.body_count
        );
        assert_eq!(
            loaded_config.physics.octree_theta,
            default_config.physics.octree_theta
        );
        assert_eq!(
            loaded_config.physics.octree_leaf_threshold,
            default_config.physics.octree_leaf_threshold
        );
        assert_eq!(
            loaded_config.rendering.bloom_intensity,
            default_config.rendering.bloom_intensity
        );

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_config_with_version_less_than_default_ignored() {
        use std::fs;

        // Create a config file with version 0 (less than default 1)
        let config_content = r#"version = 0

[physics]
gravitational_constant = 99.0
body_count = 999

[rendering]
bloom_intensity = 888.0
"#;

        let temp_path = "test_config_version_zero.toml";
        fs::write(temp_path, config_content).expect("Failed to write test config");

        let loaded_config = SimulationConfig::load_or_default(temp_path);

        // Should use defaults since version is less than 1
        let default_config = SimulationConfig::default();
        assert_eq!(loaded_config.version, default_config.version);
        assert_eq!(
            loaded_config.physics.gravitational_constant,
            default_config.physics.gravitational_constant
        );
        assert_eq!(
            loaded_config.physics.body_count,
            default_config.physics.body_count
        );
        assert_eq!(
            loaded_config.rendering.bloom_intensity,
            default_config.rendering.bloom_intensity
        );

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    #[cfg(feature = "trails")]
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
}
