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
