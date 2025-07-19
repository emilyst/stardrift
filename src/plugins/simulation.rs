use crate::config::SimulationConfig;
use crate::physics::octree::Octree;
use crate::resources::Barycenter;
use crate::resources::BarycenterGizmoVisibility;
use crate::resources::BodyCount;
use crate::resources::GravitationalConstant;
use crate::resources::GravitationalOctree;
use crate::resources::LoadingProgress;
use crate::resources::OctreeVisualizationSettings;
use crate::resources::SharedRng;
use crate::states::AppState;
use crate::states::LoadingState;
use crate::systems::camera::draw_barycenter_gizmo;
use crate::systems::camera::spawn_camera;
use crate::systems::input::pause_physics_on_space;
use crate::systems::input::quit_on_escape;
use crate::systems::input::restart_simulation_on_n;
use crate::systems::input::toggle_barycenter_gizmo_visibility_on_c;
use crate::systems::input::toggle_octree_visualization;
use crate::systems::loading::advance_loading_step;
use crate::systems::loading::complete_loading;
use crate::systems::loading::finalize_loading;
use crate::systems::loading::setup_loading_screen;
use crate::systems::loading::setup_ui_after_loading;
use crate::systems::loading::spawn_bodies_async;
use crate::systems::loading::start_loading_process;
use crate::systems::loading::update_loading_progress;
use crate::systems::physics::PhysicsSet;
use crate::systems::physics::apply_gravitation_octree;
use crate::systems::physics::counteract_barycentric_drift;
use crate::systems::physics::rebuild_octree;
use crate::systems::simulation_actions::RestartSimulationEvent;
use crate::systems::simulation_actions::ToggleBarycenterGizmoVisibilityEvent;
use crate::systems::simulation_actions::ToggleOctreeVisualizationEvent;
use crate::systems::simulation_actions::TogglePauseSimulationEvent;
use crate::systems::simulation_actions::handle_restart_simulation_event;
use crate::systems::simulation_actions::handle_toggle_barycenter_gizmo_visibility_event;
use crate::systems::simulation_actions::handle_toggle_octree_visualization_event;
use crate::systems::simulation_actions::handle_toggle_pause_simulation_event;
use crate::systems::ui::handle_barycenter_gizmo_button;
use crate::systems::ui::handle_octree_button;
use crate::systems::ui::handle_pause_button;
use crate::systems::ui::handle_restart_button;
use crate::systems::ui::update_barycenter_gizmo_button_text;
use crate::systems::ui::update_octree_button_text;
use crate::systems::ui::update_pause_button_text;
use crate::systems::visualization::visualize_octree;
#[cfg(feature = "diagnostics")]
use bevy::ecs::schedule::LogLevel;
#[cfg(feature = "diagnostics")]
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
        app.init_resource::<Barycenter>();
        app.insert_resource(GravitationalOctree::new(
            Octree::new(
                config.physics.octree_theta,
                config.physics.force_calculation_min_distance,
                config.physics.force_calculation_max_force,
            )
            .with_leaf_threshold(config.physics.octree_leaf_threshold),
        ));
        app.insert_resource(OctreeVisualizationSettings {
            enabled: false,
            ..default()
        });
        app.init_resource::<BarycenterGizmoVisibility>();
        app.init_resource::<LoadingProgress>();

        app.add_event::<RestartSimulationEvent>();
        app.add_event::<ToggleOctreeVisualizationEvent>();
        app.add_event::<ToggleBarycenterGizmoVisibilityEvent>();
        app.add_event::<TogglePauseSimulationEvent>();

        #[cfg(feature = "diagnostics")]
        app.edit_schedule(FixedUpdate, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Warn,
                ..default()
            });
        });

        app.configure_sets(
            FixedUpdate,
            (PhysicsSet::BuildOctree, PhysicsSet::ApplyForces).chain(),
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

        app.add_systems(Startup, spawn_camera);
        app.add_systems(
            OnEnter(AppState::Loading),
            (setup_loading_screen, start_loading_process),
        );
        app.add_systems(
            FixedUpdate,
            (
                rebuild_octree
                    .in_set(PhysicsSet::BuildOctree)
                    .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
                apply_gravitation_octree
                    .in_set(PhysicsSet::ApplyForces)
                    .run_if(in_state(AppState::Running)),
                counteract_barycentric_drift
                    .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
            )
                .chain(),
        );
        app.add_systems(
            Update,
            (
                update_loading_progress.run_if(in_state(AppState::Loading)),
                advance_loading_step.run_if(in_state(LoadingState::InitializingConfig)),
                spawn_bodies_async.run_if(in_state(LoadingState::SpawningBodies)),
                finalize_loading.run_if(in_state(LoadingState::BuildingOctree)),
                setup_ui_after_loading.run_if(in_state(LoadingState::SettingUpUI)),
                complete_loading.run_if(in_state(AppState::Running)),
            )
                .in_set(SimulationSet::Loading),
        );
        app.add_systems(
            Update,
            draw_barycenter_gizmo
                .in_set(SimulationSet::Camera)
                .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
        );
        app.add_systems(
            Update,
            (
                pause_physics_on_space,
                restart_simulation_on_n,
                toggle_barycenter_gizmo_visibility_on_c,
                toggle_octree_visualization,
                handle_restart_simulation_event,
                handle_toggle_octree_visualization_event,
                handle_toggle_barycenter_gizmo_visibility_event,
                handle_toggle_pause_simulation_event,
            )
                .in_set(SimulationSet::Input)
                .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
        );
        app.add_systems(
            Update,
            (
                handle_barycenter_gizmo_button,
                handle_octree_button,
                handle_pause_button,
                handle_restart_button,
                update_barycenter_gizmo_button_text,
                update_octree_button_text,
                update_pause_button_text,
            )
                .in_set(SimulationSet::UI)
                .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
        );
        app.add_systems(
            Update,
            visualize_octree
                .in_set(SimulationSet::Visualization)
                .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
        );

        app.add_systems(Update, quit_on_escape.in_set(SimulationSet::Input));
    }
}
