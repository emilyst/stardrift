# Simulation Speed Control Implementation Plan

**Created**: 2025-07-21  
**Last Updated**: 2025-08-04  
**Note**: This document was generated automatically with AI assistance.

## Overview

This document outlines the implementation plan for adding simulation speed control to Stardrift, allowing users to speed
up or slow down the gravitational N-body simulation in real-time.

## Core Technology

Since Stardrift will implement custom symplectic integrators, we cannot rely on Avian3D's `PhysicsTime` trait. Instead,
we'll implement our own time scaling system that works with any integrator:

- Custom `SimulationTimeScale` resource for time multiplier
- Applied directly in physics integration systems
- Independent of physics engine choice
- Compatible with future symplectic integrators (Verlet, PEFRL, etc.)

## Implementation Components

### 1. Time Scale Resources

Create resources to manage simulation time scaling:

```rust
// src/resources/mod.rs
#[derive(Resource)]
pub struct SimulationTimeScale {
    pub scale: f64,  // 1.0 = normal, 2.0 = twice as fast, 0.5 = half speed
    pub min_scale: f64,
    pub max_scale: f64,
}

impl Default for SimulationTimeScale {
    fn default() -> Self {
        Self {
            scale: 1.0,
            min_scale: 0.01,
            max_scale: 10.0,
        }
    }
}

// For UI display
#[derive(Resource)]
pub struct SimulationSpeedPresets {
    pub presets: Vec<(f64, String)>, // (multiplier, label)
    pub selected_index: Option<usize>,
}

impl Default for SimulationSpeedPresets {
    fn default() -> Self {
        Self {
            presets: vec![
                (0.1, "0.1x".to_string()),
                (0.5, "0.5x".to_string()),
                (1.0, "1x".to_string()),
                (2.0, "2x".to_string()),
                (5.0, "5x".to_string()),
            ],
            selected_index: Some(2), // Default to 1x
        }
    }
}
```

### 2. Input System

Multiple input methods for maximum flexibility:

- **Number Keys**: 1-5 for preset speeds (0.1x, 0.5x, 1x, 2x, 5x)
- **Incremental**: +/- keys for fine-grained speed adjustment
- **Mouse Wheel**: Shift+scroll for smooth speed changes
- **Space Bar**: Continues to toggle pause/play (unchanged)

### 3. Command System Integration

Integrate with existing `SimulationCommand` event system:

```rust
// In src/events.rs - extend SimulationCommand
pub enum SimulationCommand {
    // ... existing commands ...
    SetSimulationSpeed(f64),
    IncreaseSpeed,
    DecreaseSpeed,
    ResetSpeed,
    SetSpeedPreset(usize), // Index into presets
}

// In src/plugins/simulation/actions.rs
pub fn handle_simulation_speed_commands(
    mut commands: EventReader<SimulationCommand>,
    mut time_scale: ResMut<SimulationTimeScale>,
    mut presets: ResMut<SimulationSpeedPresets>,
) {
    for command in commands.read() {
        match command {
            SimulationCommand::SetSimulationSpeed(speed) => {
                time_scale.scale = speed.clamp(time_scale.min_scale, time_scale.max_scale);
                // Update preset selection if matches
                presets.selected_index = presets.presets
                    .iter()
                    .position(|(s, _)| (*s - time_scale.scale).abs() < 0.01);
            }
            SimulationCommand::IncreaseSpeed => {
                time_scale.scale = (time_scale.scale * 1.25)
                    .min(time_scale.max_scale);
                presets.selected_index = None;
            }
            SimulationCommand::DecreaseSpeed => {
                time_scale.scale = (time_scale.scale / 1.25)
                    .max(time_scale.min_scale);
                presets.selected_index = None;
            }
            SimulationCommand::ResetSpeed => {
                time_scale.scale = 1.0;
                presets.selected_index = Some(2); // 1x preset
            }
            SimulationCommand::SetSpeedPreset(index) => {
                if let Some((speed, _)) = presets.presets.get(*index) {
                    time_scale.scale = *speed;
                    presets.selected_index = Some(*index);
                }
            }
        }
    }
}

// Keyboard input handler
pub fn handle_speed_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: EventWriter<SimulationCommand>,
    mut mouse_wheel: EventReader<MouseWheel>,
    modifier_keys: Res<ButtonInput<KeyCode>>,
) {
    // Number keys for presets
    if keys.just_pressed(KeyCode::Digit1) {
        commands.send(SimulationCommand::SetSpeedPreset(0));
    } else if keys.just_pressed(KeyCode::Digit2) {
        commands.send(SimulationCommand::SetSpeedPreset(1));
    } else if keys.just_pressed(KeyCode::Digit3) {
        commands.send(SimulationCommand::SetSpeedPreset(2));
    } else if keys.just_pressed(KeyCode::Digit4) {
        commands.send(SimulationCommand::SetSpeedPreset(3));
    } else if keys.just_pressed(KeyCode::Digit5) {
        commands.send(SimulationCommand::SetSpeedPreset(4));
    }

    // +/- for incremental
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::Plus) {
        commands.send(SimulationCommand::IncreaseSpeed);
    } else if keys.just_pressed(KeyCode::Minus) {
        commands.send(SimulationCommand::DecreaseSpeed);
    }

    // Shift+scroll for smooth adjustment
    if modifier_keys.pressed(KeyCode::ShiftLeft) || modifier_keys.pressed(KeyCode::ShiftRight) {
        for event in mouse_wheel.read() {
            if event.y > 0.0 {
                commands.send(SimulationCommand::IncreaseSpeed);
            } else if event.y < 0.0 {
                commands.send(SimulationCommand::DecreaseSpeed);
            }
        }
    }
}
```

### 4. Physics Integration

Apply time scaling in physics systems:

```rust
// In physics update systems
pub fn update_positions_with_time_scale(
    time: Res<Time>,
    time_scale: Res<SimulationTimeScale>,
    mut bodies: Query<(&mut Transform, &LinearVelocity), With<RigidBody>>,
) {
    let scaled_dt = time.delta_seconds_f64() * time_scale.scale;

    // Use scaled_dt for integration
    // This will work with any symplectic integrator
    for (mut transform, velocity) in &mut bodies {
        // Example with simple Euler (will be replaced with symplectic integrators)
        transform.translation += velocity.0.as_vec3() * scaled_dt as f32;
    }
}

// For future symplectic integrators
pub trait SymplecticIntegrator {
    fn step(
        &self,
        bodies: &mut Query<(&mut Transform, &mut LinearVelocity, &ComputedMass)>,
        forces: &Query<&ExternalForce>,
        dt: f64, // This dt will already include time scaling
    );
}
```

### 5. UI Display

Visual feedback integrated with existing UI:

```rust
// Add speed display to diagnostics HUD
// In src/plugins/diagnostics_hud.rs
fn spawn_diagnostics_hud(
    // ... existing parameters ...
    time_scale: Res<SimulationTimeScale>,
) {
    // Add speed display row to existing HUD
    commands.spawn((
        hud_row_node.clone(),
        children![
            (Text::new("Speed"), regular_text_font.clone()),
            (
                SpeedDisplayText,
                Text::new(format!("{:.1}x", time_scale.scale)),
                TextColor(speed_color(time_scale.scale)),
                extra_bold_text_font.clone(),
            ),
        ],
    ));
}

// Color coding for speed
fn speed_color(scale: f64) -> Color {
    match scale {
        s if (s - 1.0).abs() < 0.01 => Color::srgb(0.0, 1.0, 0.0), // Green for 1x
        s if s < 1.0 => Color::srgb(1.0, 1.0, 0.0), // Yellow for slow
        _ => Color::srgb(1.0, 0.5, 0.0), // Orange for fast
    }
}

// Add UI buttons for speed control
// In src/plugins/controls.rs
fn spawn_speed_control_buttons() {
    // Speed preset buttons
    for (index, (scale, label)) in presets.presets.iter().enumerate() {
        commands.spawn((
            Button,
            SpeedPresetButton(index),
            // ... button styling ...
            Text::new(label.clone()),
        ));
    }
}
```

### 6. Configuration Integration

Add to `config.toml`:

```toml
[simulation.speed_control]
# Default simulation speed multiplier
default_speed = 1.0

# Available preset speeds (for number keys)
preset_speeds = [0.1, 0.5, 1.0, 2.0, 5.0]

# Speed adjustment settings
increment_factor = 1.25  # For +/- keys
mouse_wheel_sensitivity = 0.1

# Speed limits
min_speed = 0.01
max_speed = 10.0

# Warning thresholds
accuracy_warning_threshold = 5.0  # Warn about accuracy at high speeds
```

Update `src/config.rs`:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct SimulationConfig {
    // ... existing fields ...
    #[serde(default)]
    pub speed_control: SpeedControlConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpeedControlConfig {
    pub default_speed: f64,
    pub preset_speeds: Vec<f64>,
    pub increment_factor: f64,
    pub mouse_wheel_sensitivity: f64,
    pub min_speed: f64,
    pub max_speed: f64,
    pub accuracy_warning_threshold: f64,
}

impl Default for SpeedControlConfig {
    fn default() -> Self {
        Self {
            default_speed: 1.0,
            preset_speeds: vec![0.1, 0.5, 1.0, 2.0, 5.0],
            increment_factor: 1.25,
            mouse_wheel_sensitivity: 0.1,
            min_speed: 0.01,
            max_speed: 10.0,
            accuracy_warning_threshold: 5.0,
        }
    }
}
```

## Implementation Phases

### Phase 1: Core Functionality (MVP)

- [ ] Add `SimulationTimeScale` and `SimulationSpeedPresets` resources
- [ ] Extend `SimulationCommand` enum with speed commands
- [ ] Implement command handlers in `handle_simulation_speed_commands`
- [ ] Add keyboard input handling (number keys 1-5)
- [ ] Apply time scaling in physics systems

### Phase 2: UI Integration

- [ ] Add speed display to diagnostics HUD
- [ ] Implement color coding for speed indicator
- [ ] Create speed control buttons in UI
- [ ] Add +/- incremental adjustment controls
- [ ] Implement mouse wheel support with Shift modifier

### Phase 3: Configuration & Polish

- [ ] Add `SpeedControlConfig` to configuration system
- [ ] Load settings from config.toml
- [ ] Trail system integration (adjust recording rate with speed)
- [ ] Performance warnings for high speeds
- [ ] Smooth speed transitions (optional lerping)

## Technical Considerations

### 1. Performance Impact

- Higher speeds reduce simulation accuracy
- May cause collision detection issues at extreme speeds
- Consider warning users when `speed > 5.0`
- Potentially limit max speed based on body count

### 2. Trail System Integration

Trail point recording should account for simulation speed:

```rust
// In trail update system
pub fn update_trail_recording_rate(
    time_scale: Res<SimulationTimeScale>,
    mut trail_settings: ResMut<TrailsVisualizationSettings>,
) {
    // Adjust recording rate based on time scale
    // Record more frequently at higher speeds to maintain trail smoothness
    trail_settings.effective_interval = trail_settings.base_interval / time_scale.scale;
}
```

### 3. Visual Feedback

- Clear speed indicator always visible
- Color coding for quick recognition
- Warning notifications for extreme speeds
- Consider speed "notches" for common values

### 4. Integration with Symplectic Integrators

Since we're implementing custom integrators:

```rust
// Each integrator will receive scaled dt
impl VerletIntegrator {
    fn step(&self, bodies: &mut Query<...>, dt: f64) {
        // dt already includes time scaling
        // Integrator doesn't need to know about SimulationTimeScale
    }
}
```

### 5. Pause System Integration

- Speed control independent of pause state
- Resuming from pause maintains previous speed
- Time scale only applied when `AppState::Running`

### 6. Determinism Considerations

- Document that changing speed mid-simulation affects determinism
- Speed changes don't affect physics accuracy, only time progression
- Consistent behavior across platforms since we control time scaling

## Future Enhancements

1. **Adaptive Speed**: Automatically adjust speed based on interesting events (collisions, close encounters)
2. **Speed Profiles**: Save custom speed configurations
3. **Hotkey Customization**: Allow users to bind their preferred keys
4. **Speed Ramping**: Gradual acceleration/deceleration for smooth transitions
5. **Time Display**: Show simulation time vs real time
6. **Reverse Time**: Negative speeds for rewinding simulation (requires state history)

## Testing Plan

1. **Unit Tests**
    - Time scale resource initialization
    - Speed clamping and validation
    - Command handling logic
    - Configuration loading

2. **Integration Tests**
    - Input handling for all control methods
    - Time scaling in physics systems
    - UI update synchronization
    - Trail recording rate adjustment

3. **Compatibility Tests**
    - Verify time scaling works with existing integrator
    - Test with future symplectic integrators
    - Ensure pause/resume behavior is preserved

4. **Performance Tests**
    - Frame rate at various speeds
    - Physics accuracy at different time scales
    - Memory usage with trail recording

5. **User Experience Tests**
    - Control responsiveness
    - Visual feedback clarity
    - Learning curve for new users

## Summary

This revised plan ensures compatibility with future symplectic integrators by implementing our own time scaling system
rather than relying on Avian3D's physics engine. The time scale is applied at the integration level, making it work
seamlessly with any integrator implementation.