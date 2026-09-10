//! Diagnostics HUD plugin - Self-contained plugin pattern
//!
//! This plugin follows the self-contained pattern where all systems, components,
//! and resources are defined within the plugin module. This pattern is ideal for
//! independent features that can be cleanly added or removed without affecting
//! other systems.

use crate::physics::resources::BhProbeState;
use crate::plugins::simulation_diagnostics::SimulationDiagnosticsPlugin;
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

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct BodyCountTextNode;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct BhErrorL2TextNode;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct BhErrorMaxTextNode;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct BhChurnTextNode;

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

/// One label/value HUD row; `marker` tags the value text for updates.
fn hud_row(
    label: &str,
    initial: String,
    marker: impl Component,
    regular_font: &TextFont,
    bold_font: &TextFont,
) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(5.0),
            ..default()
        },
        children![
            (
                Text::new(label),
                Node {
                    min_width: Val::Px(100.0),
                    ..default()
                },
                TextLayout::justify(Justify::Right),
                regular_font.clone(),
            ),
            (
                marker,
                Node {
                    min_width: Val::Px(100.0),
                    ..default()
                },
                TextLayout::justify(Justify::Left),
                Text::new(initial),
                bold_font.clone(),
            ),
        ],
    )
}

pub struct DiagnosticsHudPlugin;

impl DiagnosticsHudPlugin {
    fn spawn_diagnostics_hud(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        settings: Res<DiagnosticsHudSettings>,
        body_count: Res<BodyCount>,
        bh_probe: Res<BhProbeState>,
    ) {
        let embedded_asset_source = &AssetSourceId::from("embedded");

        let regular_font_asset_path =
            AssetPath::parse("fonts/Saira-Regular").with_source(embedded_asset_source);
        let regular_font = asset_server.load(regular_font_asset_path);
        let regular_text_font = TextFont {
            font: regular_font.into(),
            font_size: FontSize::Px(12.0),
            ..default()
        };

        let extra_bold_font_asset_path =
            AssetPath::parse("fonts/Saira-ExtraBold").with_source(embedded_asset_source);
        let extra_bold_font = asset_server.load(extra_bold_font_asset_path);
        let extra_bold_text_font = TextFont {
            font: extra_bold_font.into(),
            font_size: FontSize::Px(12.0),
            ..default()
        };

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
            border_radius: BorderRadius::all(Val::Px(5.0)),
            ..default()
        };

        commands
            .spawn((container_node, DiagnosticsHudRoot))
            .with_children(|container| {
                container
                    .spawn((hud_node, background_color))
                    .with_children(|hud| {
                        hud.spawn(hud_row(
                            "FPS",
                            "-".into(),
                            FpsTextNode,
                            &regular_text_font,
                            &extra_bold_text_font,
                        ));
                        hud.spawn(hud_row(
                            "Frame count",
                            "-".into(),
                            FrameCountTextNode,
                            &regular_text_font,
                            &extra_bold_text_font,
                        ));
                        hud.spawn(hud_row(
                            "Body count",
                            format!("{}", **body_count),
                            BodyCountTextNode,
                            &regular_text_font,
                            &extra_bold_text_font,
                        ));
                        if bh_probe.enabled {
                            hud.spawn(hud_row(
                                "BH error L2",
                                "-".into(),
                                BhErrorL2TextNode,
                                &regular_text_font,
                                &extra_bold_text_font,
                            ));
                            hud.spawn(hud_row(
                                "BH error max",
                                "-".into(),
                                BhErrorMaxTextNode,
                                &regular_text_font,
                                &extra_bold_text_font,
                            ));
                            hud.spawn(hud_row(
                                "BH churn",
                                "-".into(),
                                BhChurnTextNode,
                                &regular_text_font,
                                &extra_bold_text_font,
                            ));
                        }
                    });
            });
    }

    fn advance_refresh_timer(mut state: ResMut<DiagnosticsHudState>, time: Res<Time>) {
        state.refresh_timer.tick(time.delta());
    }

    fn update_frame_count_text(
        diagnostics: Res<DiagnosticsStore>,
        mut frame_count_text: Single<&mut Text, With<FrameCountTextNode>>,
        state: ResMut<DiagnosticsHudState>,
    ) {
        if state.refresh_timer.is_finished()
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
        if state.refresh_timer.is_finished()
            && let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS)
            && let Some(fps) = fps.smoothed()
        {
            ***fps_text = format!("{fps:.2}");
        }
    }

    fn update_body_count_text(
        bodies: Query<(), With<crate::physics::components::PhysicsBody>>,
        mut body_count_text: Single<&mut Text, With<BodyCountTextNode>>,
        state: Res<DiagnosticsHudState>,
    ) {
        // Live entity count, not the BodyCount resource: that resource is the
        // configured spawn count, which collision merges rightly leave alone.
        if state.refresh_timer.is_finished() {
            ***body_count_text = format!("{}", bodies.iter().count());
        }
    }

    /// The probe rows exist only when the probe is enabled; `Single` skips
    /// these systems otherwise.
    fn update_bh_error_l2_text(
        diagnostics: Res<DiagnosticsStore>,
        mut text: Single<&mut Text, With<BhErrorL2TextNode>>,
        state: Res<DiagnosticsHudState>,
    ) {
        if state.refresh_timer.is_finished()
            && let Some(value) = diagnostics
                .get(&SimulationDiagnosticsPlugin::BH_ACCEL_ERROR_L2)
                .and_then(|d| d.value())
        {
            ***text = format!("{value:.2e}");
        }
    }

    fn update_bh_error_max_text(
        diagnostics: Res<DiagnosticsStore>,
        mut text: Single<&mut Text, With<BhErrorMaxTextNode>>,
        state: Res<DiagnosticsHudState>,
    ) {
        if state.refresh_timer.is_finished()
            && let Some(value) = diagnostics
                .get(&SimulationDiagnosticsPlugin::BH_ACCEL_ERROR_MAX)
                .and_then(|d| d.value())
        {
            ***text = format!("{value:.2e}");
        }
    }

    fn update_bh_churn_text(
        diagnostics: Res<DiagnosticsStore>,
        mut text: Single<&mut Text, With<BhChurnTextNode>>,
        state: Res<DiagnosticsHudState>,
    ) {
        if state.refresh_timer.is_finished()
            && let Some(value) = diagnostics
                .get(&SimulationDiagnosticsPlugin::BH_TOPOLOGY_CHURN)
                .and_then(|d| d.value())
        {
            ***text = format!("{:.1}%", value * 100.0);
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
                Self::update_body_count_text,
                Self::update_bh_error_l2_text,
                Self::update_bh_error_max_text,
                Self::update_bh_churn_text,
                Self::update_diagnostics_hud_visibility,
            ),
        );
    }
}
