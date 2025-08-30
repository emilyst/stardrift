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
//! use stardrift::resources::RenderingRng;
//!
//! fn create_materials(
//!     mut materials: ResMut<Assets<StandardMaterial>>,
//!     mut rng: ResMut<RenderingRng>
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

use crate::prelude::*;

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
pub fn random_rainbow_color(rng: &mut crate::resources::RenderingRng) -> (f32, f32, f32) {
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

    // Calculate grayscale using ITU-R BT.709 (HDTV) luminance formula for perceptual accuracy
    let gray = 0.2126 * r + 0.7152 * g + 0.0722 * b;

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

    // Scale using simple average for bloom (for artistic effect, not perceptual accuracy)
    // This ensures all vibrant colors produce visible bloom, including blues
    let luminance = (r + g + b) / 3.0;
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

// ============================================================================
// Colorblind-Safe Palettes
// ============================================================================

/// Generates a random color from the deuteranopia-safe palette.
///
/// Optimized for red-green colorblindness (affects ~8% of males).
/// Uses blue, orange, yellow, and white colors that are easily distinguishable.
pub fn deuteranopia_safe_color(rng: &mut crate::resources::RenderingRng) -> (f32, f32, f32) {
    use rand::prelude::*;

    let colors = [
        (0.0, 0.447, 0.698),   // Blue
        (0.902, 0.624, 0.0),   // Orange
        (0.835, 0.369, 0.0),   // Dark orange
        (0.0, 0.620, 0.451),   // Teal
        (0.941, 0.894, 0.259), // Yellow
        (0.337, 0.706, 0.914), // Sky blue
        (0.8, 0.475, 0.655),   // Pink
        (0.95, 0.95, 0.95),    // Near white
    ];

    colors[rng.random_range(0..colors.len())]
}

/// Generates a random color from the protanopia-safe palette.
///
/// Optimized for red-blindness. Uses blue, yellow, and teal colors.
pub fn protanopia_safe_color(rng: &mut crate::resources::RenderingRng) -> (f32, f32, f32) {
    use rand::prelude::*;

    let colors = [
        (0.0, 0.267, 0.533),   // Navy blue
        (0.0, 0.533, 0.8),     // Blue
        (0.333, 0.667, 0.933), // Light blue
        (0.0, 0.667, 0.667),   // Teal
        (0.667, 0.667, 0.0),   // Olive
        (0.933, 0.867, 0.0),   // Yellow
        (0.467, 0.0, 0.533),   // Purple
        (0.9, 0.9, 0.9),       // Light gray
    ];

    colors[rng.random_range(0..colors.len())]
}

/// Generates a random color from the tritanopia-safe palette.
///
/// Optimized for blue-yellow colorblindness (rare). Uses red, green, and magenta.
pub fn tritanopia_safe_color(rng: &mut crate::resources::RenderingRng) -> (f32, f32, f32) {
    use rand::prelude::*;

    let colors = [
        (0.894, 0.102, 0.110), // Red
        (0.702, 0.0, 0.0),     // Dark red
        (0.216, 0.494, 0.722), // Blue (still visible)
        (0.302, 0.686, 0.290), // Green
        (0.0, 0.392, 0.0),     // Dark green
        (0.596, 0.306, 0.639), // Purple
        (1.0, 0.498, 0.0),     // Orange
        (0.95, 0.95, 0.95),    // Near white
    ];

    colors[rng.random_range(0..colors.len())]
}

/// Generates a random high-contrast color for maximum distinguishability.
///
/// Uses widely separated hues with high saturation differences.
pub fn high_contrast_color(rng: &mut crate::resources::RenderingRng) -> (f32, f32, f32) {
    use rand::prelude::*;

    let colors = [
        (1.0, 1.0, 1.0), // White
        (0.0, 0.0, 0.0), // Black
        (1.0, 0.0, 0.0), // Red
        (0.0, 1.0, 0.0), // Green
        (0.0, 0.0, 1.0), // Blue
        (1.0, 1.0, 0.0), // Yellow
        (1.0, 0.0, 1.0), // Magenta
        (0.0, 1.0, 1.0), // Cyan
    ];

    colors[rng.random_range(0..colors.len())]
}

// ============================================================================
// Scientific Colormaps
// ============================================================================

/// Generates a random color from the Viridis colormap.
///
/// Purple-blue-green-yellow gradient that is perceptually uniform and colorblind-safe.
pub fn viridis_color(rng: &mut crate::resources::RenderingRng) -> (f32, f32, f32) {
    use rand::prelude::*;

    let t = rng.random::<f32>();
    viridis_gradient(t)
}

/// Maps a value [0,1] to a color in the Viridis colormap.
fn viridis_gradient(t: f32) -> (f32, f32, f32) {
    let t = t.clamp(0.0, 1.0);

    // Approximation of Viridis colormap using key points
    if t < 0.25 {
        let s = t * 4.0;
        lerp_rgb((0.267, 0.004, 0.329), (0.282, 0.140, 0.457), s)
    } else if t < 0.5 {
        let s = (t - 0.25) * 4.0;
        lerp_rgb((0.282, 0.140, 0.457), (0.163, 0.471, 0.558), s)
    } else if t < 0.75 {
        let s = (t - 0.5) * 4.0;
        lerp_rgb((0.163, 0.471, 0.558), (0.316, 0.718, 0.424), s)
    } else {
        let s = (t - 0.75) * 4.0;
        lerp_rgb((0.316, 0.718, 0.424), (0.993, 0.906, 0.144), s)
    }
}

/// Generates a random color from the Plasma colormap.
///
/// Magenta-purple-pink-yellow gradient with high visual appeal.
pub fn plasma_color(rng: &mut crate::resources::RenderingRng) -> (f32, f32, f32) {
    use rand::prelude::*;

    let t = rng.random::<f32>();
    plasma_gradient(t)
}

/// Maps a value [0,1] to a color in the Plasma colormap.
fn plasma_gradient(t: f32) -> (f32, f32, f32) {
    let t = t.clamp(0.0, 1.0);

    // Approximation of Plasma colormap
    if t < 0.25 {
        let s = t * 4.0;
        lerp_rgb((0.050, 0.030, 0.528), (0.294, 0.012, 0.631), s)
    } else if t < 0.5 {
        let s = (t - 0.25) * 4.0;
        lerp_rgb((0.294, 0.012, 0.631), (0.566, 0.053, 0.684), s)
    } else if t < 0.75 {
        let s = (t - 0.5) * 4.0;
        lerp_rgb((0.566, 0.053, 0.684), (0.875, 0.393, 0.502), s)
    } else {
        let s = (t - 0.75) * 4.0;
        lerp_rgb((0.875, 0.393, 0.502), (0.940, 0.975, 0.131), s)
    }
}

/// Generates a random color from the Inferno colormap.
///
/// Black-red-yellow-white heat map for intensity visualization.
pub fn inferno_color(rng: &mut crate::resources::RenderingRng) -> (f32, f32, f32) {
    use rand::prelude::*;

    let t = rng.random::<f32>();
    inferno_gradient(t)
}

/// Maps a value [0,1] to a color in the Inferno colormap.
fn inferno_gradient(t: f32) -> (f32, f32, f32) {
    let t = t.clamp(0.0, 1.0);

    // Approximation of Inferno colormap
    if t < 0.25 {
        let s = t * 4.0;
        lerp_rgb((0.001, 0.000, 0.014), (0.258, 0.039, 0.407), s)
    } else if t < 0.5 {
        let s = (t - 0.25) * 4.0;
        lerp_rgb((0.258, 0.039, 0.407), (0.573, 0.106, 0.467), s)
    } else if t < 0.75 {
        let s = (t - 0.5) * 4.0;
        lerp_rgb((0.573, 0.106, 0.467), (0.866, 0.387, 0.290), s)
    } else {
        let s = (t - 0.75) * 4.0;
        lerp_rgb((0.866, 0.387, 0.290), (0.988, 0.998, 0.645), s)
    }
}

/// Generates a random color from the Turbo colormap.
///
/// Google's improved rainbow colormap with better perceptual properties.
pub fn turbo_color(rng: &mut crate::resources::RenderingRng) -> (f32, f32, f32) {
    let t = rng.random::<f32>();
    turbo_gradient(t)
}

/// Maps a value [0,1] to a color in the Turbo colormap.
fn turbo_gradient(t: f32) -> (f32, f32, f32) {
    let t = t.clamp(0.0, 1.0);

    // Approximation of Turbo colormap (simplified version)
    if t < 0.14 {
        let s = t / 0.14;
        lerp_rgb((0.189, 0.071, 0.232), (0.125, 0.371, 0.656), s)
    } else if t < 0.28 {
        let s = (t - 0.14) / 0.14;
        lerp_rgb((0.125, 0.371, 0.656), (0.057, 0.640, 0.693), s)
    } else if t < 0.42 {
        let s = (t - 0.28) / 0.14;
        lerp_rgb((0.057, 0.640, 0.693), (0.163, 0.844, 0.442), s)
    } else if t < 0.56 {
        let s = (t - 0.42) / 0.14;
        lerp_rgb((0.163, 0.844, 0.442), (0.559, 0.968, 0.113), s)
    } else if t < 0.70 {
        let s = (t - 0.56) / 0.14;
        lerp_rgb((0.559, 0.968, 0.113), (0.915, 0.846, 0.075), s)
    } else if t < 0.85 {
        let s = (t - 0.70) / 0.15;
        lerp_rgb((0.915, 0.846, 0.075), (0.989, 0.513, 0.069), s)
    } else {
        let s = (t - 0.85) / 0.15;
        lerp_rgb((0.989, 0.513, 0.069), (0.479, 0.010, 0.010), s)
    }
}

// ============================================================================
// Aesthetic Themes
// ============================================================================

/// Generates a random pastel color.
///
/// Soft, low-saturation colors with high lightness.
pub fn pastel_color(rng: &mut crate::resources::RenderingRng) -> (f32, f32, f32) {
    use rand::prelude::*;

    let hue = rng.random_range(0.0..=360.0);
    let saturation = rng.random_range(0.3..=0.5);
    let lightness = rng.random_range(0.7..=0.85);

    let color = Color::hsl(hue, saturation, lightness);
    let linear_rgba = LinearRgba::from(color);
    (linear_rgba.red, linear_rgba.green, linear_rgba.blue)
}

/// Generates a random neon color.
///
/// High saturation, bright cyberpunk-style colors with electric appearance.
pub fn neon_color(rng: &mut crate::resources::RenderingRng) -> (f32, f32, f32) {
    use rand::prelude::*;

    // Limited hue ranges for cohesive neon palette
    let hue_ranges = [
        (280.0, 320.0), // Purple/magenta
        (180.0, 210.0), // Cyan
        (120.0, 150.0), // Green
        (40.0, 60.0),   // Yellow/orange
        (0.0, 20.0),    // Red
    ];

    let range = hue_ranges[rng.random_range(0..hue_ranges.len())];
    let hue = rng.random_range(range.0..=range.1);
    let saturation = 1.0;
    let lightness = rng.random_range(0.65..=0.75);

    let color = Color::hsl(hue, saturation, lightness);
    let linear_rgba = LinearRgba::from(color);
    (linear_rgba.red, linear_rgba.green, linear_rgba.blue)
}

/// Generates a random monochrome (grayscale) color.
///
/// Different lightness levels with no hue or saturation.
pub fn monochrome_color(rng: &mut crate::resources::RenderingRng) -> (f32, f32, f32) {
    use rand::prelude::*;

    // Avoid pure black and white for better visibility
    let gray = rng.random_range(0.2..=0.9);
    (gray, gray, gray)
}

/// Generates a random vaporwave color.
///
/// Retrofuturistic aesthetic with signature pink-purple-cyan palette.
/// Colors are electric yet dreamy, capturing 1980s Miami neon meets Japanese city pop.
pub fn vaporwave_color(rng: &mut crate::resources::RenderingRng) -> (f32, f32, f32) {
    use rand::prelude::*;

    // Vaporwave signature color ranges with weighted selection
    let color_weights = [
        (300.0, 330.0, 3), // Hot pink/magenta (most iconic - higher weight)
        (270.0, 290.0, 2), // Purple/violet (classic vaporwave)
        (170.0, 190.0, 3), // Cyan/turquoise (80s terminal cyan - higher weight)
        (200.0, 220.0, 1), // Soft electric blue
        (150.0, 165.0, 1), // Mint green (occasional accent)
        (15.0, 30.0, 1),   // Sunset peach/coral (occasional)
    ];

    // Build weighted selection
    let total_weight: u32 = color_weights.iter().map(|(_, _, w)| w).sum();
    let mut selection = rng.random_range(0..total_weight);

    let mut chosen_range = (0.0, 0.0);
    for (start, end, weight) in color_weights.iter() {
        if selection < *weight {
            chosen_range = (*start, *end);
            break;
        }
        selection -= *weight;
    }

    let hue = rng.random_range(chosen_range.0..=chosen_range.1);
    // Vaporwave uses high but not always max saturation for depth
    let saturation = rng.random_range(0.85..=1.0);
    // Bright but with some variation for that dreamy quality
    let lightness = rng.random_range(0.62..=0.72);

    let color = Color::hsl(hue, saturation, lightness);
    let linear_rgba = LinearRgba::from(color);
    (linear_rgba.red, linear_rgba.green, linear_rgba.blue)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Linear interpolation between two RGB colors.
fn lerp_rgb(from: (f32, f32, f32), to: (f32, f32, f32), t: f32) -> (f32, f32, f32) {
    (
        from.0 + (to.0 - from.0) * t,
        from.1 + (to.1 - from.1) * t,
        from.2 + (to.2 - from.2) * t,
    )
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

    #[test]
    fn test_lerp_rgb_endpoints() {
        let black = (0.0, 0.0, 0.0);
        let white = (1.0, 1.0, 1.0);

        // At t=0, should return 'from' color
        let result = lerp_rgb(black, white, 0.0);
        assert_eq!(result, black);

        // At t=1, should return 'to' color
        let result = lerp_rgb(black, white, 1.0);
        assert_eq!(result, white);
    }

    #[test]
    fn test_lerp_rgb_midpoint() {
        let red = (1.0, 0.0, 0.0);
        let blue = (0.0, 0.0, 1.0);

        // At t=0.5, should return midpoint
        let result = lerp_rgb(red, blue, 0.5);
        assert_eq!(result, (0.5, 0.0, 0.5));

        // Test with different colors
        let color1 = (0.2, 0.4, 0.6);
        let color2 = (0.8, 0.6, 0.4);
        let result = lerp_rgb(color1, color2, 0.5);
        assert_eq!(result, (0.5, 0.5, 0.5));
    }

    #[test]
    fn test_lerp_rgb_quarter_points() {
        let start = (0.0, 0.0, 0.0);
        let end = (1.0, 1.0, 1.0);

        // At t=0.25
        let result = lerp_rgb(start, end, 0.25);
        assert_eq!(result, (0.25, 0.25, 0.25));

        // At t=0.75
        let result = lerp_rgb(start, end, 0.75);
        assert_eq!(result, (0.75, 0.75, 0.75));
    }

    #[test]
    fn test_lerp_rgb_negative_direction() {
        // Test interpolation when 'to' values are smaller than 'from'
        let bright = (0.8, 0.9, 1.0);
        let dark = (0.2, 0.1, 0.0);

        let result = lerp_rgb(bright, dark, 0.5);
        assert_eq!(result, (0.5, 0.5, 0.5));

        let result = lerp_rgb(bright, dark, 0.25);
        assert_eq!(result, (0.65, 0.7, 0.75));
    }

    #[test]
    fn test_lerp_rgb_extrapolation() {
        // Test behavior with t values outside [0,1]
        // This tests the mathematical behavior, though in practice
        // t should be clamped to [0,1] before calling
        let color1 = (0.2, 0.3, 0.4);
        let color2 = (0.6, 0.7, 0.8);

        // t > 1 should extrapolate beyond color2
        let result = lerp_rgb(color1, color2, 1.5);
        assert!((result.0 - 0.8).abs() < 0.0001);
        assert!((result.1 - 0.9).abs() < 0.0001);
        assert!((result.2 - 1.0).abs() < 0.0001);

        // t < 0 should extrapolate before color1
        let result = lerp_rgb(color1, color2, -0.5);
        assert!((result.0 - 0.0).abs() < 0.0001);
        assert!((result.1 - 0.1).abs() < 0.0001);
        assert!((result.2 - 0.2).abs() < 0.0001);
    }

    #[test]
    fn test_lerp_rgb_precision() {
        // Test that lerp maintains reasonable precision
        let color1 = (0.123456, 0.234567, 0.345678);
        let color2 = (0.876543, 0.765432, 0.654321);

        let result = lerp_rgb(color1, color2, 0.333);

        // Expected values calculated manually:
        // r: 0.123456 + (0.876543 - 0.123456) * 0.333 = 0.374235
        // g: 0.234567 + (0.765432 - 0.234567) * 0.333 = 0.411355
        // b: 0.345678 + (0.654321 - 0.345678) * 0.333 = 0.448472
        assert!((result.0 - 0.374235).abs() < 0.0001);
        assert!((result.1 - 0.411355).abs() < 0.0001);
        assert!((result.2 - 0.448472).abs() < 0.0001);
    }

    #[test]
    fn test_lerp_rgb_same_colors() {
        // When interpolating between the same color, result should be that color
        let color = (0.5, 0.6, 0.7);

        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let result = lerp_rgb(color, color, t);
            assert_eq!(result, color);
        }
    }

    #[test]
    fn test_lerp_rgb_gradient_consistency() {
        // Verify that lerp creates a smooth gradient
        let start = (0.0, 0.0, 0.0);
        let end = (1.0, 0.5, 0.25);

        let mut prev = start;
        for i in 1..=10 {
            let t = i as f32 / 10.0;
            let current = lerp_rgb(start, end, t);

            // Each component should increase monotonically
            assert!(current.0 >= prev.0);
            assert!(current.1 >= prev.1);
            assert!(current.2 >= prev.2);

            // Check the rate of change is consistent
            if i > 1 {
                let delta_r = current.0 - prev.0;
                assert!((delta_r - 0.1).abs() < 0.0001);

                let delta_g = current.1 - prev.1;
                assert!((delta_g - 0.05).abs() < 0.0001);

                let delta_b = current.2 - prev.2;
                assert!((delta_b - 0.025).abs() < 0.0001);
            }

            prev = current;
        }
    }
}
