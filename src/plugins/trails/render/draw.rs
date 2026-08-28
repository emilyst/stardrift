//! The queue system and draw commands for the trail ring: one transient
//! `Transparent3d` phase item per view, one instanced draw over the ring's
//! live range (two across a wrap).

use bevy::core_pipeline::core_3d::{Transparent3d, TransparentSortingInfo3d};
use bevy::ecs::query::ROQueryItem;
use bevy::ecs::system::SystemParamItem;
use bevy::ecs::system::lifetimeless::SRes;
use bevy::pbr::{SetMeshViewBindGroup, ViewKeyCache};
use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;
use bevy::render::render_phase::{
    DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand, RenderCommandResult,
    SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
};
use bevy::render::render_resource::{PipelineCache, SpecializedRenderPipelines};
use bevy::render::sync_world::MainEntity;
use bevy::render::view::ExtractedView;

use super::buffers::{ExtractedTrailFrame, TrailParamsBindGroup, TrailRing};
use super::pipeline::{TrailPipeline, TrailPipelineKey};

/// Sorted render phases require a main-world entity to associate the phase
/// item with; trails have no per-trail render entities, so one named entity
/// stands in for the whole draw (the pattern `bevy_gizmos_render` uses).
#[derive(Resource, Clone, ExtractResource)]
pub struct TrailRendererEntity(pub MainEntity);

impl FromWorld for TrailRendererEntity {
    fn from_world(world: &mut World) -> Self {
        Self(MainEntity::from(
            world.spawn(Name::new("TrailRenderer")).id(),
        ))
    }
}

pub type DrawTrailsCommands = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetTrailParamsBindGroup<1>,
    DrawTrailRing,
);

pub struct SetTrailParamsBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetTrailParamsBindGroup<I> {
    type Param = SRes<TrailParamsBindGroup>;
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(bind_group) = &bind_group.into_inner().0 else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawTrailRing;

impl<P: PhaseItem> RenderCommand<P> for DrawTrailRing {
    type Param = SRes<TrailRing>;
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        ring: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let ring = ring.into_inner();
        let Some(buffer) = &ring.buffer else {
            return RenderCommandResult::Skip;
        };
        if ring.live == 0 {
            return RenderCommandResult::Success;
        }

        pass.set_vertex_buffer(0, buffer.slice(..));

        // Oldest live slot; the live range may wrap the ring's end.
        let start = (ring.head + ring.capacity - ring.live) % ring.capacity;
        let first_len = (ring.capacity - start).min(ring.live);
        pass.draw(0..6, start..start + first_len);
        if first_len < ring.live {
            pass.draw(0..6, 0..ring.live - first_len);
        }

        RenderCommandResult::Success
    }
}

// Render queue systems legitimately take many params; the gizmo queue this
// mirrors has the same shape.
#[allow(clippy::too_many_arguments)]
pub fn queue_trails(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    pipeline: Res<TrailPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TrailPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    frame: Res<ExtractedTrailFrame>,
    ring: Res<TrailRing>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<&ExtractedView>,
    view_key_cache: Res<ViewKeyCache>,
    renderer_entity: Res<TrailRendererEntity>,
) {
    if !frame.visible {
        return;
    }
    // Queue runs before this frame's ring prepare; count the extracted batch
    // so the very first segments still draw this frame.
    if ring.live == 0 && frame.segments.is_empty() {
        return;
    }

    let draw_function = draw_functions
        .read()
        .get_id::<DrawTrailsCommands>()
        .unwrap();

    for view in &views {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };
        let Some(&view_key) = view_key_cache.get(&view.retained_view_entity) else {
            continue;
        };

        let pipeline_id =
            pipelines.specialize(&pipeline_cache, &pipeline, TrailPipelineKey { view_key });

        // Additive blending is order-independent, so the sort position within
        // the transparent phase is irrelevant for correctness.
        transparent_phase.add_transient(Transparent3d {
            sorting_info: TransparentSortingInfo3d::AlwaysOnTop,
            entity: (Entity::PLACEHOLDER, renderer_entity.0),
            draw_function,
            pipeline: pipeline_id,
            distance: 0.,
            batch_range: 0..1,
            extra_index: PhaseItemExtraIndex::None,
            indexed: false,
        });
    }
}
