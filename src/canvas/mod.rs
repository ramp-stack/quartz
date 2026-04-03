pub mod core;
pub mod actions;
pub mod physics;
pub mod events;
pub mod watch;
pub mod location;

// Flatten the public surface: callers use `crate::canvas::Canvas` etc.
pub use core::{Canvas, CanvasMode, CanvasLayout};
// physics helper needed by object update path
pub(crate) use physics::rotation_adjusted_offset;