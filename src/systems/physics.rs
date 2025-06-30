use crate::config::SimulationConfig;
use crate::physics::octree::OctreeBody;
use crate::resources::*;
use crate::utils::color;
use crate::utils::math;
use avian3d::math::Scalar;
use avian3d::math::Vector;
use avian3d::prelude::*;
use bevy::prelude::*;
use rand::Rng;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PhysicsSet {
    BuildOctree,
    ApplyForces,
    UpdateBarycenter,
}

pub fn spawn_simulation_bodies(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    rng: &mut ResMut<SharedRng>,
    body_count: usize,
    config: &SimulationConfig,
) {
    // Pre-calculate common values
    let body_distribution_sphere_radius = math::min_sphere_radius_for_surface_distribution(
        body_count,
        config.physics.body_distribution_sphere_radius_multiplier,
        config.physics.body_distribution_min_distance,
    );
    let min_temp = config.rendering.min_temperature;
    let max_temp = config.rendering.max_temperature;
    let min_radius = config.physics.min_body_radius;
    let max_radius = config.physics.max_body_radius;
    let bloom_intensity = config.rendering.bloom_intensity;
    let saturation_intensity = config.rendering.saturation_intensity;

    // Prepare batch data for efficient spawning
    let mut spawn_data = Vec::with_capacity(body_count);

    for _ in 0..body_count {
        let position = math::random_unit_vector(rng) * body_distribution_sphere_radius;
        let transform = Transform::from_translation(position.as_vec3());
        let radius =
            rng.random_range(config.physics.min_body_radius..=config.physics.max_body_radius);
        let mesh = meshes.add(Sphere::new(radius as f32));

        let temperature =
            min_temp + (max_temp - min_temp) * (max_radius - radius) / (max_radius - min_radius);
        let material = color::emissive_material_for_temp(
            materials,
            temperature,
            bloom_intensity,
            saturation_intensity,
        );

        spawn_data.push((
            transform,
            Collider::sphere(radius),
            GravityScale(0.0),
            RigidBody::Dynamic,
            MeshMaterial3d(material),
            Mesh3d(mesh),
        ));
    }

    // Batch spawn all entities for better performance
    commands.spawn_batch(spawn_data);
}

pub fn rebuild_octree(
    bodies: Query<(Entity, &Transform, &ComputedMass), With<RigidBody>>,
    mut octree: ResMut<GravitationalOctree>,
) {
    let octree_bodies: Vec<OctreeBody> = bodies
        .iter()
        .map(|(entity, transform, mass)| OctreeBody {
            entity,
            position: Vector::from(transform.translation),
            mass: mass.value(),
        })
        .collect();

    octree.build(octree_bodies);
}

pub fn apply_gravitation_octree(
    time: Res<Time<Fixed>>,
    g: Res<GravitationalConstant>,
    octree: Res<GravitationalOctree>,
    mut bodies: Query<(Entity, &Transform, &ComputedMass, &mut LinearVelocity), With<RigidBody>>,
) {
    let delta_time = time.delta_secs_f64();
    let gravitational_constant = **g;

    bodies
        .par_iter_mut()
        .for_each(|(entity, transform, mass, mut velocity)| {
            let body = OctreeBody {
                entity,
                position: Vector::from(transform.translation),
                mass: mass.value(),
            };

            let force = octree.calculate_force(&body, octree.root.as_ref(), gravitational_constant);
            let acceleration = force * mass.inverse();
            **velocity += acceleration * delta_time;
        });
}

pub fn update_barycenter(
    bodies: Query<(&Transform, &ComputedMass), (With<RigidBody>, Changed<Transform>)>,
    all_bodies: Query<(&Transform, &ComputedMass), With<RigidBody>>,
    mut current_barycenter: ResMut<CurrentBarycenter>,
    mut previous_barycenter: ResMut<PreviousBarycenter>,
) {
    // Only recalculate if any body has moved
    if bodies.is_empty() {
        return;
    }

    **previous_barycenter = **current_barycenter;

    let (weighted_positions, total_mass): (Vector, Scalar) = all_bodies
        .iter()
        .map(|(transform, mass)| {
            let mass = mass.value();
            (Vector::from(transform.translation) * mass, mass)
        })
        .fold((Vector::ZERO, 0.0), |(pos_acc, mass_acc), (pos, mass)| {
            (pos_acc + pos, mass_acc + mass)
        });

    if total_mass > 0.0 {
        **current_barycenter = weighted_positions / total_mass;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use avian3d::math::Vector;
    use bevy::ecs::system::SystemState;

    #[test]
    fn test_update_barycenter_no_bodies() {
        let mut world = World::new();
        world.init_resource::<CurrentBarycenter>();
        world.init_resource::<PreviousBarycenter>();

        let initial_current = Vector::new(5.0, 5.0, 5.0);
        **world.resource_mut::<CurrentBarycenter>() = initial_current;

        // No bodies spawned at all
        let mut system_state: SystemState<(
            Query<(&Transform, &ComputedMass), (With<RigidBody>, Changed<Transform>)>,
            Query<(&Transform, &ComputedMass), With<RigidBody>>,
            ResMut<CurrentBarycenter>,
            ResMut<PreviousBarycenter>,
        )> = SystemState::new(&mut world);

        let (bodies, all_bodies, current_barycenter, previous_barycenter) =
            system_state.get_mut(&mut world);

        update_barycenter(bodies, all_bodies, current_barycenter, previous_barycenter);

        // Should remain unchanged when no bodies exist
        assert_eq!(**world.resource::<CurrentBarycenter>(), initial_current);
        assert_eq!(**world.resource::<PreviousBarycenter>(), Vector::ZERO);
    }

    #[test]
    fn test_update_barycenter_no_changed_bodies() {
        let mut world = World::new();
        world.init_resource::<CurrentBarycenter>();
        world.init_resource::<PreviousBarycenter>();

        world.spawn((
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            ComputedMass::new(5.0),
            RigidBody::Dynamic,
        ));

        let mut system_state: SystemState<(
            Query<(&Transform, &ComputedMass), (With<RigidBody>, Changed<Transform>)>,
            Query<(&Transform, &ComputedMass), With<RigidBody>>,
            ResMut<CurrentBarycenter>,
            ResMut<PreviousBarycenter>,
        )> = SystemState::new(&mut world);

        {
            let (bodies, all_bodies, current_barycenter, previous_barycenter) =
                system_state.get_mut(&mut world);
            update_barycenter(bodies, all_bodies, current_barycenter, previous_barycenter);
        }

        let first_run_current = **world.resource::<CurrentBarycenter>();
        let first_run_previous = **world.resource::<PreviousBarycenter>();

        // Clear change detection to simulate no changes in the next frame
        world.clear_trackers();

        // Second run - no bodies should be marked as changed now
        {
            let (bodies, all_bodies, current_barycenter, previous_barycenter) =
                system_state.get_mut(&mut world);
            update_barycenter(bodies, all_bodies, current_barycenter, previous_barycenter);
        }

        // Values should remain unchanged since no bodies were marked as changed
        assert_eq!(**world.resource::<CurrentBarycenter>(), first_run_current);
        assert_eq!(**world.resource::<PreviousBarycenter>(), first_run_previous);
    }

    #[test]
    fn test_update_barycenter_single_body() {
        let mut world = World::new();
        world.init_resource::<CurrentBarycenter>();
        world.init_resource::<PreviousBarycenter>();

        let body_position = Vec3::new(5.0, 10.0, -3.0);
        let body_mass = 2.0;

        let entity = world
            .spawn((
                Transform::from_translation(body_position),
                ComputedMass::new(body_mass),
                RigidBody::Dynamic,
            ))
            .id();

        world
            .entity_mut(entity)
            .get_mut::<Transform>()
            .unwrap()
            .set_changed();

        let mut system_state: SystemState<(
            Query<(&Transform, &ComputedMass), (With<RigidBody>, Changed<Transform>)>,
            Query<(&Transform, &ComputedMass), With<RigidBody>>,
            ResMut<CurrentBarycenter>,
            ResMut<PreviousBarycenter>,
        )> = SystemState::new(&mut world);

        let (bodies, all_bodies, current_barycenter, previous_barycenter) =
            system_state.get_mut(&mut world);

        update_barycenter(bodies, all_bodies, current_barycenter, previous_barycenter);

        // For a single body, barycenter should be at the body's position
        let expected_barycenter = Vector::from(body_position);
        assert_eq!(**world.resource::<CurrentBarycenter>(), expected_barycenter);
        assert_eq!(**world.resource::<PreviousBarycenter>(), Vector::ZERO);
    }

    #[test]
    fn test_update_barycenter_multiple_bodies() {
        let mut world = World::new();
        world.init_resource::<CurrentBarycenter>();
        world.init_resource::<PreviousBarycenter>();

        // Set initial current barycenter
        let initial_current = Vector::new(1.0, 1.0, 1.0);
        **world.resource_mut::<CurrentBarycenter>() = initial_current;

        let entity1 = world
            .spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                ComputedMass::new(1.0),
                RigidBody::Dynamic,
            ))
            .id();

        world.spawn((
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            ComputedMass::new(3.0),
            RigidBody::Dynamic,
        ));

        world.spawn((
            Transform::from_translation(Vec3::new(0.0, 20.0, 0.0)),
            ComputedMass::new(2.0),
            RigidBody::Dynamic,
        ));

        // Mark one body as changed
        world
            .entity_mut(entity1)
            .get_mut::<Transform>()
            .unwrap()
            .set_changed();

        let mut system_state: SystemState<(
            Query<(&Transform, &ComputedMass), (With<RigidBody>, Changed<Transform>)>,
            Query<(&Transform, &ComputedMass), With<RigidBody>>,
            ResMut<CurrentBarycenter>,
            ResMut<PreviousBarycenter>,
        )> = SystemState::new(&mut world);

        let (bodies, all_bodies, current_barycenter, previous_barycenter) =
            system_state.get_mut(&mut world);

        update_barycenter(bodies, all_bodies, current_barycenter, previous_barycenter);

        // Calculate expected barycenter manually
        // weighted_sum = (0,0,0)*1 + (10,0,0)*3 + (0,20,0)*2 = (30, 40, 0)
        // total_mass = 1 + 3 + 2 = 6
        // barycenter = (30, 40, 0) / 6 = (5, 6.666..., 0)
        let expected_barycenter = Vector::new(5.0, 40.0 / 6.0, 0.0);

        let current_barycenter = **world.resource::<CurrentBarycenter>();
        let previous_barycenter = **world.resource::<PreviousBarycenter>();

        // Use approximate equality for floating point comparison
        assert!((current_barycenter.x - expected_barycenter.x).abs() < 1e-10);
        assert!((current_barycenter.y - expected_barycenter.y).abs() < 1e-10);
        assert!((current_barycenter.z - expected_barycenter.z).abs() < 1e-10);

        // Previous barycenter should be the initial current value
        assert_eq!(previous_barycenter, initial_current);
    }

    #[test]
    fn test_update_barycenter_zero_total_mass() {
        let mut world = World::new();
        world.init_resource::<CurrentBarycenter>();
        world.init_resource::<PreviousBarycenter>();

        let initial_current = Vector::new(5.0, 5.0, 5.0);
        **world.resource_mut::<CurrentBarycenter>() = initial_current;

        // Create a body with zero mass
        let entity = world
            .spawn((
                Transform::from_translation(Vec3::new(10.0, 10.0, 10.0)),
                ComputedMass::new(0.0),
                RigidBody::Dynamic,
            ))
            .id();

        world
            .entity_mut(entity)
            .get_mut::<Transform>()
            .unwrap()
            .set_changed();

        let mut system_state: SystemState<(
            Query<(&Transform, &ComputedMass), (With<RigidBody>, Changed<Transform>)>,
            Query<(&Transform, &ComputedMass), With<RigidBody>>,
            ResMut<CurrentBarycenter>,
            ResMut<PreviousBarycenter>,
        )> = SystemState::new(&mut world);

        let (bodies, all_bodies, current_barycenter, previous_barycenter) =
            system_state.get_mut(&mut world);

        update_barycenter(bodies, all_bodies, current_barycenter, previous_barycenter);

        // With zero total mass, current barycenter should remain unchanged
        assert_eq!(**world.resource::<CurrentBarycenter>(), initial_current);
        // Previous should be updated to the initial current value
        assert_eq!(**world.resource::<PreviousBarycenter>(), initial_current);
    }
}
