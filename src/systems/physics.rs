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
            ExternalForce::ZERO,
            MeshMaterial3d(material),
            Mesh3d(mesh),
        ));
    }

    commands.spawn_batch(spawn_data);
}

pub fn rebuild_octree(
    bodies: Query<(Entity, &Transform, &ComputedMass), (With<RigidBody>, Changed<Transform>)>,
    mut octree: ResMut<GravitationalOctree>,
) {
    if bodies.is_empty() {
        return;
    }

    octree.build(bodies.iter().map(|(entity, transform, mass)| OctreeBody {
        entity,
        position: Vector::from(transform.translation),
        mass: mass.value(),
    }));
}

pub fn apply_gravitation_octree(
    g: Res<GravitationalConstant>,
    octree: Res<GravitationalOctree>,
    mut bodies: Query<
        (Entity, &Transform, &ComputedMass, &mut ExternalForce),
        (With<RigidBody>, Changed<Transform>),
    >,
) {
    let gravitational_constant = **g;

    bodies
        .par_iter_mut()
        .for_each(|(entity, transform, mass, mut external_force)| {
            let body = OctreeBody {
                entity,
                position: Vector::from(transform.translation),
                mass: mass.value(),
            };

            external_force.set_force(octree.calculate_force(
                &body,
                octree.root.as_ref(),
                gravitational_constant,
            ));
        });
}

pub fn update_barycenter(
    bodies: Query<(&Transform, &ComputedMass), With<RigidBody>>,
    mut current_barycenter: ResMut<CurrentBarycenter>,
    mut initial_barycenter: ResMut<InitialBarycenter>,
    mut barycenter_events: EventWriter<BarycenterInitialized>,
) {
    if bodies.is_empty() {
        return;
    }

    let (weighted_positions, total_mass): (Vector, Scalar) = bodies
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

        // Set initial barycenter if this is the first calculation
        if initial_barycenter.is_none() {
            **initial_barycenter = Some(**current_barycenter);

            barycenter_events.write(BarycenterInitialized {
                initial_position: **current_barycenter,
            });
        }
    }
}

pub fn enable_barycenter_shifting(
    mut events: EventReader<BarycenterInitialized>,
    mut commands: Commands,
) {
    for event in events.read() {
        info!("Barycenter initialized at: {:?}", event.initial_position);
        commands.insert_resource(BarycenterShiftingEnabled);
    }
}
