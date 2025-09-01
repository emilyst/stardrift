use crate::physics::aabb3d::Aabb3d;
use crate::physics::math::{Scalar, Vector};
use bevy::prelude::Entity;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy)]
pub struct OctreeBody {
    pub position: Vector,
    pub mass: Scalar,
    pub entity: Entity,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OctreeStats {
    pub node_count: usize,
    pub body_count: usize,
    pub total_mass: Scalar,
    pub center_of_mass: Vector,
    pub force_calculation_count: u64,
}

#[derive(Debug)]
pub struct OctreeNodePool {
    internal_nodes: Vec<[Option<Box<OctreeNode>>; 8]>,
    external_bodies: Vec<Vec<OctreeBody>>,
}

impl Default for OctreeNodePool {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl OctreeNodePool {
    pub fn new() -> Self {
        Self {
            internal_nodes: Vec::new(),
            external_bodies: Vec::new(),
        }
    }

    pub fn with_capacity(internal_capacity: usize, external_capacity: usize) -> Self {
        Self {
            internal_nodes: Vec::with_capacity(internal_capacity),
            external_bodies: Vec::with_capacity(external_capacity),
        }
    }

    pub fn get_internal_children(&mut self) -> [Option<Box<OctreeNode>>; 8] {
        self.internal_nodes
            .pop()
            .unwrap_or([None, None, None, None, None, None, None, None])
    }

    pub fn get_external_bodies(&mut self, capacity: usize) -> Vec<OctreeBody> {
        if let Some(mut bodies) = self.external_bodies.pop() {
            bodies.clear();
            bodies.reserve(capacity);
            bodies
        } else {
            Vec::with_capacity(capacity)
        }
    }

    pub fn return_internal_children(&mut self, mut children: [Option<Box<OctreeNode>>; 8]) {
        for child in children.iter_mut() {
            if let Some(node) = child.take() {
                self.return_node(*node);
            }
        }

        self.internal_nodes.push(children);
    }

    pub fn return_external_bodies(&mut self, mut bodies: Vec<OctreeBody>) {
        bodies.clear();
        self.external_bodies.push(bodies);
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

#[derive(Debug)]
pub struct Octree {
    pub root: Option<OctreeNode>,
    pub theta: Scalar, // Barnes-Hut approximation parameter
    #[allow(dead_code)]
    pub min_distance: Scalar, // Minimum distance for force calculation
    pub max_force: Scalar, // Maximum force magnitude
    pub leaf_threshold: usize, // Maximum bodies per leaf node
    min_distance_squared: Scalar, // Cached value to avoid repeated multiplication
    node_pool: OctreeNodePool, // Pool for reusing node allocations
    force_calculation_count: AtomicU64, // Counter for force calculations performed
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
            node_pool: OctreeNodePool::new(),
            force_calculation_count: AtomicU64::new(0),
        }
    }

    pub fn with_leaf_threshold(mut self, leaf_threshold: usize) -> Self {
        self.leaf_threshold = leaf_threshold;
        self
    }

    #[allow(dead_code)]
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
            node_pool: OctreeNodePool::with_capacity(internal_capacity, external_capacity),
            force_calculation_count: AtomicU64::new(0),
        }
    }

    #[allow(dead_code)]
    pub fn node_pool_stats(&self) -> (usize, usize) {
        self.node_pool.stats()
    }

    #[allow(dead_code)]
    pub fn octree_stats(&self) -> OctreeStats {
        match &self.root {
            Some(root) => OctreeStats {
                node_count: root.count_nodes(),
                body_count: root.count_bodies(),
                total_mass: root.total_mass(),
                center_of_mass: root.center_of_mass(),
                force_calculation_count: self.force_calculation_count.load(Ordering::Relaxed),
            },
            None => OctreeStats {
                node_count: 0,
                body_count: 0,
                total_mass: 0.0,
                center_of_mass: Vector::ZERO,
                force_calculation_count: 0,
            },
        }
    }

    #[allow(dead_code)]
    pub fn clear_node_pool(&mut self) {
        self.node_pool.clear();
    }

    pub fn bounds(&self) -> Vec<Aabb3d> {
        let mut bounds = Vec::with_capacity(64);
        if let Some(root) = &self.root {
            root.collect_bounds(&mut bounds);
        }
        bounds
    }

    pub fn build(&mut self, bodies: impl IntoIterator<Item = OctreeBody>) {
        if let Some(old_root) = self.root.take() {
            self.node_pool.return_node(old_root);
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
            &mut self.node_pool,
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
        let octants = bounds.octants();

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
                children[i] = Some(Box::new(Self::build_node(
                    octants[i],
                    bodies_in_octant,
                    leaf_threshold,
                    pool,
                )));
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

        self.force_calculation_count.fetch_add(1, Ordering::Relaxed);

        let distance = distance_squared.sqrt();
        let direction_normalized = direction / distance;
        let force_magnitude = g * body.mass * point_mass / distance_squared;
        let force_magnitude = force_magnitude.min(self.max_force);

        direction_normalized * force_magnitude
    }

    /// Calculate force at an arbitrary position, excluding a specific entity
    ///
    /// This method allows integrators to evaluate forces at intermediate positions
    /// during multi-stage integration. It excludes the specified entity to avoid
    /// self-interaction when calculating forces for a body at a different position.
    ///
    /// # Arguments
    /// * `position` - The position at which to evaluate the force
    /// * `mass` - The mass of the body for which force is being calculated
    /// * `exclude_entity` - Entity to exclude from force calculation (typically the body itself)
    /// * `g` - Gravitational constant
    ///
    /// # Returns
    /// The force vector at the given position
    pub fn calculate_force_at_position(
        &self,
        position: Vector,
        mass: Scalar,
        exclude_entity: Entity,
        g: Scalar,
    ) -> Vector {
        let temp_body = OctreeBody {
            position,
            mass,
            entity: exclude_entity,
        };
        self.traverse_tree_for_force(&temp_body, self.root.as_ref(), g)
    }

    /// Recursively traverse the octree to calculate forces, excluding a specific entity
    fn traverse_tree_for_force(
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
                let distance_squared = body.position.distance_squared(*center_of_mass);
                let size_squared = bounds.min.distance_squared(bounds.max);

                // Barnes-Hut criterion: if s/d < theta, treat as single body
                if size_squared < distance_squared * self.theta * self.theta {
                    self.calculate_force_from_point(body, *center_of_mass, *total_mass, g)
                } else {
                    let mut force = Vector::ZERO;
                    children.iter().for_each(|child| {
                        force +=
                            self.traverse_tree_for_force(body, child.as_ref().map(|v| &**v), g);
                    });
                    force
                }
            }
            Some(OctreeNode::External { bodies, .. }) => {
                let mut force = Vector::ZERO;
                bodies.iter().for_each(|other_body| {
                    // Exclude the specified entity from force calculation
                    if other_body.entity != body.entity {
                        force += self.calculate_force_from_point(
                            body,
                            other_body.position,
                            other_body.mass,
                            g,
                        );
                    }
                });
                force
            }
            None => Vector::ZERO,
        }
    }
}

#[derive(Debug)]
pub enum OctreeNode {
    Internal {
        bounds: Aabb3d,
        center_of_mass: Vector,
        total_mass: Scalar,
        children: [Option<Box<OctreeNode>>; 8],
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

    pub fn collect_bounds(&self, bounds: &mut Vec<Aabb3d>) {
        bounds.push(self.bounds());

        if let OctreeNode::Internal { children, .. } = self {
            children.iter().flatten().for_each(|child| {
                child.collect_bounds(bounds);
            });
        }
    }

    #[allow(dead_code)]
    pub fn count_bodies(&self) -> usize {
        match self {
            OctreeNode::External { bodies, .. } => bodies.len(),
            OctreeNode::Internal { children, .. } => children
                .iter()
                .flatten()
                .map(|child| child.count_bodies())
                .sum(),
        }
    }

    #[allow(dead_code)]
    pub fn count_nodes(&self) -> usize {
        match self {
            OctreeNode::External { .. } => 1,
            OctreeNode::Internal { children, .. } => {
                1 + children
                    .iter()
                    .flatten()
                    .map(|child| child.count_nodes())
                    .sum::<usize>()
            }
        }
    }

    #[allow(dead_code)]
    pub fn is_leaf(&self) -> bool {
        matches!(self, OctreeNode::External { .. })
    }

    #[allow(dead_code)]
    pub fn total_mass(&self) -> Scalar {
        match self {
            OctreeNode::Internal { total_mass, .. } => *total_mass,
            OctreeNode::External { bodies, .. } => bodies.iter().map(|b| b.mass).sum(),
        }
    }

    #[allow(dead_code)]
    pub fn center_of_mass(&self) -> Vector {
        match self {
            OctreeNode::Internal { center_of_mass, .. } => *center_of_mass,
            OctreeNode::External { bodies, .. } => {
                if bodies.is_empty() {
                    return Vector::ZERO;
                }

                let total_mass: Scalar = bodies.iter().map(|b| b.mass).sum();

                if total_mass > 0.0 {
                    bodies
                        .iter()
                        .map(|b| b.position * b.mass)
                        .fold(Vector::ZERO, |acc, pos| acc + pos)
                        / total_mass
                } else {
                    Vector::ZERO
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_octree_force_calculation() {
        let mut octree = Octree::new(0.5, 10.0, 1e4);

        let body1 = OctreeBody {
            position: Vector::new(0.0, 0.0, 0.0),
            mass: 1000.0,
            entity: Entity::from_raw(0),
        };

        let body2 = OctreeBody {
            position: Vector::new(10.0, 0.0, 0.0),
            mass: 1000.0,
            entity: Entity::from_raw(1),
        };

        octree.build(vec![body1, body2]);

        // Calculate force on body1 from the octree
        let force =
            octree.calculate_force_at_position(body1.position, body1.mass, body1.entity, 1000.0);

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
            position: Vector::new(0.0, 0.0, 0.0),
            mass: 1000.0,
            entity: Entity::from_raw(0),
        };

        // Create bodies in different octants
        let body1 = OctreeBody {
            position: Vector::new(-1.0, -1.0, -1.0),
            mass: 1000.0,
            entity: Entity::from_raw(1),
        };

        let body2 = OctreeBody {
            position: Vector::new(1.0, 1.0, 1.0),
            mass: 1000.0,
            entity: Entity::from_raw(2),
        };

        // Build octree with these bodies
        octree.build(vec![center_body, body1, body2]);

        // The octree should be built successfully without infinite recursion
        assert!(octree.root.is_some());

        // Calculate force on center body - should not be zero due to other bodies
        let force = octree.calculate_force_at_position(
            center_body.position,
            center_body.mass,
            center_body.entity,
            1000.0,
        );

        // Force should be finite (not NaN or infinite)
        assert!(force.is_finite(), "Force should be finite");
    }

    #[test]
    fn test_octree_no_body_duplication() {
        let mut octree = Octree::new(0.5, 10.0, 1e4);

        // Create bodies, including one exactly on octant boundary
        let bodies = vec![
            OctreeBody {
                position: Vector::new(0.0, 0.0, 0.0), // Exactly at center
                mass: 1000.0,
                entity: Entity::from_raw(0),
            },
            OctreeBody {
                position: Vector::new(-2.0, -2.0, -2.0),
                mass: 1000.0,
                entity: Entity::from_raw(1),
            },
            OctreeBody {
                position: Vector::new(2.0, 2.0, 2.0),
                mass: 1000.0,
                entity: Entity::from_raw(2),
            },
        ];

        octree.build(bodies.clone());

        // Count total bodies in the octree
        let total_bodies_in_octree = octree.root.as_ref().map_or(0, |node| node.count_bodies());

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

        // Resource cleanup
        pool.return_internal_children(children2);
        pool.return_internal_children(children3);
        pool.return_external_bodies(bodies2);
        pool.return_external_bodies(bodies3);
    }

    #[test]
    fn test_octree_pool_integration() {
        let mut octree = Octree::with_pool_capacity(0.5, 10.0, 1e4, 10, 10).with_leaf_threshold(1); // Force tree creation with small leaf threshold

        // Initially pool should be empty
        assert_eq!(octree.node_pool_stats(), (0, 0));

        // Create enough bodies to force tree structure creation
        let bodies = vec![
            OctreeBody {
                position: Vector::new(-5.0, -5.0, -5.0),
                mass: 1000.0,
                entity: Entity::from_raw(0),
            },
            OctreeBody {
                position: Vector::new(5.0, 5.0, 5.0),
                mass: 1000.0,
                entity: Entity::from_raw(1),
            },
            OctreeBody {
                position: Vector::new(-5.0, 5.0, -5.0),
                mass: 1000.0,
                entity: Entity::from_raw(2),
            },
            OctreeBody {
                position: Vector::new(5.0, -5.0, 5.0),
                mass: 1000.0,
                entity: Entity::from_raw(3),
            },
            OctreeBody {
                position: Vector::new(-5.0, -5.0, 5.0),
                mass: 1000.0,
                entity: Entity::from_raw(4),
            },
            OctreeBody {
                position: Vector::new(5.0, 5.0, -5.0),
                mass: 1000.0,
                entity: Entity::from_raw(5),
            },
        ];

        // Build the octree
        octree.build(bodies.clone());

        // Pool should still be empty (nodes are in use)
        assert_eq!(octree.node_pool_stats(), (0, 0));

        // Build again with fewer bodies - should return old nodes to pool
        let new_bodies = vec![
            OctreeBody {
                position: Vector::new(0.0, 0.0, 0.0),
                mass: 1000.0,
                entity: Entity::from_raw(0),
            },
            OctreeBody {
                position: Vector::new(1.0, 1.0, 1.0),
                mass: 1000.0,
                entity: Entity::from_raw(1),
            },
        ];

        octree.build(new_bodies);

        // Pool should now have some returned nodes
        let (internal_count, external_count) = octree.node_pool_stats();
        assert!(
            internal_count > 0 || external_count > 0,
            "Pool should have some returned nodes"
        );

        // Build again - should reuse nodes from pool
        octree.build(bodies);

        // Verify the octree still works correctly
        assert!(octree.root.is_some());
        let total_bodies = octree.root.as_ref().map_or(0, |node| node.count_bodies());
        assert_eq!(total_bodies, 6);
    }

    #[test]
    fn test_pool_clear_functionality() {
        let mut octree = Octree::with_pool_capacity(0.5, 10.0, 1e4, 5, 5);

        // Build and rebuild to populate the pool
        let bodies = vec![
            OctreeBody {
                position: Vector::new(-1.0, -1.0, -1.0),
                mass: 1000.0,
                entity: Entity::from_raw(0),
            },
            OctreeBody {
                position: Vector::new(1.0, 1.0, 1.0),
                mass: 1000.0,
                entity: Entity::from_raw(1),
            },
        ];

        octree.build(bodies.clone());
        octree.build(vec![]); // Empty build to return nodes to pool

        // Pool should have some nodes
        let (internal_count, external_count) = octree.node_pool_stats();
        assert!(internal_count > 0 || external_count > 0);

        // Clear the pool
        octree.clear_node_pool();
        assert_eq!(octree.node_pool_stats(), (0, 0));

        // Should still work after clearing
        octree.build(bodies);
        assert!(octree.root.is_some());
    }

    #[test]
    fn test_pool_with_capacity() {
        let octree = Octree::with_pool_capacity(0.5, 10.0, 1e4, 20, 30);
        assert_eq!(octree.node_pool_stats(), (0, 0)); // Should start empty but have capacity

        // Test that it has the same functionality as regular octree
        assert_eq!(octree.theta, 0.5);
        assert_eq!(octree.min_distance, 10.0);
        assert_eq!(octree.max_force, 1e4);
    }

    #[test]
    fn test_node_count_bodies() {
        let mut octree = Octree::new(0.5, 10.0, 1e4).with_leaf_threshold(2);

        // Test with single body (external node)
        let single_body = vec![OctreeBody {
            position: Vector::new(0.0, 0.0, 0.0),
            mass: 1000.0,
            entity: Entity::from_raw(2),
        }];

        octree.build(single_body);
        let root = octree.root.as_ref().unwrap();
        assert_eq!(root.count_bodies(), 1);

        // Test with multiple bodies that create internal nodes
        let multiple_bodies = vec![
            OctreeBody {
                position: Vector::new(-5.0, -5.0, -5.0),
                mass: 1000.0,
                entity: Entity::from_raw(3),
            },
            OctreeBody {
                position: Vector::new(5.0, 5.0, 5.0),
                mass: 1000.0,
                entity: Entity::from_raw(4),
            },
            OctreeBody {
                position: Vector::new(-5.0, 5.0, -5.0),
                mass: 1000.0,
                entity: Entity::from_raw(5),
            },
            OctreeBody {
                position: Vector::new(5.0, -5.0, 5.0),
                mass: 1000.0,
                entity: Entity::from_raw(6),
            },
        ];

        octree.build(multiple_bodies.clone());
        let root = octree.root.as_ref().unwrap();
        assert_eq!(root.count_bodies(), multiple_bodies.len());

        // Test empty octree
        octree.build(vec![]);
        assert!(octree.root.is_none());
    }

    #[test]
    fn test_node_is_leaf() {
        let mut octree = Octree::new(0.5, 10.0, 1e4).with_leaf_threshold(1);

        // Test external node (leaf)
        let single_body = vec![OctreeBody {
            position: Vector::new(0.0, 0.0, 0.0),
            mass: 1000.0,
            entity: Entity::from_raw(7),
        }];

        octree.build(single_body);
        let root = octree.root.as_ref().unwrap();
        assert!(root.is_leaf(), "Single body should create a leaf node");

        // Test internal node (not leaf)
        let multiple_bodies = vec![
            OctreeBody {
                position: Vector::new(-5.0, -5.0, -5.0),
                mass: 1000.0,
                entity: Entity::from_raw(8),
            },
            OctreeBody {
                position: Vector::new(5.0, 5.0, 5.0),
                mass: 1000.0,
                entity: Entity::from_raw(9),
            },
        ];

        octree.build(multiple_bodies);
        let root = octree.root.as_ref().unwrap();
        assert!(
            !root.is_leaf(),
            "Multiple bodies should create an internal node"
        );
    }

    #[test]
    fn test_node_total_mass() {
        let mut octree = Octree::new(0.5, 10.0, 1e4).with_leaf_threshold(2);

        // Test external node mass calculation
        let bodies_external = vec![
            OctreeBody {
                position: Vector::new(0.0, 0.0, 0.0),
                mass: 500.0,
                entity: Entity::from_raw(10),
            },
            OctreeBody {
                position: Vector::new(0.1, 0.1, 0.1),
                mass: 300.0,
                entity: Entity::from_raw(11),
            },
        ];

        octree.build(bodies_external.clone());
        let root = octree.root.as_ref().unwrap();
        let expected_mass: Scalar = bodies_external.iter().map(|b| b.mass).sum();
        assert_eq!(root.total_mass(), expected_mass);

        // Test internal node mass calculation
        let bodies_internal = vec![
            OctreeBody {
                position: Vector::new(-5.0, -5.0, -5.0),
                mass: 1000.0,
                entity: Entity::from_raw(12),
            },
            OctreeBody {
                position: Vector::new(5.0, 5.0, 5.0),
                mass: 2000.0,
                entity: Entity::from_raw(13),
            },
            OctreeBody {
                position: Vector::new(-5.0, 5.0, -5.0),
                mass: 1500.0,
                entity: Entity::from_raw(14),
            },
        ];

        octree.build(bodies_internal.clone());
        let root = octree.root.as_ref().unwrap();
        let expected_mass: Scalar = bodies_internal.iter().map(|b| b.mass).sum();
        assert_eq!(root.total_mass(), expected_mass);

        // Test with zero mass bodies
        let zero_mass_bodies = vec![
            OctreeBody {
                position: Vector::new(0.0, 0.0, 0.0),
                mass: 0.0,
                entity: Entity::from_raw(15),
            },
            OctreeBody {
                position: Vector::new(1.0, 1.0, 1.0),
                mass: 0.0,
                entity: Entity::from_raw(16),
            },
        ];

        octree.build(zero_mass_bodies);
        let root = octree.root.as_ref().unwrap();
        assert_eq!(root.total_mass(), 0.0);
    }

    #[test]
    fn test_node_center_of_mass() {
        let mut octree = Octree::new(0.5, 10.0, 1e4).with_leaf_threshold(2);

        // Test external node center of mass calculation
        let bodies_external = vec![
            OctreeBody {
                position: Vector::new(0.0, 0.0, 0.0),
                mass: 1000.0,
                entity: Entity::from_raw(17),
            },
            OctreeBody {
                position: Vector::new(2.0, 0.0, 0.0),
                mass: 1000.0,
                entity: Entity::from_raw(18),
            },
        ];

        octree.build(bodies_external.clone());
        let root = octree.root.as_ref().unwrap();
        let center_of_mass = root.center_of_mass();

        // Expected center of mass should be at (1.0, 0.0, 0.0) for equal masses
        assert!(
            (center_of_mass.x - 1.0).abs() < 1e-10,
            "X coordinate should be 1.0"
        );
        assert!(center_of_mass.y.abs() < 1e-10, "Y coordinate should be 0.0");
        assert!(center_of_mass.z.abs() < 1e-10, "Z coordinate should be 0.0");

        // Test internal node center of mass calculation
        let bodies_internal = vec![
            OctreeBody {
                position: Vector::new(-10.0, -10.0, -10.0),
                mass: 1000.0,
                entity: Entity::from_raw(19),
            },
            OctreeBody {
                position: Vector::new(10.0, 10.0, 10.0),
                mass: 1000.0,
                entity: Entity::from_raw(20),
            },
            OctreeBody {
                position: Vector::new(-10.0, 10.0, -10.0),
                mass: 2000.0,
                entity: Entity::from_raw(21),
            },
        ];

        octree.build(bodies_internal.clone());
        let root = octree.root.as_ref().unwrap();
        let center_of_mass = root.center_of_mass();

        // Verify center of mass is calculated correctly
        assert!(
            center_of_mass.is_finite(),
            "Center of mass should be finite"
        );

        // Test with empty external node
        let empty_bodies = vec![];
        octree.build(empty_bodies);
        assert!(octree.root.is_none());

        // Test with zero mass bodies
        let zero_mass_bodies = vec![
            OctreeBody {
                position: Vector::new(5.0, 5.0, 5.0),
                mass: 0.0,
                entity: Entity::from_raw(22),
            },
            OctreeBody {
                position: Vector::new(-5.0, -5.0, -5.0),
                mass: 0.0,
                entity: Entity::from_raw(23),
            },
        ];

        octree.build(zero_mass_bodies);
        let root = octree.root.as_ref().unwrap();
        let center_of_mass = root.center_of_mass();
        assert_eq!(
            center_of_mass,
            Vector::ZERO,
            "Zero mass should result in zero center of mass"
        );
    }

    #[test]
    fn test_node_collect_bounds() {
        let mut octree = Octree::new(0.5, 10.0, 1e4).with_leaf_threshold(1);

        // Create bodies that will force tree subdivision
        let bodies = vec![
            OctreeBody {
                position: Vector::new(-5.0, -5.0, -5.0),
                mass: 1000.0,
                entity: Entity::from_raw(24),
            },
            OctreeBody {
                position: Vector::new(5.0, 5.0, 5.0),
                mass: 1000.0,
                entity: Entity::from_raw(25),
            },
            OctreeBody {
                position: Vector::new(-5.0, 5.0, -5.0),
                mass: 1000.0,
                entity: Entity::from_raw(26),
            },
            OctreeBody {
                position: Vector::new(5.0, -5.0, 5.0),
                mass: 1000.0,
                entity: Entity::from_raw(27),
            },
        ];

        octree.build(bodies);
        let root = octree.root.as_ref().unwrap();

        // Test collecting bounds
        let mut bounds = Vec::new();
        root.collect_bounds(&mut bounds);
        assert!(
            !bounds.is_empty(),
            "Should collect at least the root bounds"
        );

        // Test that all bounds are valid
        for bound in &bounds {
            assert!(bound.min.x <= bound.max.x, "Bound min.x should be <= max.x");
            assert!(bound.min.y <= bound.max.y, "Bound min.y should be <= max.y");
            assert!(bound.min.z <= bound.max.z, "Bound min.z should be <= max.z");
        }

        // Test with single body (external node)
        let single_body = vec![OctreeBody {
            position: Vector::new(0.0, 0.0, 0.0),
            mass: 1000.0,
            entity: Entity::from_raw(28),
        }];

        octree.build(single_body);
        let root = octree.root.as_ref().unwrap();
        let mut single_bounds = Vec::new();
        root.collect_bounds(&mut single_bounds);
        assert_eq!(
            single_bounds.len(),
            1,
            "Single body should produce one bound"
        );
    }

    #[test]
    fn test_octree_get_bounds_integration() {
        let mut octree = Octree::new(0.5, 10.0, 1e4).with_leaf_threshold(1);

        // Test that octree.get_bounds() uses the moved collect_bounds method correctly
        let bodies = vec![
            OctreeBody {
                position: Vector::new(-3.0, -3.0, -3.0),
                mass: 1000.0,
                entity: Entity::from_raw(29),
            },
            OctreeBody {
                position: Vector::new(3.0, 3.0, 3.0),
                mass: 1000.0,
                entity: Entity::from_raw(30),
            },
        ];

        octree.build(bodies);

        // Test get_bounds returns all bounds
        let bounds = octree.bounds();
        assert!(!bounds.is_empty(), "Should return bounds");

        // Test with empty octree
        octree.build(vec![]);
        let empty_bounds = octree.bounds();
        assert!(
            empty_bounds.is_empty(),
            "Empty octree should return no bounds"
        );
    }

    #[test]
    fn test_octree_stats() {
        let mut octree = Octree::new(0.5, 1.0, 1e4);

        // Test empty octree stats
        let empty_stats = octree.octree_stats();
        assert_eq!(empty_stats.node_count, 0);
        assert_eq!(empty_stats.body_count, 0);
        assert_eq!(empty_stats.total_mass, 0.0);
        assert_eq!(empty_stats.center_of_mass, Vector::ZERO);
        assert_eq!(empty_stats.force_calculation_count, 0);

        // Create test bodies
        let bodies = vec![
            OctreeBody {
                position: Vector::new(0.0, 0.0, 0.0),
                mass: 100.0,
                entity: Entity::from_raw(31),
            },
            OctreeBody {
                position: Vector::new(10.0, 0.0, 0.0),
                mass: 200.0,
                entity: Entity::from_raw(32),
            },
            OctreeBody {
                position: Vector::new(0.0, 10.0, 0.0),
                mass: 300.0,
                entity: Entity::from_raw(33),
            },
        ];

        octree.build(bodies.clone());

        // Test populated octree stats
        let stats = octree.octree_stats();
        assert!(stats.node_count > 0, "Should have nodes");
        assert_eq!(stats.body_count, 3, "Should count all bodies");
        assert_eq!(stats.total_mass, 600.0, "Should sum all masses");

        // Center of mass should be weighted average
        let expected_com = (Vector::new(0.0, 0.0, 0.0) * 100.0
            + Vector::new(10.0, 0.0, 0.0) * 200.0
            + Vector::new(0.0, 10.0, 0.0) * 300.0)
            / 600.0;
        assert!(
            (stats.center_of_mass - expected_com).length() < 1e-10,
            "Center of mass should be correct"
        );

        // Test force calculation counter
        let initial_count = stats.force_calculation_count;
        let _force = octree.calculate_force_at_position(
            bodies[0].position,
            bodies[0].mass,
            bodies[0].entity,
            1.0,
        );
        let updated_stats = octree.octree_stats();
        assert!(
            updated_stats.force_calculation_count > initial_count,
            "Force calculation count should increase"
        );
    }

    #[test]
    fn test_count_nodes() {
        let mut octree = Octree::new(0.5, 1.0, 1e4).with_leaf_threshold(1);

        // Single body should create one external node
        let single_body = vec![OctreeBody {
            position: Vector::new(0.0, 0.0, 0.0),
            mass: 100.0,
            entity: Entity::from_raw(34),
        }];
        octree.build(single_body);
        let stats = octree.octree_stats();
        assert_eq!(stats.node_count, 1, "Single body should create one node");

        octree = Octree::new(0.5, 1.0, 1e4).with_leaf_threshold(1);

        // Multiple bodies spread out should create internal nodes (with leaf_threshold=1)
        let multiple_bodies = vec![
            OctreeBody {
                position: Vector::new(-10.0, -10.0, -10.0),
                mass: 100.0,
                entity: Entity::from_raw(35),
            },
            OctreeBody {
                position: Vector::new(10.0, 10.0, 10.0),
                mass: 100.0,
                entity: Entity::from_raw(36),
            },
            OctreeBody {
                position: Vector::new(-10.0, 10.0, -10.0),
                mass: 100.0,
                entity: Entity::from_raw(37),
            },
            OctreeBody {
                position: Vector::new(10.0, -10.0, 10.0),
                mass: 100.0,
                entity: Entity::from_raw(38),
            },
        ];
        octree.build(multiple_bodies);
        let stats = octree.octree_stats();

        assert_eq!(
            stats.node_count, 5,
            "Four bodies should create five nodes (including the root node)"
        );
    }

    #[test]
    fn test_gravitational_force_calculation_is_correct() {
        // THIS TEST VERIFIES WHETHER THE GRAVITATIONAL FORCE CALCULATION IS MATHEMATICALLY CORRECT

        // Newton's law of universal gravitation states:
        // F = G * m1 * m2 / r^2
        //
        // The acceleration experienced by m1 is:
        // a1 = F / m1 = G * m2 / r^2

        // Test setup: Two bodies
        let g: Scalar = 100.0; // Gravitational constant
        let m1: Scalar = 5.0; // Mass of body 1  
        let m2: Scalar = 3.0; // Mass of body 2
        let separation: Scalar = 10.0; // Distance between bodies

        // Create two bodies separated along the x-axis
        let body1 = OctreeBody {
            position: Vector::new(0.0, 0.0, 0.0),
            mass: m1,
            entity: Entity::from_raw(1),
        };

        let body2 = OctreeBody {
            position: Vector::new(separation, 0.0, 0.0),
            mass: m2,
            entity: Entity::from_raw(2),
        };

        // Build octree with both bodies
        let mut octree = Octree::new(
            0.0,  // theta: 0 forces exact calculation (no Barnes-Hut approximation)
            0.01, // min_distance
            1e10, // max_force
        );

        octree.build(vec![body1, body2]);

        // Calculate the force on body1 due to body2
        let calculated_force =
            octree.calculate_force_at_position(body1.position, body1.mass, body1.entity, g);

        // Expected force calculation according to Newton's law
        let r_squared = separation * separation;
        let expected_force_magnitude = g * m1 * m2 / r_squared;
        let expected_force = Vector::new(expected_force_magnitude, 0.0, 0.0);

        // Calculate the acceleration (what the integrators actually use)
        let calculated_acceleration = calculated_force / m1;
        let _expected_acceleration = expected_force / m1;
        let expected_acceleration_simplified = g * m2 / r_squared;

        // CRITICAL ASSERTION: Check if the calculation matches physics
        let force_error = (calculated_force - expected_force).length();
        let acceleration_error =
            (calculated_acceleration.length() - expected_acceleration_simplified).abs();

        // The force should match Newton's law within numerical precision
        // If this assertion fails, the gravitational calculation is WRONG
        assert!(
            force_error < 1e-6,
            "GRAVITATIONAL FORCE CALCULATION IS INCORRECT!\n\
             Calculated: {:?}\n\
             Expected: {:?}\n\
             Error: {}",
            calculated_force,
            expected_force,
            force_error
        );

        assert!(
            acceleration_error < 1e-6,
            "GRAVITATIONAL ACCELERATION IS INCORRECT!\n\
             Calculated: {}\n\
             Expected: {}\n\
             Error: {}",
            calculated_acceleration.length(),
            expected_acceleration_simplified,
            acceleration_error
        );
    }

    #[test]
    fn test_inverse_square_law() {
        // Test that force follows inverse square law
        let g: Scalar = 1.0;
        let m1: Scalar = 1.0;
        let m2: Scalar = 1.0;

        let distances = vec![1.0, 2.0, 4.0, 8.0];
        let mut forces = Vec::new();

        for distance in &distances {
            let body1 = OctreeBody {
                position: Vector::new(0.0, 0.0, 0.0),
                mass: m1,
                entity: Entity::from_raw(1),
            };

            let body2 = OctreeBody {
                position: Vector::new(*distance, 0.0, 0.0),
                mass: m2,
                entity: Entity::from_raw(2),
            };

            let mut octree = Octree::new(
                0.0,  // theta: 0 for exact calculation
                0.01, // min_distance
                1e10, // max_force
            );

            octree.build(vec![body1, body2]);

            let force =
                octree.calculate_force_at_position(body1.position, body1.mass, body1.entity, g);

            forces.push(force.length());
        }

        let first_force = forces[0];
        for (i, (distance, force)) in distances.iter().zip(forces.iter()).enumerate() {
            if i > 0 {
                // Each doubling of distance should quarter the force
                let expected_ratio = (distances[0] / distance).powi(2);
                let actual_ratio = force / first_force;
                let ratio_error = (actual_ratio - expected_ratio).abs();

                assert!(
                    ratio_error < 1e-6,
                    "Force does not follow inverse square law!\n\
                     At distance {}: expected ratio {}, got {}",
                    distance,
                    expected_ratio,
                    actual_ratio
                );
            }
        }
    }
}
