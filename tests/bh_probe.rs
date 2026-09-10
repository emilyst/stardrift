//! Barnes-Hut probe tests: the exact-summation reference and the topology
//! churn metric in `physics::bh_probe`, exercised on the real octree.
//!
//! These pin the properties the probe's numbers depend on:
//! - at theta = 0 the production tree and the reference agree to roundoff
//!   (same pair terms, different summation order), including with both
//!   force clamps engaged — the reference shares the force law rather than
//!   reimplementing it;
//! - at theta > 0 the error is nonzero, bracketed, and grows with theta on a
//!   scene where Barnes-Hut actually approximates;
//! - the net Barnes-Hut force on the system vanishes at theta = 0 (pairwise
//!   forces cancel, clamps included) and does not at theta > 0;
//! - the barycenter speed ratio is 0 in the centre-of-momentum frame and 1
//!   for a uniformly boosted system;
//! - churn is 0 for an identical or uniformly translated rebuild (the
//!   translation-equivariance the FSAL cache and drift correction rely on),
//!   positive for a body crossing an octant boundary, and absent on the
//!   first observation or when no body survived.

use bevy::ecs::entity::Entity;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use stardrift::physics::bh_probe::{BhProbe, BhSample};
use stardrift::physics::math::{Scalar, Vector};
use stardrift::physics::octree::{Octree, OctreeBody};

const G: Scalar = 100.0;
const MIN_DISTANCE: Scalar = 1.0;
const MAX_FORCE: Scalar = 1e7;
const ROUNDOFF: Scalar = 1e-12;

fn entity(index: usize) -> Entity {
    Entity::from_raw_u32(index as u32).unwrap()
}

fn body(index: usize, position: Vector, mass: Scalar) -> OctreeBody {
    OctreeBody {
        position,
        mass,
        entity: entity(index),
    }
}

fn octree(theta: Scalar) -> Octree {
    Octree::new(theta, MIN_DISTANCE, MAX_FORCE).with_leaf_threshold(1)
}

/// Uniform ball of `count` bodies of radius `radius` around `center`, with
/// masses in the default body-mass range.
fn ball(
    rng: &mut ChaCha8Rng,
    first_index: usize,
    count: usize,
    center: Vector,
    radius: Scalar,
) -> Vec<OctreeBody> {
    (0..count)
        .map(|i| {
            let direction = Vector::new(
                rng.random_range(-1.0..1.0),
                rng.random_range(-1.0..1.0),
                rng.random_range(-1.0..1.0),
            )
            .normalize();
            let r = radius * rng.random_range(0.0..1.0_f64).cbrt();
            body(
                first_index + i,
                center + direction * r,
                rng.random_range(30.0..270.0),
            )
        })
        .collect()
}

/// Build the tree, evaluate the production field, and observe it with the
/// given velocities.
fn observe_with_velocities(
    probe: &mut BhProbe,
    octree: &mut Octree,
    snapshot: &[OctreeBody],
    velocities: &[Vector],
) -> BhSample {
    octree.build_from_slice(snapshot);
    let accels: Vec<Vector> = snapshot
        .iter()
        .map(|b| octree.calculate_force_at_position(b.position, b.mass, b.entity, G) / b.mass)
        .collect();
    probe.observe(octree, snapshot, &accels, velocities, G)
}

/// Build the tree, evaluate the production field, and observe it (bodies at
/// rest).
fn observe(probe: &mut BhProbe, octree: &mut Octree, snapshot: &[OctreeBody]) -> BhSample {
    let velocities = vec![Vector::ZERO; snapshot.len()];
    observe_with_velocities(probe, octree, snapshot, &velocities)
}

#[test]
fn exact_tree_matches_reference_to_roundoff() {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let snapshot = ball(&mut rng, 0, 64, Vector::ZERO, 1000.0);
    let sample = observe(&mut BhProbe::default(), &mut octree(0.0), &snapshot);
    assert_eq!(sample.bodies, 64);
    assert!(
        sample.accel_error_l2 < ROUNDOFF,
        "l2 {}",
        sample.accel_error_l2
    );
    assert!(
        sample.accel_error_max < ROUNDOFF,
        "max {}",
        sample.accel_error_max
    );
    assert!(
        sample.momentum_asymmetry < ROUNDOFF,
        "exact pairwise forces must cancel: {}",
        sample.momentum_asymmetry
    );
    assert_eq!(
        sample.topology_churn, None,
        "first observation has no churn"
    );
}

/// Both clamps engaged, theta = 0: still roundoff. A reference that
/// reimplemented the force law without the clamps, or mishandled
/// self-exclusion or the mass division, would fail here.
#[test]
fn reference_shares_clamp_semantics() {
    let min_distance = 2.0;
    let max_force = 1e4;
    let mut tree = Octree::new(0.0, min_distance, max_force).with_leaf_threshold(1);
    let mut rng = ChaCha8Rng::seed_from_u64(7);
    // Sparse background: far apart, no pair anywhere near either clamp
    let mut snapshot = ball(&mut rng, 0, 60, Vector::ZERO, 5000.0);
    // One pair inside the softening radius, light enough to stay under the
    // cap so only the ramp is exercised: G·5·5·0.6/8 ≈ 190 < 1e4
    snapshot.push(body(60, Vector::new(8000.0, 0.0, 0.0), 5.0));
    snapshot.push(body(
        61,
        Vector::new(8000.0 + 0.3 * min_distance, 0.0, 0.0),
        5.0,
    ));
    // One pair outside it whose inverse-square force exceeds the cap:
    // G·m·m/d² = 100·1e3·1e3/9 ≫ 1e4
    snapshot.push(body(62, Vector::new(-8000.0, 0.0, 0.0), 1e3));
    snapshot.push(body(63, Vector::new(-8000.0, 1.5 * min_distance, 0.0), 1e3));

    let sample = observe(&mut BhProbe::default(), &mut tree, &snapshot);
    assert!(
        sample.accel_error_l2 < ROUNDOFF,
        "l2 {}",
        sample.accel_error_l2
    );
    assert!(
        sample.accel_error_max < ROUNDOFF,
        "max {}",
        sample.accel_error_max
    );
    // The clamps are symmetric in the pair, so Newton's third law survives them
    assert!(
        sample.momentum_asymmetry < ROUNDOFF,
        "{}",
        sample.momentum_asymmetry
    );

    // Sanity that the scene really engages both clamps
    let capped = tree.pairwise_force(&snapshot[62], snapshot[63].position, snapshot[63].mass, G);
    assert_eq!(capped.length(), max_force);
    let inside = tree.pairwise_force(&snapshot[60], snapshot[61].position, snapshot[61].mass, G);
    let ramp = G * 5.0 * 5.0 * (0.3 * min_distance) / min_distance.powi(3);
    assert!((inside.length() - ramp).abs() < 1e-9 * ramp);
}

/// Two blobs ~3 radii apart: each body's cross-blob force is served by a
/// few coarse nodes at theta = 1 that must be opened at theta = 0.1.
#[test]
fn error_is_bracketed_and_grows_with_theta() {
    let mut rng = ChaCha8Rng::seed_from_u64(3);
    let mut snapshot = ball(&mut rng, 0, 128, Vector::new(-1500.0, 0.0, 0.0), 500.0);
    snapshot.extend(ball(
        &mut rng,
        128,
        128,
        Vector::new(1500.0, 0.0, 0.0),
        500.0,
    ));

    let err = |theta: Scalar| observe(&mut BhProbe::default(), &mut octree(theta), &snapshot);
    let coarse = err(1.0);
    let mid = err(0.5);
    let fine = err(0.1);

    assert!(
        coarse.accel_error_l2 > 1e-6 && coarse.accel_error_l2 < 1e-1,
        "theta=1 l2 {} outside bracket",
        coarse.accel_error_l2
    );
    assert!(mid.accel_error_l2 > 0.0);
    assert!(
        coarse.accel_error_l2 > fine.accel_error_l2,
        "l2 theta=1 {} should exceed theta=0.1 {}",
        coarse.accel_error_l2,
        fine.accel_error_l2
    );
    assert!(coarse.accel_error_max >= coarse.accel_error_l2);

    // Asymmetric acceptance leaves a net force on the system at theta > 0
    assert!(
        coarse.momentum_asymmetry > 1e-8 && coarse.momentum_asymmetry < 1e-1,
        "theta=1 net force {} outside bracket",
        coarse.momentum_asymmetry
    );
    assert!(coarse.momentum_asymmetry > fine.momentum_asymmetry);
}

#[test]
fn barycenter_speed_ratio_reads_the_momentum_frame() {
    let mut rng = ChaCha8Rng::seed_from_u64(21);
    let snapshot = ball(&mut rng, 0, 32, Vector::ZERO, 1000.0);
    let mut tree = octree(0.5);

    // Random velocities with the centre-of-momentum motion removed: ratio 0
    let mut velocities: Vec<Vector> = (0..32)
        .map(|_| {
            Vector::new(
                rng.random_range(-1.0..1.0),
                rng.random_range(-1.0..1.0),
                rng.random_range(-1.0..1.0),
            )
        })
        .collect();
    let total_mass: Scalar = snapshot.iter().map(|b| b.mass).sum();
    let com_velocity = snapshot
        .iter()
        .zip(&velocities)
        .fold(Vector::ZERO, |p, (b, v)| p + *v * b.mass)
        / total_mass;
    for v in &mut velocities {
        *v -= com_velocity;
    }
    let sample =
        observe_with_velocities(&mut BhProbe::default(), &mut tree, &snapshot, &velocities);
    assert!(
        sample.barycenter_speed_ratio < 1e-12,
        "{}",
        sample.barycenter_speed_ratio
    );

    // A uniform boost has all its kinetic energy in the barycenter: ratio 1
    let boosted = vec![Vector::new(3.0, -4.0, 0.0); 32];
    let sample = observe_with_velocities(&mut BhProbe::default(), &mut tree, &snapshot, &boosted);
    assert!(
        (sample.barycenter_speed_ratio - 1.0).abs() < 1e-12,
        "{}",
        sample.barycenter_speed_ratio
    );

    // Bodies at rest: 0, not NaN
    let at_rest = vec![Vector::ZERO; 32];
    let sample = observe_with_velocities(&mut BhProbe::default(), &mut tree, &snapshot, &at_rest);
    assert_eq!(sample.barycenter_speed_ratio, 0.0);
}

#[test]
fn churn_is_zero_for_identical_and_translated_rebuilds() {
    let mut rng = ChaCha8Rng::seed_from_u64(11);
    let snapshot = ball(&mut rng, 0, 256, Vector::ZERO, 1000.0);
    let mut probe = BhProbe::default();
    let mut tree = octree(0.5);

    assert_eq!(
        observe(&mut probe, &mut tree, &snapshot).topology_churn,
        None
    );
    assert_eq!(
        observe(&mut probe, &mut tree, &snapshot).topology_churn,
        Some(0.0)
    );

    let translated: Vec<OctreeBody> = snapshot
        .iter()
        .map(|b| OctreeBody {
            position: b.position + Vector::splat(1000.0),
            ..*b
        })
        .collect();
    assert_eq!(
        observe(&mut probe, &mut tree, &translated).topology_churn,
        Some(0.0),
        "uniform translation must not change tree topology"
    );
}

#[test]
fn churn_counts_a_body_crossing_an_octant_boundary() {
    // Four bodies in distinct octants of a cube; moving one across the
    // centre plane changes only its own path.
    let mut snapshot = vec![
        body(0, Vector::new(-100.0, -100.0, -100.0), 1.0),
        body(1, Vector::new(100.0, 100.0, 100.0), 1.0),
        body(2, Vector::new(-100.0, 100.0, -100.0), 1.0),
        body(3, Vector::new(40.0, -100.0, 100.0), 1.0),
    ];
    let mut probe = BhProbe::default();
    let mut tree = octree(0.5);
    observe(&mut probe, &mut tree, &snapshot);
    snapshot[3].position.x = -40.0;
    let sample = observe(&mut probe, &mut tree, &snapshot);
    assert_eq!(sample.topology_churn, Some(0.25));
}

#[test]
fn churn_is_absent_when_no_body_survives() {
    let mut rng = ChaCha8Rng::seed_from_u64(5);
    let first = ball(&mut rng, 0, 32, Vector::ZERO, 1000.0);
    let respawned = ball(&mut rng, 1000, 32, Vector::ZERO, 1000.0);
    let mut probe = BhProbe::default();
    let mut tree = octree(0.5);
    observe(&mut probe, &mut tree, &first);
    assert_eq!(
        observe(&mut probe, &mut tree, &respawned).topology_churn,
        None
    );
    // ...and resumes once a common set exists again
    assert_eq!(
        observe(&mut probe, &mut tree, &respawned).topology_churn,
        Some(0.0)
    );
}

#[test]
fn merged_bodies_are_excluded_from_churn() {
    let mut rng = ChaCha8Rng::seed_from_u64(9);
    let snapshot = ball(&mut rng, 0, 64, Vector::ZERO, 1000.0);
    let mut probe = BhProbe::default();
    let mut tree = octree(0.5);
    observe(&mut probe, &mut tree, &snapshot);
    // Half the bodies "merge away"; the survivors do not move
    let survivors: Vec<OctreeBody> = snapshot
        .iter()
        .copied()
        .enumerate()
        .filter(|(i, _)| i % 2 == 0)
        .map(|(_, b)| b)
        .collect();
    let sample = observe(&mut probe, &mut tree, &survivors);
    // Removing bodies can shift the root box and rekey survivors, so churn
    // may be nonzero — but it must be a fraction of the 32 survivors only.
    let churn = sample.topology_churn.expect("survivors form a common set");
    assert!((0.0..=1.0).contains(&churn));
    assert!(
        (churn * 32.0).fract().abs() < 1e-12,
        "churn {churn} is not k/32"
    );
}
