use crate::diagnostics::SimulationDiagnosticsPlugin;
use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use bevy::asset::io::AssetSourceId;
use bevy::asset::AssetPath;
use bevy::diagnostic::DiagnosticsStore;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use std::path::Path;
use std::time::Duration;

static REGULAR_OTF_BYTES: &[u8] = include_bytes!("../assets/fonts/BerkeleyMono-Regular.otf");
static OBLIQUE_OTF_BYTES: &[u8] = include_bytes!("../assets/fonts/BerkeleyMono-Oblique.otf");
static BOLD_OTF_BYTES: &[u8] = include_bytes!("../assets/fonts/BerkeleyMono-Bold.otf");
static BOLD_OBLIQUE_OTF_BYTES: &[u8] =
    include_bytes!("../assets/fonts/BerkeleyMono-Bold-Oblique.otf");

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
            refresh_interval: Duration::from_secs_f64(0.5),
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
        let embedded = world.resource_mut::<EmbeddedAssetRegistry>();

        embedded.insert_asset(
            Path::new("fonts/BerkeleyMono-Regular").into(),
            Path::new("fonts/BerkeleyMono-Regular"),
            REGULAR_OTF_BYTES,
        );

        embedded.insert_asset(
            Path::new("fonts/BerkeleyMono-Bold").into(),
            Path::new("fonts/BerkeleyMono-Bold"),
            BOLD_OTF_BYTES,
        );

        embedded.insert_asset(
            Path::new("fonts/BerkeleyMono-Oblique").into(),
            Path::new("fonts/BerkeleyMono-Oblique"),
            OBLIQUE_OTF_BYTES,
        );

        embedded.insert_asset(
            Path::new("fonts/BerkeleyMono-Bold-Oblique").into(),
            Path::new("fonts/BerkeleyMono-Bold-Oblique"),
            BOLD_OBLIQUE_OTF_BYTES,
        );
    }

    fn spawn_diagnostics_hud(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        settings: Res<DiagnosticsHudSettings>,
    ) {
        commands
            .spawn((
                Name::new("Diagnostics HUD"),
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
                DiagnosticsHudNode,
                BorderRadius::all(Val::Px(5.0)),
                BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
            ))
            .with_children(|commands| {
                commands
                    .spawn((
                        DiagnosticsHudGroupNode,
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(1.0),
                            ..default()
                        },
                    ))
                    .with_children(|commands| {
                        commands
                            .spawn((
                                DiagnosticsHudRowNode,
                                Node {
                                    display: Display::Flex,
                                    justify_content: JustifyContent::SpaceBetween,
                                    column_gap: Val::Px(20.0),
                                    ..default()
                                },
                            ))
                            .with_children(|commands| {
                                commands.spawn(Node::default()).with_children(|commands| {
                                    commands.spawn((
                                        Text::new("FPS"),
                                        Self::hud_bold_text_font(&asset_server),
                                    ));
                                });
                                commands.spawn(Node::default()).with_children(|commands| {
                                    commands.spawn((
                                        FpsValueTextNode,
                                        Text::new("-"),
                                        Self::hud_regular_text_font(&asset_server),
                                    ));
                                });
                            });
                    });

                commands
                    .spawn((
                        DiagnosticsHudGroupNode,
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(1.0),
                            ..default()
                        },
                    ))
                    .with_children(|commands| {
                        commands
                            .spawn((
                                DiagnosticsHudRowNode,
                                Node {
                                    display: Display::Flex,
                                    justify_content: JustifyContent::SpaceBetween,
                                    column_gap: Val::Px(20.0),
                                    ..default()
                                },
                            ))
                            .with_children(|commands| {
                                commands.spawn(Node::default()).with_children(|commands| {
                                    commands.spawn((
                                        Text::new("Barycenter"),
                                        Self::hud_bold_text_font(&asset_server),
                                    ));
                                });
                                commands.spawn(Node::default()).with_children(|commands| {
                                    commands.spawn((
                                        BarycenterValueTextNode,
                                        Text::new("-"),
                                        Self::hud_regular_text_font(&asset_server),
                                    ));
                                });
                            });
                    });
            });
    }

    fn hud_regular_text_font(asset_server: &AssetServer) -> TextFont {
        let path = Path::new("fonts/BerkeleyMono-Regular");
        let source = AssetSourceId::from("embedded");
        let asset_path = AssetPath::from_path(&path).with_source(source);

        TextFont {
            font: asset_server.load(asset_path),
            font_size: 12.0,
            ..default()
        }
    }

    fn hud_bold_text_font(asset_server: &AssetServer) -> TextFont {
        let path = Path::new("fonts/BerkeleyMono-Bold");
        let source = AssetSourceId::from("embedded");
        let asset_path = AssetPath::from_path(&path).with_source(source);

        TextFont {
            font: asset_server.load(asset_path),
            font_size: 12.0,
            ..default()
        }
    }

    fn update_barycenter_value(
        diagnostics: Res<DiagnosticsStore>,
        mut barycenter_value_text: Single<&mut Text, With<BarycenterValueTextNode>>,
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
                    ***barycenter_value_text = format!(
                        "(X: {:.2}, Y: {:.2}, Z: {:.2})",
                        barycenter_x, barycenter_y, barycenter_z,
                    );
                }
            }
        }
    }

    fn update_fps_value(
        diagnostics: Res<DiagnosticsStore>,
        mut fps_hud_value: Single<&mut Text, With<FpsValueTextNode>>,
        state: Res<DiagnosticsHudState>,
    ) {
        if state.refresh_timer.finished() {
            if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
                if let Some(fps) = fps.smoothed() {
                    ***fps_hud_value = format!("{fps:.2}");
                }
            }
        }
    }

    fn advance_refresh_timer(mut state: ResMut<DiagnosticsHudState>, time: Res<Time>) {
        state.refresh_timer.tick(time.delta());
    }
}

impl Plugin for DiagnosticsHudPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DiagnosticsHudSettings::default());
        app.insert_resource(DiagnosticsHudState::default());
        // app.insert_resource(BarycenterNameValuePair::default());
        // app.insert_resource(FpsNameValuePair::default());

        app.add_systems(
            Startup,
            (Self::insert_font_assets, Self::spawn_diagnostics_hud).chain(),
        );
        app.add_systems(
            Update,
            (
                Self::update_fps_value,
                Self::update_barycenter_value,
                Self::advance_refresh_timer,
            ),
        );
    }
}

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct DiagnosticsHudNode;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct DiagnosticsHudGroupNode;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct DiagnosticsHudRowNode;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct FpsValueTextNode;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct BarycenterValueTextNode;

#[derive(Resource, Debug)]
struct FpsNameValuePair {
    name: String,
    value: String,
}

impl Default for FpsNameValuePair {
    fn default() -> Self {
        Self {
            name: String::from("FPS"),
            value: String::from("-"),
        }
    }
}

#[derive(Resource, Debug)]
struct BarycenterNameValuePair {
    name: String,
    value: String,
}

impl Default for BarycenterNameValuePair {
    fn default() -> Self {
        Self {
            name: String::from("Barycenter"),
            value: String::from("-"),
        }
    }
}
