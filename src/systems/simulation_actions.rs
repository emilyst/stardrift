use crate::config::SimulationConfig;
use crate::resources::*;
use crate::states::AppState;
use crate::systems::physics::spawn_simulation_bodies;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;

#[derive(Event)]
pub struct RestartSimulationEvent;

pub fn handle_restart_simulation_event(
    mut restart_events: EventReader<RestartSimulationEvent>,
    mut commands: Commands,
    simulation_bodies: Query<Entity, With<RigidBody>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<SharedRng>,
    body_count: Res<BodyCount>,
    mut barycenter: ResMut<Barycenter>,
    mut octree: ResMut<GravitationalOctree>,
    mut pan_orbit_camera: Single<&mut PanOrbitCamera>,
    config: Res<SimulationConfig>,
) {
    restart_events.read().for_each(|_| {
        simulation_bodies.iter().for_each(|entity| {
            commands.entity(entity).despawn();
        });

        **barycenter = None;

        octree.build(vec![]);

        pan_orbit_camera.target_focus = Vec3::ZERO;
        pan_orbit_camera.force_update = true;

        *rng = SharedRng::default();

        spawn_simulation_bodies(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut rng,
            **body_count,
            &config,
        );
    });
}

pub fn toggle_octree_visualization(settings: &mut ResMut<OctreeVisualizationSettings>) {
    settings.enabled = !settings.enabled;
}

pub fn toggle_barycenter_gizmo_visibility(settings: &mut ResMut<BarycenterGizmoVisibility>) {
    settings.enabled = !settings.enabled;
}

pub fn toggle_pause_simulation(
    current_state: &mut Res<State<AppState>>,
    next_state: &mut ResMut<NextState<AppState>>,
    time: &mut ResMut<Time<Physics>>,
) {
    match current_state.get() {
        AppState::Running => {
            next_state.set(AppState::Paused);
            time.pause();
        }
        AppState::Paused => {
            next_state.set(AppState::Running);
            time.unpause();
        }
        _ => {} // ignore Loading state
    }
}
