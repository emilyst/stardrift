# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.62] - 2025-10-09

### Changed

- Lower default body count to 25
  - It's more friendly for WebAssembly
- Use `ColorScheme::Rainbow` as default color scheme

## [0.0.61] - 2025-10-08

### Fixed

- Bloom effect now applies uniformly across all celestial body colors
  - Removed `metallic: 0.0` and `reflectance: 0.0` material properties that were interfering with bloom extraction
  - Bodies now use default PBR reflectance (0.5) which ensures consistent bloom visibility across all color ranges

## [0.0.60] - 2025-10-06

### Fixed

- iOS Safari crashing on load
  - Added polyfill for `exitPointerLock`

## [0.0.59] - 2025-10-05

### Changed

- Web build infrastructure migrated to Tailwind CSS
  - Replaced 73 lines of inline CSS with Tailwind CSS v4 utility classes
  - Moved web assets from project root to `web/` directory
  - Updated `Trunk.toml` build target from `index.html` to `web/index.html`
  - Changed WASM `opt-level` from `"s"` (size optimization) to `"3"` (speed optimization)
  - Added Tailwind CSS v4.1.14 to Trunk tools configuration
  - Loading progress bar width changed from fixed 300px to 80% viewport width
  - Loading percentage text uses `mix-blend-difference` for contrast instead of text stroke
  - Expanded Trunk watch ignore list to include `target`, `docs`, `benches` directories

## [0.0.58] - 2025-10-05

### Added

- Added WebAssembly loading progress indicator for web builds
  - Displays minimal progress bar with percentage during WASM download
  - Uses Trunk's initializer framework to track loading progress
  - Automatically fades out when application starts

## [0.0.57] - 2025-10-05

## Changed

- Upgraded to Bevy 0.17

## [0.0.56] - 2025-09-30

## [0.0.55] - 2025-09-30

### Changed

- Extracted command line parsing and handling into dedicated `cli` module
  - Moved `Args` struct and CLI functions from `main.rs` to `src/cli.rs`
  - Added `CliError` enum for proper error handling with `Result` types
  - Integrator validation now happens at CLI parse time with helpful error messages
- Replaced `unsafe` environment variable setting with Bevy's `LogPlugin` configuration API
  - Eliminates potential data race issues with `std::env::set_var`
  - `--verbose` flag now correctly controls log levels for both stardrift and bevy

### Fixed

- Fixed `--verbose` flag being ignored due to hardcoded log filter
  - Without `--verbose`: logs at INFO level for both stardrift and bevy
  - With `--verbose`: logs at DEBUG level for both stardrift and bevy

## [0.0.54] - 2025-09-28

### Changed

- Replaced ConfigDefaults procedural macro with manual Default implementations
    - Configuration structs now use conventional Rust Default trait implementations
    - Default values documented with inline comments in implementation blocks
- Dependencies updated

### Removed

- Removed entire macros crate and procedural macro system
    - Deleted macros crate from workspace dependencies
    - Eliminated build-time procedural macro compilation overhead
    - Simplified project structure by removing macros directory

### Fixed

- Fixed integrator benchmarks by copying acceleration functions locally after test utilities removal

## [0.0.53] - 2025-09-20

### Changed

- Dependencies updated.

### Added

- Initial attempt at a new kinetic energy simulation diagnostic
    - I'll check the math closer later

### Removed

- All tests

## [0.0.52] - 2025-09-09

## [0.0.51] - 2025-09-09

### Changed

- Temporary new defaults for slower simulations.
    - Lesbian pride flag colors.

## [0.0.50] - 2025-09-02

## [0.0.49] - 2025-09-01

### Removed

- Removed octree visualization depth control
    - Removed keyboard shortcuts (0-9) for setting octree visualization depth
    - Octree visualization now always shows all levels when enabled
    - Simplified visualization system by removing unnecessary complexity

### Fixed

- **CRITICAL**: Fixed gravitational force calculation missing inverse-square law
    - Force calculation was missing division by r² in `calculate_force_from_point`
    - All gravitational interactions were calculating F = G·m₁·m₂ instead of F = G·m₁·m₂/r²
    - Bodies at any distance experienced the same force magnitude (only direction varied)
    - Adjusted default gravitational constant from 0.001 to 1000.0 to compensate for the now-correct 1/r² scaling
    - The previous low G value (0.001) only worked because forces weren't decaying with distance
    - Added comprehensive physics verification tests to prevent regression
    - Error likely introduced when custom physics replaced Avian3D (early August 2025)

- Fixed automated screenshots occasionally failing to save when application exits immediately
    - Added sufficient frame delay after final screenshot to ensure asynchronous save completes
    - Resolves intermittent issue where window focus changes affected screenshot capture

## [0.0.48] - 2025-08-30

### Added

- Comprehensive set of pride flag color schemes for bodies
    - `bisexual` - Pink-purple-blue with smooth gradient interpolation (2:1:2 proportions)
    - `transgender` - Light blue-pink-white stripes (Monica Helms design)
    - `lesbian` - Orange-to-pink gradient (2018 sunset flag)
    - `pansexual` - Pink-yellow-blue representing attraction regardless of gender
    - `nonbinary` - Yellow-white-purple-black (Kye Rowan design)
    - `asexual` - Black-gray-white-purple for the ace spectrum
    - `genderfluid` - Pink-white-purple-black-blue for gender fluidity
    - `aromantic` - Green-white-gray-black for the aromantic spectrum
    - `agender` - Black-gray-white-green in symmetric pattern
    - All pride schemes use gradient interpolation for smooth color transitions
    - Black stripes rendered as dark gray for visibility against space background

## [0.0.47] - 2025-08-30

### Changed

- Changed luminance formula calcs to use simple average for trails and bloom and
  ITU-R BT.709 for bodies.

## [0.0.46] - 2025-08-30

### Added

- Command-line option `--color-scheme` to override the color scheme for bodies
    - Accepts any of the 14 available color schemes in snake_case format
    - Provides helpful error message listing all valid options for invalid input
    - Overrides the configuration file setting when specified

## [0.0.45] - 2025-08-30

### Added

- Expanded color scheme system with 12 new color generation modes
    - **Colorblind-safe palettes**: `deuteranopia_safe`, `protanopia_safe`, `tritanopia_safe`, `high_contrast`
    - **Scientific colormaps**: `viridis`, `plasma`, `inferno`, `turbo` (perceptually uniform gradients)
    - **Aesthetic themes**: `pastel`, `neon`, `monochrome`, `vaporwave`
    - All schemes follow existing spawn-time color assignment architecture
    - Color selection maintains deterministic behavior with seeded RNG

## [0.0.44] - 2025-08-30

## [0.0.43] - 2025-08-30

### Added

- Configurable color schemes for bodies
    - New `color_scheme` configuration option in `[rendering]` section
    - `"black_body"` (default): Physics-based black body radiation colors
    - `"rainbow"`: Random vibrant colors using full HSL spectrum
    - Color generation separated from material creation

## [0.0.42] - 2025-08-28

### Added

- Explicit Euler integrator for educational and comparison purposes
    - Provides classic forward Euler integration method
    - Includes comprehensive warnings about energy drift characteristics
    - Available as `explicit_euler` with alias `forward_euler`
    - Documented limitations for orbital mechanics simulations

- Self-describing integrator capabilities for plugin architecture
    - Added `clone_box()` method enabling prototype pattern in registry
    - Added `convergence_order()` method for mathematical order declaration
    - Added `name()` and `aliases()` methods for self-identification
    - Registry now uses prototype pattern with no hardcoded type knowledge

### Changed

- Renamed `AccelerationFunction` trait to `AccelerationField`
    - Better reflects physics field concept (acceleration fields evaluated at positions)
    - Renamed method `evaluate()` to `at()` following physics convention
    - Updated all implementations and documentation to use new naming

- Integrator registry refactored to prototype pattern
    - Registry stores prototype instances that clone themselves
    - Integrators are fully self-describing with metadata
    - Benchmark suite now uses automatic discovery via registry
    - No registry modifications needed when adding new integrators

- Enhanced documentation for all numerical integrators
    - Standardized documentation structure across all 7 integrator modules
    - Added comprehensive technical details including algorithm descriptions, mathematical properties, and energy
      behavior
    - Expanded comparison tables showing trade-offs between different methods
    - Added historical context and implementation notes where relevant
    - Improved test coverage with energy conservation and convergence order verification tests
    - Fixed compilation warnings from unused variables in test code

## [0.0.41] - 2025-08-26

### Fixed

- Restored `Without<PhysicsBody>` filter in `sync_transform_from_position` system to prevent query conflicts
    - Fixes panic caused by overlapping `Transform` component access between camera and physics body queries
    - Restores system stability after previous filter removal in commit 5fad58a

## [0.0.40] - 2025-08-25

### Changed

- Website address: stardrift.run

## [0.0.39] - 2025-08-23

### Changed

- Flattened project structure from workspace to single crate
    - Moved main application from `crates/stardrift/` to project root
    - Relocated `stardrift-macros` from `crates/stardrift-macros/` to package to `macros`
    - Eliminated workspace configuration for simpler project layout
    - Updated all documentation and CI/CD workflows for new structure
    - Moved `crates/stardrift/assets/fonts/` to `assets/fonts/`
    - Updated `include_bytes!` paths in `embedded_assets.rs` to reference workspace root location
    - Consolidated all assets (fonts and icons) at workspace level for centralized management

## [0.0.38] - 2025-08-23

### Fixed

- Button fonts not appearing in WebAssembly build

## [0.0.37] - 2025-08-23

### Added

- Automated screenshot capture system for UI testing and validation
    - CLI arguments for time-based (`--screenshot-after N`) and frame-based (`--screenshot-use-frames`) capture delays
    - Multiple screenshot capture with intervals (`--screenshot-interval N --screenshot-count N`)
    - Configurable output directory (`--screenshot-dir PATH`) with automatic creation
    - Flexible naming options: timestamps, sequential numbering (`--screenshot-sequential`), or static names (
      `--screenshot-no-timestamp`)
    - Machine-readable path output to stdout (`--screenshot-list-paths`)
    - Auto-exit after capture completion (`--exit-after-screenshots`)
    - UI elements remain visible in automated screenshots for validation purposes

### Changed

- Modularized controls plugin from monolithic 686-line file into organized module structure
    - Created `constants.rs` module for shared styling constants (button dimensions, colors)
    - Extracted `builder.rs` module with `ControlsCommandsExt` trait and `ButtonWithLabel` trait
    - Separated each button component into individual modules under `buttons/` directory
    - Each button module contains component definition, trait implementation, and text update logic
    - Main `mod.rs` retained core plugin structure, input handlers, and UI setup (399 lines)
    - Module structure enables easier maintenance and addition of new control buttons

## [0.0.36] - 2025-08-17

### Changed

- Improved release notes with GitHub attestation verification instructions
    - Added instructions for verifying build provenance using `gh attestation verify`
    - Removed redundant binary listings from release notes to avoid duplicating GitHub's automatic asset section
    - Provides cryptographic verification of artifacts built by GitHub Actions

- Adjusted UI styling and layout
    - Changed font sizes from 10px to 12px in diagnostics HUD
    - Changed button width from 128px to 160px and border radius from 5px to 4px
    - Added platform-specific top offset for macOS (32px) vs other platforms (4px)
    - Centered diagnostics HUD horizontally within viewport
    - Added min-width constraints (100px) to HUD text elements with left/right justification

- Updated dependencies

## [0.0.35] - 2025-08-17

### Fixed

- Trail decay timing after pause/unpause cycles
    - Trail points no longer generate excessively after unpausing the simulation
    - Trail point decay now progresses at correct simulation time rate rather than wall clock time
    - Ensures consistent trail behavior regardless of pause duration or frequency

## [0.0.34] - 2025-08-13

### Added

- Added (back) diagnostic logging plugin for performance monitoring
    - Integrated `LogDiagnosticsPlugin` that outputs performance metrics when debug logging is enabled
    - Provides visibility into frame times and other performance diagnostics

### Fixed

- Trail visibility toggle now works correctly when simulation is restarted
    - Trails respect the visibility settings when restarting the simulation
    - Fixed issue where trails would still render even when visibility was disabled

### Changed

- Simplified integrator configuration
    - Removed unused `IntegratorParams` structure from integrator creation
    - Integrators no longer require parameters for initialization
    - Cleaner API for creating integrator instances

- Internal code improvements
    - Updated to use Rust let-chains for cleaner conditional logic
    - Simplified string formatting to use inline variables throughout the codebase

## [0.0.33] - 2025-08-11

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
    - **PEFRL (Position-Extended Forest-Ruth-Like)**: 4th order symplectic integrator optimized for long-term energy
      conservation
    - All integrators now support multiple convenient aliases (e.g., `"rk4"`, `"improved_euler"`, `"forest_ruth"`)
    - All new integrators are included in comprehensive benchmark suite

- Acceleration field architecture for accurate multi-stage integration
    - Added `AccelerationField` trait allowing integrators to query accelerations at arbitrary positions
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
    - Implements `Integrator` trait with acceleration field support
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
