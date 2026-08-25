# Architecture

This document describes Stardrift's architecture, including its plugin-based design, physics engine, and code organization.

## Overview

Stardrift is built on the [Bevy](https://bevyengine.org/) game engine using the Entity Component System (ECS) pattern. The codebase follows a plugin-based architecture where each major feature is encapsulated in a self-contained plugin.

**Key Technologies:**
- **Language**: Rust
- **Engine**: Bevy 0.17.x (ECS game engine)
- **Physics**: Custom N-body simulation with double precision (f64)
- **Rendering**: Bevy's PBR (Physically Based Rendering) pipeline

## Project Structure

```
src/
├── main.rs              # Application entry point
├── lib.rs               # Library root, plugin registration
├── cli.rs               # Command-line argument parsing
├── config.rs            # Configuration loading and management
├── states.rs            # Application state machine
├── prelude.rs           # Common imports
├── messages.rs          # User-facing messages
│
├── physics/             # Physics engine (not a Bevy plugin)
│   ├── mod.rs
│   ├── components.rs    # Physics components (Mass, Velocity, etc.)
│   ├── resources.rs     # Physics resources (Octree, Barycenter)
│   ├── octree.rs        # Barnes-Hut octree implementation
│   ├── aabb3d.rs        # Axis-aligned bounding box
│   ├── math.rs          # Mathematical utilities
│   └── integrators/     # Numerical integration methods
│       ├── mod.rs
│       ├── registry.rs  # Integrator registration system
│       ├── velocity_verlet.rs
│       ├── symplectic_euler.rs
│       ├── pefrl.rs
│       ├── explicit_euler.rs
│       ├── heun.rs
│       └── runge_kutta.rs
│
├── plugins/             # Bevy plugins
│   ├── mod.rs           # Plugin exports
│   ├── simulation/      # Core simulation plugin
│   │   ├── mod.rs
│   │   ├── components.rs
│   │   ├── physics.rs   # Physics systems
│   │   └── actions.rs   # Simulation commands
│   ├── camera.rs        # Camera controls
│   ├── controls/        # UI controls plugin
│   │   ├── mod.rs
│   │   ├── builder.rs
│   │   ├── constants.rs
│   │   └── buttons/     # Individual button implementations
│   ├── diagnostics_hud.rs
│   ├── trails.rs
│   ├── visualization.rs # Octree/barycenter visualization
│   ├── screenshot/      # Screenshot functionality
│   ├── keep_awake.rs    # Screen sleep prevention
│   ├── attribution.rs   # Attribution display
│   └── embedded_assets.rs
│
├── resources/           # Global resources
│   └── mod.rs
│
└── utils/               # Utility functions
    └── mod.rs
```

## Design Principles

### 1. Plugin-Based Architecture

Each plugin is completely self-contained with its own:
- Systems (logic that runs each frame)
- Components (data attached to entities)
- Resources (global state)
- Events (inter-plugin communication)

Plugins (ideally) don't reach into each other's internals. This enforces clear boundaries and makes the codebase easier to understand and modify.

### 2. Event-Driven Communication

Plugins communicate exclusively through the `SimulationCommand` event system rather than directly accessing each other's state:

```rust
// Example: Triggering a simulation restart from the controls plugin
fn handle_restart_button(
    mut commands: EventWriter<SimulationCommand>,
) {
    commands.send(SimulationCommand::Restart);
}

// The simulation plugin listens for this event
fn handle_simulation_commands(
    mut commands: EventReader<SimulationCommand>,
) {
    for command in commands.read() {
        match command {
            SimulationCommand::Restart => { /* restart logic */ }
            // ...
        }
    }
}
```

### 3. Zero Orchestration

There's no central coordinator managing plugins. Each plugin:
- Registers its own systems
- Responds to relevant events
- Manages its own state

This reduces coupling and makes plugins truly independent.

### 4. Configuration-Driven Behavior

Runtime behavior is controlled through a centralized configuration system:
- TOML configuration file for persistent settings
- Command-line overrides for temporary changes
- Type-safe configuration structs

See [Configuration Reference](configuration.md) for details.

## Core Plugins

### Simulation Plugin

**Location**: `src/plugins/simulation/`

The heart of the application. Manages:
- Body spawning and lifecycle
- Physics updates (via the physics engine)
- Barycenter calculation
- Collision detection (planned)

**Key Systems:**
- `spawn_initial_bodies` - Creates initial celestial bodies
- `update_physics` - Runs physics simulation each frame
- `update_barycenter` - Tracks center of mass

### Camera Plugin

**Location**: `src/plugins/camera.rs`

Handles 3D camera controls using `bevy_panorbit_camera`:
- Pan, orbit, zoom controls
- Touch support for mobile
- Automatic barycenter tracking

### Controls Plugin

**Location**: `src/plugins/controls/`

UI button bar and keyboard shortcuts:
- Toggle buttons for visualization options
- Restart/screenshot buttons
- Keyboard bindings

Uses a builder pattern for button construction.

### Trails Plugin

**Location**: `src/plugins/trails.rs`

Renders fading trails behind moving bodies:
- Trail point recording at configurable intervals
- Multiple fade curves (linear, exponential, smooth)
- Width tapering
- Bloom effects

### Visualization Plugin

**Location**: `src/plugins/visualization.rs`

Debug visualizations:
- Octree wireframe rendering
- Barycenter gizmo (crosshair indicator)

### Diagnostics HUD Plugin

**Location**: `src/plugins/diagnostics_hud.rs`

On-screen display showing:
- Frame rate (FPS)
- Frame count
- Body count

### Screenshot Plugin

**Location**: `src/plugins/screenshot/`

Screenshot capture functionality:
- Manual screenshots (hides UI)
- Automated screenshots (preserves UI for testing)
- Configurable output paths and naming

## Physics Engine

**Location**: `src/physics/`

The physics engine is implemented as a library module (not a Bevy plugin) that the simulation plugin uses. This separation allows the physics code to be tested independently.

### Barnes-Hut Algorithm

For N bodies, naive gravitational calculation is O(N²). The Barnes-Hut algorithm reduces this to O(N log N) by:

1. Building an octree (3D spatial partitioning)
2. For distant body groups, treating them as a single body at their center of mass
3. Using exact calculations only for nearby bodies

The `octree_theta` parameter controls the accuracy/speed tradeoff:
- `theta = 0`: Exact calculation (O(N²))
- `theta = 0.5`: Good balance (default)
- `theta > 0.5`: Faster but less accurate

### Numerical Integration

The physics module implements multiple numerical integrators:

| Integrator | Order | Symplectic | Best For |
|------------|-------|------------|----------|
| Velocity Verlet | 2 | Yes | General use (default) |
| PEFRL | 4 | Yes | Long-term accuracy |
| Symplectic Euler | 1 | Yes | Performance |
| RK4 | 4 | No | Short-term accuracy |

See [Integrators Guide](integrators.md) for detailed selection advice.

### Double Precision

All physics calculations use `f64` (double precision) floating-point arithmetic. This is important for:
- Numerical stability over long simulations
- Accuracy when bodies have very different masses or distances
- Proper energy conservation in symplectic integrators

Rendering uses `f32` since GPU precision requirements are different.

## State Machine

**Location**: `src/states.rs`

The application uses Bevy's state system to manage lifecycle:

```rust
pub enum SimulationState {
    Loading,    // Asset loading
    Running,    // Active simulation
    Paused,     // Simulation paused
}
```

Systems are scheduled to run in specific states, preventing physics updates while paused, for example.

## Performance Optimizations

### Build Profiles

The project uses Cargo build profiles for different use cases:

- **dev**: Fast compilation, some optimizations for dependencies
- **release**: Full optimizations, suitable for normal use
- **dist**: LTO (Link-Time Optimization), single codegen unit, smallest/fastest binary

### Parallel Processing

Physics calculations leverage Rayon for parallel processing where beneficial:
- Octree construction
- Force calculations
- Position/velocity updates

### Benchmark Suite

**Location**: `benches/`

Criterion benchmarks measure:
- Integrator performance
- Accuracy vs analytical solutions
- Convergence order
- Energy conservation
- N-body scaling

Run benchmarks with:
```bash
cargo bench
```

## Platform Support

### Native (Windows, macOS, Linux)

Full feature support with optimal performance. Uses native windowing and input handling.

### WebAssembly

Browser-based version with some limitations:
- WebGL2 for rendering
- No native file system access
- Some features (like screen sleep prevention) unavailable

Build with:
```bash
trunk build --release
```

## See Also

- [Configuration Reference](configuration.md) - Configuration options
- [Integrators Guide](integrators.md) - Numerical integration details
- [Usage Guide](usage.md) - Using the application
