//! Button component modules
//!
//! Each module contains the component definition and ButtonWithLabel implementation
//! for a specific control button type.

pub mod barycenter;
pub mod diagnostics;
pub mod octree;
pub mod pause;
#[cfg(not(target_arch = "wasm32"))]
pub mod quit;
pub mod restart;
pub mod screenshot;
pub mod trails;

pub use barycenter::BarycenterGizmoToggleButton;
pub use diagnostics::DiagnosticsHudToggleButton;
pub use octree::OctreeToggleButton;
pub use pause::PauseButton;
#[cfg(not(target_arch = "wasm32"))]
pub use quit::QuitButton;
pub use restart::RestartSimulationButton;
pub use screenshot::ScreenshotButton;
pub use trails::TrailsToggleButton;
