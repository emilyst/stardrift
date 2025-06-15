use avian3d::math::AsF32;
use avian3d::math::Scalar;
use avian3d::math::Vector;
use avian3d::prelude::*;
use bevy::color::palettes::css;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::diagnostic::DiagnosticsStore;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
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

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct SimulationRng(ChaCha8Rng);

impl Default for SimulationRng {
    fn default() -> Self {
        Self(ChaCha8Rng::from_os_rng())
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

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct FpsHudLabel;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct FpsHudValue;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct BarycenterHudLabel;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct BarycenterHudValue;

#[derive(Resource, Debug, Reflect)]
#[reflect(Resource, Debug)]
pub struct SimulationDiagnosticsUiSettings {
    pub enabled: bool,
}

impl Default for SimulationDiagnosticsUiSettings {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct SimulationHud;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct SimulationHudGroup;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct SimulationHudRow;

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
        PanOrbitCameraPlugin,
        PhysicsPlugins::default(),
        FrameTimeDiagnosticsPlugin {
            smoothing_factor: 0.5,
            ..default()
        },
        PhysicsDiagnosticsPlugin,
    ));

    app.insert_resource(BodyCount::default());
    app.insert_resource(CurrentBarycenter::default());
    app.insert_resource(G::default());
    app.insert_resource(PreviousBarycenter::default());
    app.insert_resource(SimulationRng::default());
    app.insert_resource(SimulationDiagnosticsUiSettings::default());

    app.edit_schedule(Update, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            ..default()
        });
    });

    app.add_systems(Startup, (spawn_camera, spawn_bodies, spawn_hud));
    app.add_systems(
        FixedUpdate,
        (
            apply_initial_impulses,
            apply_gravitation,
            update_barycenter,
            follow_barycenter,
            refresh_fps_hud_value,
            refresh_barycenter_hud_value,
        )
            .chain(),
    );

    app.run();
}

fn spawn_camera(mut commands: Commands) {
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

// liberally borrowed from avian3d physics diagnostics
fn spawn_hud(
    mut commands: Commands,
    settings: Res<SimulationDiagnosticsUiSettings>,
    asset_server: Res<AssetServer>,
) {
    let ui_monospace: Handle<Font> = asset_server.load("fonts/BerkeleyMono-Retina.otf");

    commands
        .spawn((
            Name::new("Simulation HUD"),
            SimulationHud,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(5.0),
                right: Val::Px(5.0),
                padding: UiRect::all(Val::Px(5.0)),
                display: if settings.enabled {
                    Display::Flex
                } else {
                    Display::None
                },
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(1.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.75)),
            BorderRadius::all(Val::Px(5.0)),
        ))
        .with_children(|commands| {
            commands
                .spawn((
                    SimulationHudGroup,
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(1.0),
                        ..default()
                    },
                ))
                .with_children(|commands| {
                    commands
                        .spawn((
                            SimulationHudRow,
                            Node {
                                display: Display::Flex,
                                justify_content: JustifyContent::SpaceBetween,
                                column_gap: Val::Px(10.0),
                                ..default()
                            },
                        ))
                        .with_children(|commands| {
                            commands.spawn((
                                FpsHudLabel,
                                Text::new("FPS"),
                                TextFont {
                                    font: ui_monospace.clone(),
                                    font_size: 10.0,
                                    line_height: Default::default(),
                                    font_smoothing: Default::default(),
                                },
                            ));

                            commands.spawn(Node::default()).with_children(|commands| {
                                commands.spawn((
                                    FpsHudValue,
                                    Text::new("-"),
                                    TextFont {
                                        font: ui_monospace.clone(),
                                        font_size: 10.0,
                                        line_height: Default::default(),
                                        font_smoothing: Default::default(),
                                    },
                                ));
                            });
                        });
                });

            commands
                .spawn((
                    SimulationHudGroup,
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(1.0),
                        ..default()
                    },
                ))
                .with_children(|commands| {
                    commands
                        .spawn((
                            SimulationHudRow,
                            Node {
                                display: Display::Flex,
                                justify_content: JustifyContent::SpaceBetween,
                                column_gap: Val::Px(20.0),
                                ..default()
                            },
                        ))
                        .with_children(|commands| {
                            commands.spawn((
                                BarycenterHudLabel,
                                Text::new("Barycenter"),
                                TextFont {
                                    font: ui_monospace.clone(),
                                    font_size: 10.0,
                                    line_height: Default::default(),
                                    font_smoothing: Default::default(),
                                },
                            ));

                            commands.spawn(Node::default()).with_children(|commands| {
                                commands.spawn((
                                    BarycenterHudValue,
                                    Text::new("-"),
                                    TextFont {
                                        font: ui_monospace.clone(),
                                        font_size: 10.0,
                                        line_height: Default::default(),
                                        font_smoothing: Default::default(),
                                    },
                                ));
                            });
                        });
                });
        });
}

// TODO: test
fn apply_gravitation(
    time: ResMut<Time>,
    g: Res<G>,
    mut bodies: Query<RigidBodyQuery, Without<RigidBodyDisabled>>,
) {
    let g = **g;
    let delta_time = time.delta_secs_f64();

    let mut body_pairs = bodies.iter_combinations_mut();
    while let Some([mut body1, mut body2]) = body_pairs.fetch_next() {
        let direction = **body2.position - **body1.position;
        let mass1 = *Mass::from(*body1.mass) as Scalar;
        let mass2 = *Mass::from(*body2.mass) as Scalar;
        let force = g * mass1 * mass2 / direction.length_squared() / 2.0;

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
    let pos_acc: Vector = bodies.iter().map(|b| **b.position).sum();
    let mass_acc: Scalar = bodies.iter().map(|b| *Mass::from(*b.mass) as Scalar).sum();

    **previous_barycenter = **current_barycenter;
    **current_barycenter = Position::from(pos_acc / mass_acc);
}

fn follow_barycenter(
    mut pan_orbit_camera: Single<&mut PanOrbitCamera>,
    mut gizmos: Gizmos,
    current_barycenter: Res<CurrentBarycenter>,
) {
    pan_orbit_camera.target_focus = current_barycenter.as_vec3(); // TODO: fix move jitter?
    gizmos.cross(current_barycenter.as_vec3(), 5.0, css::WHITE);
}

fn refresh_fps_hud_value(
    diagnostics: Res<DiagnosticsStore>,
    mut fps_hud_value: Single<&mut Text, With<FpsHudValue>>,
) {
    if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(value) = fps.smoothed() {
            ***fps_hud_value = format!("{value:.2}");
        }
    }
}

fn refresh_barycenter_hud_value(
    current_barycenter: Res<CurrentBarycenter>,
    mut barycenter_hud_value: Single<&mut Text, With<BarycenterHudValue>>,
) {
    let value = ***current_barycenter;
    ***barycenter_hud_value = format!("{value:.2}");
}
