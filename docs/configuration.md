# Configuration Reference

Stardrift uses a TOML-based configuration file for customizing simulation behavior, rendering, and other settings. This document provides a complete reference for all configuration options.

## Configuration File Location

The configuration file is automatically loaded from platform-specific directories:

| Platform | Path |
|----------|------|
| **Linux** | `$XDG_CONFIG_HOME/stardrift/config.toml` (usually `~/.config/stardrift/config.toml`) |
| **macOS** | `~/Library/Application Support/Stardrift/config.toml` |
| **Windows** | `%APPDATA%\Stardrift\config\config.toml` |

A different file can be specified with `--config FILE`. WebAssembly builds do not load a configuration file.

The file is layered over the built-in defaults key by key, so it only needs to contain the settings you want to change. If no file exists (or it fails to parse, with a warning), the defaults are used. Run with `--verbose` to log the full effective configuration at startup.

To seed a config file with the complete defaults:

```bash
stardrift --print-default-config > path/to/config.toml
```

## Quick Start

Here's a minimal configuration file to get started:

```toml
[physics]
body_count = 150

[rendering]
color_scheme = "viridis"

[trails]
trail_length_seconds = 15.0
```

## Physics Configuration

The `[physics]` section controls the simulation's physical behavior.

### Core Physics Parameters

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `gravitational_constant` | `f64` | `100.0` | Strength of gravitational attraction between bodies. Higher values create stronger gravity. |
| `body_count` | `usize` | `25` | Number of celestial bodies to simulate |
| `initial_seed` | `Option<u64>` | `None` | Random seed for deterministic body generation. `None` uses a random seed each run. |

### Barnes-Hut Algorithm

The simulation uses the Barnes-Hut algorithm for efficient O(N log N) force calculations.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `octree_theta` | `f64` | `0.5` | Accuracy parameter (0.0-2.0). Lower values are more accurate but slower. `0.0` = exact N-body calculation. |
| `octree_leaf_threshold` | `usize` | `1` | Maximum bodies per octree leaf node before subdivision |

**Theta parameter guidance:**
- `0.0` - Exact calculation (O(N²), no approximation)
- `0.3` - High accuracy, slower
- `0.5` - Good balance (default)
- `0.8` - Faster, lower accuracy
- `1.0+` - Fast but may show artifacts

### Body Generation

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min_body_radius` | `f32` | `2.0` | Minimum radius for generated bodies |
| `max_body_radius` | `f32` | `4.0` | Maximum radius for generated bodies |

The `[physics.body_distribution]` subsection controls the spawn shell bodies are placed on:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min_spacing` | `f32` | `500.0` | Target minimum center-to-center spacing between spawned bodies, in world units; the spawn-shell radius is derived from it |
| `radius_tolerance` | `f32` | `0.001` | Relative convergence tolerance for the spawn-shell radius solve |

### Force Calculation Limits

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `force_calculation_min_distance` | `f64` | `1.0` | Minimum distance for force calculations (softening parameter to prevent singularities) |
| `force_calculation_max_force` | `f64` | `10000000.0` | Maximum force magnitude to prevent numerical instabilities. With collisions enabled, pair separations are bounded by contact distance and this clamp only matters for pathological configurations; sized to clear the default scene's fully-merged endgame (contact force between merged clumps grows as N^(4/3)) |

### Simulation Behavior

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `barycentric_drift_correction` | `bool` | `false` | Recenter the simulation around the barycenter every step. Off by default: bodies spawn in the center-of-momentum frame, so residual barycenter motion is a useful Barnes-Hut asymmetry diagnostic |

### Collisions

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `collisions.enabled` | `bool` | `true` | Merge bodies on contact (perfectly inelastic: mass sums, momentum conserved, radius from volume conservation). Detection is swept, so fast bodies cannot tunnel through each other between steps |
| `collisions.contact_factor` | `f64` | `1.0` | Multiplier on the sum of two bodies' radii that counts as contact. Below `1.0` requires overlap; above `1.0` merges early |

### Integrator Selection

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `integrator.type` | `string` | `"velocity_verlet"` | Numerical integration method |

Available integrators (use snake_case):
- `"symplectic_euler"` - 1st order symplectic
- `"velocity_verlet"` - 2nd order symplectic (recommended)
- `"pefrl"` - 4th order symplectic (best energy conservation)
- `"explicit_euler"` - 1st order explicit (educational use only)
- `"heun"` - 2nd order explicit
- `"runge_kutta_second_order_midpoint"` - 2nd order explicit
- `"runge_kutta_fourth_order"` - 4th order explicit

An unrecognized integrator name in the config file logs a warning and falls back to `velocity_verlet`; an unrecognized name passed via `--integrator` is an error. Run `stardrift --list-integrators` for the accepted names and aliases.

For detailed information on choosing an integrator, see the [Integrators Guide](integrators.md).

### Initial Velocity Configuration

The `[physics.initial_velocity]` subsection controls how bodies are given their starting velocities.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Whether bodies spawn with initial velocities |
| `min_speed` | `f64` | `5.0` | Minimum initial speed |
| `max_speed` | `f64` | `5.0` | Maximum initial speed |
| `velocity_mode` | `string` | `"orbital"` | Velocity distribution mode |
| `tangential_bias` | `f64` | `0.7` | Bias toward tangential motion (0.0-1.0) for `"random"` mode |

**Velocity modes:**

| Mode | Description |
|------|-------------|
| `"random"` | Random velocity vectors with configurable tangential bias |
| `"orbital"` | Circular orbital velocities around the barycenter |
| `"tangential"` | Pure tangential motion perpendicular to radius from center |
| `"radial"` | Pure radial motion toward/away from barycenter |

**Example:**

```toml
[physics.initial_velocity]
enabled = true
velocity_mode = "orbital"
min_speed = 20.0
max_speed = 80.0
```

## Rendering Configuration

The `[rendering]` section controls visual appearance.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `color_scheme` | `string` | `"black_body"` | Color scheme for celestial bodies |
| `min_temperature` | `f32` | `3000.0` | Minimum stellar temperature in Kelvin (for `"black_body"` scheme) |
| `max_temperature` | `f32` | `15000.0` | Maximum stellar temperature in Kelvin (for `"black_body"` scheme) |
| `bloom_intensity` | `f32` | `250.0` | Intensity of bloom visual effect |
| `saturation_intensity` | `f32` | `3.0` | Color saturation multiplier |
| `camera_radius_multiplier` | `f32` | `3.0` | Camera distance relative to system size |

For the complete list of color schemes and their descriptions, see [Color Schemes](color-schemes.md).

**Example:**

```toml
[rendering]
color_scheme = "viridis"
bloom_intensity = 300.0
saturation_intensity = 2.5
camera_radius_multiplier = 5.0
```

## Trail Configuration

The `[trails]` section controls the visual trails behind moving bodies.

### Basic Trail Settings

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `trail_length_seconds` | `f32` | `30.0` | How long trails persist in seconds |
| `update_interval_seconds` | `f32` | `0.1` | How often to record trail points (10 Hz) |
| `max_points_per_trail` | `usize` | `10000` | Maximum trail points per body |

### Trail Appearance

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `base_width` | `f32` | `1.0` | Base trail width |
| `width_relative_to_body` | `bool` | `true` | Scale trail width relative to body size |
| `body_size_multiplier` | `f32` | `0.7` | Trail width multiplier when `width_relative_to_body` is true |
| `bloom_factor` | `f32` | `1.0` | Trail bloom intensity multiplier |
| `use_additive_blending` | `bool` | `true` | Use additive blending for glowing effect |

### Trail Fading

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enable_fading` | `bool` | `true` | Enable trail fade-out effect |
| `fade_curve` | `string` | `"exponential"` | Fade curve type |
| `min_alpha` | `f32` | `0.0` | Minimum trail transparency (0.0 = fully transparent) |
| `max_alpha` | `f32` | `0.3333` | Maximum trail transparency (1.0 = fully opaque) |

**Fade curve types:**

| Curve | Description |
|-------|-------------|
| `"linear"` | Constant fade rate from head to tail |
| `"exponential"` | Rapid initial fade, then gradual (default) |
| `"smooth_step"` | S-curve with smooth transitions at both ends |
| `"ease_in_out"` | Slow start and end, faster in the middle |

### Trail Tapering

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enable_tapering` | `bool` | `true` | Enable trail width tapering (driven by point age toward expiry, so young trails stay uniform width) |
| `taper_curve` | `string` | `"smooth_step"` | Taper curve type |
| `min_width_ratio` | `f32` | `0.6` | Width ratio at full point age (0.6 = 60% of base width) |

**Taper curve types:** `"linear"`, `"exponential"`, `"smooth_step"`

**Example:**

```toml
[trails]
trail_length_seconds = 15.0
base_width = 1.5
enable_fading = true
fade_curve = "smooth_step"
max_alpha = 0.5
enable_tapering = true
taper_curve = "exponential"
```

## Screenshot Configuration

The `[screenshots]` section controls screenshot behavior.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `directory` | `Option<String>` | `None` | Save directory (created if missing). `None` = current working directory. A leading `~` expands to your home directory |
| `filename_prefix` | `String` | `"stardrift_screenshot"` | Filename prefix for screenshots |
| `include_timestamp` | `bool` | `true` | Include timestamp in filenames |
| `notification_enabled` | `bool` | `true` | Log screenshot captures to console |
| `hide_ui_frame_delay` | `u32` | `2` | Frames to wait before capture (ensures UI is hidden) |

**Example:**

```toml
[screenshots]
directory = "~/Pictures/Stardrift"
filename_prefix = "sim"
include_timestamp = true
```

## System Configuration

The `[system]` section controls system-level behavior.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prevent_screen_sleep` | `bool` | `true` | Prevent display from sleeping during simulation. Override per run with `--prevent-screen-sleep` / `--no-prevent-screen-sleep` |

## Complete Example

Here's a comprehensive configuration file demonstrating many options:

```toml
[physics]
gravitational_constant = 150.0
body_count = 200
octree_theta = 0.5
initial_seed = 42
barycentric_drift_correction = false

[physics.collisions]
enabled = true
contact_factor = 1.0

[physics.integrator]
type = "velocity_verlet"

[physics.initial_velocity]
enabled = true
velocity_mode = "orbital"
min_speed = 15.0
max_speed = 60.0

[rendering]
color_scheme = "plasma"
bloom_intensity = 280.0
saturation_intensity = 2.8
camera_radius_multiplier = 4.5

[trails]
trail_length_seconds = 12.0
base_width = 1.2
fade_curve = "exponential"
max_alpha = 0.4
taper_curve = "linear"

[screenshots]
directory = "~/Pictures/Stardrift"
filename_prefix = "capture"

[system]
prevent_screen_sleep = true
```

## Command-Line Overrides

Several settings (body count, seed, gravitational constant, integrator, color scheme) can be overridden on the command line, taking precedence over the configuration file. Run `stardrift --help` for the full list.

## See Also

- [Usage Guide](usage.md) - Controls and command-line usage
- [Integrators Guide](integrators.md) - Detailed integrator selection advice
- [Color Schemes](color-schemes.md) - Available color palettes with descriptions
