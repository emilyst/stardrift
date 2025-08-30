# Stardrift

A high-performance 3D gravitational N-body simulation built with Rust and Bevy game engine. This
project simulates the gravitational interactions between multiple celestial bodies with real-time visualization and
interactive camera controls.

## Project Status

Stardrift is an experimental side project for personal enjoyment and learning. The project explores gravitational
physics simulation while learning Rust and Bevy. Development happens in spare time without specific goals or timeline.

This project also serves as an exploration of AI capabilities in software development. Its development involves heavy
use of AI assistance.

**Note**: This is not intended for scientific or research purposes. The project remains experimental and subject to
significant changes.

## Features

### Core Simulation

- **N-body gravitational physics**: Accurate gravitational force calculations between all bodies
- **Barnes-Hut octree algorithm**: Efficient O(N log N) gravitational force calculations using spatial partitioning
- **High precision**: Uses f64 floating-point precision for increased accuracy
- **Custom physics engine**: Purpose-built component-based physics system optimized for n-body simulation
- **Multiple numerical integrators**: Six integration methods with different accuracy/performance trade-offs:
    - Symplectic Euler (1st order, symplectic)
    - Velocity Verlet (2nd order, symplectic)
    - Heun/Improved Euler (2nd order)
    - Runge-Kutta 2nd Order Midpoint (2nd order)
    - Runge-Kutta 4th Order (4th order)
    - PEFRL (4th order, symplectic)
- **Parallel processing**: Multi-threaded physics calculations for performance optimization
- **Dynamic barycenter tracking**: Real-time calculation and visualization of the system's center of mass

### Visualization & Controls

- **3D real-time rendering**: 3D visualization with Bevy's rendering pipeline
- **Interactive camera**: Pan, orbit, and zoom controls
- **Touch support**: Touch controls for mobile and tablet devices
- **Visual effects**: Bloom effects and tone mapping for visual rendering
- **Screenshot capture**: Take screenshots without UI elements
    - Automatic UI and HUD hiding during capture
    - Configurable save directory and filename format
    - Timestamp-based filenames for chronological organization
    - PNG format with full window resolution
- **Dynamic trails**: High-performance fading trails for celestial bodies
    - Time-based trail length management
    - Multiple fade curves (Linear, Exponential, SmoothStep, EaseInOut)
    - Configurable width with tapering options
    - Bloom effects and additive blending
    - Automatically pauses when simulation is paused
- **Barycenter visualization**: Cross-hair indicator showing the system's center of mass with toggle controls
- **Octree visualization**: Real-time wireframe rendering of the spatial partitioning structure
- **Interactive visualization controls**: Toggle octree display, barycenter gizmos, and adjust visualization depth
  levels

### User Interface

- **Real-time diagnostics HUD**: On-screen display showing:
    - Frame rate (FPS)
    - Frame count
    - Body count
- **Pause/Resume functionality**: Space bar to pause and resume the simulation
- **Interactive UI buttons**:
    - **Octree toggle button**: Show/hide octree visualization
    - **Barycenter gizmo toggle button**: Show/hide barycenter cross-hair indicator
    - **Trails toggle button**: Show/hide celestial body trails
    - **Diagnostics HUD toggle button**: Show/hide the FPS and frame count display
    - **Restart simulation button**: Generate new random bodies and restart the simulation
    - **Screenshot button**: Capture the current view without UI elements

### Platform Support

- **Native desktop**: Windows, macOS, and Linux support
- **WebAssembly (WASM)**: Browser-based version with WebGL2 support

## Planned Features

The following features are planned for future development:

1. **Enhanced Diagnostics** - Comprehensive physics accuracy monitoring including energy conservation (Hamiltonian),
   angular momentum tracking, virial ratio, and performance profiling
2. **Collision Detection** - Implement collision detection and response between bodies with configurable restitution
3. **Configurable Simulation Speed** - Time scaling controls for faster or slower simulation playback
4. **UI rework** - Replacing the current provisional UI with something more friendly and comprehensive
5. **Advanced Integrators** - Support for specialized integration schemes (Yoshida symplectic methods, etc.)

## Installation

### Pre-built Packages

Pre-built packages are available from the [releases page](https://github.com/emilyst/stardrift/releases):

#### Linux

- **Portable**: `.tar.gz` archives for x86_64 and ARM64

#### Windows

- **Portable**: `.zip` archives for x86_64 and ARM64

#### macOS

- **Disk Image**: `.dmg` for Intel and Apple Silicon
- **Portable**: `.tar.gz` archives for manual installation

#### Web

- **WebAssembly**: Pre-built WASM packages for web deployment

All packages include SHA256 checksums for verification.

### Building from Source

#### Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs/)
- **Git**: For cloning the repository

#### Clone the Repository

```bash
git clone https://github.com/emilyst/stardrift.git
cd stardrift
```

### Native Build

```bash
# Development build (faster compilation)
cargo run -p stardrift

# Release build (optimized performance)
cargo run -p stardrift --release
```

### WebAssembly Build

The project uses [trunk](https://trunkrs.dev/) for WebAssembly builds:

```bash
# Install trunk if not present
cargo install trunk

# Development build with hot-reloading
trunk serve

# Release build
trunk build --release
```

The built files will be in the `dist/` directory, ready for deployment. Trunk automatically handles:

- WASM compilation and optimization
- Asset bundling and injection
- Development server with hot-reloading
- Gzip compression for release builds

## Usage

### Controls

| Key/Action      | Function                                        |
|-----------------|-------------------------------------------------|
| **Mouse**       | Pan and orbit camera around the simulation      |
| **Mouse Wheel** | Zoom in/out                                     |
| **Space**       | Pause/Resume simulation                         |
| **N**           | New simulation with new random bodies           |
| **O**           | Toggle octree visualization on/off              |
| **C**           | Toggle barycenter gizmo visibility on/off       |
| **T**           | Toggle trail visibility on/off                  |
| **D**           | Toggle diagnostics HUD visibility on/off        |
| **S**           | Take screenshot (hides UI and HUD)              |
| **0-9**         | Set octree visualization depth (0 = all levels) |
| **Escape**      | Quit application                                |
| **Touch**       | Pan, orbit, and zoom (mobile/tablet)            |

### Automated Screenshot Capture

The simulation includes comprehensive automated screenshot capabilities for UI testing and validation:

#### Basic Usage

```bash
# Take a single screenshot after 2 seconds
stardrift --screenshot-after 2 --exit-after-screenshots

# Take 5 screenshots at 1-second intervals
stardrift --screenshot-interval 1 --screenshot-count 5 --exit-after-screenshots

# Use frame-based timing for deterministic captures
stardrift --screenshot-after 120 --screenshot-use-frames
```

#### CLI Options

| Option                      | Description                                                                |
|-----------------------------|----------------------------------------------------------------------------|
| `--screenshot-after N`      | Take screenshot after N seconds (or frames with `--screenshot-use-frames`) |
| `--screenshot-interval N`   | Take screenshots every N seconds/frames                                    |
| `--screenshot-count N`      | Number of screenshots to take (default: 1)                                 |
| `--screenshot-use-frames`   | Use frame counting instead of time                                         |
| `--screenshot-dir PATH`     | Output directory (creates if needed)                                       |
| `--screenshot-name NAME`    | Base filename without extension                                            |
| `--screenshot-sequential`   | Use sequential numbering (e.g., `test_0001.png`)                           |
| `--screenshot-no-timestamp` | Disable timestamps for predictable names                                   |
| `--screenshot-list-paths`   | Output file paths to stdout                                                |
| `--exit-after-screenshots`  | Exit after all screenshots taken                                           |

#### AI Testing Integration

```bash
# Deterministic capture for regression testing
stardrift --seed 42 --bodies 100 \
          --screenshot-after 60 --screenshot-use-frames \
          --screenshot-dir ./test_output \
          --screenshot-name ui_state \
          --screenshot-no-timestamp \
          --screenshot-list-paths

# Output: SCREENSHOT_PATH: ./test_output/ui_state.png
```

Note: Automated screenshots preserve UI visibility for validation, unlike manual screenshots (key 'S') which hide UI
elements.

### Camera Behavior

- The camera automatically follows the barycenter (center of mass) of the system
- Pan and orbit controls allow you to explore the simulation from different angles
- The camera tracks the movement of the gravitational system

### Configuration

The simulation uses a TOML-based configuration file.

#### Complete Configuration Reference

##### Physics Configuration (`[physics]`)

| Field                                        | Type          | Default             | Description                                                            |
|----------------------------------------------|---------------|---------------------|------------------------------------------------------------------------|
| `gravitational_constant`                     | `f64`         | `0.01`              | Strength of gravitational attraction between bodies                    |
| `body_count`                                 | `usize`       | `100`               | Number of celestial bodies to simulate                                 |
| `octree_theta`                               | `f64`         | `0.5`               | Barnes-Hut accuracy parameter (0.0-2.0). Lower = more accurate, slower |
| `octree_leaf_threshold`                      | `usize`       | `4`                 | Maximum bodies per octree leaf before subdivision                      |
| `body_distribution_sphere_radius_multiplier` | `f32`         | `100.0`             | Multiplier for initial body distribution radius                        |
| `body_distribution_min_distance`             | `f32`         | `0.001`             | Minimum distance between bodies at spawn                               |
| `min_body_radius`                            | `f32`         | `1.0`               | Minimum radius for generated bodies                                    |
| `max_body_radius`                            | `f32`         | `2.0`               | Maximum radius for generated bodies                                    |
| `force_calculation_min_distance`             | `f64`         | `2.0`               | Minimum distance for force calculations (prevents singularities)       |
| `force_calculation_max_force`                | `f64`         | `10000.0`           | Maximum force magnitude to prevent instabilities                       |
| `initial_seed`                               | `Option<u64>` | `None`              | Random seed for deterministic generation. None = random                |
| `barycentric_drift_correction`               | `bool`        | `true`              | Enable automatic recentering around barycenter. False = pure physics   |
| `integrator.type`                            | `string`      | `"velocity_verlet"` | Numerical integration method (see Integrator Types below)              |

##### Initial Velocity Configuration (`[physics.initial_velocity]`)

| Field             | Type     | Default    | Description                                                    |
|-------------------|----------|------------|----------------------------------------------------------------|
| `enabled`         | `bool`   | `true`     | Whether bodies spawn with initial velocities                   |
| `min_speed`       | `f64`    | `10.0`     | Minimum initial speed                                          |
| `max_speed`       | `f64`    | `100.0`    | Maximum initial speed                                          |
| `velocity_mode`   | `string` | `"random"` | Velocity distribution mode (see below)                         |
| `tangential_bias` | `f64`    | `0.7`      | Bias toward tangential motion (0.0-1.0) when using Random mode |

**Integrator Types:** (use snake_case in config)

- `"explicit_euler"` - 1st order explicit integrator, non-symplectic (alias: `"forward_euler"`)
- `"symplectic_euler"` - 1st order symplectic integrator (aliases: `"euler"`, `"semi_implicit_euler"`)
- `"velocity_verlet"` - 2nd order symplectic integrator, excellent energy conservation (alias: `"verlet"`)
- `"heun"` - 2nd order explicit integrator, also known as Improved Euler (alias: `"improved_euler"`)
- `"runge_kutta_second_order_midpoint"` - 2nd order explicit integrator (aliases: `"rk2"`, `"midpoint"`)
- `"runge_kutta_fourth_order"` - 4th order explicit integrator, highest accuracy (alias: `"rk4"`)
- `"pefrl"` - 4th order symplectic integrator, superior long-term energy conservation (alias: `"forest_ruth"`)

##### Integrator Selection Guide

The choice of integrator significantly affects simulation accuracy, stability, and performance. Here's a detailed guide:

**Symplectic Integrators** (Energy-conserving, ideal for long-term simulations):

- **`symplectic_euler`** (1st order)
    - **Pros**: Fast, simple, better energy conservation than explicit Euler
    - **Cons**: Low accuracy, requires small timesteps
    - **Use Case**: Quick visualizations where accuracy isn't critical
    - **Algorithm**: Updates velocity before position to preserve phase space volume

- **`velocity_verlet`** (2nd order) - **RECOMMENDED DEFAULT**
    - **Pros**: Excellent energy conservation, time-reversible, good stability
    - **Cons**: Requires force recalculation (2 evaluations per step)
    - **Use Case**: General n-body simulations, especially long-term orbital mechanics
    - **Algorithm**: Uses average of accelerations at start and end of timestep

- **`pefrl`** (4th order)
    - **Pros**: Superior long-term energy conservation, high accuracy, symplectic
    - **Cons**: Most expensive (4 force evaluations per step)
    - **Use Case**: Scientific simulations requiring long-term stability
    - **Algorithm**: Optimized Forest-Ruth composition with minimal error coefficients

**Explicit Integrators** (General-purpose, not energy-conserving):

- **`explicit_euler`** (1st order) - **WARNING: For educational/comparison use only**
    - **Pros**: Simplest possible integrator, fast computation
    - **Cons**: Poor energy conservation, unstable for oscillatory systems, energy drift
    - **Use Case**: Educational comparisons, debugging, very short simulations
    - **Algorithm**: Updates position before velocity using current state values
    - **Warning**: Energy grows/decays exponentially - unsuitable for orbital mechanics

- **`heun`** (2nd order)
    - **Pros**: Better accuracy than Euler, predictor-corrector approach
    - **Cons**: Energy drift in long simulations
    - **Use Case**: Short-term simulations with smooth forces
    - **Algorithm**: Averages derivatives at start and predicted endpoint

- **`runge_kutta_second_order_midpoint`** (2nd order)
    - **Pros**: Good accuracy for smooth problems
    - **Cons**: Energy drift, not suitable for long-term simulations
    - **Use Case**: Non-Hamiltonian systems or short integration periods
    - **Algorithm**: Evaluates derivative at the midpoint of timestep

- **`runge_kutta_fourth_order`** (4th order)
    - **Pros**: High accuracy for smooth functions
    - **Cons**: Energy drift, expensive (4 evaluations), can be unstable for stiff problems
    - **Use Case**: High-accuracy requirements over short timeframes
    - **Algorithm**: Weighted average of 4 intermediate derivative evaluations

**Performance vs. Accuracy Trade-offs**:

| Integrator                          | Order | Force Evals/Step | Energy Conservation | Relative Speed |
|-------------------------------------|-------|------------------|---------------------|----------------|
| `explicit_euler`                    | 1     | 1                | Very Poor           | Fastest        |
| `symplectic_euler`                  | 1     | 1                | Good                | Fastest        |
| `heun`                              | 2     | 2                | Poor                | Fast           |
| `runge_kutta_second_order_midpoint` | 2     | 2                | Poor                | Fast           |
| `velocity_verlet`                   | 2     | 2                | Excellent           | Fast           |
| `runge_kutta_fourth_order`          | 4     | 4                | Poor                | Slow           |
| `pefrl`                             | 4     | 4                | Superior            | Slow           |

**Choosing Guidelines**:

1. **Default Choice**: Use `velocity_verlet` for most n-body simulations
2. **Long-term stability**: Use `pefrl` for scientific accuracy over extended periods
3. **Performance critical**: Use `symplectic_euler` with smaller timesteps
4. **High accuracy, short-term**: Use `runge_kutta_fourth_order`
5. **Energy conservation important**: Always choose symplectic integrators

**Velocity Modes:** (use snake_case in config, e.g. `"random"`, `"orbital"`)

- `"random"` - Random velocity vectors with optional tangential bias
- `"orbital"` - Circular orbital velocities around barycenter
- `"tangential"` - Pure tangential motion perpendicular to radius
- `"radial"` - Pure radial motion toward/away from barycenter

##### Rendering Configuration (`[rendering]`)

| Field                      | Type     | Default        | Description                                                       |
|----------------------------|----------|----------------|-------------------------------------------------------------------|
| `color_scheme`             | `string` | `"black_body"` | Color scheme for bodies (see Color Schemes section below)         |
| `min_temperature`          | `f32`    | `3000.0`       | Minimum stellar temperature in Kelvin (for `"black_body"` scheme) |
| `max_temperature`          | `f32`    | `15000.0`      | Maximum stellar temperature in Kelvin (for `"black_body"` scheme) |
| `bloom_intensity`          | `f32`    | `250.0`        | Intensity of bloom visual effect                                  |
| `saturation_intensity`     | `f32`    | `3.0`          | Color saturation multiplier                                       |
| `camera_radius_multiplier` | `f32`    | `4.0`          | Camera distance multiplier relative to system size                |

###### Color Schemes

The simulation supports multiple color schemes for celestial bodies, grouped into categories:

**Physics-Based:**

- `black_body` (default) - Colors based on black body radiation temperatures. Smaller bodies appear hotter (blue-white),
  larger bodies appear cooler (red-orange)

**Colorblind-Safe Palettes:**

- `deuteranopia_safe` - Optimized for red-green colorblindness (most common, ~8% of males). Uses blue, orange, yellow,
  and teal colors
- `protanopia_safe` - Optimized for red-blindness. Uses blue, yellow, and teal colors
- `tritanopia_safe` - Optimized for blue-yellow colorblindness (rare). Uses red, green, and magenta
- `high_contrast` - Maximum distinguishability with widely separated hues and high saturation differences

**Scientific Colormaps (Perceptually Uniform):**

- `viridis` - Purple-blue-green-yellow gradient. Industry standard for scientific visualization, colorblind-safe
- `plasma` - Magenta-purple-pink-yellow gradient with high visual appeal
- `inferno` - Black-red-yellow-white heat map for intensity visualization
- `turbo` - Google's improved rainbow colormap with better perceptual properties than standard rainbow

**Aesthetic Themes:**

- `rainbow` - Random vibrant colors using the full HSL spectrum with high saturation
- `pastel` - Soft, low-saturation colors (S: 0.3-0.5, L: 0.7-0.85)
- `neon` - High saturation cyberpunk-style colors with limited hue ranges
- `monochrome` - Grayscale variations (excludes pure black and white for visibility)
- `vaporwave` - Retrofuturistic pink-purple-cyan palette with weighted distribution for authentic 80s aesthetic

##### Trail Configuration (`[trails]`)

Trail visualization configuration options.

| Field                     | Type     | Default         | Description                                     |
|---------------------------|----------|-----------------|-------------------------------------------------|
| `trail_length_seconds`    | `f32`    | `10.0`          | How long trails persist in seconds              |
| `update_interval_seconds` | `f32`    | `0.03333`       | How often to add trail points (default: 30 FPS) |
| `max_points_per_trail`    | `usize`  | `10000`         | Maximum trail points per body                   |
| `base_width`              | `f32`    | `1.0`           | Base trail width                                |
| `width_relative_to_body`  | `bool`   | `false`         | Scale trail width relative to body size         |
| `body_size_multiplier`    | `f32`    | `2.0`           | Trail width multiplier when relative to body    |
| `enable_fading`           | `bool`   | `true`          | Enable trail fade-out effect                    |
| `fade_curve`              | `string` | `"exponential"` | Fade curve type (see below)                     |
| `min_alpha`               | `f32`    | `0.0`           | Minimum trail transparency (0.0-1.0)            |
| `max_alpha`               | `f32`    | `0.3333`        | Maximum trail transparency (0.0-1.0)            |
| `enable_tapering`         | `bool`   | `true`          | Enable trail width tapering                     |
| `taper_curve`             | `string` | `"linear"`      | Taper curve type (see below)                    |
| `min_width_ratio`         | `f32`    | `0.2`           | Minimum width ratio at trail end                |
| `bloom_factor`            | `f32`    | `1.0`           | Trail bloom intensity multiplier                |
| `use_additive_blending`   | `bool`   | `true`          | Use additive blending for trails                |

**Fade Curves:** (use snake_case in config)

- `"linear"` - Linear fade from head to tail
- `"exponential"` - Exponential fade (aggressive)
- `"smooth_step"` - Smooth interpolation curve
- `"ease_in_out"` - Ease in and out curve

**Taper Curves:** (use snake_case in config)

- `"linear"` - Linear width reduction
- `"exponential"` - Exponential width reduction
- `"smooth_step"` - Smooth width transition

##### Screenshot Configuration (`[screenshots]`)

| Field                  | Type             | Default                  | Description                                          |
|------------------------|------------------|--------------------------|------------------------------------------------------|
| `directory`            | `Option<String>` | `None`                   | Save directory. None = current directory             |
| `filename_prefix`      | `String`         | `"stardrift_screenshot"` | Filename prefix for screenshots                      |
| `include_timestamp`    | `bool`           | `true`                   | Add timestamp to filenames                           |
| `notification_enabled` | `bool`           | `true`                   | Log screenshot captures                              |
| `hide_ui_frame_delay`  | `u32`            | `2`                      | Frames to wait before capture (ensures UI is hidden) |

#### Configuration File Location

The configuration is automatically loaded from the platform-specific config directory:

- **Linux**: `~/.config/Stardrift/config.toml`
- **macOS**: `~/Library/Application Support/Stardrift/config.toml`
- **Windows**: `%APPDATA%\Stardrift\config.toml`

If no configuration file exists, the application uses default values.

## Technical Details

### Architecture

- **Engine**: Bevy 0.16.1 (Entity Component System game engine)
- **Physics**: Custom ECS-based physics with f64 precision and parallel processing
- **Spatial Optimization**: Barnes-Hut octree algorithm with configurable theta parameter for accuracy/performance
  balance
- **Rendering**: Bevy's PBR (Physically Based Rendering) pipeline with real-time octree wireframe visualization
- **Random Number Generation**: ChaCha8 algorithm for efficient PRNG
- **Mathematical Utilities**: Sphere surface distribution algorithms and statistical validation

### Performance Optimizations

- **Build Profiles**:
    - Development: Fast compilation with basic optimizations
    - Release: Full optimizations with debug info stripped
    - Distribution: Link-time optimization (LTO) and single codegen unit
- **Algorithmic Efficiency**: Barnes-Hut octree reduces gravitational calculations from O(N²) to O(N log N)
- **Parallel Processing**: Multi-threaded physics calculations
- **Memory Efficiency**: Optimized data structures and minimal allocations
- **Rendering Optimizations**: Efficient mesh and material management
- **Integrator Benchmarks**: Comprehensive benchmark suite testing all six integrators across:
    - Raw performance (operations per second)
    - Accuracy (error vs analytical solutions)
    - Convergence order verification
    - Energy conservation over long simulations
    - Work-precision tradeoffs
    - Realistic N-body scenarios

### Dependencies

#### Core Dependencies

- **bevy**: Game engine and rendering framework
- **bevy_panorbit_camera**: Camera controls with touch support
- **libm**: Mathematical functions for no-std environments
- **rand**: Random number generation
- **rand_chacha**: ChaCha random number generator
- **chrono**: Date and time handling for screenshot timestamps

#### Build Dependencies

- **trunk**: Modern WASM application bundler (for WebAssembly builds)
- **wasm-bindgen**: Rust-WASM bindings (handled automatically by trunk)
- **web-sys**: Web API bindings
- **getrandom**: With `wasm_js` backend

### Browser Requirements (WASM Version)

- **WebGL2 & WebAssembly support**: Required for 3D rendering and application execution
- **Minimum browser versions**:
    - Chrome 57+ (WebAssembly requirement)
    - Firefox 52+ (WebAssembly requirement)
    - Safari 15+ (WebGL2 requirement)
    - Edge 79+ (Chromium-based Edge)
- **Hardware acceleration**: Required for optimal performance

## Project Structure

Stardrift uses a plugin-based architecture built on Bevy ECS (Entity Component System).

### Project Layout

- **`src/`** - Main application source code
    - `src/plugins/` - Self-contained feature plugins
    - `src/physics/` - Physics engine and integrators
- **`benches/`** - Performance benchmarks
- **`tests/`** - Integration tests
- **`assets/`** - Static assets (fonts, icons)
- **`macros/`** - Procedural macros for configuration

### Architecture

The project follows a **pure plugin architecture** where each major feature is a self-contained Bevy plugin:

- **Simulation Plugin** - Core physics simulation and body management
- **Controls Plugin** - Input handling and UI
- **Visualization Plugin** - Debug rendering (octree wireframe, barycenter)
- **Trails Plugin** - Particle trail rendering
- **Diagnostics Plugin** - Performance metrics and HUD display
- **Camera Plugin** - 3D camera controls
- **Attribution Plugin** - Version and credit display

Plugins communicate through:

- **Events** - Command pattern for user actions
- **Resources** - Shared state (RNG, constants, octree)
- **Components** - ECS data (Mass, Velocity, Trail, etc.)

### Physics Engine

The physics module implements:

- **Barnes-Hut Algorithm** - Octree-based force calculation for O(n log n) performance
- **Multiple Integrators** - Symplectic and Runge-Kutta methods
- **Collision Detection** - Body merging on contact
- **Barycenter Tracking** - System center of mass calculation

### Design Principles

- **Plugin-Based Architecture**: Each plugin is completely self-contained with internal systems, components, and clear
  boundaries
- **Event-Driven Communication**: Plugins communicate exclusively through `SimulationCommand` events
- **Zero Orchestration**: No external coordination or management of plugin internals
- **Scalable Organization**: Large plugins use internal submodules for code organization
- **Resource Management**: Global state is managed through Bevy's resource system
- **Configuration-Driven**: Centralized configuration system for runtime customization
- **Clear Feature Boundaries**: Plugin boundaries enforce architectural constraints

### Key Modules

- **`plugins/simulation/`**: Self-contained physics simulation with internal submodules
    - `mod.rs`: Plugin definition and system coordination
    - `physics.rs`: Physics system orchestration and ECS integration
    - `actions.rs`: Simulation control and action handlers (restart, pause, screenshot)
    - `components.rs`: Physics component bundles and entity creation
- **`plugins/controls.rs`**: Complete input handling and UI structure (keyboard, mouse, buttons)
- **`plugins/camera.rs`**: Camera setup and positioning logic
- **`plugins/visualization.rs`**: Debug rendering for octree wireframe and barycenter gizmo
- **`plugins/simulation_diagnostics.rs`**: Simulation metrics and performance diagnostics
- **`plugins/diagnostics_hud.rs`**: Real-time HUD display for simulation information (feature-gated)
- **`plugins/trails.rs`**: Visual trail rendering system
- **`plugins/embedded_assets.rs`**: Embedded asset management for web deployment
- **`events.rs`**: Centralized event definitions with `SimulationCommand` enum for inter-plugin communication
- **`resources/mod.rs`**: Shared state including RNG, gravitational constants, and octree data
- **`physics/math.rs`**: Mathematical utilities for sphere distribution and physics calculations
- **`physics/components.rs`**: Core physics components (Mass, Velocity, Acceleration)
- **`physics/resources.rs`**: Physics resources and runtime configuration
- **`physics/integrators/`**: Six numerical integration methods with acceleration field architecture and registry system
- **`config.rs`**: Centralized configuration management with serialization support
- **`states.rs`**: Application state management and transitions
- **`physics/octree.rs`**: High-performance Barnes-Hut spatial partitioning implementation
- **`physics/aabb3d.rs`**: Axis-aligned bounding box for spatial calculations

This structure enables extension, testing, and maintenance while providing defined entry points for understanding and
modifying the simulation behavior.

## Troubleshooting

### Common Issues

**WASM build fails**: Ensure you have the latest version of trunk:

```bash
cargo install trunk --force
```

**WebGL2 not supported**: Use a supported browser or enable hardware acceleration in browser settings.

**Poor performance**: Try the native build for better performance, or reduce the number of bodies in the simulation.

**Build errors**: Ensure you have the latest Rust toolchain:

```bash
rustup update
```

## License

This project is dedicated to the public domain under
the [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/) license.

You can copy, modify, distribute and perform the work, even for commercial purposes, all without asking permission. See
the [LICENSE](LICENSE) file for details.

## Security & Verification

### Build Provenance Attestation

All release binaries include cryptographic attestations that prove they were built by the official GitHub Actions
workflow. This provides supply chain security and ensures binaries haven't been tampered with.

To verify a downloaded binary:

```bash
# Install GitHub CLI if needed
# https://cli.github.com/

# Verify a release binary (example with version)
gh attestation verify stardrift-0.0.15-x86_64-unknown-linux-gnu.tar.gz \
  --repo emilyst/stardrift

# The command will confirm the binary was built from the official repository
```

Attestations use [Sigstore](https://sigstore.dev/) and follow the [SLSA](https://slsa.dev/) standard for software supply
chain security.

## Release Process

This project uses [cargo-release](https://github.com/crate-ci/cargo-release) for automated release management following
semantic versioning principles.

### Version Numbering

Following Rust ecosystem conventions:

- **0.0.x** - Early experimental releases with frequent breaking changes
- **0.x.y** - Pre-1.0 development phase where the API may still evolve
- **1.0.0** - First stable release with a commitment to API stability

### Making a Release

1. **Ensure all changes are committed**:
   ```bash
   git status  # Should show a clean working directory
   ```

2. **Review changes since last release**:
   ```bash
   cargo release changes
   ```

3. **Perform a dry run** (recommended):
   ```bash
   cargo release patch  # Shows what would happen
   cargo release minor  # For feature releases
   ```

4. **Execute the release**:
   ```bash
   cargo release patch --execute
   ```

5. **Push to GitHub**:
   ```bash
   git push origin main
   git push origin --tags
   ```

### What Happens During Release

The `cargo-release` tool automatically:

1. Updates the version in `Cargo.toml`
2. Updates `CHANGELOG.md` with the new version and current date
3. Creates a signed git commit with message "chore: release vX.Y.Z"
4. Tags the commit with "vX.Y.Z" (also signed)
5. Updates `Cargo.lock` with the new version

### Release Configuration

Release behavior is configured in `Cargo.toml` under `[package.metadata.release]`:

- Commits and tags are signed if GPG is configured
- Publishing to crates.io is disabled (private project)
- Releases are only allowed from the `main` branch
- Changelog format follows [Keep a Changelog](https://keepachangelog.com/) conventions

## Acknowledgments

- Built with [Bevy](https://bevyengine.org/) game engine
- Camera controls provided by [bevy_panorbit_camera](https://github.com/johanhelsing/bevy_panorbit_camera)
