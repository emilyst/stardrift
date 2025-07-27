//! Components for the simulation plugin.
//!
//! This module contains components that are specific to the physics simulation,
//! following the self-contained plugin pattern.

use crate::prelude::*;
use crate::utils::color::emissive_material_for_temp;
use crate::utils::math::{min_sphere_radius_for_surface_distribution, random_unit_vector};
use bevy::render::mesh::SphereKind;
use rand::prelude::*;

/// Bundle containing all components needed for a celestial body.
///
/// This bundle follows idiomatic Bevy patterns by focusing solely on component organization.
/// Asset creation and complex calculations are handled by separate factory functions that
/// can properly utilize system resources and existing utilities.
#[derive(Bundle)]
pub struct BodyBundle {
    pub transform: Transform,
    pub collider: Collider,
    pub gravity_scale: GravityScale,
    pub rigid_body: RigidBody,
    pub external_force: ExternalForce,
    pub linear_velocity: LinearVelocity,
    pub restitution: Restitution,
    pub friction: Friction,
    pub mesh_material: MeshMaterial3d<StandardMaterial>,
    pub mesh: Mesh3d,
}

impl BodyBundle {
    /// Creates a new celestial body bundle with the provided components.
    ///
    /// This constructor follows idiomatic Bevy patterns by accepting pre-created
    /// components rather than performing complex calculations or asset creation.
    /// Use the factory functions in this module to create the necessary components.
    pub fn new(
        position: Vec3,
        radius: f64,
        velocity: Vec3,
        material: Handle<StandardMaterial>,
        mesh: Handle<Mesh>,
        config: &SimulationConfig,
    ) -> Self {
        Self {
            transform: Transform::from_translation(position),
            collider: Collider::sphere(radius),
            gravity_scale: GravityScale(0.0),
            rigid_body: RigidBody::Dynamic,
            external_force: ExternalForce::ZERO,
            linear_velocity: LinearVelocity(velocity.as_dvec3()),
            restitution: Restitution::new(config.physics.collision_restitution),
            friction: Friction::new(config.physics.collision_friction),
            mesh_material: MeshMaterial3d(material),
            mesh: Mesh3d(mesh),
        }
    }
}

/// Factory functions for creating celestial body components.
///
/// These functions are designed to be used in Bevy systems where proper resource
/// access is available. They utilize existing utilities and follow idiomatic patterns.
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
        let position = random_unit_vector(rng) * body_distribution_sphere_radius;
        position.as_vec3()
    }

    /// Generates a random radius for a celestial body within configured bounds.
    pub fn random_radius(rng: &mut SharedRng, config: &SimulationConfig) -> f64 {
        rng.random_range(config.physics.min_body_radius..=config.physics.max_body_radius)
    }

    /// Calculates temperature based on radius using inverse relationship.
    pub fn calculate_temperature(radius: f64, config: &SimulationConfig) -> f64 {
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
                random_unit_vector(rng).as_vec3()
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
                let random_dir = random_unit_vector(rng).as_vec3();
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
    pub fn create_detailed_mesh(meshes: &mut Assets<Mesh>, radius: f64) -> Handle<Mesh> {
        meshes.add(
            Sphere::new(radius as f32)
                .mesh()
                .kind(SphereKind::Ico {
                    subdivisions: if cfg!(target_arch = "wasm32") { 1 } else { 4 },
                })
                .build(),
        )
    }

    /// Creates a celestial body bundle using random generation and proper utilities.
    ///
    /// This function demonstrates the idiomatic way to create bodies in systems,
    /// using existing utilities and proper resource access patterns.
    pub fn create_random_body(
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<StandardMaterial>>,
        rng: &mut ResMut<SharedRng>,
        config: &SimulationConfig,
        total_body_count: usize,
    ) -> BodyBundle {
        let position = random_position(rng, total_body_count, config);
        let radius = random_radius(rng, config);
        let temperature = calculate_temperature(radius, config);
        let velocity = random_velocity(rng, position, config);

        let material = emissive_material_for_temp(
            materials,
            temperature,
            config.rendering.bloom_intensity,
            config.rendering.saturation_intensity,
        );

        let mesh = create_detailed_mesh(meshes, radius);

        BodyBundle::new(position, radius, velocity, material, mesh, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SimulationConfig;

    #[test]
    fn test_temperature_calculation() {
        let config = SimulationConfig::default();
        let min_radius = config.physics.min_body_radius;
        let max_radius = config.physics.max_body_radius;
        let min_temp = config.rendering.min_temperature;
        let max_temp = config.rendering.max_temperature;

        // Test minimum radius gives maximum temperature
        let temp_min_radius = factory::calculate_temperature(min_radius, &config);
        assert!((temp_min_radius - max_temp).abs() < f64::EPSILON);

        // Test maximum radius gives minimum temperature
        let temp_max_radius = factory::calculate_temperature(max_radius, &config);
        assert!((temp_max_radius - min_temp).abs() < f64::EPSILON);

        // Test middle radius gives middle temperature
        let mid_radius = (min_radius + max_radius) / 2.0;
        let temp_mid_radius = factory::calculate_temperature(mid_radius, &config);
        let expected_mid_temp = (min_temp + max_temp) / 2.0;
        assert!((temp_mid_radius - expected_mid_temp).abs() < 0.001);
    }
}
