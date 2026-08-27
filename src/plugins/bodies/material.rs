//! GPU body material: one shared `BodyMaterial` renders every body.
//!
//! Per-body color travels in `MeshTag` (packed linear RGB), not in the
//! material, so every body shares one material handle and one mesh handle —
//! the two keys Bevy's automatic instancing batches on. The material itself
//! is never mutated at runtime, so unlike `TrailMaterial` there is no
//! per-frame bind-group rebuild and no sync system.

use crate::prelude::*;
use bevy::pbr::Material;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

pub const BODY_SHADER_PATH: &str = "embedded://stardrift/plugins/bodies/body.wgsl";

#[derive(ShaderType, Clone, Debug)]
pub struct BodyParams {
    /// Emissive ramp factor: the shader computes
    /// `base * (bloom_intensity * mean(base) + 1)`, the same ramp
    /// `utils::color::intensify_for_bloom` used to bake into each body's
    /// `StandardMaterial::emissive`.
    pub bloom_intensity: f32,
    /// WebGL2 requires 16-byte-aligned uniform structs; a bare f32 is not
    /// portable (cf. `TrailParams`, padded the same way).
    pub _pad: Vec3,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct BodyMaterial {
    #[uniform(0)]
    pub params: BodyParams,
}

impl Material for BodyMaterial {
    // Default vertex shader and AlphaMode::Opaque: same binned opaque phase,
    // depth writes, and batching behavior as the StandardMaterial it replaces.
    fn fragment_shader() -> ShaderRef {
        BODY_SHADER_PATH.into()
    }

    // The scene has no lights and the camera no prepass, so both pipelines
    // would only compile dead specializations. If a depth/normal prepass is
    // ever added (SSAO, TAA, and motion blur all force one), flip these or
    // bodies will be missing from it.
    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

/// The one material shared by every body.
#[derive(Resource, Deref)]
pub struct BodyMaterialHandle(pub Handle<BodyMaterial>);

impl FromWorld for BodyMaterialHandle {
    fn from_world(world: &mut World) -> Self {
        let bloom_intensity = world
            .resource::<SimulationConfig>()
            .rendering
            .bloom_intensity;
        let mut materials = world.resource_mut::<Assets<BodyMaterial>>();
        Self(materials.add(BodyMaterial {
            params: BodyParams {
                bloom_intensity,
                _pad: Vec3::ZERO,
            },
        }))
    }
}

/// Pack a body color for `MeshTag`: linear RGB at 10:10:10 (top 2 bits
/// spare). 10 bits per channel keeps the quantization step small enough that
/// the bloom-clip contour around each body sits within a pixel of where the
/// f32 color put it; at 8 bits the contour shift was measurable in seeded
/// screenshot diffs (though not visible).
///
/// Deliberately different from `pack_trail_color`, which reproduces the old
/// CPU tessellator's srgba-product quirks for trail parity. Do not unify.
pub fn pack_body_color(color: Color) -> u32 {
    let linear = color.to_linear();
    let quantize = |c: f32| (c.clamp(0.0, 1.0) * 1023.0).round() as u32;
    (quantize(linear.red) << 20) | (quantize(linear.green) << 10) | quantize(linear.blue)
}
