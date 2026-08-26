//! Merge-on-contact collision detection and resolution.
//!
//! Detection is a swept (continuous) test over each step's motion: every
//! body's segment from `PreviousPosition` to `Position` is checked pairwise
//! for closest approach within contact distance. Sweeping is required rather
//! than an end-of-step overlap test because the default scene collapses to a
//! dense core where per-step relative displacement (~4-5 units) is comparable
//! to contact distance (4-8 units); a uniform-phase model puts the discrete
//! test's miss rate there at 5-15% of contacts (P ≈ L²/12R² for step length
//! L against contact distance R — a derived estimate, not a measurement),
//! and a missed contact produces an unphysical interpenetrating slingshot
//! exactly where merging is meant to remove that pathology.
//!
//! Linearizing the step's motion is safe: an isolated attracting pair's true
//! relative orbit is convex toward the focus, so the chord underestimates
//! minimum separation and pair curvature can only produce (slightly) early
//! merges, not missed ones. Third-body tidal bending can locally exceed the
//! pair term near a massive merged clump (2GMd/D³ against the pair's
//! G(mᵢ+mⱼ)/d²), so a grazing encounter there can in principle be missed by
//! a hair; a still-converging pair is caught on a later step, and an exactly
//! grazing one that isn't was a zero-measure contact to begin with. The
//! early-merge bias from pair curvature is bounded by
//! (π/6)·G·ρ·dt² of the contact distance (~1.5% worst case at defaults);
//! validity requires dt ≪ sqrt(6/(πGρ)), which the 60 Hz step clears by 8x.
//!
//! Merging is perfectly inelastic: mass sums, position and velocity go to the
//! mass-weighted mean of the end-of-step states, radius is re-derived from
//! total mass through the shared density relation. This conserves linear
//! momentum and the mass-weighted position sum exactly (so barycentric drift
//! correction sees nothing), while kinetic energy and the pair's internal
//! angular momentum are genuinely lost — the physics of an inelastic
//! collision, not an error. Simultaneous and chained contacts are resolved
//! as connected components in one operation, with accumulation in
//! entity-sorted order so the result is independent of discovery order.

use crate::config::SimulationConfig;
use crate::physics::components::{Mass, PhysicsBody, Position, PreviousPosition, Radius, Velocity};
use crate::physics::math::{Scalar, Vector, radius_for_mass};
use crate::physics::resources::PhysicsTime;
use bevy::prelude::*;

/// Swept-sphere contact test for one step of relative motion.
///
/// `d0` and `d1` are the separation vectors (body j minus body i) at the
/// start and end of the step; `contact_distance_squared` is (rᵢ+rⱼ)² times
/// the configured contact factor, squared. Returns whether the pair passed
/// within contact distance at any point during the step, assuming linear
/// motion.
///
/// Division-free and NaN-proof by construction: zero relative displacement
/// lands in the first branch (a static overlap test) without any epsilon,
/// and the interior case uses the Lagrange-identity form
/// |d₀×w|² ≤ R²|w|², which avoids the catastrophic cancellation of
/// |d₀|² − (d₀·w)²/|w|² on deep approaches. Never takes a square root.
#[inline]
fn segments_contact(d0: Vector, d1: Vector, contact_distance_squared: Scalar) -> bool {
    let w = d1 - d0;
    let num = -d0.dot(w);
    let den = w.length_squared();
    if num <= 0.0 {
        // Closest at the start of the step; also the exact w == 0 case.
        d0.length_squared() <= contact_distance_squared
    } else if num >= den {
        // Closest at the end of the step.
        d1.length_squared() <= contact_distance_squared
    } else {
        // Interior minimum.
        d0.cross(w).length_squared() <= contact_distance_squared * den
    }
}

/// Perfectly inelastic merge accumulator.
///
/// The lerp form p + α(p_b − p) keeps intermediates on the segment between
/// the inputs, where the naive weighted sum Σmp/Σm forms intermediates of
/// magnitude m·|p| that carry the answer's structure in their low bits.
#[derive(Clone, Copy)]
struct MergedState {
    mass: Scalar,
    position: Vector,
    velocity: Vector,
}

#[inline]
fn accumulate(acc: MergedState, mass: Scalar, position: Vector, velocity: Vector) -> MergedState {
    let total = acc.mass + mass;
    // Guard the massless-pair corner (e.g. a config with min_body_radius = 0):
    // alpha would be 0/0 and the NaN survivor would poison the whole scene
    // through the octree on the next step. An even split is as principled as
    // anything for two zero-mass points.
    let alpha = if total > 0.0 { mass / total } else { 0.5 };
    MergedState {
        mass: total,
        position: acc.position + alpha * (position - acc.position),
        velocity: acc.velocity + alpha * (velocity - acc.velocity),
    }
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

fn union(parent: &mut [usize], a: usize, b: usize) {
    let ra = find(parent, a);
    let rb = find(parent, b);
    if ra != rb {
        parent[ra] = rb;
    }
}

#[derive(Clone, Copy)]
struct Candidate {
    entity: Entity,
    prev: Vector,
    curr: Vector,
    velocity: Vector,
    mass: Scalar,
    radius: Scalar,
}

/// Buffers reused across steps to keep the steady state allocation-free.
#[derive(Default)]
pub struct CollisionBuffers {
    bodies: Vec<Candidate>,
    /// Broad-phase bounding sphere per body: segment midpoint and radius
    /// inflated by half the segment length.
    spheres: Vec<(Vector, Scalar)>,
    order: Vec<usize>,
    parent: Vec<usize>,
    grouped: Vec<usize>,
}

/// Detect contacts over the step just integrated and merge the bodies
/// involved. Runs between `PhysicsSet::IntegrateMotions` (which writes both
/// `PreviousPosition` and the committed `Position`) and
/// `PhysicsSet::CorrectBarycentricDrift` (so drift correction sees the
/// post-merge set, and both segment endpoints are in the same frame).
pub fn detect_and_merge_collisions(
    mut commands: Commands,
    mut bodies: Query<
        (
            Entity,
            &mut Position,
            &PreviousPosition,
            &mut Velocity,
            &mut Mass,
            &mut Radius,
            &mut Transform,
        ),
        With<PhysicsBody>,
    >,
    config: Res<SimulationConfig>,
    physics_time: Res<PhysicsTime>,
    mut buffers: Local<CollisionBuffers>,
) {
    // The pause guard is load-bearing beyond consistency with the other
    // physics systems: while paused, prev and curr stop advancing (the last
    // completed step's segment just before the pause, or the spawn position
    // before any step), and re-testing that stale segment every frame would
    // merge bodies left near contact in a frozen scene.
    if physics_time.is_paused() || !config.physics.collisions.enabled {
        return;
    }

    // Negative values would make the broad phase (which uses the factor
    // linearly) and the narrow phase (which squares it) disagree.
    let contact_factor = config.physics.collisions.contact_factor.max(0.0);
    let buffers = &mut *buffers;

    buffers.bodies.clear();
    for (entity, position, previous_position, velocity, mass, radius, _) in bodies.iter() {
        buffers.bodies.push(Candidate {
            entity,
            prev: previous_position.value(),
            curr: position.value(),
            velocity: velocity.value(),
            mass: mass.value(),
            radius: radius.value(),
        });
    }
    let n = buffers.bodies.len();
    if n < 2 {
        return;
    }

    // Broad phase: sweep-and-prune over per-body bounding spheres. Keying
    // on the segment midpoint with radius inflated by half the segment
    // length bounds the swept volume exactly; keying on endpoints would
    // double the slack. The sweep axis is the one with the largest midpoint
    // variance this step — a fixed axis degenerates to ~n²/2 sphere checks
    // when the scene collapses into a core whose extent along that axis is
    // smaller than the contact windows, which is precisely the dense
    // configuration merging produces. One O(n) pass, deterministic.
    buffers.spheres.clear();
    let mut mean = Vector::ZERO;
    for candidate in &buffers.bodies {
        let midpoint = (candidate.prev + candidate.curr) * 0.5;
        let inflated =
            candidate.radius * contact_factor + (candidate.curr - candidate.prev).length() * 0.5;
        mean += midpoint;
        buffers.spheres.push((midpoint, inflated));
    }
    mean /= n as Scalar;
    let mut variance = Vector::ZERO;
    for (midpoint, _) in &buffers.spheres {
        let d = *midpoint - mean;
        variance += d * d;
    }
    let axis = if variance.x >= variance.y && variance.x >= variance.z {
        0
    } else if variance.y >= variance.z {
        1
    } else {
        2
    };
    let axis_value = |v: Vector| -> Scalar {
        match axis {
            0 => v.x,
            1 => v.y,
            _ => v.z,
        }
    };

    buffers.order.clear();
    buffers.order.extend(0..n);
    let spheres = &buffers.spheres;
    buffers.order.sort_unstable_by(|&i, &j| {
        (axis_value(spheres[i].0) - spheres[i].1)
            .total_cmp(&(axis_value(spheres[j].0) - spheres[j].1))
    });

    buffers.parent.clear();
    buffers.parent.extend(0..n);
    let mut any_contact = false;
    for (rank, &i) in buffers.order.iter().enumerate() {
        let (center_i, inflated_i) = buffers.spheres[i];
        let sweep_end = axis_value(center_i) + inflated_i;
        for &j in &buffers.order[rank + 1..] {
            let (center_j, inflated_j) = buffers.spheres[j];
            if axis_value(center_j) - inflated_j > sweep_end {
                break;
            }
            let inflated_sum = inflated_i + inflated_j;
            if center_i.distance_squared(center_j) > inflated_sum * inflated_sum {
                continue;
            }
            let body_i = &buffers.bodies[i];
            let body_j = &buffers.bodies[j];
            let contact_distance = (body_i.radius + body_j.radius) * contact_factor;
            if segments_contact(
                body_j.prev - body_i.prev,
                body_j.curr - body_i.curr,
                contact_distance * contact_distance,
            ) {
                union(&mut buffers.parent, i, j);
                any_contact = true;
            }
        }
    }
    if !any_contact {
        return;
    }

    // Resolve each connected component of the contact graph in one merge.
    // Sorting members by Entity fixes the accumulation order (and hence the
    // rounding), making the result independent of pair-discovery order —
    // query iteration order is archetype order and permutes after despawns,
    // so it must not leak into the physics.
    //
    // Grouping below keys on parent[i] directly, so every slot must hold its
    // true root: find() alone only *halves* paths, which leaves depth-3+
    // chains (a 4-body contact chain) partially compressed and would split
    // the component. The write-back makes compression total.
    for i in 0..n {
        let root = find(&mut buffers.parent, i);
        buffers.parent[i] = root;
    }
    buffers.grouped.clear();
    buffers.grouped.extend(0..n);
    let parent = &buffers.parent;
    let candidates = &buffers.bodies;
    buffers
        .grouped
        .sort_unstable_by_key(|&i| (parent[i], candidates[i].entity));

    let mut group_start = 0;
    while group_start < n {
        let root = buffers.parent[buffers.grouped[group_start]];
        let mut group_end = group_start + 1;
        while group_end < n && buffers.parent[buffers.grouped[group_end]] == root {
            group_end += 1;
        }
        let members = &buffers.grouped[group_start..group_end];
        group_start = group_end;
        if members.len() < 2 {
            continue;
        }

        let first = &buffers.bodies[members[0]];
        let mut merged = MergedState {
            mass: first.mass,
            position: first.curr,
            velocity: first.velocity,
        };
        for &index in &members[1..] {
            let member = &buffers.bodies[index];
            merged = accumulate(merged, member.mass, member.curr, member.velocity);
        }

        // Survivor: the most massive member, ties broken toward the smaller
        // entity. Its trail continues naturally; the others' trails orphan
        // and fade out.
        //
        // Known limitation: the survivor's jump from its end-of-step
        // position to the merged centroid is itself never swept against
        // bystanders, so a third body sitting exactly in that gap is found
        // only by the next step's static-overlap branch (one step late, or
        // — if it is moving away fast enough — not at all). Accepted: the
        // window is one step, the geometry requires a fast long-range merge
        // with a precisely placed bystander, and sweeping the jump would
        // mean re-running detection within the step.
        let survivor_index = members
            .iter()
            .copied()
            .max_by(|&a, &b| {
                buffers.bodies[a]
                    .mass
                    .total_cmp(&buffers.bodies[b].mass)
                    .then(buffers.bodies[b].entity.cmp(&buffers.bodies[a].entity))
            })
            .expect("component has at least two members");

        let new_radius = radius_for_mass(merged.mass);
        let survivor = &buffers.bodies[survivor_index];
        if let Ok((_, mut position, _, mut velocity, mut mass, mut radius, mut transform)) =
            bodies.get_mut(survivor.entity)
        {
            *position.value_mut() = merged.position;
            *velocity.value_mut() = merged.velocity;
            *mass = Mass::new(merged.mass);
            // Scale the existing mesh rather than regenerating it: the sphere
            // is scale-invariant, and sync_transform_from_position writes
            // only translation, so scale persists.
            transform.scale *= (new_radius / radius.value()) as f32;
            *radius = Radius::new(new_radius);
        }

        for &index in members {
            if index != survivor_index {
                commands.entity(buffers.bodies[index].entity).despawn();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R2: Scalar = 4.0; // contact distance 2.0, squared

    #[test]
    fn static_overlap_detected_with_zero_displacement() {
        // w == 0 exercises the branch that would divide by zero in the
        // naive t* formula.
        let d = Vector::new(1.0, 0.0, 0.0);
        assert!(segments_contact(d, d, R2));
        let far = Vector::new(3.0, 0.0, 0.0);
        assert!(!segments_contact(far, far, R2));
    }

    #[test]
    fn no_nan_near_contact_boundary_with_identical_velocities() {
        for offset in [-1e-12, 0.0, 1e-12] {
            let d = Vector::new(2.0 + offset, 0.0, 0.0);
            let result = segments_contact(d, d, R2);
            assert_eq!(result, (2.0 + offset) * (2.0 + offset) <= R2);
        }
    }

    #[test]
    fn tunneling_pass_through_detected() {
        // Straight through the origin in one step: both endpoints far
        // outside contact, interior minimum at zero. This is the case a
        // discrete end-of-step overlap test cannot see — the test that pins
        // the sweep against a future "simplification".
        let d0 = Vector::new(-100.0, 0.1, 0.0);
        let d1 = Vector::new(100.0, 0.1, 0.0);
        assert!(d0.length_squared() > R2 && d1.length_squared() > R2);
        assert!(segments_contact(d0, d1, R2));
    }

    #[test]
    fn grazing_interior_minimum_resolves_by_impact_parameter() {
        // Impact parameter just inside vs. just outside contact distance.
        let inside = Vector::new(-100.0, 2.0 - 1e-9, 0.0);
        assert!(segments_contact(
            inside,
            Vector::new(100.0, 2.0 - 1e-9, 0.0),
            R2
        ));
        let outside = Vector::new(-100.0, 2.0 + 1e-9, 0.0);
        assert!(!segments_contact(
            outside,
            Vector::new(100.0, 2.0 + 1e-9, 0.0),
            R2
        ));
    }

    #[test]
    fn endpoint_minimum_uses_endpoint_not_line() {
        // The infinite line through the segment passes within contact
        // distance, but the segment ends before reaching it.
        let d0 = Vector::new(10.0, 0.5, 0.0);
        let d1 = Vector::new(5.0, 0.5, 0.0);
        assert!(!segments_contact(d0, d1, R2));
        // Approaching case: closest at end of step, inside contact.
        let d1_close = Vector::new(1.0, 0.5, 0.0);
        assert!(segments_contact(d0, d1_close, R2));
    }

    #[test]
    fn merge_conserves_momentum_and_mass_weighted_position() {
        let states = [
            (2.0, Vector::new(1.0, 2.0, 3.0), Vector::new(-1.0, 0.5, 0.0)),
            (
                5.0,
                Vector::new(-2.0, 0.0, 1.0),
                Vector::new(2.0, -1.0, 3.0),
            ),
            (
                0.5,
                Vector::new(700.0, -300.0, 100.0),
                Vector::new(0.0, 8.0, -2.0),
            ),
        ];
        let mut merged = MergedState {
            mass: states[0].0,
            position: states[0].1,
            velocity: states[0].2,
        };
        for &(m, p, v) in &states[1..] {
            merged = accumulate(merged, m, p, v);
        }

        let total_mass: Scalar = states.iter().map(|s| s.0).sum();
        let momentum: Vector = states.iter().map(|s| s.2 * s.0).sum();
        let weighted_position: Vector = states.iter().map(|s| s.1 * s.0).sum();

        assert_eq!(merged.mass, total_mass);
        assert!((merged.velocity * merged.mass - momentum).length() < 1e-12 * momentum.length());
        assert!(
            (merged.position * merged.mass - weighted_position).length()
                < 1e-12 * weighted_position.length()
        );
    }

    #[test]
    fn mass_radius_invariant_roundtrips() {
        use crate::physics::math::mass_for_radius;
        let r1 = 3.0;
        let r2 = 4.0;
        let merged_mass = mass_for_radius(r1) + mass_for_radius(r2);
        let merged_radius = radius_for_mass(merged_mass);
        // Volume conservation and density conservation coincide under one
        // global density: r' = cbrt(r1³ + r2³).
        let expected = (r1 * r1 * r1 + r2 * r2 * r2).cbrt();
        assert!((merged_radius - expected).abs() < 1e-12);
    }
}
