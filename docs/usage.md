# Usage Guide

This guide covers how to use Stardrift, including controls, command-line options, and advanced features like automated screenshots.

## Controls

### Keyboard and Mouse

| Key/Action      | Function                                        |
|-----------------|-------------------------------------------------|
| **Left-drag**   | Orbit camera around the simulation              |
| **Right-drag**  | Pan camera                                      |
| **Mouse Wheel** | Zoom in/out                                     |
| **Space**       | Pause/Resume simulation                         |
| **N**           | New simulation with new random bodies           |
| **O**           | Toggle octree visualization on/off              |
| **C**           | Toggle barycenter gizmo visibility on/off       |
| **T**           | Toggle trail visibility on/off                  |
| **D**           | Toggle diagnostics HUD visibility on/off        |
| **S**           | Take screenshot (hides UI and HUD)              |
| **Escape**      | Quit application                                |

### Touch Controls

On mobile and tablet devices, touch gestures are supported:

- **Single finger drag**: Pan and orbit the camera
- **Pinch**: Zoom in/out
- **Two finger drag**: Pan the view

### Camera Behavior

The camera automatically follows the barycenter (center of mass) of the system. As bodies interact gravitationally and the system's center of mass shifts, the camera tracks this movement to keep the action in view.

- Pan and orbit controls allow you to view the simulation from any angle
- Zoom controls let you get close to individual bodies or see the entire system
- The camera smoothly interpolates its position to avoid jarring movements

## Command-Line Options

```bash
stardrift [OPTIONS]
```

### Simulation Options

| Option | Description |
|--------|-------------|
| `--bodies COUNT` | Set number of bodies to simulate (default: 100) |
| `--seed SEED` | Use specific random seed for reproducible simulations |
| `--paused` | Start simulation in paused state |
| `--integrator NAME` | Select numerical integrator (see [Integrators Guide](integrators.md)) |

### Display Options

| Option | Description |
|--------|-------------|
| `--color-scheme NAME` | Color scheme for bodies (see [Color Schemes](color-schemes.md)) |
| `--prevent-screen-sleep` | Prevent display from sleeping during simulation (enabled by default) |

### Information Options

| Option | Description |
|--------|-------------|
| `--help` | Show all available options |
| `--list-integrators` | List all available integration methods |

Run `stardrift --help` for the complete list of options including configuration overrides.

### Example Commands

```bash
# Run with specific body count and seed for reproducibility
stardrift --bodies 50 --seed 123

# Try a specific color scheme
stardrift --color-scheme viridis

# Use a different integrator for better energy conservation
stardrift --integrator pefrl --bodies 100

# Start paused to set up the view before simulation begins
stardrift --paused --bodies 200

# Generate identical simulations with different color schemes
for scheme in viridis plasma inferno turbo; do
    stardrift --seed 42 --bodies 50 --color-scheme $scheme
done
```

## Screenshots

### Manual Screenshots

Press **S** to take a screenshot at any time. Manual screenshots:

- Automatically hide UI elements and HUD before capture
- Save to the configured directory (or current directory by default)
- Use PNG format at full window resolution
- Include timestamps in filenames by default

Screenshot behavior can be customized in the [configuration file](configuration.md#screenshot-configuration).

### Automated Screenshot Capture

Stardrift includes comprehensive automated screenshot capabilities, useful for testing, generating promotional materials, or creating time-lapse sequences.

#### Basic Automated Usage

```bash
# Take a single screenshot after 2 seconds
stardrift --screenshot-after 2 --exit-after-screenshots

# Take 5 screenshots at 1-second intervals
stardrift --screenshot-interval 1 --screenshot-count 5 --exit-after-screenshots

# Use frame-based timing for deterministic captures
stardrift --screenshot-after 120 --screenshot-use-frames
```

#### Automated Screenshot Options

| Option | Description |
|--------|-------------|
| `--screenshot-after N` | Take screenshot after N seconds (or frames with `--screenshot-use-frames`) |
| `--screenshot-interval N` | Take screenshots every N seconds/frames |
| `--screenshot-count N` | Number of screenshots to take (default: 1) |
| `--screenshot-use-frames` | Use frame counting instead of wall-clock time |
| `--screenshot-dir PATH` | Output directory (creates if needed) |
| `--screenshot-name NAME` | Base filename without extension |
| `--screenshot-sequential` | Use sequential numbering (e.g., `test_0001.png`) |
| `--screenshot-no-timestamp` | Disable timestamps for predictable filenames |
| `--screenshot-list-paths` | Output file paths to stdout after capture |
| `--exit-after-screenshots` | Exit application after all screenshots are taken |

#### Deterministic Captures for Testing

For regression testing or CI/CD pipelines, use frame-based timing with a fixed seed to produce identical screenshots:

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

This produces the same screenshot every time, making it suitable for automated visual testing.

#### Automated vs Manual Screenshots

| Behavior | Manual (S key) | Automated (CLI) |
|----------|----------------|-----------------|
| UI visibility | Hidden during capture | Preserved (for UI testing) |
| Timing | Instant | Configurable delay/interval |
| Exit behavior | Continues running | Optional auto-exit |

## User Interface

### Diagnostics HUD

The on-screen diagnostics display shows:

- **FPS**: Current frame rate
- **Frame**: Total frame count since start
- **Bodies**: Number of celestial bodies in the simulation

Toggle with **D** or the UI button.

### UI Buttons

The interface includes toggle buttons for common actions:

- **Octree**: Show/hide the spatial partitioning visualization
- **Barycenter**: Show/hide the center of mass indicator
- **Trails**: Show/hide body movement trails
- **Diagnostics**: Show/hide the HUD
- **Restart**: Generate new random bodies
- **Screenshot**: Capture the current view

### Visualization Toggles

- **Octree visualization**: Shows the Barnes-Hut octree structure as wireframe boxes, useful for understanding how the spatial partitioning works
- **Barycenter gizmo**: Displays a cross-hair at the system's center of mass
- **Trails**: Fading trails behind each body showing recent movement paths

## Platform-Specific Notes

### Desktop (Windows, macOS, Linux)

The native desktop build offers the best performance. Use release builds for smooth simulation of larger body counts:

```bash
cargo run -p stardrift --release
```

### WebAssembly (Browser)

The WASM version runs in modern browsers with WebGL2 support. Some considerations:

- Performance may be lower than native builds
- Browser hardware acceleration should be enabled
- Some features like screen sleep prevention are not available
- Minimum browser versions: Chrome 57+, Firefox 52+, Safari 15+, Edge 79+

## See Also

- [Configuration Reference](configuration.md) - Customize simulation parameters
- [Integrators Guide](integrators.md) - Choose the right numerical integrator
- [Color Schemes](color-schemes.md) - Available color palettes
