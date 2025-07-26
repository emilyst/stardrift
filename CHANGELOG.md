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
- Added chrono dependency for timestamp generation

### Changed
- Updated UI system to include UIRoot marker component for programmatic visibility control
- Updated diagnostics HUD to include DiagnosticsHudRoot marker component
- Replaced BerkeleyMono fonts with Saira Semi-Condensed font family
- Redesigned UI button layout from bottom-right horizontal to top-left vertical arrangement
- Updated button styling with more subtle background colors and fixed width
- Enhanced diagnostics HUD text layout with right-aligned values and improved contrast
- Adjusted button dimensions and spacing for better visual hierarchy

### Fixed
- N/A

### Deprecated
- N/A

### Removed
- N/A

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