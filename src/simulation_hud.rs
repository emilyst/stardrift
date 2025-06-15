use crate::CurrentBarycenter;

use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use bevy::asset::io::AssetSourceId;
use bevy::asset::AssetPath;
use bevy::diagnostic::DiagnosticsStore;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use std::path::Path;

static BERKELEY_MONO_REGULAR_OTF: &[u8] =
    include_bytes!("../assets/fonts/BerkeleyMono-Regular.otf");
static BERKELEY_MONO_OBLIQUE_OTF: &[u8] =
    include_bytes!("../assets/fonts/BerkeleyMono-Oblique.otf");
static BERKELEY_MONO_BOLD_OTF: &[u8] = include_bytes!("../assets/fonts/BerkeleyMono-Bold.otf");
static BERKELEY_MONO_BOLD_OBLIQUE_OTF: &[u8] =
    include_bytes!("../assets/fonts/BerkeleyMono-Bold-Oblique.otf");

#[derive(Resource, Reflect, Deref, DerefMut, Debug)]
#[reflect(Resource, Debug)]
struct SimulationHudSettings {
    pub enabled: bool,
}

impl Default for SimulationHudSettings {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct FpsHudLabel;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct FpsHudValue;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct BarycenterHudLabel;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct BarycenterHudValue;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct SimulationHud;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct SimulationHudGroup;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct SimulationHudRow;

pub(crate) struct SimulationHudPlugin;

impl Plugin for SimulationHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationHudSettings>();
        app.add_systems(Startup, (insert_embedded_font_assets, spawn_hud).chain());
        app.add_systems(
            Update,
            (refresh_fps_hud_value, refresh_barycenter_hud_value).chain(),
        );
    }
}

fn insert_embedded_font_assets(world: &mut World) {
    let embedded = world.resource_mut::<EmbeddedAssetRegistry>();

    embedded.insert_asset(
        Path::new("fonts/BerkeleyMono-Regular").into(),
        Path::new("fonts/BerkeleyMono-Regular"),
        BERKELEY_MONO_REGULAR_OTF,
    );

    embedded.insert_asset(
        Path::new("fonts/BerkeleyMono-Bold").into(),
        Path::new("fonts/BerkeleyMono-Bold"),
        BERKELEY_MONO_BOLD_OTF,
    );

    embedded.insert_asset(
        Path::new("fonts/BerkeleyMono-Oblique").into(),
        Path::new("fonts/BerkeleyMono-Oblique"),
        BERKELEY_MONO_OBLIQUE_OTF,
    );

    embedded.insert_asset(
        Path::new("fonts/BerkeleyMono-Bold-Oblique").into(),
        Path::new("fonts/BerkeleyMono-Bold-Oblique"),
        BERKELEY_MONO_BOLD_OBLIQUE_OTF,
    );
}

// liberally adapted from avian3d physics diagnostics
fn spawn_hud(
    mut commands: Commands,
    settings: Res<SimulationHudSettings>,
    asset_server: Res<AssetServer>,
) {
    let hud_text_font = hud_text_font(&asset_server);
    let hud_bold_text_font = hud_bold_text_font(&asset_server);

    commands
        .spawn((
            Name::new("Simulation HUD"),
            SimulationHud,
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
            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.75)),
            BorderRadius::all(Val::Px(5.0)),
        ))
        .with_children(|commands| {
            commands
                .spawn((
                    SimulationHudGroup,
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
                            SimulationHudRow,
                            Node {
                                display: Display::Flex,
                                justify_content: JustifyContent::SpaceBetween,
                                column_gap: Val::Px(10.0),
                                ..default()
                            },
                        ))
                        .with_children(|commands| {
                            commands.spawn((
                                FpsHudLabel,
                                Text::new("FPS"),
                                hud_bold_text_font.clone(),
                            ));

                            commands.spawn(Node::default()).with_children(|commands| {
                                commands.spawn((
                                    FpsHudValue,
                                    Text::new("-"),
                                    hud_text_font.clone(),
                                ));
                            });
                        });
                });

            commands
                .spawn((
                    SimulationHudGroup,
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
                            SimulationHudRow,
                            Node {
                                display: Display::Flex,
                                justify_content: JustifyContent::SpaceBetween,
                                column_gap: Val::Px(20.0),
                                ..default()
                            },
                        ))
                        .with_children(|commands| {
                            commands.spawn((
                                BarycenterHudLabel,
                                Text::new("Barycenter"),
                                hud_bold_text_font.clone(),
                            ));

                            commands.spawn(Node::default()).with_children(|commands| {
                                commands.spawn((
                                    BarycenterHudValue,
                                    Text::new("-"),
                                    hud_text_font.clone(),
                                ));
                            });
                        });
                });
        });
}

fn hud_text_font(asset_server: &AssetServer) -> TextFont {
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

fn refresh_fps_hud_value(
    diagnostics: Res<DiagnosticsStore>,
    mut fps_hud_value: Single<&mut Text, With<FpsHudValue>>,
) {
    if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(fps) = fps.smoothed() {
            ***fps_hud_value = format!("{fps:.2}");
        }
    }
}

fn refresh_barycenter_hud_value(
    current_barycenter: Res<CurrentBarycenter>,
    mut barycenter_hud_value: Single<&mut Text, With<BarycenterHudValue>>,
) {
    ***barycenter_hud_value = format!("{:.2}", ***current_barycenter);
}
