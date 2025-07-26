use crate::prelude::*;
use crate::resources::LoadingProgress;
#[cfg(not(target_arch = "wasm32"))]
use crate::systems::ui::QuitButton;
use crate::systems::{
    camera::{draw_barycenter_gizmo, spawn_camera},
    input::{
        pause_physics_on_space, quit_on_escape, restart_simulation_on_n, take_screenshot_on_s,
        toggle_barycenter_gizmo_visibility_on_c, toggle_octree_visualization,
    },
    loading::{
        advance_loading_step, complete_loading, finalize_loading, setup_loading_screen,
        setup_ui_after_loading, spawn_bodies_async, start_loading_process, update_loading_progress,
    },
    physics::{PhysicsSet, apply_gravitation_octree, counteract_barycentric_drift, rebuild_octree},
    simulation_actions::{
        ScreenshotState, handle_restart_simulation_event, handle_take_screenshot_event,
        handle_toggle_barycenter_gizmo_visibility_event, handle_toggle_octree_visualization_event,
        handle_toggle_pause_simulation_event, process_screenshot_capture,
    },
    ui::{
        BarycenterGizmoToggleButton, OctreeToggleButton, PauseButton, RestartSimulationButton,
        ScreenshotButton, emit_barycenter_button_update, emit_octree_button_update,
        emit_pause_button_update_on_state_change, handle_button_interaction, update_button_text,
    },
    visualization::visualize_octree,
};
#[cfg(feature = "diagnostics")]
use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};

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

        match toml::to_string_pretty(&config) {
            Ok(toml_string) => {
                info!("=== Current Configuration (TOML) ===\n{}", toml_string);
                info!("=== End Configuration ===");
            }
            Err(e) => {
                error!("Failed to serialize configuration to TOML: {}", e);
            }
        }

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
        app.init_resource::<ScreenshotState>();

        app.add_event::<RestartSimulationEvent>();
        app.add_event::<ToggleOctreeVisualizationEvent>();
        app.add_event::<ToggleBarycenterGizmoVisibilityEvent>();
        app.add_event::<TogglePauseSimulationEvent>();
        app.add_event::<TakeScreenshotEvent>();

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
                SimulationSet::Input,
                SimulationSet::UI,
                SimulationSet::Visualization,
                SimulationSet::Camera,
            )
                .chain(),
        );

        app.add_systems(Startup, spawn_camera);
        app.add_systems(
            OnEnter(AppState::Loading),
            (setup_loading_screen, start_loading_process),
        );

        app.add_systems(
            OnEnter(AppState::Running),
            emit_pause_button_update_on_state_change,
        );
        app.add_systems(
            OnEnter(AppState::Paused),
            emit_pause_button_update_on_state_change,
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
                take_screenshot_on_s,
                toggle_barycenter_gizmo_visibility_on_c,
                toggle_octree_visualization,
                handle_restart_simulation_event,
                handle_toggle_octree_visualization_event,
                handle_toggle_barycenter_gizmo_visibility_event,
                handle_toggle_pause_simulation_event,
                handle_take_screenshot_event,
                process_screenshot_capture,
            )
                .in_set(SimulationSet::Input)
                .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
        );
        app.add_event::<UpdateButtonTextEvent<OctreeToggleButton>>();
        app.add_event::<UpdateButtonTextEvent<BarycenterGizmoToggleButton>>();
        app.add_event::<UpdateButtonTextEvent<PauseButton>>();

        app.add_systems(
            Update,
            (
                handle_button_interaction::<BarycenterGizmoToggleButton>,
                handle_button_interaction::<OctreeToggleButton>,
                handle_button_interaction::<PauseButton>,
                handle_button_interaction::<RestartSimulationButton>,
                handle_button_interaction::<ScreenshotButton>,
                emit_octree_button_update,
                emit_barycenter_button_update,
                update_button_text::<OctreeToggleButton>,
                update_button_text::<BarycenterGizmoToggleButton>,
                update_button_text::<PauseButton>,
            )
                .in_set(SimulationSet::UI)
                .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
        );

        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(
            Update,
            handle_button_interaction::<QuitButton>
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
