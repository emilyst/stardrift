//! Spatial octree for approximating gravitational forces using the Barnes-Hut algorithm.

use avian3d::math::Scalar;
use avian3d::math::Vector;
use bevy::prelude::*;

const PADDING_FACTOR: Scalar = 0.1;
const DEFAULT_LEAF_THRESHOLD: usize = 4;

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
    pub theta: Scalar,            // Barnes-Hut approximation parameter
    pub min_distance: Scalar,     // Minimum distance for force calculation
    pub max_force: Scalar,        // Maximum force magnitude
    pub leaf_threshold: usize,    // Maximum bodies per leaf node
    min_distance_squared: Scalar, // Cached value to avoid repeated multiplication
}

impl Octree {
    pub fn new(theta: Scalar, min_distance: Scalar, max_force: Scalar) -> Self {
        Self {
            root: None,
            theta,
            min_distance,
            max_force,
            leaf_threshold: DEFAULT_LEAF_THRESHOLD,
            min_distance_squared: min_distance * min_distance,
        }
    }

    pub fn with_leaf_threshold(mut self, leaf_threshold: usize) -> Self {
        self.leaf_threshold = leaf_threshold;
        self
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
            for child in children.iter().flatten() {
                self.collect_bounds(child, bounds, current_depth + 1, max_depth);
            }
        }
    }

    pub fn build(&mut self, bodies: impl IntoIterator<Item = OctreeBody>) {
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

        for body in bodies_iter {
            min.x = min.x.min(body.position.x);
            min.y = min.y.min(body.position.y);
            min.z = min.z.min(body.position.z);
            max.x = max.x.max(body.position.x);
            max.y = max.y.max(body.position.y);
            max.z = max.z.max(body.position.z);
            bodies_vec.push(body);
        }

        let padding = (max - min) * PADDING_FACTOR;
        min -= padding;
        max += padding;

        let bounds = Aabb3d::new(min, max);
        self.root = Some(Self::build_node(bounds, bodies_vec, self.leaf_threshold));
    }

    fn build_node(bounds: Aabb3d, bodies: Vec<OctreeBody>, leaf_threshold: usize) -> OctreeNode {
        if bodies.len() <= leaf_threshold {
            return OctreeNode::External { bounds, bodies };
        }

        let center = bounds.center();
        let octants = bounds.subdivide_into_children();

        // Count bodies per octant first for better allocation
        let mut octant_counts = [0usize; 8];
        for body in &bodies {
            let octant_index = Self::get_octant_index(body.position, center);
            octant_counts[octant_index] += 1;
        }

        // Create vectors with exact capacity for non-empty octants
        let mut octant_bodies: [Vec<OctreeBody>; 8] = [
            Vec::with_capacity(octant_counts[0]),
            Vec::with_capacity(octant_counts[1]),
            Vec::with_capacity(octant_counts[2]),
            Vec::with_capacity(octant_counts[3]),
            Vec::with_capacity(octant_counts[4]),
            Vec::with_capacity(octant_counts[5]),
            Vec::with_capacity(octant_counts[6]),
            Vec::with_capacity(octant_counts[7]),
        ];
        let mut children: [Option<OctreeNode>; 8] = Default::default();

        for body in &bodies {
            let octant_index = Self::get_octant_index(body.position, center);
            octant_bodies[octant_index].push(*body);
        }

        for (i, bodies_in_octant) in octant_bodies.into_iter().enumerate() {
            if !bodies_in_octant.is_empty() {
                children[i] = Some(Self::build_node(
                    octants[i],
                    bodies_in_octant,
                    leaf_threshold,
                ));
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
            children: Box::new(children),
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
                    for child in children.iter() {
                        force += self.calculate_force(body, child.as_ref(), g);
                    }
                    force
                }
            }
            Some(OctreeNode::External { bodies, .. }) => {
                let mut force = Vector::ZERO;
                for other_body in bodies {
                    if other_body.entity != body.entity {
                        force += self.calculate_direct_force(body, other_body, g);
                    }
                }
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
