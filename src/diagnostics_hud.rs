use crate::diagnostics::SimulationDiagnosticsPlugin;
use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use bevy::asset::io::AssetSourceId;
use bevy::asset::AssetPath;
use bevy::diagnostic::DiagnosticsStore;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use core::time::Duration;

static REGULAR_OTF_BYTES: &[u8] = include_bytes!("../assets/fonts/BerkeleyMono-Regular.otf");
static BOLD_OTF_BYTES: &[u8] = include_bytes!("../assets/fonts/BerkeleyMono-Bold.otf");

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct FrameCountTextNode;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct FpsTextNode;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct BarycenterTextNode;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct CameraTextNode;

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
            refresh_interval: Duration::from_secs_f64(1_f64 / 6_f64),
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
    fn insert_font_assets(world: &mut World) {
        let embedded_asset_registry = world.resource_mut::<EmbeddedAssetRegistry>();

        embedded_asset_registry.insert_asset(
            "fonts/BerkeleyMono-Regular".into(),
            "fonts/BerkeleyMono-Regular".as_ref(),
            REGULAR_OTF_BYTES,
        );

        embedded_asset_registry.insert_asset(
            "fonts/BerkeleyMono-Bold".into(),
            "fonts/BerkeleyMono-Bold".as_ref(),
            BOLD_OTF_BYTES,
        );
    }

    fn spawn_diagnostics_hud(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        settings: Res<DiagnosticsHudSettings>,
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

        commands.spawn((
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
            BorderRadius::all(Val::Px(5.0)),
            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
            children![
                (
                    Node {
                        display: Display::Flex,
                        justify_content: JustifyContent::SpaceBetween,
                        column_gap: Val::Px(20.0),
                        ..default()
                    },
                    children![
                        (Text::new("FPS"), bold_text_font.clone()),
                        (FpsTextNode, Text::new("-"), regular_text_font.clone())
                    ],
                ),
                (
                    Node {
                        display: Display::Flex,
                        justify_content: JustifyContent::SpaceBetween,
                        column_gap: Val::Px(20.0),
                        ..default()
                    },
                    children![
                        (Text::new("Frame count"), bold_text_font.clone()),
                        (
                            FrameCountTextNode,
                            Text::new("-"),
                            regular_text_font.clone()
                        )
                    ],
                ),
                (
                    Node {
                        display: Display::Flex,
                        justify_content: JustifyContent::SpaceBetween,
                        column_gap: Val::Px(20.0),
                        ..default()
                    },
                    children![
                        (Text::new("Barycenter"), bold_text_font.clone()),
                        (
                            BarycenterTextNode,
                            Text::new("-"),
                            regular_text_font.clone()
                        )
                    ],
                ),
                (
                    Node {
                        display: Display::Flex,
                        justify_content: JustifyContent::SpaceBetween,
                        column_gap: Val::Px(20.0),
                        ..default()
                    },
                    children![
                        (Text::new("Camera"), bold_text_font),
                        (CameraTextNode, Text::new("-"), regular_text_font)
                    ],
                )
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
                        "(X: {:.2}, Y: {:.2}, Z: {:.2})",
                        barycenter_x, barycenter_y, barycenter_z,
                    );
                }
            }
        }
    }

    fn update_camera_text(
        diagnostics: Res<DiagnosticsStore>,
        mut camera_text: Single<&mut Text, With<CameraTextNode>>,
        state: Res<DiagnosticsHudState>,
    ) {
        if state.refresh_timer.finished() {
            if let (Some(camera_x), Some(camera_y), Some(camera_z)) = (
                diagnostics.get(&SimulationDiagnosticsPlugin::CAMERA_X_PATH),
                diagnostics.get(&SimulationDiagnosticsPlugin::CAMERA_Y_PATH),
                diagnostics.get(&SimulationDiagnosticsPlugin::CAMERA_Z_PATH),
            ) {
                if let (Some(camera_x), Some(camera_y), Some(camera_z)) = (
                    camera_x.smoothed(),
                    camera_y.smoothed(),
                    camera_z.smoothed(),
                ) {
                    ***camera_text = format!(
                        "(X: {:.2}, Y: {:.2}, Z: {:.2})",
                        camera_x, camera_y, camera_z,
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

        app.add_systems(
            Startup,
            (Self::insert_font_assets, Self::spawn_diagnostics_hud).chain(),
        );
        app.add_systems(
            Update,
            (
                Self::advance_refresh_timer,
                Self::update_frame_count_text,
                Self::update_fps_text,
                Self::update_barycenter_text,
                Self::update_camera_text,
            ),
        );
    }
}
