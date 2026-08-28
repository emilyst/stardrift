# Architecture

This document describes Stardrift's architecture, including its plugin-based design, physics engine, and code organization.

## Overview

Stardrift is built on the [Bevy](https://bevyengine.org/) game engine using the Entity Component System (ECS) pattern. The codebase follows a plugin-based architecture where each major feature is encapsulated in a self-contained plugin.

**Key Technologies:**
- **Language**: Rust
- **Engine**: Bevy 0.19.x (ECS game engine)
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
├── messages.rs          # SimulationCommand message definitions
│
├── physics/             # Physics engine (not a Bevy plugin)
│   ├── components.rs    # Physics components (Mass, Velocity, etc.)
│   ├── resources.rs     # Physics resources (timing, current integrator)
│   ├── octree.rs        # Barnes-Hut octree implementation
│   ├── aabb3d.rs        # Axis-aligned bounding box
│   ├── math.rs          # Type aliases (Scalar, Vector) and math utilities
│   └── integrators/     # Numerical integration methods + registry
│
├── plugins/             # Bevy plugins
│   ├── simulation/      # Core simulation: spawning, physics driver,
│   │                    #   collisions, command handling
│   ├── bodies/          # Body rendering (shared mesh + material + WGSL shader)
│   ├── camera.rs        # Camera setup and controls
│   ├── controls/        # Keyboard bindings and UI button bar
│   ├── diagnostics_hud.rs
│   ├── simulation_diagnostics.rs
│   ├── trails/         # GPU ribbon trails (data + material + WGSL shader)
│   ├── visualization.rs # Octree/barycenter visualization
│   ├── screenshot/      # Manual and automated capture
│   ├── keep_awake.rs    # Screen sleep prevention
│   ├── attribution.rs   # Version/attribution display
│   └── embedded_assets.rs
│
├── resources/           # Global resources (RNGs, body count, etc.)
└── utils/               # Utilities (color scheme generators)
```

## Design Principles

### 1. Plugin-Based Architecture

Each plugin is self-contained with its own:
- Systems (logic that runs each frame)
- Components (data attached to entities)
- Resources (global state)
- Messages (inter-plugin communication)

Plugins (ideally) don't reach into each other's internals. This enforces clear boundaries and makes the codebase easier to understand and modify.

### 2. Message-Driven Communication

Plugins communicate through the `SimulationCommand` message rather than directly accessing each other's state:

```rust
// Triggering a restart from the controls plugin
fn handle_restart(mut commands: MessageWriter<SimulationCommand>) {
    commands.write(SimulationCommand::Restart);
}

// The simulation plugin listens for the command
fn handle_simulation_commands(mut commands: MessageReader<SimulationCommand>) {
    for command in commands.read() {
        match command {
            SimulationCommand::Restart => { /* restart logic */ }
            // ...
        }
    }
}
```

### 3. Zero Orchestration

There's no central coordinator managing plugins. Each plugin registers its own systems, responds to relevant messages, and manages its own state. This reduces coupling and keeps plugins independent.

### 4. Configuration-Driven Behavior

Runtime behavior is controlled through a centralized configuration system: a TOML file for persistent settings, command-line overrides for temporary changes, and type-safe configuration structs. See [Configuration Reference](configuration.md).

## Core Plugins

### Simulation Plugin

**Location**: `src/plugins/simulation/`

The heart of the application. Manages:
- Body spawning and lifecycle
- The staged integration driver (see [Integration design](integration.md))
- Barycenter calculation
- Merge-on-contact collisions (swept detection; momentum-conserving inelastic merges)

### Bodies Plugin

**Location**: `src/plugins/bodies/`

Renders every physics body as a camera-facing disc impostor: one shared quad mesh, billboarded in the vertex shader (`body.wgsl`) and shaded with a fake sphere normal reproducing the ambient/fresnel and bloom-emissive look the earlier per-body `StandardMaterial` spheres produced. Per-body color travels in `MeshTag` rather than the material, so all bodies share one mesh handle and one material handle and batch into a single instanced draw. The simulation plugin spawns bodies with physics state only; this plugin attaches mesh, material, color, and a billboard-safe bounding volume reactively on `Added<PhysicsBody>`, and keeps `Transform::scale` in sync with `Radius` (including on collision merges) — with a unit mesh, scale is the radius.

### Camera Plugin

**Location**: `src/plugins/camera.rs`

Sets up the 3D camera using `bevy_panorbit_camera`: pan, orbit, and zoom, with touch and trackpad support. The focus is fixed at the world origin — bodies spawn in the center-of-momentum frame, so the barycenter starts there and stays nearby. The initial distance is derived from the spawn region size.

### Controls Plugin

**Location**: `src/plugins/controls/`

Keyboard bindings and the UI button bar. Buttons are constructed with a builder pattern and dispatch `SimulationCommand` messages.

### Trails Plugin

**Location**: `src/plugins/trails/`

Renders fading trails behind moving bodies as GPU ribbons. Points are recorded on the CPU at configurable intervals; ribbon expansion, camera-facing width, fade curves, and bloom run in a custom vertex shader (`trail.wgsl`), with width tapering and per-point body radius baked into the vertex data. Trail geometry is re-uploaded only when the recorded point set changes, not every frame.

### Visualization Plugin

**Location**: `src/plugins/visualization.rs`

Debug visualizations: octree wireframe rendering and the barycenter gizmo (cross-hair indicator).

### Diagnostics HUD Plugin

**Location**: `src/plugins/diagnostics_hud.rs`

On-screen display of frame rate, frame count, and body count.

### Screenshot Plugin

**Location**: `src/plugins/screenshot/`

Manual screenshots (hides UI) and automated capture (preserves UI for testing), with configurable output paths and naming.

## Physics Engine

**Location**: `src/physics/`

The physics engine is implemented as a library module (not a Bevy plugin) that the simulation plugin drives. This separation allows the physics code to be tested independently.

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

Multiple integrators are available, from symplectic methods (velocity Verlet, PEFRL) to classic Runge-Kutta. Selection, trade-offs, and the staged integration protocol are covered in the [Integrators Guide](integrators.md) and [Integration design](integration.md).

### Double Precision

All physics calculations use `f64` (double precision) floating-point arithmetic, via the `Scalar` and `Vector` type aliases. This matters for numerical stability over long simulations, accuracy across widely varying masses and distances, and the conservation behavior of the symplectic integrators. Rendering uses `f32`, matching GPU precision.

## State Machine

**Location**: `src/states.rs`

The application uses Bevy's state system with a two-state machine:

```rust
pub enum AppState {
    Running,    // Active simulation (default)
    Paused,     // Simulation paused
}
```

Systems are scheduled to run in specific states — physics updates stop while paused, for example.

## Performance

### Build Profiles

- **dev**: Fast compilation; dependencies still optimized at level 2
- **release**: Full optimization — LTO, single codegen unit, stripped symbols
- **bench**: Inherits from release

### Parallel Processing

Force calculations and transform updates are parallelized across Bevy's `ComputeTaskPool` (`par_chunk_map_mut`/`par_iter_mut`), chunked by body count.

### Benchmarks and Correctness Tests

**Location**: `benches/`, `tests/`

Criterion benchmarks (`cargo bench`) cover octree construction/traversal and the full simulation step at various body counts. Integrator accuracy, convergence order, and conservation properties are asserted by the test suites (`tests/integrator_correctness.rs`, `tests/two_body_system.rs`), not benchmarked.

## Platform Support

### Native (Windows, macOS, Linux)

Full feature support with optimal performance. Uses native windowing and input handling.

### WebAssembly

Browser-based version with some limitations:
- WebGL2 for rendering
- No configuration file or command line
- Some features (like screen sleep prevention and quitting) unavailable

Build with:
```bash
trunk build --release
```

## See Also

- [Configuration Reference](configuration.md) - Configuration options
- [Integrators Guide](integrators.md) - Numerical integration details
- [Integration design](integration.md) - The staged integration pipeline
- [Usage Guide](usage.md) - Using the application
