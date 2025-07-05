//! Spatial octree for approximating gravitational forces using the Barnes-Hut algorithm.

use avian3d::math::Scalar;
use avian3d::math::Vector;
use bevy::prelude::*;
use std::collections::VecDeque;

// TODO: pool diagnostics?

#[derive(Debug)]
pub struct OctreeNodePool {
    internal_nodes: VecDeque<Box<[Option<OctreeNode>; 8]>>,
    external_bodies: VecDeque<Vec<OctreeBody>>,
}

impl Default for OctreeNodePool {
    fn default() -> Self {
        Self::new()
    }
}

impl OctreeNodePool {
    pub fn new() -> Self {
        Self {
            internal_nodes: VecDeque::new(),
            external_bodies: VecDeque::new(),
        }
    }

    pub fn with_capacity(internal_capacity: usize, external_capacity: usize) -> Self {
        Self {
            internal_nodes: VecDeque::with_capacity(internal_capacity),
            external_bodies: VecDeque::with_capacity(external_capacity),
        }
    }

    pub fn get_internal_children(&mut self) -> Box<[Option<OctreeNode>; 8]> {
        self.internal_nodes
            .pop_front()
            .unwrap_or_else(|| Box::new([None, None, None, None, None, None, None, None]))
    }

    pub fn get_external_bodies(&mut self, capacity: usize) -> Vec<OctreeBody> {
        if let Some(mut bodies) = self.external_bodies.pop_front() {
            bodies.clear();
            bodies.reserve(capacity);
            bodies
        } else {
            Vec::with_capacity(capacity)
        }
    }

    pub fn return_internal_children(&mut self, mut children: Box<[Option<OctreeNode>; 8]>) {
        for child in children.iter_mut() {
            if let Some(node) = child.take() {
                self.return_node(node);
            }
        }

        self.internal_nodes.push_back(children);
    }

    pub fn return_external_bodies(&mut self, mut bodies: Vec<OctreeBody>) {
        bodies.clear();
        self.external_bodies.push_back(bodies);
    }

    pub fn return_node(&mut self, node: OctreeNode) {
        match node {
            OctreeNode::Internal { children, .. } => {
                self.return_internal_children(children);
            }
            OctreeNode::External { bodies, .. } => {
                self.return_external_bodies(bodies);
            }
        }
    }

    pub fn clear(&mut self) {
        self.internal_nodes.clear();
        self.external_bodies.clear();
    }

    pub fn stats(&self) -> (usize, usize) {
        (self.internal_nodes.len(), self.external_bodies.len())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Aabb3d {
    pub min: Vector,
    pub max: Vector,
}

impl Aabb3d {
    pub fn new(min: Vector, max: Vector) -> Self {
        Self { min, max }
    }

    #[inline]
    pub fn center(&self) -> Vector {
        (self.min + self.max) * 0.5
    }

    #[inline]
    pub fn size(&self) -> Vector {
        self.max - self.min
    }

    pub fn subdivide_into_children(&self) -> [Aabb3d; 8] {
        let center = self.center();
        [
            Aabb3d::new(self.min, center),
            Aabb3d::new(
                Vector::new(center.x, self.min.y, self.min.z),
                Vector::new(self.max.x, center.y, center.z),
            ),
            Aabb3d::new(
                Vector::new(self.min.x, center.y, self.min.z),
                Vector::new(center.x, self.max.y, center.z),
            ),
            Aabb3d::new(
                Vector::new(center.x, center.y, self.min.z),
                Vector::new(self.max.x, self.max.y, center.z),
            ),
            Aabb3d::new(
                Vector::new(self.min.x, self.min.y, center.z),
                Vector::new(center.x, center.y, self.max.z),
            ),
            Aabb3d::new(
                Vector::new(center.x, self.min.y, center.z),
                Vector::new(self.max.x, center.y, self.max.z),
            ),
            Aabb3d::new(
                Vector::new(self.min.x, center.y, center.z),
                Vector::new(center.x, self.max.y, self.max.z),
            ),
            Aabb3d::new(center, self.max),
        ]
    }
}

#[derive(Debug)]
pub struct Octree {
    pub root: Option<OctreeNode>,
    pub theta: Scalar,                // Barnes-Hut approximation parameter
    pub min_distance: Scalar,         // Minimum distance for force calculation
    pub max_force: Scalar,            // Maximum force magnitude
    pub leaf_threshold: usize,        // Maximum bodies per leaf node
    min_distance_squared: Scalar,     // Cached value to avoid repeated multiplication
    octree_node_pool: OctreeNodePool, // Pool for reusing node allocations
}

impl Octree {
    pub fn new(theta: Scalar, min_distance: Scalar, max_force: Scalar) -> Self {
        Self {
            root: None,
            theta,
            min_distance,
            max_force,
            leaf_threshold: 4,
            min_distance_squared: min_distance * min_distance,
            octree_node_pool: OctreeNodePool::new(),
        }
    }

    pub fn with_leaf_threshold(mut self, leaf_threshold: usize) -> Self {
        self.leaf_threshold = leaf_threshold;
        self
    }

    pub fn with_pool_capacity(
        theta: Scalar,
        min_distance: Scalar,
        max_force: Scalar,
        internal_capacity: usize,
        external_capacity: usize,
    ) -> Self {
        Self {
            root: None,
            theta,
            min_distance,
            max_force,
            leaf_threshold: 4,
            min_distance_squared: min_distance * min_distance,
            octree_node_pool: OctreeNodePool::with_capacity(internal_capacity, external_capacity),
        }
    }

    pub fn pool_stats(&self) -> (usize, usize) {
        self.octree_node_pool.stats()
    }

    pub fn clear_pool(&mut self) {
        self.octree_node_pool.clear();
    }

    pub fn get_bounds(&self, max_depth: Option<usize>) -> Vec<Aabb3d> {
        // Estimate capacity based on max_depth (8^depth nodes at each level)
        let estimated_capacity = match max_depth {
            Some(depth) => (0..=depth)
                .map(|d| 8_usize.pow(d as u32))
                .sum::<usize>()
                .min(1024),
            None => 64, // Conservative estimate for unbounded depth
        };
        let mut bounds = Vec::with_capacity(estimated_capacity);
        if let Some(root) = &self.root {
            self.collect_bounds(root, &mut bounds, 0, max_depth);
        }
        bounds
    }

    #[allow(clippy::only_used_in_recursion)]
    fn collect_bounds(
        &self,
        node: &OctreeNode,
        bounds: &mut Vec<Aabb3d>,
        current_depth: usize,
        max_depth: Option<usize>,
    ) {
        if let Some(max_depth) = max_depth {
            if current_depth > max_depth {
                return;
            }
        }

        bounds.push(node.bounds());

        if let OctreeNode::Internal { children, .. } = node {
            children.iter().flatten().for_each(|child| {
                self.collect_bounds(child, bounds, current_depth + 1, max_depth);
            });
        }
    }

    pub fn build(&mut self, bodies: impl IntoIterator<Item = OctreeBody>) {
        if let Some(old_root) = self.root.take() {
            self.octree_node_pool.return_node(old_root);
        }

        let mut bodies_iter = bodies.into_iter();

        let first_body = match bodies_iter.next() {
            Some(body) => body,
            None => {
                self.root = None;
                return;
            }
        };

        let mut min = first_body.position;
        let mut max = first_body.position;
        // Pre-allocate with estimated capacity based on size hint
        let estimated_capacity = bodies_iter.size_hint().0.max(1) + 1;
        let mut bodies_vec = Vec::with_capacity(estimated_capacity);
        bodies_vec.push(first_body);

        bodies_iter.for_each(|body| {
            min.x = min.x.min(body.position.x);
            min.y = min.y.min(body.position.y);
            min.z = min.z.min(body.position.z);
            max.x = max.x.max(body.position.x);
            max.y = max.y.max(body.position.y);
            max.z = max.z.max(body.position.z);
            bodies_vec.push(body);
        });

        let padding = (max - min) * 0.1;
        min -= padding;
        max += padding;

        let bounds = Aabb3d::new(min, max);
        self.root = Some(Self::build_node(
            bounds,
            bodies_vec,
            self.leaf_threshold,
            &mut self.octree_node_pool,
        ));
    }

    fn build_node(
        bounds: Aabb3d,
        bodies: Vec<OctreeBody>,
        leaf_threshold: usize,
        pool: &mut OctreeNodePool,
    ) -> OctreeNode {
        if bodies.len() <= leaf_threshold {
            let pooled_bodies = pool.get_external_bodies(bodies.len());
            let mut external_bodies = pooled_bodies;

            external_bodies.extend(bodies);

            return OctreeNode::External {
                bounds,
                bodies: external_bodies,
            };
        }

        let center = bounds.center();
        let octants = bounds.subdivide_into_children();

        // Count bodies per octant first for better allocation
        let mut octant_counts = [0usize; 8];
        bodies.iter().for_each(|body| {
            let octant_index = Self::get_octant_index(body.position, center);
            octant_counts[octant_index] += 1;
        });

        // Create vectors with exact capacity for non-empty octants using pool
        let mut octant_bodies: [Vec<OctreeBody>; 8] = [
            pool.get_external_bodies(octant_counts[0]),
            pool.get_external_bodies(octant_counts[1]),
            pool.get_external_bodies(octant_counts[2]),
            pool.get_external_bodies(octant_counts[3]),
            pool.get_external_bodies(octant_counts[4]),
            pool.get_external_bodies(octant_counts[5]),
            pool.get_external_bodies(octant_counts[6]),
            pool.get_external_bodies(octant_counts[7]),
        ];

        let mut children = pool.get_internal_children();

        bodies.iter().for_each(|body| {
            let octant_index = Self::get_octant_index(body.position, center);
            octant_bodies[octant_index].push(*body);
        });

        for (i, bodies_in_octant) in octant_bodies.into_iter().enumerate() {
            if !bodies_in_octant.is_empty() {
                children[i] = Some(Self::build_node(
                    octants[i],
                    bodies_in_octant,
                    leaf_threshold,
                    pool,
                ));
            } else {
                pool.return_external_bodies(bodies_in_octant);
            }
        }

        let (total_mass, weighted_sum) = bodies
            .iter()
            .fold((0.0, Vector::ZERO), |(mass_acc, pos_acc), body| {
                (mass_acc + body.mass, pos_acc + body.position * body.mass)
            });
        let center_of_mass = if total_mass > 0.0 {
            weighted_sum / total_mass
        } else {
            bounds.center()
        };

        OctreeNode::Internal {
            bounds,
            center_of_mass,
            total_mass,
            children,
        }
    }

    #[inline]
    fn get_octant_index(position: Vector, center: Vector) -> usize {
        ((position.x > center.x) as usize)
            | (((position.y > center.y) as usize) << 1)
            | (((position.z > center.z) as usize) << 2)
    }

    pub fn calculate_force(
        &self,
        body: &OctreeBody,
        node: Option<&OctreeNode>,
        g: Scalar,
    ) -> Vector {
        match node {
            Some(OctreeNode::Internal {
                bounds,
                center_of_mass,
                total_mass,
                children,
                ..
            }) => {
                let distance = (body.position - *center_of_mass).length();
                let size = bounds.size().length();

                // Barnes-Hut criterion: if s/d < theta, treat as single body
                if size / distance < self.theta {
                    self.calculate_force_from_point(body, *center_of_mass, *total_mass, g)
                } else {
                    let mut force = Vector::ZERO;
                    children.iter().for_each(|child| {
                        force += self.calculate_force(body, child.as_ref(), g);
                    });
                    force
                }
            }
            Some(OctreeNode::External { bodies, .. }) => {
                let mut force = Vector::ZERO;
                bodies.iter().for_each(|other_body| {
                    if other_body.entity != body.entity {
                        force += self.calculate_direct_force(body, other_body, g);
                    }
                });
                force
            }
            None => Vector::ZERO,
        }
    }

    #[inline]
    fn calculate_force_from_point(
        &self,
        body: &OctreeBody,
        point_position: Vector,
        point_mass: Scalar,
        g: Scalar,
    ) -> Vector {
        let direction = point_position - body.position;
        let distance_squared = direction.length_squared();

        if distance_squared < self.min_distance_squared {
            return Vector::ZERO;
        }

        let distance = distance_squared.sqrt();
        let direction_normalized = direction / distance;
        let force_magnitude = g * body.mass * point_mass / distance_squared;
        let force_magnitude = force_magnitude.min(self.max_force);

        direction_normalized * force_magnitude
    }

    #[inline]
    fn calculate_direct_force(&self, body1: &OctreeBody, body2: &OctreeBody, g: Scalar) -> Vector {
        self.calculate_force_from_point(body1, body2.position, body2.mass, g)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OctreeBody {
    pub entity: Entity,
    pub position: Vector,
    pub mass: Scalar,
}

#[derive(Debug)]
pub enum OctreeNode {
    Internal {
        bounds: Aabb3d,
        center_of_mass: Vector,
        total_mass: Scalar,
        children: Box<[Option<OctreeNode>; 8]>,
    },
    External {
        bounds: Aabb3d,
        bodies: Vec<OctreeBody>,
    },
}

impl OctreeNode {
    pub fn bounds(&self) -> Aabb3d {
        match self {
            OctreeNode::Internal { bounds, .. } => *bounds,
            OctreeNode::External { bounds, .. } => *bounds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::Entity;

    #[test]
    fn test_octree_force_calculation() {
        let mut octree = Octree::new(0.5, 10.0, 1e4);

        let body1 = OctreeBody {
            entity: Entity::from_raw(0),
            position: Vector::new(0.0, 0.0, 0.0),
            mass: 1000.0,
        };

        let body2 = OctreeBody {
            entity: Entity::from_raw(1),
            position: Vector::new(10.0, 0.0, 0.0),
            mass: 1000.0,
        };

        octree.build(vec![body1, body2]);

        // Calculate force on body1 from the octree
        let force = octree.calculate_force(&body1, octree.root.as_ref(), 1000.0);

        // The force should be non-zero and pointing towards body2 (positive x direction)
        assert!(force.length() > 0.0, "Force should be non-zero");
        assert!(
            force.x > 0.0,
            "Force should point towards body2 (positive x direction)"
        );
    }

    #[test]
    fn test_octree_boundary_handling() {
        let mut octree = Octree::new(0.5, 10.0, 1e4);

        // Create a body exactly at the center (boundary of all octants)
        let center_body = OctreeBody {
            entity: Entity::from_raw(0),
            position: Vector::new(0.0, 0.0, 0.0),
            mass: 1000.0,
        };

        // Create bodies in different octants
        let body1 = OctreeBody {
            entity: Entity::from_raw(1),
            position: Vector::new(-1.0, -1.0, -1.0),
            mass: 1000.0,
        };

        let body2 = OctreeBody {
            entity: Entity::from_raw(2),
            position: Vector::new(1.0, 1.0, 1.0),
            mass: 1000.0,
        };

        // Build octree with these bodies
        octree.build(vec![center_body, body1, body2]);

        // The octree should be built successfully without infinite recursion
        assert!(octree.root.is_some());

        // Calculate force on center body - should not be zero due to other bodies
        let force = octree.calculate_force(&center_body, octree.root.as_ref(), 1000.0);

        // Force should be finite (not NaN or infinite)
        assert!(force.is_finite(), "Force should be finite");
    }

    #[test]
    fn test_octree_no_body_duplication() {
        let mut octree = Octree::new(0.5, 10.0, 1e4);

        // Create bodies, including one exactly on octant boundary
        let bodies = vec![
            OctreeBody {
                entity: Entity::from_raw(0),
                position: Vector::new(0.0, 0.0, 0.0), // Exactly at center
                mass: 1000.0,
            },
            OctreeBody {
                entity: Entity::from_raw(1),
                position: Vector::new(-2.0, -2.0, -2.0),
                mass: 1000.0,
            },
            OctreeBody {
                entity: Entity::from_raw(2),
                position: Vector::new(2.0, 2.0, 2.0),
                mass: 1000.0,
            },
        ];

        octree.build(bodies.clone());

        // Count total bodies in the octree
        let total_bodies_in_octree = count_bodies_in_node(octree.root.as_ref());

        // Should equal the number of input bodies (no duplication)
        assert_eq!(
            total_bodies_in_octree,
            bodies.len(),
            "Number of bodies in octree should match input bodies"
        );
    }

    #[test]
    fn test_node_pool_basic_functionality() {
        let mut pool = OctreeNodePool::new();

        // Test getting and returning internal children
        let children1 = pool.get_internal_children();
        let children2 = pool.get_internal_children();

        // Initially pool should be empty
        assert_eq!(pool.stats(), (0, 0));

        // Return one children array
        pool.return_internal_children(children1);
        assert_eq!(pool.stats(), (1, 0));

        // Get it back - should reuse the returned one
        let children3 = pool.get_internal_children();
        assert_eq!(pool.stats(), (0, 0));

        // Test external bodies
        let bodies1 = pool.get_external_bodies(10);
        let bodies2 = pool.get_external_bodies(5);

        pool.return_external_bodies(bodies1);
        assert_eq!(pool.stats(), (0, 1));

        let bodies3 = pool.get_external_bodies(15);
        assert_eq!(pool.stats(), (0, 0));

        // Clean up
        pool.return_internal_children(children2);
        pool.return_internal_children(children3);
        pool.return_external_bodies(bodies2);
        pool.return_external_bodies(bodies3);
    }

    #[test]
    fn test_octree_pool_integration() {
        let mut octree = Octree::with_pool_capacity(0.5, 10.0, 1e4, 10, 10).with_leaf_threshold(1); // Force tree creation with small leaf threshold

        // Initially pool should be empty
        assert_eq!(octree.pool_stats(), (0, 0));

        // Create enough bodies to force tree structure creation
        let bodies = vec![
            OctreeBody {
                entity: Entity::from_raw(0),
                position: Vector::new(-5.0, -5.0, -5.0),
                mass: 1000.0,
            },
            OctreeBody {
                entity: Entity::from_raw(1),
                position: Vector::new(5.0, 5.0, 5.0),
                mass: 1000.0,
            },
            OctreeBody {
                entity: Entity::from_raw(2),
                position: Vector::new(-5.0, 5.0, -5.0),
                mass: 1000.0,
            },
            OctreeBody {
                entity: Entity::from_raw(3),
                position: Vector::new(5.0, -5.0, 5.0),
                mass: 1000.0,
            },
            OctreeBody {
                entity: Entity::from_raw(4),
                position: Vector::new(-5.0, -5.0, 5.0),
                mass: 1000.0,
            },
            OctreeBody {
                entity: Entity::from_raw(5),
                position: Vector::new(5.0, 5.0, -5.0),
                mass: 1000.0,
            },
        ];

        // Build the octree
        octree.build(bodies.clone());

        // Pool should still be empty (nodes are in use)
        assert_eq!(octree.pool_stats(), (0, 0));

        // Build again with fewer bodies - should return old nodes to pool
        let new_bodies = vec![
            OctreeBody {
                entity: Entity::from_raw(6),
                position: Vector::new(0.0, 0.0, 0.0),
                mass: 1000.0,
            },
            OctreeBody {
                entity: Entity::from_raw(7),
                position: Vector::new(1.0, 1.0, 1.0),
                mass: 1000.0,
            },
        ];

        octree.build(new_bodies);

        // Pool should now have some returned nodes
        let (internal_count, external_count) = octree.pool_stats();
        assert!(
            internal_count > 0 || external_count > 0,
            "Pool should have some returned nodes"
        );

        // Build again - should reuse nodes from pool
        octree.build(bodies);

        // Verify the octree still works correctly
        assert!(octree.root.is_some());
        let total_bodies = count_bodies_in_node(octree.root.as_ref());
        assert_eq!(total_bodies, 6);
    }

    #[test]
    fn test_pool_clear_functionality() {
        let mut octree = Octree::with_pool_capacity(0.5, 10.0, 1e4, 5, 5);

        // Build and rebuild to populate the pool
        let bodies = vec![
            OctreeBody {
                entity: Entity::from_raw(0),
                position: Vector::new(-1.0, -1.0, -1.0),
                mass: 1000.0,
            },
            OctreeBody {
                entity: Entity::from_raw(1),
                position: Vector::new(1.0, 1.0, 1.0),
                mass: 1000.0,
            },
        ];

        octree.build(bodies.clone());
        octree.build(vec![]); // Empty build to return nodes to pool

        // Pool should have some nodes
        let (internal_count, external_count) = octree.pool_stats();
        assert!(internal_count > 0 || external_count > 0);

        // Clear the pool
        octree.clear_pool();
        assert_eq!(octree.pool_stats(), (0, 0));

        // Should still work after clearing
        octree.build(bodies);
        assert!(octree.root.is_some());
    }

    #[test]
    fn test_pool_with_capacity() {
        let octree = Octree::with_pool_capacity(0.5, 10.0, 1e4, 20, 30);
        assert_eq!(octree.pool_stats(), (0, 0)); // Should start empty but have capacity

        // Test that it has the same functionality as regular octree
        assert_eq!(octree.theta, 0.5);
        assert_eq!(octree.min_distance, 10.0);
        assert_eq!(octree.max_force, 1e4);
    }

    fn count_bodies_in_node(node: Option<&OctreeNode>) -> usize {
        match node {
            Some(OctreeNode::External { bodies, .. }) => bodies.len(),
            Some(OctreeNode::Internal { children, .. }) => children
                .iter()
                .map(|child| count_bodies_in_node(child.as_ref()))
                .sum(),
            None => 0,
        }
    }
}
