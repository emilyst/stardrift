use crate::config::SimulationConfig;
use crate::physics::octree::Octree;
use crate::resources::*;
use crate::systems::{camera, config, input, physics, ui, visualization};
use bevy::ecs::schedule::LogLevel;
use bevy::ecs::schedule::ScheduleBuildSettings;
use bevy::prelude::*;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        let config = SimulationConfig::load_from_user_config();

        app.insert_resource(config.clone());
        app.init_resource::<SharedRng>();
        app.insert_resource(GravitationalConstant(config.physics.gravitational_constant));
        app.insert_resource(BodyCount(config.physics.default_body_count));
        app.init_resource::<CurrentBarycenter>();
        app.init_resource::<PreviousBarycenter>();
        app.insert_resource(GravitationalOctree::new(Octree::new(
            config.physics.octree_theta,
            config.physics.force_calculation_min_distance,
            config.physics.force_calculation_max_force,
        )));
        app.insert_resource(OctreeVisualizationSettings {
            enabled: false,
            ..default()
        });
        app.init_resource::<BarycenterGizmoVisibility>();

        app.edit_schedule(FixedUpdate, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Warn,
                ..default()
            });
        });

        app.add_systems(Startup, (camera::spawn_camera, physics::spawn_bodies));
        app.add_systems(PostStartup, ui::setup_ui);

        // run in FixedUpdate for deterministic physics
        app.add_systems(
            FixedUpdate,
            (
                physics::rebuild_octree,
                physics::apply_gravitation_octree,
                physics::update_barycenter,
                camera::follow_barycenter,
            )
                .chain(),
        );

        // run in Update for responsive input
        app.add_systems(
            Update,
            (
                input::quit_on_escape,
                input::restart_simulation_on_r,
                input::pause_physics_on_space,
                input::toggle_octree_visualization,
                visualization::visualize_octree,
                ui::handle_octree_button,
                ui::handle_barycenter_gizmo_button,
                ui::handle_restart_button,
                ui::update_octree_button_text,
                ui::update_barycenter_gizmo_button_text,
            ),
        );

        // runs in Last schedule to ensure it runs after all other systems
        app.add_systems(Last, config::save_config_on_exit);
    }
}
