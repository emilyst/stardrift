//! Attribution plugin - Displays program name and version
//!
//! This plugin shows the attribution text in the lower right corner of the screen.
//! The attribution remains visible during screenshots to ensure proper credit.
//! Clicking the attribution opens the project repository.

use crate::prelude::*;
use bevy::asset::{AssetPath, io::AssetSourceId};
use bevy::window::{CursorIcon, SystemCursorIcon};

#[derive(Component)]
pub struct AttributionText;

pub struct AttributionPlugin;

impl Plugin for AttributionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_attribution);
        app.add_systems(Update, handle_attribution_interaction);
    }
}

fn setup_attribution(mut commands: Commands, asset_server: Res<AssetServer>) {
    let embedded_asset_source = &AssetSourceId::from("embedded");

    let font_asset_path = AssetPath::parse("fonts/Saira-Light").with_source(embedded_asset_source);
    let font = asset_server.load(font_asset_path);
    let attribution_text_font = TextFont {
        font,
        font_size: 10.0,
        ..default()
    };

    // Attribution with version in bottom right corner (visible in screenshots)
    // Now interactive - clicking opens the repository
    commands.spawn((
        Button,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            right: Val::Px(5.0),
            padding: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::NONE),
        Text::new(format!(
            "Stardrift v{} ({})",
            env!("CARGO_PKG_VERSION"),
            env!("BUILD_DATE")
        )),
        attribution_text_font,
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
        AttributionText,
        Interaction::default(),
    ));
}

fn handle_attribution_interaction(
    mut commands: Commands,
    window: Single<Entity, With<Window>>,
    mut interaction_query: Query<
        (&Interaction, &mut TextColor),
        (Changed<Interaction>, With<AttributionText>),
    >,
) {
    for (interaction, mut text_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                commands
                    .entity(*window)
                    .insert(CursorIcon::System(SystemCursorIcon::Pointer));

                // Open the repository URL from package metadata
                if let Some(repo_url) = option_env!("CARGO_PKG_REPOSITORY") {
                    if let Err(e) = webbrowser::open(repo_url) {
                        warn!("Failed to open repository URL: {}", e);
                    }
                } else {
                    warn!("Repository URL not found in package metadata");
                }

                text_color.0 = Color::srgba(1.0, 1.0, 1.0, 0.5);
            }
            Interaction::Hovered => {
                commands
                    .entity(*window)
                    .insert(CursorIcon::System(SystemCursorIcon::Pointer));

                text_color.0 = Color::srgba(1.0, 1.0, 1.0, 0.5);
            }
            Interaction::None => {
                commands
                    .entity(*window)
                    .insert(CursorIcon::System(SystemCursorIcon::Default));

                text_color.0 = Color::srgba(1.0, 1.0, 1.0, 0.3);
            }
        }
    }
}
