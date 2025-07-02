use crate::config::SimulationConfig;
use crate::resources::*;
use crate::states::AppState;
use crate::systems::physics::spawn_simulation_bodies;
use avian3d::math::Vector;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;

pub fn restart_simulation(
    commands: &mut Commands,
    simulation_bodies: &Query<Entity, With<RigidBody>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    rng: &mut ResMut<SharedRng>,
    body_count: &Res<BodyCount>,
    current_barycenter: &mut ResMut<CurrentBarycenter>,
    previous_barycenter: &mut ResMut<PreviousBarycenter>,
    initial_barycenter: &mut ResMut<InitialBarycenter>,
    octree: &mut ResMut<GravitationalOctree>,
    pan_orbit_camera: &mut Single<&mut PanOrbitCamera>,
    config: &Res<SimulationConfig>,
) {
    for entity in simulation_bodies {
        commands.entity(entity).despawn();
    }

    ***current_barycenter = Vector::ZERO;
    ***previous_barycenter = Vector::ZERO;

    // Reset barycenter initialization state
    ***initial_barycenter = None;
    commands.remove_resource::<BarycenterShiftingEnabled>();

    octree.build(vec![]);

    pan_orbit_camera.target_focus = Vec3::ZERO;
    pan_orbit_camera.force_update = true;

    **rng = SharedRng::default();

    spawn_simulation_bodies(commands, meshes, materials, rng, ***body_count, config);
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
