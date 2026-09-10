# Stardrift

A 3D gravitational N-body simulation built with Rust and the Bevy game engine. Simulates gravitational interactions between celestial bodies with real-time visualization and interactive controls.

## Project Status

Stardrift is an experimental side project for personal enjoyment and learning. Development happens in spare time without specific goals or timeline.

This project also serves as an exploration of AI capabilities in software development. Its development involves use of AI assistance.

**Note**: This is not intended for scientific or research purposes. The project remains experimental and subject to significant changes.

## Features

- **N-body gravitational physics** with Barnes-Hut octree optimization (O(N log N))
- **Multiple numerical integrators** including symplectic and Runge-Kutta methods
- **Merging collisions** with swept (tunnel-proof) detection and momentum-conserving inelastic merges
- **Real-time 3D visualization** with bloom effects, trails, and octree/barycenter overlays
- **Interactive camera** with pan, orbit, zoom, and touch support
- **Cross-platform**: Windows, macOS, Linux, and WebAssembly

## Installation

### Pre-built Packages

Download from the [releases page](https://github.com/emilyst/stardrift/releases):

| Platform | Formats |
|----------|---------|
| Linux | `.tar.gz` (x86_64, ARM64) |
| Windows | `.zip` (x86_64, ARM64) |
| macOS | `.dmg`, `.tar.gz` (Apple Silicon) |
| Web | WebAssembly package |

All packages include SHA256 checksums and [build provenance attestations](docs/release.md#build-provenance) for verification.

### Building from Source

```bash
# Prerequisites: Rust (https://rustup.rs/) and Git

git clone https://github.com/emilyst/stardrift.git
cd stardrift

# Development build
cargo run -p stardrift
```

For WebAssembly builds, see the [Usage Guide](docs/usage.md#platform-specific-notes).

## Quick Start

```bash
# Basic run (borderless fullscreen; Escape quits)
stardrift

# Run in a regular window instead
stardrift --windowed

# Customize body count and seed
stardrift --bodies 150 --seed 42

# Try different color scheme
stardrift --color-scheme viridis

# See all options
stardrift --help

# Seed a config file with the defaults (see docs/configuration.md for its location)
stardrift --print-default-config > config.toml
```

## Controls

| Key/Action | Function |
|------------|----------|
| **Left-drag** | Orbit camera |
| **Right-drag** | Pan camera |
| **Mouse Wheel** | Zoom in/out |
| **Space** | Pause/Resume |
| **N** | New simulation |
| **O** | Toggle octree visualization |
| **C** | Toggle barycenter gizmo |
| **T** | Toggle trails |
| **D** | Toggle diagnostics HUD |
| **S** | Take screenshot |
| **Q** / **Escape** | Quit (desktop only) |

Touch controls are supported on mobile/tablet devices.

## Documentation

| Document | Description |
|----------|-------------|
| [Usage Guide](docs/usage.md) | Controls, CLI options |
| [Configuration](docs/configuration.md) | Configuration options |
| [Integrators](docs/integrators.md) | Numerical integration methods |
| [Color Schemes](docs/color-schemes.md) | Available color palettes |
| [Architecture](docs/architecture.md) | Project structure and design |
| [Release Process](docs/release.md) | Version management and builds |

## License

This project is dedicated to the public domain under the [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/) license. See the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Bevy](https://bevyengine.org/) game engine
- Camera controls by [bevy_panorbit_camera](https://github.com/johanhelsing/bevy_panorbit_camera)
