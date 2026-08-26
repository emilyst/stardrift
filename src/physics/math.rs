use crate::resources::SharedRng;
use rand::Rng;

/// Scalar type for physics calculations (f64 for precision)
pub type Scalar = f64;

/// 3D vector type for positions, velocities, and forces
pub type Vector = bevy::math::DVec3;

/// Extension trait for Vector operations
pub trait VectorExt {
    /// Returns component-wise minimum of two vectors
    fn component_min(self, other: Self) -> Self;

    /// Returns component-wise maximum of two vectors
    fn component_max(self, other: Self) -> Self;
}

impl VectorExt for Vector {
    #[inline]
    fn component_min(self, other: Self) -> Self {
        Vector::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
        )
    }

    #[inline]
    fn component_max(self, other: Self) -> Self {
        Vector::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
        )
    }
}

/// Uniform density shared by every body. Mass is the source of truth and
/// radius is derived from it (and vice versa at spawn), so the mass ∝ r³
/// invariant holds by construction — including across merges, where volume
/// conservation and density conservation coincide only because this is one
/// global constant.
pub const BODY_DENSITY: Scalar = 1.0;

/// Mass of a uniform-density sphere of the given radius.
#[inline]
pub fn mass_for_radius(radius: Scalar) -> Scalar {
    BODY_DENSITY * 4.0 / 3.0 * std::f64::consts::PI * radius.powi(3)
}

/// Radius of a uniform-density sphere of the given mass. Inverse of
/// [`mass_for_radius`].
#[inline]
pub fn radius_for_mass(mass: Scalar) -> Scalar {
    (mass / (BODY_DENSITY * 4.0 / 3.0 * std::f64::consts::PI)).cbrt()
}

/// Re-export commonly used math constants
use crate::prelude::Vec3;

/// Minimum sphere radius on whose surface `n` points can sit at least
/// `min_spacing` apart (Tammes-style approximation with iterative
/// refinement; `radius_tolerance` is the relative convergence tolerance).
pub fn min_sphere_radius_for_surface_distribution(
    n: usize,
    min_spacing: f32,
    radius_tolerance: f32,
) -> f32 {
    let minimum_radius = min_spacing * libm::sqrtf(n as f32 / 4.0);
    let spherical_correction = if n > 4 {
        // Tammes problem approximation
        let solid_angle_per_point = 4.0 * std::f32::consts::PI / n as f32;
        let half_angle = solid_angle_per_point / libm::sqrtf(2.0 * std::f32::consts::PI);
        min_spacing / (2.0 * libm::sinf(half_angle))
    } else {
        // For small N, use exact solutions
        match n {
            1 => min_spacing,                          // Any radius works
            2 => min_spacing / 2.0,                    // Points are antipodal
            3 => min_spacing / libm::sqrtf(3.0),       // Equilateral triangle
            4 => min_spacing / libm::sqrtf(8.0 / 3.0), // Tetrahedron
            _ => minimum_radius,
        }
    };
    let mut corrected_minimum_radius = minimum_radius.max(spherical_correction);

    // Iterative refinement using the sphere cap
    for _ in 0..10 {
        let cap_radius = min_spacing / 2.0;
        let cap_area = 2.0
            * std::f32::consts::PI
            * corrected_minimum_radius
            * corrected_minimum_radius
            * libm::powf(
                1.0 - libm::sqrtf(1.0 - (cap_radius / corrected_minimum_radius)),
                2.0,
            );
        let total_cap_area = n as f32 * cap_area;
        let sphere_area =
            4.0 * std::f32::consts::PI * corrected_minimum_radius * corrected_minimum_radius;

        if total_cap_area > sphere_area {
            corrected_minimum_radius *= 1.1;
        } else if sphere_area - total_cap_area > radius_tolerance * sphere_area {
            corrected_minimum_radius *= 0.95;
        } else {
            break; // Converged
        }
    }

    corrected_minimum_radius
}

pub fn random_unit_vector(rng: &mut SharedRng) -> Vec3 {
    let theta = rng.random_range(0.0..=2.0 * std::f32::consts::PI);
    let phi = libm::acosf(rng.random_range(-1.0..=1.0));
    let r = 1.0;

    Vec3::new(
        r * libm::sinf(phi) * libm::cosf(theta),
        r * libm::sinf(phi) * libm::sinf(theta),
        r * libm::cosf(phi),
    )
}
