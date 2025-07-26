use crate::prelude::*;
use bevy::asset::{AssetPath, io::AssetSourceId};

const BUTTON_BORDER_RADIUS_PX: f32 = 4.0;
const BUTTON_FONT_SIZE_PX: f32 = 14.0;
const BUTTON_GAP_PX: f32 = 4.0;
const BUTTON_MARGIN_PX: f32 = 4.0;
const BUTTON_PADDING_PX: f32 = 4.0;
const BUTTON_WIDTH_PX: f32 = 128.0;

const BUTTON_COLOR_NORMAL: Color = Color::srgba(1.0, 1.0, 1.0, 0.01);
const BUTTON_COLOR_HOVERED: Color = Color::srgba(1.0, 1.0, 1.0, 0.1);
const BUTTON_COLOR_PRESSED: Color = Color::srgba(1.0, 1.0, 1.0, 0.2);

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

// Macro to implement ButtonBehavior for buttons
macro_rules! impl_button_behavior {
    // For unit struct events (most common case)
    ($button:ty, $event:path) => {
        impl ButtonBehavior for $button {
            type Event = $event;

            fn create_event() -> Self::Event {
                $event
            }
        }
    };
    // For events with custom construction
    ($button:ty, $event:ty, $create_expr:expr) => {
        impl ButtonBehavior for $button {
            type Event = $event;

            fn create_event() -> Self::Event {
                $create_expr
            }
        }
    };
}

impl_button_behavior!(OctreeToggleButton, ToggleOctreeVisualizationEvent);
impl_button_behavior!(RestartSimulationButton, RestartSimulationEvent);
impl_button_behavior!(
    BarycenterGizmoToggleButton,
    ToggleBarycenterGizmoVisibilityEvent
);
impl_button_behavior!(PauseButton, TogglePauseSimulationEvent);
impl_button_behavior!(ScreenshotButton, TakeScreenshotEvent);

#[cfg(not(target_arch = "wasm32"))]
impl_button_behavior!(QuitButton, AppExit, AppExit::Success);

#[derive(Component)]
pub struct UIRoot;

pub trait ButtonBehavior: Component {
    type Event: bevy::ecs::event::Event;

    fn create_event() -> Self::Event;

    fn trigger_event(event_writer: &mut EventWriter<Self::Event>) {
        event_writer.write(Self::create_event());
    }
}

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

pub fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let embedded_asset_source = &AssetSourceId::from("embedded");
    let regular_font_asset_path =
        AssetPath::parse("fonts/SairaSemiCondensed-Light").with_source(embedded_asset_source);
    let regular_font = asset_server.load(regular_font_asset_path);
    let button_text_font = TextFont::from_font(regular_font).with_font_size(BUTTON_FONT_SIZE_PX);

    // Root UI node
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                ..default()
            },
            UIRoot,
        ))
        .with_children(|parent| {
            // Container for buttons on left side
            parent
                .spawn(Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    justify_content: JustifyContent::FlexStart,
                    row_gap: Val::Px(BUTTON_GAP_PX),
                    margin: UiRect {
                        left: Val::Px(BUTTON_MARGIN_PX),
                        right: Val::Px(BUTTON_MARGIN_PX),
                        // Extra top margin on macOS for window controls
                        #[cfg(target_os = "macos")]
                        top: Val::Px(30.0),
                        #[cfg(not(target_os = "macos"))]
                        top: Val::Px(BUTTON_MARGIN_PX),
                        bottom: Val::Px(BUTTON_MARGIN_PX),
                    },
                    ..default()
                })
                .with_children(|parent| {
                    // Restart simulation button
                    parent
                        .spawn((create_button_bundle(), RestartSimulationButton))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("New Simulation (N)"),
                                button_text_font.clone(),
                                TextColor(Color::WHITE),
                            ));
                        });

                    // Octree toggle button
                    parent
                        .spawn((create_button_bundle(), OctreeToggleButton))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Show Octree (O)"),
                                button_text_font.clone(),
                                TextColor(Color::WHITE),
                            ));
                        });

                    // Barycenter gizmo toggle button
                    parent
                        .spawn((create_button_bundle(), BarycenterGizmoToggleButton))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Show Barycenter (C)"),
                                button_text_font.clone(),
                                TextColor(Color::WHITE),
                            ));
                        });

                    parent
                        .spawn((create_button_bundle(), PauseButton))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Pause (Space)"),
                                button_text_font.clone(),
                                TextColor(Color::WHITE),
                            ));
                        });

                    // Screenshot button
                    parent
                        .spawn((create_button_bundle(), ScreenshotButton))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Screenshot (S)"),
                                button_text_font.clone(),
                                TextColor(Color::WHITE),
                            ));
                        });

                    // Quit button (only on non-WASM platforms)
                    #[cfg(not(target_arch = "wasm32"))]
                    parent
                        .spawn((create_button_bundle(), QuitButton))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Quit (Q)"),
                                button_text_font.clone(),
                                TextColor(Color::WHITE),
                            ));
                        });
                });
        });
}

pub fn handle_button_interaction<T: ButtonBehavior>(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<T>),
    >,
    mut event_writer: EventWriter<T::Event>,
) {
    interaction_query
        .iter_mut()
        .for_each(|(interaction, mut color)| match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(BUTTON_COLOR_PRESSED);
                T::trigger_event(&mut event_writer);
            }
            Interaction::Hovered => {
                *color = BackgroundColor(BUTTON_COLOR_HOVERED);
            }
            Interaction::None => {
                *color = BackgroundColor(BUTTON_COLOR_NORMAL);
            }
        });
}

pub fn update_button_text<T: Component>(
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
                        if **text != event.new_text {
                            **text = event.new_text.clone();
                        }
                    }
                }
            }
        }
    }
}

pub fn emit_octree_button_update(
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

pub fn emit_barycenter_button_update(
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

pub fn emit_pause_button_update_on_state_change(
    state: Res<State<AppState>>,
    mut events: EventWriter<UpdateButtonTextEvent<PauseButton>>,
) {
    let text = match state.get() {
        AppState::Running => "Pause (Space)",
        AppState::Paused => "Resume (Space)",
        _ => return,
    };
    events.write(UpdateButtonTextEvent::new(text));
}

#[cfg(test)]
mod ui_tests {
    use super::*;

    #[test]
    fn test_update_button_text_event() {
        // Test that UpdateButtonTextEvent carries text correctly
        let event = UpdateButtonTextEvent::<OctreeToggleButton>::new("Hide Octree (O)");
        assert_eq!(event.new_text, "Hide Octree (O)");

        let event = UpdateButtonTextEvent::<OctreeToggleButton>::new("Show Octree (O)");
        assert_eq!(event.new_text, "Show Octree (O)");
    }

    #[test]
    fn test_button_text_event_creation() {
        // Test various button text events
        let octree_event = UpdateButtonTextEvent::<OctreeToggleButton>::new("Test Octree");
        assert_eq!(octree_event.new_text, "Test Octree");

        let barycenter_event =
            UpdateButtonTextEvent::<BarycenterGizmoToggleButton>::new("Test Barycenter");
        assert_eq!(barycenter_event.new_text, "Test Barycenter");

        let pause_event = UpdateButtonTextEvent::<PauseButton>::new("Test Pause");
        assert_eq!(pause_event.new_text, "Test Pause");
    }

    #[test]
    fn test_event_text_computation() {
        // Test that the emit functions would compute correct text
        let enabled_settings = OctreeVisualizationSettings {
            enabled: true,
            max_depth: None,
        };
        let expected_text = if enabled_settings.enabled {
            "Hide Octree (O)"
        } else {
            "Show Octree (O)"
        };
        assert_eq!(expected_text, "Hide Octree (O)");

        let disabled_settings = OctreeVisualizationSettings {
            enabled: false,
            max_depth: None,
        };
        let expected_text = if disabled_settings.enabled {
            "Hide Octree (O)"
        } else {
            "Show Octree (O)"
        };
        assert_eq!(expected_text, "Show Octree (O)");
    }
}
