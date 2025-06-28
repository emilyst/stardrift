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
        let position = math::random_unit_vector(&mut **rng) * body_distribution_sphere_radius;
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

// TODO: test
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
