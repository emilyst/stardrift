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
