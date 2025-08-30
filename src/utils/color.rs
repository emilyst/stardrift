//! Color utilities for material creation and color generation.
//!
//! This module provides color management capabilities for the N-body simulation,
//! including physically-based color temperature conversion, random color generation,
//! and material creation with bloom effects.
//!
//! # Key Features
//!
//! - **Temperature-to-RGB conversion**: Converts color temperatures (in Kelvin) to normalized RGB
//!   values using Tanner Helland's approximation algorithm for black body radiation
//! - **Rainbow color generation**: Creates random vibrant colors using Bevy's HSL color space
//! - **Material creation**: Unified API for creating emissive materials with bloom effects
//! - **Bevy integration**: Full integration with Bevy's color and material systems
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
//! - [`create_emissive_material`]: Creates Bevy materials from RGB colors with bloom effects
//! - [`rgb_for_temp`]: Converts color temperature (Kelvin) to normalized RGB values
//! - [`random_rainbow_color`]: Generates random vibrant colors using HSL color space
//! - [`intensify_for_bloom`]: Applies luminance-based intensity scaling for bloom effects
//!
//! # Usage
//!
//! ```rust
//! # use bevy::prelude::*;
//! use stardrift::utils::color::{create_emissive_material, rgb_for_temp, random_rainbow_color};
//! use stardrift::resources::SharedRng;
//!
//! fn create_materials(
//!     mut materials: ResMut<Assets<StandardMaterial>>,
//!     mut rng: ResMut<SharedRng>
//! ) {
//!     // Create material from temperature (black body radiation)
//!     let star_color = rgb_for_temp(5778.0);  // Sun's temperature
//!     let star_material = create_emissive_material(
//!         &mut materials,
//!         star_color,
//!         2.0,  // Bloom intensity
//!         1.0   // Saturation
//!     );
//!     
//!     // Create material from random rainbow color
//!     let rainbow_color = random_rainbow_color(&mut rng);
//!     let rainbow_material = create_emissive_material(
//!         &mut materials,
//!         rainbow_color,
//!         2.0,
//!         1.0
//!     );
//! }
//! ```
//!
//! # Algorithm Details
//!
//! The color conversion is based on Tanner Helland's approximation algorithm, which provides
//! a good balance between computational efficiency and visual accuracy. The algorithm uses
//! different mathematical approaches for temperatures above and below 6600K to account for
//! the different spectral characteristics in these ranges.

use bevy::prelude::*;

const MIN_TEMPERATURE: f32 = 1000.0;
const MAX_TEMPERATURE: f32 = 40000.0;
const DAYLIGHT_TEMP_THRESHOLD: f32 = 6600.0;
const BLUE_TEMP_THRESHOLD: f32 = 1900.0;
const MAX_COLOR_VALUE: f32 = 255.0;

const RED_COEFFICIENT: f32 = 329.698_73;
const RED_OFFSET: f32 = 60.0;
const RED_EXPONENT: f32 = -0.133_204_76;

const GREEN_WARM_COEFFICIENT: f32 = 99.470_8;
const GREEN_WARM_OFFSET: f32 = -161.119_57;
const GREEN_COOL_COEFFICIENT: f32 = 288.122_17;
const GREEN_COOL_EXPONENT: f32 = -0.075_514_85;

const BLUE_COEFFICIENT: f32 = 138.517_73;
const BLUE_OFFSET: f32 = -305.044_8;
const BLUE_LOG_OFFSET: f32 = 10.0;

/// Generates a random rainbow color using Bevy's HSL color space.
///
/// Creates vibrant colors with random hue, high saturation, and balanced lightness.
///
/// # Arguments
///
/// * `rng` - Random number generator for hue selection
///
/// # Returns
///
/// A tuple `(r, g, b)` of normalized RGB values in the range `[0.0, 1.0]`.
#[must_use]
pub fn random_rainbow_color(rng: &mut crate::resources::SharedRng) -> (f32, f32, f32) {
    use rand::prelude::*;

    let hue = rng.random_range(0.0..=360.0);
    let saturation = rng.random_range(0.8..=1.0); // High saturation for vibrant colors
    let lightness = rng.random_range(0.4..=0.6); // Balanced lightness

    // Use Bevy's built-in HSL to RGB conversion
    let color = Color::hsl(hue, saturation, lightness);

    // Convert to LinearRgba for consistent color space
    let linear_rgba = LinearRgba::from(color);
    (linear_rgba.red, linear_rgba.green, linear_rgba.blue)
}

/// Creates a Bevy `StandardMaterial` with emissive colors and bloom effects.
///
/// This function creates emissive materials for celestial bodies with bloom enhancement.
/// The resulting material has both a base color and an emissive component enhanced for bloom effects.
///
/// # Arguments
///
/// * `materials` - Mutable reference to Bevy's material asset storage
/// * `rgb` - Normalized RGB color values (0.0 - 1.0)
/// * `bloom_intensity` - Multiplier for bloom effect intensity (typically 1.0 - 5.0)
/// * `saturation_intensity` - Multiplier for color saturation (typically 1.0 - 3.0)
///
/// # Returns
///
/// A `Handle<StandardMaterial>` that can be used with Bevy's rendering system.
///
/// # Examples
///
/// ```rust
/// # use bevy::prelude::*;
/// use stardrift::utils::color::create_emissive_material;
///
/// fn create_materials(mut materials: ResMut<Assets<StandardMaterial>>) {
///     // Create from any RGB color
///     let material = create_emissive_material(
///         &mut materials,
///         (1.0, 0.5, 0.2),
///         2.5,
///         1.0
///     );
/// }
/// ```
pub fn create_emissive_material(
    materials: &mut ResMut<Assets<StandardMaterial>>,
    rgb: (f32, f32, f32),
    bloom_intensity: f32,
    saturation_intensity: f32,
) -> Handle<StandardMaterial> {
    let saturated_rgb = enhance_saturation(rgb, saturation_intensity);

    let bloom_color = {
        let (r, g, b) = intensify_for_bloom(saturated_rgb, bloom_intensity);
        Color::LinearRgba(LinearRgba::rgb(r, g, b))
    };

    let base_color = {
        let (r, g, b) = saturated_rgb;
        Color::LinearRgba(LinearRgba::rgb(r, g, b))
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
/// A tuple of enhanced RGB values, clamped to `[0.0, 1.0]` range.
///
/// # Example
///
/// ```text
/// // Enhance saturation of an orange color
/// enhance_saturation((1.0, 0.5, 0.2), 2.0)
/// // Returns more vivid orange with increased color separation from gray
/// ```
#[must_use]
fn enhance_saturation(rgb: (f32, f32, f32), saturation_factor: f32) -> (f32, f32, f32) {
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
/// ```rust
/// use stardrift::utils::color::intensify_for_bloom;
///
/// // Enhance a bright white color
/// let enhanced = intensify_for_bloom((1.0, 1.0, 1.0), 2.0);
/// assert!(enhanced.0 > 1.0); // Values can exceed 1.0 for bloom
///
/// // Enhance a dim red color
/// let enhanced = intensify_for_bloom((0.3, 0.1, 0.1), 2.0);
/// assert!(enhanced.0 < 1.0); // Dim colors receive less enhancement
/// ```
#[must_use]
pub fn intensify_for_bloom(rgb: (f32, f32, f32), intensity: f32) -> (f32, f32, f32) {
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
/// A tuple `(r, g, b)` of normalized RGB values in the range `[0.0, 1.0]`.
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
/// ```rust
/// use stardrift::utils::color::rgb_for_temp;
///
/// // Get RGB for the Sun's surface temperature
/// let (r, g, b) = rgb_for_temp(5778.0);
/// assert!(r > 0.9 && g > 0.8 && b > 0.7); // Warm white
///
/// // Get RGB for a red giant star
/// let (r, g, b) = rgb_for_temp(3500.0);
/// assert!(r > 0.99 && g > 0.7 && g < 0.8 && b > 0.5 && b < 0.6); // Orange
///
/// // Get RGB for a blue star
/// let (r, g, b) = rgb_for_temp(10000.0);
/// assert!(r < 0.9 && b > 0.9); // Blue-white
/// ```
///
/// # References
///
/// Based on Tanner Helland's approximation algorithm, which provides
/// accurate color temperature conversion for the range 1000K - 40000K.
///
/// See <https://tannerhelland.com/2012/09/18/convert-temperature-rgb-algorithm-code.html>.
#[must_use]
pub fn rgb_for_temp(temperature: f32) -> (f32, f32, f32) {
    let temp = temperature.clamp(MIN_TEMPERATURE, MAX_TEMPERATURE);

    let red = red_channel_for_temp(temp);
    let green = green_channel_for_temp(temp);
    let blue = blue_channel_for_temp(temp);

    let red_norm = red.clamp(0.0, MAX_COLOR_VALUE) / MAX_COLOR_VALUE;
    let green_norm = green.clamp(0.0, MAX_COLOR_VALUE) / MAX_COLOR_VALUE;
    let blue_norm = blue.clamp(0.0, MAX_COLOR_VALUE) / MAX_COLOR_VALUE;

    (red_norm, green_norm, blue_norm)
}

/// Calculates the red channel value for a given color temperature.
///
/// Returns 255.0 for temperatures at or below 6600K (warm colors),
/// and uses a power function for cooler temperatures.
fn red_channel_for_temp(temp: f32) -> f32 {
    if temp <= DAYLIGHT_TEMP_THRESHOLD {
        MAX_COLOR_VALUE
    } else {
        RED_COEFFICIENT * libm::powf(temp / 100.0 - RED_OFFSET, RED_EXPONENT)
    }
}

/// Calculates the green channel value for a given color temperature.
///
/// Uses logarithmic calculation for warm temperatures (≤6600K)
/// and power function for cool temperatures.
fn green_channel_for_temp(temp: f32) -> f32 {
    if temp <= DAYLIGHT_TEMP_THRESHOLD {
        GREEN_WARM_COEFFICIENT * libm::logf(temp / 100.0) + GREEN_WARM_OFFSET
    } else {
        GREEN_COOL_COEFFICIENT * libm::powf(temp / 100.0 - RED_OFFSET, GREEN_COOL_EXPONENT)
    }
}

/// Calculates the blue channel value for a given color temperature.
///
/// Returns 255.0 for cool temperatures (≥6600K), 0.0 for very warm
/// temperatures (<1900K), and uses logarithmic calculation in between.
fn blue_channel_for_temp(temp: f32) -> f32 {
    if temp >= DAYLIGHT_TEMP_THRESHOLD {
        MAX_COLOR_VALUE
    } else if temp < BLUE_TEMP_THRESHOLD {
        0.0
    } else {
        BLUE_COEFFICIENT * libm::logf(temp / 100.0 - BLUE_LOG_OFFSET) + BLUE_OFFSET
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
            let (r, g, b) = rgb_for_temp(temp as f32);

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
