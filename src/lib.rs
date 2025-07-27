//! Stardrift library
//!
//! This provides the core functionality of stardrift as a library
//! to enable integration testing.

pub mod components;
pub mod config;
pub mod events;
pub mod physics;
pub mod plugins;
pub mod prelude;
pub mod resources;
pub mod states;
pub mod utils;

// Test utilities are public for integration tests
pub mod test_utils;
