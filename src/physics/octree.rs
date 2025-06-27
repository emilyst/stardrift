//! Spatial octree for approximating gravitational forces using the Barnes-Hut algorithm.

use avian3d::math::Scalar;
use avian3d::math::Vector;
use bevy::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct Aabb3d {
    pub min: Vector,
    pub max: Vector,
}

impl Aabb3d {
    pub fn new(min: Vector, max: Vector) -> Self {
        Self { min, max }
    }

    pub fn center(&self) -> Vector {
        (self.min + self.max) * 0.5
    }

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
    pub theta: Scalar,        // Barnes-Hut approximation parameter
    pub min_distance: Scalar, // Minimum distance for force calculation
    pub max_force: Scalar,    // Maximum force magnitude
}

impl Octree {
    pub fn new(theta: Scalar, min_distance: Scalar, max_force: Scalar) -> Self {
        Self {
            root: None,
            theta,
            min_distance,
            max_force,
        }
    }

    pub fn get_bounds(&self, max_depth: Option<usize>) -> Vec<Aabb3d> {
        let mut bounds = Vec::new();
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
        // Check if we should include this node based on depth limit
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

    pub fn build(&mut self, bodies: Vec<OctreeBody>) {
        if bodies.is_empty() {
            self.root = None;
            return;
        }

        let mut min = bodies[0].position;
        let mut max = bodies[0].position;

        for body in &bodies {
            min = Vector::new(
                min.x.min(body.position.x),
                min.y.min(body.position.y),
                min.z.min(body.position.z),
            );
            max = Vector::new(
                max.x.max(body.position.x),
                max.y.max(body.position.y),
                max.z.max(body.position.z),
            );
        }

        // Add some padding to ensure all bodies are inside
        let padding = (max - min) * 0.1;
        min -= padding;
        max += padding;

        let bounds = Aabb3d::new(min, max);
        self.root = Some(Self::build_node(bounds, bodies));
    }

    fn build_node(bounds: Aabb3d, bodies: Vec<OctreeBody>) -> OctreeNode {
        if bodies.len() <= 1 {
            return OctreeNode::External { bounds, bodies };
        }

        let octants = bounds.subdivide_into_children();
        let mut children: [Option<OctreeNode>; 8] = Default::default();
        let center = bounds.center();

        // Distribute bodies to octants, ensuring each body goes to exactly one octant
        let mut octant_bodies: [Vec<OctreeBody>; 8] = Default::default();

        for body in bodies.iter() {
            let octant_index = Self::get_octant_index(body.position, center);
            octant_bodies[octant_index].push(*body);
        }

        for (i, bodies_in_octant) in octant_bodies.into_iter().enumerate() {
            if !bodies_in_octant.is_empty() {
                children[i] = Some(Self::build_node(octants[i], bodies_in_octant));
            }
        }

        let total_mass: Scalar = bodies.iter().map(|b| b.mass).sum();
        let center_of_mass = if total_mass > 0.0 {
            let weighted_pos: Vector = bodies
                .iter()
                .map(|b| b.position * b.mass)
                .fold(Vector::ZERO, |acc, pos| acc + pos);
            weighted_pos / total_mass
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

    fn get_octant_index(position: Vector, center: Vector) -> usize {
        let mut index = 0;

        if position.x > center.x {
            index |= 1;
        }
        if position.y > center.y {
            index |= 2;
        }
        if position.z > center.z {
            index |= 4;
        }

        index
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
                    let virtual_body = OctreeBody {
                        entity: Entity::PLACEHOLDER, // Won't be used in comparison
                        position: *center_of_mass,
                        mass: *total_mass,
                    };
                    self.calculate_direct_force(body, &virtual_body, g)
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

    fn calculate_direct_force(&self, body1: &OctreeBody, body2: &OctreeBody, g: Scalar) -> Vector {
        let min_distance_squared = self.min_distance * self.min_distance;

        let direction = body2.position - body1.position;
        let distance_squared = direction.length_squared();

        if distance_squared < min_distance_squared {
            return Vector::ZERO;
        }

        let distance = distance_squared.sqrt();
        let direction_normalized = direction / distance;
        let force_magnitude = g * body1.mass * body2.mass / distance_squared;
        let force_magnitude = force_magnitude.min(self.max_force);

        direction_normalized * force_magnitude
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
        let mut octree = Octree::new(0.5, 10.0, 1e4); // theta = 0.5, min_distance = 10.0, max_force = 1e4

        // Create two bodies separated by some distance
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

        // Build octree with these bodies
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
