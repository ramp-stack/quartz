pub mod core;
pub mod actions;
pub mod conditions;
pub mod helpers;
pub mod physics;
pub mod events;
pub mod watch;
pub mod location;
pub mod physics_bridge;
pub mod lighting_bridge;

// Flatten the public surface: callers use `crate::canvas::Canvas` etc.
pub use core::{Canvas, CanvasMode, CanvasLayout};
// physics helper needed by object update path
pub(crate) use physics::rotation_adjusted_offset;