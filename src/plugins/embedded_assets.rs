//! UI plugin module.
//!
//! This module provides the UI plugin that handles font asset registration
//! and other UI-related initialization that needs to happen before other
//! plugins with UI code run.

use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use bevy::prelude::*;

static REGULAR_OTF_BYTES: &[u8] = include_bytes!("../../assets/fonts/BerkeleyMono-Regular.otf");
static BOLD_OTF_BYTES: &[u8] = include_bytes!("../../assets/fonts/BerkeleyMono-Bold.otf");

pub struct EmbeddedAssetsPlugin;

impl EmbeddedAssetsPlugin {
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
}

impl Plugin for EmbeddedAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, Self::insert_font_assets);
    }
}
