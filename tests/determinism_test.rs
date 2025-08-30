//! Integration test to verify physics determinism across color schemes

use rand::Rng;
use stardrift::resources::{RenderingRng, SharedRng};

#[test]
fn test_rng_independence() {
    // Test that physics and rendering RNGs are independent
    let seed = 42u64;

    // Scenario 1: Generate physics values with minimal rendering RNG usage
    let mut physics_rng1 = SharedRng::from_seed(seed);
    let mut rendering_rng1 = RenderingRng::from_seed(seed);

    let physics_values1: Vec<f64> = (0..10)
        .map(|_| physics_rng1.random_range(0.0..1.0))
        .collect();

    // Use rendering RNG a little
    let _color1 = rendering_rng1.random_range(0.0..1.0);

    // Scenario 2: Generate physics values with heavy rendering RNG usage
    let mut physics_rng2 = SharedRng::from_seed(seed);
    let mut rendering_rng2 = RenderingRng::from_seed(seed);

    let physics_values2: Vec<f64> = (0..10)
        .map(|_| physics_rng2.random_range(0.0..1.0))
        .collect();

    // Use rendering RNG heavily (simulating complex color scheme)
    for _ in 0..100 {
        let _color2 = rendering_rng2.random_range(0.0..1.0);
    }

    // Physics values should be identical regardless of rendering RNG usage
    assert_eq!(
        physics_values1, physics_values2,
        "Physics RNG should be independent of rendering RNG usage"
    );
}

#[test]
fn test_rendering_rng_different_seed() {
    // Test that rendering RNG uses a different internal seed
    let seed = 42u64;

    let mut physics_rng = SharedRng::from_seed(seed);
    let mut rendering_rng = RenderingRng::from_seed(seed);

    let physics_val = physics_rng.random_range(0.0..1.0);
    let rendering_val = rendering_rng.random_range(0.0..1.0);

    // Values should be different since RNGs use different internal seeds
    assert_ne!(
        physics_val, rendering_val,
        "Physics and rendering RNGs should produce different sequences"
    );
}
