pub mod recursive;

use avian3d::math::Scalar;
use avian3d::math::Vector;

use crate::config;
use crate::physics;

#[derive(Debug, Clone, Copy)]
pub struct OctreeBody {
    pub position: Vector,
    pub mass: Scalar,
}

#[derive(Debug, Clone)]
pub struct OctreeStats {
    pub node_count: usize,
    pub body_count: usize,
    pub total_mass: Scalar,
    pub center_of_mass: Vector,
    pub force_calculation_count: u64,
}

pub trait Octree {
    fn new(theta: Scalar, min_distance: Scalar, max_force: Scalar) -> Self
    where
        Self: Sized;
    fn with_leaf_threshold(self, leaf_threshold: usize) -> Self
    where
        Self: Sized;
    fn pool_stats(&self) -> (usize, usize);
    fn octree_stats(&self) -> OctreeStats;
    fn clear_pool(&mut self);
    fn get_bounds(&self, max_depth: Option<usize>) -> Vec<physics::aabb3d::Aabb3d>;
    fn build(&mut self, bodies: Vec<OctreeBody>);
    fn calculate_force_on_body(&self, body: &OctreeBody, g: Scalar) -> Vector;
}

pub fn create_octree(
    implementation: config::OctreeImplementation,
    theta: Scalar,
    min_distance: Scalar,
    max_force: Scalar,
    leaf_threshold: usize,
) -> Box<dyn Octree + Send + Sync> {
    match implementation {
        config::OctreeImplementation::Recursive => Box::new(
            recursive::Octree::new(theta, min_distance, max_force)
                .with_leaf_threshold(leaf_threshold),
        ),
    }
}
