#![no_std]

mod color;
mod diagnostics;
mod diagnostics_hud;
mod math;
mod octree;

use crate::diagnostics::SimulationDiagnosticsPlugin;
use crate::diagnostics_hud::DiagnosticsHudPlugin;
use crate::octree::Octree;
use crate::octree::OctreeBody;
use avian3d::math::Scalar;
use avian3d::math::Vector;
use avian3d::prelude::*;
use bevy::color::palettes::css;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::diagnostic::EntityCountDiagnosticsPlugin;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy::diagnostic::SystemInformationDiagnosticsPlugin;
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
        Self(1e1)
    }
}

#[derive(Resource, Deref, DerefMut, Copy, Clone, PartialEq, Debug)]
struct BodyCount(usize);

impl Default for BodyCount {
    fn default() -> Self {
        if cfg!(target_arch = "wasm32") {
            Self(100)
        } else {
            Self(100)
        }
    }
}

#[derive(Resource, Deref, DerefMut, Copy, Clone, Default, PartialEq, Debug)]
struct CurrentBarycenter(Vector);

#[derive(Resource, Deref, DerefMut, Copy, Clone, Default, PartialEq, Debug)]
struct PreviousBarycenter(Vector);

#[derive(Resource, Deref, DerefMut, Debug)]
struct GravitationalOctree(Octree);

#[derive(Resource, Default)]
struct OctreeVisualizationSettings {
    enabled: bool,
    max_depth: Option<usize>, // None means show all levels
}

#[derive(Component)]
struct OctreeToggleButton;

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins,
        DiagnosticsHudPlugin,
        EntityCountDiagnosticsPlugin,
        FrameTimeDiagnosticsPlugin::default(),
        LogDiagnosticsPlugin::default(),
        PanOrbitCameraPlugin,
        PhysicsPlugins::default(),
        SimulationDiagnosticsPlugin::default(),
        SystemInformationDiagnosticsPlugin,
    ));

    app.init_resource::<SharedRng>();
    app.init_resource::<GravitationalConstant>();
    app.init_resource::<BodyCount>();
    app.init_resource::<CurrentBarycenter>();
    app.init_resource::<PreviousBarycenter>();
    app.insert_resource(GravitationalOctree(Octree::new(0.5))); // theta = 0.5 for Barnes-Hut approximation
    app.insert_resource(OctreeVisualizationSettings {
        enabled: false,
        ..default()
    });

    app.edit_schedule(FixedUpdate, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            ..default()
        });
    });

    app.add_systems(Startup, (spawn_camera, spawn_bodies));
    app.add_systems(PostStartup, setup_ui);
    app.add_systems(
        FixedUpdate,
        (
            // TODO: move these systems into a Simulation struct
            rebuild_octree,
            apply_gravitation_octree,
            update_barycenter,
            follow_barycenter,
        )
            .chain(),
    );
    app.add_systems(
        Update,
        (
            quit_on_escape,
            pause_physics_on_space,
            toggle_octree_visualization,
            visualize_octree,
            handle_octree_button,
            update_octree_button_text,
        ),
    );

    app.run();
}

fn spawn_camera(mut commands: Commands, body_count: Res<BodyCount>) {
    // TODO: calculate distance at which min sphere radius subtends camera frustum
    let body_distribution_sphere_radius =
        math::min_sphere_radius_for_surface_distribution(**body_count, 200.0, 0.001);

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
            radius: Some((body_distribution_sphere_radius * 2.0) as f32),
            touch_controls: TouchControls::OneFingerOrbit,
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
            math::min_sphere_radius_for_surface_distribution(**body_count, 200.0, 0.001);
        let position = math::random_unit_vector(&mut *rng) * body_distribution_sphere_radius;
        let transform = Transform::from_translation(position.as_vec3());
        let radius = rng.random_range(10.0..=20.0);
        let mesh = meshes.add(Sphere::new(radius as f32));

        let min_temp = 2000.0;
        let max_temp = 15000.0;
        let min_radius = 10.0;
        let max_radius = 20.0;
        let temperature =
            min_temp + (max_temp - min_temp) * (max_radius - radius) / (max_radius - min_radius);
        let bloom_intensity = 100.0;
        let saturation_intensity = 3.0;
        let material = color::emissive_material_for_temp(
            &mut materials,
            temperature,
            bloom_intensity,
            saturation_intensity,
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

fn rebuild_octree(
    bodies: Query<(Entity, &Transform, &ComputedMass), With<RigidBody>>,
    mut octree: ResMut<GravitationalOctree>,
) {
    let octree_bodies: Vec<OctreeBody> = bodies
        .iter()
        .map(|(entity, transform, mass)| OctreeBody {
            entity,
            position: Vector::from(transform.translation),
            mass: mass.value(),
        })
        .collect();

    octree.build(octree_bodies);
}

// Apply gravitational forces using the octree for approximation
fn apply_gravitation_octree(
    time: ResMut<Time>,
    g: Res<GravitationalConstant>,
    octree: Res<GravitationalOctree>,
    mut bodies: Query<
        (Entity, &Transform, &ComputedMass, &mut LinearVelocity),
        (With<RigidBody>, Without<RigidBodyDisabled>),
    >,
) {
    let delta_time = time.delta_secs_f64();

    for (entity, transform, mass, mut velocity) in bodies.iter_mut() {
        let body = OctreeBody {
            entity,
            position: Vector::from(transform.translation),
            mass: mass.value(),
        };

        let force = octree.calculate_force(&body, octree.root.as_ref(), **g);
        let acceleration = force * mass.inverse();
        **velocity += acceleration * delta_time;
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

fn toggle_octree_visualization(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<OctreeVisualizationSettings>,
) {
    for &keycode in keys.get_just_pressed() {
        match keycode {
            KeyCode::KeyO => settings.enabled = !settings.enabled,
            KeyCode::Digit0 => settings.max_depth = None,
            KeyCode::Digit1 => settings.max_depth = Some(1),
            KeyCode::Digit2 => settings.max_depth = Some(2),
            KeyCode::Digit3 => settings.max_depth = Some(3),
            KeyCode::Digit4 => settings.max_depth = Some(4),
            KeyCode::Digit5 => settings.max_depth = Some(5),
            KeyCode::Digit6 => settings.max_depth = Some(6),
            KeyCode::Digit7 => settings.max_depth = Some(7),
            KeyCode::Digit8 => settings.max_depth = Some(8),
            KeyCode::Digit9 => settings.max_depth = Some(9),
            _ => {}
        }
    }
}

fn visualize_octree(
    mut gizmos: Gizmos,
    octree: Res<GravitationalOctree>,
    settings: Res<OctreeVisualizationSettings>,
) {
    if !settings.enabled {
        return;
    }

    let bounds = octree.get_bounds(settings.max_depth);

    for aabb in bounds {
        draw_bounding_box_wireframe_gizmo(&mut gizmos, &aabb, css::WHITE);
    }
}

fn draw_bounding_box_wireframe_gizmo(
    gizmos: &mut Gizmos,
    aabb: &octree::Aabb3d,
    color: impl Into<Color>,
) {
    let min = aabb.min.as_vec3();
    let max = aabb.max.as_vec3();
    let color = color.into();

    let corners = [
        Vec3::new(min.x, min.y, min.z), // 0: min corner
        Vec3::new(max.x, min.y, min.z), // 1: +x
        Vec3::new(max.x, max.y, min.z), // 2: +x+y
        Vec3::new(min.x, max.y, min.z), // 3: +y
        Vec3::new(min.x, min.y, max.z), // 4: +z
        Vec3::new(max.x, min.y, max.z), // 5: +x+z
        Vec3::new(max.x, max.y, max.z), // 6: max corner
        Vec3::new(min.x, max.y, max.z), // 7: +y+z
    ];

    // Bottom face (z = min)
    gizmos.line(corners[0], corners[1], color); // min to +x
    gizmos.line(corners[1], corners[2], color); // +x to +x+y
    gizmos.line(corners[2], corners[3], color); // +x+y to +y
    gizmos.line(corners[3], corners[0], color); // +y to min

    // Top face (z = max)
    gizmos.line(corners[4], corners[5], color); // +z to +x+z
    gizmos.line(corners[5], corners[6], color); // +x+z to max
    gizmos.line(corners[6], corners[7], color); // max to +y+z
    gizmos.line(corners[7], corners[4], color); // +y+z to +z

    // Vertical edges
    gizmos.line(corners[0], corners[4], color); // min to +z
    gizmos.line(corners[1], corners[5], color); // +x to +x+z
    gizmos.line(corners[2], corners[6], color); // +x+y to max
    gizmos.line(corners[3], corners[7], color); // +y to +y+z
}

fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load the same font as the diagnostic hud
    let embedded_asset_source = &bevy::asset::io::AssetSourceId::from("embedded");
    let regular_font_asset_path = bevy::asset::AssetPath::parse("fonts/BerkeleyMono-Regular")
        .with_source(embedded_asset_source);
    let regular_font = asset_server.load(regular_font_asset_path);
    let button_text_font = TextFont::from_font(regular_font).with_font_size(12.0);

    // Root UI node
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::FlexEnd,
            ..default()
        })
        .with_children(|parent| {
            // Octree toggle button in bottom left corner
            parent
                .spawn((
                    Button,
                    Node {
                        top: Val::Px(5.0),
                        right: Val::Px(5.0),
                        margin: UiRect::all(Val::Px(10.0)),
                        padding: UiRect::all(Val::Px(5.0)),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,

                        row_gap: Val::Px(1.0),
                        ..default()
                    },
                    BorderRadius::all(Val::Px(5.0)),
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
                    OctreeToggleButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Show Octree"),
                        button_text_font,
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

fn handle_octree_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<OctreeToggleButton>),
    >,
    mut settings: ResMut<OctreeVisualizationSettings>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8));
                settings.enabled = !settings.enabled;
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 0.8));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8));
            }
        }
    }
}

fn update_octree_button_text(
    button_query: Query<Entity, With<OctreeToggleButton>>,
    children_query: Query<&Children>,
    mut text_query: Query<&mut Text>,
    settings: Res<OctreeVisualizationSettings>,
) {
    if !settings.is_changed() {
        return;
    }

    for button_entity in &button_query {
        if let Ok(children) = children_query.get(button_entity) {
            for child in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    text.0 = if settings.enabled {
                        "Hide Octree".to_string()
                    } else {
                        "Show Octree".to_string()
                    };
                }
            }
        }
    }
}

#[cfg(test)]
mod ui_tests {
    use super::*;

    #[test]
    fn test_octree_button_text_logic() {
        let enabled_settings = OctreeVisualizationSettings {
            enabled: true,
            max_depth: None,
        };
        let disabled_settings = OctreeVisualizationSettings {
            enabled: false,
            max_depth: None,
        };

        let expected_text_when_enabled = if enabled_settings.enabled {
            "Hide Octree"
        } else {
            "Show Octree"
        };
        assert_eq!(expected_text_when_enabled, "Hide Octree");

        let expected_text_when_disabled = if disabled_settings.enabled {
            "Hide Octree"
        } else {
            "Show Octree"
        };
        assert_eq!(expected_text_when_disabled, "Show Octree");
    }
}
