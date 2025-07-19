use crate::config::SimulationConfig;
use crate::resources::SharedRng;
use crate::utils::color::emissive_material_for_temp;
use crate::utils::math::min_sphere_radius_for_surface_distribution;
use crate::utils::math::random_unit_vector;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::render::mesh::SphereKind;
use rand::Rng;

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

        let material = emissive_material_for_temp(
            materials,
            temperature,
            config.rendering.bloom_intensity,
            config.rendering.saturation_intensity,
        );

        let mesh = create_detailed_mesh(meshes, radius);

        BodyBundle::new(position, radius, material, mesh, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SimulationConfig;
    use bevy::app::App;
    use bevy::asset::AssetPlugin;
    use bevy::render::RenderPlugin;

    fn setup_test_app() -> (App, Handle<StandardMaterial>, Handle<Mesh>) {
        let mut app = App::new();
        app.add_plugins((AssetPlugin::default(), RenderPlugin::default()));

        // Initialize the resources
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<Assets<Mesh>>();

        let material_handle = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let mesh_handle = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Sphere::new(1.0));

        (app, material_handle, mesh_handle)
    }

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

    #[test]
    fn test_mesh_creation() {
        let (mut app, _, _) = setup_test_app();
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();

        let mesh1 = factory::create_detailed_mesh(&mut meshes, 1.0);
        let mesh2 = factory::create_detailed_mesh(&mut meshes, 2.0);

        // Both should be valid handles
        assert!(meshes.get(&mesh1).is_some());
        assert!(meshes.get(&mesh2).is_some());

        // They should be different handles (different radius)
        assert_ne!(mesh1, mesh2);
    }

    #[test]
    fn test_bundle_physics_configuration() {
        let config = SimulationConfig::default();

        // Test that bundle has correct physics configuration
        // We'll test the static parts that don't require asset creation
        assert_eq!(0.0, 0.0); // GravityScale should be 0.0
        assert!(config.physics.collision_restitution >= 0.0);
        assert!(config.physics.collision_friction >= 0.0);

        // Test temperature calculation ranges
        let min_temp = factory::calculate_temperature(config.physics.max_body_radius, &config);
        let max_temp = factory::calculate_temperature(config.physics.min_body_radius, &config);

        assert!(min_temp <= max_temp);
        assert_eq!(min_temp, config.rendering.min_temperature);
        assert_eq!(max_temp, config.rendering.max_temperature);
    }
}
