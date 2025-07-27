# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/emilyst/stardrift/compare/v0.0.1...HEAD

[0.0.1]: https://github.com/emilyst/stardrift/releases/tag/v0.0.1