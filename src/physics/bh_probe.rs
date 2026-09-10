//! Barnes-Hut probe: measures what the approximation costs.
//!
//! Given the stage snapshot the production octree was built from and the
//! accelerations that tree produced, the probe evaluates exact pairwise
//! accelerations through the same force law ([`Octree::pairwise_force`], so
//! the clamps and self-exclusion are shared, not reimplemented) and reports:
//!
//! - **Acceleration error**: a scale-free L2 norm
//!   `sqrt(Σ|δa|² / Σ|a_ref|²)` (the standard treecode/FMM figure) and a
//!   per-body relative maximum with a floor at `1e-3·a_rms` so field nulls
//!   cannot dominate it.
//! - **Momentum asymmetry**: `|Σ m_i·a_bh_i| / Σ m_i·|a_ref_i|`. Exact
//!   pairwise forces cancel in pairs (Newton's third law), so the net force
//!   on the system is zero to roundoff. Barnes-Hut acceptance is evaluated
//!   from each body's own side, so body i may see j through a node's centre
//!   of mass while j sees i directly; the pair no longer cancels and the
//!   system feels a small net force. This is that force, normalised by the
//!   total magnitude of the exact forces.
//! - **Barycenter speed ratio**: `|Σ m_i·v_i| / M` over the mass-weighted
//!   RMS speed `sqrt(Σ m_i·|v_i|² / M)`. Bodies spawn in the
//!   centre-of-momentum frame and merges conserve momentum, so any
//!   barycenter velocity is the momentum asymmetry integrated over the run.
//!   Zero at theta = 0 up to roundoff; a restart resets it.
//! - **Topology churn**: the fraction of bodies, among those present in
//!   both this and the previous observation, whose root-to-leaf octant path
//!   changed. Because the octree build is translation-equivariant (see
//!   [`Octree::build`]), uniform drift does not register; relative motion,
//!   leaf splits caused by neighbours, and root-box shifts caused by hull
//!   bodies all do. The latter is the amplification the "quantized root
//!   box" idea in `docs/integration.md` targets, and this metric is its
//!   business case.
//!
//! The reference pass is O(N²); the driver rate-limits it. This module is
//! pure library code with no ECS dependencies beyond `Entity` as a key.

use crate::physics::math::{Scalar, Vector};
use crate::physics::octree::{Octree, OctreeBody};
use bevy::ecs::entity::EntityHashMap;
use bevy::tasks::{ComputeTaskPool, ParallelSliceMut};

/// Below this body count the reference pass runs sequentially (matches the
/// driver's threshold; small worlds and tests never touch the task pool).
const PARALLEL_REFERENCE_THRESHOLD: usize = 128;

/// Relative floor, as a fraction of the RMS exact acceleration, applied to
/// the per-body denominator of [`BhSample::accel_error_max`].
const RELATIVE_FLOOR: Scalar = 1e-3;

/// One observation of the Barnes-Hut field against exact summation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BhSample {
    /// `sqrt(Σ|a_bh − a_ref|² / Σ|a_ref|²)`; 0 when the field is identically zero
    pub accel_error_l2: Scalar,
    /// `max_i |a_bh_i − a_ref_i| / max(|a_ref_i|, 1e-3·a_rms)`
    pub accel_error_max: Scalar,
    /// `|Σ m_i·a_bh_i| / Σ m_i·|a_ref_i|` — net force on the system from
    /// the asymmetric acceptance, relative to the total exact force magnitude
    pub momentum_asymmetry: Scalar,
    /// Barycenter speed over the mass-weighted RMS body speed; the
    /// integrated momentum leak in a centre-of-momentum spawn frame
    pub barycenter_speed_ratio: Scalar,
    /// Fraction of surviving bodies whose octant path changed since the
    /// previous observation; `None` on the first observation or when no
    /// body survived from the previous one (restart)
    pub topology_churn: Option<Scalar>,
    /// Bodies in the observed snapshot
    pub bodies: usize,
}

/// Persistent probe state: reference buffers and the previous build's paths.
#[derive(Default)]
pub struct BhProbe {
    reference_accels: Vec<Vector>,
    prev_paths: EntityHashMap<u128>,
    curr_paths: EntityHashMap<u128>,
}

impl BhProbe {
    /// Observe one build.
    ///
    /// `octree` must have been built from exactly `snapshot`, and
    /// `bh_accels[i]` must be the acceleration that tree produced for
    /// `snapshot[i]`; `velocities[i]` is the body's current velocity. The
    /// reference divides the summed force by the body's mass, as the driver
    /// does, so at theta = 0 the two agree to roundoff.
    pub fn observe(
        &mut self,
        octree: &Octree,
        snapshot: &[OctreeBody],
        bh_accels: &[Vector],
        velocities: &[Vector],
        g: Scalar,
    ) -> BhSample {
        assert_eq!(snapshot.len(), bh_accels.len());
        assert_eq!(snapshot.len(), velocities.len());
        self.reference_accelerations(octree, snapshot, g);

        let (mut err_sq_sum, mut ref_sq_sum) = (0.0, 0.0);
        let (mut net_force_bh, mut total_force_ref) = (Vector::ZERO, 0.0);
        for ((bh, reference), body) in bh_accels.iter().zip(&self.reference_accels).zip(snapshot) {
            err_sq_sum += (*bh - *reference).length_squared();
            ref_sq_sum += reference.length_squared();
            net_force_bh += *bh * body.mass;
            total_force_ref += reference.length() * body.mass;
        }
        let momentum_asymmetry = if total_force_ref > 0.0 {
            net_force_bh.length() / total_force_ref
        } else {
            0.0
        };

        let (mut momentum, mut total_mass, mut twice_kinetic) = (Vector::ZERO, 0.0, 0.0);
        for (body, velocity) in snapshot.iter().zip(velocities) {
            momentum += *velocity * body.mass;
            total_mass += body.mass;
            twice_kinetic += body.mass * velocity.length_squared();
        }
        let barycenter_speed_ratio = if twice_kinetic > 0.0 {
            // |P|/M over sqrt(Σ m v² / M) = |P| / sqrt(M · Σ m v²)
            momentum.length() / (total_mass * twice_kinetic).sqrt()
        } else {
            0.0
        };
        let n = snapshot.len();
        let accel_error_l2 = if ref_sq_sum > 0.0 {
            (err_sq_sum / ref_sq_sum).sqrt()
        } else {
            0.0
        };
        let floor = if n > 0 {
            RELATIVE_FLOOR * (ref_sq_sum / n as Scalar).sqrt()
        } else {
            0.0
        };
        // Explicit fold so a NaN propagates instead of being dropped by f64::max
        let accel_error_max = bh_accels
            .iter()
            .zip(&self.reference_accels)
            .map(|(bh, reference)| {
                let denominator = reference.length().max(floor);
                if denominator > 0.0 {
                    (*bh - *reference).length() / denominator
                } else {
                    0.0
                }
            })
            .fold(
                0.0,
                |max: Scalar, e| if e > max || e.is_nan() { e } else { max },
            );

        let topology_churn = self.topology_churn(octree);

        BhSample {
            accel_error_l2,
            accel_error_max,
            momentum_asymmetry,
            barycenter_speed_ratio,
            topology_churn,
            bodies: n,
        }
    }

    fn reference_accelerations(&mut self, octree: &Octree, snapshot: &[OctreeBody], g: Scalar) {
        let n = snapshot.len();
        self.reference_accels.clear();
        self.reference_accels.resize(n, Vector::ZERO);

        let evaluate = |body: &OctreeBody| {
            // Inner sum stays sequential and in snapshot order: deterministic
            let force = snapshot
                .iter()
                .filter(|other| other.entity != body.entity)
                .fold(Vector::ZERO, |force, other| {
                    force + octree.pairwise_force(body, other.position, other.mass, g)
                });
            force / body.mass
        };

        match ComputeTaskPool::try_get() {
            Some(task_pool) if n >= PARALLEL_REFERENCE_THRESHOLD => {
                let chunk_size = (n / (task_pool.thread_num() * 4)).max(32);
                self.reference_accels.par_chunk_map_mut(
                    task_pool,
                    chunk_size,
                    |chunk_index, chunk| {
                        for (offset, accel) in chunk.iter_mut().enumerate() {
                            *accel = evaluate(&snapshot[chunk_index * chunk_size + offset]);
                        }
                    },
                );
            }
            _ => {
                for (accel, body) in self.reference_accels.iter_mut().zip(snapshot) {
                    *accel = evaluate(body);
                }
            }
        }
    }

    fn topology_churn(&mut self, octree: &Octree) -> Option<Scalar> {
        self.curr_paths.clear();
        octree.for_each_leaf_path(|key, bodies| {
            for body in bodies {
                self.curr_paths.insert(body.entity, key);
            }
        });

        let (mut common, mut changed) = (0usize, 0usize);
        for (entity, key) in &self.curr_paths {
            if let Some(prev) = self.prev_paths.get(entity) {
                common += 1;
                if prev != key {
                    changed += 1;
                }
            }
        }
        let had_previous = !self.prev_paths.is_empty();
        std::mem::swap(&mut self.prev_paths, &mut self.curr_paths);

        (had_previous && common > 0).then(|| changed as Scalar / common as Scalar)
    }
}
