pub mod targeting;
pub mod collision;
pub mod effects;
pub mod input_types;
pub mod condition;
pub mod action;
pub mod event;
pub mod gravity;

pub use targeting::{Target, Location, Anchor};
pub use collision::{CollisionMode, CollisionShape, collision_layers};
pub use effects::{GlowConfig, HighlightEffect};
pub use input_types::{MouseButton, ScrollAxis};
pub use condition::{Condition, ConditionOps};
pub use action::Action;
pub use event::GameEvent;
pub use gravity::GravityFalloff;

/// Pins a screen-space object to a normalised anchor point on the viewport.
///
/// The engine recomputes the object's rendered position every frame so the
/// anchor point on the screen always aligns with the same anchor point on the
/// object's bounding box.
///
/// * `anchor` — normalised viewport position: `(0.0, 0.0)` = top-left,
///   `(1.0, 1.0)` = bottom-right, `(0.5, 0.5)` = center.
/// * `offset`  — pixel nudge applied after anchoring in virtual screen
///   coordinates. Positive X is right, positive Y is down.
///
/// Setting a `ScreenPin` automatically implies `ignore_zoom = true`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScreenPin {
    pub anchor: (f32, f32),
    pub offset: (f32, f32),
}