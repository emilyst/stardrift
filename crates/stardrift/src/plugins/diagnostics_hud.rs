//! Diagnostics HUD plugin - Self-contained plugin pattern
//!
//! This plugin follows the self-contained pattern where all systems, components,
//! and resources are defined within the plugin module. This pattern is ideal for
//! independent features that can be cleanly added or removed without affecting
//! other systems.

use crate::resources::BodyCount;
use bevy::asset::AssetPath;
use bevy::asset::io::AssetSourceId;
use bevy::diagnostic::DiagnosticsStore;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use core::time::Duration;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct FrameCountTextNode;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct FpsTextNode;

#[derive(Component)]
pub struct DiagnosticsHudRoot;

// TODO: change detection
#[derive(Resource, Reflect, Debug)]
#[reflect(Resource, Debug)]
pub struct DiagnosticsHudSettings {
    pub enabled: bool,
    pub refresh_interval: Duration,
}

impl Default for DiagnosticsHudSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            refresh_interval: Duration::from_secs_f64(1.0 / 6.0),
        }
    }
}

#[derive(Resource)]
pub struct DiagnosticsHudState {
    pub refresh_timer: Timer,
}

impl Default for DiagnosticsHudState {
    fn default() -> Self {
        Self {
            refresh_timer: Timer::new(
                DiagnosticsHudSettings::default().refresh_interval,
                TimerMode::Repeating,
            ),
        }
    }
}

pub struct DiagnosticsHudPlugin;

impl DiagnosticsHudPlugin {
    fn spawn_diagnostics_hud(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        settings: Res<DiagnosticsHudSettings>,
        body_count: Res<BodyCount>,
    ) {
        let embedded_asset_source = &AssetSourceId::from("embedded");

        let regular_font_asset_path =
            AssetPath::parse("fonts/Saira-Regular").with_source(embedded_asset_source);
        let regular_font = asset_server.load(regular_font_asset_path);
        let regular_text_font = TextFont::from_font(regular_font).with_font_size(12.0);

        let extra_bold_font_asset_path =
            AssetPath::parse("fonts/Saira-ExtraBold").with_source(embedded_asset_source);
        let extra_bold_font = asset_server.load(extra_bold_font_asset_path);
        let extra_bold_text_font = TextFont::from_font(extra_bold_font).with_font_size(12.0);

        let border_radius = BorderRadius::all(Val::Px(5.0));
        let background_color = BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.01));

        // Platform-specific top offset
        let top_offset = if cfg!(target_os = "macos") {
            32.0 // macOS needs clearance for title bar
        } else {
            4.0 // Other platforms use minimal offset
        };

        // Container node for centering with constrained width
        let container_node = Node {
            position_type: PositionType::Absolute,
            top: Val::Px(top_offset),
            width: Val::Percent(100.0),
            display: if settings.enabled {
                Display::Flex
            } else {
                Display::None
            },
            justify_content: JustifyContent::Center,
            ..default()
        };

        // HUD content node with natural sizing
        let hud_node = Node {
            padding: UiRect::all(Val::Px(5.0)),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(1.0),
            ..default()
        };
        let hud_row_node = Node {
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(5.0),
            ..default()
        };

        commands.spawn((
            container_node,
            DiagnosticsHudRoot,
            children![(
                hud_node,
                border_radius,
                background_color,
                children![
                    (
                        hud_row_node.clone(),
                        children![
                            (
                                Text::new("FPS"),
                                Node {
                                    min_width: Val::Px(100.0),
                                    ..default()
                                },
                                TextLayout::new_with_justify(JustifyText::Right),
                                regular_text_font.clone(),
                            ),
                            (
                                FpsTextNode,
                                Node {
                                    min_width: Val::Px(100.0),
                                    ..default()
                                },
                                TextLayout::new_with_justify(JustifyText::Left),
                                Text::new("-"),
                                extra_bold_text_font.clone(),
                            ),
                        ],
                    ),
                    (
                        hud_row_node.clone(),
                        children![
                            (
                                Text::new("Frame count"),
                                Node {
                                    min_width: Val::Px(100.0),
                                    ..default()
                                },
                                TextLayout::new_with_justify(JustifyText::Right),
                                regular_text_font.clone(),
                            ),
                            (
                                FrameCountTextNode,
                                Node {
                                    min_width: Val::Px(100.0),
                                    ..default()
                                },
                                TextLayout::new_with_justify(JustifyText::Left),
                                Text::new("-"),
                                extra_bold_text_font.clone(),
                            ),
                        ],
                    ),
                    (
                        hud_row_node.clone(),
                        children![
                            (
                                Text::new("Body count"),
                                Node {
                                    min_width: Val::Px(100.0),
                                    ..default()
                                },
                                TextLayout::new_with_justify(JustifyText::Right),
                                regular_text_font.clone(),
                            ),
                            (
                                Text::new(format!("{}", **body_count)),
                                Node {
                                    min_width: Val::Px(100.0),
                                    ..default()
                                },
                                TextLayout::new_with_justify(JustifyText::Left),
                                extra_bold_text_font.clone(),
                            ),
                        ],
                    ),
                ],
            )],
        ));
    }

    fn advance_refresh_timer(mut state: ResMut<DiagnosticsHudState>, time: Res<Time>) {
        state.refresh_timer.tick(time.delta());
    }

    fn update_frame_count_text(
        diagnostics: Res<DiagnosticsStore>,
        mut frame_count_text: Single<&mut Text, With<FrameCountTextNode>>,
        state: ResMut<DiagnosticsHudState>,
    ) {
        if state.refresh_timer.finished()
            && let Some(frame_count) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_COUNT)
            && let Some(fps) = frame_count.smoothed()
        {
            ***frame_count_text = format!("{fps}");
        }
    }

    fn update_fps_text(
        diagnostics: Res<DiagnosticsStore>,
        mut fps_text: Single<&mut Text, With<FpsTextNode>>,
        state: Res<DiagnosticsHudState>,
    ) {
        if state.refresh_timer.finished()
            && let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS)
            && let Some(fps) = fps.smoothed()
        {
            ***fps_text = format!("{fps:.2}");
        }
    }

    fn update_diagnostics_hud_visibility(
        settings: Res<DiagnosticsHudSettings>,
        mut root_query: Query<&mut Node, With<DiagnosticsHudRoot>>,
    ) {
        if settings.is_changed() {
            for mut node in &mut root_query {
                node.display = if settings.enabled {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }
    }
}

impl Plugin for DiagnosticsHudPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DiagnosticsHudSettings::default());
        app.insert_resource(DiagnosticsHudState::default());
        app.add_systems(Startup, Self::spawn_diagnostics_hud);
        app.add_systems(
            Update,
            (
                Self::advance_refresh_timer,
                Self::update_frame_count_text,
                Self::update_fps_text,
                Self::update_diagnostics_hud_visibility,
            ),
        );
    }
}
