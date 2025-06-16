use crate::diagnostics::SimulationDiagnosticsPlugin;
use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use bevy::asset::io::AssetSourceId;
use bevy::asset::AssetPath;
use bevy::diagnostic::DiagnosticsStore;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::ecs::relationship::RelatedSpawnerCommands;
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
    pub update_interval: Duration,
}

impl Default for DiagnosticsHudSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            update_interval: Duration::from_secs_f64(0.5),
        }
    }
}

#[derive(Resource)]
pub struct DiagnosticsHudState {
    pub update_timer: Timer,
}

impl Default for DiagnosticsHudState {
    fn default() -> Self {
        Self {
            update_timer: Timer::new(
                DiagnosticsHudSettings::default().update_interval,
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

    // fn init_update_timer(
    //     settings: Res<DiagnosticsHudSettings>,
    //     mut state: ResMut<DiagnosticsHudState>,
    // ) {
    //     state.update_timer = Timer::new(settings.update_interval, TimerMode::Repeating);
    // }

    fn spawn_diagnostics_hud(
        mut commands: Commands,
        settings: Res<DiagnosticsHudSettings>,
        asset_server: Res<AssetServer>,
    ) {
        commands
            .spawn((
                Name::new("Diagnostics HUD"),
                DiagnosticsHudUiNode,
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
                BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
                BorderRadius::all(Val::Px(5.0)),
            ))
            .with_children(|commands| {
                commands
                    .spawn((
                        DiagnosticsHudGroup,
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(1.0),
                            ..default()
                        },
                    ))
                    .with_children(|mut commands| {
                        hud_row(
                            &asset_server,
                            &mut commands,
                            FpsLabelText,
                            FpsValueText,
                            "FPS",
                            "-",
                        );
                    });

                commands
                    .spawn((
                        DiagnosticsHudGroup,
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(1.0),
                            ..default()
                        },
                    ))
                    .with_children(|mut commands| {
                        hud_row(
                            &asset_server,
                            &mut commands,
                            BarycenterLabelText,
                            BarycenterValueText,
                            "Barycenter",
                            "-",
                        );
                    });
            });
    }

    fn update_barycenter_value(
        diagnostics: Res<DiagnosticsStore>,
        mut barycenter_value_text: Single<&mut Text, With<BarycenterValueText>>,
        state: Res<DiagnosticsHudState>,
    ) {
        if state.update_timer.finished() {
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
        mut fps_hud_value: Single<&mut Text, With<FpsValueText>>,
        state: Res<DiagnosticsHudState>,
    ) {
        if state.update_timer.finished() {
            if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
                if let Some(fps) = fps.smoothed() {
                    ***fps_hud_value = format!("{fps:.2}");
                }
            }
        }
    }

    fn update_timer_ticks(mut state: ResMut<DiagnosticsHudState>, time: Res<Time>) {
        state.update_timer.tick(time.delta());
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
                Self::update_fps_value,
                Self::update_barycenter_value,
                Self::update_timer_ticks,
            ),
        );
    }
}

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct DiagnosticsHudUiNode;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct DiagnosticsHudGroup;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct DiagnosticsHudRow;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct FpsLabelText;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct FpsValueText;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct BarycenterLabelText;

#[derive(Component, Copy, Clone, Default, PartialEq, Debug)]
struct BarycenterValueText;

// TODO: continue CommandsExt refactor

// trait CommandsExt {
//     fn diagnostics_hud_group(&mut self) -> EntityCommands;
//     fn diagnostics_hud_row(&mut self) -> EntityCommands;
// }

// impl CommandsExt for RelatedSpawnerCommands<'_, ChildOf> {
//     fn diagnostics_hud_group(&mut self) -> EntityCommands {
//         self.spawn((
//             DiagnosticsHudGroup,
//             Node {
//                 display: Display::Flex,
//                 flex_direction: FlexDirection::Column,
//                 row_gap: Val::Px(1.0),
//                 ..default()
//             },
//         ))
//     }
//
//     fn diagnostics_hud_row(&mut self) -> EntityCommands {
//         self.spawn((
//             DiagnosticsHudRow,
//             Node {
//                 display: Display::Flex,
//                 justify_content: JustifyContent::SpaceBetween,
//                 column_gap: Val::Px(20.0),
//                 ..default()
//             },
//         ))
//     }
// }

fn hud_row<T: Component + Clone, U: Component + Clone>(
    asset_server: &AssetServer,
    commands: &mut RelatedSpawnerCommands<ChildOf>,
    label_marker: T,
    value_marker: U,
    label: &str,
    value: &str,
) {
    commands
        .spawn((
            DiagnosticsHudRow,
            Node {
                display: Display::Flex,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(20.0),
                ..default()
            },
        ))
        .with_children(|mut commands| {
            hud_label(&asset_server, &mut commands, label_marker, label);
            hud_value(&asset_server, &mut commands, value_marker, value);
        });
}

fn hud_label<T: Component + Clone>(
    asset_server: &AssetServer,
    commands: &mut RelatedSpawnerCommands<ChildOf>,
    marker: T,
    label: &str,
) {
    commands.spawn(Node::default()).with_children(|commands| {
        commands.spawn((marker, Text::new(label), hud_bold_text_font(&asset_server)));
    });
}

fn hud_value<T: Component + Clone>(
    asset_server: &AssetServer,
    commands: &mut RelatedSpawnerCommands<ChildOf>,
    marker: T,
    value: &str,
) {
    commands.spawn(Node::default()).with_children(|commands| {
        commands.spawn((
            marker,
            Text::new(value),
            hud_regular_text_font(&asset_server),
        ));
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
