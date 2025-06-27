use crate::config::SimulationConfig;
use crate::physics::octree::OctreeBody;
use crate::resources::*;
use crate::utils::color;
use crate::utils::math;
use avian3d::math::Vector;
use avian3d::prelude::*;
use bevy::prelude::*;
use rand::Rng;

pub fn spawn_bodies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<SharedRng>,
    body_count: Res<BodyCount>,
    config: Res<SimulationConfig>,
) {
    spawn_simulation_bodies(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut rng,
        **body_count,
        &config,
    );
}

pub fn spawn_simulation_bodies(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    rng: &mut ResMut<SharedRng>,
    body_count: usize,
    config: &SimulationConfig,
) {
    for _ in 0..body_count {
        let body_distribution_sphere_radius = math::min_sphere_radius_for_surface_distribution(
            body_count,
            config.physics.body_distribution_sphere_radius_multiplier,
            config.physics.body_distribution_min_distance,
        );
        let position = math::random_unit_vector(&mut **rng) * body_distribution_sphere_radius;
        let transform = Transform::from_translation(position.as_vec3());
        let radius =
            rng.random_range(config.physics.min_body_radius..=config.physics.max_body_radius);
        let mesh = meshes.add(Sphere::new(radius as f32));

        let min_temp = config.rendering.min_temperature;
        let max_temp = config.rendering.max_temperature;
        let min_radius = config.physics.min_body_radius;
        let max_radius = config.physics.max_body_radius;
        let temperature =
            min_temp + (max_temp - min_temp) * (max_radius - radius) / (max_radius - min_radius);
        let bloom_intensity = config.rendering.bloom_intensity;
        let saturation_intensity = config.rendering.saturation_intensity;
        let material = color::emissive_material_for_temp(
            materials,
            temperature,
            bloom_intensity,
            saturation_intensity,
        );

        commands.spawn((
            transform,
            Collider::sphere(radius),
            GravityScale(0.0),
            RigidBody::Dynamic,
            MeshMaterial3d(material.clone()),
            Mesh3d(mesh),
        ));
    }
}

pub fn rebuild_octree(
    bodies: Query<(Entity, &Transform, &ComputedMass), With<RigidBody>>,
    octree: ResMut<GravitationalOctree>,
) {
    let octree_bodies: Vec<OctreeBody> = bodies
        .iter()
        .map(|(entity, transform, mass)| OctreeBody {
            entity,
            position: Vector::from(transform.translation),
            mass: mass.value(),
        })
        .collect();

    if let Ok(mut octree_guard) = octree.write() {
        octree_guard.build(octree_bodies);
    }
}

pub fn apply_gravitation_octree(
    time: ResMut<Time>,
    g: Res<GravitationalConstant>,
    octree: Res<GravitationalOctree>,
    mut bodies: Query<
        (Entity, &Transform, &ComputedMass, &mut LinearVelocity),
        (With<RigidBody>, Without<RigidBodyDisabled>),
    >,
) {
    let delta_time = time.delta_secs_f64();

    if let Ok(octree_guard) = octree.read() {
        for (entity, transform, mass, mut velocity) in bodies.iter_mut() {
            let body = OctreeBody {
                entity,
                position: Vector::from(transform.translation),
                mass: mass.value(),
            };

            let force = octree_guard.calculate_force(&body, octree_guard.root.as_ref(), **g);
            let acceleration = force * mass.inverse();
            **velocity += acceleration * delta_time;
        }
    }
}

// TODO: test
pub fn update_barycenter(
    bodies: Query<(&Transform, &ComputedMass), (With<RigidBody>, Without<RigidBodyDisabled>)>,
    mut current_barycenter: ResMut<CurrentBarycenter>,
    mut previous_barycenter: ResMut<PreviousBarycenter>,
) {
    **previous_barycenter = **current_barycenter;

    let (weighted_positions, total_mass): (Vector, avian3d::math::Scalar) = bodies
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
