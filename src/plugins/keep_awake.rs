//! Keep Awake plugin - Prevents screen idle/sleep during simulation
//!
//! This plugin uses the keepawake crate to prevent the screen from going
//! to sleep or dimming during the simulation. The feature is configurable
//! via config file and CLI flags.
//!
//! On WebAssembly, this plugin is a no-op since browsers have their own
//! wake lock mechanisms.

use crate::prelude::*;

/// Plugin that prevents screen idle/sleep when enabled
pub struct KeepAwakePlugin;

// Native implementation (not WASM)
#[cfg(not(target_arch = "wasm32"))]
impl Plugin for KeepAwakePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, initialize_keep_awake);
    }
}

// WebAssembly no-op implementation
#[cfg(target_arch = "wasm32")]
impl Plugin for KeepAwakePlugin {
    fn build(&self, _app: &mut App) {
        // No-op on WASM - browsers use their own Screen Wake Lock API
        info!("Screen sleep prevention not available on WebAssembly");
    }
}

// Native-only code
#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;

    /// Resource that holds the KeepAwake handle
    /// When this resource is dropped, screen sleep prevention stops
    #[derive(Resource)]
    pub struct KeepAwakeHandle {
        #[allow(dead_code)]
        handle: keepawake::KeepAwake,
    }

    /// Initializes screen sleep prevention based on configuration
    pub fn initialize_keep_awake(mut commands: Commands, config: Res<SimulationConfig>) {
        if !config.system.prevent_screen_sleep {
            info!("Screen sleep prevention disabled via configuration");
            return;
        }

        match keepawake::Builder::default()
            .display(true)
            .reason("Simulation running")
            .app_name("Stardrift")
            .app_reverse_domain("app.stardrift")
            .create()
        {
            Ok(handle) => {
                info!("Screen sleep prevention enabled");
                commands.insert_resource(KeepAwakeHandle { handle });
            }
            Err(e) => {
                warn!("Failed to enable screen sleep prevention: {e}");
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
use native::initialize_keep_awake;
