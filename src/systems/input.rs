use crate::prelude::*;

pub fn quit_on_escape(keys: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write_default();
    }
}

pub fn restart_simulation_on_n(
    keys: Res<ButtonInput<KeyCode>>,
    mut restart_events: EventWriter<RestartSimulationEvent>,
) {
    if keys.just_pressed(KeyCode::KeyN) {
        restart_events.write(RestartSimulationEvent);
    }
}

pub fn pause_physics_on_space(
    keys: Res<ButtonInput<KeyCode>>,
    mut pause_events: EventWriter<TogglePauseSimulationEvent>,
) {
    if keys.just_pressed(KeyCode::Space) {
        pause_events.write(TogglePauseSimulationEvent);
    }
}

pub fn toggle_octree_visualization(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<OctreeVisualizationSettings>,
    mut octree_events: EventWriter<ToggleOctreeVisualizationEvent>,
) {
    for &keycode in keys.get_just_pressed() {
        match keycode {
            KeyCode::KeyO => {
                octree_events.write(ToggleOctreeVisualizationEvent);
            }
            KeyCode::Digit0 => settings.max_depth = None,
            KeyCode::Digit1 => settings.max_depth = Some(1),
            KeyCode::Digit2 => settings.max_depth = Some(2),
            KeyCode::Digit3 => settings.max_depth = Some(3),
            KeyCode::Digit4 => settings.max_depth = Some(4),
            KeyCode::Digit5 => settings.max_depth = Some(5),
            KeyCode::Digit6 => settings.max_depth = Some(6),
            KeyCode::Digit7 => settings.max_depth = Some(7),
            KeyCode::Digit8 => settings.max_depth = Some(8),
            KeyCode::Digit9 => settings.max_depth = Some(9),
            _ => {}
        }
    }
}

pub fn toggle_barycenter_gizmo_visibility_on_c(
    keys: Res<ButtonInput<KeyCode>>,
    mut barycenter_events: EventWriter<ToggleBarycenterGizmoVisibilityEvent>,
) {
    for &keycode in keys.get_just_pressed() {
        if keycode == KeyCode::KeyC {
            barycenter_events.write(ToggleBarycenterGizmoVisibilityEvent);
        }
    }
}

pub fn take_screenshot_on_s(
    keys: Res<ButtonInput<KeyCode>>,
    mut screenshot_events: EventWriter<TakeScreenshotEvent>,
) {
    if keys.just_pressed(KeyCode::KeyS) {
        screenshot_events.write(TakeScreenshotEvent);
    }
}
