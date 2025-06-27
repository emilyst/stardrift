use crate::components::*;
use crate::config::SimulationConfig;
use crate::resources::*;
use crate::systems::simulation_actions;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;

pub fn setup_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    config: Res<SimulationConfig>,
) {
    // Load the same font as the diagnostic hud
    let embedded_asset_source = &bevy::asset::io::AssetSourceId::from("embedded");
    let regular_font_asset_path = bevy::asset::AssetPath::parse("fonts/BerkeleyMono-Regular")
        .with_source(embedded_asset_source);
    let regular_font = asset_server.load(regular_font_asset_path);
    let button_text_font = TextFont::from_font(regular_font).with_font_size(config.ui.font_size);

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
            // Container for buttons in bottom right corner
            parent
                .spawn(Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexEnd,
                    column_gap: Val::Px(config.ui.button_gap),
                    margin: UiRect::all(Val::Px(config.ui.button_margin)),
                    ..default()
                })
                .with_children(|parent| {
                    // Restart simulation button
                    parent
                        .spawn((
                            Button,
                            Node {
                                padding: UiRect::all(Val::Px(config.ui.button_padding)),
                                display: Display::Flex,
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                row_gap: Val::Px(1.0),
                                ..default()
                            },
                            BorderRadius::all(Val::Px(config.ui.button_border_radius)),
                            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
                            RestartSimulationButton,
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("New Simulation"),
                                button_text_font.clone(),
                                TextColor(Color::WHITE),
                            ));
                        });

                    // Octree toggle button
                    parent
                        .spawn((
                            Button,
                            Node {
                                padding: UiRect::all(Val::Px(config.ui.button_padding)),
                                display: Display::Flex,
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                row_gap: Val::Px(1.0),
                                ..default()
                            },
                            BorderRadius::all(Val::Px(config.ui.button_border_radius)),
                            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
                            OctreeToggleButton,
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Show Octree"),
                                button_text_font.clone(),
                                TextColor(Color::WHITE),
                            ));
                        });

                    // Barycenter gizmo toggle button
                    parent
                        .spawn((
                            Button,
                            Node {
                                padding: UiRect::all(Val::Px(config.ui.button_padding)),
                                display: Display::Flex,
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                row_gap: Val::Px(1.0),
                                ..default()
                            },
                            BorderRadius::all(Val::Px(config.ui.button_border_radius)),
                            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
                            BarycenterGizmoToggleButton,
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Hide Barycenter"),
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
    mut settings: ResMut<OctreeVisualizationSettings>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8));
                simulation_actions::toggle_octree_visualization(&mut settings);
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

pub fn handle_barycenter_gizmo_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<BarycenterGizmoToggleButton>),
    >,
    mut settings: ResMut<BarycenterGizmoVisibility>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8));
                simulation_actions::toggle_barycenter_gizmo_visibility(&mut settings);
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

pub fn handle_restart_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<RestartSimulationButton>),
    >,
    mut commands: Commands,
    simulation_bodies: Query<Entity, With<RigidBody>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<SharedRng>,
    body_count: Res<BodyCount>,
    mut current_barycenter: ResMut<CurrentBarycenter>,
    mut previous_barycenter: ResMut<PreviousBarycenter>,
    octree: ResMut<GravitationalOctree>,
    mut pan_orbit_camera: Single<&mut PanOrbitCamera>,
    config: Res<SimulationConfig>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8));

                simulation_actions::restart_simulation(
                    &mut commands,
                    &simulation_bodies,
                    &mut meshes,
                    &mut materials,
                    &mut rng,
                    &body_count,
                    &mut current_barycenter,
                    &mut previous_barycenter,
                    &octree,
                    &mut pan_orbit_camera,
                    &config,
                );
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

pub fn update_octree_button_text(
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
            for child in children {
                if let Ok(mut text) = text_query.get_mut(*child) {
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
                        "Hide Barycenter".to_string()
                    } else {
                        "Show Barycenter".to_string()
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

        let expected_text_when_disabled = if disabled_settings.enabled {
            "Hide Octree"
        } else {
            "Show Octree"
        };

        assert_eq!(expected_text_when_enabled, "Hide Octree");
        assert_eq!(expected_text_when_disabled, "Show Octree");
    }
}
