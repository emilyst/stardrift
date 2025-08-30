use crate::config::SimulationConfig;
use crate::physics::integrators::AccelerationField;
use crate::physics::math::{Scalar, Vector};
use crate::physics::{
    components::{Mass, PhysicsBody, PhysicsBodyBundle, Position, Velocity},
    octree::{Octree, OctreeBody},
    resources::{CurrentIntegrator, PhysicsTime},
};
use crate::resources::{
    Barycenter, GravitationalConstant, GravitationalOctree, RenderingRng, SharedRng,
};
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::Mesh3d;
use bevy::prelude::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PhysicsSet {
    BuildOctree,
    IntegrateMotions,
    SyncTransforms,
    CorrectBarycentricDrift,
}

/// Rebuild the octree structure from current body positions
pub fn rebuild_octree(
    bodies: Query<(Entity, &Position, &Mass)>,
    mut octree: ResMut<GravitationalOctree>,
) {
    if bodies.is_empty() {
        return;
    }

    octree.build(bodies.iter().map(|(entity, position, mass)| OctreeBody {
        position: position.value(),
        mass: mass.value(),
        entity,
    }));
}

/// Acceleration field that wraps the octree for a specific body
///
/// This struct implements the AccelerationField trait to allow integrators
/// to calculate accelerations at arbitrary positions during multi-stage integration.
struct BodyAccelerationField<'a> {
    octree: &'a Octree,
    body_entity: Entity,
    body_mass: Scalar,
    g: Scalar,
}

impl<'a> AccelerationField for BodyAccelerationField<'a> {
    fn at(&self, position: Vector) -> Vector {
        let force = self.octree.calculate_force_at_position(
            position,
            self.body_mass,
            self.body_entity,
            self.g,
        );
        force / self.body_mass
    }
}

/// Integrate positions and velocities for all bodies
pub fn integrate_motions(
    mut query: Query<(Entity, &mut Position, &mut Velocity, &Mass)>,
    integrator: Res<CurrentIntegrator>,
    physics_time: Res<PhysicsTime>,
    octree: Res<GravitationalOctree>,
    g: Res<GravitationalConstant>,
) {
    if physics_time.is_paused() {
        return;
    }

    let dt = physics_time.dt;
    let octree: &Octree = &octree;

    query
        .par_iter_mut()
        .for_each(|(entity, mut position, mut velocity, mass)| {
            let field = BodyAccelerationField {
                octree,
                body_entity: entity,
                body_mass: mass.value(),
                g: **g,
            };

            integrator
                .0
                .step(position.value_mut(), velocity.value_mut(), &field, dt);
        });
}

/// Synchronize Transform components from high-precision Position components
pub fn sync_transform_from_position(
    mut query: Query<(&Position, &mut Transform), (With<PhysicsBody>, Changed<Position>)>,
    camera_query: Query<&Transform, (With<Camera>, Without<PhysicsBody>)>,
) {
    let camera_translation = camera_query
        .iter()
        .next()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    for (position, mut transform) in query.iter_mut() {
        let position_as_vec3 = position.value().as_vec3();
        let distance_to_camera = (position_as_vec3 - camera_translation).length();

        if position.needs_transform_update(&transform) || distance_to_camera < 100.0 {
            transform.translation = position_as_vec3;
        }
    }
}

/// Counteract barycentric drift to keep simulation centered
pub fn counteract_barycentric_drift(
    mut bodies: Query<(&mut Position, &Mass)>,
    mut barycenter: ResMut<Barycenter>,
    config: Res<SimulationConfig>,
) {
    let (weighted_positions, total_mass): (Vector, Scalar) = bodies
        .iter()
        .map(|(position, mass)| (position.value(), mass.value()))
        .fold((Vector::ZERO, 0.0), |(pos_acc, mass_acc), (pos, mass)| {
            (pos_acc + pos * mass, mass_acc + mass)
        });

    if total_mass.abs() <= Scalar::EPSILON {
        return;
    }

    let updated_barycenter = weighted_positions / total_mass;

    if !updated_barycenter.is_finite() {
        return;
    }

    let Some(previous_barycenter) = **barycenter else {
        **barycenter = Some(updated_barycenter);
        return;
    };

    if !config.physics.barycentric_drift_correction {
        **barycenter = Some(updated_barycenter);
        return;
    }

    let barycentric_drift = updated_barycenter - previous_barycenter;

    if barycentric_drift.length_squared().abs() <= Scalar::EPSILON {
        return;
    }

    // When correcting drift, we move bodies back so the barycenter stays at the previous position
    bodies.par_iter_mut().for_each(|(mut position, _)| {
        *position.value_mut() += -barycentric_drift;
    });

    // After correction, the barycenter remains at the previous position
    // So we don't update the stored barycenter value
}

/// Helper function to spawn bodies with the given parameters
pub fn spawn_bodies(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    physics_rng: &mut ResMut<SharedRng>,
    rendering_rng: &mut ResMut<RenderingRng>,
    body_count: usize,
    config: &SimulationConfig,
) {
    use super::components::factory;
    use crate::config::ColorScheme;
    use crate::utils::color::*;

    for _ in 0..body_count {
        // Use physics RNG for position, radius, and velocity (physics determinism)
        let position = factory::random_position(physics_rng, body_count, config);
        let radius = factory::random_radius(physics_rng, config);
        let velocity = factory::random_velocity(physics_rng, position, config);

        // Use rendering RNG for color generation (visual determinism, independent of physics)
        let color = match config.rendering.color_scheme {
            ColorScheme::BlackBody => {
                let temperature = factory::calculate_temperature(radius, config);
                rgb_for_temp(temperature)
            }
            ColorScheme::Rainbow => random_rainbow_color(rendering_rng),
            // Colorblind-safe palettes
            ColorScheme::DeuteranopiaSafe => deuteranopia_safe_color(rendering_rng),
            ColorScheme::ProtanopiaSafe => protanopia_safe_color(rendering_rng),
            ColorScheme::TritanopiaSafe => tritanopia_safe_color(rendering_rng),
            ColorScheme::HighContrast => high_contrast_color(rendering_rng),
            // Scientific colormaps
            ColorScheme::Viridis => viridis_color(rendering_rng),
            ColorScheme::Plasma => plasma_color(rendering_rng),
            ColorScheme::Inferno => inferno_color(rendering_rng),
            ColorScheme::Turbo => turbo_color(rendering_rng),
            // Aesthetic themes
            ColorScheme::Pastel => pastel_color(rendering_rng),
            ColorScheme::Neon => neon_color(rendering_rng),
            ColorScheme::Monochrome => monochrome_color(rendering_rng),
            ColorScheme::Vaporwave => vaporwave_color(rendering_rng),
            // Pride flag color schemes
            ColorScheme::Bisexual => bisexual_pride_color(rendering_rng),
            ColorScheme::Transgender => transgender_pride_color(rendering_rng),
            ColorScheme::Lesbian => lesbian_pride_color(rendering_rng),
            ColorScheme::Pansexual => pansexual_pride_color(rendering_rng),
            ColorScheme::Nonbinary => nonbinary_pride_color(rendering_rng),
            ColorScheme::Asexual => asexual_pride_color(rendering_rng),
            ColorScheme::Genderfluid => genderfluid_pride_color(rendering_rng),
            ColorScheme::Aromantic => aromantic_pride_color(rendering_rng),
            ColorScheme::Agender => agender_pride_color(rendering_rng),
        };

        // Create material from color (single API path)
        let material = create_emissive_material(
            materials,
            color,
            config.rendering.bloom_intensity,
            config.rendering.saturation_intensity,
        );

        let mesh = factory::create_detailed_mesh(meshes, radius);

        // Mass proportional to volume (r³) with default density
        let density = 1.0; // Default density, could be made configurable
        let mass = density * 4.0 / 3.0 * std::f32::consts::PI * radius.powi(3);

        commands.spawn((
            PhysicsBodyBundle::new(Vector::from(position), mass, radius, Vector::from(velocity)),
            MeshMaterial3d(material),
            Mesh3d(mesh),
        ));
    }
}

/// Bevy system to spawn simulation bodies at startup
pub fn spawn_simulation_bodies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut physics_rng: ResMut<SharedRng>,
    mut rendering_rng: ResMut<RenderingRng>,
    body_count: Res<crate::resources::BodyCount>,
    config: Res<SimulationConfig>,
) {
    spawn_bodies(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut physics_rng,
        &mut rendering_rng,
        **body_count,
        &config,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::integrators::{RungeKuttaFourthOrder, SymplecticEuler};
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn test_rk4_integration_works() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Use RK4 integrator
        app.insert_resource(CurrentIntegrator(Box::new(RungeKuttaFourthOrder)));
        app.insert_resource(PhysicsTime::default());
        app.insert_resource(GravitationalOctree(Octree::new(0.5, 0.01, 1e6)));
        app.insert_resource(GravitationalConstant(1.0));

        // Create a test body with known initial conditions
        let entity = app
            .world_mut()
            .spawn((
                Position::new(Vector::new(0.0, 10.0, 0.0)),
                Mass::new(1.0),
                Velocity::new(Vector::new(5.0, 0.0, 0.0)),
                PhysicsBody,
            ))
            .id();

        // Add a massive body below to generate downward force
        app.world_mut().spawn((
            Position::new(Vector::new(0.0, -1000.0, 0.0)),
            Mass::new(1e15), // Very massive to generate noticeable force
            Velocity::new(Vector::ZERO),
            PhysicsBody,
        ));

        // Add integration system (only one now)
        app.add_systems(Update, (rebuild_octree, integrate_motions).chain());

        // Store initial position for comparison
        let initial_pos = app
            .world()
            .entity(entity)
            .get::<Position>()
            .unwrap()
            .value();
        let initial_vel = app
            .world()
            .entity(entity)
            .get::<Velocity>()
            .unwrap()
            .value();

        // Run update to integrate motion
        app.update();

        // Check that position and velocity were updated (RK4 should work)
        let position = app.world().entity(entity).get::<Position>().unwrap();
        let velocity = app.world().entity(entity).get::<Velocity>().unwrap();

        // Verify motion happened
        assert_ne!(
            position.value(),
            initial_pos,
            "Position should have changed"
        );
        assert_ne!(
            velocity.value(),
            initial_vel,
            "Velocity should have changed"
        );

        // With gravity, y-velocity should decrease
        assert!(
            velocity.value().y < initial_vel.y,
            "Y-velocity should decrease due to gravity"
        );
    }

    #[test]
    fn test_rk4_vs_euler_difference() {
        use crate::physics::integrators::Integrator;

        // This test demonstrates that RK4 currently produces the same results as Euler
        // because the multi-stage system isn't implemented
        let dt = 1.0 / 60.0;
        let acceleration = Vector::new(0.0, -9.81, 0.0);

        // Test with Euler
        let mut euler_pos = Vector::new(0.0, 10.0, 0.0);
        let mut euler_vel = Vector::new(5.0, 0.0, 0.0);
        let euler_integrator = SymplecticEuler;
        // Simple test acceleration field that returns constant acceleration
        struct TestAccelerationField {
            acceleration: Vector,
        }
        impl AccelerationField for TestAccelerationField {
            fn at(&self, _: Vector) -> Vector {
                self.acceleration
            }
        }
        let test_field = TestAccelerationField { acceleration };

        euler_integrator.step(&mut euler_pos, &mut euler_vel, &test_field, dt);

        // Test with RK4
        let mut rk4_pos = Vector::new(0.0, 10.0, 0.0);
        let mut rk4_vel = Vector::new(5.0, 0.0, 0.0);
        let rk4_integrator = RungeKuttaFourthOrder;
        rk4_integrator.step(&mut rk4_pos, &mut rk4_vel, &test_field, dt);

        // Print the values for debugging
        println!("Euler pos: {:?}, vel: {:?}", euler_pos, euler_vel);
        println!("RK4 pos: {:?}, vel: {:?}", rk4_pos, rk4_vel);

        // Currently RK4 produces slightly different results than Euler
        // The difference is small because we're using constant acceleration
        // In a proper N-body simulation with varying forces, the difference would be larger

        // For constant acceleration, RK4 should still be slightly different due to
        // the different integration method
        let pos_diff = (euler_pos - rk4_pos).length();
        let vel_diff = (euler_vel - rk4_vel).length();

        // There should be some difference, even if small
        assert!(
            pos_diff > 1e-10 || vel_diff > 1e-10,
            "RK4 should produce different results than Euler, but got pos_diff={}, vel_diff={}",
            pos_diff,
            vel_diff
        );
    }

    #[test]
    fn test_rk4_multi_stage_integration() {
        use crate::physics::integrators::Integrator;

        let integrator = RungeKuttaFourthOrder;
        let mut position = Vector::new(0.0, 10.0, 0.0);
        let mut velocity = Vector::new(5.0, 0.0, 0.0);
        let dt = 1.0 / 60.0;

        // Simple constant acceleration field
        struct TestAccelerationField;
        impl AccelerationField for TestAccelerationField {
            fn at(&self, _: Vector) -> Vector {
                Vector::new(0.0, -9.81, 0.0)
            }
        }
        let test_field = TestAccelerationField;

        let initial_pos = position;
        let initial_vel = velocity;

        // Run one integration step
        integrator.step(&mut position, &mut velocity, &test_field, dt);

        // Check that position and velocity changed
        assert_ne!(position, initial_pos, "Position should change");
        assert_ne!(velocity, initial_vel, "Velocity should change");

        // RK4 with constant acceleration should produce specific results
        // For constant acceleration, RK4 reduces to exact solution
        let expected_vel = initial_vel + Vector::new(0.0, -9.81, 0.0) * dt;
        let expected_pos =
            initial_pos + initial_vel * dt + Vector::new(0.0, -9.81, 0.0) * dt * dt * 0.5;

        assert!(
            (velocity - expected_vel).length() < 1e-10,
            "Velocity should match expected"
        );
        assert!(
            (position - expected_pos).length() < 1e-10,
            "Position should match expected"
        );
    }

    #[test]
    fn test_integration_step() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Add resources
        app.insert_resource(CurrentIntegrator(Box::new(SymplecticEuler)));
        app.insert_resource(PhysicsTime::default());
        app.insert_resource(GravitationalOctree(Octree::new(0.5, 0.01, 1e6)));
        app.insert_resource(GravitationalConstant(1.0));

        // Create a test body
        let entity = app
            .world_mut()
            .spawn((
                Position::new(Vector::new(0.0, 10.0, 0.0)),
                Mass::new(1.0),
                Velocity::new(Vector::new(5.0, 0.0, 0.0)),
                PhysicsBody,
            ))
            .id();

        // Add a massive body below to generate downward force
        app.world_mut().spawn((
            Position::new(Vector::new(0.0, -1000.0, 0.0)),
            Mass::new(1e15), // Very massive to generate noticeable force
            Velocity::new(Vector::ZERO),
            PhysicsBody,
        ));

        // Run integration
        app.add_systems(Update, (rebuild_octree, integrate_motions).chain());
        app.update();

        // Check that position and velocity were updated
        let position = app.world().entity(entity).get::<Position>().unwrap();
        let velocity = app.world().entity(entity).get::<Velocity>().unwrap();

        // The system now uses gravitational forces from the octree
        // We just verify that motion occurred (body moved and velocity changed)

        // X position should increase (initial velocity was 5.0 in x)
        assert!(position.value().x > 0.0, "X position should have increased");

        // Y velocity should be negative (gravitational attraction from massive body below)
        assert!(
            velocity.value().y < 0.0,
            "Y velocity should be negative due to gravity"
        );

        // Position should have changed from initial (0, 10, 0)
        assert!(
            position.value().x != 0.0 || position.value().y != 10.0,
            "Position should have changed from initial state"
        );
    }

    #[test]
    fn test_barycentric_drift_correction_enabled() {
        use crate::test_utils::create_test_app;

        let mut app = create_test_app();

        // Add SimulationConfig resource and set up with drift correction enabled
        let mut config = SimulationConfig::default();
        config.physics.barycentric_drift_correction = true;
        app.insert_resource(config);
        app.insert_resource(Barycenter::default());

        // Spawn two bodies at different positions
        app.world_mut().spawn((
            Position::new(Vector::new(10.0, 0.0, 0.0)),
            Mass::new(1.0),
            PhysicsBody,
        ));
        app.world_mut().spawn((
            Position::new(Vector::new(-10.0, 0.0, 0.0)),
            Mass::new(1.0),
            PhysicsBody,
        ));

        // Run the system once to initialize barycenter
        let _ = app
            .world_mut()
            .run_system_once(counteract_barycentric_drift);

        let initial_barycenter = app.world().resource::<Barycenter>().0;
        assert!(initial_barycenter.is_some());
        let initial_barycenter_value = initial_barycenter.unwrap();

        // Move bodies to simulate drift
        let mut query = app.world_mut().query::<&mut Position>();
        for mut pos in query.iter_mut(app.world_mut()) {
            *pos.value_mut() += Vector::new(5.0, 0.0, 0.0);
        }

        // Run the correction system
        let _ = app
            .world_mut()
            .run_system_once(counteract_barycentric_drift);

        // With correction enabled, barycenter should remain at original position
        let final_barycenter = app.world().resource::<Barycenter>().0.unwrap();
        assert!(
            (final_barycenter - initial_barycenter_value).length() < 1e-6,
            "Barycenter should remain fixed when correction is enabled"
        );

        // Bodies should have been shifted back
        let mut query = app.world_mut().query::<&Position>();
        let positions: Vec<Vector> = query.iter(app.world()).map(|p| p.value()).collect();

        // Center of mass should still be at origin (original barycenter)
        let center_of_mass = positions.iter().sum::<Vector>() / positions.len() as f64;
        assert!(
            center_of_mass.length() < 1e-6,
            "Center of mass should be at origin after correction"
        );
    }

    #[test]
    fn test_barycentric_drift_correction_disabled() {
        use crate::test_utils::create_test_app;

        let mut app = create_test_app();

        // Add SimulationConfig resource and set up with drift correction disabled
        let mut config = SimulationConfig::default();
        config.physics.barycentric_drift_correction = false;
        app.insert_resource(config);
        app.insert_resource(Barycenter::default());

        // Spawn two bodies at different positions
        app.world_mut().spawn((
            Position::new(Vector::new(10.0, 0.0, 0.0)),
            Mass::new(1.0),
            PhysicsBody,
        ));
        app.world_mut().spawn((
            Position::new(Vector::new(-10.0, 0.0, 0.0)),
            Mass::new(1.0),
            PhysicsBody,
        ));

        // Run the system once to initialize barycenter
        let _ = app
            .world_mut()
            .run_system_once(counteract_barycentric_drift);

        let initial_barycenter = app.world().resource::<Barycenter>().0;
        assert!(initial_barycenter.is_some());
        assert!(
            initial_barycenter.unwrap().length() < 1e-6,
            "Initial barycenter should be at origin"
        );

        // Move bodies to simulate drift
        let mut query = app.world_mut().query::<&mut Position>();
        for mut pos in query.iter_mut(app.world_mut()) {
            *pos.value_mut() += Vector::new(5.0, 0.0, 0.0);
        }

        // Run the correction system
        let _ = app
            .world_mut()
            .run_system_once(counteract_barycentric_drift);

        // With correction disabled, barycenter should have moved
        let final_barycenter = app.world().resource::<Barycenter>().0.unwrap();
        assert!(
            (final_barycenter - Vector::new(5.0, 0.0, 0.0)).length() < 1e-6,
            "Barycenter should have moved to new position when correction is disabled"
        );

        // Bodies should not have been shifted
        let mut query = app.world_mut().query::<&Position>();
        let positions: Vec<Vector> = query.iter(app.world()).map(|p| p.value()).collect();

        // Check that bodies are at their moved positions
        assert!(positions.contains(&Vector::new(15.0, 0.0, 0.0)));
        assert!(positions.contains(&Vector::new(-5.0, 0.0, 0.0)));
    }

    #[test]
    fn test_barycentric_drift_with_different_masses() {
        use crate::test_utils::create_test_app;

        let mut app = create_test_app();

        // Add SimulationConfig resource with correction disabled to test barycenter calculation
        let mut config = SimulationConfig::default();
        config.physics.barycentric_drift_correction = false;
        app.insert_resource(config);
        app.insert_resource(Barycenter::default());

        // Spawn bodies with different masses
        app.world_mut().spawn((
            Position::new(Vector::new(10.0, 0.0, 0.0)),
            Mass::new(2.0), // Heavier mass
            PhysicsBody,
        ));
        app.world_mut().spawn((
            Position::new(Vector::new(-10.0, 0.0, 0.0)),
            Mass::new(1.0), // Lighter mass
            PhysicsBody,
        ));

        // Run the system to calculate barycenter
        let _ = app
            .world_mut()
            .run_system_once(counteract_barycentric_drift);

        // Barycenter should be weighted towards heavier mass
        // Expected: (10*2 + (-10)*1) / (2+1) = (20-10)/3 = 10/3 ≈ 3.33
        let barycenter = app.world().resource::<Barycenter>().0.unwrap();
        assert!(
            (barycenter.x - 10.0 / 3.0).abs() < 1e-6,
            "Barycenter should be weighted by mass, expected x=3.33, got x={}",
            barycenter.x
        );
    }
}
