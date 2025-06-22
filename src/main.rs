#![no_std]

mod color;
mod diagnostics;
mod diagnostics_hud;
mod math;

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
struct SharedRng(ChaCha8Rng);

// TODO: use a seedable RNG and make the seed configurable
impl Default for SharedRng {
    fn default() -> Self {
        Self(ChaCha8Rng::from_rng(&mut rand::rng()))
    }
}

#[derive(Resource, Deref, DerefMut, Copy, Clone, PartialEq, Debug)]
struct GravitationalConstant(Scalar);

impl Default for GravitationalConstant {
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

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins,
        DiagnosticsHudPlugin,
        FrameTimeDiagnosticsPlugin::default(),
        LogDiagnosticsPlugin::default(),
        PanOrbitCameraPlugin,
        #[cfg(not(target_arch = "wasm32"))]
        PhysicsDiagnosticsPlugin,
        PhysicsPlugins::default(),
        SimulationDiagnosticsPlugin::default(),
    ));

    app.init_resource::<SharedRng>();
    app.init_resource::<GravitationalConstant>();
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
        math::min_sphere_radius_for_surface_distribution(**body_count, 100.0, 0.001);

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
    mut rng: ResMut<SharedRng>,
    body_count: Res<BodyCount>,
) {
    for _ in 0..**body_count {
        let body_distribution_sphere_radius =
            math::min_sphere_radius_for_surface_distribution(**body_count, 100.0, 0.001);
        let position = math::random_unit_vector(&mut *rng) * body_distribution_sphere_radius;
        let transform = Transform::from_translation(position.as_vec3());
        let radius = rng.random_range(2.0..=3.0);
        let mesh = meshes.add(Sphere::new(radius as f32));

        let temperature = rng.random_range(2500.0..=20000.0);
        let bloom_intensity = 100.0;

        let material = color::create_emissive_material_from_temperature(
            &mut materials,
            temperature,
            bloom_intensity,
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

// TODO: test
fn apply_gravitation(
    time: ResMut<Time>,
    g: Res<GravitationalConstant>,
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
            libm::cbrt(**body_count as Scalar * **body_count as Scalar / 3.0) as f32,
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
