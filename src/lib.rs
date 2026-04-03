pub mod value;
pub mod entropy;
pub mod lerp;
pub mod object;
pub mod types;
pub mod sprite;
pub mod text;
pub mod sound;
pub mod scene;
pub mod camera;
pub mod store;
pub mod input;
pub mod canvas;
pub(crate) mod file_watcher;
pub mod expr;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use prism::Context;
pub use prism::canvas::{ShapeType, Image, Text, Span, Align, Font, Color};
pub use prism::event::{Key, NamedKey};

pub use types::{
    Action, Condition, GameEvent,
    Target, Location, Anchor,
    CollisionMode, CollisionShape,
    GlowConfig, HighlightEffect,
    MouseButton, ScrollAxis,
    ConditionOps,
};

pub use canvas::{Canvas, CanvasMode, CanvasLayout};

pub use object::{GameObject, GameObjectBuilder};

pub use sprite::{
    AnimatedSprite, RotationOptions, RotationDirection,
    load_image, load_image_sized, load_animation,
    solid_circle, planet_image,
    planet_grayscale, with_tint,
    planet_atmosphere, glow_ring, tint_overlay,
    flip_horizontal, flip_vertical,
    rotate_cw, rotate_ccw, rotate_180,
};

pub use scene::{Scene, SceneManager};
pub use camera::Camera;
pub use store::ObjectStore;
pub use input::{
    InputState, Callback, MouseState, MouseCallback,
    MouseMoveCallback, MouseScrollCallback, CallbackStore, EventCallback,
};

pub use sound::{SoundOptions, SoundHandle};
pub use expr::{parse_condition, parse_action};
pub use entropy::Entropy;
pub use text::{TextSpec, SpanSpec, make_text, make_text_aligned, make_text_multi};
pub use lerp::Lerp;
pub use file_watcher::{Shared, SourceSettings, FromSource};

pub use value::{
    Expr, Value, MathOp, CompOp,
    resolve_expr, apply_op, compare_operands,
};