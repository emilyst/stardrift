mod components;
mod config;
mod physics;
mod plugins;
mod resources;
mod states;
mod systems;
mod utils;

use crate::plugins::diagnostics_hud::DiagnosticsHudPlugin;
use crate::plugins::simulation::SimulationPlugin;
use crate::plugins::simulation_diagnostics::SimulationDiagnosticsPlugin;
use crate::states::AppState;
use crate::states::LoadingState;
use avian3d::prelude::*;
use bevy::diagnostic::EntityCountDiagnosticsPlugin;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy::diagnostic::SystemInformationDiagnosticsPlugin;
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCameraPlugin;

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins,
        DiagnosticsHudPlugin,
        EntityCountDiagnosticsPlugin,
        FrameTimeDiagnosticsPlugin::default(),
        LogDiagnosticsPlugin::default(),
        PanOrbitCameraPlugin,
        PhysicsPlugins::default(),
        SimulationDiagnosticsPlugin::default(),
        SystemInformationDiagnosticsPlugin,
        SimulationPlugin,
    ));

    // Initialize app states after DefaultPlugins (which includes StatesPlugin)
    app.init_state::<AppState>();
    app.add_sub_state::<LoadingState>();

    app.run();
}
