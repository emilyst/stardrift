use avian3d::math::Scalar;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravitational_constant: 1e1,
            default_body_count: if cfg!(target_arch = "wasm32") { 100 } else { 100 },
            octree_theta: 0.5,
            body_distribution_sphere_radius_multiplier: 200.0,
            body_distribution_min_distance: 0.001,
            min_body_radius: 10.0,
            max_body_radius: 20.0,
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
    /// Load configuration from a file, falling back to defaults if the file doesn't exist
    pub fn load_or_default(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    warn!("Failed to parse config file {}: {}. Using defaults.", path, e);
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
}