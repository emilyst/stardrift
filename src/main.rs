mod components;
mod config;
mod physics;
mod plugins;
mod resources;
mod systems;
mod utils;

use crate::plugins::diagnostics_hud::DiagnosticsHudPlugin;
use crate::plugins::simulation::SimulationPlugin;
use crate::plugins::simulation_diagnostics::SimulationDiagnosticsPlugin;
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

    app.run();
}
