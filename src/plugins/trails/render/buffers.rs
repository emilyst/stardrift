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
            // Placeholder until the stage-4 tuning session: a skirt of a
            // quarter of the default-radius core reads as a soft edge
            // without smearing.
            skirt_sigma: 0.25 * config.body_size_multiplier.max(0.05),
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
        }
    }
}

/// Snapshot taken from the main world once per render frame.
#[derive(Resource, Default)]
pub struct ExtractedTrailFrame {
    pub segments: Vec<TrailSegment>,
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
    frame.segments.clear();
    frame.segments.append(&mut queue.pending);
}

/// Write the frame's segments into the ring. One `write_buffer` per record
/// tick (two across a wrap) — this is the entire per-frame upload cost.
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
        ring.seen_generation = frame.reset_generation;
    }

    let segments: &[TrailSegment] = &frame.segments;
    // A batch larger than the whole ring can only happen under degenerate
    // config; keep the newest slots' worth.
    let segments = if segments.len() as u32 > ring.capacity {
        &segments[segments.len() - ring.capacity as usize..]
    } else {
        segments
    };
    let n = segments.len() as u32;
    if n == 0 {
        return;
    }

    let buffer = ring.buffer.as_ref().unwrap();
    let first_len = (ring.capacity - ring.head).min(n) as usize;
    render_queue.write_buffer(
        buffer,
        ring.head as u64 * SEGMENT_STRIDE,
        bytemuck::cast_slice(&segments[..first_len]),
    );
    if first_len < segments.len() {
        render_queue.write_buffer(buffer, 0, bytemuck::cast_slice(&segments[first_len..]));
    }

    ring.head = (ring.head + n) % ring.capacity;
    ring.live = (ring.live + n).min(ring.capacity);
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
