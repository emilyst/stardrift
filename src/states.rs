use bevy::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Loading,
    Running,
    Paused,
}

#[derive(SubStates, Debug, Clone, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::Loading)]
pub enum LoadingState {
    #[default]
    InitializingConfig,
    SpawningBodies,
    BuildingOctree,
    SettingUpUI,
}
