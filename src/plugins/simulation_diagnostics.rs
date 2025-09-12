//! Simulation diagnostics plugin - Self-contained plugin pattern
//!
//! This plugin follows the self-contained pattern for collecting and tracking
//! simulation-specific performance metrics. Currently a placeholder for future
//! diagnostics implementation such as octree build time, force calculation
//! performance, and physics step timing.

use crate::physics::components::{Mass, PhysicsBody, Position, Velocity};
use crate::physics::math::Scalar;
use crate::states::AppState;
use bevy::diagnostic::DEFAULT_MAX_HISTORY_LENGTH;
use bevy::diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic};
use bevy::prelude::*;
use core::time::Duration;

/// Lightweight resource for tracking simulation metrics
#[derive(Resource, Default)]
pub struct SimulationMetrics {
    /// Last calculated kinetic energy
    pub kinetic_energy: Scalar,
}

#[derive(Resource)]
pub struct SimulationDiagnosticsState {
    pub update_timer: Timer,
}

pub struct SimulationDiagnosticsPlugin {
    max_history_length: usize,
    smoothing_factor: f64,
    update_interval: Duration,
}

impl Default for SimulationDiagnosticsPlugin {
    fn default() -> Self {
        Self {
            max_history_length: DEFAULT_MAX_HISTORY_LENGTH,
            smoothing_factor: 0.0,
            update_interval: Duration::from_secs_f64(1_f64 / 60_f64),
        }
    }
}

impl SimulationDiagnosticsPlugin {
    /// Diagnostic path for kinetic energy
    pub const KINETIC_ENERGY: DiagnosticPath =
        DiagnosticPath::const_new("simulation/energy/kinetic");

    fn register_diagnostics(&self, app: &mut App) {
        // Register kinetic energy diagnostic with units
        app.register_diagnostic(
            Diagnostic::new(Self::KINETIC_ENERGY)
                .with_max_history_length(self.max_history_length)
                .with_smoothing_factor(self.smoothing_factor)
                .with_suffix("J"), // Add units for clarity
        );
    }

    fn update_timer_ticks(mut state: ResMut<SimulationDiagnosticsState>, time: Res<Time>) {
        state.update_timer.tick(time.delta());
    }

    /// Calculate total kinetic energy of the system
    ///
    /// Kinetic energy = ½ Σ(m·v²) for all bodies
    fn calculate_kinetic_energy(
        bodies: Query<(&Velocity, &Mass), With<PhysicsBody>>,
        mut metrics: ResMut<SimulationMetrics>,
        mut diagnostics: Diagnostics,
        state: Res<SimulationDiagnosticsState>,
    ) {
        // Only update when timer is ready to avoid excessive computation
        if !state.update_timer.finished() {
            return;
        }

        let kinetic_energy: Scalar = bodies
            .iter()
            .map(|(velocity, mass)| {
                let v_squared = velocity.value().length_squared();
                0.5 * mass.value() * v_squared
            })
            .sum();

        metrics.kinetic_energy = kinetic_energy;
        diagnostics.add_measurement(&Self::KINETIC_ENERGY, || kinetic_energy as f64);
    }
}

impl Plugin for SimulationDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimulationDiagnosticsState {
            update_timer: Timer::new(self.update_interval, TimerMode::Repeating),
        })
        .init_resource::<SimulationMetrics>();

        self.register_diagnostics(app);

        app.add_systems(
            FixedPostUpdate,
            (Self::update_timer_ticks, Self::calculate_kinetic_energy)
                .run_if(in_state(AppState::Running)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::math::Vector;
    use bevy::diagnostic::DiagnosticsStore;
    use bevy::state::app::StatesPlugin;

    /// Test helper to create a physics body with specified mass and velocity
    fn spawn_test_body(commands: &mut Commands, mass: Scalar, velocity: Vector) -> Entity {
        commands
            .spawn((
                PhysicsBody,
                Mass::new(mass),
                Velocity::new(velocity),
                Transform::default(),
            ))
            .id()
    }

    #[test]
    fn test_kinetic_energy_single_body() {
        // Create test app with diagnostic plugin
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>()
            .insert_state(AppState::Running)
            .add_plugins(SimulationDiagnosticsPlugin::default());

        // Spawn single body with known mass and velocity
        // KE = 0.5 * m * v²
        // m = 2.0, v = (3, 4, 0) => |v|² = 9 + 16 = 25
        // KE = 0.5 * 2.0 * 25 = 25.0 J
        let mass = 2.0;
        let velocity = Vector::new(3.0, 4.0, 0.0);
        spawn_test_body(&mut app.world_mut().commands(), mass, velocity);

        // Apply commands to actually spawn the entity
        app.world_mut().flush();

        // Advance the time resource to make the timer system work
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(Duration::from_secs(1));

        // Run the FixedPostUpdate schedule where our systems are registered
        app.world_mut().run_schedule(FixedPostUpdate);

        // Verify kinetic energy calculation
        let metrics = app.world().resource::<SimulationMetrics>();
        let expected_ke = 0.5 * mass * velocity.length_squared();
        assert!(
            (metrics.kinetic_energy - expected_ke).abs() < 1e-6,
            "Expected KE: {expected_ke}, got: {}",
            metrics.kinetic_energy
        );

        // Verify diagnostic was recorded
        let diagnostics_store = app.world().resource::<DiagnosticsStore>();
        if let Some(diagnostic) =
            diagnostics_store.get(&SimulationDiagnosticsPlugin::KINETIC_ENERGY)
        {
            assert!(
                diagnostic.measurement().is_some(),
                "Kinetic energy diagnostic should have a measurement"
            );
            if let Some(value) = diagnostic.measurement() {
                assert!(
                    (value.value - expected_ke as f64).abs() < 1e-6,
                    "Diagnostic value mismatch: expected {expected_ke}, got {}",
                    value.value
                );
            }
        } else {
            panic!("Kinetic energy diagnostic not found in store");
        }
    }

    #[test]
    fn test_kinetic_energy_multiple_bodies() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>()
            .insert_state(AppState::Running)
            .add_plugins(SimulationDiagnosticsPlugin::default());

        // Spawn multiple bodies with different masses and velocities
        // Body 1: m=1.0, v=(2,0,0) => KE = 0.5 * 1.0 * 4 = 2.0 J
        // Body 2: m=3.0, v=(0,1,0) => KE = 0.5 * 3.0 * 1 = 1.5 J
        // Body 3: m=2.0, v=(1,1,1) => KE = 0.5 * 2.0 * 3 = 3.0 J
        // Total KE = 6.5 J
        spawn_test_body(
            &mut app.world_mut().commands(),
            1.0,
            Vector::new(2.0, 0.0, 0.0),
        );
        spawn_test_body(
            &mut app.world_mut().commands(),
            3.0,
            Vector::new(0.0, 1.0, 0.0),
        );
        spawn_test_body(
            &mut app.world_mut().commands(),
            2.0,
            Vector::new(1.0, 1.0, 1.0),
        );

        // Apply commands to actually spawn the entities
        app.world_mut().flush();

        // Advance the time resource to make the timer system work
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(Duration::from_secs(1));

        // Run the FixedPostUpdate schedule where our systems are registered
        app.world_mut().run_schedule(FixedPostUpdate);

        let metrics = app.world().resource::<SimulationMetrics>();
        let expected_total_ke = 2.0 + 1.5 + 3.0;
        assert!(
            (metrics.kinetic_energy - expected_total_ke).abs() < 1e-6,
            "Expected total KE: {expected_total_ke}, got: {}",
            metrics.kinetic_energy
        );
    }

    #[test]
    fn test_kinetic_energy_zero_velocity() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>()
            .insert_state(AppState::Running)
            .add_plugins(SimulationDiagnosticsPlugin::default());

        // Spawn body with zero velocity
        spawn_test_body(&mut app.world_mut().commands(), 5.0, Vector::ZERO);

        // Apply commands to actually spawn the entity
        app.world_mut().flush();

        // Advance the time resource to make the timer system work
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(Duration::from_secs(1));

        // Run the FixedPostUpdate schedule where our systems are registered
        app.world_mut().run_schedule(FixedPostUpdate);

        let metrics = app.world().resource::<SimulationMetrics>();
        assert!(
            metrics.kinetic_energy.abs() < 1e-10,
            "KE should be zero for stationary body, got: {}",
            metrics.kinetic_energy
        );
    }

    #[test]
    fn test_kinetic_energy_large_values() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>()
            .insert_state(AppState::Running)
            .add_plugins(SimulationDiagnosticsPlugin::default());

        // Test with large but realistic values
        // Mass of Earth-like body: 1e24 kg
        // Orbital velocity: 30000 m/s
        let mass = 1e24;
        let velocity = Vector::new(30000.0, 0.0, 0.0);
        spawn_test_body(&mut app.world_mut().commands(), mass, velocity);

        // Apply commands to actually spawn the entity
        app.world_mut().flush();

        // Advance the time resource to make the timer system work
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(Duration::from_secs(1));

        // Run the FixedPostUpdate schedule where our systems are registered
        app.world_mut().run_schedule(FixedPostUpdate);

        let metrics = app.world().resource::<SimulationMetrics>();
        let expected_ke = 0.5 * mass * velocity.length_squared();

        // Use relative error for large values
        let relative_error = ((metrics.kinetic_energy - expected_ke) / expected_ke).abs();
        assert!(
            relative_error < 1e-6,
            "Relative error too large: {relative_error}, expected KE: {expected_ke}, got: {}",
            metrics.kinetic_energy
        );
    }

    #[test]
    fn test_kinetic_energy_updates_only_when_timer_finished() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>()
            .insert_state(AppState::Running)
            .add_plugins(SimulationDiagnosticsPlugin {
                update_interval: Duration::from_secs(1),
                ..Default::default()
            });

        spawn_test_body(
            &mut app.world_mut().commands(),
            1.0,
            Vector::new(1.0, 0.0, 0.0),
        );

        // Apply commands to actually spawn the entity
        app.world_mut().flush();

        // First update without timer finishing - timer not advanced (time delta is 0)
        app.world_mut().run_schedule(FixedPostUpdate);

        let metrics = app.world().resource::<SimulationMetrics>();
        assert_eq!(
            metrics.kinetic_energy, 0.0,
            "KE should not update before timer finishes"
        );

        // Advance time and update
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(Duration::from_secs(1));

        // Run the FixedPostUpdate schedule where our systems are registered
        app.world_mut().run_schedule(FixedPostUpdate);

        let metrics = app.world().resource::<SimulationMetrics>();
        assert!(
            metrics.kinetic_energy > 0.0,
            "KE should update after timer finishes"
        );
    }

    #[test]
    fn test_diagnostic_units_suffix() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(SimulationDiagnosticsPlugin::default());

        // Verify diagnostic is registered with correct units
        let diagnostics_store = app.world().resource::<DiagnosticsStore>();
        if let Some(diagnostic) =
            diagnostics_store.get(&SimulationDiagnosticsPlugin::KINETIC_ENERGY)
        {
            assert_eq!(
                diagnostic.suffix, "J",
                "Kinetic energy diagnostic should have Joules suffix"
            );
        } else {
            panic!("Kinetic energy diagnostic not found");
        }
    }

    #[test]
    fn test_kinetic_energy_changes_after_velocity_update() {
        // This test verifies that KE changes correctly when velocities are modified
        // simulating what would happen after a physics timestep
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>()
            .insert_state(AppState::Running)
            .add_plugins(SimulationDiagnosticsPlugin::default());

        // Create two bodies initially at rest
        let body1 = app
            .world_mut()
            .spawn((
                PhysicsBody,
                Mass::new(1.0),
                Position::new(Vector::new(-5.0, 0.0, 0.0)),
                Velocity::new(Vector::ZERO),
                Transform::default(),
            ))
            .id();

        let body2 = app
            .world_mut()
            .spawn((
                PhysicsBody,
                Mass::new(1.0),
                Position::new(Vector::new(5.0, 0.0, 0.0)),
                Velocity::new(Vector::ZERO),
                Transform::default(),
            ))
            .id();

        app.world_mut().flush();

        // Advance time for timer
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(Duration::from_secs(1));

        // Measure initial KE (should be 0)
        app.world_mut().run_schedule(FixedPostUpdate);
        let initial_ke = app.world().resource::<SimulationMetrics>().kinetic_energy;
        assert_eq!(initial_ke, 0.0, "Initial KE should be 0 for bodies at rest");

        // Simulate what physics would do: add velocities as if gravitational attraction occurred
        // For two unit masses at distance 10 with G=1:
        // F = G*m1*m2/r² = 1*1*1/100 = 0.01
        // a = F/m = 0.01
        // After dt=0.01: Δv = a*dt = 0.01*0.01 = 0.0001
        let velocity_change = 0.0001;

        // Body 1 moves right, body 2 moves left (attracted to each other)
        app.world_mut()
            .entity_mut(body1)
            .insert(Velocity::new(Vector::new(velocity_change, 0.0, 0.0)));
        app.world_mut()
            .entity_mut(body2)
            .insert(Velocity::new(Vector::new(-velocity_change, 0.0, 0.0)));

        // Measure KE after velocity update
        app.world_mut().run_schedule(FixedPostUpdate);
        let final_ke = app.world().resource::<SimulationMetrics>().kinetic_energy;

        // Expected KE = 2 * (0.5 * 1 * (0.0001)²) = 1e-8
        let expected_ke = 1e-8;

        assert!(
            (final_ke - expected_ke).abs() < 1e-10,
            "KE after velocity update: expected {}, got {}",
            expected_ke,
            final_ke
        );
    }

    #[test]
    fn test_kinetic_energy_with_circular_orbit_velocity() {
        // Test KE calculation for a body with circular orbit velocity
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>()
            .insert_state(AppState::Running)
            .add_plugins(SimulationDiagnosticsPlugin::default());

        // Create central mass and orbiting body
        // For circular orbit: v = sqrt(GM/r)
        // With G=1, M=100, r=10: v = sqrt(100/10) = sqrt(10) ≈ 3.162
        let _central = app
            .world_mut()
            .spawn((
                PhysicsBody,
                Mass::new(100.0),
                Position::new(Vector::ZERO),
                Velocity::new(Vector::ZERO),
                Transform::default(),
            ))
            .id();

        let orbital_velocity = 10.0_f64.sqrt(); // sqrt(GM/r) = sqrt(100/10)
        let _orbiter = app
            .world_mut()
            .spawn((
                PhysicsBody,
                Mass::new(1.0),
                Position::new(Vector::new(10.0, 0.0, 0.0)),
                Velocity::new(Vector::new(0.0, orbital_velocity, 0.0)),
                Transform::default(),
            ))
            .id();

        app.world_mut().flush();

        // Advance timer
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(Duration::from_secs(1));

        // Measure KE
        app.world_mut().run_schedule(FixedPostUpdate);
        let ke = app.world().resource::<SimulationMetrics>().kinetic_energy;

        // KE = 0.5 * m * v² = 0.5 * 1 * 10 = 5.0 (orbiter) + 0 (central)
        let expected_ke = 5.0;
        assert!(
            (ke - expected_ke).abs() < 1e-6,
            "Circular orbit KE: expected {}, got {}",
            expected_ke,
            ke
        );
    }
}
