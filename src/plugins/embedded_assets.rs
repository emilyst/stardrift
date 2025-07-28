//! Embedded assets plugin - Self-contained plugin pattern
//!
//! This plugin follows the self-contained pattern where all asset embedding logic
//! is contained within the plugin module. It handles font asset registration and
//! other asset embedding for web deployment. This pattern is ideal for this
//! plugin since asset embedding is an independent concern that doesn't interact
//! with other systems.

use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use bevy::prelude::*;

static SAIRA_LIGHT: &[u8] = include_bytes!("../../assets/fonts/Saira-Light.ttf");
static SAIRA_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Saira-Regular.ttf");
static SAIRA_BOLD: &[u8] = include_bytes!("../../assets/fonts/Saira-Bold.ttf");

pub struct EmbeddedAssetsPlugin;

impl EmbeddedAssetsPlugin {
    fn insert_font_assets(world: &mut World) {
        let embedded_asset_registry = world.resource_mut::<EmbeddedAssetRegistry>();

        embedded_asset_registry.insert_asset(
            "fonts/Saira-Light".into(),
            "fonts/Saira-Light".as_ref(),
            SAIRA_LIGHT,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-Regular".into(),
            "fonts/Saira-Regular".as_ref(),
            SAIRA_REGULAR,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-Bold".into(),
            "fonts/Saira-Bold".as_ref(),
            SAIRA_BOLD,
        );
    }
}

impl Plugin for EmbeddedAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, Self::insert_font_assets);
    }
}
