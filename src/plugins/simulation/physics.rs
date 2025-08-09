use crate::config::SimulationConfig;
use crate::physics::math::{Scalar, Vector};
use crate::physics::{
    components::{Acceleration, Mass, PhysicsBody, PhysicsBodyBundle, Position, Velocity},
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
    CalculateAccelerations,
    IntegrateMotions,
    SyncTransforms,
    CorrectBarycentricDrift,
}

/// Rebuild the octree structure from current body positions
pub fn rebuild_octree(
    bodies: Query<(&Position, &Mass), (With<PhysicsBody>, Changed<Position>)>,
    mut octree: ResMut<GravitationalOctree>,
) {
    if bodies.is_empty() {
        return;
    }

    octree.build(bodies.iter().map(|(position, mass)| OctreeBody {
        position: position.value(),
        mass: mass.value(),
    }));
}

/// Calculate gravitational forces and convert to accelerations
pub fn calculate_accelerations(
    g: Res<GravitationalConstant>,
    octree: Res<GravitationalOctree>,
    mut bodies: Query<(&Position, &Mass, &mut Acceleration), With<PhysicsBody>>,
) {
    let octree: &Octree = &octree;
    let g: Scalar = **g;

    bodies
        .par_iter_mut()
        .for_each(|(position, mass, mut acceleration)| {
            let force = octree.calculate_force(
                &OctreeBody {
                    position: position.value(),
                    mass: mass.value(),
                },
                octree.root.as_ref(),
                g,
            );

            *acceleration.value_mut() = force / mass.value();
        });
}

/// Integrate positions and velocities using the currently active integrator
pub fn integrate_motions(
    mut query: Query<(&mut Position, &mut Velocity, &Acceleration), With<PhysicsBody>>,
    integrator: Res<CurrentIntegrator>,
    physics_time: Res<PhysicsTime>,
) {
    if physics_time.is_paused() {
        return;
    }

    let dt = physics_time.dt;

    query
        .par_iter_mut()
        .for_each(|(mut position, mut velocity, acceleration)| {
            integrator.0.integrate_single(
                position.value_mut(),
                velocity.value_mut(),
                acceleration.value(),
                dt,
            );
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

/// Spawn simulation bodies using the new physics components
pub fn spawn_simulation_bodies(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::integrators::SymplecticEuler;

    #[test]
    fn test_integration_step() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Add resources
        app.insert_resource(CurrentIntegrator(Box::new(SymplecticEuler)));
        app.insert_resource(PhysicsTime::default());

        // Create a test body
        let entity = app
            .world_mut()
            .spawn((
                Position::new(Vector::new(0.0, 10.0, 0.0)),
                Mass::new(1.0),
                Velocity::new(Vector::new(5.0, 0.0, 0.0)),
                Acceleration::new(Vector::new(0.0, -9.81, 0.0)),
                PhysicsBody,
            ))
            .id();

        // Run integration
        app.add_systems(Update, integrate_motions);
        app.update();

        // Check that position and velocity were updated
        let position = app.world().entity(entity).get::<Position>().unwrap();
        let velocity = app.world().entity(entity).get::<Velocity>().unwrap();

        // With dt = 1/60 and semi-implicit Euler:
        // v_new = v_old + a * dt = (5, 0, 0) + (0, -9.81, 0) * (1/60)
        // x_new = x_old + v_new * dt

        let dt = 1.0 / 60.0;
        let expected_vel_y = -9.81 * dt;
        let expected_pos_x = 5.0 * dt; // Starting from x=0
        let expected_pos_y = 10.0 + expected_vel_y * dt;

        assert!((position.value().x - expected_pos_x).abs() < 1e-10);
        assert!((position.value().y - expected_pos_y).abs() < 1e-10);
        assert!((velocity.value().y - expected_vel_y).abs() < 1e-10);
    }
}
