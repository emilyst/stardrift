use avian3d::math::Scalar;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
pub struct SimulationConfig {
    pub physics: PhysicsConfig,
    pub rendering: RenderingConfig,
    pub ui: UiConfig,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            physics: PhysicsConfig::default(),
            rendering: RenderingConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PhysicsConfig {
    pub gravitational_constant: Scalar,
    pub default_body_count: usize,
    pub octree_theta: Scalar,
    pub body_distribution_sphere_radius_multiplier: Scalar,
    pub body_distribution_min_distance: Scalar,
    pub min_body_radius: Scalar,
    pub max_body_radius: Scalar,
    pub force_calculation_min_distance: Scalar,
    pub force_calculation_max_force: Scalar,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravitational_constant: 1e1,
            default_body_count: if cfg!(target_arch = "wasm32") {
                100
            } else {
                100
            },
            octree_theta: 0.5,
            body_distribution_sphere_radius_multiplier: 200.0,
            body_distribution_min_distance: 0.001,
            min_body_radius: 10.0,
            max_body_radius: 20.0,
            force_calculation_min_distance: 10.0,
            force_calculation_max_force: 1e4,
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
            // Fallback to current directory if HOME is not set
            PathBuf::from(".")
        };

        config_dir.join("many_body_simulation").join("config.toml")
    }

    /// Load configuration from the XDG config path, falling back to defaults if it doesn't exist
    pub fn load_from_user_config() -> Self {
        let config_path = Self::get_xdg_config_path();
        Self::load_or_default(config_path.to_string_lossy().as_ref())
    }

    /// Load configuration from a file, falling back to defaults if the file doesn't exist
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

    /// Save configuration to a file
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Save configuration to the XDG config path
    pub fn save_to_user_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::get_xdg_config_path();

        // Create the directory if it doesn't exist
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
    fn test_load_from_user_config() {
        // This should load defaults since the XDG config file likely doesn't exist
        let config = SimulationConfig::load_from_user_config();
        let default_config = SimulationConfig::default();

        // Verify that we get the default values
        assert_eq!(
            config.physics.gravitational_constant,
            default_config.physics.gravitational_constant
        );
        assert_eq!(
            config.physics.default_body_count,
            default_config.physics.default_body_count
        );
        assert_eq!(
            config.physics.octree_theta,
            default_config.physics.octree_theta
        );
        assert_eq!(
            config.rendering.min_temperature,
            default_config.rendering.min_temperature
        );
        assert_eq!(
            config.rendering.max_temperature,
            default_config.rendering.max_temperature
        );
    }

    #[test]
    fn test_xdg_config_path_structure() {
        // Test that the path structure is correct
        let path = SimulationConfig::get_xdg_config_path();
        let path_str = path.to_string_lossy();

        // Should end with the correct application directory and filename
        assert!(path_str.ends_with("many_body_simulation/config.toml"));

        // Should contain either .config (HOME fallback) or be an absolute path (XDG_CONFIG_HOME)
        assert!(path_str.contains(".config") || path_str.starts_with("/"));
    }
}
