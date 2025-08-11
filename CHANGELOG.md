# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- macOS DMG files now include an Applications folder symlink for easier drag-and-drop installation

## [0.0.32] - 2025-08-11

### Added

- Configurable barycentric drift correction option
    - Added `barycentric_drift_correction` boolean field to physics configuration (defaults to true)
    - When enabled (default), simulation automatically recenters around the barycenter for visual stability
    - When disabled, bodies drift naturally according to gravitational interactions for pure physics accuracy
    - Maintains backwards compatibility with existing configurations
    - Provides control over trade-off between visual stability and scientific accuracy

### Changed

- **BREAKING**: Restructured project to use Cargo workspace architecture
    - Main application moved to `crates/stardrift/` directory
    - Procedural macros moved to `crates/stardrift-macros/` directory
    - Created library target (`lib.rs`) alongside binary for better modularity
    - Assets relocated to `crates/stardrift/assets/` for proper encapsulation
    - Enables integration testing, benchmarking, and future extensibility
    - GitHub Actions workflows updated to support workspace structure
    - Commands now require package specification: `cargo run -p stardrift`

## [0.0.31] - 2025-08-10

## [0.0.30] - 2025-08-10

### Added

- Four new numerical integrators with proper accuracy guarantees
    - **Heun (Improved Euler)**: 2nd order explicit integrator with better stability than basic Euler
    - **Runge-Kutta 2nd Order (Midpoint)**: 2nd order explicit integrator with midpoint evaluation
    - **Runge-Kutta 4th Order**: Classical 4th order integrator with highest accuracy
    - **PEFRL (Position-Extended Forest-Ruth-Like)**: 4th order symplectic integrator optimized for long-term energy conservation
    - All integrators now support multiple convenient aliases (e.g., `"rk4"`, `"improved_euler"`, `"forest_ruth"`)
    - All new integrators are included in comprehensive benchmark suite

- Force evaluator architecture for accurate multi-stage integration
    - Added `ForceEvaluator` trait allowing integrators to query forces at arbitrary positions
    - Enables mathematically correct implementation of Velocity Verlet, RK4, and other multi-stage methods
    - Forces can now be calculated at intermediate positions without rebuilding the octree
    - All integrators now achieve their theoretical order of convergence

### Changed

- Default physics configuration values updated for better stability
    - `gravitational_constant` reduced from `500.0` to `0.01` for more realistic forces
    - `octree_leaf_threshold` increased from `2` to `4` for better performance/accuracy balance
    - `min_speed` increased from `5.0` to `10.0` for more dynamic initial velocities
    - `max_speed` increased from `20.0` to `100.0` to match new gravitational scaling

- **BREAKING**: Integrator configuration format updated
    - Configuration now uses `integrator.type` instead of `integrator` field
    - All integrator names use snake_case format (e.g., `"velocity_verlet"` instead of `"VelocityVerlet"`)
    - Multiple aliases supported for user convenience (see README for full list)

- Major architectural simplification and accuracy improvements
    - Removed over 1200 lines of unnecessary abstraction code
    - Eliminated history-based infrastructure (`KinematicHistory`, `MultiStepIntegrator` trait)
    - Removed complex multi-stage abstractions (`MultiStageIntegrator` trait)
    - Simplified registry pattern from factory-based to simple match statement
    - Single, unified integration system handles all methods consistently

- Fixed critical accuracy issues across all integrators
    - **Velocity Verlet**: Now correctly recalculates forces (0.1% energy drift vs 619% previously)
    - **RK4**: Achieves true 4th order convergence (was artificially limited to 1st order)
    - **Heun and RK2**: Now achieve proper 2nd order convergence as mathematically expected
    - All integrators properly implement their theoretical properties

### Removed

- **BREAKING**: Adams-Bashforth integrator and all related infrastructure
    - Complexity of maintaining history for one rarely-used integrator was not justified
    - Users should use RK4 for high-order accuracy or Velocity Verlet for symplectic integration
    - Removed associated configuration files, tests, and documentation

## [0.0.29] - 2025-08-09

### Added

- Configurable integrator selection in physics configuration
    - Added `integrator` field to `PhysicsConfig` with `IntegratorType` enum
    - Currently supports `SymplecticEuler` integrator (default) and `VelocityVerlet`
    - Integrator is now selected at startup based on configuration
    - Foundation for adding additional integrator types in the future

- Velocity Verlet integrator implementation
    - Second-order symplectic integrator with excellent energy conservation
    - Implements `Integrator` trait with force evaluator support
    - Correctly recalculates forces at new position for true energy conservation
    - Particularly suited for high-precision gravitational n-body simulations

- Support for kinematic history in integrators [REMOVED in Unreleased]
    - Added `KinematicHistory` component with fixed size of 8 states (sufficient for most methods)
    - Added `MultiStepIntegrator` trait for integrators that use historical data
    - Split integration into two systems: `integrate_motions_simple` and `integrate_motions_with_history`
    - Systems automatically route bodies based on presence of `KinematicHistory` component
    - Enables implementation of multi-step integration methods (Adams-Bashforth, Runge-Kutta, etc.)
    - Added comprehensive documentation for kinematic state management
    - NOTE: This infrastructure was removed in a later refactor for simplification

### Changed

- Configuration enums now use snake_case format in TOML files
    - Version bumped to 7 to ensure compatibility
    - Affected enums: `VelocityMode`, `FadeCurve`, `TaperCurve`, `IntegratorType`
    - Example: `velocity_mode = "tangential"` instead of `"Tangential"`
    - Makes configuration files more consistent with TOML conventions

- Renamed `SemiImplicitEuler` integrator to `SymplecticEuler`
    - Better emphasizes the method's key property of preserving symplectic structure
    - More commonly recognized name in scientific computing literature
    - File renamed from `semi_implicit_euler.rs` to `symplectic_euler.rs`
    - All references updated throughout codebase including configuration

## [0.0.28] - 2025-08-07

### Added

- Gravitational softening parameter for improved numerical stability
    - Added `force_calculation_softening` configuration parameter (default: 0.5)
    - Prevents force singularities during close gravitational encounters by modifying force calculation from F = GMm/r²
      to F = GMm/(r² + ε²)
    - Provides smoother force transitions and eliminates numerical instabilities when bodies approach each other closely
    - Configurable via physics section in config.toml with typical values between 0.1-1.0 times minimum body radius

### Changed

- Refactored several types' names to make it clear that integrators can be non-symplectic as well
    - Clarified some documentation as well

## [0.0.27] - 2025-08-06

### Changed

- Implement new simulation configuration defaults

## [0.0.26] - 2025-08-06

### Changed

- Complete physics system overhaul
    - Migrated from Avian3D physics engine to custom integrator implementation
    - Introduced high-precision `Position` and `Velocity` components using `f64` internally
    - Added `PhysicsBody` marker component for cleaner entity queries
    - Decoupled physics calculations from rendering transforms for better numerical stability
    - Split physics update into distinct phases: BuildOctree, CalculateAccelerations, IntegrateMotions, SyncTransforms,
      CorrectBarycentricDrift
    - Physics now uses acceleration-based approach rather than direct force application

### Added

- Custom symplectic integrator system with pluggable implementations
    - Semi-implicit Euler integrator for energy conservation
    - `ActiveSymplecticIntegrator` resource for runtime integrator selection
- Independent `PhysicsTime` resource for frame-rate independent physics updates
- Dedicated math module with high-precision vector operations
- New `PhysicsBodyBundle` for spawning physics-enabled entities

### Removed

- Dependency on Avian3D physics engine (removed from Cargo.toml)
- Collision handling temporarily disabled pending physics system redesign
- `stars.rs` module containing unused stellar simulation code
- `utils/math.rs` module (functionality moved to `physics/math.rs`)

## [0.0.25] - 2025-08-05

### Changed

- Significantly lower max force
    - Reduces chance of bodies getting stuck together

## [0.0.24] - 2025-08-04

- Use pointer cursor (hand) uniformly for all controls

## [0.0.23] - 2025-08-04

### Changed

- Attribution text cursor feedback and font update
    - Attribution text now displays pointer cursor when hovered
    - Changed font from Saira-Regular to Saira-Light
- Diagnostics HUD is now hidden by default
    - Can be toggled on with the 'D' key or UI button

## [0.0.22] - 2025-08-04

## [0.0.21] - 2025-08-04

## [0.0.20] - 2025-08-04

### Added

- Diagnostics HUD visibility toggle functionality
    - New keyboard shortcut 'D' to show/hide diagnostics HUD during simulation
    - UI button that dynamically updates between "Show Diagnostics (D)" and "Hide Diagnostics (D)"
    - HUD continues to collect performance data even when hidden
    - Efficient implementation using Bevy's built-in Visibility component

## [0.0.19] - 2025-08-04

### Changed

- Removed all compile-time feature flags
    - Removed `[features]` section from `Cargo.toml`
    - All functionality now included by default
    - No longer need `--all-features` or `--features` build flags

## [0.0.18] - 2025-08-03

### Changed

- Trail rendering system architecture
    - Decoupled trails from body lifecycle events
    - Introduced `TrailRenderParams` for centralized rendering configuration
    - Added `TrailBundle` for cleaner entity spawning and better ECS patterns
    - Simplified trail initialization process
    - Removed unused system parameters
    - Consolidated trail-specific components into a single, type-safe bundle
    - Eliminated explicit `NoFrustumCulling` component assignment
    - Improved type safety and code organization for trail entity creation

- Octree visualization rendering
    - Reduced line width from 2.0 to 1.0 pixels
    - Added configurable line color with 50% opacity by default
    - Disabled perspective scaling for consistent line appearance
    - Improved visibility of bodies and trails during simulation

### Fixed

- Trails now render continuously throughout the simulation
- Trails persist when bodies are despawned, preparing for future collision/merging features
- Keyboard input handling for non-QWERTY keyboard layouts
    - Switched from physical key codes (KeyCode) to logical key values (KeyboardInput events)
    - Keys now respond to the character printed on them regardless of keyboard layout
    - All keyboard shortcuts (N, O, C, S, T, Q, Space, 0-9) now work correctly on AZERTY, Dvorak, and other layouts

## [0.0.17] - 2025-08-02

### Changed

- Packaging system simplification
    - macOS: Now creates `.app` bundles directly using shell commands
    - macOS: Uses `plutil` commands to generate Info.plist directly

### Removed

- Linux packaging support (`.deb` and `.rpm` packages)
    - Removed `cargo-deb` and `cargo-generate-rpm` dependencies from CI
    - Removed `[package.metadata.deb]` and `[package.metadata.generate-rpm]` sections from Cargo.toml
    - Linux distributions now only receive `.tar.gz` archives
- Dependency on `cargo-bundle` in all workflows
- `[package.metadata.bundle]` configuration from Cargo.toml

## [0.0.16] - 2025-08-02

### Changed

- WebAssembly build process changes
    - Migrated from manual `wasm-bindgen` build script to `trunk` for automated WASM builds
    - Removed custom `wasm` profile in favor of trunk's built-in optimizations
    - Modified `index.html` to work with trunk's asset injection
    - Added gzip compression as post-build hook

- CI/CD binary naming changes
    - All release artifacts now include version in filename (e.g., `stardrift-0.0.15-x86_64-unknown-linux-gnu.tar.gz`)
    - Bundle outputs include version: `.dmg` (macOS)
    - Updated build attestation to include all bundle formats

## [0.0.15] - 2025-08-02

## [0.0.14] - 2025-08-02

## [0.0.13] - 2025-08-01

## [0.0.12] - 2025-07-31

### Added

- Trail visibility toggle functionality
    - New keyboard shortcut 'T' to show/hide trails during simulation
    - UI button that dynamically updates between "Show Trails" and "Hide Trails"
    - Trails continue to accumulate data even when hidden
    - Efficient implementation using Bevy's built-in Visibility component

## [0.0.11] - 2025-07-31

## [0.0.10] - 2025-07-30

## [0.0.9] - 2025-07-30

## [0.0.8] - 2025-07-29

## [0.0.7] - 2025-07-29

## [0.0.6] - 2025-07-28

### Changed

- Made trails visualization a default feature
    - Trail effects are now included in standard builds without requiring feature flags
    - Users can still opt out with `--no-default-features --features graphics`

## [0.0.5] - 2025-07-28

### Changed

- Replaced SairaSemiCondensed fonts with Saira fonts across the UI for improved readability
    - Updated controls UI, diagnostics HUD, and attribution text
    - Simplified font variants from 9 to 3 (Regular, Light, Bold)
    - Reduced total font file size in embedded assets

## [0.0.4] - 2025-07-28

## [0.0.3] - 2025-07-28

### Added

- Native DMG disk images with proper .app bundles
- ARM64 support for Linux and Windows builds using GitHub's native ARM runners
- Bundle metadata in Cargo.toml for application identity and descriptions

### Changed

- Consolidated distribution profile into release profile for simplification
    - Moved all optimizations (LTO, single codegen unit, stripping) to release profile
    - Removed separate distribution profile to work better with cargo-bundle
    - Updated all workflows to use standard --release flag
- Enhanced release workflow to generate both DMG packages (macOS) and portable binaries
- Fixed sha256 checksum generation for macOS builds (using shasum instead of sha256sum)

### Fixed

- Linux and macOS ARM64 builds now compile correctly without cross-compilation
- Added missing system dependencies for Linux builds in release workflow

## [0.0.2] - 2025-07-27

### Added

- GitHub Actions CI/CD pipeline for automated testing and releases
    - Continuous integration workflow for all commits and pull requests
    - Automated cross-platform binary builds for releases
    - Support for Linux (x86_64/aarch64), Windows (x86_64/aarch64), macOS (Intel/Apple Silicon)
    - WebAssembly build automation with wasm-bindgen
    - Security vulnerability scanning via cargo-audit
- Build provenance attestation for all release binaries
    - Cryptographic signatures proving binaries were built by official CI/CD
    - Uses Sigstore and follows SLSA standard for supply chain security
    - Verifiable via GitHub CLI: `gh attestation verify <binary> --repo emilyst/stardrift`
    - Attestations for all platform binaries and WASM builds

### Security

- Added build provenance attestations to prevent tampering with release binaries
- Integrated cargo-audit for automated dependency vulnerability scanning

## [0.0.1] - 2025-07-27

### Added

- Screenshot functionality with configurable options
    - Press 'S' key or click Screenshot button to capture
    - Automatically hides UI and diagnostics HUD during capture
    - Configurable save directory via `screenshots.directory` in config.toml
    - Customizable filename prefix and timestamp options
    - Frame delay configuration for ensuring UI is fully hidden
    - Creates screenshot directory automatically if it doesn't exist
- Screenshot configuration section in config.toml with the following options:
    - `directory`: Custom save location (default: current directory)
    - `filename_prefix`: Filename prefix (default: "stardrift_screenshot")
    - `include_timestamp`: Add timestamp to filenames (default: true)
    - `notification_enabled`: Log captures (default: true)
    - `hide_ui_frame_delay`: Frames to wait before capture (default: 2)
- Chrono dependency for timestamp generation in screenshots
- Quit button and 'Q' key shortcut for application exit (non-WASM platforms only)
- Comprehensive benchmark suite reorganization:
    - Organized into focused groups: construction, physics, realworld, configurations, characteristics
    - Added configuration profiles for testing different accuracy/performance trade-offs
    - Created benchmark-specific configs: fast_inaccurate, balanced, high_accuracy, stress_test
    - All benchmarks now report throughput metrics for easier comparison
    - Pre-generate test data outside benchmark loops to reduce measurement noise
- Benchmark configuration profiles in `configs/benchmark_profiles/` directory
- Attribution plugin displaying "Stardrift v{version} ({date})" in bottom-right corner
    - Shows application name and version from Cargo.toml
    - Includes build date captured at compile time via build script
    - Remains visible during screenshots for proper attribution
    - Uses 10px Regular font with 30% opacity for subtlety
    - Self-contained plugin following established architecture patterns
- Build script (`build.rs`) to capture compile-time date using chrono crate
- Extended `EmbeddedAssetsPlugin` to include SairaSemiCondensed-Regular font

### Changed

- **BREAKING**: Complete architectural transformation to pure self-contained plugin pattern
    - Eliminated orchestration patterns in favor of event-driven communication
    - All plugins now self-contained with internal systems and clear boundaries
    - Removed entire `src/systems/` directory - all code migrated to plugins
    - Plugin communication exclusively through `SimulationCommand` events
    - Large plugins use internal submodules for organization (e.g., `simulation/physics.rs`)
- Plugin architecture reorganization:
    - `ControlsPlugin`: Complete input handling and UI structure (was `systems/ui.rs` + `systems/input.rs`)
    - `SimulationPlugin`: Self-contained physics with submodules (was orchestration plugin)
    - `CameraPlugin`: Camera setup and positioning (was `systems/camera.rs`)
    - `VisualizationPlugin`: Debug rendering for octree and barycenter (was `systems/visualization.rs`)
    - All feature-gated plugins remain self-contained (`DiagnosticsHudPlugin`, `TrailsPlugin`, `EmbeddedAssetsPlugin`)
- Event system centralized in `src/events.rs` with `SimulationCommand` enum for inter-plugin communication
- Created library structure with `src/lib.rs` to enable integration testing
- Added comprehensive test infrastructure:
    - `src/test_utils.rs` for shared test utilities
    - Unit tests remain embedded in plugin modules
- UI system includes UIRoot marker component for programmatic visibility control
- Diagnostics HUD includes DiagnosticsHudRoot marker component
- Font system switched from BerkeleyMono to Saira Semi-Condensed family
- UI layout repositioned from bottom-right horizontal to top-left vertical
- Button styling with subtle backgrounds and fixed dimensions
- Diagnostics HUD with right-aligned values and enhanced contrast
- Button dimensions and spacing adjusted for visual hierarchy
- UI button system refactored to event-driven architecture
    - Text updates triggered by events with computed text
    - Code duplication eliminated through shared functions
    - CommandButton trait pattern for consistent button behavior
    - Improved extensibility for future button additions
- Component organization aligned with self-contained plugin architecture:
    - Trail component moved into TrailsPlugin (`src/plugins/trails.rs`)
    - BodyBundle moved into SimulationPlugin (`src/plugins/simulation/components.rs`)
    - Eliminated separate `src/components/` directory
    - Plugin-specific components now co-located with their plugins

### Fixed

- Digit keys (0-9) now properly control octree visualization depth through command pattern

### Deprecated

- N/A

### Removed

- Attribution text that was mistakenly added to controls UI
    - Replaced with proper version attribution in bottom-right corner

### Security

- N/A

## [0.0.1] - Initial Release

### Added

- Core N-body gravitational simulation with Barnes-Hut octree optimization
- Real-time 3D visualization with Bevy engine
- Interactive camera controls (pan, orbit, zoom)
- Touch support for mobile devices
- Configurable physics parameters
- Dynamic trail visualization (optional feature)
- Real-time diagnostics HUD
- Octree visualization toggle
- Barycenter tracking and visualization
- Pause/resume functionality
- Simulation restart with new random bodies
- Comprehensive configuration system via TOML files
- WebAssembly support for browser deployment
- Multiple build profiles (dev, release, distribution, wasm)
- Platform-specific configuration paths (XDG compliant)

[Unreleased]: https://github.com/emilyst/stardrift/compare/v0.0.31...HEAD
[0.0.31]: https://github.com/emilyst/stardrift/compare/v0.0.30...v0.0.31
[0.0.30]: https://github.com/emilyst/stardrift/compare/v0.0.29...v0.0.30
[0.0.29]: https://github.com/emilyst/stardrift/compare/v0.0.28...v0.0.29

[0.0.28]: https://github.com/emilyst/stardrift/compare/v0.0.27...v0.0.28

[0.0.27]: https://github.com/emilyst/stardrift/compare/v0.0.26...v0.0.27

[0.0.26]: https://github.com/emilyst/stardrift/compare/v0.0.25...v0.0.26

[0.0.25]: https://github.com/emilyst/stardrift/compare/v0.0.24...v0.0.25

[0.0.24]: https://github.com/emilyst/stardrift/compare/v0.0.23...v0.0.24

[0.0.23]: https://github.com/emilyst/stardrift/compare/v0.0.22...v0.0.23

[0.0.22]: https://github.com/emilyst/stardrift/compare/v0.0.21...v0.0.22

[0.0.21]: https://github.com/emilyst/stardrift/compare/v0.0.20...v0.0.21

[0.0.20]: https://github.com/emilyst/stardrift/compare/v0.0.19...v0.0.20

[0.0.19]: https://github.com/emilyst/stardrift/compare/v0.0.18...v0.0.19

[0.0.18]: https://github.com/emilyst/stardrift/compare/v0.0.17...v0.0.18

[0.0.17]: https://github.com/emilyst/stardrift/compare/v0.0.16...v0.0.17

[0.0.16]: https://github.com/emilyst/stardrift/compare/v0.0.15...v0.0.16

[0.0.15]: https://github.com/emilyst/stardrift/compare/v0.0.14...v0.0.15

[0.0.14]: https://github.com/emilyst/stardrift/compare/v0.0.13...v0.0.14

[0.0.13]: https://github.com/emilyst/stardrift/compare/v0.0.12...v0.0.13

[0.0.12]: https://github.com/emilyst/stardrift/compare/v0.0.11...v0.0.12

[0.0.11]: https://github.com/emilyst/stardrift/compare/v0.0.10...v0.0.11

[0.0.10]: https://github.com/emilyst/stardrift/compare/v0.0.9...v0.0.10

[0.0.9]: https://github.com/emilyst/stardrift/compare/v0.0.8...v0.0.9

[0.0.8]: https://github.com/emilyst/stardrift/compare/v0.0.7...v0.0.8

[0.0.7]: https://github.com/emilyst/stardrift/compare/v0.0.6...v0.0.7

[0.0.6]: https://github.com/emilyst/stardrift/compare/v0.0.5...v0.0.6

[0.0.5]: https://github.com/emilyst/stardrift/compare/v0.0.4...v0.0.5

[0.0.4]: https://github.com/emilyst/stardrift/compare/v0.0.3...v0.0.4

[0.0.3]: https://github.com/emilyst/stardrift/compare/v0.0.2...v0.0.3

[0.0.2]: https://github.com/emilyst/stardrift/compare/v0.0.1...v0.0.2

[0.0.1]: https://github.com/emilyst/stardrift/releases/tag/v0.0.1