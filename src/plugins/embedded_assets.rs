//! Embedded assets plugin module.
//!
//! This module provides the embedded assets plugin that handles font asset registration
//! and other asset embedding for web deployment. All embedded assets are available
//! through their asset paths.

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
