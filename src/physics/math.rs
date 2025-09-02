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

/// Re-export commonly used math constants
use crate::prelude::Vec3;

pub fn min_sphere_radius_for_surface_distribution(
    n: usize,
    min_distance: f32,
    tolerance: f32,
) -> f32 {
    let minimum_radius = min_distance * libm::sqrtf(n as f32 / 4.0);
    let spherical_correction = if n > 4 {
        // Tammes problem approximation
        let solid_angle_per_point = 4.0 * std::f32::consts::PI / n as f32;
        let half_angle = solid_angle_per_point / libm::sqrtf(2.0 * std::f32::consts::PI);
        min_distance / (2.0 * libm::sinf(half_angle))
    } else {
        // For small N, use exact solutions
        match n {
            1 => min_distance,                          // Any radius works
            2 => min_distance / 2.0,                    // Points are antipodal
            3 => min_distance / libm::sqrtf(3.0),       // Equilateral triangle
            4 => min_distance / libm::sqrtf(8.0 / 3.0), // Tetrahedron
            _ => minimum_radius,
        }
    };
    let mut corrected_minimum_radius = minimum_radius.max(spherical_correction);

    // Iterative refinement using the sphere cap
    for _ in 0..10 {
        let cap_radius = min_distance / 2.0;
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
        } else if sphere_area - total_cap_area > tolerance * sphere_area {
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

#[cfg(test)]
mod math_tests {
    use super::*;

    #[test]
    fn test_vector_component_min_max() {
        let v1 = Vector::new(1.0, 5.0, 3.0);
        let v2 = Vector::new(4.0, 2.0, 6.0);

        let min = v1.component_min(v2);
        assert_eq!(min.x, 1.0);
        assert_eq!(min.y, 2.0);
        assert_eq!(min.z, 3.0);

        let max = v1.component_max(v2);
        assert_eq!(max.x, 4.0);
        assert_eq!(max.y, 5.0);
        assert_eq!(max.z, 6.0);

        // Test with negative values
        let v3 = Vector::new(-2.0, -1.0, 0.0);
        let v4 = Vector::new(-1.0, -3.0, 1.0);

        let min2 = v3.component_min(v4);
        assert_eq!(min2.x, -2.0);
        assert_eq!(min2.y, -3.0);
        assert_eq!(min2.z, 0.0);

        let max2 = v3.component_max(v4);
        assert_eq!(max2.x, -1.0);
        assert_eq!(max2.y, -1.0);
        assert_eq!(max2.z, 1.0);
    }

    // Alternative test using coordinate moments
    #[test]
    fn test_random_unit_vector_coordinate_moments_uniformity() {
        let count_of_samples = 100_000;
        let mut rng = SharedRng::default();

        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_z = 0.0;
        let mut sum_x2 = 0.0;
        let mut sum_y2 = 0.0;
        let mut sum_z2 = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_xz = 0.0;
        let mut sum_yz = 0.0;

        for _ in 0..count_of_samples {
            let v = random_unit_vector(&mut rng);
            sum_x += v.x;
            sum_y += v.y;
            sum_z += v.z;
            sum_x2 += v.x * v.x;
            sum_y2 += v.y * v.y;
            sum_z2 += v.z * v.z;
            sum_xy += v.x * v.y;
            sum_xz += v.x * v.z;
            sum_yz += v.y * v.z;
        }

        let n = count_of_samples as f32;
        let tolerance = 3.0 / n.sqrt(); // 3-sigma tolerance

        // First moments should be ~0 (center of mass at origin)
        assert!(
            (sum_x / n).abs() < tolerance,
            "X coordinate mean too far from 0: {:.6}",
            sum_x / n
        );
        assert!(
            (sum_y / n).abs() < tolerance,
            "Y coordinate mean too far from 0: {:.6}",
            sum_y / n
        );
        assert!(
            (sum_z / n).abs() < tolerance,
            "Z coordinate mean too far from 0: {:.6}",
            sum_z / n
        );

        // Second moments should be ~1/3 (uniform on unit sphere)
        let expected_second_moment = 1.0 / 3.0;
        assert!(
            ((sum_x2 / n) - expected_second_moment).abs() < tolerance,
            "X² moment deviation: {:.6}, expected: {:.6}",
            sum_x2 / n,
            expected_second_moment
        );
        assert!(
            ((sum_y2 / n) - expected_second_moment).abs() < tolerance,
            "Y² moment deviation: {:.6}, expected: {:.6}",
            sum_y2 / n,
            expected_second_moment
        );
        assert!(
            ((sum_z2 / n) - expected_second_moment).abs() < tolerance,
            "Z² moment deviation: {:.6}, expected: {:.6}",
            sum_z2 / n,
            expected_second_moment
        );

        // Cross moments should be ~0 (no correlation)
        assert!(
            (sum_xy / n).abs() < tolerance,
            "XY correlation too high: {:.6}",
            sum_xy / n
        );
        assert!(
            (sum_xz / n).abs() < tolerance,
            "XZ correlation too high: {:.6}",
            sum_xz / n
        );
        assert!(
            (sum_yz / n).abs() < tolerance,
            "YZ correlation too high: {:.6}",
            sum_yz / n
        );
    }

    #[test]
    fn test_random_unit_vector_properties() {
        for _ in 0..100_000 {
            let v = random_unit_vector(&mut SharedRng::default());
            let length = libm::sqrtf(v.x * v.x + v.y * v.y + v.z * v.z);

            assert!(
                (length - 1.0).abs() < 1e-6, // f32 has ~7 decimal digits of precision
                "Vector length should be 1, but was: {length}",
            );
        }
    }
}
