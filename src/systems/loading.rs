use crate::components::*;
use crate::config::SimulationConfig;
use crate::resources::*;
use crate::states::AppState;
use crate::systems::ui;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::render::mesh::SphereKind;

pub fn setup_loading_screen(mut commands: Commands, mut loading_state: ResMut<LoadingState>) {
    loading_state.is_loading = true;
    loading_state.current_step = LoadingStep::InitializingConfig;

    // Create loading screen overlay
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            ZIndex(1000),
            LoadingScreen,
        ))
        .with_children(|parent| {
            // Loading container
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(20.0),
                    ..default()
                })
                .with_children(|parent| {
                    // Progress bar background
                    parent
                        .spawn((
                            Node {
                                width: Val::Px(400.0),
                                height: Val::Px(10.0),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 1.0)),
                        ))
                        .with_children(|parent| {
                            // Progress bar fill
                            parent.spawn((
                                Node {
                                    width: Val::Percent(0.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
                                LoadingProgressBar,
                            ));
                        });
                });
        });
}

pub fn start_loading_process(mut commands: Commands, mut loading_state: ResMut<LoadingState>) {
    // Keep in InitializingConfig step initially to show the loading screen
    loading_state.progress = 0.05;
    commands.insert_resource(LoadingTimer(Timer::from_seconds(0.5, TimerMode::Once)));
}

pub fn advance_loading_step(
    mut loading_state: ResMut<LoadingState>,
    mut timer: ResMut<LoadingTimer>,
    time: Res<Time>,
) {
    if loading_state.current_step == LoadingStep::InitializingConfig {
        timer.tick(time.delta());

        if timer.finished() {
            loading_state.current_step = LoadingStep::SpawningBodies;
            loading_state.progress = 0.1;
        } else {
            let progress = timer.elapsed_secs() / timer.duration().as_secs_f32();
            loading_state.progress = 0.05 + (progress * 0.05);
        }
    }
}

pub fn spawn_bodies_async(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<SharedRng>,
    body_count: Res<BodyCount>,
    config: Res<SimulationConfig>,
    mut loading_state: ResMut<LoadingState>,
    mut spawning_progress: Local<Option<BodySpawningProgress>>,
) {
    if spawning_progress.is_none() {
        *spawning_progress = Some(BodySpawningProgress {
            bodies_spawned: 0,
            total_bodies: **body_count,
            batch_size: ((**body_count).max(50) / 20).max(1),
        });
        loading_state.current_step = LoadingStep::SpawningBodies;
        loading_state.progress = 0.1;
    }

    if let Some(ref mut progress) = spawning_progress.as_mut() {
        let bodies_to_spawn =
            (progress.batch_size).min(progress.total_bodies - progress.bodies_spawned);

        for _ in 0..bodies_to_spawn {
            spawn_single_body(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut rng,
                &config,
                progress.total_bodies,
            );
            progress.bodies_spawned += 1;
        }

        let spawn_progress = progress.bodies_spawned as f32 / progress.total_bodies as f32;
        loading_state.progress = 0.1 + (spawn_progress * 0.7);

        if progress.bodies_spawned >= progress.total_bodies {
            loading_state.current_step = LoadingStep::BuildingOctree;
            loading_state.progress = 0.9;
            *spawning_progress = None;
        }
    }
}

fn spawn_single_body(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    rng: &mut ResMut<SharedRng>,
    config: &SimulationConfig,
    total_body_count: usize,
) {
    use crate::utils::{color, math};
    use rand::Rng;

    let body_distribution_sphere_radius = math::min_sphere_radius_for_surface_distribution(
        total_body_count, // Use total body count for proper distribution
        config.physics.body_distribution_sphere_radius_multiplier,
        config.physics.body_distribution_min_distance,
    );
    let position = math::random_unit_vector(&mut **rng) * body_distribution_sphere_radius;
    let transform = Transform::from_translation(position.as_vec3());
    let radius = rng.random_range(config.physics.min_body_radius..=config.physics.max_body_radius);
    let mesh = meshes.add(
        Sphere::new(radius as f32)
            .mesh()
            .kind(SphereKind::Ico {
                subdivisions: if cfg!(target_arch = "wasm32") { 1 } else { 4 },
            })
            .build(),
    );

    let min_temp = config.rendering.min_temperature;
    let max_temp = config.rendering.max_temperature;
    let min_radius = config.physics.min_body_radius;
    let max_radius = config.physics.max_body_radius;
    let temperature =
        min_temp + (max_temp - min_temp) * (max_radius - radius) / (max_radius - min_radius);
    let bloom_intensity = config.rendering.bloom_intensity;
    let saturation_intensity = config.rendering.saturation_intensity;
    let material = color::emissive_material_for_temp(
        materials,
        temperature,
        bloom_intensity,
        saturation_intensity,
    );

    commands.spawn((
        transform,
        Collider::sphere(radius),
        GravityScale(0.0),
        RigidBody::Dynamic,
        MeshMaterial3d(material.clone()),
        Mesh3d(mesh),
    ));
}

pub fn finalize_loading(
    mut loading_state: ResMut<LoadingState>,
    time: Res<Time>,
    mut finalize_timer: Local<Option<Timer>>,
) {
    if finalize_timer.is_none() {
        *finalize_timer = Some(Timer::from_seconds(0.3, TimerMode::Once));
        loading_state.progress = 0.9;
    }

    if let Some(ref mut timer) = finalize_timer.as_mut() {
        timer.tick(time.delta());

        if timer.finished() {
            loading_state.current_step = LoadingStep::SettingUpUI;
            loading_state.progress = 0.95;
            *finalize_timer = None;
        } else {
            // Gradually increase progress during finalization
            let progress = timer.elapsed_secs() / timer.duration().as_secs_f32();
            loading_state.progress = 0.9 + (progress * 0.05);
        }
    }
}

pub fn setup_ui_after_loading(
    commands: Commands,
    asset_server: Res<AssetServer>,
    config: Res<SimulationConfig>,
    mut loading_state: ResMut<LoadingState>,
) {
    ui::setup_ui(commands, asset_server, config);

    loading_state.current_step = LoadingStep::Complete;
    loading_state.progress = 1.0;
}

pub fn update_loading_progress(
    loading_state: Res<LoadingState>,
    mut progress_bar_query: Query<&mut Node, With<LoadingProgressBar>>,
) {
    if !loading_state.is_loading {
        return;
    }

    if let Ok(mut progress_bar) = progress_bar_query.single_mut() {
        progress_bar.width = Val::Percent(loading_state.progress * 100.0);
    }
}

pub fn complete_loading(
    mut commands: Commands,
    loading_screen_query: Query<Entity, With<LoadingScreen>>,
    mut loading_state: ResMut<LoadingState>,
    time: Res<Time>,
    mut completion_timer: Local<Option<Timer>>,
) {
    if loading_state.current_step == LoadingStep::Complete {
        if completion_timer.is_none() {
            *completion_timer = Some(Timer::from_seconds(0.5, TimerMode::Once));
        }

        if let Some(ref mut timer) = completion_timer.as_mut() {
            timer.tick(time.delta());

            if timer.finished() {
                for entity in &loading_screen_query {
                    commands.entity(entity).despawn();
                }

                loading_state.is_loading = false;
                *completion_timer = None;
            }
        }
    }
}

pub fn is_loading(loading_state: Res<LoadingState>) -> bool {
    loading_state.is_loading
}

pub fn should_spawn_bodies(loading_state: Res<LoadingState>) -> bool {
    loading_state.is_loading && matches!(loading_state.current_step, LoadingStep::SpawningBodies)
}

pub fn should_finalize_loading(loading_state: Res<LoadingState>) -> bool {
    loading_state.is_loading && matches!(loading_state.current_step, LoadingStep::BuildingOctree)
}

pub fn should_setup_ui(loading_state: Res<LoadingState>) -> bool {
    loading_state.is_loading && matches!(loading_state.current_step, LoadingStep::SettingUpUI)
}

pub fn loading_complete(loading_state: Res<LoadingState>) -> bool {
    matches!(loading_state.current_step, LoadingStep::Complete)
}

pub fn transition_to_running(
    loading_state: Res<LoadingState>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if !loading_state.is_loading {
        next_state.set(AppState::Running);
    }
}
