//! Builder pattern utilities for controls UI
//!
//! This module provides a CommandsExt trait and associated builder types
//! to simplify the creation and management of control buttons in the UI.

use crate::plugins::controls::constants::*;
use crate::prelude::*;
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;

pub trait ControlsCommandsExt {
    fn spawn_control_button<T: ButtonWithLabel>(&mut self, font: Handle<Font>) -> Entity;
}

impl ControlsCommandsExt for ChildSpawnerCommands<'_> {
    fn spawn_control_button<T: ButtonWithLabel>(&mut self, font: Handle<Font>) -> Entity {
        self.spawn((
            Button,
            Node {
                width: Val::Px(BUTTON_WIDTH_PX),
                height: Val::Auto,
                padding: UiRect::all(Val::Px(BUTTON_PADDING_PX)),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(1.0),
                ..default()
            },
            BorderRadius::all(Val::Px(BUTTON_BORDER_RADIUS_PX)),
            BackgroundColor(BUTTON_COLOR_NORMAL),
            T::marker(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(T::label()),
                TextColor(Color::WHITE),
                TextFont {
                    font,
                    font_size: BUTTON_FONT_SIZE_PX,
                    ..default()
                },
            ));
        })
        .id()
    }
}

pub trait ButtonWithLabel: Component + 'static {
    /// The command this button triggers
    fn command() -> SimulationCommand;

    /// The marker component instance
    fn marker() -> Self;

    /// The base text for the button (without shortcut)
    fn base_text() -> &'static str;

    /// The keyboard shortcut for this button
    fn shortcut() -> &'static str;

    /// The base text with shortcut appended
    fn label() -> String {
        format!("{} ({})", Self::base_text(), Self::shortcut())
    }
}
