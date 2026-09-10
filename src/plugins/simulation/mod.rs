//! Simulation plugin - Self-contained plugin pattern
//!
//! This plugin contains all the core simulation functionality including
//! physics calculations, body spawning, and simulation control actions.
//! All systems, components, and resources are self-contained within this plugin.

use crate::prelude::*;

pub mod actions;
pub mod collisions;
mod components;
pub mod physics;

use crate::physics::integrators::VelocityVerlet;
use crate::physics::integrators::registry::IntegratorRegistry;
use crate::physics::resources::{BhProbeState, CurrentIntegrator};
use actions::{handle_restart_simulation_event, handle_toggle_pause_simulation_event};
use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use physics::{
    PhysicsSet, counteract_barycentric_drift, integrate_motions, sync_transform_from_position,
};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimulationSet {
    Input,
    UI,
    Camera,
}

pub struct SimulationPlugin {
    config: Option<SimulationConfig>,
}

impl SimulationPlugin {
    pub fn new() -> Self {
        Self { config: None }
    }

    pub fn with_config(config: SimulationConfig) -> Self {
        Self {
            config: Some(config),
        }
    }
}

impl Default for SimulationPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        let config = self
            .config
            .clone()
            .unwrap_or_else(SimulationConfig::load_from_user_config);

        match toml::to_string_pretty(&config) {
            Ok(toml_string) => {
                debug!("=== Current Configuration (TOML) ===\n{}", toml_string);
                debug!("=== End Configuration ===");
            }
            Err(e) => {
                error!("Failed to serialize configuration to TOML: {}", e);
            }
        }

        app.insert_resource(config.clone());
        app.insert_resource(SharedRng::from_optional_seed(config.physics.initial_seed));
        app.insert_resource(RenderingRng::from_optional_seed(
            config.physics.initial_seed,
        ));
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

        // Create integrator using flexible configuration system
        let registry = IntegratorRegistry::new().with_standard_integrators();
        let integrator: Box<dyn crate::physics::integrators::Integrator + Send + Sync> =
            match registry.create(&config.physics.integrator.integrator_type) {
                Ok(integrator) => integrator,
                Err(e) => {
                    warn!(
                        "Failed to create integrator '{}': {}. Falling back to velocity_verlet",
                        config.physics.integrator.integrator_type, e
                    );
                    Box::new(VelocityVerlet)
                }
            };
        app.insert_resource(CurrentIntegrator(integrator));
        app.insert_resource(IntegratorRegistry::default());

        // Pin the FixedUpdate rate to the physics tick rate; PhysicsTime::dt
        // derives from the same constant, so the schedule and the integration
        // step cannot drift apart.
        app.insert_resource(Time::<Fixed>::from_hz(
            crate::physics::resources::PHYSICS_TICK_HZ,
        ));
        app.init_resource::<crate::physics::resources::PhysicsTime>();
        app.insert_resource(BhProbeState::from_config(&config.physics.bh_probe));

        // New unified command event
        app.add_message::<SimulationCommand>();

        app.edit_schedule(FixedUpdate, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Warn,
                ..default()
            });
        });

        // Transform sync runs last so drift corrections reach Transform in
        // the same fixed step. The octree build is internal to the
        // integration driver (one build per integrator stage). Collision
        // detection sits between integration (which writes both endpoints of
        // each body's swept segment) and drift correction (which must see
        // the post-merge set, and would otherwise put the segment endpoints
        // in different frames).
        app.configure_sets(
            FixedUpdate,
            (
                PhysicsSet::IntegrateMotions,
                PhysicsSet::DetectCollisions,
                PhysicsSet::CorrectBarycentricDrift,
                PhysicsSet::SyncTransforms,
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

        app.add_systems(Startup, physics::spawn_simulation_bodies);

        // Pause is governed by PhysicsTime alone (checked inside the physics
        // systems); AppState remains a UI-level concept. Transform sync is
        // ungated: it is a render sync whose Changed<Position> filter makes
        // it a near-no-op when nothing moved.
        app.add_systems(
            FixedUpdate,
            (
                integrate_motions.in_set(PhysicsSet::IntegrateMotions),
                collisions::detect_and_merge_collisions.in_set(PhysicsSet::DetectCollisions),
                counteract_barycentric_drift.in_set(PhysicsSet::CorrectBarycentricDrift),
                sync_transform_from_position.in_set(PhysicsSet::SyncTransforms),
            ),
        );
        // Core simulation command handlers
        app.add_systems(
            Update,
            (
                handle_restart_simulation_event,
                handle_toggle_pause_simulation_event,
            )
                .in_set(SimulationSet::Input),
        );
    }
}
