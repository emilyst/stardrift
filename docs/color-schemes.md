# Color Schemes

Stardrift offers a variety of color schemes for celestial bodies, ranging from physics-based coloring to artistic palettes and pride flag themes. This guide describes each available scheme and helps you choose the right one for your use case. The long help output (`stardrift --help`) also lists every scheme with a one-line description under `--color-scheme`.

## Quick Reference

| Category | Schemes |
|----------|---------|
| Physics-Based | `black_body` |
| Colorblind-Safe | `deuteranopia_safe`, `protanopia_safe`, `tritanopia_safe`, `high_contrast` |
| Scientific | `viridis`, `plasma`, `inferno`, `turbo` |
| Aesthetic | `rainbow`, `pastel`, `neon`, `monochrome`, `vaporwave` |
| Pride Flags | `bisexual`, `transgender`, `lesbian`, `pansexual`, `nonbinary`, `asexual`, `genderfluid`, `aromantic`, `agender` |

## Configuration

Set your color scheme in the config file:

```toml
[rendering]
color_scheme = "viridis"
```

Or via command line:

```bash
stardrift --color-scheme plasma
```

## Physics-Based

### black_body (Default)

Colors based on black body radiation temperatures, simulating how real stars appear based on their surface temperature.

- **Smaller bodies** appear hotter (blue-white)
- **Larger bodies** appear cooler (red-orange)

This creates a physically intuitive visualization where body size correlates with apparent temperature.

**Configuration options:**

```toml
[rendering]
color_scheme = "black_body"
min_temperature = 3000.0   # Kelvin (cooler, redder)
max_temperature = 15000.0  # Kelvin (hotter, bluer)
```

**Temperature reference:**
| Temperature | Color | Real-World Example |
|-------------|-------|-------------------|
| 3,000 K | Deep red-orange | Red dwarf star |
| 5,800 K | Yellow-white | Our Sun |
| 10,000 K | Blue-white | Sirius A |
| 15,000 K | Blue | Hot blue giants |

## Colorblind-Safe Palettes

These schemes are designed to be distinguishable by people with various types of color vision deficiency.

### deuteranopia_safe

Optimized for deuteranopia (red-green colorblindness), the most common form affecting approximately 8% of males.

**Colors used:** Blue, orange, yellow, teal

Avoids red-green distinctions that would be difficult to perceive.

### protanopia_safe

Optimized for protanopia (red-blindness), where red cones are absent or non-functional.

**Colors used:** Blue, yellow, teal

Excludes red entirely, relying on blue-yellow contrast.

### tritanopia_safe

Optimized for tritanopia (blue-yellow colorblindness), a rare condition affecting blue cone perception.

**Colors used:** Red, green, magenta

Avoids blue-yellow distinctions, using red-green and magenta instead.

### high_contrast

Maximum distinguishability using widely separated hues and high saturation differences.

**Characteristics:**
- Large hue separation between colors
- High saturation for vivid distinction
- Works well for most types of color vision

**Best for:** Presentations, accessibility-focused use, situations requiring maximum clarity.

## Scientific Colormaps

These are perceptually uniform colormaps commonly used in scientific visualization. "Perceptually uniform" means that equal steps in the data produce visually equal steps in color, making them ideal for accurately representing continuous data.

### viridis

The modern standard for scientific visualization. A purple-blue-green-yellow gradient.

**Characteristics:**
- Perceptually uniform
- Colorblind-safe
- Prints well in grayscale
- High visual appeal

**Best for:** General use, scientific accuracy, accessibility.

### plasma

A magenta-purple-pink-yellow gradient with high visual impact.

**Characteristics:**
- Perceptually uniform
- Warm, vibrant appearance
- Good colorblind accessibility

**Best for:** Visually striking presentations, heat-map style visualization.

### inferno

A black-red-yellow-white gradient resembling heat or fire.

**Characteristics:**
- Perceptually uniform
- High dynamic range (black to white)
- Intuitive "intensity" interpretation

**Best for:** Representing intensity or energy, dramatic visuals.

### turbo

Google's improved rainbow colormap, addressing the perceptual issues of traditional rainbow palettes.

**Characteristics:**
- Near-perceptually-uniform
- Full spectrum coverage
- Better than naive rainbow implementations

**Best for:** When you want rainbow colors but with better perceptual properties.

## Aesthetic Themes

Artistic color schemes for visual appeal.

### rainbow

Random vibrant colors sampling the full hue spectrum at high saturation. Maximum variety.

### pastel

Soft, muted colors — low saturation, high lightness. Gentle and easy on the eyes.

### neon

Very high saturation concentrated in cyan, magenta, and lime for an electric, cyberpunk look.

### monochrome

Grayscale only (excluding pure black and white for visibility). Minimalist and print-friendly.

### vaporwave

Retrofuturistic 80s aesthetic: weighted toward pink and purple with cyan accents.

## Pride Flag Themes

Color schemes based on LGBTQ+ pride flags. Colors are distributed across bodies to represent the flag's design.

### bisexual

Based on the bisexual pride flag designed by Michael Page in 1998.

**Colors:** Pink, purple, blue (2:1:2 proportions)

The purple represents the blending of attraction to multiple genders.

### transgender

Based on the transgender pride flag designed by Monica Helms in 1999.

**Colors:** Light blue, pink, white

The symmetric design means the flag is always "correct" regardless of orientation.

### lesbian

Based on the 2018 orange-to-pink "sunset" lesbian flag.

**Colors:** Orange through pink gradient

This design has become the most widely recognized lesbian pride flag.

### pansexual

Based on the pansexual pride flag.

**Colors:** Pink, yellow, blue

Representing attraction regardless of gender identity.

### nonbinary

Based on the non-binary pride flag created by Kye Rowan in 2014.

**Colors:** Yellow, white, purple, black

- Yellow: gender outside the binary
- White: many or all genders
- Purple: mix of male and female
- Black: absence of gender

### asexual

Based on the asexual pride flag representing the ace spectrum.

**Colors:** Black, gray, white, purple

Representing the spectrum from asexual to sexual, with purple for community.

### genderfluid

Based on the genderfluid pride flag.

**Colors:** Pink, white, purple, black, blue

Representing the fluidity and spectrum of gender identity.

### aromantic

Based on the aromantic pride flag.

**Colors:** Green, white, gray, black

Representing the aromantic spectrum.

### agender

Based on the agender pride flag.

**Colors:** Black, gray, white, green (symmetric pattern)

Green represents non-binary identity; the symmetric black-gray-white pattern represents absence of gender.

## Tips for Choosing

| Goal | Recommended Schemes |
|------|-------------------|
| Scientific accuracy | `black_body`, `viridis` |
| Accessibility | `deuteranopia_safe`, `high_contrast`, `viridis` |
| Visual impact | `plasma`, `inferno`, `neon` |
| Relaxing viewing | `pastel`, `monochrome` |
| Fun/playful | `rainbow`, `vaporwave` |
| Representation | Pride flag themes |

## See Also

- [Configuration Reference](configuration.md) - Full rendering options
- [Usage Guide](usage.md) - Command-line options for color schemes
