//! Shared styling constants for controls UI
//!
//! This module defines the visual constants used across all control buttons
//! to ensure consistent appearance and behavior.

use bevy::prelude::Color;

pub const BUTTON_BORDER_RADIUS_PX: f32 = 4.0;
pub const BUTTON_FONT_SIZE_PX: f32 = 12.0;
pub const BUTTON_GAP_PX: f32 = 4.0;
pub const BUTTON_MARGIN_PX: f32 = 4.0;
pub const BUTTON_PADDING_PX: f32 = 4.0;
pub const BUTTON_WIDTH_PX: f32 = 160.0;

pub const BUTTON_COLOR_NORMAL: Color = Color::srgba(1.0, 1.0, 1.0, 0.01);
pub const BUTTON_COLOR_HOVERED: Color = Color::srgba(1.0, 1.0, 1.0, 0.1);
pub const BUTTON_COLOR_PRESSED: Color = Color::srgba(1.0, 1.0, 1.0, 0.2);
