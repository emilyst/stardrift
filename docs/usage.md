# Usage Guide

How to use Stardrift: controls, command-line options, and automated screenshots.

## Controls

### Keyboard and Mouse

| Key/Action      | Function                                  |
|-----------------|-------------------------------------------|
| **Left-drag**   | Orbit camera                              |
| **Right-drag**  | Pan camera                                |
| **Mouse Wheel** | Zoom in/out                               |
| **Space**       | Pause/Resume simulation                   |
| **N**           | New simulation with new random bodies     |
| **O**           | Toggle octree visualization               |
| **C**           | Toggle barycenter gizmo                   |
| **T**           | Toggle trails                             |
| **D**           | Toggle diagnostics HUD                    |
| **S**           | Take screenshot (hides UI and HUD)        |
| **Q** / **Escape** | Quit (desktop only)                    |

Every action except quitting also has an on-screen button in the bottom-left
button column.

### Touch and Trackpad

On touch devices: one-finger drag orbits, two-finger drag pans, and pinch
zooms. On a trackpad, scrolling orbits, **Shift**+scroll pans, **Ctrl**+scroll
zooms, and pinch zooms.

### Camera Behavior

The camera orbits a fixed focus at the world origin. Bodies spawn in the
center-of-momentum frame, so the system's barycenter starts at the origin and
stays nearby; the camera does not track it. The initial camera distance is
scaled to fit the spawn region (adjustable via
`rendering.camera_radius_multiplier` in the [configuration](configuration.md)).

## Command-Line Options

```bash
stardrift [OPTIONS]
```

`stardrift --help` is the authoritative reference for all flags and their
descriptions. The most commonly used:

| Option | Description |
|--------|-------------|
| `-n, --bodies COUNT` | Number of bodies to simulate |
| `-s, --seed SEED` | Random seed for reproducible simulations |
| `-g, --gravity VALUE` | Gravitational constant |
| `-i, --integrator NAME` | Numerical integrator (see [Integrators Guide](integrators.md)) |
| `--color-scheme NAME` | Color scheme for bodies (see [Color Schemes](color-schemes.md)) |
| `-p, --paused` | Start paused |
| `--windowed` / `--fullscreen` | Regular window or borderless fullscreen (the default); Escape quits either way |
| `-c, --config FILE` | Use a specific config file |
| `-v, --verbose` | Debug logging (includes a dump of the effective configuration) |
| `--bh-probe` | Measure Barnes-Hut error and octree churn against exact summation; logged and shown in the diagnostics HUD (see [Integration](integration.md#instrumented-what-theta-costs)) |
| `--bench-mode` | Benchmarking: ignore the user config file (other flags still apply), borderless fullscreen with vsync off, frame-time diagnostics logged at info level. Conflicts with `--config` |
| `--list-integrators` | List available integrators and aliases, then exit |
| `--print-default-config` | Print the default configuration as TOML, then exit |

Command-line values override the [configuration file](configuration.md).

### Example Commands

```bash
# Reproducible simulation with a specific body count
stardrift --bodies 50 --seed 123

# Use a different integrator for better energy conservation
stardrift --integrator pefrl

# Start paused to set up the view before simulation begins
stardrift --paused --bodies 200

# Generate identical simulations with different color schemes
for scheme in viridis plasma inferno turbo; do
    stardrift --seed 42 --bodies 50 --color-scheme $scheme
done

# Measure frame times at 1000 bodies from a known baseline (takes over the
# screen; windowed macOS pins to the display refresh rate regardless of
# present mode, which is why bench mode goes fullscreen)
cargo run --profile perf -p stardrift -- --bench-mode --seed 42 --bodies 1000
```

## Screenshots

### Manual Screenshots

Press **S** (or the Screenshot button) to capture at any time. Manual
screenshots hide the UI and HUD before capture, save as PNG at full window
resolution to the configured directory (current directory by default), and
include a timestamp in the filename. See the
[screenshot configuration](configuration.md#screenshot-configuration) to
customize this.

### Automated Screenshots

The `--screenshot-*` family of flags captures screenshots on a schedule,
useful for testing and generating material. See `stardrift --help` for the
full list.

```bash
# Take a single screenshot after 2 seconds
stardrift --screenshot-after 2 --exit-after-screenshots

# Take 5 screenshots at 1-second intervals
stardrift --screenshot-interval 1 --screenshot-count 5 --exit-after-screenshots

# Use frame-based timing for deterministic captures
stardrift --screenshot-after 120 --screenshot-use-frames
```

Note that `--screenshot-count` only takes effect alongside
`--screenshot-after` or `--screenshot-interval` — one of those two defines the
schedule.

#### Deterministic Captures for Testing

Frame-based timing with a fixed seed produces identical screenshots on every
run, suitable for regression testing:

```bash
stardrift --seed 42 --bodies 100 \
          --screenshot-after 60 --screenshot-use-frames \
          --screenshot-dir ./test_output \
          --screenshot-name ui_state \
          --screenshot-no-timestamp \
          --screenshot-list-paths \
          --exit-after-screenshots

# Output: SCREENSHOT_PATH: ./test_output/ui_state.png
```

#### Automated vs Manual Screenshots

| Behavior | Manual (S key) | Automated (CLI) |
|----------|----------------|-----------------|
| UI visibility | Hidden during capture | Preserved (for UI testing) |
| Timing | Instant | Configurable delay/interval |
| Exit behavior | Continues running | Optional auto-exit |

## User Interface

The bottom-left button column mirrors the keyboard shortcuts: New Simulation,
Show/Hide Octree, Show/Hide Barycenter, Show/Hide Trails, Show/Hide
Diagnostics, Pause/Resume, Screenshot, and (on desktop) Quit.

At startup, trails are visible; the octree wireframe, barycenter gizmo, and
diagnostics HUD are hidden until toggled.

- **Octree visualization**: wireframe boxes showing the Barnes-Hut spatial
  partitioning
- **Barycenter gizmo**: a cross-hair at the system's center of mass
- **Trails**: fading paths showing each body's recent motion
- **Diagnostics HUD**: FPS, frame count, and body count

## Platform-Specific Notes

### Desktop (Windows, macOS, Linux)

The native desktop build offers the best performance. Use release builds for
smooth simulation of larger body counts:

```bash
cargo run -p stardrift --release
```

### WebAssembly (Browser)

The WASM version requires a browser with WebGPU support (Chrome/Edge 113+,
Safari 26+, Firefox 141+ on Windows and later releases elsewhere). There is
no WebGL2 fallback. Differences from desktop:

- Performance may be lower than native builds
- No configuration file or command line; built-in defaults are used
- No quit control or screen sleep prevention
- A "Compiling shaders…" screen shows while WebGPU compiles the render
  pipelines; on Safari this can take several seconds on a cold start. The
  simulation does not advance until it finishes.

## See Also

- [Configuration Reference](configuration.md) - Customize simulation parameters
- [Integrators Guide](integrators.md) - Choose the right numerical integrator
- [Color Schemes](color-schemes.md) - Available color palettes
