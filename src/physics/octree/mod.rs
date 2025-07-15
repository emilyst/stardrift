pub mod recursive;

use super::aabb3d::Aabb3d;
use avian3d::math::Scalar;
use avian3d::math::Vector;

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
    fn new(theta: Scalar, min_distance: Scalar, max_force: Scalar) -> Self;
    fn with_leaf_threshold(self, leaf_threshold: usize) -> Self;
    fn pool_stats(&self) -> (usize, usize);
    fn octree_stats(&self) -> OctreeStats;
    fn clear_pool(&mut self);
    fn get_bounds(&self, max_depth: Option<usize>) -> Vec<Aabb3d>;
    fn build(&mut self, bodies: impl IntoIterator<Item = OctreeBody>);
    fn calculate_force_on_body(&self, body: &OctreeBody, g: Scalar) -> Vector;
}
