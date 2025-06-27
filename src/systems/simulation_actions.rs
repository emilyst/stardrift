use crate::config::SimulationConfig;
use crate::resources::*;
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
    octree: &ResMut<GravitationalOctree>,
    pan_orbit_camera: &mut Single<&mut PanOrbitCamera>,
    config: &Res<SimulationConfig>,
) {
    for entity in simulation_bodies {
        commands.entity(entity).despawn();
    }

    ***current_barycenter = Vector::ZERO;
    ***previous_barycenter = Vector::ZERO;

    if let Ok(mut octree_guard) = octree.write() {
        octree_guard.build(vec![]);
    }

    pan_orbit_camera.target_focus = Vec3::ZERO;
    pan_orbit_camera.force_update = true;

    **rng = SharedRng::default();

    spawn_simulation_bodies(commands, meshes, materials, rng, ***body_count, config);
}

pub fn toggle_octree_visualization(settings: &mut ResMut<OctreeVisualizationSettings>) {
    settings.enabled = !settings.enabled;
}
