//! Unit tests for the trail data layer (`stardrift::plugins::trails`):
//! `TrailClock` pause bookkeeping and `Trail` point recording. These types
//! are pure logic — no rendering, no ECS schedule needed.
//!
//! What these pin:
//! - `TrailClock::effective_time` is wall time minus accumulated pauses,
//!   frozen while paused, and continuous across pause/unpause (no jump at
//!   the unpause instant) — the property that makes pause seamless without
//!   per-trail adjustment;
//! - a second `pause()` while paused is a no-op (first pause wins);
//! - `Trail::add_point` pushes the newest point to index 0, records
//!   birth/radius as given, maintains the tangent convention (toward
//!   the older neighbor; first point backfilled once a second exists; zero
//!   tangent for a stationary body), and enforces the point-count cap at
//!   record time (evicting the oldest; a cap of 0 clamps to 1);
//! - `should_update` is inclusive at exactly one interval elapsed;
//! - `trim_expired` keeps age == max_age (strict cutoff is `<=`) and never
//!   sets `dirty` — expired points are hidden shader-side, so the trim must
//!   not trigger GPU uploads; that is the contract the whole
//!   upload-at-record-rate design rests on.

use bevy::prelude::*;
use stardrift::plugins::trails::{Trail, TrailClock, TrailPoint};

/// Newest-first accessor; index 0 is the most recently added point.
fn point(trail: &Trail, index: usize) -> &TrailPoint {
    &trail.points[index]
}

fn assert_vec3_close(actual: Vec3, expected: Vec3, epsilon: f32, context: &str) {
    assert!(
        actual.abs_diff_eq(expected, epsilon),
        "{context}: expected {expected}, got {actual}"
    );
}

// ---------------------------------------------------------------------------
// TrailClock
// ---------------------------------------------------------------------------

#[test]
fn effective_time_equals_wall_time_with_no_pauses() {
    let clock = TrailClock::default();
    // Pure pass-through: exact equality is intentional.
    assert_eq!(clock.effective_time(0.0), 0.0);
    assert_eq!(clock.effective_time(1.5), 1.5);
    assert_eq!(clock.effective_time(123.25), 123.25);
    assert!(!clock.is_paused());
}

#[test]
fn pause_freezes_effective_time_at_the_pause_moment() {
    let mut clock = TrailClock::default();
    clock.pause(5.0);

    assert!(clock.is_paused());
    // Frozen: later wall times don't matter.
    assert_eq!(clock.effective_time(5.0), 5.0);
    assert_eq!(clock.effective_time(10.0), 5.0);
    assert_eq!(clock.effective_time(1000.0), 5.0);
}

#[test]
fn pause_while_paused_is_a_noop_first_pause_wins() {
    let mut clock = TrailClock::default();
    clock.pause(5.0);
    clock.pause(8.0); // must not move the freeze point

    assert_eq!(clock.effective_time(10.0), 5.0);

    // The ignored second pause must not corrupt the accumulated duration
    // either: unpausing at 9.0 accumulates 9 - 5 = 4, not 9 - 8 = 1.
    clock.unpause(9.0);
    assert_eq!(clock.effective_time(10.0), 6.0);
}

#[test]
fn unpause_resumes_without_a_jump_and_advances_again() {
    let mut clock = TrailClock::default();
    let frozen = clock.effective_time(5.0);
    clock.pause(5.0);
    clock.unpause(9.0);

    assert!(!clock.is_paused());
    // Continuity: effective time at the unpause instant equals the value at
    // the pause instant. All values here are exactly representable, so the
    // arithmetic (9 - (9 - 5)) is exact.
    assert_eq!(clock.effective_time(9.0), frozen);
    // And it advances 1:1 with wall time afterwards.
    assert_eq!(clock.effective_time(11.5), frozen + 2.5);
}

#[test]
fn unpause_without_pause_is_a_noop() {
    let mut clock = TrailClock::default();
    clock.unpause(7.0);

    assert!(!clock.is_paused());
    assert_eq!(clock.effective_time(7.0), 7.0);
}

#[test]
fn multiple_pause_cycles_accumulate_total_pause_duration() {
    let mut clock = TrailClock::default();
    clock.pause(2.0);
    clock.unpause(5.0); // +3
    clock.pause(7.0);
    assert!(clock.is_paused());
    assert_eq!(clock.effective_time(100.0), 4.0); // frozen at 7 - 3
    clock.unpause(11.0); // +4, total 7

    assert!(!clock.is_paused());
    assert_eq!(clock.effective_time(12.0), 5.0);
    assert_eq!(clock.effective_time(20.0), 13.0);
}

// ---------------------------------------------------------------------------
// Trail
// ---------------------------------------------------------------------------

#[test]
fn new_trail_is_empty_and_clean() {
    let trail = Trail::new();
    assert!(trail.points.is_empty());
    assert!(!trail.dirty);
}

#[test]
fn add_point_pushes_front_and_records_birth_and_radius() {
    let mut trail = Trail::new();
    trail.add_point(Vec3::new(1.0, 2.0, 3.0), 0.5, 10.0, usize::MAX);
    trail.add_point(Vec3::new(4.0, 5.0, 6.0), 0.75, 11.0, usize::MAX);

    assert_eq!(trail.points.len(), 2);
    // Newest at index 0; pass-through values are exact.
    assert_eq!(point(&trail, 0).position, Vec3::new(4.0, 5.0, 6.0));
    assert_eq!(point(&trail, 0).birth, 11.0);
    assert_eq!(point(&trail, 0).radius, 0.75);
    assert_eq!(point(&trail, 1).position, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(point(&trail, 1).birth, 10.0);
    assert_eq!(point(&trail, 1).radius, 0.5);
    assert!(trail.dirty);
}

#[test]
fn first_point_has_zero_tangent_until_a_second_point_exists() {
    let mut trail = Trail::new();
    trail.add_point(Vec3::new(1.0, 2.0, 3.0), 0.5, 0.0, usize::MAX);

    assert_eq!(point(&trail, 0).tangent, Vec3::ZERO);
}

#[test]
fn second_point_sets_tangent_toward_older_neighbor_and_backfills_first() {
    let mut trail = Trail::new();
    trail.add_point(Vec3::ZERO, 0.5, 0.0, usize::MAX);
    // Displacement (3, 4, 0): tangent from the newer point toward the older
    // one is normalize(older - newer) = (-0.6, -0.8, 0).
    trail.add_point(Vec3::new(3.0, 4.0, 0.0), 0.5, 1.0, usize::MAX);

    let expected = Vec3::new(-0.6, -0.8, 0.0);
    assert_vec3_close(point(&trail, 0).tangent, expected, 1e-6, "newest tangent");
    // The first point had no older neighbor at record time; it must have been
    // backfilled with the same direction so the tail segment is coherent.
    assert_vec3_close(
        point(&trail, 1).tangent,
        expected,
        1e-6,
        "backfilled first tangent",
    );
}

#[test]
fn stationary_body_records_zero_tangent_and_does_not_backfill() {
    let mut trail = Trail::new();
    let here = Vec3::new(1.0, 2.0, 3.0);
    trail.add_point(here, 0.5, 0.0, usize::MAX);
    trail.add_point(here, 0.5, 1.0, usize::MAX); // hasn't moved

    // Zero tangent means zero ribbon width (invisible), never an arbitrarily
    // oriented quad — and a zero direction must not overwrite the first
    // point's tangent placeholder.
    assert_eq!(point(&trail, 0).tangent, Vec3::ZERO);
    assert_eq!(point(&trail, 1).tangent, Vec3::ZERO);
}

#[test]
fn should_update_is_inclusive_at_one_full_interval() {
    let mut trail = Trail::new();
    // A fresh trail's last update is time zero, so it becomes due one full
    // interval into the effective timeline.
    assert!(!trail.should_update(0.5, 1.0));
    assert!(trail.should_update(1.0, 1.0));

    trail.add_point(Vec3::ZERO, 0.5, 1.0, usize::MAX);
    // All values exactly representable: 2.0 - 1.0 == 1.0 tests >= precisely.
    assert!(!trail.should_update(1.5, 1.0));
    assert!(trail.should_update(2.0, 1.0), "boundary must be inclusive");
    assert!(trail.should_update(2.5, 1.0));
}

#[test]
fn trim_keeps_points_at_exactly_max_age() {
    let mut trail = Trail::new();
    trail.add_point(Vec3::new(0.0, 0.0, 0.0), 0.5, 0.0, usize::MAX); // age 3 at now = 3
    trail.add_point(Vec3::new(1.0, 0.0, 0.0), 0.5, 1.0, usize::MAX); // age 2
    trail.add_point(Vec3::new(2.0, 0.0, 0.0), 0.5, 2.0, usize::MAX); // age 1

    let removed = trail.trim_expired(3.0, 2.0);

    // The cutoff is age <= max_age: the age-2 point survives, age-3 does not.
    assert_eq!(removed, 1);
    assert_eq!(trail.points.len(), 2);
    assert_eq!(point(&trail, 0).birth, 2.0);
    assert_eq!(point(&trail, 1).birth, 1.0);
}

#[test]
fn trim_never_sets_dirty() {
    let mut trail = Trail::new();
    trail.add_point(Vec3::ZERO, 0.5, 0.0, usize::MAX);
    trail.add_point(Vec3::new(1.0, 0.0, 0.0), 0.5, 1.0, usize::MAX);
    trail.dirty = false; // as the mesh rebuild system does after an upload

    // A no-op trim stays clean, and so does a removing trim: expired points
    // are hidden shader-side, so the trim must never trigger a GPU upload.
    // Live trails sync the buffer at their next record; orphaned trails
    // stop uploading entirely during fade-out.
    assert_eq!(trail.trim_expired(1.0, 100.0), 0);
    assert!(!trail.dirty, "no-op trim must not dirty the trail");
    assert_eq!(trail.points.len(), 2);

    assert_eq!(trail.trim_expired(200.0, 100.0), 2);
    assert!(!trail.dirty, "a removing trim must not dirty the trail");
    assert!(trail.points.is_empty());
}

#[test]
fn add_point_evicts_the_oldest_beyond_the_cap() {
    let mut trail = Trail::new();
    for i in 0..5 {
        trail.add_point(Vec3::new(i as f32, 0.0, 0.0), 0.5, i as f32, 3);
    }

    // The cap holds exactly at record time — no cleanup pass involved.
    assert_eq!(trail.points.len(), 3);
    // Newest-first: births 4, 3, 2 survive; the oldest two were evicted.
    assert_eq!(point(&trail, 0).birth, 4.0);
    assert_eq!(point(&trail, 1).birth, 3.0);
    assert_eq!(point(&trail, 2).birth, 2.0);
}

#[test]
fn add_point_cap_of_zero_clamps_to_one() {
    let mut trail = Trail::new();
    trail.add_point(Vec3::ZERO, 0.5, 0.0, 0);
    trail.add_point(Vec3::new(1.0, 0.0, 0.0), 0.5, 1.0, 0);

    // A zero cap must not empty the deque it just pushed into: the newest
    // point always survives.
    assert_eq!(trail.points.len(), 1);
    assert_eq!(point(&trail, 0).birth, 1.0);
}

#[test]
fn dirty_is_clearable_and_add_point_sets_it_again() {
    let mut trail = Trail::new();
    trail.add_point(Vec3::ZERO, 0.5, 0.0, usize::MAX);
    assert!(trail.dirty);

    trail.dirty = false;
    trail.add_point(Vec3::new(1.0, 0.0, 0.0), 0.5, 1.0, usize::MAX);
    assert!(trail.dirty, "add_point after a clear must re-dirty");
}
