//! The trail render pipeline: a `SpecializedRenderPipeline` shaped after
//! `bevy_gizmos_render`'s line pipeline (the in-tree precedent for
//! "instanced quads, one draw, `Transparent3d`"). Bevy's mesh-view layout is
//! reused for group 0, so camera matrices come for free; group 1 is our one
//! `TrailParams` uniform.

use bevy::asset::{AssetServer, load_embedded_asset};
use bevy::core_pipeline::core_3d::CORE_3D_DEPTH_FORMAT;
use bevy::mesh::VertexBufferLayout;
use bevy::pbr::{MeshPipeline, MeshPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::binding_types::uniform_buffer;
use bevy::render::render_resource::*;
use bevy::shader::Shader;

use super::buffers::TrailParams;
use crate::plugins::trails::segment::SEGMENT_STRIDE;

#[derive(Resource)]
pub struct TrailPipeline {
    pub mesh_pipeline: MeshPipeline,
    pub params_layout: BindGroupLayoutDescriptor,
    pub shader: Handle<Shader>,
}

pub fn init_trail_pipeline(
    mut commands: Commands,
    mesh_pipeline: Res<MeshPipeline>,
    asset_server: Res<AssetServer>,
) {
    let params_layout = BindGroupLayoutDescriptor::new(
        "trail params layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX_FRAGMENT,
            uniform_buffer::<TrailParams>(false),
        ),
    );

    commands.insert_resource(TrailPipeline {
        mesh_pipeline: mesh_pipeline.clone(),
        params_layout,
        shader: load_embedded_asset!(asset_server.as_ref(), "trail_ring.wgsl"),
    });
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct TrailPipelineKey {
    pub view_key: MeshPipelineKey,
}

impl SpecializedRenderPipeline for TrailPipeline {
    type Key = TrailPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let view_layout = self.mesh_pipeline.get_view_layout(key.view_key.into());

        RenderPipelineDescriptor {
            label: Some("trail ring pipeline".into()),
            layout: vec![view_layout.main_layout.clone(), self.params_layout.clone()],
            vertex: VertexState {
                shader: self.shader.clone(),
                entry_point: Some("vertex".into()),
                buffers: vec![segment_instance_layout()],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format: key.view_key.target_format(),
                    // True additive (src One, dst One): order-independent,
                    // which is what lets every trail share one draw. Not
                    // expressible through AlphaMode — Bevy keys Add to
                    // premultiplied blending on the Material path.
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            // Depth-test against opaque geometry (reverse-Z, matching Bevy's
            // transparent meshes) so bodies occlude trails; no depth writes,
            // as for any transparent content.
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::GreaterEqual),
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: key.view_key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            ..default()
        }
    }
}

/// The single instance-rate vertex buffer layout for `TrailSegment`.
/// Mirrors the `#[repr(C)]` struct in `segment.rs` and the `Instance`
/// struct in `trail_ring.wgsl`.
fn segment_instance_layout() -> VertexBufferLayout {
    VertexBufferLayout {
        array_stride: SEGMENT_STRIDE,
        step_mode: VertexStepMode::Instance,
        attributes: vec![
            // p0
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            // p1
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 12,
                shader_location: 1,
            },
            // birth0, birth1
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 24,
                shader_location: 2,
            },
            // radius
            VertexAttribute {
                format: VertexFormat::Float32,
                offset: 32,
                shader_location: 3,
            },
            // packed color
            VertexAttribute {
                format: VertexFormat::Uint32,
                offset: 36,
                shader_location: 4,
            },
            // previous segment direction (miter at the older end)
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 40,
                shader_location: 5,
            },
            // next segment direction (miter at the newer end)
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 52,
                shader_location: 6,
            },
            // smoothed speed (long-exposure energy)
            VertexAttribute {
                format: VertexFormat::Float32,
                offset: 64,
                shader_location: 7,
            },
        ],
    }
}
