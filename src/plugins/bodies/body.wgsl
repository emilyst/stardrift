// One shared material for every body: per-body color rides in `MeshTag`, so
// all bodies batch into a single instanced draw (shared unit-sphere mesh +
// shared material + shared pipeline).
//
// The fragment reproduces what the per-body `StandardMaterial` rendered in
// this light-less scene: Bevy's ambient term (diffuse plus the view-dependent
// fresnel from ambient specular — the only per-pixel shading on the sphere)
// and the emissive ramp from `utils::color`'s old intensify_for_bloom.
// Ambient brightness and exposure come from the view bindings, so the output
// tracks `AmbientLight` / `Exposure` changes by construction. The camera has
// Hdr, so tonemapping and bloom run as post-processes on this linear output.

#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_functions,
    mesh_view_bindings::{lights, view},
}

struct BodyParams {
    bloom_intensity: f32,
    _pad: vec3<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: BodyParams;

// Inlined from bevy_pbr's pbr_lighting.wgsl (F_AB / EnvBRDFApprox, the
// Karis environment-BRDF polynomial) to avoid importing the whole lighting
// module. Stable since the DFG LUT feature is off in this build.
fn F_AB(perceptual_roughness: f32, NdotV: f32) -> vec2<f32> {
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = perceptual_roughness * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * NdotV)) * r.x + r.y;
    return vec2<f32>(-1.04, 1.04) * a004 + r.zw;
}

fn env_brdf_approx(f0: vec3<f32>, f_ab: vec2<f32>) -> vec3<f32> {
    return f0 * f_ab.x + f_ab.y;
}

fn unpack_tag_color(tag: u32) -> vec3<f32> {
    return vec3<f32>(
        f32((tag >> 20u) & 0x3FFu),
        f32((tag >> 10u) & 0x3FFu),
        f32(tag & 0x3FFu),
    ) / 1023.0;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = unpack_tag_color(mesh_functions::get_tag(in.instance_index));

    // Emissive ramp; emissive_exposure_weight was 0 on the old material, so
    // this term is deliberately NOT scaled by view.exposure.
    let emissive = base * (params.bloom_intensity * (base.r + base.g + base.b) / 3.0 + 1.0);

    // Ambient as StandardMaterial's pbr_ambient.wgsl computes it with default
    // material params: metallic 0 (diffuse_color = base), reflectance 0.5
    // (F0 = 0.04), perceptual_roughness 0.5, specular occlusion saturates to
    // 1. V assumes a perspective camera (true for PanOrbitCamera); an
    // orthographic camera would need pbr_functions::calculate_view.
    let N = normalize(in.world_normal);
    let V = normalize(view.world_position.xyz - in.world_position.xyz);
    let NdotV = max(dot(N, V), 0.0001);
    let ambient = (env_brdf_approx(base, F_AB(1.0, NdotV))
        + env_brdf_approx(vec3<f32>(0.04), F_AB(0.5, NdotV))) * lights.ambient_color.rgb;

    return vec4<f32>(view.exposure * ambient + emissive, 1.0);
}
