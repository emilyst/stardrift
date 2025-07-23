use crate::prelude::*;
use crate::systems::physics::spawn_simulation_bodies;
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
    #[cfg(feature = "trails")] trail_renderers: Query<
        Entity,
        With<crate::systems::trails::TrailRenderer>,
    >,
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
        // Despawn all bodies
        simulation_bodies.iter().for_each(|entity| {
            commands.entity(entity).despawn();
        });

        // Despawn all trail renderers
        #[cfg(feature = "trails")]
        trail_renderers.iter().for_each(|entity| {
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

pub fn handle_toggle_octree_visualization_event(
    mut octree_events: EventReader<ToggleOctreeVisualizationEvent>,
    mut settings: ResMut<OctreeVisualizationSettings>,
) {
    octree_events.read().for_each(|_| {
        settings.enabled = !settings.enabled;
    });
}

pub fn handle_toggle_barycenter_gizmo_visibility_event(
    mut barycenter_events: EventReader<ToggleBarycenterGizmoVisibilityEvent>,
    mut settings: ResMut<BarycenterGizmoVisibility>,
) {
    barycenter_events.read().for_each(|_| {
        settings.enabled = !settings.enabled;
    });
}

pub fn handle_toggle_pause_simulation_event(
    mut pause_events: EventReader<TogglePauseSimulationEvent>,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut time: ResMut<Time<Physics>>,
) {
    pause_events.read().for_each(|_| {
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
    });
}
