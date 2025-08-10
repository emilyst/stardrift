//! Test utilities for plugin testing

use bevy::input::ButtonState;
use bevy::input::keyboard::Key;
use bevy::prelude::*;

use crate::prelude::*;

/// Creates a minimal test app with core Bevy plugins needed for testing
pub fn create_test_app() -> App {
    let mut app = App::new();

    // Add minimal plugins needed for testing
    app.add_plugins((
        MinimalPlugins.set(TaskPoolPlugin::default()),
        AssetPlugin::default(),
        bevy::input::InputPlugin,
        bevy::state::app::StatesPlugin,
        TransformPlugin,
        bevy::diagnostic::DiagnosticsPlugin,
    ));

    // Add physics time resource
    app.insert_resource(crate::physics::resources::PhysicsTime::default());

    // Initialize assets needed by various plugins
    app.init_asset::<Font>();
    app.init_asset::<Mesh>();
    app.init_asset::<Shader>();
    app.init_asset::<StandardMaterial>();

    // Add gizmo-related resources
    app.init_resource::<GizmoConfigStore>();

    // Add visualization resources
    app.init_resource::<BarycenterGizmoVisibility>();
    app.init_resource::<OctreeVisualizationSettings>();
    app.init_resource::<TrailsVisualizationSettings>();
    app.init_resource::<crate::plugins::diagnostics_hud::DiagnosticsHudSettings>();

    // Add events used by plugins
    app.add_event::<SimulationCommand>();
    app.add_event::<UpdateButtonTextEvent<crate::plugins::controls::PauseButton>>();
    app.add_event::<UpdateButtonTextEvent<crate::plugins::controls::OctreeToggleButton>>();
    app.add_event::<UpdateButtonTextEvent<crate::plugins::controls::BarycenterGizmoToggleButton>>();
    app.add_event::<UpdateButtonTextEvent<crate::plugins::controls::TrailsToggleButton>>();
    app.add_event::<UpdateButtonTextEvent<crate::plugins::controls::DiagnosticsHudToggleButton>>();

    // Add states
    app.init_state::<AppState>();

    app
}

/// Helper to simulate a keyboard input event
pub fn send_keyboard_input(app: &mut App, key: Key, state: ButtonState) {
    use bevy::input::keyboard::KeyboardInput;

    // For tests, we can use a placeholder entity since we don't have actual windows
    let window_entity = Entity::PLACEHOLDER;

    let event = KeyboardInput {
        key_code: KeyCode::KeyA, // Placeholder, not used in new system
        logical_key: key,
        state,
        text: None,
        repeat: false,
        window: window_entity,
    };

    app.world_mut().send_event(event);
}

/// Helper to simulate a character key press
pub fn press_character_key(app: &mut App, character: &str) {
    send_keyboard_input(app, Key::Character(character.into()), ButtonState::Pressed);
}

/// Helper to simulate a special key press
pub fn press_special_key(app: &mut App, key: Key) {
    send_keyboard_input(app, key, ButtonState::Pressed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_app() {
        let app = create_test_app();
        assert!(app.world().contains_resource::<Time>());
        assert!(app.world().contains_resource::<ButtonInput<KeyCode>>());
    }

    #[test]
    fn test_keyboard_input_helpers() {
        let mut app = create_test_app();

        // Add a system to consume events (verifies they're being sent properly)
        app.add_systems(
            Update,
            |mut events: EventReader<bevy::input::keyboard::KeyboardInput>| {
                for _ in events.read() {
                    // Just consume the events to verify they're being sent
                }
            },
        );

        // Test character key press
        press_character_key(&mut app, "a");
        app.update();

        // Test special key press
        press_special_key(&mut app, Key::Space);
        app.update();

        // The test passes if no panic occurs during event sending
    }
}
