//! GPU trail material: one shared `TrailMaterial` renders every trail.
//!
//! Per-trail color travels in `MeshTag` (packed RGB), not in the material —
//! Bevy recreates a material's bind group whenever its asset is mutated, so
//! per-trail materials carrying a per-frame time uniform would mean one bind
//! group rebuild per trail per frame. With a single shared material the
//! per-frame `effective_time` write costs one rebuild total.

use crate::config::{FadeCurve, TrailConfig};
use crate::prelude::*;
use bevy::mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey};
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError, VertexFormat,
};
use bevy::shader::ShaderRef;

pub const TRAIL_SHADER_PATH: &str = "embedded://stardrift/plugins/trails/trail.wgsl";

// Unique high ids per MeshVertexAttribute docs (built-ins sit near zero).
pub const ATTRIBUTE_TRAIL_TANGENT: MeshVertexAttribute = MeshVertexAttribute::new(
    "TrailTangent",
    0x5452_4149_4C00_0001,
    VertexFormat::Float32x3,
);
pub const ATTRIBUTE_TRAIL_BIRTH: MeshVertexAttribute =
    MeshVertexAttribute::new("TrailBirth", 0x5452_4149_4C00_0002, VertexFormat::Float32);
/// Signed half-width: negative for the left strip vertex, positive for the
/// right. The sign replaces any `vertex_index` parity trick, which would be
/// unsound under Bevy's mesh slab allocator (vertex_index is slab-absolute).
/// Taper and per-point body radius are baked in CPU-side at rebuild time.
pub const ATTRIBUTE_TRAIL_OFFSET: MeshVertexAttribute =
    MeshVertexAttribute::new("TrailOffset", 0x5452_4149_4C00_0003, VertexFormat::Float32);

pub const TRAIL_FLAG_FADING: u32 = 1;
pub const TRAIL_FLAG_ADDITIVE: u32 = 2;

#[derive(ShaderType, Clone, Debug)]
pub struct TrailParams {
    /// Pause-adjusted now; ages are `effective_time - birth`. Written every
    /// frame by `sync_trail_material`. (`globals.time` is unusable here: it
    /// wraps hourly and ignores pause.)
    pub effective_time: f32,
    pub trail_length_seconds: f32,
    pub min_alpha: f32,
    pub max_alpha: f32,
    pub bloom_factor: f32,
    /// 0 Linear, 1 Exponential, 2 SmoothStep, 3 EaseInOut
    pub fade_curve: u32,
    pub flags: u32,
    pub _pad: u32,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct TrailMaterial {
    #[uniform(0)]
    pub params: TrailParams,
    /// Drives `alpha_mode()`; the fragment shader reads the matching flag bit
    /// because `Add` and `Premultiplied` share one pipeline key in Bevy and
    /// the output convention differs in-shader.
    pub additive: bool,
}

impl Material for TrailMaterial {
    fn vertex_shader() -> ShaderRef {
        TRAIL_SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        TRAIL_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        if self.additive {
            AlphaMode::Add
        } else {
            AlphaMode::Premultiplied
        }
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            ATTRIBUTE_TRAIL_TANGENT.at_shader_location(1),
            ATTRIBUTE_TRAIL_BIRTH.at_shader_location(2),
            ATTRIBUTE_TRAIL_OFFSET.at_shader_location(3),
        ])?];
        // Ribbons are double-sided: the winding flips with view direction
        // under camera-facing expansion, so backface culling would blank them
        // at some angles.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

impl TrailMaterial {
    pub fn from_config(config: &TrailConfig) -> Self {
        let mut flags = 0;
        if config.enable_fading {
            flags |= TRAIL_FLAG_FADING;
        }
        if config.use_additive_blending {
            flags |= TRAIL_FLAG_ADDITIVE;
        }

        Self {
            params: TrailParams {
                effective_time: 0.0,
                trail_length_seconds: config.trail_length_seconds,
                min_alpha: config.min_alpha,
                max_alpha: config.max_alpha,
                bloom_factor: config.bloom_factor,
                fade_curve: match config.fade_curve {
                    FadeCurve::Linear => 0,
                    FadeCurve::Exponential => 1,
                    FadeCurve::SmoothStep => 2,
                    FadeCurve::EaseInOut => 3,
                },
                flags,
                _pad: 0,
            },
            additive: config.use_additive_blending,
        }
    }
}

/// The one material shared by every trail renderer.
#[derive(Resource, Deref)]
pub struct TrailMaterialHandle(pub Handle<TrailMaterial>);

impl FromWorld for TrailMaterialHandle {
    fn from_world(world: &mut World) -> Self {
        let config = world.resource::<SimulationConfig>().trails.clone();
        let mut materials = world.resource_mut::<Assets<TrailMaterial>>();
        Self(materials.add(TrailMaterial::from_config(&config)))
    }
}

/// Pack a trail color for `MeshTag`: luminance in bits 31..24, base RGB in
/// bits 23..0. The shader reconstructs `base * (bloom_factor * lum + 1)`.
///
/// Parity quirks of the old pipeline, kept deliberately: the CPU tessellator
/// fed `color.to_srgba()` components into linear vertex-color slots, and the
/// unlit `StandardMaterial` then multiplied its linear `base_color` into
/// them. Both the srgba-encoded luminance and the linear×srgba product are
/// reproduced here so the rendered colors match.
pub fn pack_trail_color(color: Color) -> u32 {
    let linear = color.to_linear();
    let srgba = color.to_srgba();
    let luminance = (srgba.red + srgba.green + srgba.blue) / 3.0;
    let quantize = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u32;
    (quantize(luminance) << 24)
        | (quantize(linear.red * srgba.red) << 16)
        | (quantize(linear.green * srgba.green) << 8)
        | quantize(linear.blue * srgba.blue)
}
