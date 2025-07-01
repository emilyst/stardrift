use crate::config::SimulationConfig;
use crate::physics::octree::Octree;
use crate::resources::*;
use crate::states::AppState;
use crate::states::LoadingState;
use crate::systems::camera;
use crate::systems::input;
use crate::systems::loading;
use crate::systems::physics;
use crate::systems::physics::PhysicsSet;
use crate::systems::ui;
use crate::systems::visualization;
use bevy::ecs::schedule::LogLevel;
use bevy::ecs::schedule::ScheduleBuildSettings;
use bevy::prelude::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimulationSet {
    Loading,
    Input,
    UI,
    Camera,
    Visualization,
    Configuration,
}

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        let config = SimulationConfig::load_from_user_config();

        app.insert_resource(config.clone());
        app.insert_resource(SharedRng::from_optional_seed(config.physics.initial_seed));
        app.insert_resource(GravitationalConstant(config.physics.gravitational_constant));
        app.insert_resource(BodyCount(config.physics.body_count));
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
        app.init_resource::<LoadingProgress>();

        app.edit_schedule(FixedUpdate, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Warn,
                ..default()
            });
        });

        app.configure_sets(
            FixedUpdate,
            (
                PhysicsSet::BuildOctree,
                PhysicsSet::ApplyForces,
                PhysicsSet::UpdateBarycenter,
            )
                .chain(),
        );

        app.configure_sets(
            Update,
            (
                SimulationSet::Loading,
                (
                    SimulationSet::Input,
                    SimulationSet::UI,
                    SimulationSet::Visualization,
                ), // These can run in parallel
                SimulationSet::Camera, // Camera follows after input/UI for responsiveness
            )
                .chain(),
        );

        app.add_systems(Startup, camera::spawn_camera);
        app.add_systems(
            OnEnter(AppState::Loading),
            (
                loading::setup_loading_screen,
                loading::start_loading_process,
            ),
        );
        app.add_systems(
            FixedUpdate,
            (
                physics::rebuild_octree
                    .in_set(PhysicsSet::BuildOctree)
                    .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
                physics::apply_gravitation_octree
                    .in_set(PhysicsSet::ApplyForces)
                    .run_if(in_state(AppState::Running)),
                physics::update_barycenter
                    .in_set(PhysicsSet::UpdateBarycenter)
                    .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
            ),
        );
        app.add_systems(
            Update,
            (
                loading::update_loading_progress.run_if(in_state(AppState::Loading)),
                loading::advance_loading_step.run_if(in_state(LoadingState::InitializingConfig)),
                loading::spawn_bodies_async.run_if(in_state(LoadingState::SpawningBodies)),
                loading::finalize_loading.run_if(in_state(LoadingState::BuildingOctree)),
                loading::setup_ui_after_loading.run_if(in_state(LoadingState::SettingUpUI)),
                loading::complete_loading.run_if(in_state(AppState::Running)),
            )
                .in_set(SimulationSet::Loading),
        );
        app.add_systems(
            Update,
            camera::follow_barycenter
                .in_set(SimulationSet::Camera)
                .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
        );
        app.add_systems(
            Update,
            (
                input::pause_physics_on_space,
                input::restart_simulation_on_n,
                input::toggle_barycenter_gizmo_visibility_on_c,
                input::toggle_octree_visualization,
            )
                .in_set(SimulationSet::Input)
                .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
        );
        app.add_systems(
            Update,
            (
                ui::handle_barycenter_gizmo_button,
                ui::handle_octree_button,
                ui::handle_pause_button,
                ui::handle_restart_button,
                ui::update_barycenter_gizmo_button_text,
                ui::update_octree_button_text,
                ui::update_pause_button_text,
            )
                .in_set(SimulationSet::UI)
                .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
        );
        app.add_systems(
            Update,
            visualization::visualize_octree
                .in_set(SimulationSet::Visualization)
                .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
        );

        app.add_systems(Update, input::quit_on_escape.in_set(SimulationSet::Input));
    }
}
