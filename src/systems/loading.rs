use crate::config::SimulationConfig;
use crate::resources::*;
use crate::states::AppState;
use crate::states::LoadingState;
use crate::systems::ui;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::render::mesh::SphereKind;

#[derive(Component)]
pub struct LoadingScreen;

#[derive(Component)]
pub struct LoadingProgressBar;

pub fn setup_loading_screen(mut commands: Commands, mut loading_progress: ResMut<LoadingProgress>) {
    loading_progress.progress = 0.0;
    loading_progress.current_message = "Initializing...".to_string();
    info!("Loading: {}", loading_progress.current_message);

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

pub fn start_loading_process(
    mut commands: Commands,
    mut loading_progress: ResMut<LoadingProgress>,
) {
    loading_progress.progress = 0.05;
    loading_progress.current_message = "Starting loading process...".to_string();
    info!("Loading: {}", loading_progress.current_message);
    commands.insert_resource(LoadingTimer(Timer::from_seconds(0.5, TimerMode::Once)));
}

pub fn advance_loading_step(
    mut loading_progress: ResMut<LoadingProgress>,
    mut timer: ResMut<LoadingTimer>,
    mut next_state: ResMut<NextState<LoadingState>>,
    time: Res<Time>,
) {
    timer.tick(time.delta());

    if timer.finished() {
        next_state.set(LoadingState::SpawningBodies);
        loading_progress.progress = 0.1;
        loading_progress.current_message = "Spawning celestial bodies...".to_string();
        info!("Loading: {}", loading_progress.current_message);
    } else {
        let progress = timer.elapsed_secs() / timer.duration().as_secs_f32();
        loading_progress.progress = 0.05 + (progress * 0.05);
    }
}

pub fn spawn_bodies_async(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<SharedRng>,
    body_count: Res<BodyCount>,
    config: Res<SimulationConfig>,
    mut loading_progress: ResMut<LoadingProgress>,
    mut next_state: ResMut<NextState<LoadingState>>,
    mut spawning_progress: Local<Option<BodySpawningProgress>>,
) {
    if spawning_progress.is_none() {
        *spawning_progress = Some(BodySpawningProgress {
            bodies_spawned: 0,
            total_bodies: **body_count,
            batch_size: ((**body_count).max(50) / 20).max(1),
        });
        loading_progress.progress = 0.1;
        loading_progress.current_message = "Spawning celestial bodies...".to_string();
        info!("Loading: {}", loading_progress.current_message);
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
        loading_progress.progress = 0.1 + (spawn_progress * 0.7);
        loading_progress.current_message = format!(
            "Spawning bodies: {}/{}",
            progress.bodies_spawned, progress.total_bodies
        );

        if progress.bodies_spawned >= progress.total_bodies {
            next_state.set(LoadingState::BuildingOctree);
            loading_progress.progress = 0.9;
            loading_progress.current_message = "Building octree...".to_string();
            info!("Loading: {}", loading_progress.current_message);
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
    mut loading_progress: ResMut<LoadingProgress>,
    mut next_state: ResMut<NextState<LoadingState>>,
    time: Res<Time>,
    mut finalize_timer: Local<Option<Timer>>,
) {
    if finalize_timer.is_none() {
        *finalize_timer = Some(Timer::from_seconds(0.3, TimerMode::Once));
        loading_progress.progress = 0.9;
        loading_progress.current_message = "Finalizing octree...".to_string();
        info!("Loading: {}", loading_progress.current_message);
    }

    if let Some(ref mut timer) = finalize_timer.as_mut() {
        timer.tick(time.delta());

        if timer.finished() {
            next_state.set(LoadingState::SettingUpUI);
            loading_progress.progress = 0.95;
            loading_progress.current_message = "Setting up UI...".to_string();
            info!("Loading: {}", loading_progress.current_message);
            *finalize_timer = None;
        } else {
            // Gradually increase progress during finalization
            let progress = timer.elapsed_secs() / timer.duration().as_secs_f32();
            loading_progress.progress = 0.9 + (progress * 0.05);
        }
    }
}

pub fn setup_ui_after_loading(
    commands: Commands,
    asset_server: Res<AssetServer>,
    mut loading_progress: ResMut<LoadingProgress>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    ui::setup_ui(commands, asset_server);

    loading_progress.progress = 1.0;
    loading_progress.current_message = "Loading complete!".to_string();
    info!("Loading: {}", loading_progress.current_message);

    next_app_state.set(AppState::Running);
}

pub fn update_loading_progress(
    loading_progress: Res<LoadingProgress>,
    mut progress_bar_query: Query<&mut Node, With<LoadingProgressBar>>,
) {
    if let Ok(mut progress_bar) = progress_bar_query.single_mut() {
        progress_bar.width = Val::Percent(loading_progress.progress * 100.0);
    }
}

pub fn complete_loading(
    mut commands: Commands,
    loading_screen_query: Query<Entity, With<LoadingScreen>>,
    time: Res<Time>,
    mut completion_timer: Local<Option<Timer>>,
) {
    if completion_timer.is_none() {
        *completion_timer = Some(Timer::from_seconds(0.5, TimerMode::Once));
    }

    if let Some(ref mut timer) = completion_timer.as_mut() {
        timer.tick(time.delta());

        if timer.finished() {
            for entity in &loading_screen_query {
                commands.entity(entity).despawn();
            }
            *completion_timer = None;
        }
    }
}
