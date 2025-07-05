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
    bodies
        .par_iter_mut()
        .for_each(|(entity, transform, mass, mut external_force)| {
            external_force.set_force(octree.calculate_force(
                &OctreeBody {
                    entity,
                    position: Vector::from(transform.translation),
                    mass: mass.value(),
                },
                octree.root.as_ref(),
                **g,
            ));
        });
}

pub fn counteract_barycentric_drift(
    mut bodies: Query<(&mut Transform, &ComputedMass), With<RigidBody>>,
    mut barycenter: ResMut<Barycenter>,
) {
    let (weighted_positions, total_mass): (Vector, Scalar) = bodies
        .iter()
        .map(|(transform, mass)| (Vector::from(transform.translation), mass.value()))
        .fold((Vector::ZERO, 0.0), |(pos_acc, mass_acc), (pos, mass)| {
            (pos_acc + pos * mass, mass_acc + mass)
        });

    let updated_barycenter = weighted_positions / total_mass;

    if total_mass <= Scalar::EPSILON {
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

    if barycentric_drift.length_squared() <= Scalar::EPSILON {
        return;
    }

    bodies.iter_mut().for_each(|(mut transform, _)| {
        transform.translation += -barycentric_drift.as_vec3();
    });
}
