mod diagnostics;
mod diagnostics_hud;

use crate::diagnostics::SimulationDiagnosticsPlugin;
use crate::diagnostics_hud::DiagnosticsHudPlugin;
use avian3d::math::Scalar;
use avian3d::math::Vector;
use avian3d::prelude::*;
use bevy::color::palettes::css;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy::ecs::schedule::LogLevel;
use bevy::ecs::schedule::ScheduleBuildSettings;
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use bevy_panorbit_camera::TouchControls;
use bevy_panorbit_camera::TrackpadBehavior;
use rand::distr::weighted::Weight;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[derive(Resource, Deref, DerefMut, Debug, Clone, PartialEq)]
struct SimulationRng(ChaCha8Rng);

// TODO: use a seedable RNG and make the seed configurable
impl Default for SimulationRng {
    fn default() -> Self {
        Self(ChaCha8Rng::from_rng(&mut rand::rng()))
    }
}

#[derive(Resource, Deref, DerefMut, Copy, Clone, PartialEq, Debug)]
struct G(Scalar);

impl Default for G {
    fn default() -> Self {
        Self(1000.0)
    }
}

#[derive(Resource, Deref, DerefMut, Copy, Clone, PartialEq, Debug)]
struct BodyCount(usize);

impl Default for BodyCount {
    fn default() -> Self {
        Self(100)
    }
}

#[derive(Resource, Deref, DerefMut, Copy, Clone, Default, PartialEq, Debug)]
struct CurrentBarycenter(Position);

#[derive(Resource, Deref, DerefMut, Copy, Clone, Default, PartialEq, Debug)]
struct PreviousBarycenter(Position);

#[derive(Bundle, Clone, Debug, Default)]
struct BodyBundle {
    collider: Collider,
    gravity_scale: GravityScale,
    rigid_body: RigidBody,
    transform: Transform,
}

impl BodyBundle {
    fn new(transform: Transform, radius: Scalar) -> Self {
        Self {
            collider: Collider::sphere(radius),
            gravity_scale: GravityScale(0.0),
            rigid_body: RigidBody::Dynamic,
            transform,
        }
    }
}

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins,
        DiagnosticsHudPlugin,
        FrameTimeDiagnosticsPlugin::default(),
        LogDiagnosticsPlugin::default(),
        PanOrbitCameraPlugin,
        PhysicsDiagnosticsPlugin,
        PhysicsPlugins::default(),
        SimulationDiagnosticsPlugin::default(),
    ));

    app.init_resource::<SimulationRng>();
    app.init_resource::<G>();
    app.init_resource::<BodyCount>();
    app.init_resource::<CurrentBarycenter>();
    app.init_resource::<PreviousBarycenter>();

    app.edit_schedule(FixedUpdate, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            ..default()
        });
    });

    app.add_systems(Startup, (spawn_camera, spawn_bodies));
    app.add_systems(
        FixedUpdate,
        (
            // TODO: move these systems into a Simulation struct
            apply_gravitation,
            update_barycenter,
            follow_barycenter,
        )
            .chain(),
    );
    app.add_systems(Update, (quit_on_escape, pause_physics_on_space));

    app.run();
}

fn spawn_camera(mut commands: Commands, body_count: Res<BodyCount>) {
    commands.spawn((
        Name::new("Main Camera"),
        Camera {
            hdr: true,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Camera3d::default(),
        Tonemapping::TonyMcMapface,
        Bloom::NATURAL,
        Msaa::default(),
        PanOrbitCamera {
            focus: Vec3::ZERO,
            pan_smoothness: 0.0,
            radius: Some((**body_count * **body_count / 3) as f32),
            touch_controls: TouchControls::TwoFingerOrbit,
            trackpad_behavior: TrackpadBehavior::blender_default(),
            trackpad_pinch_to_zoom_enabled: true,
            ..default()
        },
    ));
}

fn spawn_bodies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<SimulationRng>,
    body_count: Res<BodyCount>,
) {
    for _ in 0..**body_count {
        let position = random_unit_vector(&mut *rng) * (**body_count * **body_count / 10) as f64;
        let transform = Transform::from_translation(position.as_vec3());
        let radius = rng.random_range(0.5..=10.0);

        let material = MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::LinearRgba(LinearRgba::rgb(10000.0, 0.0, 100.0)),
            ..default()
        }));
        let mesh = Mesh3d(meshes.add(Sphere::new(radius as f32)));

        commands.spawn((BodyBundle::new(transform, radius), material, mesh));
    }
}

fn random_unit_vector(rng: &mut SimulationRng) -> Vector {
    // Use Box-Muller transform to generate normally distributed values
    let u1: f64 = rng.random::<f64>();
    let u2: f64 = rng.random::<f64>();
    let u3: f64 = rng.random::<f64>();
    let u4: f64 = rng.random::<f64>();

    // Box-Muller transform for first two normal values
    let z1 = libm::sqrt(-2.0 * libm::log(u1)) * libm::cos(2.0 * std::f64::consts::PI * u2);
    let z2 = libm::sqrt(-2.0 * libm::log(u1)) * libm::sin(2.0 * std::f64::consts::PI * u2);

    // Box-Muller transform for third normal value
    let z3 = libm::sqrt(-2.0 * libm::log(u3)) * libm::cos(2.0 * std::f64::consts::PI * u4);

    // Normalize to unit sphere
    Vector::new(z1, z2, z3) / libm::sqrt(z1 * z1 + z2 * z2 + z3 * z3)
}

// TODO: test
fn apply_gravitation(
    time: ResMut<Time>,
    g: Res<G>,
    mut bodies: Query<
        (&Transform, &ComputedMass, &mut LinearVelocity),
        (With<RigidBody>, Without<RigidBodyDisabled>),
    >,
) {
    let delta_time = time.delta_secs_f64();
    let mut body_pairs = bodies.iter_combinations_mut();

    const MIN_DISTANCE: Scalar = 1.0;
    const MAX_FORCE: Scalar = 100000.0;
    const MIN_DISTANCE_SQUARED: Scalar = MIN_DISTANCE * MIN_DISTANCE;

    while let Some(
        [
            (transform1, computed_mass1, mut linear_velocity1),
            (transform2, computed_mass2, mut linear_velocity2),
        ],
    ) = body_pairs.fetch_next()
    {
        let direction = Vector::from(transform2.translation) - Vector::from(transform1.translation);
        let distance_squared = direction.length_squared() as Scalar;

        if distance_squared < MIN_DISTANCE_SQUARED {
            continue;
        }

        let distance = distance_squared.sqrt();
        let direction_normalized = direction / distance;
        let force_magnitude = (**g * computed_mass1.value() * computed_mass2.value()
            / distance_squared)
            .min(MAX_FORCE);
        let acceleration1 = force_magnitude * direction_normalized * computed_mass1.inverse();
        let acceleration2 = -force_magnitude * direction_normalized * computed_mass2.inverse();

        **linear_velocity1 += acceleration1 * delta_time;
        **linear_velocity2 += acceleration2 * delta_time;
    }
}

// TODO: test
fn update_barycenter(
    bodies: Query<(&Transform, &ComputedMass), (With<RigidBody>, Without<RigidBodyDisabled>)>,
    mut current_barycenter: ResMut<CurrentBarycenter>,
    mut previous_barycenter: ResMut<PreviousBarycenter>,
) {
    **previous_barycenter = **current_barycenter;

    let (weighted_positions, total_mass): (Vector, Scalar) = bodies
        .iter()
        .map(|(transform, mass)| {
            let mass = mass.value();
            (Vector::from(transform.translation) * mass, mass)
        })
        .fold((Vector::ZERO, 0.0), |(pos_acc, mass_acc), (pos, mass)| {
            (pos_acc + pos, mass_acc + mass)
        });

    if total_mass > Scalar::ZERO {
        **current_barycenter = Position::from(weighted_positions / total_mass);
    }
}

fn follow_barycenter(
    mut pan_orbit_camera: Single<&mut PanOrbitCamera>,
    mut gizmos: Gizmos,
    current_barycenter: Res<CurrentBarycenter>,
    body_count: Res<BodyCount>,
) {
    if current_barycenter.is_finite() {
        pan_orbit_camera.force_update = true;
        pan_orbit_camera.target_focus = current_barycenter.as_vec3();
        gizmos.cross(
            current_barycenter.as_vec3(),
            libm::cbrtf((**body_count * **body_count / 3) as f32),
            css::WHITE,
        );
    }
}

fn quit_on_escape(keys: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write_default();
    }
}

fn pause_physics_on_space(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    enabled_rigid_bodies: Query<Entity, (With<RigidBody>, Without<RigidBodyDisabled>)>,
    disabled_rigid_bodies: Query<Entity, (With<RigidBody>, With<RigidBodyDisabled>)>,
) {
    if keys.just_pressed(KeyCode::Space) {
        for entity in &enabled_rigid_bodies {
            commands.entity(entity).insert(RigidBodyDisabled);
        }

        for entity in &disabled_rigid_bodies {
            commands.entity(entity).remove::<RigidBodyDisabled>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use statrs::distribution::{ChiSquared, ContinuousCDF};

    #[test]
    fn test_chi_square_uniformity_of_random_unit_vector() {
        let mut rng = SimulationRng(ChaCha8Rng::seed_from_u64(0));
        let count_of_samples = 100_000;
        let count_of_bins = 10;

        let mut bins = vec![0; count_of_bins];

        for _ in 0..count_of_samples {
            let v = random_unit_vector(&mut rng);
            let bin_index = ((v.z + 1.0) * count_of_bins as f64 / 2.0).floor() as usize;
            let bin_index = bin_index.min(count_of_bins - 1);
            bins[bin_index] += 1;
        }

        let expected_count_per_bin = count_of_samples as f64 / count_of_bins as f64;
        let chi_square: f64 = bins
            .iter()
            .map(|&observed| {
                let diff = observed as f64 - expected_count_per_bin;
                diff * diff / expected_count_per_bin
            })
            .sum();

        let degrees_of_freedom = (count_of_bins - 1) as f64;
        let chi_squared_distribution = ChiSquared::new(degrees_of_freedom).unwrap();
        let p_value = 1.0 - chi_squared_distribution.cdf(chi_square);

        assert!(
            p_value > 0.01,
            "P-value too low: {:.4}. Chi-square: {:.4}, degrees of freedom: {}",
            p_value,
            chi_square,
            degrees_of_freedom
        );
    }

    #[test]
    fn test_random_unit_vector_properties() {
        let mut rng = SimulationRng(ChaCha8Rng::seed_from_u64(0));

        for _ in 0..100000 {
            let v = random_unit_vector(&mut rng);
            let length = libm::sqrt(v.x * v.x + v.y * v.y + v.z * v.z);

            assert!(
                (length - 1.0).abs() < 1e-10,
                "Vector length should be 1, but was: {}",
                length
            );
        }
    }
}
