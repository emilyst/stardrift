use avian3d::math::Scalar;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct SimulationConfig {
    pub physics: PhysicsConfig,
    pub rendering: RenderingConfig,
    pub ui: UiConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PhysicsConfig {
    pub gravitational_constant: Scalar,
    pub body_count: usize,
    pub octree_theta: Scalar,
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
            body_count: if cfg!(target_arch = "wasm32") {
                100
            } else {
                1000
            },
            octree_theta: 0.5,
            body_distribution_sphere_radius_multiplier: 200.0,
            body_distribution_min_distance: 0.001,
            min_body_radius: 10.0,
            max_body_radius: 20.0,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UiConfig {
    pub button_padding: f32,
    pub button_gap: f32,
    pub button_margin: f32,
    pub button_border_radius: f32,
    pub font_size: f32,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            button_padding: 5.0,
            button_gap: 10.0,
            button_margin: 10.0,
            button_border_radius: 5.0,
            font_size: 12.0,
        }
    }
}

impl SimulationConfig {
    /// Get the XDG config directory path for the application
    fn get_xdg_config_path() -> PathBuf {
        let config_dir = if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_config_home)
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config")
        } else {
            PathBuf::from(".")
        };

        config_dir.join("stardrift").join("config.toml")
    }

    pub fn load_from_user_config() -> Self {
        let config_path = Self::get_xdg_config_path();
        Self::load_or_default(config_path.to_string_lossy().as_ref())
    }

    pub fn load_or_default(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    warn!(
                        "Failed to parse config file {}: {}. Using defaults.",
                        path, e
                    );
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
}
