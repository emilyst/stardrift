// Trail ring shader. Each instance is one 40-byte segment record; the
// vertex stage expands it into a camera-facing quad (six vertices from
// vertex_index, no vertex-rate buffer), evaluates fade and the age-keyed
// taper, and the fragment stage shades a soft capsule cross-profile: full
// energy inside the core, a Gaussian skirt outside it. Expired segments
// collapse to zero area against the effective-time uniform, so ring slots
// overwritten laps later never need a CPU touch.
//
// Joints (v1 policy): no cap extension — quads end exactly at their
// endpoints, so consecutive segments tile with zero overlap and additive
// blending shows no beading; the skirt blurs the hairline notch on the
// outside of sharp bends.

#import bevy_render::view::View

@group(0) @binding(0) var<uniform> view: View;

struct TrailParams {
    effective_time: f32,
    trail_length_seconds: f32,
    min_alpha: f32,
    max_alpha: f32,
    bloom_factor: f32,
    base_width: f32,
    body_size_multiplier: f32,
    min_width_ratio: f32,
    skirt_sigma: f32,
    fade_curve: u32,
    taper_curve: u32,
    flags: u32,
};

const TRAIL_FLAG_FADING: u32 = 1u;
const TRAIL_FLAG_TAPERING: u32 = 2u;
const TRAIL_FLAG_WIDTH_RELATIVE: u32 = 4u;
const PI: f32 = 3.14159265358979;

@group(1) @binding(0) var<uniform> params: TrailParams;

struct Instance {
    @location(0) p0: vec3<f32>,
    @location(1) p1: vec3<f32>,
    // x: birth of p0, y: birth of p1 (effective time).
    @location(2) births: vec2<f32>,
    @location(3) radius: f32,
    @location(4) color: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    // rgb: HDR base color; a: fade alpha at this vertex.
    @location(0) color: vec4<f32>,
    // Signed distance from the centerline, world units.
    @location(1) across: f32,
    // Core half-width at this vertex, world units (taper applied).
    @location(2) core_half: f32,
    // Gaussian sigma at this vertex, world units (taper applied).
    @location(3) sigma: f32,
};

// Tag layout matches pack_trail_color: luminance in bits 31..24, base RGB
// in bits 23..0.
fn unpack_color(tag: u32) -> vec4<f32> {
    return vec4<f32>(
        f32((tag >> 16u) & 0xFFu),
        f32((tag >> 8u) & 0xFFu),
        f32(tag & 0xFFu),
        f32((tag >> 24u) & 0xFFu),
    ) / 255.0;
}

// Same curve family and semantics as the legacy renderer's fade: alpha
// slides from max_alpha at age 0 to min_alpha at expiry.
fn fade_alpha(age: f32) -> f32 {
    if (params.flags & TRAIL_FLAG_FADING) == 0u {
        return params.max_alpha;
    }
    let max_age = params.trail_length_seconds;
    if age <= 0.0 || max_age <= 0.0 {
        return params.max_alpha;
    }
    let r = clamp(age / max_age, 0.0, 1.0);
    var c: f32;
    switch params.fade_curve {
        case 0u: { c = r; }
        case 1u: { c = r * r; }
        case 2u: { c = 3.0 * r * r - 2.0 * r * r * r; }
        default: { c = 0.5 * (1.0 - cos(r * PI)); }
    }
    return params.max_alpha - c * (params.max_alpha - params.min_alpha);
}

// Age-keyed width factor: 1.0 at record time, easing to min_width_ratio at
// expiry. A young trail is therefore a uniform ribbon; thinning appears
// only as the tail genuinely ages out.
fn taper_factor(age: f32) -> f32 {
    if (params.flags & TRAIL_FLAG_TAPERING) == 0u {
        return 1.0;
    }
    let max_age = params.trail_length_seconds;
    if age <= 0.0 || max_age <= 0.0 {
        return 1.0;
    }
    let r = clamp(age / max_age, 0.0, 1.0);
    let floor_w = params.min_width_ratio;
    switch params.taper_curve {
        case 0u: { return 1.0 - r * (1.0 - floor_w); }
        case 1u: {
            let t = 1.0 - r;
            return floor_w + (1.0 - floor_w) * t * t;
        }
        default: {
            let t = 1.0 - r;
            let eased = 3.0 * t * t - 2.0 * t * t * t;
            return floor_w + (1.0 - floor_w) * eased;
        }
    }
}

@vertex
fn vertex(@builtin(vertex_index) index: u32, instance: Instance) -> VertexOutput {
    // Quad corners as (t along the segment, s across it).
    var corners = array<vec2<f32>, 6>(
        vec2(0.0, -1.0),
        vec2(0.0, 1.0),
        vec2(1.0, 1.0),
        vec2(0.0, -1.0),
        vec2(1.0, 1.0),
        vec2(1.0, -1.0),
    );
    let corner = corners[index];

    let birth = mix(instance.births.x, instance.births.y, corner.x);
    let age = params.effective_time - birth;
    // Expired (or stale ring slots from a previous lap): collapse to zero
    // area so no fragments are rasterized. A non-positive trail length
    // disables expiry.
    let live = select(
        0.0,
        1.0,
        params.trail_length_seconds <= 0.0 || age <= params.trail_length_seconds,
    );

    let p = mix(instance.p0, instance.p1, corner.x);
    // The epsilon nudge keeps normalize() finite for zero-length segments
    // (a stationary body); the quad then has near-zero visible extent.
    let tangent = normalize(instance.p1 - instance.p0 + vec3(1e-30, 0.0, 0.0));
    let to_cam = normalize(view.world_position - p);
    var perp = cross(tangent, to_cam);
    let l2 = dot(perp, perp);
    if l2 < 1e-6 {
        perp = normalize(cross(tangent, vec3(0.0, 1.0, 0.0)) + vec3(1e-30, 0.0, 0.0));
    } else {
        perp = perp * inverseSqrt(l2);
    }

    var core_half: f32;
    if (params.flags & TRAIL_FLAG_WIDTH_RELATIVE) != 0u {
        core_half = 0.5 * instance.radius * params.body_size_multiplier;
    } else {
        core_half = 0.5 * params.base_width;
    }
    // Taper shrinks the whole profile — core and skirt together — so the
    // tail thins as one shape rather than dissolving into bare halo.
    let taper = taper_factor(age);
    core_half *= taper;
    let sigma = params.skirt_sigma * taper;

    // The quad extends 3 sigma past the core; beyond that the Gaussian is
    // visually zero.
    let outset = (core_half + 3.0 * sigma) * live;
    let world = p + perp * corner.y * outset;

    var out: VertexOutput;
    out.clip_position = view.clip_from_world * vec4(world, 1.0);

    let base = unpack_color(instance.color);
    let hdr = base.rgb * (params.bloom_factor * base.a + 1.0);
    out.color = vec4(hdr, fade_alpha(age) * live);
    out.across = corner.y * outset;
    out.core_half = core_half;
    out.sigma = sigma;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Capsule cross-profile: full energy inside the core, Gaussian falloff
    // outside it. True additive blending (src One, dst One), so emit
    // premultiplied energy; the alpha channel does not participate.
    let d = max(abs(in.across) - in.core_half, 0.0);
    let sigma = max(in.sigma, 1e-5);
    let skirt = exp(-0.5 * d * d / (sigma * sigma));
    let a = in.color.a * skirt;
    return vec4(in.color.rgb * a, a);
}
