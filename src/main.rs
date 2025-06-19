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
        Self(10.0)
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
    app.add_systems(Update, quit_on_escape);

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
    let v = Vector::new(
        rng.random_range(-radius..radius),
        rng.random_range(-radius..radius),
        rng.random_range(-radius..radius),
    );

    // TODO: normalize to sphere with radius
    v.normalize() * libm::cbrt(rng.random_range(0.0..radius)) * 15.0 // ???
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

    while let Some([mut body1, mut body2]) = body_pairs.fetch_next() {
        let direction = **body2.position - **body1.position;
        let mass1 = *Mass::from(*body1.mass) as Scalar;
        let mass2 = *Mass::from(*body2.mass) as Scalar;
        let force = **g * mass1 * mass2 / direction.length_squared() / 2.0;

        **body1.linear_velocity += force * direction * delta_time;
        **body2.linear_velocity -= force * direction * delta_time;
    }
}

// TODO: test
fn update_barycenter(
    bodies: Query<RigidBodyQueryReadOnly, Without<RigidBodyDisabled>>,
    mut current_barycenter: ResMut<CurrentBarycenter>,
    mut previous_barycenter: ResMut<PreviousBarycenter>,
) {
    **previous_barycenter = **current_barycenter;

    let positions: Vector = bodies.iter().map(|b| **b.position * b.mass.value()).sum();
    let masses: Scalar = bodies.iter().map(|b| b.mass.value()).sum();

    **current_barycenter = Position::from(positions / masses);
}

fn follow_barycenter(
    mut pan_orbit_camera: Single<&mut PanOrbitCamera>,
    mut gizmos: Gizmos,
    current_barycenter: Res<CurrentBarycenter>,
) {
    if !current_barycenter.is_nan() {
        pan_orbit_camera.target_focus = current_barycenter.as_vec3(); // TODO: fix move jitter?
        gizmos.cross(current_barycenter.as_vec3(), 5.0, css::WHITE);
    }
}

fn quit_on_escape(keys: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write_default();
    }
}
