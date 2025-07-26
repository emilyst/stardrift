use crate::prelude::*;
use crate::resources::LoadingProgress;
#[cfg(not(target_arch = "wasm32"))]
use crate::systems::ui::handle_quit_button;
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
        handle_barycenter_gizmo_button, handle_octree_button, handle_pause_button,
        handle_restart_button, handle_screenshot_button, update_barycenter_gizmo_button_text,
        update_octree_button_text, update_pause_button_text,
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
        app.add_systems(
            Update,
            (
                handle_barycenter_gizmo_button,
                handle_octree_button,
                handle_pause_button,
                handle_restart_button,
                handle_screenshot_button,
                update_barycenter_gizmo_button_text,
                update_octree_button_text,
                update_pause_button_text,
            )
                .in_set(SimulationSet::UI)
                .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
        );

        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(
            Update,
            handle_quit_button
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
