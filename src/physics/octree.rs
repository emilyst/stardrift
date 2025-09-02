use crate::physics::aabb3d::Aabb3d;
use crate::physics::math::{Scalar, Vector, VectorExt};
use bevy::prelude::Entity;
use std::sync::atomic::{AtomicU64, Ordering};

/// Maximum depth allowed for the octree to prevent stack overflow
/// and performance degradation. Depth of 24 provides spatial resolution
/// down to ~10^-7 of the root node size, which is sufficient for any
/// realistic simulation while preventing pathological cases.
const MAX_OCTREE_DEPTH: usize = 24;

/// Represents one of the eight octants in 3D space relative to a center point
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Octant {
    LeftBottomBack = 0b000,   // x-, y-, z-
    RightBottomBack = 0b001,  // x+, y-, z-
    LeftTopBack = 0b010,      // x-, y+, z-
    RightTopBack = 0b011,     // x+, y+, z-
    LeftBottomFront = 0b100,  // x-, y-, z+
    RightBottomFront = 0b101, // x+, y-, z+
    LeftTopFront = 0b110,     // x-, y+, z+
    RightTopFront = 0b111,    // x+, y+, z+
}

impl Octant {
    const ALL: [Octant; 8] = [
        Octant::LeftBottomBack,
        Octant::RightBottomBack,
        Octant::LeftTopBack,
        Octant::RightTopBack,
        Octant::LeftBottomFront,
        Octant::RightBottomFront,
        Octant::LeftTopFront,
        Octant::RightTopFront,
    ];

    /// Determines which octant a position falls into relative to a center point
    #[inline]
    fn from_position(position: Vector, center: Vector) -> Self {
        let index = ((position.x > center.x) as usize)
            | (((position.y > center.y) as usize) << 1)
            | (((position.z > center.z) as usize) << 2);

        Self::ALL[index] // Safe: index is guaranteed to be 0-7
    }

    /// Returns the array index (0-7) for this octant
    #[inline]
    fn index(self) -> usize {
        self as usize
    }
}

/// A body in the octree with position, mass, and entity identifier
///
/// The Entity field is required for self-exclusion during force calculations.
/// While position-based exclusion might seem simpler (comparing positions with ==),
/// it fails in practice because:
/// 1. Bodies need to exclude themselves when calculating forces
/// 2. During multi-stage integration, forces are evaluated at intermediate positions
/// 3. Floating-point precision means a body's query position may not exactly match its stored position
/// 4. Without reliable self-exclusion, bodies calculate forces on themselves, causing instability
///
/// The Entity provides a robust identifier that remains consistent regardless of
/// numerical precision or intermediate calculations.
#[derive(Debug, Clone, Copy)]
pub struct OctreeBody {
    pub position: Vector,
    pub mass: Scalar,
    pub entity: Entity,
}

/// Memory pool for efficient octree node allocation and reuse.
///
/// The octree is rebuilt every frame, which would normally cause many allocations.
/// This pool maintains collections of previously allocated nodes and body vectors,
/// allowing them to be reused across rebuilds to minimize allocation overhead.
///
/// # Performance Benefits
///
/// - Reduces heap allocations by ~90% after initial frames
/// - Improves cache locality by reusing memory
/// - Eliminates allocation/deallocation overhead during tree rebuilds
#[derive(Debug)]
pub struct OctreeNodePool {
    internal_nodes: Vec<[Option<Box<OctreeNode>>; 8]>, // Pool of child arrays for internal nodes
    external_bodies: Vec<Vec<OctreeBody>>,             // Pool of body vectors for leaf nodes
}

impl Default for OctreeNodePool {
    fn default() -> Self {
        Self::new()
    }
}

impl OctreeNodePool {
    pub fn new() -> Self {
        Self {
            internal_nodes: Vec::new(),
            external_bodies: Vec::new(),
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
        children
            .iter_mut()
            .filter_map(|child| child.take())
            .for_each(|node| self.return_node(*node));

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
}

/// An octree implementation for efficient N-body gravitational force calculations.
///
/// This structure implements the Barnes-Hut algorithm, which reduces the computational
/// complexity of N-body simulations from O(N²) to O(N log N) by grouping distant bodies
/// and treating them as single point masses.
///
/// # Key Parameters
///
/// * `theta` - Barnes-Hut approximation parameter (0.0 = exact, higher = faster but less accurate)
///   - 0.0: Exact N-body calculation, no approximation (slowest, perfect accuracy)
///   - 0.3: High accuracy, ~10% performance gain over exact
///   - 0.5: Good balance of accuracy and performance (recommended default)
///   - 0.7: Faster calculation, acceptable for visual simulations
///   - 1.0: Maximum approximation (fastest, accuracy sufficient for many visual effects)
/// * `min_distance` - Minimum distance for force calculations to prevent singularities
/// * `max_force` - Maximum force magnitude to maintain numerical stability
/// * `leaf_threshold` - Maximum bodies per leaf node before subdivision
///
/// # Performance Characteristics
///
/// * Tree construction: O(N log N)
/// * Force calculation per body: O(log N) average, O(N) worst case
/// * Memory usage: O(N) for bodies + O(N) for tree nodes
#[derive(Debug)]
pub struct Octree {
    pub root: Option<OctreeNode>,
    pub theta: Scalar,                  // Barnes-Hut approximation parameter
    pub min_distance: Scalar,           // Minimum distance for force calculation
    pub max_force: Scalar,              // Maximum force magnitude
    pub leaf_threshold: usize,          // Maximum bodies per leaf node
    min_distance_squared: Scalar,       // Cached value to avoid repeated multiplication
    node_pool: OctreeNodePool,          // Pool for reusing node allocations
    force_calculation_count: AtomicU64, // Counter for force calculations performed
}

impl Octree {
    /// Creates a new octree with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `theta` - Barnes-Hut approximation parameter:
    ///   - 0.0 for exact N-body calculations
    ///   - 0.5 for good accuracy/performance balance (recommended)
    ///   - 1.0 for maximum speed with acceptable visual accuracy
    /// * `min_distance` - Minimum distance between bodies to prevent force singularities.
    ///                    Forces are calculated as if bodies are at least this far apart.
    /// * `max_force` - Maximum allowed force magnitude. Forces exceeding this are clamped.
    ///
    /// # Example
    ///
    /// ```
    /// use stardrift::physics::octree::Octree;
    ///
    /// let octree = Octree::new(0.5, 0.01, 1e6);
    /// ```
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

    /// Sets the maximum number of bodies allowed in a leaf node before subdivision.
    ///
    /// Lower values create deeper trees with better spatial resolution but more overhead.
    /// Higher values create shallower trees with less overhead but coarser approximations.
    ///
    /// Default is 4, which provides a good balance for most simulations.
    ///
    /// # Arguments
    ///
    /// * `leaf_threshold` - Maximum bodies per leaf (typically 1-16)
    pub fn with_leaf_threshold(mut self, leaf_threshold: usize) -> Self {
        self.leaf_threshold = leaf_threshold;
        self
    }

    /// Returns the bounding boxes of all nodes in the octree.
    ///
    /// Useful for visualization and debugging purposes to see the spatial subdivision.
    ///
    /// # Returns
    ///
    /// A vector of axis-aligned bounding boxes (AABB) for all nodes in the tree.
    pub fn bounds(&self) -> Vec<Aabb3d> {
        let mut bounds = Vec::with_capacity(64);
        if let Some(root) = &self.root {
            root.collect_bounds(&mut bounds);
        }
        bounds
    }

    /// Builds the octree from a collection of bodies.
    ///
    /// This reconstructs the entire tree structure, reusing memory from the node pool
    /// when possible. The tree bounds are automatically computed from the body positions
    /// with 10% padding to ensure bodies near edges are properly contained.
    ///
    /// # Arguments
    ///
    /// * `bodies` - Iterator of bodies to insert into the tree
    ///
    /// # Performance
    ///
    /// O(N log N) time complexity for N bodies.
    /// Reuses node allocations from previous builds to minimize memory allocation.
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

        // Pre-allocate with estimated capacity based on size hint for efficiency
        let estimated_capacity = bodies_iter.size_hint().0.max(1) + 1;

        // Single pass to collect bodies and compute bounding box
        let (bodies_vec, min, max) = bodies_iter.fold(
            (
                {
                    let mut vec = Vec::with_capacity(estimated_capacity);
                    vec.push(first_body);
                    vec
                },
                first_body.position,
                first_body.position,
            ),
            |(mut bodies, min, max), body| {
                bodies.push(body);
                (
                    bodies,
                    min.component_min(body.position), // Track minimum on each axis
                    max.component_max(body.position), // Track maximum on each axis
                )
            },
        );

        // Add 10% padding to prevent bodies exactly on boundaries
        // This ensures numerical stability during octant assignment
        let padding = (max - min) * 0.1;
        let padded_min = min - padding;
        let padded_max = max + padding;
        let bounds = Aabb3d::new(padded_min, padded_max);
        self.root = Some(Self::build_node(
            bounds,
            bodies_vec,
            self.leaf_threshold,
            &mut self.node_pool,
            0, // Start at depth 0
        ));
    }

    fn build_node(
        bounds: Aabb3d,
        bodies: Vec<OctreeBody>,
        leaf_threshold: usize,
        pool: &mut OctreeNodePool,
        depth: usize,
    ) -> OctreeNode {
        if depth >= MAX_OCTREE_DEPTH || bodies.len() <= leaf_threshold {
            let pooled_bodies = pool.get_external_bodies(bodies.len());
            let mut external_bodies = pooled_bodies;

            external_bodies.extend(bodies);

            return OctreeNode::External {
                bounds,
                bodies: external_bodies,
            };
        }

        // Find center point and create 8 octant bounding boxes
        let center = bounds.center();
        let octants = bounds.octants();

        // First pass: count bodies per octant for optimal memory allocation
        let mut octant_counts = [0usize; 8];
        bodies.iter().for_each(|body| {
            let octant = Octant::from_position(body.position, center);
            octant_counts[octant.index()] += 1;
        });

        // Create vectors with exact capacity for each octant using the memory pool
        // This avoids reallocation during the distribution phase
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

        // Get a reusable array for child nodes from the pool
        let mut children = pool.get_internal_children();

        // Second pass: distribute bodies to their respective octants
        bodies.iter().for_each(|body| {
            let octant = Octant::from_position(body.position, center);
            octant_bodies[octant.index()].push(*body);
        });

        octant_bodies
            .into_iter()
            .enumerate()
            .for_each(|(i, bodies_in_octant)| {
                if !bodies_in_octant.is_empty() {
                    children[i] = Some(Box::new(Self::build_node(
                        octants[i],
                        bodies_in_octant,
                        leaf_threshold,
                        pool,
                        depth + 1, // Increment depth for child nodes
                    )));
                } else {
                    pool.return_external_bodies(bodies_in_octant);
                }
            });

        // Calculate aggregate properties for Barnes-Hut approximation
        // This allows treating this entire node as a single point mass when viewed from far away
        let (total_mass, weighted_sum) = bodies
            .iter()
            .fold((0.0, Vector::ZERO), |(mass_acc, pos_acc), body| {
                (mass_acc + body.mass, pos_acc + body.position * body.mass)
            });

        // Center of mass is the weighted average position
        // Handle edge case of zero total mass (shouldn't happen in practice)
        let center_of_mass = if total_mass > 0.0 {
            weighted_sum / total_mass
        } else {
            bounds.center()
        };

        // Debug assertions to verify invariants
        #[cfg(debug_assertions)]
        {
            // Total mass should be non-negative
            debug_assert!(
                total_mass >= 0.0,
                "Total mass must be non-negative, got {}",
                total_mass
            );

            // Center of mass should be within bounds (with small tolerance for numerical errors)
            if total_mass > 0.0 {
                let tolerance = (bounds.max - bounds.min).length() * 0.01;
                debug_assert!(
                    center_of_mass.x >= bounds.min.x - tolerance
                        && center_of_mass.x <= bounds.max.x + tolerance
                        && center_of_mass.y >= bounds.min.y - tolerance
                        && center_of_mass.y <= bounds.max.y + tolerance
                        && center_of_mass.z >= bounds.min.z - tolerance
                        && center_of_mass.z <= bounds.max.z + tolerance,
                    "Center of mass {:?} outside bounds {:?}",
                    center_of_mass,
                    bounds
                );
            }

            // At least one child should exist if we created an internal node
            debug_assert!(
                children.iter().any(|c| c.is_some()),
                "Internal node must have at least one child"
            );
        }

        OctreeNode::Internal {
            bounds,
            center_of_mass,
            total_mass,
            children,
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

        // Clamp distance to minimum to prevent singularities
        // This ensures forces remain finite but don't vanish
        let clamped_distance_squared = distance_squared.max(self.min_distance_squared);

        self.force_calculation_count.fetch_add(1, Ordering::Relaxed);

        let distance = clamped_distance_squared.sqrt();
        let direction_normalized = direction / distance;
        let force_magnitude = g * body.mass * point_mass / clamped_distance_squared;
        let force_magnitude = force_magnitude.min(self.max_force);

        direction_normalized * force_magnitude
    }

    /// Calculate force at an arbitrary position, excluding a specific entity.
    ///
    /// This method allows integrators to evaluate forces at intermediate positions
    /// during multi-stage integration. It excludes the specified entity to avoid
    /// self-interaction when calculating forces for a body at a different position.
    ///
    /// # Barnes-Hut Algorithm
    ///
    /// For each node, if s/d < theta (where s is node size and d is distance):
    /// - Treat the node as a single point mass at its center of mass
    /// Otherwise:
    /// - Recursively calculate forces from child nodes
    ///
    /// # Arguments
    ///
    /// * `position` - The position at which to evaluate the force
    /// * `mass` - The mass of the body for which force is being calculated
    /// * `exclude_entity` - Entity to exclude from force calculation (typically the body itself)
    /// * `g` - Gravitational constant
    ///
    /// # Returns
    ///
    /// The total force vector from all other bodies in the tree.
    ///
    /// # Performance
    ///
    /// O(log N) average case with Barnes-Hut approximation (theta > 0)
    /// O(N) worst case for exact calculation (theta = 0)
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

    /// Recursively traverses the octree to calculate forces using Barnes-Hut approximation.
    ///
    /// This is the core of the Barnes-Hut algorithm. For each node, it decides whether to:
    /// 1. Use the node's center of mass (if far enough away)
    /// 2. Recurse into child nodes (if too close for approximation)
    /// 3. Calculate exact forces from individual bodies (for leaf nodes)
    ///
    /// # Barnes-Hut Criterion
    ///
    /// A node can be treated as a single mass if: s/d < theta
    /// - s = size of the node (diagonal of bounding box)
    /// - d = distance from the body to the node's center of mass
    /// - theta = accuracy parameter (0 = exact, larger = more approximation)
    ///
    /// # Arguments
    ///
    /// * `body` - The body for which we're calculating forces
    /// * `node` - Current node being evaluated
    /// * `g` - Gravitational constant
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
                // Calculate distance from body to node's center of mass
                let distance_squared = body.position.distance_squared(*center_of_mass);

                // Calculate node size (diagonal of bounding box)
                let size_squared = bounds.min.distance_squared(bounds.max);

                // Barnes-Hut criterion: if s/d < theta, treat as single body
                // This is the key optimization - distant groups of bodies are treated as one
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

/// Represents a node in the octree, which can be either internal or external (leaf).
///
/// The octree uses this enum to distinguish between nodes that subdivide space
/// (internal) and nodes that contain actual bodies (external/leaf).
///
/// # Node Types
///
/// * `Internal` - A node that subdivides space into 8 octants
///   - Contains aggregated mass and center of mass for Barnes-Hut approximation
///   - Has up to 8 children (one per octant, may be None if octant is empty)
///
/// * `External` - A leaf node containing actual bodies
///   - Contains a small number of bodies (≤ leaf_threshold)
///   - No further subdivision occurs at this level
#[derive(Debug)]
pub enum OctreeNode {
    /// Internal node that subdivides space
    Internal {
        bounds: Aabb3d,                         // Spatial bounds of this node
        center_of_mass: Vector,                 // Weighted average position of all contained bodies
        total_mass: Scalar,                     // Sum of all contained body masses
        children: [Option<Box<OctreeNode>>; 8], // Child nodes for each octant
    },
    /// Leaf node containing actual bodies
    External {
        bounds: Aabb3d,          // Spatial bounds of this node
        bodies: Vec<OctreeBody>, // Bodies contained in this leaf
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_octant_enum() {
        // Test that octant indexing is correct
        let center = Vector::new(0.0, 0.0, 0.0);

        // Test all 8 octants
        let test_cases = vec![
            (Vector::new(-1.0, -1.0, -1.0), Octant::LeftBottomBack),
            (Vector::new(1.0, -1.0, -1.0), Octant::RightBottomBack),
            (Vector::new(-1.0, 1.0, -1.0), Octant::LeftTopBack),
            (Vector::new(1.0, 1.0, -1.0), Octant::RightTopBack),
            (Vector::new(-1.0, -1.0, 1.0), Octant::LeftBottomFront),
            (Vector::new(1.0, -1.0, 1.0), Octant::RightBottomFront),
            (Vector::new(-1.0, 1.0, 1.0), Octant::LeftTopFront),
            (Vector::new(1.0, 1.0, 1.0), Octant::RightTopFront),
        ];

        for (position, expected_octant) in test_cases {
            let octant = Octant::from_position(position, center);
            assert_eq!(
                octant, expected_octant,
                "Position {:?} should map to {:?}",
                position, expected_octant
            );

            // Verify index matches the enum's binary representation
            assert_eq!(
                octant.index(),
                expected_octant as usize,
                "Index should match enum value"
            );
        }

        // Test that indices are 0-7
        for i in 0..8 {
            assert_eq!(
                Octant::ALL[i].index(),
                i,
                "Octant at index {} should have index value {}",
                i,
                i
            );
        }

        // Test boundary case (exactly on center)
        let boundary_position = Vector::new(0.0, 0.0, 0.0);
        let octant = Octant::from_position(boundary_position, center);
        assert_eq!(
            octant,
            Octant::LeftBottomBack,
            "Position exactly at center should map to LeftBottomBack (all comparisons false)"
        );
    }

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

        let forces: Vec<Scalar> = distances
            .iter()
            .map(|distance| {
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

                force.length()
            })
            .collect();

        let first_force = forces[0];
        distances
            .iter()
            .zip(forces.iter())
            .enumerate()
            .skip(1) // Skip the first element (i=0)
            .for_each(|(_, (distance, force))| {
                // Each doubling of distance should quarter the force
                // Since force decreases with distance, we compare first_force/force
                let expected_ratio = (distance / distances[0]).powi(2);
                let actual_ratio = first_force / force;
                let ratio_error = (actual_ratio - expected_ratio).abs();

                assert!(
                    ratio_error < 1e-6,
                    "Force does not follow inverse square law!\n\
                     At distance {}: expected ratio {}, got {}",
                    distance,
                    expected_ratio,
                    actual_ratio
                );
            });
    }

    #[test]
    fn test_force_symmetry() {
        // Verify that F_ab = -F_ba for all body pairs (Newton's Third Law)
        // Use theta=0 to disable Barnes-Hut approximation and get exact forces
        let mut octree = Octree::new(0.0, 0.01, 1e6);

        let body_a = OctreeBody {
            position: Vector::new(0.0, 0.0, 0.0),
            mass: 100.0,
            entity: Entity::from_raw(1),
        };
        let body_b = OctreeBody {
            position: Vector::new(10.0, 0.0, 0.0),
            mass: 200.0,
            entity: Entity::from_raw(2),
        };

        octree.build(vec![body_a, body_b]);

        let force_on_a =
            octree.calculate_force_at_position(body_a.position, body_a.mass, body_a.entity, 1.0);
        let force_on_b =
            octree.calculate_force_at_position(body_b.position, body_b.mass, body_b.entity, 1.0);

        // With theta=0, forces should be exactly equal and opposite
        let sum_of_forces = force_on_a + force_on_b;

        // Check for exact symmetry (allowing only for floating-point precision)
        assert!(
            sum_of_forces.length() < 1e-12,
            "Force symmetry violated: F_a = {:?}, F_b = {:?}, sum = {:?}",
            force_on_a,
            force_on_b,
            sum_of_forces
        );

        // Also verify forces are in opposite directions with same magnitude
        let force_a_magnitude = force_on_a.length();
        let force_b_magnitude = force_on_b.length();
        assert!(
            (force_a_magnitude - force_b_magnitude).abs() < 1e-12,
            "Force magnitudes not equal: |F_a| = {}, |F_b| = {}",
            force_a_magnitude,
            force_b_magnitude
        );
    }

    #[test]
    fn test_minimum_distance_prevents_singularity() {
        // Verify that min_distance parameter prevents force singularities
        // when bodies are very close or coincident
        // Use theta=0 for exact force calculation without approximation
        let min_distance = 0.1;
        let mut octree = Octree::new(0.0, min_distance, 1e6);

        let body_a = OctreeBody {
            position: Vector::new(0.0, 0.0, 0.0),
            mass: 100.0,
            entity: Entity::from_raw(1),
        };

        // Place body_b extremely close, well below min_distance
        let body_b = OctreeBody {
            position: Vector::new(1e-10, 0.0, 0.0), // Practically coincident
            mass: 200.0,
            entity: Entity::from_raw(2),
        };

        octree.build(vec![body_a, body_b]);

        let force_on_a =
            octree.calculate_force_at_position(body_a.position, body_a.mass, body_a.entity, 1.0);

        // Force should be finite (not NaN or infinite)
        assert!(
            force_on_a.is_finite(),
            "Force should be finite, got: {:?}",
            force_on_a
        );

        // Calculate what the force would be if limited by min_distance
        let expected_force_magnitude =
            1.0 * body_a.mass * body_b.mass / (min_distance * min_distance);
        let actual_force_magnitude = force_on_a.length();

        // The actual force should not exceed what we'd get at min_distance
        // With theta=0 (exact calculation), we can use a tighter tolerance
        assert!(
            actual_force_magnitude <= expected_force_magnitude * 1.0001, // Tight tolerance for floating-point precision
            "Force exceeds min_distance limit: actual = {}, expected <= {}",
            actual_force_magnitude,
            expected_force_magnitude
        );
    }

    #[test]
    fn test_max_force_clamping() {
        // Verify that forces are actually clamped to max_force
        // Use theta=0 for exact force calculation without approximation
        let max_force = 1000.0;
        let mut octree = Octree::new(0.0, 0.01, max_force);

        let body_a = OctreeBody {
            position: Vector::new(0.0, 0.0, 0.0),
            mass: 1e10, // Very large mass
            entity: Entity::from_raw(1),
        };

        let body_b = OctreeBody {
            position: Vector::new(0.1, 0.0, 0.0), // Very close
            mass: 1e10,                           // Very large mass
            entity: Entity::from_raw(2),
        };

        octree.build(vec![body_a, body_b]);

        // With huge masses and tiny distance, unclamped force would be enormous
        let force_on_a =
            octree.calculate_force_at_position(body_a.position, body_a.mass, body_a.entity, 1.0);

        let force_magnitude = force_on_a.length();

        // Force magnitude should not exceed max_force
        assert!(
            force_magnitude <= max_force * 1.0001,
            "Force exceeds max_force: actual = {}, max = {}",
            force_magnitude,
            max_force
        );

        // Verify clamping is actually happening (force would be much larger without it)
        let unclamped_estimate = 1.0 * body_a.mass * body_b.mass / (0.1 * 0.1);
        assert!(
            unclamped_estimate > max_force * 100.0,
            "Test setup issue: unclamped force not large enough to test clamping"
        );
    }

    #[test]
    fn test_barnes_hut_theta_accuracy_tradeoff() {
        // Test that theta parameter correctly controls accuracy vs performance tradeoff
        // theta=0 should give exact N-body calculation
        // Higher theta values trade accuracy for speed

        // Create a simple 4-body system in a square configuration
        // This allows us to calculate exact forces analytically
        let bodies = vec![
            OctreeBody {
                position: Vector::new(-10.0, -10.0, 0.0),
                mass: 100.0,
                entity: Entity::from_raw(1),
            },
            OctreeBody {
                position: Vector::new(10.0, -10.0, 0.0),
                mass: 100.0,
                entity: Entity::from_raw(2),
            },
            OctreeBody {
                position: Vector::new(-10.0, 10.0, 0.0),
                mass: 100.0,
                entity: Entity::from_raw(3),
            },
            OctreeBody {
                position: Vector::new(10.0, 10.0, 0.0),
                mass: 100.0,
                entity: Entity::from_raw(4),
            },
        ];

        // Test point at origin - equidistant from all bodies
        let test_position = Vector::new(0.0, 0.0, 0.0);
        let test_mass = 50.0;
        let test_entity = Entity::from_raw(5);
        let g = 1.0;

        // Calculate exact force (all bodies contribute equally, forces cancel)
        // Due to symmetry, the exact force at origin should be zero
        let exact_force = Vector::ZERO;

        // Test different theta values
        let theta_values = [0.0, 0.3, 0.5, 0.7, 1.0];
        let mut errors = Vec::new();

        for &theta in &theta_values {
            let mut octree = Octree::new(theta, 0.01, 1e6);
            octree.build(bodies.clone());

            let force =
                octree.calculate_force_at_position(test_position, test_mass, test_entity, g);

            let error = (force - exact_force).length();
            errors.push((theta, error));

            // For perfect symmetry, even with approximation, force should be near zero
            assert!(
                error < 1e-10 || theta > 0.0,
                "Theta {} produced error {}: force = {:?}",
                theta,
                error,
                force
            );
        }

        // Verify that theta=0 gives exact result
        assert!(
            errors[0].1 < 1e-12,
            "Theta=0 should give exact result, but error was {}",
            errors[0].1
        );

        // Now test with an asymmetric point where forces don't cancel
        let test_position_2 = Vector::new(5.0, 5.0, 0.0);

        // Calculate exact force by summing contributions from all bodies
        let mut exact_force_2 = Vector::ZERO;
        for body in &bodies {
            let direction = body.position - test_position_2;
            let distance_squared = direction.length_squared();
            if distance_squared > 1e-10 {
                let distance = distance_squared.sqrt();
                let force_magnitude = g * test_mass * body.mass / distance_squared;
                exact_force_2 += (direction / distance) * force_magnitude;
            }
        }

        // Test accuracy degradation with increasing theta
        let mut relative_errors = Vec::new();
        for &theta in &theta_values {
            let mut octree = Octree::new(theta, 0.01, 1e6);
            octree.build(bodies.clone());

            let force =
                octree.calculate_force_at_position(test_position_2, test_mass, test_entity, g);

            let error = (force - exact_force_2).length();
            let relative_error = error / exact_force_2.length();
            relative_errors.push((theta, relative_error));

            // Even with theta=1.0, error should be reasonable (< 50%)
            assert!(
                relative_error < 0.5 || theta == 0.0,
                "Theta {} produced unacceptable relative error {}",
                theta,
                relative_error
            );
        }

        // Verify theta=0 gives near-exact result
        assert!(
            relative_errors[0].1 < 1e-10,
            "Theta=0 should give near-exact result, but relative error was {}",
            relative_errors[0].1
        );

        // Verify error increases with theta (allowing some noise)
        for i in 1..relative_errors.len() {
            if relative_errors[i].0 > relative_errors[i - 1].0 {
                // Generally, higher theta should have higher or similar error
                // We allow for some variation due to tree structure
                assert!(
                    relative_errors[i].1 >= relative_errors[i - 1].1 * 0.5,
                    "Error should generally increase with theta"
                );
            }
        }
    }

    #[test]
    fn test_multiple_bodies_same_position() {
        // Test that multiple bodies at identical positions are handled gracefully
        // This edge case can occur when bodies spawn at the same location
        let mut octree = Octree::new(0.0, 0.01, 1e6);

        let position = Vector::new(100.0, 200.0, 300.0);
        let bodies = vec![
            OctreeBody {
                position,
                mass: 50.0,
                entity: Entity::from_raw(1),
            },
            OctreeBody {
                position, // Exact same position
                mass: 75.0,
                entity: Entity::from_raw(2),
            },
            OctreeBody {
                position, // Yet another at same position
                mass: 100.0,
                entity: Entity::from_raw(3),
            },
        ];

        // Should build without panic or infinite recursion
        octree.build(bodies.clone());
        assert!(octree.root.is_some(), "Tree should be built");

        // Test force calculation from a nearby point
        let test_position = Vector::new(101.0, 200.0, 300.0);
        let force =
            octree.calculate_force_at_position(test_position, 10.0, Entity::from_raw(4), 1.0);

        // Force should be finite (no NaN or infinity)
        assert!(force.is_finite(), "Force should be finite: {:?}", force);

        // Force should point towards the coincident bodies
        let direction = position - test_position;
        let dot_product = force.normalize().dot(direction.normalize());
        assert!(
            dot_product > 0.99,
            "Force should point towards coincident bodies"
        );
    }

    #[test]
    fn test_extreme_mass_ratios() {
        // Test numerical stability with extreme mass ratios (>1e10)
        let mut octree = Octree::new(0.0, 0.01, 1e20);

        let bodies = vec![
            OctreeBody {
                position: Vector::new(0.0, 0.0, 0.0),
                mass: 1e-5, // Very small mass
                entity: Entity::from_raw(1),
            },
            OctreeBody {
                position: Vector::new(10.0, 0.0, 0.0),
                mass: 1e15, // Huge mass (ratio of 1e20)
                entity: Entity::from_raw(2),
            },
        ];

        octree.build(bodies);

        // Calculate force on small mass
        let force_on_small = octree.calculate_force_at_position(
            Vector::new(0.0, 0.0, 0.0),
            1e-5,
            Entity::from_raw(1),
            1.0,
        );

        // Calculate force on large mass
        let force_on_large = octree.calculate_force_at_position(
            Vector::new(10.0, 0.0, 0.0),
            1e15,
            Entity::from_raw(2),
            1.0,
        );

        // Both forces should be finite
        assert!(
            force_on_small.is_finite(),
            "Force on small mass should be finite: {:?}",
            force_on_small
        );
        assert!(
            force_on_large.is_finite(),
            "Force on large mass should be finite: {:?}",
            force_on_large
        );

        // Forces should still obey Newton's third law (within numerical precision)
        // Note: with extreme ratios, we allow more tolerance
        let sum = force_on_small + force_on_large;
        let relative_error = sum.length() / force_on_small.length().max(force_on_large.length());
        assert!(
            relative_error < 1e-6,
            "Forces should sum to near zero even with extreme mass ratios"
        );
    }

    #[test]
    fn test_extreme_distances() {
        // Test with very large and very small distances
        let mut octree = Octree::new(0.0, 1e-15, 1e30);

        // Test very large distances
        let bodies_far = vec![
            OctreeBody {
                position: Vector::new(-1e10, 0.0, 0.0),
                mass: 100.0,
                entity: Entity::from_raw(1),
            },
            OctreeBody {
                position: Vector::new(1e10, 0.0, 0.0),
                mass: 100.0,
                entity: Entity::from_raw(2),
            },
        ];

        octree.build(bodies_far);
        let force_far = octree.calculate_force_at_position(
            Vector::new(-1e10, 0.0, 0.0),
            100.0,
            Entity::from_raw(1),
            1.0,
        );

        assert!(
            force_far.is_finite(),
            "Force at extreme distance should be finite: {:?}",
            force_far
        );

        // Force should be very small but non-zero
        assert!(
            force_far.length() > 0.0 && force_far.length() < 1e-10,
            "Force at extreme distance should be tiny but non-zero: {}",
            force_far.length()
        );

        // Test very small distances (handled by min_distance)
        let bodies_close = vec![
            OctreeBody {
                position: Vector::new(0.0, 0.0, 0.0),
                mass: 100.0,
                entity: Entity::from_raw(3),
            },
            OctreeBody {
                position: Vector::new(1e-12, 0.0, 0.0), // Extremely close
                mass: 100.0,
                entity: Entity::from_raw(4),
            },
        ];

        let mut octree_close = Octree::new(0.0, 1e-10, 1e30); // min_distance prevents singularity
        octree_close.build(bodies_close);

        let force_close = octree_close.calculate_force_at_position(
            Vector::new(0.0, 0.0, 0.0),
            100.0,
            Entity::from_raw(3),
            1.0,
        );

        assert!(
            force_close.is_finite(),
            "Force at tiny distance should be finite: {:?}",
            force_close
        );
    }

    #[test]
    fn test_pathological_tree_depth() {
        // Create a configuration that would cause very deep tree without depth limits
        // Bodies positioned at exponentially decreasing separations
        let mut octree = Octree::new(0.0, 1e-15, 1e6);

        // Create bodies with exponentially decreasing separations
        // This forces subdivision at each level to separate them
        let mut bodies = Vec::new();

        for i in 0..30 {
            // Each body is exponentially closer to origin
            let position = 1.0 / (2.0_f64).powi(i);
            bodies.push(OctreeBody {
                position: Vector::new(position, 0.0, 0.0),
                mass: 100.0,
                entity: Entity::from_raw(i as u32),
            });
        }

        // Should build without stack overflow or excessive recursion
        octree.build(bodies.clone());
        assert!(octree.root.is_some(), "Tree should be built");

        // Test force calculation doesn't cause stack overflow during traversal
        let force = octree.calculate_force_at_position(
            Vector::new(2.0, 0.0, 0.0),
            50.0,
            Entity::from_raw(100),
            1.0,
        );

        assert!(force.is_finite(), "Force should be finite");
    }

    #[test]
    fn test_empty_octree() {
        // Verify force calculation on empty tree returns zero
        let mut octree = Octree::new(0.5, 0.01, 1e6);

        // Build with empty vector
        octree.build(vec![]);

        // Tree should have no root for empty input
        assert!(octree.root.is_none(), "Empty octree should have no root");

        // Force calculation should return zero
        let force = octree.calculate_force_at_position(
            Vector::new(0.0, 0.0, 0.0),
            100.0,
            Entity::from_raw(1),
            1.0,
        );

        assert_eq!(
            force,
            Vector::ZERO,
            "Force from empty octree should be zero"
        );

        // Should handle multiple queries without issues
        for i in 0..10 {
            let test_pos = Vector::new(i as f64, i as f64 * 2.0, i as f64 * 3.0);
            let f = octree.calculate_force_at_position(test_pos, 50.0, Entity::from_raw(i), 1.0);
            assert_eq!(f, Vector::ZERO, "All forces from empty tree should be zero");
        }
    }

    #[test]
    #[ignore] // Run with --ignored for performance tests
    fn test_octree_scales_n_log_n() {
        // Verify O(n log n) scaling for both construction and force calculation
        // Uses doubling experiments to measure scaling behavior

        use std::time::Instant;

        // Test with increasing body counts (doubling)
        let body_counts = [100, 200, 400, 800, 1600];
        let mut construction_times = Vec::new();
        let mut force_calc_times = Vec::new();

        for &n in &body_counts {
            // Create a clustered distribution that showcases octree optimization
            // We create 8 well-separated clusters, one in each octant
            // This allows Barnes-Hut to approximate distant clusters as point masses
            let bodies: Vec<OctreeBody> = (0..n)
                .map(|i| {
                    // Distribute bodies into 8 clusters
                    let cluster_id = i % 8;
                    let within_cluster_id = i / 8;

                    // Base position for each cluster (well separated)
                    let cluster_base = match cluster_id {
                        0 => Vector::new(-500.0, -500.0, -500.0),
                        1 => Vector::new(500.0, -500.0, -500.0),
                        2 => Vector::new(-500.0, 500.0, -500.0),
                        3 => Vector::new(500.0, 500.0, -500.0),
                        4 => Vector::new(-500.0, -500.0, 500.0),
                        5 => Vector::new(500.0, -500.0, 500.0),
                        6 => Vector::new(-500.0, 500.0, 500.0),
                        7 => Vector::new(500.0, 500.0, 500.0),
                        _ => Vector::ZERO,
                    };

                    // Add small offset within cluster (tightly packed)
                    let fi = within_cluster_id as f64;
                    let offset = Vector::new(
                        (fi * 0.1).sin() * 5.0,
                        (fi * 0.2).cos() * 5.0,
                        (fi * 0.3).sin() * 5.0,
                    );

                    OctreeBody {
                        position: cluster_base + offset,
                        mass: 100.0,
                        entity: Entity::from_raw(i as u32),
                    }
                })
                .collect();

            // Use theta=0.7 for better approximation with clustered distribution
            let mut octree = Octree::new(0.7, 0.01, 1e6).with_leaf_threshold(1);

            // Measure construction time
            let start = Instant::now();
            let iterations = 100;
            for _ in 0..iterations {
                octree.build(bodies.clone());
            }
            let construction_time = start.elapsed().as_secs_f64() / iterations as f64;
            construction_times.push((n, construction_time));

            // Build once for force calculation test
            octree.build(bodies.clone());

            // Measure force calculation time (for all bodies)
            let start = Instant::now();
            let force_iterations = 10;
            for _ in 0..force_iterations {
                for body in &bodies {
                    let _force = octree.calculate_force_at_position(
                        body.position,
                        body.mass,
                        body.entity,
                        1.0,
                    );
                }
            }
            let force_time = start.elapsed().as_secs_f64() / (force_iterations * n) as f64;
            force_calc_times.push((n, force_time * n as f64)); // Total time for all bodies
        }

        // Verify O(n log n) scaling
        // For O(n log n), doubling n should increase time by factor of ~2.2-2.4
        // (because 2n * log(2n) / (n * log(n)) ≈ 2 * (log(2n)/log(n)) ≈ 2.2 for large n)

        println!("\nConstruction Time Scaling:");
        for i in 1..construction_times.len() {
            let (n1, t1) = construction_times[i - 1];
            let (n2, t2) = construction_times[i];
            let actual_ratio = t2 / t1;
            let n_ratio = n2 as f64 / n1 as f64;
            let expected_ratio = n_ratio * (n2 as f64).ln() / (n1 as f64).ln();

            println!(
                "n: {} -> {}, time ratio: {:.3}, expected (n log n): {:.3}",
                n1, n2, actual_ratio, expected_ratio
            );

            // Allow 50% deviation from theoretical (due to cache effects, etc.)
            assert!(
                actual_ratio < expected_ratio * 1.5,
                "Construction scaling worse than O(n log n): {} vs {}",
                actual_ratio,
                expected_ratio
            );
        }

        println!("\nForce Calculation Scaling:");
        for i in 1..force_calc_times.len() {
            let (n1, t1) = force_calc_times[i - 1];
            let (n2, t2) = force_calc_times[i];
            let actual_ratio = t2 / t1;
            let n_ratio = n2 as f64 / n1 as f64;
            let expected_ratio = n_ratio * (n2 as f64).ln() / (n1 as f64).ln();

            println!(
                "n: {} -> {}, time ratio: {:.3}, expected (n log n): {:.3}",
                n1, n2, actual_ratio, expected_ratio
            );

            // With clustered distribution, we expect better O(n log n) scaling
            // Allow 50% deviation for cache effects and debug mode overhead
            assert!(
                actual_ratio < expected_ratio * 1.5,
                "Force calculation scaling worse than O(n log n): {} vs {}",
                actual_ratio,
                expected_ratio
            );
        }

        // Verify it's better than O(n²)
        // For O(n²), doubling n would increase time by factor of 4
        // We check that it's < 3.5 to confirm sub-quadratic scaling
        for i in 1..construction_times.len() {
            let ratio = construction_times[i].1 / construction_times[i - 1].1;
            assert!(
                ratio < 3.5,
                "Construction scaling looks like O(n²), ratio: {}",
                ratio
            );
        }

        for i in 1..force_calc_times.len() {
            let ratio = force_calc_times[i].1 / force_calc_times[i - 1].1;
            assert!(
                ratio < 3.5,
                "Force calculation scaling looks like O(n²), ratio: {}",
                ratio
            );
        }
    }
}
