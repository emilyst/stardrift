//! Loading screen plugin - holds the simulation until the GPU is ready.
//!
//! Bevy presents frames before its render pipelines have compiled; a mesh
//! simply does not draw until its pipeline is ready. Native compiles in a
//! frame or two, but WebGPU in the browser (Safari in particular) can take
//! several seconds, during which the user would see a black canvas while the
//! simulation advanced unseen. This plugin covers that window with an opaque
//! overlay while the app sits in `AppState::Loading` (where the simulation
//! plugin holds physics) until the pipeline cache has been idle for a few
//! consecutive frames, then hands off to the state named by
//! [`PostLoadingState`].
//!
//! Readiness detection follows Bevy's `loading_screen` example: a render
//! world system mirrors "no pipelines waiting" into the main world on every
//! extract, and the main world requires that to hold for several frames in a
//! row because pipelines are queued lazily as things first come into view.

use crate::plugins::simulation::SimulationSet;
use crate::prelude::*;
use bevy::asset::{AssetPath, LoadState, io::AssetSourceId};
use bevy::render::{ExtractSchedule, MainWorld, RenderApp, render_resource::PipelineCache};

/// Consecutive frames the pipeline cache must be idle (and the overlay font
/// loaded) before loading ends. Pipelines are queued lazily, so a single
/// idle frame proves little.
const CONFIRMATION_FRAMES: u32 = 5;

/// Upper bound on the loading state. Safari's WebGPU shader compilation
/// takes a few seconds; anything approaching this means readiness detection
/// has broken (a pipeline stuck waiting, an asset that never settles) and
/// the simulation should start anyway rather than hang.
const MAX_LOADING_SECONDS: f32 = 15.0;

/// Period of the overlay text's opacity pulse.
const PULSE_PERIOD_SECONDS: f32 = 2.0;
const PULSE_MIN_ALPHA: f32 = 0.3;
const PULSE_MAX_ALPHA: f32 = 0.7;

pub struct LoadingScreenPlugin;

/// State to enter once loading completes. `main` inserts this (`--paused`
/// selects `Paused`); the default hands off to `Running`.
#[derive(Resource, Debug, Clone)]
pub struct PostLoadingState(pub AppState);

impl Default for PostLoadingState {
    fn default() -> Self {
        Self(AppState::Running)
    }
}

/// Mirrored from the render world each extract: true when no pipelines are
/// queued or compiling.
#[derive(Resource, Debug, Default)]
pub struct PipelinesReady(pub bool);

#[derive(Resource, Debug, Default)]
struct LoadingProgress {
    /// The overlay font. Loading is not considered complete until it is
    /// loaded, so the text pipeline has had a chance to be queued and
    /// compiled alongside everything else.
    font: Handle<Font>,
    confirmation_frames: u32,
    elapsed_seconds: f32,
}

#[derive(Component)]
struct LoadingOverlay;

#[derive(Component)]
struct LoadingText;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PipelinesReady>();
        app.init_resource::<LoadingProgress>();
        app.init_resource::<PostLoadingState>();

        // Startup rather than OnEnter(Loading): the initial OnEnter runs
        // before PreStartup, where the embedded font is registered.
        app.add_systems(Startup, spawn_overlay.run_if(in_state(AppState::Loading)));
        // finish_loading writes NextState<AppState>, as does the pause toggle
        // in SimulationSet::Input; sharing the set pins their order.
        app.add_systems(
            Update,
            (pulse_text, finish_loading)
                .in_set(SimulationSet::Input)
                .run_if(in_state(AppState::Loading)),
        );
        app.add_systems(OnExit(AppState::Loading), despawn_overlay);
    }

    fn finish(&self, app: &mut App) {
        // The render sub-app is fully set up only after RenderPlugin's own
        // finish, hence registering here rather than in build. Headless
        // worlds have no render app and nothing to wait for.
        match app.get_sub_app_mut(RenderApp) {
            Some(render_app) => {
                render_app.add_systems(ExtractSchedule, mirror_pipelines_ready);
            }
            None => {
                app.insert_resource(PipelinesReady(true));
            }
        }
    }
}

/// Render world: copy pipeline-cache idleness into the main world.
fn mirror_pipelines_ready(mut main_world: ResMut<MainWorld>, pipeline_cache: Res<PipelineCache>) {
    if let Some(mut ready) = main_world.get_resource_mut::<PipelinesReady>() {
        ready.0 = pipeline_cache.waiting_pipelines().count() == 0;
    }
}

fn spawn_overlay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut progress: ResMut<LoadingProgress>,
) {
    let font_asset_path =
        AssetPath::parse("fonts/Saira-Light").with_source(AssetSourceId::from("embedded"));
    let font: Handle<Font> = asset_server.load(font_asset_path);
    progress.font = font.clone();

    commands
        .spawn((
            LoadingOverlay,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            // Opaque: also hides the scene popping in piecemeal as each
            // pipeline finishes compiling. Above every other UI node.
            BackgroundColor(Color::BLACK),
            GlobalZIndex(i32::MAX),
        ))
        .with_child((
            LoadingText,
            Text::new("Compiling shaders\u{2026}"),
            TextFont {
                font: font.into(),
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(Color::srgba(1.0, 1.0, 1.0, PULSE_MAX_ALPHA)),
        ));
}

fn pulse_text(time: Res<Time<Real>>, mut text: Query<&mut TextColor, With<LoadingText>>) {
    let phase = time.elapsed_secs() / PULSE_PERIOD_SECONDS * std::f32::consts::TAU;
    let alpha = PULSE_MIN_ALPHA + (PULSE_MAX_ALPHA - PULSE_MIN_ALPHA) * (0.5 + 0.5 * phase.sin());
    for mut color in &mut text {
        color.0.set_alpha(alpha);
    }
}

fn finish_loading(
    mut progress: ResMut<LoadingProgress>,
    pipelines_ready: Res<PipelinesReady>,
    asset_server: Res<AssetServer>,
    time: Res<Time<Real>>,
    post_loading_state: Res<PostLoadingState>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    progress.elapsed_seconds += time.delta_secs();
    let timed_out = progress.elapsed_seconds >= MAX_LOADING_SECONDS;

    // A failed font load counts as settled: the text just falls back to the
    // default font, and waiting on it would hang here.
    let font_settled = matches!(
        asset_server.get_load_state(&progress.font),
        Some(LoadState::Loaded | LoadState::Failed(_))
    );
    if !(timed_out || font_settled && pipelines_ready.0) {
        progress.confirmation_frames = 0;
        return;
    }

    progress.confirmation_frames += 1;
    if progress.confirmation_frames < CONFIRMATION_FRAMES && !timed_out {
        return;
    }
    if timed_out {
        warn!(
            "Loading exceeded {MAX_LOADING_SECONDS}s without pipelines settling; starting anyway"
        );
    }

    let target = post_loading_state.0.clone();
    info!("Render pipelines ready; entering {target:?}");
    next_state.set(target);
}

fn despawn_overlay(mut commands: Commands, overlays: Query<Entity, With<LoadingOverlay>>) {
    for entity in &overlays {
        commands.entity(entity).despawn();
    }
}
