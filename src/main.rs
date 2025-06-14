// #![allow(dead_code)]
// #![allow(unused_assignments)]
// #![allow(unused_doc_comments)]
// #![allow(unused_imports)]
// #![allow(unused_mut)]
// #![allow(unused_parens)]
// #![allow(unused_variables)]

use avian3d::math::AsF32;
use avian3d::math::{Scalar, Vector};
use avian3d::prelude::*;
use bevy::color::palettes::css;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;
use bevy_panorbit_camera::TouchControls;
use bevy_panorbit_camera::{PanOrbitCameraPlugin, TrackpadBehavior};
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

const G: Scalar = 0.01;
const NUM_BODIES: usize = 3;

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct SimulationRng(ChaCha8Rng);

impl Default for SimulationRng {
    fn default() -> Self {
        Self(ChaCha8Rng::from_os_rng())
    }
}

#[derive(Resource, Copy, Clone, Default, PartialEq, Debug)]
struct Barycenter {
    current: Vector,
    previous: Vector,
}

#[derive(Bundle, Clone, Debug, Default)]
struct BodyBundle {
    collider: Collider,
    gravity_scale: GravityScale,
    mass: Mass,
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
            mass: Mass(100.0), // TODO: scale with collider radius
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
    ));

    app.insert_resource(SimulationRng::default());
    app.insert_resource(Barycenter::default());

    app.add_systems(Startup, spawn_camera);
    app.add_systems(Startup, spawn_bodies);
    app.add_systems(
        FixedUpdate,
        apply_initial_impulses
            .after(spawn_bodies)
            .before(apply_gravitation),
    );
    app.add_systems(FixedUpdate, apply_gravitation);
    app.add_systems(FixedUpdate, update_barycenter.after(apply_gravitation));
    app.add_systems(FixedUpdate, follow_barycenter.after(apply_gravitation));

    app.run();
}

fn spawn_camera(mut commands: Commands /*, asset_server: Res<AssetServer>*/) {
    commands.spawn((
        Name::new("Main Camera"),
        PanOrbitCamera {
            touch_controls: TouchControls::TwoFingerOrbit,
            trackpad_behavior: TrackpadBehavior::blender_default(),
            focus: Vec3::ZERO,
            radius: Some(250.0), // TODO: scale proportional to G
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
    Vector::new(
        rng.random_range(-radius..radius),
        rng.random_range(-radius..radius),
        rng.random_range(-radius..radius),
    )

    // TODO: normalize to sphere with radius
    // v.normalize() * libm::cbrt(rng.random_range(0.0..radius)) * 15.0 // ???
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
    mut barycenter: ResMut<Barycenter>,
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

    barycenter.previous = barycenter.current;
    barycenter.current = mass_position_accumulator;
}

fn follow_barycenter(
    mut pan_orbit_camera: Single<&mut PanOrbitCamera>,
    // camera_transform: Single<&mut Transform, With<Camera>>,
    mut gizmos: Gizmos,
    barycenter: Res<Barycenter>,
) {
    let current_barycenter = barycenter.current.as_vec3();
    pan_orbit_camera.target_focus = current_barycenter; // TODO: fix move jitter?
    gizmos.cross(current_barycenter, 5.0, css::WHITE);
}
