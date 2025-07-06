//! Color utilities for temperature-based color conversion and bloom effects.
//!
//! This module provides comprehensive color management capabilities for the N-body simulation,
//! focusing on physically-based color temperature conversion and visual enhancement through
//! bloom effects. It enables realistic color representation of celestial bodies based on their
//! temperature characteristics.
//!
//! # Key Features
//!
//! - **Temperature-to-RGB conversion**: Converts color temperatures (in Kelvin) to normalized RGB
//!   values using Tanner Helland's approximation algorithm
//! - **Bloom effect generation**: Creates enhanced colors for emissive materials with configurable
//!   intensity
//! - **Bevy integration**: Integration with Bevy's material and color systems
//!
//! # Temperature Ranges
//!
//! The module handles various temperature ranges corresponding to different light sources:
//!
//! - **1000K - 2000K**: Candle flames, very warm orange/red light
//! - **2700K - 3000K**: Incandescent bulbs, warm white light
//! - **5000K - 6000K**: Daylight, balanced white light
//! - **6500K - 7000K**: Cool daylight, slightly blue-tinted
//! - **8000K+**: Blue sky, cool blue light
//!
//! # Main Functions
//!
//! - [`emissive_material_for_temp`]: Creates Bevy materials with temperature-based
//!   colors and bloom
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::color::*;
//!
//! // Create a material for a star at 5778K (Sun's temperature)
//! let material = emissive_material_for_temp(
//!     &mut materials,
//!     5778.0,  // Temperature in Kelvin
//!     2.0      // Bloom intensity multiplier
//!     2.0      // Saturation intensity multiplier
//! );
//! ```
//!
//! # Algorithm Details
//!
//! The color conversion is based on Tanner Helland's approximation algorithm, which provides
//! a good balance between computational efficiency and visual accuracy. The algorithm uses
//! different mathematical approaches for temperatures above and below 6600K to account for
//! the different spectral characteristics in these ranges.

use bevy::prelude::*;

const MIN_TEMPERATURE: f64 = 1000.0;
const MAX_TEMPERATURE: f64 = 40000.0;
const DAYLIGHT_TEMP_THRESHOLD: f64 = 6600.0;
const BLUE_TEMP_THRESHOLD: f64 = 1900.0;
const MAX_COLOR_VALUE: f64 = 255.0;

const RED_COEFFICIENT: f64 = 329.698727446;
const RED_OFFSET: f64 = 60.0;
const RED_EXPONENT: f64 = -0.1332047592;

const GREEN_WARM_COEFFICIENT: f64 = 99.4708025861;
const GREEN_WARM_OFFSET: f64 = -161.1195681661;
const GREEN_COOL_COEFFICIENT: f64 = 288.1221695283;
const GREEN_COOL_EXPONENT: f64 = -0.0755148492;

const BLUE_COEFFICIENT: f64 = 138.5177312231;
const BLUE_OFFSET: f64 = -305.0447927307;
const BLUE_LOG_OFFSET: f64 = 10.0;

/// Creates a Bevy `StandardMaterial` with temperature-based colors and bloom effects.
///
/// This function combines temperature-to-color conversion with bloom enhancement to create
/// realistic emissive materials for celestial bodies or other temperature-based objects.
/// The resulting material has both a base color derived from the temperature and an
/// emissive component enhanced for bloom effects.
///
/// # Arguments
///
/// * `materials` - Mutable reference to Bevy's material asset storage
/// * `temperature` - Color temperature in Kelvin (clamped to 1000K - 40000K range)
/// * `bloom_intensity` - Multiplier for bloom effect intensity (typically 1.0 - 5.0)
/// * `saturation_intensity` - Multiplier for saturation intensity (typically 1.0 - 5.0)
///
/// # Returns
///
/// A `Handle<StandardMaterial>` that can be used with Bevy's rendering system.
/// The material includes:
/// - `base_color`: The natural color at the given temperature
/// - `emissive`: An intensified version of the color for bloom effects
///
/// # Examples
///
/// ```rust,ignore
/// // Create a material for the Sun (5778K) with moderate bloom
/// let sun_material = emissive_material_for_temp(
///     &mut materials,
///     5778.0,
///     2.5,
///     1.0,
/// );
///
/// // Create a material for a red giant star (3500K) with intense bloom and saturation
/// let red_giant_material = emissive_material_for_temp(
///     &mut materials,
///     3500.0,
///     4.0,
///     2.0,
/// );
/// ```
pub(crate) fn emissive_material_for_temp(
    materials: &mut ResMut<Assets<StandardMaterial>>,
    temperature: f64,
    bloom_intensity: f64,
    saturation_intensity: f64,
) -> Handle<StandardMaterial> {
    let base_rgb = rgb_for_temp(temperature);
    let saturated_rgb = enhance_saturation(base_rgb, saturation_intensity);

    let bloom_color = {
        let (r, g, b) = intensify_for_bloom(saturated_rgb, bloom_intensity);
        Color::LinearRgba(LinearRgba::rgb(r as f32, g as f32, b as f32))
    };

    let base_color = {
        let (r, g, b) = saturated_rgb;
        Color::LinearRgba(LinearRgba::rgb(r as f32, g as f32, b as f32))
    };

    materials.add(StandardMaterial {
        base_color,
        metallic: 0.0,
        reflectance: 0.0,
        emissive: bloom_color.into(),
        ..default()
    })
}

/// Enhances the saturation of RGB values by scaling the distance from grayscale.
///
/// This function increases color saturation by pushing RGB values further away
/// from their grayscale equivalent, making colors more vivid and intense.
///
/// # Arguments
///
/// * `rgb` - A tuple of normalized RGB values (0.0 - 1.0)
/// * `saturation_factor` - Saturation enhancement factor (1.0 = no change, >1.0 = more saturated)
///
/// # Returns
///
/// A tuple of enhanced RGB values, clamped to [0.0, 1.0] range.
fn enhance_saturation(rgb: (f64, f64, f64), saturation_factor: f64) -> (f64, f64, f64) {
    let (r, g, b) = rgb;

    // Calculate grayscale using ITU-R BT.601 luminance formula
    let gray = 0.299 * r + 0.587 * g + 0.114 * b;

    let enhanced_r = gray + (r - gray) * saturation_factor;
    let enhanced_g = gray + (g - gray) * saturation_factor;
    let enhanced_b = gray + (b - gray) * saturation_factor;

    (
        enhanced_r.clamp(0.0, 1.0),
        enhanced_g.clamp(0.0, 1.0),
        enhanced_b.clamp(0.0, 1.0),
    )
}

/// Applies luminance-based intensity scaling to RGB values for bloom effects.
///
/// This function enhances RGB color values by applying a scaling factor based on the
/// color's luminance (perceived brightness). The scaling uses the ITU-R BT.601 standard
/// for luminance calculation, which weights the RGB channels according to human visual
/// perception (green is weighted most heavily, then red, then blue).
///
/// The intensity scaling is applied proportionally to the luminance, meaning brighter
/// colors receive more enhancement, creating more realistic bloom effects.
///
/// # Arguments
///
/// * `rgb` - A tuple of normalized RGB values (0.0 - 1.0)
/// * `intensity` - Bloom intensity multiplier (typically 1.0 - 5.0)
///
/// # Returns
///
/// A tuple of enhanced RGB values. Values may exceed 1.0 for bloom effects.
///
/// # Algorithm
///
/// The function calculates luminance using: `L = 0.299*R + 0.587*G + 0.114*B`
/// Then applies scaling: `scale_factor = intensity * luminance + 1.0`
///
/// # Examples
///
/// ```rust,ignore
/// // Enhance a bright white color
/// let enhanced = intensify_for_bloom((1.0, 1.0, 1.0), 2.0);
///
/// // Enhance a dim red color
/// let enhanced = intensify_for_bloom((0.3, 0.1, 0.1), 2.0);
/// ```
fn intensify_for_bloom(rgb: (f64, f64, f64), intensity: f64) -> (f64, f64, f64) {
    let (r, g, b) = rgb;

    // Scale using ITU-R BT.601 luminance formula
    let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
    let scale_factor = intensity * luminance + 1.0;
    (r * scale_factor, g * scale_factor, b * scale_factor)
}

/// Converts a color temperature in Kelvin to normalized RGB values.
///
/// This is the core color conversion function that implements Tanner Helland's
/// approximation algorithm for converting blackbody radiation temperatures to
/// RGB color values. The algorithm provides a good balance between computational
/// efficiency and visual accuracy for the full range of stellar temperatures.
///
/// The function uses different mathematical approaches for temperatures above
/// and below 6600K to account for the different spectral characteristics in
/// these ranges, ensuring smooth transitions and accurate color representation.
///
/// # Arguments
///
/// * `temperature` - Color temperature in Kelvin. Values are automatically
///   clamped to the valid range of 1000K - 40000K.
///
/// # Returns
///
/// A tuple `(r, g, b)` of normalized RGB values in the range [0.0, 1.0].
/// The values represent the linear RGB color space suitable for further
/// processing or conversion to other color formats.
///
/// # Temperature Ranges and Characteristics
///
/// - **1000K - 1900K**: Deep red/orange (candle flames, very warm sources)
/// - **2000K - 3000K**: Orange to warm white (incandescent bulbs)
/// - **3000K - 5000K**: Warm white to neutral white
/// - **5000K - 6600K**: Neutral to cool white (daylight)
/// - **6600K+**: Cool white to blue (overcast sky, blue stars)
///
/// # Algorithm Details
///
/// The algorithm uses different formulas for red, green, and blue channels
/// depending on the temperature range:
///
/// - **Below 6600K**: Uses logarithmic functions for green and blue channels
/// - **Above 6600K**: Uses power functions for red and green channels
/// - **Clamping**: All intermediate values are clamped to [0, 255] before normalization
///
/// # Examples
///
/// ```rust,ignore
/// // Get RGB for the Sun's surface temperature
/// let (r, g, b) = kelvin_to_rgb(5778.0);
/// // Result: approximately (1.0, 0.93, 0.84) - warm white
///
/// // Get RGB for a red giant star
/// let (r, g, b) = kelvin_to_rgb(3500.0);
/// // Result: approximately (1.0, 0.67, 0.35) - orange-red
///
/// // Get RGB for a blue star
/// let (r, g, b) = kelvin_to_rgb(10000.0);
/// // Result: approximately (0.78, 0.84, 1.0) - blue-white
/// ```
///
/// # References
///
/// Based on Tanner Helland's approximation algorithm, which provides
/// accurate color temperature conversion for the range 1000K - 40000K.
///
/// See https://tannerhelland.com/2012/09/18/convert-temperature-rgb-algorithm-code.html.
fn rgb_for_temp(temperature: f64) -> (f64, f64, f64) {
    let temp = temperature.clamp(MIN_TEMPERATURE, MAX_TEMPERATURE);

    let red = red_channel_for_temp(temp);
    let green = green_channel_for_temp(temp);
    let blue = blue_channel_for_temp(temp);

    let red_norm = red.clamp(0.0, MAX_COLOR_VALUE) / MAX_COLOR_VALUE;
    let green_norm = green.clamp(0.0, MAX_COLOR_VALUE) / MAX_COLOR_VALUE;
    let blue_norm = blue.clamp(0.0, MAX_COLOR_VALUE) / MAX_COLOR_VALUE;

    (red_norm, green_norm, blue_norm)
}

fn red_channel_for_temp(temp: f64) -> f64 {
    if temp <= DAYLIGHT_TEMP_THRESHOLD {
        MAX_COLOR_VALUE
    } else {
        RED_COEFFICIENT * libm::pow(temp / 100.0 - RED_OFFSET, RED_EXPONENT)
    }
}

fn green_channel_for_temp(temp: f64) -> f64 {
    if temp <= DAYLIGHT_TEMP_THRESHOLD {
        GREEN_WARM_COEFFICIENT * libm::log(temp / 100.0) + GREEN_WARM_OFFSET
    } else {
        GREEN_COOL_COEFFICIENT * libm::pow(temp / 100.0 - RED_OFFSET, GREEN_COOL_EXPONENT)
    }
}

fn blue_channel_for_temp(temp: f64) -> f64 {
    if temp >= DAYLIGHT_TEMP_THRESHOLD {
        MAX_COLOR_VALUE
    } else if temp < BLUE_TEMP_THRESHOLD {
        0.0
    } else {
        BLUE_COEFFICIENT * libm::log(temp / 100.0 - BLUE_LOG_OFFSET) + BLUE_OFFSET
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;

    #[test]
    fn test_common_temperatures() {
        // Candle flame (~1900K) - should be very warm/orange
        let (r, g, b) = rgb_for_temp(1900.0);
        assert!(r > 0.9); // Very red
        assert!(g < 0.7); // Less green
        assert!(b < 0.1); // Very little blue

        // Incandescent bulb (~2700K) - warm white
        let (r, g, b) = rgb_for_temp(2700.0);
        assert!(r > 0.9);
        assert!(g > 0.6);
        assert!(b < 0.5);

        // Daylight (~5500K) - should be relatively balanced
        let (r, g, b) = rgb_for_temp(5500.0);
        assert!(r > 0.8);
        assert!(g > 0.8);
        assert!(b > 0.7);

        // Cool daylight (~6500K) - slightly blue
        let (r, g, b) = rgb_for_temp(6500.0);
        assert!(r > 0.7);
        assert!(g > 0.8);
        assert!(b > 0.8);

        // Blue sky (~10000K) - should be very blue
        let (r, g, b) = rgb_for_temp(10000.0);
        assert!(r < 0.8);
        assert!(g < 0.9);
        assert!(b > 0.9);
    }

    #[test]
    fn test_rgb_range() {
        for temp in (1000..=40000).step_by(500) {
            let (r, g, b) = rgb_for_temp(temp as f64);

            assert!((0.0..=1.0).contains(&r), "R out of range at {temp}K: {r}");
            assert!((0.0..=1.0).contains(&g), "G out of range at {temp}K: {g}",);
            assert!((0.0..=1.0).contains(&b), "B out of range at {temp}K: {b}",);
        }
    }

    #[test]
    fn test_temperature_clamping() {
        let (r1, g1, b1) = rgb_for_temp(500.0); // Too low
        let (r2, g2, b2) = rgb_for_temp(1000.0); // Minimum
        assert_eq!((r1, g1, b1), (r2, g2, b2));

        let (r3, g3, b3) = rgb_for_temp(50000.0); // Too high
        let (r4, g4, b4) = rgb_for_temp(40000.0); // Maximum
        assert_eq!((r3, g3, b3), (r4, g4, b4));
    }
}
