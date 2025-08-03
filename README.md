# Stardrift

A high-performance 3D gravitational N-body simulation built with Rust, Bevy game engine, and Avian3D physics. This
project simulates the gravitational interactions between multiple celestial bodies with real-time visualization and
interactive camera controls.

## Features

### Core Simulation

- **N-body gravitational physics**: Accurate gravitational force calculations between all bodies
- **Barnes-Hut octree algorithm**: Efficient O(N log N) gravitational force calculations using spatial partitioning
- **High precision**: Uses f64 floating-point precision for increased accuracy
- **Deterministic simulation**: Physics use deterministic behavior for reproducible results
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
    - **Restart simulation button**: Generate new random bodies and restart the simulation
    - **Screenshot button**: Capture the current view without UI elements

### Platform Support

- **Native desktop**: Windows, macOS, and Linux support
- **WebAssembly (WASM)**: Browser-based version with WebGL2 support

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
cargo run --all-features

# Release build (optimized performance)
cargo run --release --all-features

# Build with specific features
cargo run --features dev        # Development features only
cargo run --no-default-features --features graphics  # Run without diagnostics
```

### WebAssembly Build

The project uses [trunk](https://trunkrs.dev/) for WebAssembly builds:

```bash
# Install trunk if not present
cargo install trunk

# Development build with hot-reloading
trunk serve

# Production build
trunk build --release
```

The built files will be in the `dist/` directory, ready for deployment. Trunk automatically handles:
- WASM compilation and optimization
- Asset bundling and injection
- Development server with hot-reloading
- Gzip compression for production builds

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
| **S**           | Take screenshot (hides UI and HUD)              |
| **0-9**         | Set octree visualization depth (0 = all levels) |
| **Escape**      | Quit application                                |
| **Touch**       | Pan, orbit, and zoom (mobile/tablet)            |

### Camera Behavior

- The camera automatically follows the barycenter (center of mass) of the system
- Pan and orbit controls allow you to explore the simulation from different angles
- The camera tracks the movement of the gravitational system

### Configuration

The simulation uses a TOML-based configuration system with support for XDG config directories. Below is a complete
reference of all available configuration options.

#### Configuration File Format

```toml
version = 5  # Configuration format version (required)

[physics]
# Physics simulation parameters

[physics.initial_velocity]
# Initial velocity settings for bodies

[rendering]
# Visual rendering settings

[trails]
# Trail visualization settings

[screenshots]
# Screenshot capture settings
```

#### Complete Configuration Reference

##### Root Settings

| Field     | Type  | Default | Description                                                              |
|-----------|-------|---------|--------------------------------------------------------------------------|
| `version` | `u32` | `5`     | Configuration format version. Configs with outdated versions are ignored |

##### Physics Configuration (`[physics]`)

| Field                                        | Type          | Default    | Description                                                            |
|----------------------------------------------|---------------|------------|------------------------------------------------------------------------|
| `gravitational_constant`                     | `f64`         | `200.0`    | Strength of gravitational attraction between bodies                    |
| `body_count`                                 | `usize`       | `30`       | Number of celestial bodies to simulate                                 |
| `octree_theta`                               | `f64`         | `0.5`      | Barnes-Hut accuracy parameter (0.0-2.0). Lower = more accurate, slower |
| `octree_leaf_threshold`                      | `usize`       | `2`        | Maximum bodies per octree leaf before subdivision                      |
| `body_distribution_sphere_radius_multiplier` | `f64`         | `100.0`    | Multiplier for initial body distribution radius                        |
| `body_distribution_min_distance`             | `f64`         | `0.001`    | Minimum distance between bodies at spawn                               |
| `min_body_radius`                            | `f64`         | `1.0`      | Minimum radius for generated bodies                                    |
| `max_body_radius`                            | `f64`         | `2.0`      | Maximum radius for generated bodies                                    |
| `force_calculation_min_distance`             | `f64`         | `2.0`      | Minimum distance for force calculations (prevents singularities)       |
| `force_calculation_max_force`                | `f64`         | `100000.0` | Maximum force magnitude to prevent instabilities                       |
| `initial_seed`                               | `Option<u64>` | `None`     | Random seed for deterministic generation. None = random                |
| `collision_restitution`                      | `f64`         | `0.8`      | Bounciness of collisions (0.0 = inelastic, 1.0 = perfectly elastic)    |
| `collision_friction`                         | `f64`         | `0.5`      | Friction coefficient for collisions                                    |

##### Initial Velocity Configuration (`[physics.initial_velocity]`)

| Field             | Type     | Default    | Description                                                    |
|-------------------|----------|------------|----------------------------------------------------------------|
| `enabled`         | `bool`   | `true`     | Whether bodies spawn with initial velocities                   |
| `min_speed`       | `f64`    | `5.0`      | Minimum initial speed                                          |
| `max_speed`       | `f64`    | `20.0`     | Maximum initial speed                                          |
| `velocity_mode`   | `string` | `"Random"` | Velocity distribution mode (see below)                         |
| `tangential_bias` | `f64`    | `0.7`      | Bias toward tangential motion (0.0-1.0) when using Random mode |

**Velocity Modes:**

- `"Random"` - Random velocity vectors with optional tangential bias
- `"Orbital"` - Circular orbital velocities around barycenter
- `"Tangential"` - Pure tangential motion perpendicular to radius
- `"Radial"` - Pure radial motion toward/away from barycenter

##### Rendering Configuration (`[rendering]`)

| Field                      | Type  | Default   | Description                                           |
|----------------------------|-------|-----------|-------------------------------------------------------|
| `min_temperature`          | `f64` | `3000.0`  | Minimum stellar temperature in Kelvin (affects color) |
| `max_temperature`          | `f64` | `15000.0` | Maximum stellar temperature in Kelvin (affects color) |
| `bloom_intensity`          | `f64` | `100.0`   | Intensity of bloom visual effect                      |
| `saturation_intensity`     | `f64` | `3.0`     | Color saturation multiplier                           |
| `camera_radius_multiplier` | `f64` | `4.0`     | Camera distance multiplier relative to system size    |

##### Trail Configuration (`[trails]`)

Trail visualization configuration options.

| Field                     | Type     | Default         | Description                                     |
|---------------------------|----------|-----------------|-------------------------------------------------|
| `trail_length_seconds`    | `f64`    | `10.0`          | How long trails persist in seconds              |
| `update_interval_seconds` | `f64`    | `0.03333`       | How often to add trail points (default: 30 FPS) |
| `max_points_per_trail`    | `usize`  | `10000`         | Maximum trail points per body                   |
| `base_width`              | `f64`    | `1.0`           | Base trail width                                |
| `width_relative_to_body`  | `bool`   | `false`         | Scale trail width relative to body size         |
| `body_size_multiplier`    | `f64`    | `2.0`           | Trail width multiplier when relative to body    |
| `enable_fading`           | `bool`   | `true`          | Enable trail fade-out effect                    |
| `fade_curve`              | `string` | `"Exponential"` | Fade curve type (see below)                     |
| `min_alpha`               | `f64`    | `0.0`           | Minimum trail transparency (0.0-1.0)            |
| `max_alpha`               | `f64`    | `0.3333`        | Maximum trail transparency (0.0-1.0)            |
| `enable_tapering`         | `bool`   | `true`          | Enable trail width tapering                     |
| `taper_curve`             | `string` | `"Linear"`      | Taper curve type (see below)                    |
| `min_width_ratio`         | `f64`    | `0.2`           | Minimum width ratio at trail end                |
| `bloom_factor`            | `f64`    | `1.0`           | Trail bloom intensity multiplier                |
| `use_additive_blending`   | `bool`   | `true`          | Use additive blending for trails                |

**Fade Curves:**

- `"Linear"` - Linear fade from head to tail
- `"Exponential"` - Exponential fade (aggressive)
- `"SmoothStep"` - Smooth interpolation curve
- `"EaseInOut"` - Ease in and out curve

**Taper Curves:**

- `"Linear"` - Linear width reduction
- `"Exponential"` - Exponential width reduction
- `"SmoothStep"` - Smooth width transition

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

#### Example Configuration

Here's a complete example configuration file with all default values:

```toml
version = 5

[physics]
gravitational_constant = 200.0
body_count = 30
octree_theta = 0.5
octree_leaf_threshold = 2
body_distribution_sphere_radius_multiplier = 100.0
body_distribution_min_distance = 0.001
min_body_radius = 1.0
max_body_radius = 2.0
force_calculation_min_distance = 2.0
force_calculation_max_force = 100000.0
# initial_seed = 12345  # Uncomment to use a specific seed for deterministic generation
collision_restitution = 0.8
collision_friction = 0.5

[physics.initial_velocity]
enabled = true
min_speed = 5.0
max_speed = 20.0
velocity_mode = "Random"  # Options: "Random", "Orbital", "Tangential", "Radial"
tangential_bias = 0.7

[rendering]
min_temperature = 3000.0
max_temperature = 15000.0
bloom_intensity = 100.0
saturation_intensity = 3.0
camera_radius_multiplier = 4.0

[trails]
trail_length_seconds = 10.0
update_interval_seconds = 0.03333333333333333
max_points_per_trail = 10000
base_width = 1.0
width_relative_to_body = false
body_size_multiplier = 2.0
enable_fading = true
fade_curve = "Exponential"  # Options: "Linear", "Exponential", "SmoothStep", "EaseInOut"
min_alpha = 0.0
max_alpha = 0.3333
enable_tapering = true
taper_curve = "Linear"  # Options: "Linear", "Exponential", "SmoothStep"
min_width_ratio = 0.2
bloom_factor = 1.0
use_additive_blending = true

[screenshots]
# directory = "screenshots"  # Uncomment to set a custom screenshot directory
filename_prefix = "stardrift_screenshot"
include_timestamp = true
notification_enabled = true
hide_ui_frame_delay = 2
```

## Technical Details

### Architecture

- **Engine**: Bevy 0.16.1 (Entity Component System game engine)
- **Physics**: Avian3D with f64 precision and parallel processing
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

### Dependencies

#### Core Dependencies

- **bevy**: Game engine and rendering framework
- **avian3d**: 3D physics engine with gravitational simulation support
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

The codebase uses a pure self-contained plugin architecture designed for maintainability, scalability, and clear
separation of concerns. Each plugin owns all its functionality internally with event-driven communication between
plugins:

```
src/
├── main.rs                       # Application entry point and plugin registration
├── prelude.rs                    # Common imports and type aliases
├── config.rs                     # Configuration management system
├── states.rs                     # Application state management
├── events.rs                     # Centralized event definitions for inter-plugin communication
├── plugins/                      # Self-contained Bevy plugins
│   ├── mod.rs
│   ├── simulation/               # Core simulation plugin with submodules
│   │   ├── mod.rs                # Plugin definition and system coordination
│   │   ├── physics.rs            # Physics calculations and body management
│   │   ├── actions.rs            # Simulation control and action handling
│   │   └── components.rs         # BodyBundle and celestial body factory functions
│   ├── controls.rs               # Complete input handling and UI structure
│   ├── camera.rs                 # Camera setup and positioning logic
│   ├── visualization.rs          # Debug rendering (octree wireframe, barycenter gizmo)
│   ├── simulation_diagnostics.rs # Simulation metrics and diagnostics plugin
│   ├── diagnostics_hud.rs        # Real-time HUD display plugin (feature-gated)
│   ├── embedded_assets.rs        # Embedded asset management plugin
│   ├── trails.rs                 # Trail rendering plugin with Trail component
│   └── attribution.rs            # Version attribution display plugin
├── resources/                    # Bevy ECS resources (global state)
│   └── mod.rs                    # Shared resources like RNG, constants, and octree
├── utils/                        # Utility modules
│   ├── mod.rs
│   ├── math.rs                   # Mathematical functions and algorithms
│   └── color.rs                  # Color and material utilities
└── physics/                      # Physics-specific modules
    ├── mod.rs
    ├── stars.rs                  # Stellar physics and realistic body generation
    └── octree.rs                 # Barnes-Hut octree implementation
```

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
    - `physics.rs`: Core physics calculations including octree rebuilding and force application
    - `actions.rs`: Simulation control and action handlers (restart, pause, screenshot)
- **`plugins/controls.rs`**: Complete input handling and UI structure (keyboard, mouse, buttons)
- **`plugins/camera.rs`**: Camera setup and positioning logic
- **`plugins/visualization.rs`**: Debug rendering for octree wireframe and barycenter gizmo
- **`plugins/simulation_diagnostics.rs`**: Simulation metrics and performance diagnostics
- **`plugins/diagnostics_hud.rs`**: Real-time HUD display for simulation information (feature-gated)
- **`plugins/trails.rs`**: Visual trail rendering system
- **`plugins/embedded_assets.rs`**: Embedded asset management for web deployment
- **`events.rs`**: Centralized event definitions with `SimulationCommand` enum for inter-plugin communication
- **`resources/mod.rs`**: Shared state including RNG, gravitational constants, and octree data
- **`utils/math.rs`**: Mathematical utilities for sphere distribution and random vector generation
- **`config.rs`**: Centralized configuration management with serialization support
- **`states.rs`**: Application state management and transitions
- **`physics/octree.rs`**: High-performance Barnes-Hut spatial partitioning implementation

This structure enables extension, testing, and maintenance while providing defined entry points for understanding and
modifying the simulation behavior.

## Development

### Development Features

Enable development features for additional debugging:

```bash
cargo run --features dev
```

Development features include:

- Asset hot-reloading
- File watching
- Additional debugging information
- Dynamic linking for faster compilation

### Available Features

- **`dev`**: Development features for faster iteration
- **`diagnostics`**: Extended performance diagnostics
- **`benchmarks`**: Performance benchmarking capabilities
- **`trails`**: Dynamic trail visualization for celestial bodies

Combine features as needed:

```bash
cargo run --features dev           # Development features
cargo run --all-features            # All features enabled
```

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
- Physics simulation powered by [Avian3D](https://github.com/Jondolf/avian)
- Camera controls provided by [bevy_panorbit_camera](https://github.com/johanhelsing/bevy_panorbit_camera)
