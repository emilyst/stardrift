//! Test utilities for all testing needs
//!
//! Organized by domain for easy discovery:
//! - `app`: Bevy app creation and input simulation
//! - `physics`: Physics testing utilities including acceleration fields

use bevy::input::ButtonState;
use bevy::input::keyboard::Key;
use bevy::prelude::*;

use crate::prelude::*;

// =============================================================================
// Bevy App Testing Utilities
// =============================================================================

/// Bevy app testing utilities
pub mod app {
    use super::*;

    /// Creates a minimal test app with core Bevy plugins needed for testing
    pub fn create_test_app() -> App {
        let mut app = App::new();

        // Add minimal plugins needed for testing
        app.add_plugins((
            MinimalPlugins.set(TaskPoolPlugin::default()),
            AssetPlugin::default(),
            bevy::input::InputPlugin,
            bevy::state::app::StatesPlugin,
            TransformPlugin,
            bevy::diagnostic::DiagnosticsPlugin,
        ));

        // Add physics time resource
        app.insert_resource(crate::physics::resources::PhysicsTime::default());

        // Initialize assets needed by various plugins
        app.init_asset::<Font>();
        app.init_asset::<Mesh>();
        app.init_asset::<Shader>();
        app.init_asset::<StandardMaterial>();

        // Add gizmo-related resources
        app.init_resource::<GizmoConfigStore>();

        // Add visualization resources
        app.init_resource::<BarycenterGizmoVisibility>();
        app.init_resource::<OctreeVisualizationSettings>();
        app.init_resource::<TrailsVisualizationSettings>();
        app.init_resource::<crate::plugins::diagnostics_hud::DiagnosticsHudSettings>();

        // Add events used by plugins
        app.add_event::<SimulationCommand>();

        // Add states
        app.init_state::<AppState>();

        app
    }

    /// Helper to simulate a keyboard input event
    pub fn send_keyboard_input(app: &mut App, key: Key, state: ButtonState) {
        use bevy::input::keyboard::KeyboardInput;

        // For tests, we can use a placeholder entity since we don't have actual windows
        let window_entity = Entity::PLACEHOLDER;

        let event = KeyboardInput {
            key_code: KeyCode::KeyA, // Placeholder, not used in new system
            logical_key: key,
            state,
            text: None,
            repeat: false,
            window: window_entity,
        };

        app.world_mut().send_event(event);
    }

    /// Helper to simulate a character key press
    pub fn press_character_key(app: &mut App, character: &str) {
        send_keyboard_input(app, Key::Character(character.into()), ButtonState::Pressed);
    }

    /// Helper to simulate a special key press
    pub fn press_special_key(app: &mut App, key: Key) {
        send_keyboard_input(app, key, ButtonState::Pressed);
    }
}

// =============================================================================
// Physics Testing Utilities
// =============================================================================

/// Physics testing utilities
pub mod physics {
    /// Common acceleration fields for integrator testing and benchmarks
    pub mod acceleration_functions {
        use crate::physics::integrators::AccelerationField;
        use crate::physics::math::{Scalar, Vector};

        /// Constant acceleration in a specified direction (e.g., uniform gravity)
        pub struct ConstantAcceleration {
            pub acceleration: Vector,
        }

        impl AccelerationField for ConstantAcceleration {
            fn at(&self, _position: Vector) -> Vector {
                self.acceleration
            }
        }

        impl Default for ConstantAcceleration {
            fn default() -> Self {
                Self {
                    acceleration: Vector::new(0.0, 0.0, -9.81), // Earth gravity
                }
            }
        }

        /// Harmonic oscillator / Spring force: a = -k * x
        /// where k is the spring constant (or omega^2 for oscillators)
        pub struct HarmonicOscillator {
            pub k: Scalar,
        }

        impl HarmonicOscillator {
            /// Create from angular frequency omega (k = omega^2)
            pub fn from_omega(omega: Scalar) -> Self {
                Self { k: omega * omega }
            }
        }

        impl AccelerationField for HarmonicOscillator {
            fn at(&self, position: Vector) -> Vector {
                -self.k * position
            }
        }

        impl Default for HarmonicOscillator {
            fn default() -> Self {
                Self { k: 1.0 }
            }
        }

        /// Central force problem (e.g., Kepler orbits): a = -μ/r³ * r_vec
        /// where μ = GM for gravitational problems
        pub struct CentralForce {
            pub mu: Scalar,
        }

        impl AccelerationField for CentralForce {
            fn at(&self, position: Vector) -> Vector {
                let r = position.length();
                if r > 1e-10 {
                    -position * (self.mu / (r * r * r))
                } else {
                    Vector::ZERO
                }
            }
        }

        impl Default for CentralForce {
            fn default() -> Self {
                Self { mu: 1.0 }
            }
        }

        /// N-body acceleration field using octree for efficient force calculation
        /// Useful for testing integrators with realistic many-body forces
        pub struct NBodyAcceleration<'a> {
            pub octree: &'a crate::physics::octree::Octree,
            pub entity: bevy::ecs::entity::Entity,
            pub mass: Scalar,
            pub g: Scalar,
        }

        impl<'a> AccelerationField for NBodyAcceleration<'a> {
            fn at(&self, position: Vector) -> Vector {
                let force = self.octree.calculate_force_at_position(
                    position,
                    self.mass,
                    self.entity,
                    self.g,
                );
                force / self.mass
            }
        }
    }
}

// =============================================================================
// Backward Compatibility Exports
// =============================================================================

// Re-export app utilities at the top level for backward compatibility
pub use app::{create_test_app, press_character_key, press_special_key, send_keyboard_input};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_app() {
        let app = create_test_app();
        assert!(app.world().contains_resource::<Time>());
        assert!(app.world().contains_resource::<ButtonInput<KeyCode>>());
    }

    #[test]
    fn test_keyboard_input_helpers() {
        let mut app = create_test_app();

        // Add a system to consume events (verifies they're being sent properly)
        app.add_systems(
            Update,
            |mut events: EventReader<bevy::input::keyboard::KeyboardInput>| {
                for _ in events.read() {
                    // Just consume the events to verify they're being sent
                }
            },
        );

        // Test character key press
        press_character_key(&mut app, "a");
        app.update();

        // Test special key press
        press_special_key(&mut app, Key::Space);
        app.update();

        // The test passes if no panic occurs during event sending
    }
}
