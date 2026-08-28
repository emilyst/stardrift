// Trail ring shader. Each instance is one 64-byte segment record; the
// vertex stage projects it to SCREEN SPACE and expands it there into a
// mitered quad (six vertices from vertex_index, no vertex-rate buffer),
// evaluates fade and the age-keyed taper, and the fragment stage shades a
// soft lateral profile in pixels: full energy inside the core, a Gaussian
// skirt outside it. Expired segments collapse to zero area against the
// effective-time uniform, so ring slots overwritten laps later never need
// a CPU touch.
//
// Why screen space: world-space camera-facing quads twist around a path
// that curves in depth, so adjacent quads lie in different planes and
// alternately overlap and gap — visible as regular banding at segment
// pitch under additive blending (measured; see the plan doc). In 2D there
// is no out-of-plane direction: adjacent quads are coplanar by definition,
// and because both quads at a joint project the exact same three points
// (p_prev/p0/p1/p_next chain), their mitered shared edge agrees by
// arithmetic. This is the same transformation bevy_gizmos_render ships in
// lines.wgsl, including the near-plane clipping guard.

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
    exposure_reference_speed: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

const TRAIL_FLAG_FADING: u32 = 1u;
const TRAIL_FLAG_TAPERING: u32 = 2u;
const TRAIL_FLAG_WIDTH_RELATIVE: u32 = 4u;
const PI: f32 = 3.14159265358979;
const EPSILON: f32 = 4.88e-04;

@group(1) @binding(0) var<uniform> params: TrailParams;

struct Instance {
    @location(0) p0: vec3<f32>,
    @location(1) p1: vec3<f32>,
    // x: birth of p0, y: birth of p1 (effective time).
    @location(2) births: vec2<f32>,
    @location(3) radius: f32,
    @location(4) color: u32,
    @location(5) p_prev: vec3<f32>,
    @location(6) p_next: vec3<f32>,
    // Smoothed record-time speed (per-trail EMA), for the exposure law.
    @location(7) speed: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    // rgb: HDR base color; a: fade alpha at this vertex.
    @location(0) color: vec4<f32>,
    // Signed lateral distance from the segment's screen axis, pixels.
    @location(1) across: f32,
    // Core half-width at this vertex, pixels (taper applied).
    @location(2) core_half: f32,
    // Gaussian sigma at this vertex, pixels (taper applied).
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

// Verbatim from bevy_gizmos_render lines.wgsl: move `a` to the near plane
// when it sits behind it and `b` is in front, so the perspective divide
// stays valid.
fn clip_near_plane(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    if a.z > a.w && b.z <= b.w {
        let distance_a = a.z - a.w;
        let distance_b = b.z - b.w;
        let t = distance_a / (distance_a - distance_b) + EPSILON;
        return mix(a, b, t);
    }
    return a;
}

fn screen_of(clip: vec4<f32>, resolution: vec2<f32>) -> vec2<f32> {
    return resolution * (0.5 * clip.xy / clip.w + 0.5);
}

// Screen-space direction from `from` toward `to`, falling back when the
// projection is degenerate (coincident on screen, or behind the camera).
fn screen_dir(tail: vec2<f32>, tip: vec2<f32>, fallback: vec2<f32>) -> vec2<f32> {
    let d = tip - tail;
    let l2 = dot(d, d);
    if l2 < 1e-6 {
        return fallback;
    }
    return d * inverseSqrt(l2);
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

    let resolution = view.viewport.zw;

    var clip0 = view.clip_from_world * vec4(instance.p0, 1.0);
    var clip1 = view.clip_from_world * vec4(instance.p1, 1.0);
    let clip0n = clip_near_plane(clip0, clip1);
    let clip1n = clip_near_plane(clip1, clip0);
    clip0 = clip0n;
    clip1 = clip1n;

    let s0 = screen_of(clip0, resolution);
    let s1 = screen_of(clip1, resolution);
    let dir = screen_dir(s0, s1, vec2(1.0, 0.0));
    let n_this = vec2(-dir.y, dir.x);

    // Neighbor chords, projected the same way. The neighbor of a joint
    // endpoint is the SAME world point in the adjacent instance, so both
    // quads compute identical edge geometry there.
    var clip_prev = view.clip_from_world * vec4(instance.p_prev, 1.0);
    clip_prev = clip_near_plane(clip_prev, clip0);
    var clip_next = view.clip_from_world * vec4(instance.p_next, 1.0);
    clip_next = clip_near_plane(clip_next, clip1);
    let d_prev = screen_dir(screen_of(clip_prev, resolution), s0, dir);
    let d_next = screen_dir(s1, screen_of(clip_next, resolution), dir);

    let end_is_new = corner.x > 0.5;
    let s_end = select(s0, s1, end_is_new);
    let clip_end = select(clip0, clip1, end_is_new);
    let d_nbr = select(d_prev, d_next, end_is_new);

    // Miter: end edge along the 2D bisector of this segment's normal and
    // the neighbor chord's, scaled to preserve this segment's
    // perpendicular width; clamped 2x (degrading toward a bevel at extreme
    // angles). Sign-matching keeps the bisector stable across reversals.
    var n_nbr = vec2(-d_nbr.y, d_nbr.x);
    if dot(n_this, n_nbr) < 0.0 {
        n_nbr = -n_nbr;
    }
    var miter = n_this + n_nbr;
    let ml2 = dot(miter, miter);
    if ml2 < 1e-6 {
        miter = n_this;
    } else {
        miter = miter * inverseSqrt(ml2);
    }
    let miter_scale = 1.0 / max(abs(dot(miter, n_this)), 0.5);

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
    let sigma_world = params.skirt_sigma * taper;

    // World units per pixel at this endpoint's depth: converts the profile
    // to pixels, which also keeps width perspective-correct (nearer ends
    // draw wider).
    let world_per_px =
        2.0 * max(clip_end.w, 1e-6) / (resolution.y * view.clip_from_view[1][1]);
    let core_px = core_half / world_per_px;
    let sigma_px = sigma_world / world_per_px;
    // At least ~2 px of skirt so edges always have room to soften.
    let margin_px = max(3.0 * sigma_px, 2.0);
    let outset_px = (core_px + margin_px) * live;

    let screen = s_end + miter * (corner.y * outset_px * miter_scale);

    var out: VertexOutput;
    // Write the offset screen position back to clip space at this
    // endpoint's original depth (the gizmo-line reconstruction).
    out.clip_position = vec4(
        clip_end.w * ((2.0 * screen) / resolution - 1.0),
        clip_end.z,
        clip_end.w,
    );

    let base = unpack_color(instance.color);
    var hdr = base.rgb * (params.bloom_factor * base.a + 1.0);

    // Long-exposure energy: light deposited per unit length goes as
    // 1/speed, like a beam writing on film — slow passages pool hot, fast
    // ones streak faint. Uses the CPU-smoothed speed (raw per-segment
    // speed jitters and beads). Clamped so near-stationary bodies don't
    // blow out and ejections stay legible.
    if params.exposure_reference_speed > 0.0 {
        hdr *= clamp(
            params.exposure_reference_speed / max(instance.speed, 1e-5),
            0.15,
            5.0,
        );
    }

    out.color = vec4(hdr, fade_alpha(age) * live);
    out.across = corner.y * outset_px;
    out.core_half = core_px;
    out.sigma = sigma_px;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Lateral soft profile in pixels: full energy inside the core, Gaussian
    // skirt outside, never sharper than ~0.75 px so edges cannot alias.
    let sigma = max(in.sigma, 0.75);
    let d = max(abs(in.across) - in.core_half, 0.0);
    let skirt = exp(-0.5 * d * d / (sigma * sigma));

    // True additive blending (src One, dst One): emit premultiplied energy;
    // the alpha channel does not participate.
    let a = in.color.a * skirt;
    return vec4(in.color.rgb * a, a);
}
