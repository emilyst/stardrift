# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Quick Reference

### Daily Development Commands

```bash
cargo check              # Quick compilation check
cargo fmt                # Format code
cargo test               # Run all tests
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
| Config option   | ✓      | ✓              |        |
| Breaking change | ✓      | ✓              | ✓      |

### Quick Testing

```bash
cargo test physics::      # Test physics module
cargo test --lib         # Unit tests only
cargo test -- --nocapture # See test output
cargo test test_specific_function  # Run specific test
```

### Common File Locations

- Config: `config.toml`
- Devlogs: `docs/log/YYYY-MM-DD_NNN_*.md`
- Benchmarks: `benches/`
- Integration tests: `tests/`

### Pre-Commit Workflow

1. `cargo fmt` - Format code
2. `cargo test` - Ensure tests pass
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

```markdown
### Added

- Feature description with specific details
    - Sub-feature or implementation detail
    - Another detail

### Changed

- What changed and why

### Fixed

- Bug description and impact
```

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
cargo bench construction       # Octree construction scaling
cargo bench physics           # Force calculation performance
cargo bench realworld         # Real-world scenarios
cargo bench configurations    # Profile-based testing
cargo bench characteristics   # Special performance patterns

# After running benchmarks, create devlog if performance changed significantly
```

#### Benchmark Configuration Profiles

Located in `configs/benchmark_profiles/`:

- `fast_inaccurate.toml` - Maximum FPS, theta=1.5
- `balanced.toml` - Standard settings, theta=0.5
- `high_accuracy.toml` - Scientific precision, theta=0.1
- `stress_test.toml` - Many bodies for testing

#### Performance Targets

- **60fps goal**: <16.67ms with ~100 bodies
- **Scaling**: O(n log n) construction
- **Sweet spot**: 100-1000 bodies

## Architecture Guidelines

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

## Project Structure

See README.md for the detailed project structure. The codebase uses a plugin-based architecture with all major features implemented as self-contained Bevy plugins under `src/plugins/`.

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