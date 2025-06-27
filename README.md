# Many Body Simulation

A high-performance 3D gravitational many-body simulation built with Rust, Bevy game engine, and Avian3D physics. This
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
- **Barycenter visualization**: Cross-hair indicator showing the system's center of mass
- **Octree visualization**: Real-time wireframe rendering of the spatial partitioning structure
- **Interactive octree controls**: Toggle octree display and adjust visualization depth levels

### User Interface

- **Real-time diagnostics HUD**: On-screen display showing:
    - Frame rate (FPS)
    - Frame count
    - Barycenter coordinates (X, Y, Z)
    - Camera position (X, Y, Z)
- **Pause/Resume functionality**: Space bar to pause and resume the simulation
- **Octree toggle button**: Interactive UI button to show/hide octree visualization

### Platform Support

- **Native desktop**: Windows, macOS, and Linux support
- **WebAssembly (WASM)**: Browser-based version with WebGL2 support

## Installation

### Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs/)
- **Git**: For cloning the repository

### Clone the Repository

```bash
git clone https://github.com/emilyst/many_body_simulation.git
cd many_body_simulation
```

### Native Build

```bash
# Development build (faster compilation)
cargo run --features dev

# Release build (optimized performance)
cargo run --release
```

### WebAssembly Build

The project includes a convenient build script for WASM deployment:

```bash
# Make the script executable (Unix/Linux/macOS)
chmod +x build_wasm.sh

# Run the build script
./build_wasm.sh
```

The script will:

1. Install the `wasm32-unknown-unknown` target
2. Install `wasm-bindgen-cli` if not present
3. Build the project with WASM optimizations
4. Generate web bindings
5. Compress the WASM file with gzip
6. Copy the HTML file to the output directory

After building, serve the `out/` directory with any HTTP server:

```bash
# Using Python
python -m http.server 8000 --directory out

# Using Node.js (if you have http-server installed)
npx http-server out

# Then open http://localhost:8000 in your browser
```

## Usage

### Controls

| Key/Action      | Function                                   |
|-----------------|--------------------------------------------|
| **Mouse**       | Pan and orbit camera around the simulation |
| **Mouse Wheel** | Zoom in/out                                |
| **Space**       | Pause/Resume simulation                    |
| **O**           | Toggle octree visualization on/off         |
| **0-9**         | Set octree visualization depth (0 = all levels) |
| **Escape**      | Quit application                           |
| **Touch**       | Pan, orbit, and zoom (mobile/tablet)       |

### Camera Behavior

- The camera automatically follows the barycenter (center of mass) of the system
- Pan and orbit controls allow you to explore the simulation from different angles
- The camera smoothly tracks the movement of the gravitational system

### Simulation Parameters

The simulation can be customized by modifying the constants in the source code:

- **Body count**: Number of bodies in the simulation (default: configurable)
- **Gravitational constant**: Strength of gravitational interactions

## Technical Details

### Architecture

- **Engine**: Bevy 0.16.* (Entity Component System game engine)
- **Physics**: Avian3D with f64 precision and parallel processing
- **Spatial Optimization**: Barnes-Hut octree algorithm with configurable theta parameter for accuracy/performance balance
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

The codebase is organized using a modular architecture designed for maintainability, scalability, and ease of use by AI agents. The structure follows Rust best practices and separates concerns clearly:

```
src/
├── main.rs                 # Application entry point and plugin registration
├── components/             # Bevy ECS components
│   └── mod.rs             # UI and simulation components
├── config.rs              # Configuration management system
├── plugins/               # Bevy plugins for modular functionality
│   ├── mod.rs
│   ├── simulation.rs      # Main simulation plugin orchestrating all systems
│   ├── diagnostics.rs     # Simulation metrics and diagnostics plugin
│   └── diagnostics_hud.rs # Real-time HUD display plugin
├── resources/             # Bevy ECS resources (global state)
│   └── mod.rs            # Shared resources like RNG, constants, and octree
├── systems/              # Bevy ECS systems (game logic)
│   ├── mod.rs
│   ├── physics.rs        # Physics simulation and body management
│   ├── camera.rs         # Camera controls and barycenter following
│   ├── input.rs          # Keyboard and interaction handling
│   ├── ui.rs             # User interface systems
│   └── visualization.rs  # Octree and visual debugging systems
├── utils/                # Utility modules
│   ├── mod.rs
│   ├── math.rs           # Mathematical functions and algorithms
│   └── color.rs          # Color and material utilities
└── physics/              # Physics-specific modules
    ├── mod.rs
    ├── stars.rs          # Stellar physics and realistic body generation
    └── octree.rs         # Barnes-Hut octree implementation
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
- **`systems/physics.rs`**: Core physics calculations including octree rebuilding and force application
- **`resources/mod.rs`**: Shared state including RNG, gravitational constants, and octree data
- **`utils/math.rs`**: Mathematical utilities for sphere distribution and random vector generation
- **`config.rs`**: Centralized configuration management with serialization support
- **`physics/octree.rs`**: High-performance Barnes-Hut spatial partitioning implementation

This structure enables easy extension, testing, and maintenance while providing clear entry points for understanding and modifying the simulation behavior.

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
