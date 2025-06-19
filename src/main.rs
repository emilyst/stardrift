mod diagnostics;
mod diagnostics_hud;

use crate::diagnostics::SimulationDiagnosticsPlugin;
use crate::diagnostics_hud::DiagnosticsHudPlugin;
use avian3d::math::AsF32;
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

impl Default for SimulationRng {
    fn default() -> Self {
        Self(ChaCha8Rng::seed_from_u64(0))
    }
}

#[derive(Resource, Deref, DerefMut, Copy, Clone, PartialEq, Debug)]
struct G(Scalar);

impl Default for G {
    fn default() -> Self {
        Self(50.0)
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
    material3d: MeshMaterial3d<StandardMaterial>,
    mesh3d: Mesh3d,
    position: Transform,
    rigid_body: RigidBody,
}

impl BodyBundle {
    fn new(
        materials: &mut ResMut<Assets<StandardMaterial>>,
        meshes: &mut ResMut<Assets<Mesh>>,
        position: Transform,
        radius: Scalar,
    ) -> Self {
        // TODO: black body curve based on mass/temp
        let color = Color::LinearRgba(LinearRgba::rgb(10000.0, 0.0, 100.0));
        let mesh = meshes.add(Sphere::new(radius as f32));
        let material = materials.add(StandardMaterial {
            base_color: color,
            // emissive: color.to_linear(),
            ..default()
        });

        Self {
            collider: Collider::sphere(radius),
            gravity_scale: GravityScale(0.0),
            // mass: Mass(100.0), // TODO: scale with collider radius
            material3d: MeshMaterial3d(material),
            mesh3d: Mesh3d(mesh.clone()),
            position,
            rigid_body: RigidBody::Dynamic,
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
            apply_initial_impulses,
            apply_gravitation,
            update_barycenter,
            follow_barycenter,
        )
            .chain(),
    );
    app.add_systems(Update, (quit_on_escape, pause_physics_on_space));

    app.run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Main Camera"),
        Transform::from_translation(Vec3::NEG_Z * 500.0).looking_at(Vec3::ZERO, Vec3::Y),
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
            radius: Some(500.0), // TODO: scale proportional to G
            touch_controls: TouchControls::TwoFingerOrbit,
            trackpad_behavior: TrackpadBehavior::blender_default(),
            trackpad_pinch_to_zoom_enabled: true,
            ..default()
        },
    ));
}

// TODO: scale positions proportional to G
fn spawn_bodies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<SimulationRng>,
    body_count: Res<BodyCount>,
) {
    let BodyCount(body_count) = *body_count;

    for _ in 0..body_count {
        commands.spawn(BodyBundle::new(
            &mut materials,
            &mut meshes,
            Transform::from_translation(random_translation_within_radius(&mut *rng, 50.0).f32()),
            rng.random_range(0.5..=1.0),
        ));
    }
}

fn random_translation_within_radius(rng: &mut SimulationRng, radius: Scalar) -> Vector {
    let theta = rng.random_range(0.0..2.0 * std::f64::consts::PI);
    let phi = libm::acos(rng.random_range(-1.0..1.0));
    let r = radius * libm::cbrt(rng.random_range(0.0..1.0));

    Vector::new(
        r * libm::sin(phi) * libm::cos(theta),
        r * libm::sin(phi) * libm::sin(theta),
        r * libm::cos(phi),
    )
}

// TODO: scale forces proportional to G
fn apply_initial_impulses(
    mut rng: ResMut<SimulationRng>,
    mut bodies: Query<RigidBodyQuery, Without<RigidBodyDisabled>>,
) {
    for mut body in &mut bodies {
        if **body.linear_velocity == Vector::ZERO {
            **body.linear_velocity = random_translation_within_radius(&mut rng, 15.0);
        }
    }
}

// TODO: test
fn apply_gravitation(
    time: ResMut<Time>,
    g: Res<G>,
    mut bodies: Query<RigidBodyQuery, Without<RigidBodyDisabled>>,
) {
    let delta_time = time.delta_secs_f64();
    let mut body_pairs = bodies.iter_combinations_mut();

    // Configuration constants
    const MIN_DISTANCE: Scalar = 0.5; // Minimum allowed distance between bodies
    const MAX_FORCE: Scalar = 1000.0; // Maximum force magnitude to prevent numerical explosions
    const MIN_DISTANCE_SQUARED: Scalar = MIN_DISTANCE * MIN_DISTANCE;

    while let Some([mut body1, mut body2]) = body_pairs.fetch_next() {
        let direction = **body2.position - **body1.position;
        let distance_squared = direction.length_squared();

        if distance_squared < MIN_DISTANCE_SQUARED {
            continue;
        }

        let mass1 = body1.mass.value();
        let mass2 = body2.mass.value();
        let distance = distance_squared.sqrt();
        let direction_normalized = direction / distance;
        let force_magnitude = (**g * mass1 * mass2 / distance_squared).min(MAX_FORCE);
        let acceleration1 = force_magnitude * direction_normalized / mass1;
        let acceleration2 = -force_magnitude * direction_normalized / mass2;

        **body1.linear_velocity += acceleration1 * delta_time;
        **body2.linear_velocity += acceleration2 * delta_time;
    }
}

// TODO: test
fn update_barycenter(
    bodies: Query<RigidBodyQueryReadOnly>,
    mut current_barycenter: ResMut<CurrentBarycenter>,
    mut previous_barycenter: ResMut<PreviousBarycenter>,
) {
    **previous_barycenter = **current_barycenter;

    let (weighted_positions, total_mass): (Vector, Scalar) = bodies
        .iter()
        .map(|b| (**b.position * b.mass.value(), b.mass.value()))
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
) {
    if current_barycenter.is_finite() {
        pan_orbit_camera.force_update = true;
        pan_orbit_camera.target_focus = current_barycenter.as_vec3();
        gizmos.cross(current_barycenter.as_vec3(), 5.0, css::WHITE);
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
    enabled_rigid_bodies: Query<(Entity, RigidBodyQuery), Without<RigidBodyDisabled>>,
    disabled_rigid_bodies: Query<(Entity, RigidBodyQuery), With<RigidBodyDisabled>>,
) {
    if keys.just_pressed(KeyCode::Space) {
        for (entity, _) in &enabled_rigid_bodies {
            commands.entity(entity).insert(RigidBodyDisabled);
        }

        for (entity, _) in &disabled_rigid_bodies {
            commands.entity(entity).remove::<RigidBodyDisabled>();
        }
    }
}
