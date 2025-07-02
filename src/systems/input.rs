use crate::config::SimulationConfig;
use crate::resources::*;
use crate::states::AppState;
use crate::systems::simulation_actions;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;

pub fn quit_on_escape(keys: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write_default();
    }
}

pub fn restart_simulation_on_n(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    simulation_bodies: Query<Entity, With<RigidBody>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<SharedRng>,
    body_count: Res<BodyCount>,
    mut current_barycenter: ResMut<CurrentBarycenter>,
    mut previous_barycenter: ResMut<PreviousBarycenter>,
    mut initial_barycenter: ResMut<InitialBarycenter>,
    mut octree: ResMut<GravitationalOctree>,
    mut pan_orbit_camera: Single<&mut PanOrbitCamera>,
    config: Res<SimulationConfig>,
) {
    if keys.just_pressed(KeyCode::KeyN) {
        simulation_actions::restart_simulation(
            &mut commands,
            &simulation_bodies,
            &mut meshes,
            &mut materials,
            &mut rng,
            &body_count,
            &mut current_barycenter,
            &mut previous_barycenter,
            &mut initial_barycenter,
            &mut octree,
            &mut pan_orbit_camera,
            &config,
        );
    }
}

pub fn pause_physics_on_space(
    keys: Res<ButtonInput<KeyCode>>,
    mut current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut time: ResMut<Time<Physics>>,
) {
    if keys.just_pressed(KeyCode::Space) {
        simulation_actions::toggle_pause_simulation(&mut current_state, &mut next_state, &mut time);
    }
}

pub fn toggle_octree_visualization(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<OctreeVisualizationSettings>,
) {
    for &keycode in keys.get_just_pressed() {
        match keycode {
            KeyCode::KeyO => simulation_actions::toggle_octree_visualization(&mut settings),
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
    mut visibility: ResMut<BarycenterGizmoVisibility>,
) {
    for &keycode in keys.get_just_pressed() {
        match keycode {
            KeyCode::KeyC => {
                simulation_actions::toggle_barycenter_gizmo_visibility(&mut visibility)
            }
            _ => {}
        }
    }
}
