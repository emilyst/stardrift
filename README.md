# Stardrift

A high-performance 3D gravitational N-body simulation built with Rust, Bevy game engine, and Avian3D physics. This
project simulates the gravitational interactions between multiple celestial bodies with real-time visualization and
interactive camera controls.

## Features

### Core Simulation

- **N-body gravitational physics**: Accurate gravitational force calculations between all bodies
- **Barnes-Hut octree algorithm**: Efficient O(N log N) gravitational force calculations using spatial partitioning
- **High precision**: Uses f64 floating-point precision for enhanced accuracy
- **Deterministic simulation**: Physics use enhanced determinism for reproducible results
- **Parallel processing**: Multi-threaded physics calculations for optimal performance
- **Dynamic barycenter tracking**: Real-time calculation and visualization of the system's center of mass

### Visualization & Controls

- **3D real-time rendering**: Smooth 3D visualization with Bevy's rendering pipeline
- **Interactive camera**: Pan, orbit, and zoom controls
- **Touch support**: Touch controls for mobile and tablet devices
- **Visual effects**: Bloom effects and tone mapping for enhanced visual quality
- **Dynamic trails**: High-performance fading trails for celestial bodies (optional feature)
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

### Platform Support

- **Native desktop**: Windows, macOS, and Linux support
- **WebAssembly (WASM)**: Browser-based version with WebGL2 support

## Installation

### Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs/)
- **Git**: For cloning the repository

### Clone the Repository

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
cargo run --features trails      # Enable trail visualization
```

### WebAssembly Build

For WebAssembly builds, use the wasm-server-runner tool:

```bash
# Install wasm-server-runner if not present
cargo install wasm-server-runner

# Run the WASM build with the server
cargo run --target wasm32-unknown-unknown --all-features
```

Alternatively, you can manually build and serve:

```bash
# Add WASM target
rustup target add wasm32-unknown-unknown

# Build for WASM
cargo build --target wasm32-unknown-unknown --release --all-features

# Install and use wasm-bindgen
cargo install wasm-bindgen-cli
wasm-bindgen --out-dir out --target web target/wasm32-unknown-unknown/release/stardrift.wasm
```

After building, serve the `out/` directory with any HTTP server:

```bash
cargo install miniserve
miniserve out -p 8000 --index index.html

# Then open http://localhost:8000 in your browser
```

## Usage

### Controls

| Key/Action      | Function                                        |
|-----------------|-------------------------------------------------|
| **Mouse**       | Pan and orbit camera around the simulation      |
| **Mouse Wheel** | Zoom in/out                                     |
| **Space**       | Pause/Resume simulation                         |
| **N**           | Restart simulation with new random bodies       |
| **O**           | Toggle octree visualization on/off              |
| **C**           | Toggle barycenter gizmo visibility on/off       |
| **0-9**         | Set octree visualization depth (0 = all levels) |
| **Escape**      | Quit application                                |
| **Touch**       | Pan, orbit, and zoom (mobile/tablet)            |

### Camera Behavior

- The camera automatically follows the barycenter (center of mass) of the system
- Pan and orbit controls allow you to explore the simulation from different angles
- The camera smoothly tracks the movement of the gravitational system

### Configuration

The simulation features a comprehensive configuration system that allows customization of physics, rendering, and UI
parameters. Configuration is managed through TOML files and supports XDG config directory standards.

#### Configuration Categories

**Physics Configuration:**

- **Body count**: Number of bodies in the simulation (default: 100)
- **Gravitational constant**: Strength of gravitational interactions (default: 1e2)
- **Octree theta**: Barnes-Hut approximation parameter for accuracy/performance balance (default: 0.5)
- **Octree leaf threshold**: Maximum bodies per octree leaf node before subdivision (default: 8)
- **Body distribution**: Sphere radius multiplier (default: 100.0) and minimum distance parameters (default: 0.001)
- **Body size**: Minimum and maximum body radius settings (default: 1.0-2.0)
- **Force calculation**: Minimum distance (default: 2.0) and maximum force limits (default: 1e5)
- **Collision settings**: Restitution coefficient (default: 0.8) and friction coefficient (default: 0.5)
- **Random seed**: Optional seed for deterministic body generation (default: None - random each time)
- **Initial velocity**: Bodies spawn with configurable initial velocities
    - Multiple velocity modes: Random (default), Orbital, Tangential, Radial
    - Configurable speed range (default: 5.0-20.0)
    - Tangential bias for mixed orbital/random motion (default: 0.7)

**Rendering Configuration:**

- **Temperature range**: Min/max temperature for stellar color mapping (default: 2000-15000K)
- **Bloom intensity**: Visual bloom effect strength (default: 100.0)
- **Saturation intensity**: Color saturation level (default: 3.0)
- **Camera settings**: Radius multiplier for camera positioning (default: 2.0)

**Trail Configuration (when trails feature is enabled):**

- **Trail length**: Time-based trail duration in seconds (default: 10.0)
- **Update frequency**: Trail point creation rate (default: 10 FPS)
- **Visual appearance**: Base width, relative sizing to body, bloom factor
- **Fading effects**: Enable/disable fading, fade curves, alpha transparency range
- **Width tapering**: Enable/disable tapering, taper curves, minimum width ratio
- **Blending mode**: Additive or standard blending for trail rendering


#### Configuration File Location

The configuration is automatically loaded from the platform-specific config directory:

- **Linux**: `~/.config/Stardrift/config.toml`
- **macOS**: `~/Library/Application Support/Stardrift/config.toml`
- **Windows**: `%APPDATA%\Stardrift\config.toml`

If no configuration file exists, the application uses sensible defaults and can generate a configuration file for
customization.

## Technical Details

### Architecture

- **Engine**: Bevy 0.16.1 (Entity Component System game engine)
- **Physics**: Avian3D with f64 precision and parallel processing
- **Spatial Optimization**: Barnes-Hut octree algorithm with configurable theta parameter for accuracy/performance
  balance
- **Rendering**: Bevy's PBR (Physically Based Rendering) pipeline with real-time octree wireframe visualization
- **Random Number Generation**: ChaCha8 algorithm for efficient PRNG
- **Mathematical Utilities**: Advanced sphere surface distribution algorithms and statistical validation

### Performance Optimizations

- **Build Profiles**:
    - Development: Fast compilation with basic optimizations
    - Release: Full optimizations with debug info stripped
    - Distribution: Link-time optimization (LTO) and single codegen unit
    - WASM: Size-optimized build for web deployment
- **Algorithmic Efficiency**: Barnes-Hut octree reduces gravitational calculations from O(N²) to O(N log N)
- **Parallel Processing**: Multi-threaded physics calculations
- **Memory Efficiency**: Optimized data structures and minimal allocations
- **Rendering Optimizations**: Efficient mesh and material management

### Dependencies

#### Core Dependencies

- **bevy**: Game engine and rendering framework
- **avian3d**: 3D physics engine with gravitational simulation support
- **bevy_panorbit_camera**: Advanced camera controls with touch support
- **libm**: Mathematical functions for no-std environments
- **rand**: Random number generation
- **rand_chacha**: ChaCha random number generator

#### WASM-Specific Dependencies

- **wasm-bindgen**: Rust-WASM bindings
- **web-sys**: Web API bindings
- **getrandom**: With `wasm_js` backend

### Browser Requirements (WASM Version)

- **WebGL2 & WebAssembly support**: Required for 3D rendering and application execution
- **Minimum browser versions**:
    - Chrome 57+ (WebAssembly requirement)
    - Firefox 52+ (WebAssembly requirement)
    - Safari 15+ (WebGL2 requirement)
    - Edge 79+ (Modern Chromium-based Edge)
- **Hardware acceleration**: Required for optimal performance

## Project Structure

The codebase is organized using a modular architecture designed for maintainability, scalability, and ease of use by AI
agents. The structure follows Rust best practices and separates concerns clearly:

```
src/
├── main.rs                       # Application entry point and plugin registration
├── prelude.rs                    # Common imports and type aliases
├── config.rs                     # Configuration management system
├── states.rs                     # Application state management
├── components/                   # Bevy ECS components (data containers)
│   ├── mod.rs
│   ├── body.rs                   # Celestial body component
│   └── trail.rs                  # Trail component for visual effects
├── plugins/                      # Bevy plugins for modular functionality
│   ├── mod.rs
│   ├── simulation.rs             # Main simulation plugin orchestrating all systems
│   ├── simulation_diagnostics.rs # Simulation metrics and diagnostics plugin
│   ├── diagnostics_hud.rs        # Real-time HUD display plugin
│   ├── embedded_assets.rs        # Embedded asset management plugin
│   └── trails.rs                 # Trail rendering plugin (when trails feature enabled)
├── resources/                    # Bevy ECS resources (global state)
│   └── mod.rs                    # Shared resources like RNG, constants, and octree
├── systems/                      # Bevy ECS systems (game logic)
│   ├── mod.rs
│   ├── physics.rs                # Physics simulation and body management
│   ├── camera.rs                 # Camera controls and barycenter following
│   ├── input.rs                  # Keyboard and interaction handling
│   ├── ui.rs                     # User interface systems
│   ├── visualization.rs          # Octree and visual debugging systems
│   ├── loading.rs                # Asset and resource loading systems
│   ├── simulation_actions.rs     # Simulation control and action handling
│   └── trails.rs                 # Trail update and management systems
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

- **Separation of Concerns**: Each module has a single, well-defined responsibility
- **Plugin Architecture**: Functionality is organized into composable Bevy plugins
- **Resource Management**: Global state is managed through Bevy's resource system
- **System Organization**: Game logic is separated into focused, testable systems
- **Configuration-Driven**: Centralized configuration system for easy customization
- **AI-Friendly Structure**: Clear module boundaries and consistent naming for AI navigation

### Key Modules

- **`plugins/simulation.rs`**: Central orchestrator that coordinates all simulation systems
- **`plugins/simulation_diagnostics.rs`**: Simulation metrics and performance diagnostics
- **`plugins/diagnostics_hud.rs`**: Real-time HUD display for simulation information
- **`plugins/embedded_assets.rs`**: Embedded asset management for web deployment
- **`systems/physics.rs`**: Core physics calculations including octree rebuilding and force application
- **`systems/loading.rs`**: Asset and resource loading management
- **`systems/simulation_actions.rs`**: Simulation control and user action handling
- **`resources/mod.rs`**: Shared state including RNG, gravitational constants, and octree data
- **`utils/math.rs`**: Mathematical utilities for sphere distribution and random vector generation
- **`config.rs`**: Centralized configuration management with serialization support
- **`states.rs`**: Application state management and transitions
- **`physics/octree.rs`**: High-performance Barnes-Hut spatial partitioning implementation

This structure enables easy extension, testing, and maintenance while providing clear entry points for understanding and
modifying the simulation behavior.

## Development

### Development Features

Enable development features for enhanced debugging:

```bash
cargo run --features dev
```

Development features include:

- Asset hot-reloading
- File watching
- Enhanced debugging information
- Dynamic linking for faster compilation

### Available Features

- **`dev`**: Development features for faster iteration
- **`diagnostics`**: Extended performance diagnostics
- **`benchmarks`**: Performance benchmarking capabilities
- **`trails`**: Dynamic trail visualization for celestial bodies

Combine features as needed:

```bash
cargo run --features "dev trails"  # Development with trails
cargo run --all-features            # All features enabled
```

## Troubleshooting

### Common Issues

**WASM build fails**: Ensure you have the latest version of `wasm-bindgen-cli`:

```bash
cargo install wasm-bindgen-cli --force
```

**WebGL2 not supported**: Use a modern browser or enable hardware acceleration in browser settings.

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

## Acknowledgments

- Built with [Bevy](https://bevyengine.org/) game engine
- Physics simulation powered by [Avian3D](https://github.com/Jondolf/avian)
- Camera controls provided by [bevy_panorbit_camera](https://github.com/johanhelsing/bevy_panorbit_camera)
