# config-derive

Internal proc-macro crate for the Stardrift project. Provides the `ConfigDefaults` derive macro for generating `Default`
implementations with inline default values.

## Purpose

This crate exists solely to support configuration structs in the Stardrift n-body simulation. It allows specifying
default values directly in the struct definition using `#[default(...)]` attributes, making configuration defaults more
visible and maintainable.

## Usage

```rust
use config_derive::ConfigDefaults;
use serde::{Serialize, Deserialize};

#[derive(ConfigDefaults, Serialize, Deserialize)]
#[serde(default)]
pub struct PhysicsConfig {
    #[default(0.001)]
    pub gravitational_constant: f64,

    #[default(100)]  // Type inference handles usize
    pub body_count: usize,

    #[default(None)]
    pub optional_field: Option<u64>,

    #[default(vec![1, 2, 3])]
    pub initial_values: Vec<i32>,

    #[default("physics")]  // Automatically converts to String
    pub engine_name: String,
}
```

## Features

- **Inline defaults**: Default values are specified directly with each field
- **Expression support**: Any valid Rust expression can be used as a default
- **Smart type handling**:
    - String fields automatically convert from `&str` literals using `.into()`
    - All other types rely on Rust's type inference - no manual type annotations needed!
- **Clear error messages**: Compile-time errors with helpful diagnostics
- **Type safety**: Full Rust type checking for default values
- **Generic support**: Works with generic structs

## How It Works

The macro uses a simple approach that leverages Rust's type system:

1. **String fields**: Wraps the default value with `Into::into()` to enable `&str` → `String` conversion
2. **All other types**: Uses the default value as-is, relying on Rust's type inference

This means you can write simple, clean defaults without any type annotations:

```rust
#[derive(ConfigDefaults)]
struct Config {
    #[default(42)]        // Type inference handles u32
    pub timeout: u32,

    #[default(1000)]      // Type inference handles usize
    pub buffer_size: usize,

    #[default("hello")]   // Converts to String via Into
    pub greeting: String,

    #[default(true)]      // No conversion needed
    pub enabled: bool,
}
```

## Requirements

- Every field must have a `#[default(...)]` attribute
- Only supports structs with named fields
- The expression inside `#[default(...)]` must be valid for the field's type

## Error Handling

The macro provides clear compile-time errors for common mistakes:

- Missing `#[default(...)]` attributes
- Empty `#[default()]` attributes
- Attempting to derive on enums, unions, or tuple structs
- Invalid expressions in default attributes

## Implementation Notes

This is an internal-use-only crate (`publish = false`) designed specifically for the Stardrift project's configuration
needs. It prioritizes:

1. **Simplicity**: Single-purpose macro for configuration defaults
2. **Ergonomics**: Clean syntax without type annotations
3. **Safety**: Compile-time validation with helpful error messages
4. **Maintainability**: Clean separation from main project code

## Testing

The crate includes:

- Unit tests for various use cases
- UI tests (compile-fail tests) to verify error messages
- Integration with serde for configuration serialization

Run tests with:

```bash
cd config-derive
cargo test
```