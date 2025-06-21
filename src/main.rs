#![no_std]

mod diagnostics;
mod diagnostics_hud;

use crate::diagnostics::SimulationDiagnosticsPlugin;
use crate::diagnostics_hud::DiagnosticsHudPlugin;
use avian3d::math;
use avian3d::math::Scalar;
use avian3d::math::Vector;
use avian3d::prelude::*;
use bevy::color::palettes::css;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
#[cfg(not(target_arch = "wasm32"))]
use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy::ecs::schedule::LogLevel;
use bevy::ecs::schedule::ScheduleBuildSettings;
use bevy::math::*;
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use bevy_panorbit_camera::TouchControls;
use bevy_panorbit_camera::TrackpadBehavior;
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
        Self(100000.0)
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
struct CurrentBarycenter(Vector);

#[derive(Resource, Deref, DerefMut, Copy, Clone, Default, PartialEq, Debug)]
struct PreviousBarycenter(Vector);

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
        #[cfg(not(target_arch = "wasm32"))]
        LogDiagnosticsPlugin::default(),
        PanOrbitCameraPlugin,
        #[cfg(not(target_arch = "wasm32"))]
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
    let body_distribution_sphere_radius =
        min_sphere_radius_for_surface_distribution(**body_count, 100.0, 0.001);

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
            radius: Some((body_distribution_sphere_radius * 3.0) as f32),
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
        let body_distribution_sphere_radius =
            min_sphere_radius_for_surface_distribution(**body_count, 100.0, 0.001);
        let position = random_unit_vector(&mut *rng) * body_distribution_sphere_radius;
        let transform = Transform::from_translation(position.as_vec3());
        let body_radius = rng.random_range(2.0..=3.0);

        let material = MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::LinearRgba(LinearRgba::rgb(10000.0, 0.0, 100.0)),
            ..default()
        }));
        let mesh = Mesh3d(meshes.add(Sphere::new(body_radius as f32)));

        commands.spawn((BodyBundle::new(transform, body_radius), material, mesh));
    }
}

fn min_sphere_radius_for_surface_distribution(
    n: usize,
    min_distance: Scalar,
    tolerance: Scalar,
) -> Scalar {
    let minimum_radius = min_distance * libm::sqrt(n as Scalar / 4.0);
    let spherical_correction = if n > 4 {
        // Tammes problem approximation
        let solid_angle_per_point = 4.0 * math::PI / n as Scalar;
        let half_angle = solid_angle_per_point / libm::sqrt(2.0 * math::PI);
        min_distance / (2.0 * libm::sin(half_angle))
    } else {
        // For small N, use exact solutions
        match n {
            1 => min_distance,                         // Any radius works
            2 => min_distance / 2.0,                   // Points are antipodal
            3 => min_distance / libm::sqrt(3.0),       // Equilateral triangle
            4 => min_distance / libm::sqrt(8.0 / 3.0), // Tetrahedron
            _ => minimum_radius,
        }
    };
    let mut corrected_minimum_radius = minimum_radius.max(spherical_correction);

    // Iterative refinement using the sphere cap
    for _ in 0..10 {
        let cap_radius = min_distance / 2.0;
        let cap_area = 2.0
            * math::PI
            * corrected_minimum_radius
            * corrected_minimum_radius
            * libm::pow(
                1.0 - libm::sqrt(1.0 - (cap_radius / corrected_minimum_radius)),
                2.0,
            );
        let total_cap_area = n as Scalar * cap_area;
        let sphere_area = 4.0 * math::PI * corrected_minimum_radius * corrected_minimum_radius;

        if total_cap_area > sphere_area {
            corrected_minimum_radius *= 1.1;
        } else if sphere_area - total_cap_area > tolerance * sphere_area {
            corrected_minimum_radius *= 0.95;
        } else {
            break; // Converged
        }
    }

    corrected_minimum_radius
}

fn random_unit_vector(rng: &mut SimulationRng) -> Vector {
    let theta = rng.random_range(0.0..=2.0 * math::PI);
    let phi = libm::acos(rng.random_range(-1.0..=1.0));
    let r = 1.0;

    Vector::new(
        r * libm::sin(phi) * libm::cos(theta),
        r * libm::sin(phi) * libm::sin(theta),
        r * libm::cos(phi),
    )
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
    const MAX_FORCE: Scalar = 10000.0;
    const MIN_DISTANCE_SQUARED: Scalar = MIN_DISTANCE * MIN_DISTANCE;

    while let Some(
        [
            (transform1, computed_mass1, mut linear_velocity1),
            (transform2, computed_mass2, mut linear_velocity2),
        ],
    ) = body_pairs.fetch_next()
    {
        let direction = Vector::from(transform2.translation) - Vector::from(transform1.translation);
        let distance_squared = direction.length_squared();

        if distance_squared < MIN_DISTANCE_SQUARED {
            continue;
        }

        let distance = distance_squared;
        let direction_normalized = direction / distance;
        let force_magnitude =
            **g * computed_mass1.value() * computed_mass2.value() / distance_squared;
        let force_magnitude = force_magnitude.min(MAX_FORCE);
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

    if total_mass > 0.0 {
        **current_barycenter = weighted_positions / total_mass;
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
        pan_orbit_camera.target_focus = current_barycenter.clone().as_vec3();
        gizmos.cross(
            current_barycenter.as_vec3(),
            libm::cbrt(**body_count as Scalar * **body_count as Scalar / 3.0 as Scalar) as f32,
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
    use statrs::distribution::ChiSquared;
    use statrs::distribution::ContinuousCDF;

    #[test]
    fn test_chi_square_uniformity_of_random_unit_vector() {
        let count_of_samples = 100_000;
        let count_of_bins = 20;

        let mut bins = vec![0.0; count_of_bins];

        for _ in 0..count_of_samples {
            let v = random_unit_vector(&mut SimulationRng::default());
            let bin_index = libm::floor((v.z + 1.0) * count_of_bins as Scalar / 2.0) as usize;
            let bin_index = bin_index.min(count_of_bins - 1);
            bins[bin_index] += 1.0;
        }

        let expected_count_per_bin = count_of_samples as f64 / count_of_bins as f64;
        let chi_square: f64 = bins
            .iter()
            .map(|&observed| {
                let diff = observed - expected_count_per_bin;
                diff * diff / expected_count_per_bin
            })
            .sum();

        let degrees_of_freedom = (count_of_bins - 1) as Scalar;
        let chi_squared_distribution = ChiSquared::new(degrees_of_freedom as f64).unwrap();
        let p_value = 1.0 - chi_squared_distribution.cdf(chi_square.into());

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
        for _ in 0..100_000 {
            let v = random_unit_vector(&mut SimulationRng::default());
            let length = libm::sqrt(v.x * v.x + v.y * v.y + v.z * v.z);

            assert!(
                (length - 1.0).abs() < 1e-10,
                "Vector length should be 1, but was: {}",
                length
            );
        }
    }
}
