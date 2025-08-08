//! Simulation plugin - Self-contained plugin pattern
//!
//! This plugin contains all the core simulation functionality including
//! physics calculations, body spawning, and simulation control actions.
//! All systems, components, and resources are self-contained within this plugin.

use crate::prelude::*;

mod actions;
mod components;
mod physics;

use actions::{
    ScreenshotState, handle_restart_simulation_event, handle_take_screenshot_event,
    handle_toggle_pause_simulation_event, process_screenshot_capture,
};
use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use physics::{
    PhysicsSet, calculate_accelerations, counteract_barycentric_drift, integrate_motions,
    rebuild_octree, sync_transform_from_position,
};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimulationSet {
    Input,
    UI,
    Camera,
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
            .with_softening(config.physics.force_calculation_softening)
            .with_leaf_threshold(config.physics.octree_leaf_threshold),
        ));
        app.init_resource::<ScreenshotState>();

        // Set up integrator based on configuration
        use crate::config::IntegratorType;
        use crate::physics::integrators::SemiImplicitEuler;
        use crate::physics::resources::CurrentIntegrator;

        let integrator = match config.physics.integrator {
            IntegratorType::SemiImplicitEuler => Box::new(SemiImplicitEuler),
        };
        app.insert_resource(CurrentIntegrator(integrator));

        app.init_resource::<crate::physics::resources::PhysicsTime>();

        // New unified command event
        app.add_event::<SimulationCommand>();

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
                PhysicsSet::CalculateAccelerations,
                PhysicsSet::IntegrateMotions,
                PhysicsSet::SyncTransforms,
                PhysicsSet::CorrectBarycentricDrift,
            )
                .chain(),
        );

        app.configure_sets(
            Update,
            (
                SimulationSet::Input,
                SimulationSet::UI,
                SimulationSet::Camera,
            )
                .chain(),
        );

        app.add_systems(
            Startup,
            |mut commands: Commands,
             mut meshes: ResMut<Assets<Mesh>>,
             mut materials: ResMut<Assets<StandardMaterial>>,
             mut rng: ResMut<SharedRng>,
             body_count: Res<BodyCount>,
             config: Res<SimulationConfig>| {
                physics::spawn_simulation_bodies(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut rng,
                    **body_count,
                    &config,
                );
            },
        );

        app.add_systems(
            FixedUpdate,
            (
                rebuild_octree.in_set(PhysicsSet::BuildOctree),
                calculate_accelerations
                    .in_set(PhysicsSet::CalculateAccelerations)
                    .run_if(in_state(AppState::Running)),
                integrate_motions
                    .in_set(PhysicsSet::IntegrateMotions)
                    .run_if(in_state(AppState::Running)),
                sync_transform_from_position
                    .in_set(PhysicsSet::SyncTransforms)
                    .run_if(in_state(AppState::Running)),
                counteract_barycentric_drift.in_set(PhysicsSet::CorrectBarycentricDrift),
            ),
        );
        app.add_systems(
            Update,
            (
                handle_restart_simulation_event,
                handle_toggle_pause_simulation_event,
                handle_take_screenshot_event,
                process_screenshot_capture,
            )
                .in_set(SimulationSet::Input),
        );
    }
}
