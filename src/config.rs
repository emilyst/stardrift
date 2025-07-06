use avian3d::math::Scalar;
use bevy::prelude::*;
use config::Config;
use config::ConfigError;
use config::File;
#[cfg(not(target_arch = "wasm32"))]
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
pub struct SimulationConfig {
    pub version: u32,
    pub physics: PhysicsConfig,
    pub rendering: RenderingConfig,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            version: 2,
            physics: PhysicsConfig::default(),
            rendering: RenderingConfig::default(),
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
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        if cfg!(target_arch = "wasm32") {
            Self {
                gravitational_constant: 1e1,
                body_count: 100,
                octree_theta: 1.0,
                octree_leaf_threshold: 4,
                body_distribution_sphere_radius_multiplier: 200.0,
                body_distribution_min_distance: 0.001,
                min_body_radius: 5.0,
                max_body_radius: 10.0,
                force_calculation_min_distance: 10.0,
                force_calculation_max_force: 1e4,
                initial_seed: None,
                collision_restitution: 0.8,
                collision_friction: 0.1,
            }
        } else {
            Self {
                gravitational_constant: 1e2,
                body_count: 1000,
                octree_theta: 2.0,
                octree_leaf_threshold: 4,
                body_distribution_sphere_radius_multiplier: 100.0,
                body_distribution_min_distance: 0.001,
                min_body_radius: 5.0,
                max_body_radius: 10.0,
                force_calculation_min_distance: 1.0,
                force_calculation_max_force: 1e6,
                initial_seed: None,
                collision_restitution: 0.8,
                collision_friction: 0.1,
            }
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
            min_temperature: 2000.0,
            max_temperature: 15000.0,
            bloom_intensity: 33.333,
            saturation_intensity: 3.0,
            camera_radius_multiplier: 2.0,
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
            version: 2,
            ..Default::default()
        };
        config.physics.gravitational_constant = 42.0;
        config.physics.body_count = 123;
        config.rendering.bloom_intensity = 999.0;

        let temp_path = "test_config_temp.toml";
        config.save(temp_path).expect("Failed to save test config");

        let loaded_config = SimulationConfig::load_or_default(temp_path);

        assert_eq!(loaded_config.version, 2);
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

        // Create a complete config file with version field
        let config_content = r#"version = 2

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

        // Should load the actual values since version is present
        assert_eq!(loaded_config.version, 2);
        assert_eq!(loaded_config.physics.gravitational_constant, 99.0);
        assert_eq!(loaded_config.physics.body_count, 999);
        assert_eq!(loaded_config.physics.octree_theta, 0.7);
        assert_eq!(loaded_config.physics.octree_leaf_threshold, 8);
        assert_eq!(loaded_config.rendering.bloom_intensity, 888.0);

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
}
