// One shared material for every body: per-body color rides in `MeshTag`, so
// all bodies batch into a single instanced draw (shared unit-quad mesh +
// shared material + shared pipeline).
//
// The quad is billboarded here in the vertex stage: corners expand along the
// camera's right/up axes, scaled by the body radius carried in
// `Transform::scale` (column length of `world_from_local`). The fragment
// draws a disc impostor: outside the inscribed circle it discards
// (`AlphaMode::Mask` routes the material into the binned, depth-writing
// AlphaMask3d phase, which keeps batching; Bevy injects no cutoff test for
// custom materials, so the discard must be explicit), inside it shades with a
// fake sphere normal so the result matches the ico-sphere look this replaced:
// Bevy's ambient term (diffuse plus the view-dependent fresnel from ambient
// specular) and the emissive ramp from `utils::color`'s old
// intensify_for_bloom. Ambient brightness and exposure come from the view
// bindings, so the output tracks `AmbientLight` / `Exposure` changes by
// construction. The camera has Hdr, so tonemapping and bloom run as
// post-processes on this linear output.

#import bevy_pbr::{
    forward_io::{Vertex, VertexOutput},
    mesh_functions,
    mesh_view_bindings::{lights, view},
    view_transformations::position_world_to_clip,
}

struct BodyParams {
    bloom_intensity: f32,
    _pad: vec3<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: BodyParams;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let center = world_from_local[3].xyz;
    // Uniform scale (the scale-carries-radius invariant), so one column's
    // length is the radius.
    let radius = length(world_from_local[0].xyz);

    // View space is +x right, +y up, +z toward the camera; these columns are
    // unit-length. The quad is Rectangle::new(2.0, 2.0), so vertex.position.xy
    // is directly the [-1, 1] disc coordinate. The mesh's own UV_0 has a
    // flipped v and NORMAL is +z local — both ignored, but the attributes
    // must stay on the mesh: the pipeline's shader-defs (and therefore
    // VertexOutput's uv/world_normal fields) exist only for attributes the
    // mesh carries. Expanding as +right*x +up*y preserves the mesh's CCW
    // winding for the back-face cull; flipping `up` would cull every body.
    let right = view.world_from_view[0].xyz;
    let up = view.world_from_view[1].xyz;
    let world = center + (right * vertex.position.x + up * vertex.position.y) * radius;

    out.world_position = vec4<f32>(world, 1.0);
    out.position = position_world_to_clip(world);
    out.world_normal = view.world_from_view[2].xyz;
    // Disc coordinate for the fragment stage, not the mesh's texture UV.
    out.uv = vertex.position.xy;
    // get_tag in the fragment reads mesh[instance_index]; forgetting this
    // passthrough leaves it reading garbage (random per-body colors).
    out.instance_index = vertex.instance_index;
    return out;
}

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
    let o = in.uv;
    let d2 = dot(o, o);
    if d2 > 1.0 {
        discard;
    }

    let base = unpack_tag_color(mesh_functions::get_tag(in.instance_index));

    // Emissive ramp; emissive_exposure_weight was 0 on the old material, so
    // this term is deliberately NOT scaled by view.exposure.
    let emissive = base * (params.bloom_intensity * (base.r + base.g + base.b) / 3.0 + 1.0);

    // Impostor sphere normal: view-space (o, sqrt(1-d2)) mapped to world
    // through the same camera basis the vertex stage billboarded with.
    // Unit-length by construction (orthonormal basis, o.o + (1-d2) = 1), so
    // no normalize.
    let N = view.world_from_view[0].xyz * o.x
        + view.world_from_view[1].xyz * o.y
        + view.world_from_view[2].xyz * sqrt(1.0 - d2);

    // Ambient as StandardMaterial's pbr_ambient.wgsl computes it with default
    // material params: metallic 0 (diffuse_color = base), reflectance 0.5
    // (F0 = 0.04), perceptual_roughness 0.5, specular occlusion saturates to
    // 1. V assumes a perspective camera (true for PanOrbitCamera); an
    // orthographic camera would need pbr_functions::calculate_view.
    let V = normalize(view.world_position.xyz - in.world_position.xyz);
    let NdotV = max(dot(N, V), 0.0001);
    let ambient = (env_brdf_approx(base, F_AB(1.0, NdotV))
        + env_brdf_approx(vec3<f32>(0.04), F_AB(0.5, NdotV))) * lights.ambient_color.rgb;

    return vec4<f32>(view.exposure * ambient + emissive, 1.0);
}
