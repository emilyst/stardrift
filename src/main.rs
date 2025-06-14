#![allow(dead_code)]
#![allow(unused_assignments)]
#![allow(unused_doc_comments)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(unused_parens)]
#![allow(unused_variables)]

use avian3d::math::AsF32;
use avian3d::math::{Scalar, Vector};
use avian3d::prelude::*;
use bevy::color::palettes::css;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;
use bevy_panorbit_camera::TouchControls;
use bevy_panorbit_camera::{PanOrbitCameraPlugin, TrackpadBehavior};
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

const G: Scalar = 0.1;
const NUM_BODIES: usize = 100;

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct SimulationRng(ChaCha8Rng);

impl Default for SimulationRng {
    fn default() -> Self {
        Self(ChaCha8Rng::from_os_rng())
    }
}

#[derive(Resource, Copy, Clone, PartialEq, Debug)]
struct HudRefreshPeriod(Scalar);

impl Default for HudRefreshPeriod {
    fn default() -> Self {
        Self(0.1)
    }
}

#[derive(Resource, Copy, Clone, Default, PartialEq, Debug)]
struct PreviousHudRefreshTime(Scalar);

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct FpsHud;

#[derive(Resource, Copy, Clone, Default, PartialEq, Debug)]
struct CurrentBarycenterPosition(Vector);

#[derive(Resource, Copy, Clone, Default, PartialEq, Debug)]
struct PreviousBarycenterPosition(Vector);

#[derive(Bundle, Clone, Debug, Default)]
struct BodyBundle {
    collider: Collider,
    gravity_scale: GravityScale,
    // mass: Mass,
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
        PanOrbitCameraPlugin,
        PhysicsPlugins::default(),
        FrameTimeDiagnosticsPlugin {
            smoothing_factor: 0.25,
            ..default()
        },
    ));

    app.insert_resource(SimulationRng::default());
    app.insert_resource(CurrentBarycenterPosition::default());
    app.insert_resource(PreviousBarycenterPosition::default());
    app.insert_resource(PreviousHudRefreshTime::default());
    app.insert_resource(HudRefreshPeriod::default());

    app.add_systems(Startup, (spawn_camera, spawn_bodies, spawn_hud));

    app.add_systems(
        FixedUpdate,
        (
            apply_initial_impulses
                .after(spawn_bodies)
                .before(apply_gravitation), // TODO: add to correct schedule
            (apply_gravitation, update_barycenter, follow_barycenter).chain(),
        ),
    );

    app.add_systems(Update, (refresh_fps_hud));

    app.run();
}

fn spawn_camera(mut commands: Commands /*, asset_server: Res<AssetServer>*/) {
    commands.spawn((
        Name::new("Main Camera"),
        PanOrbitCamera {
            touch_controls: TouchControls::TwoFingerOrbit,
            trackpad_behavior: TrackpadBehavior::blender_default(),
            focus: Vec3::ZERO,
            radius: Some(500.0), // TODO: scale proportional to G
            ..default()
        },
        Camera {
            hdr: true,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Tonemapping::TonyMcMapface,
        Bloom::NATURAL,
        Msaa::default(),
    ));
}

fn spawn_bodies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<SimulationRng>,
) {
    // TODO: scale positions proportional to G
    for _ in 0..NUM_BODIES {
        commands.spawn(BodyBundle::new(
            &mut materials,
            &mut meshes,
            Transform::from_translation(random_translation_within_radius(&mut *rng, 20.0).f32()),
            rng.random_range(0.5..=2.0),
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

fn apply_initial_impulses(
    mut rng: ResMut<SimulationRng>,
    mut bodies: Query<RigidBodyQuery, Without<RigidBodyDisabled>>,
) {
    for mut body in &mut bodies {
        if body.linear_velocity.0 == Vector::ZERO {
            // TODO: scale forces proportional to G
            body.linear_velocity.0 = random_translation_within_radius(&mut rng, 15.0);
        }
    }
}

fn spawn_hud(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            Text::new("FPS: "),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(5.0),
                right: Val::Px(15.0),
                ..default()
            },
        ))
        .with_child((
            TextSpan::default(),
            (
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(css::WHITE.into()),
            ),
            FpsHud,
        ));
}

fn refresh_fps_hud(
    diagnostics: Res<DiagnosticsStore>,
    hud_refresh_period: Res<HudRefreshPeriod>,
    mut previous_hud_update_time: ResMut<PreviousHudRefreshTime>,
    mut query: Query<&mut TextSpan, With<FpsHud>>,
    time: Res<Time>,
) {
    let PreviousHudRefreshTime(previous_hud_refresh_time) = previous_hud_update_time.into_inner();

    if time.elapsed_secs_f64() - *previous_hud_refresh_time >= hud_refresh_period.0 {
        for mut span in &mut query {
            if let Some(mut fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
                if let Some(value) = fps.smoothed() {
                    **span = format!("{value:.2}");
                }
            }
        }

        *previous_hud_refresh_time = time.elapsed_secs_f64();
    }
}

fn apply_gravitation(
    time: ResMut<Time>,
    mut bodies: Query<RigidBodyQuery, Without<RigidBodyDisabled>>,
) {
    // TODO: test
    let delta_time = time.delta_secs_f64();
    let mut body_pairs = bodies.iter_combinations_mut();

    while let Some([mut body1, mut body2]) = body_pairs.fetch_next() {
        let direction = body2.position.0 - body1.position.0;
        let force = G * body1.mass.value() * body2.mass.value() / direction.length_squared() / 2.0;

        body1.linear_velocity.0 += force * direction * delta_time;
        body2.linear_velocity.0 -= force * direction * delta_time;
    }
}

fn update_barycenter(
    bodies: Query<RigidBodyQueryReadOnly, Without<RigidBodyDisabled>>,
    mut current_barycenter_position: ResMut<CurrentBarycenterPosition>,
    mut previous_barycenter_position: ResMut<PreviousBarycenterPosition>,
) {
    // TODO: reduce
    // TODO: test
    let mut mass_position_accumulator = Vector::ZERO;
    let mut mass_accumulator: Scalar = 0.0;
    for body in bodies {
        mass_position_accumulator += body.position.0 * body.mass.value();
        mass_accumulator += body.mass.value();
    }
    mass_position_accumulator /= mass_accumulator;

    previous_barycenter_position.0 = current_barycenter_position.0;
    current_barycenter_position.0 = mass_position_accumulator;
}

fn follow_barycenter(
    mut pan_orbit_camera: Single<&mut PanOrbitCamera>,
    // camera_transform: Single<&mut Transform, With<Camera>>,
    mut gizmos: Gizmos,
    current_barycenter_position: Res<CurrentBarycenterPosition>,
) {
    let current_barycenter = current_barycenter_position.0.as_vec3();

    pan_orbit_camera.target_focus = current_barycenter; // TODO: fix move jitter?
    gizmos.cross(current_barycenter, 5.0, css::WHITE);
}
