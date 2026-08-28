//! Render-world state for trails: the extracted per-frame snapshot, the
//! persistent segment ring, and the shared params uniform.

use bevy::prelude::*;
use bevy::render::MainWorld;
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};

use crate::config::{FadeCurve, SimulationConfig, TaperCurve, TrailConfig};
use crate::plugins::trails::segment::{SEGMENT_STRIDE, TrailSegment, TrailSegmentQueue};
use crate::plugins::trails::{TrailClock, TrailsVisualizationSettings};

pub const TRAIL_FLAG_FADING: u32 = 1;
pub const TRAIL_FLAG_TAPERING: u32 = 2;
pub const TRAIL_FLAG_WIDTH_RELATIVE: u32 = 4;

/// The one uniform shared by every segment. 48 bytes, a multiple of 16, so
/// no WebGL2 padding is needed if this ever ports.
#[derive(ShaderType, Clone, Copy, Default)]
pub struct TrailParams {
    /// Pause-adjusted now; segment ages are `effective_time - birth`.
    pub effective_time: f32,
    /// Doubles as the shader-side expiry cutoff: segments older than this
    /// collapse to zero area in the vertex stage.
    pub trail_length_seconds: f32,
    pub min_alpha: f32,
    pub max_alpha: f32,
    pub bloom_factor: f32,
    /// Core width when the WIDTH_RELATIVE flag is unset.
    pub base_width: f32,
    /// Core width per unit body radius when WIDTH_RELATIVE is set.
    pub body_size_multiplier: f32,
    /// Width floor for the age-keyed taper.
    pub min_width_ratio: f32,
    /// Gaussian cross-profile falloff, world units. Also sets the quad's
    /// skirt margin (3 sigma) in the vertex stage.
    pub skirt_sigma: f32,
    /// 0 Linear, 1 Exponential, 2 SmoothStep, 3 EaseInOut.
    pub fade_curve: u32,
    /// 0 Linear, 1 Exponential, 2 SmoothStep.
    pub taper_curve: u32,
    pub flags: u32,
    /// Long-exposure energy scaling reference speed; 0 disables.
    pub exposure_reference_speed: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

impl TrailParams {
    pub fn from_config(config: &TrailConfig, effective_time: f32) -> Self {
        let mut flags = 0;
        if config.enable_fading {
            flags |= TRAIL_FLAG_FADING;
        }
        if config.enable_tapering {
            flags |= TRAIL_FLAG_TAPERING;
        }
        if config.width_relative_to_body {
            flags |= TRAIL_FLAG_WIDTH_RELATIVE;
        }

        Self {
            effective_time,
            trail_length_seconds: config.trail_length_seconds,
            min_alpha: config.min_alpha,
            max_alpha: config.max_alpha,
            bloom_factor: config.bloom_factor,
            base_width: config.base_width,
            body_size_multiplier: config.body_size_multiplier,
            min_width_ratio: config.min_width_ratio,
            skirt_sigma: config.glow_sigma.max(0.001),
            fade_curve: match config.fade_curve {
                FadeCurve::Linear => 0,
                FadeCurve::Exponential => 1,
                FadeCurve::SmoothStep => 2,
                FadeCurve::EaseInOut => 3,
            },
            taper_curve: match config.taper_curve {
                TaperCurve::Linear => 0,
                TaperCurve::Exponential => 1,
                TaperCurve::SmoothStep => 2,
            },
            flags,
            exposure_reference_speed: config.exposure_reference_speed,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }
}

/// Snapshot taken from the main world once per render frame.
#[derive(Resource, Default)]
pub struct ExtractedTrailFrame {
    /// This tick's new segments, appended at the ring head.
    pub fresh: Vec<TrailSegment>,
    /// The previous tick's batch with corrected outgoing tangents,
    /// re-uploaded over the slots it already occupies.
    pub rewrite: Vec<TrailSegment>,
    pub reset_generation: u32,
    pub visible: bool,
    pub params: TrailParams,
    pub capacity: u32,
}

/// The persistent GPU ring of segments. Created once (and only recreated if
/// the configured capacity changes); appends overwrite the oldest slots a
/// full lap later, by which time they are long expired shader-side.
#[derive(Resource, Default)]
pub struct TrailRing {
    pub buffer: Option<Buffer>,
    pub capacity: u32,
    /// Next slot to write.
    pub head: u32,
    /// Number of valid slots behind `head` (saturates at `capacity`).
    pub live: u32,
    /// Size of the most recent fresh batch — the slots directly behind
    /// `head`, which the next tick's rewrite batch overwrites with
    /// corrected tangents.
    last_batch: u32,
    seen_generation: u32,
}

/// The params uniform and its bind group, rebuilt each frame.
#[derive(Resource, Default)]
pub struct TrailParamsUniform {
    pub uniform: UniformBuffer<TrailParams>,
}

#[derive(Resource, Default)]
pub struct TrailParamsBindGroup(pub Option<BindGroup>);

/// Ring capacity: enough slots for every live trail's full point history,
/// plus slack for the lap between expiry and overwrite.
fn ring_capacity(config: &SimulationConfig) -> u32 {
    let trails = &config.trails;
    let points_per_trail = if trails.update_interval_seconds > 0.0 {
        (trails.trail_length_seconds / trails.update_interval_seconds).ceil() as u64 + 2
    } else {
        2
    };
    let per_trail = points_per_trail.min(trails.max_points_per_trail as u64);
    let capacity = (config.physics.body_count as u64) * per_trail;
    // Hard safety valve: ~160 MB of ring at the 40-byte stride.
    capacity.clamp(1, 4_000_000) as u32
}

/// Drain the main world's pending segments and snapshot everything the
/// render systems need. Runs in `ExtractSchedule`.
pub fn extract_trails(mut main_world: ResMut<MainWorld>, mut frame: ResMut<ExtractedTrailFrame>) {
    let world = main_world.as_mut();

    let elapsed = world.resource::<Time>().elapsed_secs();
    let effective_time = world.resource::<TrailClock>().effective_time(elapsed);
    {
        let config = world.resource::<SimulationConfig>();
        frame.params = TrailParams::from_config(&config.trails, effective_time);
        frame.capacity = ring_capacity(config);
    }
    frame.visible = world.resource::<TrailsVisualizationSettings>().enabled;

    let mut queue = world.resource_mut::<TrailSegmentQueue>();
    frame.reset_generation = queue.reset_generation;
    frame.fresh.clear();
    frame.rewrite.clear();
    if queue.fresh_is_new {
        queue.fresh_is_new = false;
        // The rewrite batch is finished (its fix-ups happened during this
        // tick) — drain it. The fresh batch is cloned: it stays behind to
        // receive its own fix-ups next tick.
        frame.rewrite.append(&mut queue.rewrite);
        frame.fresh.extend_from_slice(&queue.fresh);
    }
}

/// Write a batch into the ring starting at `start`, splitting across the
/// wrap if needed.
fn write_wrapped(
    render_queue: &RenderQueue,
    buffer: &Buffer,
    capacity: u32,
    start: u32,
    segments: &[TrailSegment],
) {
    let first_len = (capacity - start).min(segments.len() as u32) as usize;
    render_queue.write_buffer(
        buffer,
        start as u64 * SEGMENT_STRIDE,
        bytemuck::cast_slice(&segments[..first_len]),
    );
    if first_len < segments.len() {
        render_queue.write_buffer(buffer, 0, bytemuck::cast_slice(&segments[first_len..]));
    }
}

/// Write the frame's batches into the ring: the rewrite batch over the
/// slots directly behind `head` (tangent corrections), the fresh batch at
/// `head`. A handful of `write_buffer` calls per record tick — this is the
/// entire upload cost.
pub fn prepare_trail_ring(
    mut ring: ResMut<TrailRing>,
    frame: Res<ExtractedTrailFrame>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    if ring.buffer.is_none() || ring.capacity != frame.capacity {
        ring.capacity = frame.capacity.max(1);
        ring.head = 0;
        ring.live = 0;
        ring.last_batch = 0;
        ring.seen_generation = frame.reset_generation;
        ring.buffer = Some(render_device.create_buffer(&BufferDescriptor {
            label: Some("trail segment ring"),
            size: ring.capacity as u64 * SEGMENT_STRIDE,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }

    if ring.seen_generation != frame.reset_generation {
        ring.head = 0;
        ring.live = 0;
        ring.last_batch = 0;
        ring.seen_generation = frame.reset_generation;
    }

    let buffer = ring.buffer.as_ref().unwrap().clone();

    // Tangent corrections for the previous batch, over its original slots.
    // A size mismatch means the batches raced a reset; skip rather than
    // corrupt unrelated slots.
    let n_rewrite = frame.rewrite.len() as u32;
    if n_rewrite > 0 && n_rewrite == ring.last_batch {
        let start = (ring.head + ring.capacity - ring.last_batch) % ring.capacity;
        write_wrapped(&render_queue, &buffer, ring.capacity, start, &frame.rewrite);
    }

    // A batch larger than the whole ring can only happen under degenerate
    // config; keep the newest slots' worth.
    let fresh: &[TrailSegment] = &frame.fresh;
    let fresh = if fresh.len() as u32 > ring.capacity {
        &fresh[fresh.len() - ring.capacity as usize..]
    } else {
        fresh
    };
    let n = fresh.len() as u32;
    if n == 0 {
        return;
    }

    write_wrapped(&render_queue, &buffer, ring.capacity, ring.head, fresh);
    ring.head = (ring.head + n) % ring.capacity;
    ring.live = (ring.live + n).min(ring.capacity);
    ring.last_batch = n;
}

/// Push this frame's params into the uniform buffer.
pub fn prepare_trail_params(
    mut params: ResMut<TrailParamsUniform>,
    frame: Res<ExtractedTrailFrame>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    params.uniform.set(frame.params);
    params.uniform.write_buffer(&render_device, &render_queue);
}

pub fn prepare_trail_params_bind_group(
    mut bind_group: ResMut<TrailParamsBindGroup>,
    params: Res<TrailParamsUniform>,
    pipeline: Res<super::pipeline::TrailPipeline>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
) {
    bind_group.0 = params.uniform.binding().map(|binding| {
        render_device.create_bind_group(
            "trail params bind group",
            &pipeline_cache.get_bind_group_layout(&pipeline.params_layout),
            &BindGroupEntries::single(binding),
        )
    });
}
