//! Diagnostics HUD (Heads-Up Display) module.
//!
//! This module provides a visual overlay that displays real-time diagnostic information
//! for the many-body simulation. The HUD shows key performance and simulation metrics
//! in an easy-to-read format overlaid on the main simulation view.
//!
//! # Displayed Information
//!
//! The HUD displays the following diagnostic data:
//!
//! - **Frame Count**: Total number of frames rendered since application start
//! - **FPS (Frames Per Second)**: Current rendering performance
//! - **Barycenter Position**: Real-time coordinates of the system's center of mass
//!
//! # Features
//!
//! - **Configurable refresh rate**: Control how often the HUD updates to balance
//!   performance with information freshness
//! - **Toggle visibility**: Enable or disable the HUD display (TBD)
//! - **Custom font**: Uses embedded Berkeley Mono fonts for consistent appearance
//! - **Responsive layout**: Positioned in the top-right corner with proper spacing
//!
//! # Main Components
//!
//! - [`DiagnosticsHudPlugin`]: The main plugin that sets up the HUD system
//! - [`DiagnosticsHudSettings`]: Configuration resource for HUD behavior
//! - [`DiagnosticsHudState`]: Runtime state management for refresh timing
//!
//! # Usage
//!
//! Add the plugin to your Bevy app to enable the diagnostics HUD:
//!
//! ```rust,ignore
//! app.add_plugins(DiagnosticsHudPlugin);
//! ```
//!
//! The HUD can be configured by modifying the [`DiagnosticsHudSettings`] resource:
//!
//! ```rust,ignore
//! app.insert_resource(DiagnosticsHudSettings {
//!     enabled: true,
//!     refresh_interval: Duration::from_secs_f64(1.0 / 10.0), // 10 Hz refresh rate
//! });
//! ```

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
struct BarycenterTextNode;

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
            enabled: true,
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
            AssetPath::parse("fonts/BerkeleyMono-Regular").with_source(embedded_asset_source);
        let regular_font = asset_server.load(regular_font_asset_path);
        let regular_text_font = TextFont::from_font(regular_font).with_font_size(12.0);

        let bold_font_asset_path =
            AssetPath::parse("fonts/BerkeleyMono-Bold").with_source(embedded_asset_source);
        let bold_font = asset_server.load(bold_font_asset_path);
        let bold_text_font = TextFont::from_font(bold_font).with_font_size(12.0);

        let border_radius = BorderRadius::all(Val::Px(5.0));
        let background_color = BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7));
        let hud_node = Node {
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
        };
        let hud_row_node = Node {
            display: Display::Flex,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: Val::Px(20.0),
            ..default()
        };

        commands.spawn((
            hud_node,
            border_radius,
            background_color,
            children![
                (
                    hud_row_node.clone(),
                    children![
                        (Text::new("FPS"), bold_text_font.clone()),
                        (FpsTextNode, Text::new("-"), regular_text_font.clone()),
                    ],
                ),
                (
                    hud_row_node.clone(),
                    children![
                        (Text::new("Frame count"), bold_text_font.clone()),
                        (
                            FrameCountTextNode,
                            Text::new("-"),
                            regular_text_font.clone()
                        ),
                    ],
                ),
                (
                    hud_row_node.clone(),
                    children![
                        (Text::new("Barycenter"), bold_text_font.clone()),
                        (
                            BarycenterTextNode,
                            Text::new("-"),
                            regular_text_font.clone()
                        ),
                    ],
                ),
                (
                    hud_row_node.clone(),
                    children![
                        (Text::new("Body count"), bold_text_font.clone()),
                        (
                            Text::new(format!("{}", **body_count)),
                            regular_text_font.clone()
                        ),
                    ],
                ),
            ],
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
        if state.refresh_timer.finished() {
            if let Some(frame_count) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_COUNT) {
                if let Some(fps) = frame_count.smoothed() {
                    ***frame_count_text = format!("{fps}");
                }
            }
        }
    }

    fn update_fps_text(
        diagnostics: Res<DiagnosticsStore>,
        mut fps_text: Single<&mut Text, With<FpsTextNode>>,
        state: Res<DiagnosticsHudState>,
    ) {
        if state.refresh_timer.finished() {
            if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
                if let Some(fps) = fps.smoothed() {
                    ***fps_text = format!("{fps:.2}");
                }
            }
        }
    }

    fn update_barycenter_text(
        diagnostics: Res<DiagnosticsStore>,
        mut barycenter_text: Single<&mut Text, With<BarycenterTextNode>>,
        state: Res<DiagnosticsHudState>,
    ) {
        if state.refresh_timer.finished() {
            if let (Some(barycenter_x), Some(barycenter_y), Some(barycenter_z)) = (
                diagnostics.get(&SimulationDiagnosticsPlugin::BARYCENTER_X_PATH),
                diagnostics.get(&SimulationDiagnosticsPlugin::BARYCENTER_Y_PATH),
                diagnostics.get(&SimulationDiagnosticsPlugin::BARYCENTER_Z_PATH),
            ) {
                if let (Some(barycenter_x), Some(barycenter_y), Some(barycenter_z)) = (
                    barycenter_x.smoothed(),
                    barycenter_y.smoothed(),
                    barycenter_z.smoothed(),
                ) {
                    ***barycenter_text = format!(
                        "({:.2}, {:.2}, {:.2})",
                        barycenter_x, barycenter_y, barycenter_z,
                    );
                }
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
                Self::update_barycenter_text,
            ),
        );
    }
}
