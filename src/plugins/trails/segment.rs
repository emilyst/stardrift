//! The per-segment instance record and the main-world staging queue that
//! carries each record tick's segments to the render world.

use crate::prelude::*;
use bevy::platform::collections::HashMap;
use bytemuck::{Pod, Zeroable};

/// One trail segment exactly as the GPU reads it: 64 bytes, instance-rate.
/// The layout is mirrored by `segment_instance_layout()` in
/// `render/pipeline.rs` and by the `Instance` struct in `trail_ring.wgsl`;
/// all three must change together.
///
/// The neighbor tangents exist for mitered joints: the vertex shader
/// rotates each end edge onto the bisector of the adjacent segment
/// directions, so consecutive quads share their edges exactly — no gaps at
/// bends, no overlap, no additive seams. `t_next` is unknowable when a
/// segment is first recorded (its successor does not exist yet), so it is
/// written as the segment's own direction and corrected one tick later via
/// the rewrite batch (see [`TrailSegmentQueue`]).
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct TrailSegment {
    /// Older endpoint, world space.
    pub p0: [f32; 3],
    /// Newer endpoint, world space.
    pub p1: [f32; 3],
    /// Effective-time births of `p0` (x) and `p1` (y).
    pub birth0: f32,
    pub birth1: f32,
    /// Body radius at record time; drives trail width.
    pub radius: f32,
    /// Packed color (see `pack_trail_color`).
    pub color: u32,
    /// Direction of the previous segment (into `p0`); this segment's own
    /// direction when there is no previous segment.
    pub t_prev: [f32; 3],
    /// Direction of the next segment (out of `p1`); this segment's own
    /// direction until the fix-up one tick later.
    pub t_next: [f32; 3],
}

pub const SEGMENT_STRIDE: u64 = size_of::<TrailSegment>() as u64;

/// Staging between the recorder and the render world's ring.
///
/// Each record tick produces one `fresh` batch (one segment per live
/// trail, appended to the ring) and finalizes the previous tick's batch by
/// writing corrected `t_next` values into it; that corrected copy is
/// re-uploaded over the slots it already occupies (`rewrite`). Both
/// batches are contiguous in the ring by construction, so the upload cost
/// stays a couple of `write_buffer` calls per tick.
#[derive(Resource, Default)]
pub struct TrailSegmentQueue {
    /// This tick's new segments; cloned by extract for upload, retained
    /// here until next tick's rotation.
    pub fresh: Vec<TrailSegment>,
    /// Body entity -> index into `fresh`.
    pub fresh_index: HashMap<Entity, usize>,
    /// Set on a tick, cleared by extract: tells extract that `fresh` (and
    /// `rewrite`) carry a new batch to upload.
    pub fresh_is_new: bool,
    /// The previous tick's batch, receiving t_next fix-ups during the
    /// current tick; drained by extract for re-upload over its ring slots.
    pub rewrite: Vec<TrailSegment>,
    /// Body entity -> index into `rewrite`, for the fix-ups.
    pub rewrite_index: HashMap<Entity, usize>,
    /// Effective time of the last record tick.
    pub last_tick: f32,
    /// Bumped on `SimulationCommand::Restart`. The render world wipes its
    /// ring (head = live = 0) when the generation changes.
    pub reset_generation: u32,
}

impl TrailSegmentQueue {
    /// Advances the shared record clock; returns true when a new tick
    /// begins. All trails record on the same tick, which is what keeps
    /// each batch contiguous in the ring.
    pub fn should_tick(&mut self, effective_now: f32, interval: f32) -> bool {
        if effective_now - self.last_tick >= interval {
            self.last_tick = effective_now;
            true
        } else {
            false
        }
    }

    /// Called at the start of a tick: the previous fresh batch becomes the
    /// rewrite batch, ready to receive t_next fix-ups as the tick's new
    /// segments are recorded.
    pub fn begin_tick(&mut self) {
        self.rewrite = std::mem::take(&mut self.fresh);
        self.rewrite_index = std::mem::take(&mut self.fresh_index);
        self.fresh_is_new = true;
    }

    /// Correct the previous batch's outgoing tangent for one trail.
    pub fn fix_up_prev(&mut self, body: Entity, t_next: [f32; 3]) {
        if let Some(&index) = self.rewrite_index.get(&body) {
            self.rewrite[index].t_next = t_next;
        }
    }

    pub fn clear_for_restart(&mut self) {
        self.fresh.clear();
        self.fresh_index.clear();
        self.fresh_is_new = false;
        self.rewrite.clear();
        self.rewrite_index.clear();
        self.reset_generation = self.reset_generation.wrapping_add(1);
    }
}
