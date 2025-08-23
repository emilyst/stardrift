# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Quick Reference

### Daily Development Commands

```bash
cargo check              # Quick compilation check
cargo fmt                # Format all code
cargo test --workspace   # Run all tests including macros
cargo test               # Run main crate tests only
cargo test -p macros  # Run just macro tests
cargo clippy             # Lint check
cargo run                # Run the simulation
cargo bench              # Run benchmarks
```

### When to Document

| Change Type     | Devlog | CHANGELOG      | README |
|-----------------|--------|----------------|--------|
| New feature     | ✓      | ✓              | ✓      |
| Bug fix         | ✓      | ✓ (if visible) |        |
| Performance     | ✓      |                |        |
| Config option   | ✓      | ✓              | ✓      |
| Breaking change | ✓      | ✓              | ✓      |

### Quick Testing

```bash
cargo test physics::      # Test physics module
cargo test --lib         # Unit tests only
cargo test -- --nocapture # See test output
cargo test test_specific_function  # Run specific test

# Test procedural macros specifically
cargo test -p macros  # Run just macro tests
```

### Common File Locations

- Config: `config.toml`
- Devlogs: `docs/log/YYYY-MM-DD_NNN_*.md`
- Source code: `src/`
- Proc macros: `macros/`
- Benchmarks: `benches/`
- Integration tests: `tests/`
- Integrators: `src/physics/integrators/`

### Pre-Commit Workflow

1. `cargo fmt` - Format code
2. `cargo test --workspace` - Ensure all tests pass (including macros)
3. `cargo clippy` - Check linting
4. Create devlog if feature/major change
5. Update CHANGELOG.md if user-visible

## Documentation Requirements

### Documentation Formats

#### Devlog Format (REQUIRED for features/major changes)

```bash
# Create in: docs/log/YYYY-MM-DD_NNN_description.md
# NNN = 3-digit sequence number for the day

# Example: docs/log/2025-07-31_001_trail-visibility-toggle.md
```

Template:

```markdown
# Feature Name

**Date**: YYYY-MM-DD
**Feature**: Brief description
**Author**: Generated

## Context

[Why this change was needed]

## Implementation

[What was done, key decisions]

## Technical Details

[Code snippets, architecture notes]

## Testing

[How it was tested]

## Future Considerations

[Potential improvements or related work]
```

#### CHANGELOG.md Updates (REQUIRED for all user-visible changes)

**Writing Style**: Use factual, objective language. Avoid subjective terms like "improved", "enhanced", "better", "
refined". State what changed, not qualitative assessments.

```markdown
### Added

- Feature description with specific details
    - Sub-feature or implementation detail
    - Another detail

### Changed

- What changed (factual description)
    - Specific technical modifications
    - Measurable changes (e.g., "Changed font size from 10px to 12px")

### Fixed

- Bug description and observable behavior change
```

**Examples of Objective Writing**:

- ✓ "Changed button width from 128px to 160px"
- ✗ "Improved button styling"
- ✓ "Added platform-specific offset for macOS (32px)"
- ✗ "Enhanced cross-platform compatibility"
- ✓ "Increased font size from 10px to 12px"
- ✗ "Better readability with larger fonts"

## Essential Commands

### Running Specific Tests

```bash
# Run tests in specific modules
cargo test --lib config::tests
cargo test --lib physics::tests
cargo test --lib trails::tests

# Run a specific test function
cargo test --lib test_config_path_structure

# Run tests with output displayed (for debugging)
cargo test --lib -- --nocapture

# Run integration tests
cargo test --test '*'
```

### Benchmarking

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark groups
cargo bench integrators      # Integrator accuracy and performance
cargo bench construction     # Octree construction scaling
cargo bench physics          # Force calculation performance
cargo bench realworld        # Real-world scenarios
cargo bench configurations   # Profile-based testing
cargo bench characteristics  # Special performance patterns

# After running benchmarks, create devlog if performance changed significantly
```

#### Performance Targets

- **60fps goal**: <16.67ms with ~100 bodies
- **Scaling**: O(n log n) construction
- **Sweet spot**: 100-1000 bodies

## Architecture Guidelines

### Adding a New Integrator

To add a new numerical integrator to the simulation:

1. **Create the integrator implementation** (`src/physics/integrators/your_integrator.rs`)
   ```rust
   use super::{ForceEvaluator, Integrator};
   use crate::physics::math::{Scalar, Vector};
   
   #[derive(Debug, Clone, Default)]
   pub struct YourIntegrator;
   
   impl Integrator for YourIntegrator {
       fn step(
           &self,
           position: &mut Vector,
           velocity: &mut Vector,
           evaluator: &dyn ForceEvaluator,
           dt: Scalar,
       ) {
           // Implementation
       }
   }
   ```

2. **Export from module** (`src/physics/integrators/mod.rs`)
    - Add module declaration: `pub mod your_integrator;`
    - Add public export: `pub use your_integrator::YourIntegrator;`

3. **Register in the registry** (`src/physics/integrators/registry.rs`)
    - Import the integrator: Add to the `use super::{...}` statement
    - Add to `get()` method match statement
    - Add to `list_available()` method
    - Add any convenient aliases in `new()` method

4. **Update documentation**
    - **README.md**:
        - Add to the integrator list in features and configuration sections
        - Add comprehensive description in the "Integrator Selection Guide" section including:
            - Category (Symplectic or Explicit)
            - Order of accuracy
            - Pros and cons
            - Use cases
            - Algorithm description
            - Update the performance comparison table with force evaluations/step and characteristics
    - **CHANGELOG.md**: Add entry under "Added" in [Unreleased] section
    - **Create devlog**: `docs/log/YYYY-MM-DD_NNN_integrator_name.md`

5. **Add tests and benchmarks** (optional but recommended)
    - Unit tests in the integrator file
    - Integration tests in `tests/`
    - Add to `benches/integrators.rs`:
        - Include in imports at top of file
        - Add to `get_integrators()` function
        - Add to `get_integrators_with_order()` with expected convergence order
    - The benchmark suite automatically tests:
        - Performance (raw speed)
        - Accuracy (harmonic oscillator, Kepler orbits)
        - Convergence order verification
        - Energy conservation
        - Work-precision tradeoffs
        - N-body realistic scenarios

6. **Verify the integration**
   ```bash
   # List available integrators
   ./target/debug/stardrift --list-integrators
   
   # Test the new integrator
   # Edit config.toml: integrator.type = "your_integrator"
   cargo run
   
   # Run benchmarks if added
   cargo bench integrators
   ```

### Plugin System

All major features should be implemented as Bevy plugins:

```rust
pub struct FeaturePlugin;

impl Plugin for FeaturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FeatureSettings>()
            .add_systems(Update, feature_system);
    }
}
```

### Adding New Control Buttons

To add a new control button to the UI:

1. **Create button module** (`src/plugins/controls/buttons/your_button.rs`)
   ```rust
   use crate::plugins::controls::ButtonWithLabel;
   use crate::prelude::*;
   use bevy::prelude::*;
   
   #[derive(Component, Default)]
   pub struct YourButton;
   
   impl ButtonWithLabel for YourButton {
       fn command() -> SimulationCommand {
           SimulationCommand::YourAction
       }
       
       fn marker() -> Self { Self }
       
       fn base_text() -> &'static str {
           "Your Button"
       }
       
       fn shortcut() -> &'static str {
           "Y"
       }
   }
   ```

2. **Export from buttons module** (`buttons/mod.rs`)
    - Add module declaration: `pub mod your_button;`
    - Add re-export: `pub use your_button::YourButton;`

3. **Add to controls plugin** (`controls/mod.rs`)
    - Add button interaction handler in `build()`:
      ```rust
      button_interaction_handler::<YourButton>,
      ```
    - Add button spawn in `setup_controls_ui()`:
      ```rust
      parent.spawn_control_button::<YourButton>(font.clone());
      ```

4. **Define command** in `simulation/mod.rs` if needed

### Event-Driven UI

UI interactions use the command pattern:

```rust
// Define command in simulation/mod.rs
pub enum SimulationCommand {
    NewFeatureAction,
}

// Handle in simulation systems
fn handle_simulation_commands(
    mut commands: Commands,
    mut events: EventReader<SimulationCommand>,
) {
    for event in events.read() {
        match event {
            SimulationCommand::NewFeatureAction => {
                // Implementation
            }
        }
    }
}
```

### Code Style

- Use inline format strings: `format!("{value}")` not `format!("{}", value)`
- Follow Rust naming conventions
- Limit function scope to single responsibility
- Document public APIs

### Testing Requirements

- Unit tests for algorithms
- Integration tests for features
- Benchmark tests for performance-critical code
- Example: `cargo test test_trails_visibility_toggle`

### Automated Screenshot Testing

The `ScreenshotPlugin` provides automated screenshot capture for UI testing:

#### Quick Examples

```bash
# Single screenshot after 2 seconds
./target/debug/stardrift --screenshot-after 2 --screenshot-dir ./test_screenshots \
                         --screenshot-name test --screenshot-no-timestamp \
                         --screenshot-list-paths --exit-after-screenshots

# Multiple screenshots at intervals (frame-based for determinism)
./target/debug/stardrift --screenshot-interval 30 --screenshot-count 5 \
                         --screenshot-use-frames --screenshot-sequential \
                         --exit-after-screenshots

# Regression testing with fixed seed
./target/debug/stardrift --seed 42 --bodies 100 \
                         --screenshot-after 60 --screenshot-use-frames \
                         --screenshot-dir ./regression \
                         --screenshot-name baseline
```

#### Key Differences from Manual Screenshots

- **Manual (key 'S')**: Hides UI/HUD for clean captures
- **Automated**: Preserves UI visibility for validation testing

#### Integration with AI Testing

The `--screenshot-list-paths` flag outputs file paths to stdout:

```
SCREENSHOT_PATH: ./test_screenshots/ui_test.png
```

This enables easy integration with scripts:

```bash
OUTPUT=$(./target/debug/stardrift --screenshot-after 2 --screenshot-list-paths ...)
SCREENSHOT_PATH=$(echo "$OUTPUT" | grep "SCREENSHOT_PATH:" | cut -d' ' -f2)
```

### Integrator Configuration

Configure integrators in `config.toml`:

```toml
[physics]
integrator.type = "velocity_verlet"  # Options: symplectic_euler, velocity_verlet, heun, runge_kutta_second_order_midpoint, runge_kutta_fourth_order, pefrl
```

Available integrators:

- `symplectic_euler` - 1st order symplectic, good energy conservation
- `velocity_verlet` - 2nd order symplectic, excellent energy conservation
- `heun` - 2nd order predictor-corrector (improved Euler)
- `runge_kutta_second_order_midpoint` - 2nd order RK (midpoint method)
- `runge_kutta_fourth_order` - 4th order RK, high accuracy
- `pefrl` - 4th order symplectic, superior long-term energy conservation

Available aliases for convenience:

- `euler` → `symplectic_euler`
- `verlet` → `velocity_verlet`
- `rk4` → `runge_kutta_fourth_order`
- `rk2` → `runge_kutta_second_order_midpoint`
- `midpoint` → `runge_kutta_second_order_midpoint`
- `improved_euler` → `heun`
- `forest_ruth` → `pefrl`

To list available integrators:

```bash
./target/debug/stardrift --list-integrators
```

## Project Structure

This project uses a single-crate layout with procedural macros:

```
stardrift/
├── Cargo.toml       # Main project configuration
├── src/             # Source code
├── benches/         # Benchmarks
├── tests/           # Integration tests
├── assets/          # Static assets (fonts, icons)
├── macros/          # Proc macros for configuration
│   ├── Cargo.toml
│   └── src/
└── docs/            # Documentation and devlogs
```

The main codebase uses a plugin-based architecture with all major features
implemented as self-contained Bevy plugins under `src/plugins/`.

## Sub-Agent Usage

### When to Use Specialized Agents

Use specialized agents for complex tasks:

1. **architecture-guardian** - When adding new modules or reviewing design decisions
2. **project-documentation-maintainer** - For devlog and CHANGELOG updates
3. **bevy-ecs-architect** - For ECS patterns, system design, plugin architecture

### Example Agent Workflow

```
1. Consult architecture-guardian and bevy-ecs-architect for planning feature work
2. Implement feature
3. Use project-documentation-maintainer to create devlog
```

- Always use the dev profile for any checks or compilation. The release profile has optimizations that make it take a
  very long time.
- This project is a one-person side project made in my spare time for enjoyment. It's pre-release and not yet meant for
  general consumption. Backwards compatibility during changes is a low priority.