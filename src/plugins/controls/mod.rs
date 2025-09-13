//! Controls plugin - Self-contained plugin pattern
//!
//! This plugin handles all user input (keyboard and UI buttons) and translates
//! them into SimulationCommand events. It provides a unified interface for
//! controlling the simulation, regardless of input method.

use crate::prelude::*;
use bevy::asset::AssetPath;
use bevy::asset::io::AssetSourceId;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::window::SystemCursorIcon;
use bevy::winit::cursor::CursorIcon;

mod builder;
mod buttons;
mod constants;

pub use builder::ButtonWithLabel;
use builder::ControlsCommandsExt;
use buttons::*;
use constants::*;

pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_controls_ui);

        app.add_systems(
            Update,
            (
                keyboard_input_handler,
                button_interaction_handler::<RestartSimulationButton>,
                button_interaction_handler::<OctreeToggleButton>,
                button_interaction_handler::<BarycenterGizmoToggleButton>,
                button_interaction_handler::<PauseButton>,
                button_interaction_handler::<ScreenshotButton>,
                button_interaction_handler::<TrailsToggleButton>,
                button_interaction_handler::<DiagnosticsHudToggleButton>,
                #[cfg(not(target_arch = "wasm32"))]
                quit_button_handler,
                #[cfg(not(target_arch = "wasm32"))]
                quit_on_escape,
            ),
        );

        app.add_systems(
            Startup,
            (
                octree::sync_octree_button_text,
                barycenter::sync_barycenter_button_text,
                trails::sync_trails_button_text,
                diagnostics::sync_diagnostics_button_text,
                pause::sync_pause_button_text,
            )
                .after(setup_controls_ui),
        );

        app.add_systems(
            Update,
            (
                octree::sync_octree_button_text,
                barycenter::sync_barycenter_button_text,
                trails::sync_trails_button_text,
                diagnostics::sync_diagnostics_button_text,
                pause::sync_pause_button_text,
            ),
        );

        app.add_systems(OnEnter(AppState::Running), pause::sync_pause_button_text);
        app.add_systems(OnEnter(AppState::Paused), pause::sync_pause_button_text);
    }
}

fn keyboard_input_handler(
    mut keyboard_events: EventReader<KeyboardInput>,
    mut commands: EventWriter<SimulationCommand>,
) {
    for event in keyboard_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        match &event.logical_key {
            Key::Character(c) => {
                let ch = c.to_lowercase();
                match ch.as_str() {
                    "n" => {
                        commands.write(SimulationCommand::Restart);
                    }
                    "o" => {
                        commands.write(SimulationCommand::ToggleOctreeVisualization);
                    }
                    "c" => {
                        commands.write(SimulationCommand::ToggleBarycenterGizmo);
                    }
                    "s" => {
                        commands.write(SimulationCommand::TakeScreenshot);
                    }
                    "t" => {
                        commands.write(SimulationCommand::ToggleTrailsVisualization);
                    }
                    "d" => {
                        commands.write(SimulationCommand::ToggleDiagnosticsHud);
                    }
                    _ => {}
                }
            }
            Key::Space => {
                commands.write(SimulationCommand::TogglePause);
            }
            _ => {}
        }
    }
}

#[allow(clippy::type_complexity)]
fn button_interaction_handler<T: ButtonWithLabel>(
    mut commands: Commands,
    window: Single<Entity, With<Window>>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<T>),
    >,
    mut command_writer: EventWriter<SimulationCommand>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                commands
                    .entity(*window)
                    .insert(CursorIcon::System(SystemCursorIcon::Pointer));

                *color = BackgroundColor(BUTTON_COLOR_PRESSED);
                command_writer.write(T::command());
            }
            Interaction::Hovered => {
                commands
                    .entity(*window)
                    .insert(CursorIcon::System(SystemCursorIcon::Pointer));

                *color = BackgroundColor(BUTTON_COLOR_HOVERED);
            }
            Interaction::None => {
                commands
                    .entity(*window)
                    .insert(CursorIcon::System(SystemCursorIcon::Default));

                *color = BackgroundColor(BUTTON_COLOR_NORMAL);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::type_complexity)]
fn quit_button_handler(
    mut commands: Commands,
    window: Single<Entity, With<Window>>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<QuitButton>),
    >,
    mut exit: EventWriter<AppExit>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                commands
                    .entity(*window)
                    .insert(CursorIcon::System(SystemCursorIcon::Pointer));

                *color = BackgroundColor(BUTTON_COLOR_PRESSED);
                exit.write_default();
            }
            Interaction::Hovered => {
                commands
                    .entity(*window)
                    .insert(CursorIcon::System(SystemCursorIcon::Pointer));

                *color = BackgroundColor(BUTTON_COLOR_HOVERED);
            }
            Interaction::None => {
                commands
                    .entity(*window)
                    .insert(CursorIcon::System(SystemCursorIcon::Default));

                *color = BackgroundColor(BUTTON_COLOR_NORMAL);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn quit_on_escape(mut keyboard_events: EventReader<KeyboardInput>, mut exit: EventWriter<AppExit>) {
    use bevy::input::ButtonState;

    for event in keyboard_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        match &event.logical_key {
            Key::Escape => {
                exit.write_default();
            }
            Key::Character(c) if c.to_lowercase() == "q" => {
                exit.write_default();
            }
            _ => {}
        }
    }
}

#[derive(Component)]
pub struct UIRoot;

fn setup_controls_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let embedded_asset_source = &AssetSourceId::from("embedded");
    let font_asset_path = AssetPath::parse("fonts/Saira-Medium").with_source(embedded_asset_source);
    let font = asset_server.load(font_asset_path);

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
            parent.spawn_control_button::<RestartSimulationButton>(font.clone());
            parent.spawn_control_button::<OctreeToggleButton>(font.clone());
            parent.spawn_control_button::<BarycenterGizmoToggleButton>(font.clone());
            parent.spawn_control_button::<TrailsToggleButton>(font.clone());
            parent.spawn_control_button::<DiagnosticsHudToggleButton>(font.clone());
            parent.spawn_control_button::<PauseButton>(font.clone());
            parent.spawn_control_button::<ScreenshotButton>(font.clone());
            #[cfg(not(target_arch = "wasm32"))]
            parent.spawn_control_button::<QuitButton>(font);
        });
}
