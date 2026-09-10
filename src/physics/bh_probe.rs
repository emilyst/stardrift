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
    /// `snapshot[i]`. The reference divides the summed force by the body's
    /// mass, as the driver does, so at theta = 0 the two agree to roundoff.
    pub fn observe(
        &mut self,
        octree: &Octree,
        snapshot: &[OctreeBody],
        bh_accels: &[Vector],
        g: Scalar,
    ) -> BhSample {
        assert_eq!(snapshot.len(), bh_accels.len());
        self.reference_accelerations(octree, snapshot, g);

        let (mut err_sq_sum, mut ref_sq_sum) = (0.0, 0.0);
        for (bh, reference) in bh_accels.iter().zip(&self.reference_accels) {
            err_sq_sum += (*bh - *reference).length_squared();
            ref_sq_sum += reference.length_squared();
        }
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
