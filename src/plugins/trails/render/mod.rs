//! The instanced trail renderer: all trails in one draw call, fed from a
//! persistent GPU segment ring. Native only for now; the WASM build keeps
//! the legacy per-trail ribbon renderer until a deliberate port (see
//! `docs/plans/trail-instanced-renderer-plan.md`).

mod buffers;
mod draw;
mod pipeline;

use bevy::asset::embedded_asset;
use bevy::core_pipeline::core_3d::Transparent3d;
use bevy::pbr::MeshPipelineSystems;
use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResourcePlugin;
use bevy::render::render_phase::AddRenderCommand;
use bevy::render::render_resource::SpecializedRenderPipelines;
use bevy::render::{GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems};

use buffers::{
    ExtractedTrailFrame, TrailParamsBindGroup, TrailParamsUniform, TrailRing, extract_trails,
    prepare_trail_params, prepare_trail_params_bind_group, prepare_trail_ring,
};
use draw::{DrawTrailsCommands, TrailRendererEntity, queue_trails};
use pipeline::{TrailPipeline, init_trail_pipeline};

pub struct TrailRenderPlugin;

impl Plugin for TrailRenderPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "trail_ring.wgsl");

        app.init_resource::<TrailRendererEntity>()
            .add_plugins(ExtractResourcePlugin::<TrailRendererEntity>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<ExtractedTrailFrame>()
            .init_resource::<TrailRing>()
            .init_resource::<TrailParamsUniform>()
            .init_resource::<TrailParamsBindGroup>()
            .add_render_command::<Transparent3d, DrawTrailsCommands>()
            .init_gpu_resource::<SpecializedRenderPipelines<TrailPipeline>>()
            .add_systems(
                RenderStartup,
                init_trail_pipeline.after(MeshPipelineSystems),
            )
            .add_systems(bevy::render::ExtractSchedule, extract_trails)
            .add_systems(
                Render,
                (
                    queue_trails.in_set(RenderSystems::Queue),
                    (prepare_trail_ring, prepare_trail_params)
                        .in_set(RenderSystems::PrepareResources),
                    prepare_trail_params_bind_group.in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }
}
