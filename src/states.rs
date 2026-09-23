use bevy::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Startup: render pipelines compiling, physics held. Exits to
    /// `Running` or `Paused` once the GPU is ready (see the loading screen
    /// plugin).
    #[default]
    Loading,
    Running,
    Paused,
}
