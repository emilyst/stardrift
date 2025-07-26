//! Embedded assets plugin - Self-contained plugin pattern
//!
//! This plugin follows the self-contained pattern where all asset embedding logic
//! is contained within the plugin module. It handles font asset registration and
//! other asset embedding for web deployment. This pattern is ideal for this
//! plugin since asset embedding is an independent concern that doesn't interact
//! with other systems.

use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use bevy::prelude::*;

static SAIRA_SEMI_CONDENSED_LIGHT: &[u8] =
    include_bytes!("../../assets/fonts/SairaSemiCondensed-Light.ttf");
static SAIRA_SEMI_CONDENSED_BOLD: &[u8] =
    include_bytes!("../../assets/fonts/SairaSemiCondensed-Bold.ttf");

pub struct EmbeddedAssetsPlugin;

impl EmbeddedAssetsPlugin {
    fn insert_font_assets(world: &mut World) {
        let embedded_asset_registry = world.resource_mut::<EmbeddedAssetRegistry>();

        embedded_asset_registry.insert_asset(
            "fonts/SairaSemiCondensed-Light".into(),
            "fonts/SairaSemiCondensed-Light".as_ref(),
            SAIRA_SEMI_CONDENSED_LIGHT,
        );

        embedded_asset_registry.insert_asset(
            "fonts/SairaSemiCondensed-Bold".into(),
            "fonts/SairaSemiCondensed-Bold".as_ref(),
            SAIRA_SEMI_CONDENSED_BOLD,
        );
    }
}

impl Plugin for EmbeddedAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, Self::insert_font_assets);
    }
}
