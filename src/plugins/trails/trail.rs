//! Trail data and time bookkeeping. No rendering: geometry lives on the GPU
//! side (see `material.rs` / `trail.wgsl`); this module only records points.

use crate::prelude::*;
use std::collections::VecDeque;

/// One recorded sample of a body's path.
#[derive(Clone, Debug)]
pub struct TrailPoint {
    pub position: Vec3,
    /// Polyline tangent toward the older neighbor, frozen at record time.
    /// When the older neighbor later expires this becomes slightly stale for
    /// the new tail point; adjacent segments are near-collinear and the tail
    /// sits at minimum width and alpha, so the twist is invisible.
    pub tangent: Vec3,
    /// Effective (pause-adjusted) time the point was recorded.
    pub birth: f32,
    /// Body radius at record time; drives per-point trail width.
    pub radius: f32,
}

/// Global pause bookkeeping for trails. All live trails share one pause
/// history: they are only created on `Added<PhysicsBody>` (startup and
/// restart) and pause is global via `AppState`, so per-trail clocks would be
/// identical. If per-body trail spawning is ever added mid-run, revisit.
#[derive(Resource, Debug, Default)]
pub struct TrailClock {
    pause_time: Option<f32>,
    total_pause_duration: f32,
}

impl TrailClock {
    pub fn pause(&mut self, now: f32) {
        if self.pause_time.is_none() {
            self.pause_time = Some(now);
        }
    }

    pub fn unpause(&mut self, now: f32) {
        if let Some(pause_start) = self.pause_time.take() {
            self.total_pause_duration += now - pause_start;
        }
    }

    pub fn is_paused(&self) -> bool {
        self.pause_time.is_some()
    }

    /// Simulation-facing time: wall time minus accumulated pause time, frozen
    /// while paused. All trail timestamps (`TrailPoint::birth`,
    /// `Trail::last_update`, the shader's `effective_time` uniform) live in
    /// this timeline, which is what makes pause/unpause seamless without any
    /// per-trail adjustment.
    pub fn effective_time(&self, now: f32) -> f32 {
        match self.pause_time {
            Some(pause_start) => pause_start - self.total_pause_duration,
            None => now - self.total_pause_duration,
        }
    }
}

#[derive(Component, Debug)]
pub struct Trail {
    /// Newest point at index 0 (`push_front`).
    pub points: VecDeque<TrailPoint>,
    /// Effective time of the last recorded point.
    last_update: f32,
    /// Smoothed record-time speed (EMA over ~3 ticks), feeding the
    /// instanced renderer's long-exposure energy term. Zero until the
    /// first segment.
    pub speed_ema: f32,
    /// Set when recording changes the point set; cleared by the mesh rebuild
    /// system. This is what decouples GPU uploads from frame rate: fade,
    /// camera-facing width, and expiry are all shader-side, so uploads track
    /// the record rate. Expiry deliberately does not set this — see
    /// [`Trail::trim_expired`].
    pub dirty: bool,
}

impl Trail {
    pub fn new() -> Self {
        Self {
            points: VecDeque::new(),
            last_update: 0.0,
            speed_ema: 0.0,
            dirty: false,
        }
    }

    pub fn should_update(&self, effective_now: f32, update_interval: f32) -> bool {
        effective_now - self.last_update >= update_interval
    }

    pub fn add_point(
        &mut self,
        position: Vec3,
        radius: f32,
        effective_now: f32,
        max_points: usize,
    ) {
        // Tangent toward the older neighbor, matching the CPU tessellator's
        // `points[i + 1] - points[i]` (index 0 is newest). A body that hasn't
        // moved gets a zero tangent, which the mesh builder turns into zero
        // width (invisible) rather than an arbitrarily oriented quad.
        let tangent = match self.points.front() {
            Some(older) => (older.position - position).normalize_or_zero(),
            None => Vec3::ZERO,
        };

        // The very first point was recorded with no older neighbor; give it
        // this segment's tangent once a second point exists.
        if self.points.len() == 1 && tangent != Vec3::ZERO {
            self.points[0].tangent = tangent;
        }

        self.points.push_front(TrailPoint {
            position,
            tangent,
            birth: effective_now,
            radius,
        });

        // The count cap is enforced here, at record time, so it holds exactly
        // at all times without a per-frame cleanup pass. At default config it
        // is not binding (the age bound trims first); it is a safety valve,
        // and record time is where a safety valve belongs. Clamped to 1 so a
        // zero cap cannot empty the deque it just pushed into.
        while self.points.len() > max_points.max(1) {
            self.points.pop_back();
        }

        self.last_update = effective_now;
        self.dirty = true;
    }

    /// Drop points older than `max_age`. Deliberately does NOT set `dirty`:
    /// expired points are already invisible (the vertex shader collapses
    /// them against `effective_time`), live trails fold removals into their
    /// next record rebuild, and orphaned trails stop uploading entirely
    /// during fade-out. Called at a coarse cadence, not per frame. Returns
    /// the number of points removed.
    pub fn trim_expired(&mut self, effective_now: f32, max_age: f32) -> usize {
        let before = self.points.len();
        self.points
            .retain(|point| effective_now - point.birth <= max_age);
        before - self.points.len()
    }
}

impl Default for Trail {
    fn default() -> Self {
        Self::new()
    }
}
