//! Test utilities for plugin testing

use bevy::prelude::*;

use crate::prelude::*;

/// Creates a minimal test app with core Bevy plugins needed for testing
pub fn create_test_app() -> App {
    let mut app = App::new();

    // Add minimal plugins needed for testing
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        bevy::input::InputPlugin,
        bevy::state::app::StatesPlugin,
        bevy::transform::TransformPlugin,
        bevy::diagnostic::DiagnosticsPlugin,
    ));

    // Add Avian physics time resource
    app.insert_resource(Time::<avian3d::prelude::Physics>::default());

    // Initialize assets needed by various plugins
    app.init_asset::<bevy::text::Font>();
    app.init_asset::<bevy::render::mesh::Mesh>();
    app.init_asset::<bevy::render::prelude::Shader>();
    app.init_asset::<bevy::pbr::StandardMaterial>();

    // Add gizmo-related resources
    app.init_resource::<bevy::gizmos::config::GizmoConfigStore>();

    // Add visualization resources
    app.init_resource::<crate::plugins::visualization::BarycenterGizmoVisibility>();
    app.init_resource::<crate::plugins::visualization::OctreeVisualizationSettings>();

    // Add events used by plugins
    app.add_event::<SimulationCommand>();
    app.add_event::<UpdateButtonTextEvent<crate::plugins::controls::PauseButton>>();
    app.add_event::<UpdateButtonTextEvent<crate::plugins::controls::OctreeToggleButton>>();
    app.add_event::<UpdateButtonTextEvent<crate::plugins::controls::BarycenterGizmoToggleButton>>();

    // Add states
    app.init_state::<AppState>();

    app
}

/// Helper to simulate a key press
pub fn press_key(app: &mut App, key: KeyCode) {
    // Clear the input state to ensure just_pressed works correctly
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
}

/// Helper to simulate a key release
pub fn release_key(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(key);
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
    fn test_key_helpers() {
        let mut app = create_test_app();

        press_key(&mut app, KeyCode::Space);
        let input = app.world().resource::<ButtonInput<KeyCode>>();
        assert!(input.pressed(KeyCode::Space));

        release_key(&mut app, KeyCode::Space);
        app.update(); // Need to update for release to take effect
        let input = app.world().resource::<ButtonInput<KeyCode>>();
        assert!(!input.pressed(KeyCode::Space));
    }
}
