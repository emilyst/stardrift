//! Embedded assets plugin - Self-contained plugin pattern
//!
//! This plugin follows the self-contained pattern where all asset embedding logic
//! is contained within the plugin module. It handles font asset registration and
//! other asset embedding for web deployment. This pattern is ideal for this
//! plugin since asset embedding is an independent concern that doesn't interact
//! with other systems.

use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use bevy::prelude::*;

static SAIRA_BLACK: &[u8] = include_bytes!("../../assets/fonts/Saira-Black.ttf");
static SAIRA_BLACK_ITALIC: &[u8] = include_bytes!("../../assets/fonts/Saira-BlackItalic.ttf");
static SAIRA_BOLD: &[u8] = include_bytes!("../../assets/fonts/Saira-Bold.ttf");
static SAIRA_BOLD_ITALIC: &[u8] = include_bytes!("../../assets/fonts/Saira-BoldItalic.ttf");
static SAIRA_EXTRA_BOLD: &[u8] = include_bytes!("../../assets/fonts/Saira-ExtraBold.ttf");
static SAIRA_EXTRA_BOLD_ITALIC: &[u8] =
    include_bytes!("../../assets/fonts/Saira-ExtraBoldItalic.ttf");
static SAIRA_EXTRA_LIGHT: &[u8] = include_bytes!("../../assets/fonts/Saira-ExtraLight.ttf");
static SAIRA_EXTRA_LIGHT_ITALIC: &[u8] =
    include_bytes!("../../assets/fonts/Saira-ExtraLightItalic.ttf");
static SAIRA_ITALIC: &[u8] = include_bytes!("../../assets/fonts/Saira-Italic.ttf");
static SAIRA_LIGHT: &[u8] = include_bytes!("../../assets/fonts/Saira-Light.ttf");
static SAIRA_LIGHT_ITALIC: &[u8] = include_bytes!("../../assets/fonts/Saira-LightItalic.ttf");
static SAIRA_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/Saira-Medium.ttf");
static SAIRA_MEDIUM_ITALIC: &[u8] = include_bytes!("../../assets/fonts/Saira-MediumItalic.ttf");
static SAIRA_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Saira-Regular.ttf");
static SAIRA_SEMI_BOLD: &[u8] = include_bytes!("../../assets/fonts/Saira-SemiBold.ttf");
static SAIRA_SEMI_BOLD_ITALIC: &[u8] =
    include_bytes!("../../assets/fonts/Saira-SemiBoldItalic.ttf");
static SAIRA_THIN: &[u8] = include_bytes!("../../assets/fonts/Saira-Thin.ttf");
static SAIRA_THIN_ITALIC: &[u8] = include_bytes!("../../assets/fonts/Saira-ThinItalic.ttf");

pub struct EmbeddedAssetsPlugin;

impl EmbeddedAssetsPlugin {
    fn insert_font_assets(world: &mut World) {
        let embedded_asset_registry = world.resource_mut::<EmbeddedAssetRegistry>();

        embedded_asset_registry.insert_asset(
            "fonts/Saira-Black".into(),
            "fonts/Saira-Black".as_ref(),
            SAIRA_BLACK,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-BlackItalic".into(),
            "fonts/Saira-BlackItalic".as_ref(),
            SAIRA_BLACK_ITALIC,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-Bold".into(),
            "fonts/Saira-Bold".as_ref(),
            SAIRA_BOLD,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-BoldItalic".into(),
            "fonts/Saira-BoldItalic".as_ref(),
            SAIRA_BOLD_ITALIC,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-ExtraBold".into(),
            "fonts/Saira-ExtraBold".as_ref(),
            SAIRA_EXTRA_BOLD,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-ExtraBoldItalic".into(),
            "fonts/Saira-ExtraBoldItalic".as_ref(),
            SAIRA_EXTRA_BOLD_ITALIC,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-ExtraLight".into(),
            "fonts/Saira-ExtraLight".as_ref(),
            SAIRA_EXTRA_LIGHT,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-ExtraLightItalic".into(),
            "fonts/Saira-ExtraLightItalic".as_ref(),
            SAIRA_EXTRA_LIGHT_ITALIC,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-Italic".into(),
            "fonts/Saira-Italic".as_ref(),
            SAIRA_ITALIC,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-Light".into(),
            "fonts/Saira-Light".as_ref(),
            SAIRA_LIGHT,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-LightItalic".into(),
            "fonts/Saira-LightItalic".as_ref(),
            SAIRA_LIGHT_ITALIC,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-Medium".into(),
            "fonts/Saira-Medium".as_ref(),
            SAIRA_MEDIUM,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-MediumItalic".into(),
            "fonts/Saira-MediumItalic".as_ref(),
            SAIRA_MEDIUM_ITALIC,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-Regular".into(),
            "fonts/Saira-Regular".as_ref(),
            SAIRA_REGULAR,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-SemiBold".into(),
            "fonts/Saira-SemiBold".as_ref(),
            SAIRA_SEMI_BOLD,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-SemiBoldItalic".into(),
            "fonts/Saira-SemiBoldItalic".as_ref(),
            SAIRA_SEMI_BOLD_ITALIC,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-Thin".into(),
            "fonts/Saira-Thin".as_ref(),
            SAIRA_THIN,
        );

        embedded_asset_registry.insert_asset(
            "fonts/Saira-ThinItalic".into(),
            "fonts/Saira-ThinItalic".as_ref(),
            SAIRA_THIN_ITALIC,
        );
    }
}

impl Plugin for EmbeddedAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, Self::insert_font_assets);
    }
}
