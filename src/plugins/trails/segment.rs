//! The per-segment instance record and the main-world staging queue that
//! carries each record tick's segments to the render world.

use crate::prelude::*;
use bytemuck::{Pod, Zeroable};

/// One trail segment exactly as the GPU reads it: 40 bytes, instance-rate.
/// The layout is mirrored by `segment_instance_layout()` in
/// `render/pipeline.rs` and by the `Instance` struct in `trail_ring.wgsl`;
/// all three must change together.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct TrailSegment {
    /// Older endpoint, world space.
    pub p0: [f32; 3],
    /// Newer endpoint, world space.
    pub p1: [f32; 3],
    /// Effective-time birth of `p0`.
    pub birth0: f32,
    /// Effective-time birth of `p1`.
    pub birth1: f32,
    /// Body radius at record time; drives trail width.
    pub radius: f32,
    /// Packed color (see `pack_trail_color`): luminance in bits 31..24,
    /// base RGB in bits 23..0.
    pub color: u32,
}

pub const SEGMENT_STRIDE: u64 = size_of::<TrailSegment>() as u64;

/// Segments recorded since the last render extract. The extract system
/// drains `pending` once per render frame; the recorder appends one segment
/// per live trail per record tick, so the queue holds at most one tick's
/// batch at healthy frame rates.
#[derive(Resource, Default)]
pub struct TrailSegmentQueue {
    pub pending: Vec<TrailSegment>,
    /// Bumped on `SimulationCommand::Restart`. The render world wipes its
    /// ring (head = live = 0) when the generation changes; no buffer
    /// deallocation or upload is involved.
    pub reset_generation: u32,
}
