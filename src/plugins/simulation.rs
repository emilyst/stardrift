use crate::physics::octree::Octree;
use crate::resources::*;
use crate::systems::{camera, input, physics, ui, visualization};
use bevy::ecs::schedule::LogLevel;
use bevy::ecs::schedule::ScheduleBuildSettings;
use bevy::prelude::*;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        // Initialize resources
        app.init_resource::<SharedRng>();
        app.init_resource::<GravitationalConstant>();
        app.init_resource::<BodyCount>();
        app.init_resource::<CurrentBarycenter>();
        app.init_resource::<PreviousBarycenter>();
        app.insert_resource(GravitationalOctree(Octree::new(0.5))); // theta = 0.5 for Barnes-Hut approximation
        app.insert_resource(OctreeVisualizationSettings {
            enabled: false,
            ..default()
        });

        // Configure schedule settings
        app.edit_schedule(FixedUpdate, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Warn,
                ..default()
            });
        });

        // Add startup systems
        app.add_systems(Startup, (camera::spawn_camera, physics::spawn_bodies));
        app.add_systems(PostStartup, ui::setup_ui);

        // Add physics systems (run in FixedUpdate for deterministic physics)
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

        // Add input and UI systems (run in Update for responsive input)
        app.add_systems(
            Update,
            (
                // Input systems
                input::quit_on_escape,
                input::restart_simulation_on_r,
                input::pause_physics_on_space,
                input::toggle_octree_visualization,
                // Visualization systems
                visualization::visualize_octree,
                // UI systems
                ui::handle_octree_button,
                ui::handle_restart_button,
                ui::update_octree_button_text,
            ),
        );
    }
}
