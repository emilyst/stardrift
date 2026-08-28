// Trail ring shader. Each instance is one 64-byte segment record; the
// vertex stage expands it into a camera-facing quad (six vertices from
// vertex_index, no vertex-rate buffer), evaluates fade and the age-keyed
// taper, and the fragment stage shades a soft lateral profile: full energy
// inside the core, a Gaussian skirt outside it. Expired segments collapse
// to zero area against the effective-time uniform, so ring slots
// overwritten laps later never need a CPU touch.
//
// Joints are MITERED: each end edge is rotated onto the bisector of this
// segment's direction and its neighbor's (t_prev / t_next in the instance
// data), scaled so the perpendicular width is preserved. Adjacent quads
// therefore share their edges exactly — no gaps at bends, no overlap, and
// no additive seams, at any width-to-speed ratio. (Both cap-extension and
// crossfade joints were tried first and produce visible artifacts when
// segment length drops below ribbon width; see the plan doc.) The newest
// segment's t_next is provisional (its successor doesn't exist yet) and is
// corrected one tick later by the rewrite batch.
//
// Anti-aliasing: the lateral outset is clamped to a minimum of ~2 pixels
// (computed from the projection) and the fragment's sigma to ~0.75 px via
// fwidth, so edges stay soft at any zoom instead of quantizing when the
// world-space skirt drops below a pixel.

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
    @location(5) t_prev: vec3<f32>,
    @location(6) t_next: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    // rgb: HDR base color; a: fade alpha at this vertex.
    @location(0) color: vec4<f32>,
    // Perpendicular distance from the segment axis, world units (exact
    // even on mitered edges: the miter scale preserves the projection onto
    // the segment's own perpendicular).
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

// Camera-facing perpendicular for a tangent, with the degenerate-view
// fallback the legacy renderer used.
fn facing_perp(tangent: vec3<f32>, to_cam: vec3<f32>) -> vec3<f32> {
    var perp = cross(tangent, to_cam);
    let l2 = dot(perp, perp);
    if l2 < 1e-6 {
        return normalize(cross(tangent, vec3(0.0, 1.0, 0.0)) + vec3(1e-30, 0.0, 0.0));
    }
    return perp * inverseSqrt(l2);
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

    let seg = instance.p1 - instance.p0;
    let p = mix(instance.p0, instance.p1, corner.x);
    // The epsilon nudges keep normalize() finite for zero-length segments
    // and zero neighbor tangents; those quads have zero area anyway.
    let t_this = normalize(seg + vec3(1e-30, 0.0, 0.0));
    let t_raw = select(instance.t_prev, instance.t_next, corner.x > 0.5);
    let t_nbr = normalize(t_raw + vec3(1e-30, 0.0, 0.0));

    let to_cam = normalize(view.world_position - p);
    let perp = facing_perp(t_this, to_cam);
    var perp_nbr = facing_perp(t_nbr, to_cam);
    // Keep the two perpendiculars on the same side across sharp reversals,
    // or the bisector collapses through the segment axis.
    if dot(perp, perp_nbr) < 0.0 {
        perp_nbr = -perp_nbr;
    }

    // Miter: the end edge lies along the bisector of the two
    // perpendiculars, scaled so this segment's perpendicular half-width is
    // preserved. The clamp caps the miter at 2x for extreme angles
    // (degrading toward a bevel rather than a spike).
    var miter = perp + perp_nbr;
    let ml2 = dot(miter, miter);
    if ml2 < 1e-6 {
        miter = perp;
    } else {
        miter = miter * inverseSqrt(ml2);
    }
    let miter_scale = 1.0 / max(abs(dot(miter, perp)), 0.5);

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

    // World units per screen pixel (vertically) at this vertex's depth:
    // keeps the quad wide enough that the fragment stage always has at
    // least ~2 px of skirt to soften, at any zoom.
    let clip_center = view.clip_from_world * vec4(p, 1.0);
    let world_per_px =
        2.0 * max(clip_center.w, 1e-6) / (view.viewport.w * view.clip_from_view[1][1]);
    let margin = max(3.0 * sigma, 2.0 * world_per_px);
    let outset = (core_half + margin) * live;

    let world = p + miter * (corner.y * outset * miter_scale);

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
    // Lateral soft profile: full energy inside the core, Gaussian skirt
    // outside. Resolution-aware: never let the falloff drop below ~0.75 px,
    // so edges do not alias when the world-space skirt is sub-pixel.
    let px = fwidth(in.across);
    let sigma = max(in.sigma, 0.75 * px);
    let d = max(abs(in.across) - in.core_half, 0.0);
    let skirt = exp(-0.5 * d * d / (sigma * sigma));

    // True additive blending (src One, dst One): emit premultiplied energy;
    // the alpha channel does not participate.
    let a = in.color.a * skirt;
    return vec4(in.color.rgb * a, a);
}
