//! Attribution plugin - Displays program name and version
//!
//! This plugin shows the attribution text in the lower right corner of the screen.
//! The attribution remains visible during screenshots to ensure proper credit.

use crate::prelude::*;
use bevy::asset::{AssetPath, io::AssetSourceId};

#[derive(Component)]
pub struct AttributionText;

pub struct AttributionPlugin;

impl Plugin for AttributionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_attribution);
    }
}

fn setup_attribution(mut commands: Commands, asset_server: Res<AssetServer>) {
    let embedded_asset_source = &AssetSourceId::from("embedded");

    let regular_font_asset_path =
        AssetPath::parse("fonts/Saira-Regular").with_source(embedded_asset_source);
    let regular_font = asset_server.load(regular_font_asset_path);
    let attribution_text_font = TextFont::from_font(regular_font).with_font_size(10.0);

    // Attribution with version in bottom right corner (visible in screenshots)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            right: Val::Px(5.0),
            ..default()
        },
        Text::new(format!(
            "Stardrift v{} ({})",
            env!("CARGO_PKG_VERSION"),
            env!("BUILD_DATE")
        )),
        attribution_text_font,
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
        AttributionText,
    ));
}
