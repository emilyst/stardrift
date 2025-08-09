//! Component factory functions for simulation bodies

use crate::config::SimulationConfig;
use crate::physics::math::{min_sphere_radius_for_surface_distribution, random_unit_vector};
use crate::prelude::*;
use bevy::render::mesh::SphereKind;
use rand::prelude::*;

/// Factory functions for creating celestial body components.
pub mod factory {
    use super::*;

    /// Generates a random position for a celestial body within the distribution sphere.
    pub fn random_position(
        rng: &mut SharedRng,
        total_body_count: usize,
        config: &SimulationConfig,
    ) -> Vec3 {
        let body_distribution_sphere_radius = min_sphere_radius_for_surface_distribution(
            total_body_count,
            config.physics.body_distribution_sphere_radius_multiplier,
            config.physics.body_distribution_min_distance,
        );
        random_unit_vector(rng) * body_distribution_sphere_radius
    }

    /// Generates a random radius for a celestial body within configured bounds.
    pub fn random_radius(rng: &mut SharedRng, config: &SimulationConfig) -> f32 {
        rng.random_range(config.physics.min_body_radius..=config.physics.max_body_radius)
    }

    /// Calculates temperature based on radius using inverse relationship.
    pub fn calculate_temperature(radius: f32, config: &SimulationConfig) -> f32 {
        let min_temp = config.rendering.min_temperature;
        let max_temp = config.rendering.max_temperature;
        let min_radius = config.physics.min_body_radius;
        let max_radius = config.physics.max_body_radius;

        min_temp + (max_temp - min_temp) * (max_radius - radius) / (max_radius - min_radius)
    }

    /// Generates a random initial velocity based on configuration.
    pub fn random_velocity(rng: &mut SharedRng, position: Vec3, config: &SimulationConfig) -> Vec3 {
        use crate::config::VelocityMode;

        if !config.physics.initial_velocity.enabled {
            return Vec3::ZERO;
        }

        let speed = rng.random_range(
            config.physics.initial_velocity.min_speed..=config.physics.initial_velocity.max_speed,
        );

        let velocity_dir = match config.physics.initial_velocity.velocity_mode {
            VelocityMode::Random => {
                // Pure random direction
                random_unit_vector(rng)
            }
            VelocityMode::Orbital => {
                // Perpendicular to position vector (circular orbit tendency)
                let up = Vec3::Y;
                let tangent = position.cross(up).normalize();
                if tangent.is_finite() {
                    tangent
                } else {
                    // Fallback if position is parallel to up
                    position.cross(Vec3::X).normalize()
                }
            }
            VelocityMode::Tangential => {
                // Mix of random and orbital
                let random_dir = random_unit_vector(rng);
                let up = Vec3::Y;
                let tangent = position.cross(up).normalize();
                let tangent = if tangent.is_finite() {
                    tangent
                } else {
                    position.cross(Vec3::X).normalize()
                };

                let bias = config.physics.initial_velocity.tangential_bias as f32;
                (tangent * bias + random_dir * (1.0 - bias)).normalize()
            }
            VelocityMode::Radial => {
                // Away from or towards center
                if rng.random_bool(0.5) {
                    position.normalize()
                } else {
                    -position.normalize()
                }
            }
        };

        velocity_dir * (speed as f32)
    }

    /// Creates a detailed mesh for a celestial body with high-quality subdivisions.
    pub fn create_detailed_mesh(meshes: &mut Assets<Mesh>, radius: f32) -> Handle<Mesh> {
        meshes.add(
            Sphere::new(radius)
                .mesh()
                .kind(SphereKind::Ico {
                    subdivisions: if cfg!(target_arch = "wasm32") { 1 } else { 4 },
                })
                .build(),
        )
    }
}
