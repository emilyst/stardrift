// Trail ribbon shader. Geometry arrives as one pair of coincident vertices
// per recorded point carrying a signed half-width; the vertex stage expands
// the pair into a camera-facing strip and evaluates fade, so the CPU only
// re-uploads when the point set changes, never per frame.

#import bevy_pbr::mesh_functions
#import bevy_pbr::mesh_view_bindings::view

struct TrailParams {
    effective_time: f32,
    trail_length_seconds: f32,
    min_alpha: f32,
    max_alpha: f32,
    bloom_factor: f32,
    fade_curve: u32,
    flags: u32,
    _pad: u32,
};

const TRAIL_FLAG_FADING: u32 = 1u;
const TRAIL_FLAG_ADDITIVE: u32 = 2u;
const PI: f32 = 3.14159265358979;

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> trail: TrailParams;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) tangent: vec3<f32>,
    @location(2) birth: f32,
    // Signed half-width; the sign selects the strip side.
    @location(3) offset: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

// Direct port of Trail::calculate_point_alpha. Evaluated per vertex (not per
// fragment) so the strip interpolates lerp(curve(age)) exactly like the old
// CPU vertex colors did; per-fragment evaluation would compute
// curve(lerp(age)) instead, which differs for every non-linear curve.
fn fade_alpha(age: f32) -> f32 {
    if (trail.flags & TRAIL_FLAG_FADING) == 0u {
        return trail.max_alpha;
    }
    let max_age = trail.trail_length_seconds;
    if age <= 0.0 || max_age <= 0.0 {
        return trail.max_alpha;
    }
    let r = clamp(age / max_age, 0.0, 1.0);
    var c: f32;
    switch trail.fade_curve {
        case 0u: { c = r; }
        case 1u: { c = r * r; }
        case 2u: { c = 3.0 * r * r - 2.0 * r * r * r; }
        default: { c = 0.5 * (1.0 - cos(r * PI)); }
    }
    return trail.max_alpha - c * (trail.max_alpha - trail.min_alpha);
}

// Tag layout: luminance in bits 31..24, base RGB in bits 23..0
// (see pack_trail_color).
fn unpack_tag_color(tag: u32) -> vec4<f32> {
    return vec4<f32>(
        f32((tag >> 16u) & 0xFFu),
        f32((tag >> 8u) & 0xFFu),
        f32(tag & 0xFFu),
        f32((tag >> 24u) & 0xFFu),
    ) / 255.0;
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    var world_pos =
        mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0)).xyz;
    let tangent_w = normalize(
        (world_from_local * vec4<f32>(vertex.tangent, 0.0)).xyz + vec3<f32>(1e-30, 0.0, 0.0),
    );

    // Camera-facing expansion: width perpendicular to both the trail
    // direction and the view ray. Falls back to a world-up perpendicular
    // when the view ray is parallel to the trail.
    let to_cam = normalize(view.world_position - world_pos);
    var perp = cross(tangent_w, to_cam);
    let l2 = dot(perp, perp);
    if l2 < 1e-6 {
        perp = normalize(cross(tangent_w, vec3<f32>(0.0, 1.0, 0.0)) + vec3<f32>(1e-30, 0.0, 0.0));
    } else {
        perp = perp * inverseSqrt(l2);
    }
    // Expired points are hidden here rather than removed CPU-side every
    // frame (removal would re-upload every trail mesh at frame rate; the
    // CPU trims them at a coarse cadence instead). Zeroing the offset
    // collapses them to degenerate zero-area triangles — no fragments —
    // and zeroing alpha below fades the one boundary segment into the last
    // live pair. A non-positive trail length disables expiry, mirroring
    // fade_alpha's max_age guard.
    let age = trail.effective_time - vertex.birth;
    let live = select(
        0.0,
        1.0,
        trail.trail_length_seconds <= 0.0 || age <= trail.trail_length_seconds,
    );

    // A zero tangent (stationary body) zeroes the cross product on both
    // paths above; the epsilon nudges keep normalize() finite and the
    // vertex collapses to zero width via a zero `offset` set CPU-side.
    world_pos += perp * vertex.offset * live;

    out.clip_position = view.clip_from_world * vec4<f32>(world_pos, 1.0);

    let base = unpack_tag_color(mesh_functions::get_tag(vertex.instance_index));
    let alpha = fade_alpha(age) * live;

    // Same luminance-scaled HDR boost the bodies use for bloom; the
    // luminance rides in the tag's high byte.
    let hdr = base.rgb * (trail.bloom_factor * base.a + 1.0);

    out.color = vec4<f32>(hdr, alpha);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Bevy keys both Add and Premultiplied to premultiplied blending
    // (src=One, dst=OneMinusSrcAlpha) and expects the shader to premultiply.
    // Additive output zeroes destination attenuation instead.
    let a = in.color.a;
    let rgb = in.color.rgb * a;
    let out_a = select(a, 0.0, (trail.flags & TRAIL_FLAG_ADDITIVE) != 0u);
    return vec4<f32>(rgb, out_a);
}
