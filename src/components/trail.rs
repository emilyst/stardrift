use crate::prelude::*;
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct TrailPoint {
    pub position: Vec3,
    pub age: f32,
}

#[derive(Component, Debug)]
pub struct Trail {
    pub points: VecDeque<TrailPoint>,
    pub color: Color,
    pub last_update: f32,
}

impl Trail {
    pub fn new(color: Color) -> Self {
        Self {
            points: VecDeque::new(),
            color,
            last_update: 0.0,
        }
    }

    pub fn add_point(&mut self, position: Vec3, current_time: f32) {
        self.points.push_front(TrailPoint {
            position,
            age: current_time,
        });

        self.last_update = current_time;
    }

    pub fn cleanup_old_points(&mut self, current_time: f32, max_age: f32, max_points: usize) {
        self.points
            .retain(|point| current_time - point.age <= max_age);

        // Also enforce max_points limit for performance
        if self.points.len() > max_points {
            self.points.truncate(max_points);
        }

        self.last_update = current_time;
    }

    pub fn should_update(&self, current_time: f32, update_interval: f32) -> bool {
        current_time - self.last_update >= update_interval
    }

    pub fn calculate_point_alpha(
        &self,
        point: &TrailPoint,
        current_time: f32,
        fade_config: &crate::config::TrailConfig,
    ) -> f32 {
        if !fade_config.enable_fading {
            return fade_config.max_alpha as f32;
        }

        let age = current_time - point.age;
        let max_age = fade_config.trail_length_seconds as f32;

        if age <= 0.0 || max_age <= 0.0 {
            return fade_config.max_alpha as f32;
        }

        let fade_ratio = (age / max_age).clamp(0.0, 1.0);

        let curved_fade = match fade_config.fade_curve {
            crate::config::FadeCurve::Linear => fade_ratio,
            crate::config::FadeCurve::Exponential => fade_ratio * fade_ratio,
            crate::config::FadeCurve::SmoothStep => {
                // Smooth step function: 3t² - 2t³
                let t = fade_ratio;
                3.0 * t * t - 2.0 * t * t * t
            }
            crate::config::FadeCurve::EaseInOut => {
                // Ease in-out using cosine interpolation
                let t = fade_ratio;
                0.5 * (1.0 - (t * std::f32::consts::PI).cos())
            }
        };

        let min_alpha = fade_config.min_alpha as f32;
        let max_alpha = fade_config.max_alpha as f32;

        max_alpha - curved_fade * (max_alpha - min_alpha)
    }

    /// Each trail vertex becomes two vertices forming a strip one unit wide
    pub fn get_triangle_strip_vertices(&self, camera_pos: Option<Vec3>) -> Vec<Vec3> {
        if self.points.len() < 2 {
            return Vec::new();
        }

        let mut vertices = Vec::with_capacity(self.points.len() * 2);

        for i in 0..self.points.len() {
            let current_pos = self.points[i].position;

            // Calculate direction consistently along the trail
            let direction = if i + 1 < self.points.len() {
                // Use direction to next point
                (self.points[i + 1].position - current_pos).normalize_or_zero()
            } else if i > 0 {
                // For last point, use same direction as previous segment
                (current_pos - self.points[i - 1].position).normalize_or_zero()
            } else {
                // Single point fallback
                Vec3::X
            };

            let mut perpendicular = if let Some(cam_pos) = camera_pos {
                // Camera-facing width: cross product gives us width perpendicular to both
                // trail direction and camera-to-point vector
                let to_camera = (cam_pos - current_pos).normalize_or_zero();
                let cross = direction.cross(to_camera).normalize_or_zero();

                if cross.length_squared() > 0.001 {
                    cross
                } else {
                    // Camera aligned with trail, use world-up fallback
                    direction.cross(Vec3::Y).normalize_or_zero()
                }
            } else {
                // Fallback to world-up perpendicular
                direction.cross(Vec3::Y).normalize_or_zero()
            };

            if perpendicular == Vec3::ZERO {
                // Final fallback if direction is vertical
                perpendicular = Vec3::X;
            }

            let half_width = 0.5; // Trail is 1 unit wide
            let left = current_pos - perpendicular * half_width;
            let right = current_pos + perpendicular * half_width;

            vertices.push(left);
            vertices.push(right);
        }

        vertices
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trail_creation() {
        let color = Color::srgb(1.0, 0.0, 0.0);
        let trail = Trail::new(color);

        assert_eq!(trail.points.len(), 0);
        assert_eq!(trail.color, color);
        assert_eq!(trail.last_update, 0.0);
    }

    #[test]
    fn test_trail_add_point() {
        let mut trail = Trail::new(Color::WHITE);
        let pos1 = Vec3::new(0.0, 0.0, 0.0);
        let pos2 = Vec3::new(1.0, 0.0, 0.0);

        trail.add_point(pos1, 1.0);
        assert_eq!(trail.points.len(), 1);
        assert_eq!(trail.points[0].position, pos1);

        trail.add_point(pos2, 2.0);
        assert_eq!(trail.points.len(), 2);
        assert_eq!(trail.points[0].position, pos2); // Newest point first
        assert_eq!(trail.points[1].position, pos1);
    }

    #[test]
    fn test_trail_cleanup_old_points() {
        let mut trail = Trail::new(Color::WHITE);

        // Add points at different times
        trail.add_point(Vec3::new(0.0, 0.0, 0.0), 1.0);
        trail.add_point(Vec3::new(1.0, 0.0, 0.0), 2.0);
        trail.add_point(Vec3::new(2.0, 0.0, 0.0), 3.0);
        trail.add_point(Vec3::new(3.0, 0.0, 0.0), 10.0);

        assert_eq!(trail.points.len(), 4);

        // Clean up points older than 5 seconds from time 10.0
        trail.cleanup_old_points(10.0, 5.0, 1000);

        // Points at times 1.0, 2.0, 3.0 should be removed (older than 5 seconds from 10.0)
        // Only point at time 10.0 should remain
        assert_eq!(trail.points.len(), 1);
        assert_eq!(trail.points[0].age, 10.0);
    }

    #[test]
    fn test_trail_max_points_limit() {
        let mut trail = Trail::new(Color::WHITE);

        // Add more points than our limit
        for i in 0..10 {
            trail.add_point(Vec3::new(i as f32, 0.0, 0.0), i as f32);
        }

        // Apply max_points limit of 5
        trail.cleanup_old_points(100.0, 100.0, 5);

        assert_eq!(trail.points.len(), 5);
    }

    #[test]
    fn test_alpha_calculation() {
        let trail = Trail::new(Color::WHITE);
        let mut config = crate::config::TrailConfig::default();
        config.trail_length_seconds = 10.0;
        config.min_alpha = 0.0;
        config.max_alpha = 1.0;
        config.enable_fading = true;

        // Create a test point
        let point = TrailPoint {
            position: Vec3::ZERO,
            age: 5.0,
        };

        // Test at different times
        let alpha_new = trail.calculate_point_alpha(&point, 5.0, &config); // Same age as point
        let alpha_mid = trail.calculate_point_alpha(&point, 10.0, &config); // 5 seconds old
        let alpha_old = trail.calculate_point_alpha(&point, 15.0, &config); // 10 seconds old (max age)

        // New point should be max alpha
        assert!((alpha_new - 1.0).abs() < 0.001);

        // Middle-aged point should be partially faded
        assert!(alpha_mid > 0.0 && alpha_mid < 1.0);

        // Old point should be min alpha
        assert!(alpha_old.abs() < 0.001);
    }

    #[test]
    fn test_alpha_calculation_disabled() {
        let trail = Trail::new(Color::WHITE);
        let mut config = crate::config::TrailConfig::default();
        config.enable_fading = false;
        config.max_alpha = 0.8;

        let point = TrailPoint {
            position: Vec3::ZERO,
            age: 0.0,
        };

        // When fading is disabled, should always return max_alpha
        let alpha = trail.calculate_point_alpha(&point, 10.0, &config);
        assert!((alpha - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_should_update() {
        let mut trail = Trail::new(Color::WHITE);
        trail.last_update = 5.0;

        assert!(!trail.should_update(5.5, 1.0)); // Too soon
        assert!(trail.should_update(6.0, 1.0)); // Time to update
        assert!(trail.should_update(7.0, 1.0)); // Definitely time to update
    }

    #[test]
    fn test_triangle_strip_vertices() {
        let mut trail = Trail::new(Color::WHITE);

        // Empty trail should return no vertices
        assert_eq!(trail.get_triangle_strip_vertices(None).len(), 0);

        // Single point should return no vertices (need at least 2 for direction)
        trail.add_point(Vec3::ZERO, 0.0);
        assert_eq!(trail.get_triangle_strip_vertices(None).len(), 0);

        // Two points should return 4 vertices (2 per point)
        trail.add_point(Vec3::new(1.0, 0.0, 0.0), 1.0);
        let vertices = trail.get_triangle_strip_vertices(None);
        assert_eq!(vertices.len(), 4);

        // Vertices should form a strip
        assert_ne!(vertices[0], vertices[1]); // Left and right should be different
        assert_ne!(vertices[2], vertices[3]); // Left and right should be different
    }
}
