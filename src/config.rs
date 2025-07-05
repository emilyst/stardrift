use avian3d::math::Scalar;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use xdg::BaseDirectories;

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
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravitational_constant: 1e1,
            body_count: 100,
            octree_theta: 0.5,
            octree_leaf_threshold: 4,
            body_distribution_sphere_radius_multiplier: 200.0,
            body_distribution_min_distance: 0.001,
            min_body_radius: 5.0,
            max_body_radius: 10.0,
            force_calculation_min_distance: 10.0,
            force_calculation_max_force: 1e4,
            initial_seed: None,
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
            bloom_intensity: 100.0,
            saturation_intensity: 3.0,
            camera_radius_multiplier: 2.0,
        }
    }
}

impl SimulationConfig {
    fn get_xdg_config_path() -> PathBuf {
        BaseDirectories::with_prefix("stardrift")
            .place_config_file("config.toml")
            .unwrap_or_else(|_| PathBuf::from(".").join("stardrift.toml"))
    }

    pub fn load_from_user_config() -> Self {
        let config_path = Self::get_xdg_config_path();
        Self::load_or_default(config_path.to_string_lossy().as_ref())
    }

    pub fn load_or_default(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<Self>(&content) {
                Ok(config) => {
                    if config.version < SimulationConfig::default().version {
                        info!(
                            "Config file {} has version {} which is outdated. Ignoring and using defaults.",
                            path, config.version
                        );
                        Self::default()
                    } else {
                        config
                    }
                }
                Err(e) => {
                    // Check if the error is due to missing version field
                    if e.to_string().contains("missing field `version`") {
                        info!(
                            "Config file {} missing version field. Ignoring and using defaults.",
                            path
                        );
                    } else {
                        warn!(
                            "Failed to parse config file {}: {}. Using defaults.",
                            path, e
                        );
                    }
                    Self::default()
                }
            },
            Err(_) => {
                info!("Config file {} not found. Using defaults.", path);
                Self::default()
            }
        }
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn save_to_user_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::get_xdg_config_path();

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        self.save(config_path.to_string_lossy().as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xdg_config_path_structure() {
        let path = SimulationConfig::get_xdg_config_path();
        let path_str = path.to_string_lossy();

        assert!(path_str.ends_with("stardrift/config.toml"));
        assert!(path_str.contains(".config") || path_str.starts_with("/"));
    }

    #[test]
    fn test_save_and_load_config() {
        use std::fs;

        let mut config = SimulationConfig::default();
        config.version = 2;
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
