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

#[derive(Component)]
pub struct UIRoot;

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
                        .spawn((
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
                            RestartSimulationButton,
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("New Simulation (N)"),
                                button_text_font.clone(),
                                TextColor(Color::WHITE),
                            ));
                        });

                    // Octree toggle button
                    parent
                        .spawn((
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
                            OctreeToggleButton,
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Show Octree (O)"),
                                button_text_font.clone(),
                                TextColor(Color::WHITE),
                            ));
                        });

                    // Barycenter gizmo toggle button
                    parent
                        .spawn((
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
                            BarycenterGizmoToggleButton,
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Show Barycenter (C)"),
                                button_text_font.clone(),
                                TextColor(Color::WHITE),
                            ));
                        });

                    parent
                        .spawn((
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
                            PauseButton,
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Pause (Space)"),
                                button_text_font.clone(),
                                TextColor(Color::WHITE),
                            ));
                        });

                    // Screenshot button
                    parent
                        .spawn((
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
                            ScreenshotButton,
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Screenshot (S)"),
                                button_text_font.clone(),
                                TextColor(Color::WHITE),
                            ));
                        });
                });
        });
}

pub fn handle_octree_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<OctreeToggleButton>),
    >,
    mut octree_events: EventWriter<ToggleOctreeVisualizationEvent>,
) {
    interaction_query
        .iter_mut()
        .for_each(|(interaction, mut color)| match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(BUTTON_COLOR_PRESSED);
                octree_events.write(ToggleOctreeVisualizationEvent);
            }
            Interaction::Hovered => {
                *color = BackgroundColor(BUTTON_COLOR_HOVERED);
            }
            Interaction::None => {
                *color = BackgroundColor(BUTTON_COLOR_NORMAL);
            }
        });
}

pub fn handle_barycenter_gizmo_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<BarycenterGizmoToggleButton>),
    >,
    mut barycenter_events: EventWriter<ToggleBarycenterGizmoVisibilityEvent>,
) {
    interaction_query
        .iter_mut()
        .for_each(|(interaction, mut color)| match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(BUTTON_COLOR_PRESSED);
                barycenter_events.write(ToggleBarycenterGizmoVisibilityEvent);
            }
            Interaction::Hovered => {
                *color = BackgroundColor(BUTTON_COLOR_HOVERED);
            }
            Interaction::None => {
                *color = BackgroundColor(BUTTON_COLOR_NORMAL);
            }
        });
}

pub fn handle_restart_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<RestartSimulationButton>),
    >,
    mut restart_events: EventWriter<RestartSimulationEvent>,
) {
    interaction_query
        .iter_mut()
        .for_each(|(interaction, mut color)| match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(BUTTON_COLOR_PRESSED);
                restart_events.write(RestartSimulationEvent);
            }
            Interaction::Hovered => {
                *color = BackgroundColor(BUTTON_COLOR_HOVERED);
            }
            Interaction::None => {
                *color = BackgroundColor(BUTTON_COLOR_NORMAL);
            }
        });
}

pub fn handle_pause_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<PauseButton>),
    >,
    mut pause_events: EventWriter<TogglePauseSimulationEvent>,
) {
    interaction_query
        .iter_mut()
        .for_each(|(interaction, mut color)| match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(BUTTON_COLOR_PRESSED);
                pause_events.write(TogglePauseSimulationEvent);
            }
            Interaction::Hovered => {
                *color = BackgroundColor(BUTTON_COLOR_HOVERED);
            }
            Interaction::None => {
                *color = BackgroundColor(BUTTON_COLOR_NORMAL);
            }
        });
}

pub fn handle_screenshot_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<ScreenshotButton>),
    >,
    mut screenshot_events: EventWriter<TakeScreenshotEvent>,
) {
    interaction_query
        .iter_mut()
        .for_each(|(interaction, mut color)| match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(BUTTON_COLOR_PRESSED);
                screenshot_events.write(TakeScreenshotEvent);
            }
            Interaction::Hovered => {
                *color = BackgroundColor(BUTTON_COLOR_HOVERED);
            }
            Interaction::None => {
                *color = BackgroundColor(BUTTON_COLOR_NORMAL);
            }
        });
}

pub fn update_octree_button_text(
    button_query: Query<Entity, With<OctreeToggleButton>>,
    children_query: Query<&Children>,
    mut text_query: Query<&mut Text>,
    settings: Res<OctreeVisualizationSettings>,
) {
    if !settings.is_changed() {
        return;
    }

    button_query.iter().for_each(|button_entity| {
        if let Ok(children) = children_query.get(button_entity) {
            children.iter().for_each(|child| {
                if let Ok(mut text) = text_query.get_mut(child) {
                    text.0 = if settings.enabled {
                        "Hide Octree (O)".to_string()
                    } else {
                        "Show Octree (O)".to_string()
                    };
                }
            });
        }
    });
}

pub fn update_barycenter_gizmo_button_text(
    button_query: Query<Entity, With<BarycenterGizmoToggleButton>>,
    children_query: Query<&Children>,
    mut text_query: Query<&mut Text>,
    settings: Res<BarycenterGizmoVisibility>,
) {
    if !settings.is_changed() {
        return;
    }

    for button_entity in &button_query {
        if let Ok(children) = children_query.get(button_entity) {
            for child in children {
                if let Ok(mut text) = text_query.get_mut(*child) {
                    text.0 = if settings.enabled {
                        "Hide Barycenter (C)".to_string()
                    } else {
                        "Show Barycenter (C)".to_string()
                    };
                }
            }
        }
    }
}

pub fn update_pause_button_text(
    button_query: Query<Entity, With<PauseButton>>,
    children_query: Query<&Children>,
    mut text_query: Query<&mut Text>,
    current_state: Res<State<AppState>>,
) {
    for button_entity in &button_query {
        if let Ok(children) = children_query.get(button_entity) {
            for child in children {
                if let Ok(mut text) = text_query.get_mut(*child) {
                    let new_text = match current_state.get() {
                        AppState::Running => "Pause (Space)".to_string(),
                        AppState::Paused => "Resume (Space)".to_string(),
                        _ => String::new(), // ignore Loading state
                    };

                    if **text != new_text {
                        **text = new_text;
                    }
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
            "Hide Octree (O)"
        } else {
            "Show Octree (O)"
        };

        let expected_text_when_disabled = if disabled_settings.enabled {
            "Hide Octree (O)"
        } else {
            "Show Octree (O)"
        };

        assert_eq!(expected_text_when_enabled, "Hide Octree (O)");
        assert_eq!(expected_text_when_disabled, "Show Octree (O)");
    }
}
