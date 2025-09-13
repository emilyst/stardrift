use crate::prelude::*;
use rand_chacha::{ChaCha8Rng, rand_core::SeedableRng};

#[derive(Resource, Deref, DerefMut, Debug, Clone, PartialEq)]
pub struct SharedRng(pub ChaCha8Rng);

impl SharedRng {
    pub fn from_seed(seed: u64) -> Self {
        Self(ChaCha8Rng::seed_from_u64(seed))
    }

    pub fn from_optional_seed(seed: Option<u64>) -> Self {
        match seed {
            Some(seed) => Self::from_seed(seed),
            None => Self::default(),
        }
    }
}

impl Default for SharedRng {
    fn default() -> Self {
        Self(ChaCha8Rng::from_rng(&mut rand::rng()))
    }
}

/// Separate RNG for rendering operations (colors, visual effects).
/// This ensures that color scheme changes don't affect physics determinism.
#[derive(Resource, Deref, DerefMut, Debug, Clone, PartialEq)]
pub struct RenderingRng(pub ChaCha8Rng);

impl RenderingRng {
    pub fn from_seed(seed: u64) -> Self {
        // Use a different base seed for rendering to ensure independence
        // Add a large prime to differentiate from physics seed
        Self(ChaCha8Rng::seed_from_u64(
            seed.wrapping_add(0x9E3779B97F4A7C15),
        ))
    }

    pub fn from_optional_seed(seed: Option<u64>) -> Self {
        match seed {
            Some(seed) => Self::from_seed(seed),
            None => Self::default(),
        }
    }
}

impl Default for RenderingRng {
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
        Self(100)
    }
}

impl From<usize> for BodyCount {
    fn from(count: usize) -> Self {
        Self(count)
    }
}

#[derive(Resource, Deref, DerefMut, Copy, Clone, Default, PartialEq, Debug)]
pub struct Barycenter(pub Option<Vector>);

#[derive(Resource, Deref, DerefMut)]
pub struct GravitationalOctree(pub Octree);

impl GravitationalOctree {
    pub fn new(octree: Octree) -> Self {
        Self(octree)
    }
}
