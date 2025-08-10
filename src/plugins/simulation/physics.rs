use crate::config::SimulationConfig;
use crate::physics::integrators::ForceEvaluator;
use crate::physics::math::{Scalar, Vector};
use crate::physics::{
    components::{Mass, PhysicsBody, PhysicsBodyBundle, Position, Velocity},
    octree::{Octree, OctreeBody},
    resources::{CurrentIntegrator, PhysicsTime},
};
use crate::resources::{Barycenter, GravitationalConstant, GravitationalOctree, SharedRng};
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
    bodies: Query<(Entity, &Position, &Mass), (With<PhysicsBody>, Changed<Position>)>,
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

/// Force evaluator that wraps the octree for a specific body
///
/// This struct implements the ForceEvaluator trait to allow integrators
/// to calculate forces at arbitrary positions during multi-stage integration.
struct BodyForceEvaluator<'a> {
    octree: &'a Octree,
    body_entity: Entity,
    body_mass: Scalar,
    g: Scalar,
}

impl<'a> ForceEvaluator for BodyForceEvaluator<'a> {
    fn calc_acceleration(&self, position: Vector) -> Vector {
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
    mut query: Query<(Entity, &mut Position, &mut Velocity, &Mass), With<PhysicsBody>>,
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
    let g_value: Scalar = **g;

    query
        .par_iter_mut()
        .for_each(|(entity, mut position, mut velocity, mass)| {
            // Create a force evaluator for this body
            let evaluator = BodyForceEvaluator {
                octree,
                body_entity: entity,
                body_mass: mass.value(),
                g: g_value,
            };

            // Use the integrator with force evaluator
            integrator
                .0
                .step(position.value_mut(), velocity.value_mut(), &evaluator, dt);
        });
}

/// Synchronize Transform components from high-precision Position components
pub fn sync_transform_from_position(
    mut query: Query<(&Position, &mut Transform), (With<PhysicsBody>, Changed<Position>)>,
    camera_query: Query<&Transform, (With<Camera>, Without<PhysicsBody>)>,
) {
    // Get camera position if available for distance-based culling
    let camera_pos = camera_query
        .iter()
        .next()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    for (position, mut transform) in query.iter_mut() {
        // Only update if difference is significant OR body is near camera
        let distance_to_camera = (position.value().as_vec3() - camera_pos).length();

        if position.needs_transform_update(&transform) || distance_to_camera < 100.0 {
            transform.translation = position.value().as_vec3();
        }
    }
}

/// Counteract barycentric drift to keep simulation centered
pub fn counteract_barycentric_drift(
    mut bodies: Query<(&mut Position, &Mass), With<PhysicsBody>>,
    mut barycenter: ResMut<Barycenter>,
) {
    let (weighted_positions, total_mass): (Vector, Scalar) = bodies
        .iter()
        .map(|(position, mass)| (position.value(), mass.value()))
        .fold((Vector::ZERO, 0.0), |(pos_acc, mass_acc), (pos, mass)| {
            (pos_acc + pos * mass, mass_acc + mass)
        });

    let updated_barycenter = weighted_positions / total_mass;

    if total_mass.abs() <= Scalar::EPSILON {
        return;
    }

    if !updated_barycenter.is_finite() {
        return;
    }

    let Some(barycenter) = **barycenter else {
        **barycenter = Some(updated_barycenter);
        return;
    };

    let barycentric_drift = updated_barycenter - barycenter;

    if barycentric_drift.length_squared().abs() <= Scalar::EPSILON {
        return;
    }

    bodies.par_iter_mut().for_each(|(mut position, _)| {
        *position.value_mut() += -barycentric_drift;
    });
}

/// Helper function to spawn bodies with the given parameters
pub fn spawn_bodies(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    rng: &mut ResMut<SharedRng>,
    body_count: usize,
    config: &SimulationConfig,
) {
    use super::components::factory;
    use crate::utils::color::emissive_material_for_temp;

    for _ in 0..body_count {
        let position = factory::random_position(rng, body_count, config);
        let radius = factory::random_radius(rng, config);
        let temperature = factory::calculate_temperature(radius, config);
        let velocity = factory::random_velocity(rng, position, config);

        let material = emissive_material_for_temp(
            materials,
            temperature,
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
    mut rng: ResMut<SharedRng>,
    body_count: Res<crate::resources::BodyCount>,
    config: Res<SimulationConfig>,
) {
    spawn_bodies(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut rng,
        **body_count,
        &config,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::integrators::{RungeKuttaFourthOrder, SymplecticEuler};

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
        // Simple test evaluator that returns constant acceleration
        struct TestEvaluator {
            acceleration: Vector,
        }
        impl ForceEvaluator for TestEvaluator {
            fn calc_acceleration(&self, _position: Vector) -> Vector {
                self.acceleration
            }
        }
        let evaluator = TestEvaluator { acceleration };

        euler_integrator.step(&mut euler_pos, &mut euler_vel, &evaluator, dt);

        // Test with RK4
        let mut rk4_pos = Vector::new(0.0, 10.0, 0.0);
        let mut rk4_vel = Vector::new(5.0, 0.0, 0.0);
        let rk4_integrator = RungeKuttaFourthOrder;
        rk4_integrator.step(&mut rk4_pos, &mut rk4_vel, &evaluator, dt);

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

        // This test verifies that RK4 works correctly after removing MultiStageIntegrator.
        // RK4 now handles all its stages internally using the ForceEvaluator.

        let integrator = RungeKuttaFourthOrder;
        let mut position = Vector::new(0.0, 10.0, 0.0);
        let mut velocity = Vector::new(5.0, 0.0, 0.0);
        let dt = 1.0 / 60.0;

        // Simple constant acceleration evaluator
        struct TestEvaluator;
        impl ForceEvaluator for TestEvaluator {
            fn calc_acceleration(&self, _position: Vector) -> Vector {
                Vector::new(0.0, -9.81, 0.0)
            }
        }
        let evaluator = TestEvaluator;

        let initial_pos = position;
        let initial_vel = velocity;

        // Run one integration step
        integrator.step(&mut position, &mut velocity, &evaluator, dt);

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
}
