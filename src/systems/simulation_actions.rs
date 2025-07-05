use crate::config;
use crate::resources;
use crate::states;
use crate::systems;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;

#[derive(Event)]
pub struct RestartSimulationEvent;

#[derive(Event)]
pub struct ToggleOctreeVisualizationEvent;

#[derive(Event)]
pub struct ToggleBarycenterGizmoVisibilityEvent;

#[derive(Event)]
pub struct TogglePauseSimulationEvent;

#[allow(clippy::too_many_arguments)]
pub fn handle_restart_simulation_event(
    mut restart_events: EventReader<RestartSimulationEvent>,
    mut commands: Commands,
    simulation_bodies: Query<Entity, With<RigidBody>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<resources::SharedRng>,
    body_count: Res<resources::BodyCount>,
    mut barycenter: ResMut<resources::Barycenter>,
    mut octree: ResMut<resources::GravitationalOctree>,
    mut pan_orbit_camera: Single<&mut PanOrbitCamera>,
    config: Res<config::SimulationConfig>,
) {
    restart_events.read().for_each(|_| {
        simulation_bodies.iter().for_each(|entity| {
            commands.entity(entity).despawn();
        });

        **barycenter = None;

        octree.build(vec![]);

        pan_orbit_camera.target_focus = Vec3::ZERO;
        pan_orbit_camera.force_update = true;

        *rng = resources::SharedRng::default();

        systems::physics::spawn_simulation_bodies(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut rng,
            **body_count,
            &config,
        );
    });
}

pub fn handle_toggle_octree_visualization_event(
    mut octree_events: EventReader<ToggleOctreeVisualizationEvent>,
    mut settings: ResMut<resources::OctreeVisualizationSettings>,
) {
    octree_events.read().for_each(|_| {
        settings.enabled = !settings.enabled;
    });
}

pub fn handle_toggle_barycenter_gizmo_visibility_event(
    mut barycenter_events: EventReader<ToggleBarycenterGizmoVisibilityEvent>,
    mut settings: ResMut<resources::BarycenterGizmoVisibility>,
) {
    barycenter_events.read().for_each(|_| {
        settings.enabled = !settings.enabled;
    });
}

pub fn handle_toggle_pause_simulation_event(
    mut pause_events: EventReader<TogglePauseSimulationEvent>,
    current_state: Res<State<states::AppState>>,
    mut next_state: ResMut<NextState<states::AppState>>,
    mut time: ResMut<Time<Physics>>,
) {
    pause_events.read().for_each(|_| {
        match current_state.get() {
            states::AppState::Running => {
                next_state.set(states::AppState::Paused);
                time.pause();
            }
            states::AppState::Paused => {
                next_state.set(states::AppState::Running);
                time.unpause();
            }
            _ => {} // ignore Loading state
        }
    });
}
