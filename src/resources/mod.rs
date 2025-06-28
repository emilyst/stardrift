use crate::physics::octree::Octree;
use avian3d::math::{Scalar, Vector};
use bevy::prelude::*;
use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;

#[derive(Resource, Deref, DerefMut, Debug, Clone, PartialEq)]
pub struct SharedRng(pub ChaCha8Rng);

// TODO: use a seedable RNG and make the seed configurable
impl Default for SharedRng {
    fn default() -> Self {
        Self(ChaCha8Rng::from_rng(&mut rand::rng()))
    }
}

#[derive(Resource, Deref, DerefMut, Copy, Clone, PartialEq, Debug)]
pub struct GravitationalConstant(pub Scalar);

impl Default for GravitationalConstant {
    fn default() -> Self {
        Self(1e1)
    }
}

#[derive(Resource, Deref, DerefMut, Copy, Clone, PartialEq, Debug)]
pub struct BodyCount(pub usize);

impl Default for BodyCount {
    fn default() -> Self {
        if cfg!(target_arch = "wasm32") {
            Self(100)
        } else {
            Self(100)
        }
    }
}

#[derive(Resource, Deref, DerefMut, Copy, Clone, Default, PartialEq, Debug)]
pub struct CurrentBarycenter(pub Vector);

#[derive(Resource, Deref, DerefMut, Copy, Clone, Default, PartialEq, Debug)]
pub struct PreviousBarycenter(pub Vector);

#[derive(Resource, Deref, DerefMut, Debug)]
pub struct GravitationalOctree(pub Octree);

impl GravitationalOctree {
    pub fn new(octree: Octree) -> Self {
        Self(octree)
    }
}

#[derive(Resource, Default)]
pub struct OctreeVisualizationSettings {
    pub enabled: bool,
    pub max_depth: Option<usize>, // None means show all levels
}

#[derive(Resource)]
pub struct BarycenterGizmoVisibility {
    pub enabled: bool,
}

impl Default for BarycenterGizmoVisibility {
    fn default() -> Self {
        Self { enabled: false }
    }
}

#[derive(Resource, Default)]
pub struct LoadingProgress {
    pub progress: f32, // 0.0 to 1.0
    pub current_message: String,
}

#[derive(Resource)]
pub struct BodySpawningProgress {
    pub bodies_spawned: usize,
    pub total_bodies: usize,
    pub batch_size: usize,
}

#[derive(Resource, Deref, DerefMut)]
pub struct LoadingTimer(pub Timer);
