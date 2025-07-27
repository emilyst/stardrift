//! Controls plugin - Self-contained plugin pattern
//!
//! This plugin handles all user input (keyboard and UI buttons) and translates
//! them into SimulationCommand events. It provides a unified interface for
//! controlling the simulation, regardless of input method.

use crate::prelude::*;
use bevy::asset::{AssetPath, io::AssetSourceId};

const BUTTON_BORDER_RADIUS_PX: f32 = 5.0;
const BUTTON_FONT_SIZE_PX: f32 = 14.0;
const BUTTON_GAP_PX: f32 = 4.0;
const BUTTON_MARGIN_PX: f32 = 4.0;
const BUTTON_PADDING_PX: f32 = 4.0;
const BUTTON_WIDTH_PX: f32 = 150.0;

const BUTTON_COLOR_NORMAL: Color = Color::srgba(1.0, 1.0, 1.0, 0.01);
const BUTTON_COLOR_HOVERED: Color = Color::srgba(1.0, 1.0, 1.0, 0.1);
const BUTTON_COLOR_PRESSED: Color = Color::srgba(1.0, 1.0, 1.0, 0.2);

pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        // UI setup
        app.add_systems(Startup, setup_controls_ui);

        // Input handlers
        app.add_systems(
            Update,
            (
                keyboard_input_handler,
                button_interaction_handler::<RestartSimulationButton>,
                button_interaction_handler::<OctreeToggleButton>,
                button_interaction_handler::<BarycenterGizmoToggleButton>,
                button_interaction_handler::<PauseButton>,
                button_interaction_handler::<ScreenshotButton>,
                #[cfg(not(target_arch = "wasm32"))]
                quit_button_handler,
                #[cfg(not(target_arch = "wasm32"))]
                quit_on_escape,
            ),
        );

        // Text update systems
        app.add_event::<UpdateButtonTextEvent<OctreeToggleButton>>();
        app.add_event::<UpdateButtonTextEvent<BarycenterGizmoToggleButton>>();
        app.add_event::<UpdateButtonTextEvent<PauseButton>>();

        app.add_systems(
            Update,
            (
                emit_octree_button_update,
                emit_barycenter_button_update,
                update_button_text::<OctreeToggleButton>,
                update_button_text::<BarycenterGizmoToggleButton>,
                update_button_text::<PauseButton>,
            ),
        );

        // Emit pause button update on state changes
        app.add_systems(OnEnter(AppState::Running), emit_pause_button_update);
        app.add_systems(OnEnter(AppState::Paused), emit_pause_button_update);
    }
}

/// Handles keyboard input and emits SimulationCommand events
fn keyboard_input_handler(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: EventWriter<SimulationCommand>,
) {
    for &keycode in keys.get_just_pressed() {
        match keycode {
            KeyCode::KeyN => {
                commands.write(SimulationCommand::Restart);
            }
            KeyCode::Space => {
                commands.write(SimulationCommand::TogglePause);
            }
            KeyCode::KeyO => {
                commands.write(SimulationCommand::ToggleOctreeVisualization);
            }
            KeyCode::KeyC => {
                commands.write(SimulationCommand::ToggleBarycenterGizmo);
            }
            KeyCode::KeyS => {
                commands.write(SimulationCommand::TakeScreenshot);
            }
            _ => {}
        }
    }
}

/// Generic button interaction handler that emits SimulationCommand
fn button_interaction_handler<T: Component + CommandButton>(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<T>),
    >,
    mut command_writer: EventWriter<SimulationCommand>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(BUTTON_COLOR_PRESSED);
                command_writer.write(T::get_command());
            }
            Interaction::Hovered => {
                *color = BackgroundColor(BUTTON_COLOR_HOVERED);
            }
            Interaction::None => {
                *color = BackgroundColor(BUTTON_COLOR_NORMAL);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn quit_button_handler(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<QuitButton>),
    >,
    mut exit: EventWriter<AppExit>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(BUTTON_COLOR_PRESSED);
                exit.write_default();
            }
            Interaction::Hovered => {
                *color = BackgroundColor(BUTTON_COLOR_HOVERED);
            }
            Interaction::None => {
                *color = BackgroundColor(BUTTON_COLOR_NORMAL);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn quit_on_escape(keys: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::KeyQ) {
        exit.write_default();
    }
}

// UI Component definitions
#[derive(Component)]
pub struct OctreeToggleButton;

#[derive(Component)]
pub struct RestartSimulationButton;

#[derive(Component)]
pub struct BarycenterGizmoToggleButton;

#[derive(Component)]
pub struct PauseButton;

#[derive(Component)]
pub struct ScreenshotButton;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Component)]
pub struct QuitButton;

#[derive(Component)]
pub struct UIRoot;

#[inline]
fn create_button_bundle() -> impl Bundle {
    (
        Button,
        Node {
            width: Val::Px(BUTTON_WIDTH_PX),
            padding: UiRect::all(Val::Px(BUTTON_PADDING_PX)),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(1.0),
            ..default()
        },
        BorderRadius::all(Val::Px(BUTTON_BORDER_RADIUS_PX)),
        BackgroundColor(BUTTON_COLOR_NORMAL),
    )
}

/// Sets up the controls UI
fn setup_controls_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let embedded_asset_source = &AssetSourceId::from("embedded");

    let light_font_asset_path =
        AssetPath::parse("fonts/SairaSemiCondensed-Light").with_source(embedded_asset_source);
    let light_font = asset_server.load(light_font_asset_path);
    let light_text_font = TextFont::from_font(light_font).with_font_size(BUTTON_FONT_SIZE_PX);

    let regular_font_asset_path =
        AssetPath::parse("fonts/SairaSemiCondensed-Regular").with_source(embedded_asset_source);
    let regular_font = asset_server.load(regular_font_asset_path);
    let regular_text_font = TextFont::from_font(regular_font).with_font_size(10.0);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(BUTTON_MARGIN_PX),
                left: Val::Px(BUTTON_MARGIN_PX),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                row_gap: Val::Px(BUTTON_GAP_PX),
                ..default()
            },
            UIRoot,
        ))
        .with_children(|parent| {
            // First group: simulation controls
            parent
                .spawn((create_button_bundle(), RestartSimulationButton))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Restart Simulation (N)"),
                        light_text_font.clone(),
                        TextColor(Color::WHITE),
                    ));
                });

            parent
                .spawn((create_button_bundle(), OctreeToggleButton))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Show Octree (O)"),
                        light_text_font.clone(),
                        TextColor(Color::WHITE),
                    ));
                });

            parent
                .spawn((create_button_bundle(), BarycenterGizmoToggleButton))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Show Barycenter (C)"),
                        light_text_font.clone(),
                        TextColor(Color::WHITE),
                    ));
                });

            // Second group: pause and screenshot
            parent
                .spawn((create_button_bundle(), PauseButton))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Pause (Space)"),
                        light_text_font.clone(),
                        TextColor(Color::WHITE),
                    ));
                });

            parent
                .spawn((create_button_bundle(), ScreenshotButton))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Screenshot (S)"),
                        light_text_font.clone(),
                        TextColor(Color::WHITE),
                    ));
                });

            // Add Quit button on non-WASM platforms
            #[cfg(not(target_arch = "wasm32"))]
            {
                parent
                    .spawn((create_button_bundle(), QuitButton))
                    .with_children(|parent| {
                        parent.spawn((
                            Text::new("Quit (Q)"),
                            light_text_font,
                            TextColor(Color::WHITE),
                        ));
                    });
            }

            // Attribution text at the bottom
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(-22.0),
                    ..default()
                },
                Text::new("stardrift // staropolis"),
                regular_text_font,
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
            ));
        });
}

// Text update emission functions
fn emit_octree_button_update(
    settings: Res<OctreeVisualizationSettings>,
    mut events: EventWriter<UpdateButtonTextEvent<OctreeToggleButton>>,
) {
    if settings.is_changed() {
        let text = if settings.enabled {
            "Hide Octree (O)"
        } else {
            "Show Octree (O)"
        };
        events.write(UpdateButtonTextEvent::new(text));
    }
}

fn emit_barycenter_button_update(
    settings: Res<BarycenterGizmoVisibility>,
    mut events: EventWriter<UpdateButtonTextEvent<BarycenterGizmoToggleButton>>,
) {
    if settings.is_changed() {
        let text = if settings.enabled {
            "Hide Barycenter (C)"
        } else {
            "Show Barycenter (C)"
        };
        events.write(UpdateButtonTextEvent::new(text));
    }
}

fn emit_pause_button_update(
    state: Res<State<AppState>>,
    mut events: EventWriter<UpdateButtonTextEvent<PauseButton>>,
) {
    let text = match state.get() {
        AppState::Running => "Pause (Space)",
        AppState::Paused => "Resume (Space)",
    };
    events.write(UpdateButtonTextEvent::new(text));
}

fn update_button_text<T: Component>(
    mut events: EventReader<UpdateButtonTextEvent<T>>,
    button_query: Query<Entity, With<T>>,
    children_query: Query<&Children>,
    mut text_query: Query<&mut Text>,
) {
    for event in events.read() {
        for button_entity in &button_query {
            if let Ok(children) = children_query.get(button_entity) {
                for child in children {
                    if let Ok(mut text) = text_query.get_mut(*child) {
                        *text = Text::new(event.new_text.clone());
                        break;
                    }
                }
            }
        }
    }
}

/// Trait for buttons that emit SimulationCommand
pub trait CommandButton: Component {
    fn get_command() -> SimulationCommand;
}

// CommandButton implementations
impl CommandButton for OctreeToggleButton {
    fn get_command() -> SimulationCommand {
        SimulationCommand::ToggleOctreeVisualization
    }
}

impl CommandButton for RestartSimulationButton {
    fn get_command() -> SimulationCommand {
        SimulationCommand::Restart
    }
}

impl CommandButton for BarycenterGizmoToggleButton {
    fn get_command() -> SimulationCommand {
        SimulationCommand::ToggleBarycenterGizmo
    }
}

impl CommandButton for PauseButton {
    fn get_command() -> SimulationCommand {
        SimulationCommand::TogglePause
    }
}

impl CommandButton for ScreenshotButton {
    fn get_command() -> SimulationCommand {
        SimulationCommand::TakeScreenshot
    }
}
