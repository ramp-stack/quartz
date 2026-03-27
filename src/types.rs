use crate::object::GameObject;
use crate::value::{Expr, MathOp, CompOp};
use crate::sound::SoundOptions;
use prism::canvas::Text;

#[derive(Debug, Clone)]
pub enum Target {
    ByName(String),
    ById(String),
    ByTag(String),
}

impl Target {
    pub fn name(s: impl Into<String>) -> Self { Target::ByName(s.into()) }
    pub fn id(s: impl Into<String>)   -> Self { Target::ById(s.into()) }
    pub fn tag(s: impl Into<String>)  -> Self { Target::ByTag(s.into()) }
}

#[derive(Debug, Clone, Copy)]
pub struct Anchor {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub enum Location {
    Position((f32, f32)),
    Between(Box<Target>, Box<Target>),
    AtTarget(Box<Target>),
    Relative {
        target: Box<Target>,
        offset: (f32, f32),
    },
    OnTarget {
        target: Box<Target>,
        anchor: Anchor,
        offset: (f32, f32),
    },
}

impl Location {
    pub fn at(x: f32, y: f32) -> Self {
        Location::Position((x, y))
    }
}

#[derive(Debug, Clone)]
pub enum Condition {
    Always,
    KeyHeld(prism::event::Key),
    KeyNotHeld(prism::event::Key),
    Collision(Target),
    NoCollision(Target),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
    IsVisible(Target),
    IsHidden(Target),
    Compare(Expr, CompOp, Expr),
    VarExists(String),
    Grounded(Target),
    Expr(String),
    HasTag(Target, String),
}

#[derive(Clone, Debug)]
pub enum Action {
    ApplyMomentum {
        target: Target,
        value:  (f32, f32),
    },
    SetMomentum {
        target: Target,
        value:  (f32, f32),
    },
    Spawn {
        object:   Box<GameObject>,
        location: Location,
    },
    SetResistance {
        target: Target,
        value:  (f32, f32),
    },
    Remove {
        target: Target,
    },
    TransferMomentum {
        from:  Target,
        to:    Target,
        scale: f32,
    },
    SetAnimation {
        target:          Target,
        animation_bytes: &'static [u8],
        fps:             f32,
    },
    Teleport {
        target:   Target,
        location: Location,
    },
    Show      { target: Target },
    Hide      { target: Target },
    Toggle    { target: Target },
    Conditional {
        condition: Condition,
        if_true:   Box<Action>,
        if_false:  Option<Box<Action>>,
    },
    Custom { name: String },
    SetVar {
        name:  String,
        value: Expr,
    },
    ModVar {
        name:    String,
        op:      MathOp,
        operand: Expr,
    },
    Multi(Vec<Action>),
    PlaySound {
        path:    String,
        options: SoundOptions,
    },
    SetGravity {
        target: Target,
        value:  f32,
    },
    SetSize {
        target: Target,
        value:  (f32, f32),
    },
    AddTag {
        target: Target,
        tag:    String,
    },
    RemoveTag {
        target: Target,
        tag:    String,
    },
    SetText {
        target: Target,
        text:   Text,
    },
    Expr(String),
    SetRotation {
        target: Target,
        value:  f32,
    },
    SetSlope {
        target:       Target,
        left_offset:  f32,
        right_offset: f32,
        auto_rotate:  bool,
    },
    AddRotation {
        target: Target,
        value:  f32,
    },
    ApplyRotation {
        target: Target,
        value:  f32,
    },
    SetSurfaceNormal {
        target: Target,
        nx:     f32,
        ny:     f32,
    },
}

impl Action {
    pub fn expr(s: impl Into<String>) -> Self { Action::Expr(s.into()) }

    pub fn when(cond: Condition, if_true: Action, if_false: Option<Action>) -> Self {
        Action::Conditional {
            condition: cond,
            if_true:   Box::new(if_true),
            if_false:  if_false.map(Box::new),
        }
    }

    pub fn multi(actions: Vec<Action>) -> Self { Action::Multi(actions) }

    pub fn set_var(name: impl Into<String>, value: Expr) -> Self {
        Action::SetVar { name: name.into(), value }
    }

    pub fn apply_momentum(target: Target, x: f32, y: f32) -> Self {
        Action::ApplyMomentum { target, value: (x, y) }
    }
}

impl Condition {
    pub fn expr(s: impl Into<String>) -> Self { Condition::Expr(s.into()) }

    pub fn and(a: Condition, b: Condition) -> Self {
        Condition::And(Box::new(a), Box::new(b))
    }

    pub fn or(a: Condition, b: Condition) -> Self {
        Condition::Or(Box::new(a), Box::new(b))
    }

    pub fn not(c: Condition) -> Self {
        Condition::Not(Box::new(c))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollAxis {
    Up,
    Down,
    Left,
    Right,
}

pub enum GameEvent {
    Collision        { action: Action, target: Target },
    BoundaryCollision{ action: Action, target: Target },
    KeyPress         { key: prism::event::Key, action: Action, target: Target },
    KeyRelease       { key: prism::event::Key, action: Action, target: Target },
    KeyHold          { key: prism::event::Key, action: Action, target: Target },
    Tick             { action: Action, target: Target },
    Custom           { name: String, target: Target },
    MousePress       { action: Action, target: Target, button: Option<MouseButton> },
    MouseRelease     { action: Action, target: Target, button: Option<MouseButton> },
    MouseEnter       { action: Action, target: Target },
    MouseLeave       { action: Action, target: Target },
    MouseOver        { action: Action, target: Target },
    MouseScroll      { action: Action, target: Target, axis: Option<ScrollAxis> },
    MouseMove        { action: Action, target: Target },
}

impl GameEvent {
    pub fn is_key_press(&self)    -> bool { matches!(self, GameEvent::KeyPress    { .. }) }
    pub fn is_key_release(&self)  -> bool { matches!(self, GameEvent::KeyRelease  { .. }) }
    pub fn is_key_hold(&self)     -> bool { matches!(self, GameEvent::KeyHold     { .. }) }
    pub fn is_tick(&self)         -> bool { matches!(self, GameEvent::Tick        { .. }) }
    pub fn is_custom(&self)       -> bool { matches!(self, GameEvent::Custom      { .. }) }
    pub fn is_mouse_press(&self)  -> bool { matches!(self, GameEvent::MousePress  { .. }) }
    pub fn is_mouse_release(&self)-> bool { matches!(self, GameEvent::MouseRelease{ .. }) }
    pub fn is_mouse_enter(&self)  -> bool { matches!(self, GameEvent::MouseEnter  { .. }) }
    pub fn is_mouse_leave(&self)  -> bool { matches!(self, GameEvent::MouseLeave  { .. }) }
    pub fn is_mouse_over(&self)   -> bool { matches!(self, GameEvent::MouseOver   { .. }) }
    pub fn is_mouse_scroll(&self) -> bool { matches!(self, GameEvent::MouseScroll { .. }) }
    pub fn is_mouse_move(&self)   -> bool { matches!(self, GameEvent::MouseMove   { .. }) }

    pub fn key(&self) -> Option<&prism::event::Key> {
        match self {
            GameEvent::KeyPress   { key, .. }
            | GameEvent::KeyRelease { key, .. }
            | GameEvent::KeyHold    { key, .. } => Some(key),
            _ => None,
        }
    }

    pub fn action(&self) -> &Action {
        match self {
            GameEvent::Collision          { action, .. }
            | GameEvent::BoundaryCollision{ action, .. }
            | GameEvent::KeyPress         { action, .. }
            | GameEvent::KeyRelease       { action, .. }
            | GameEvent::KeyHold          { action, .. }
            | GameEvent::Tick             { action, .. }
            | GameEvent::MousePress       { action, .. }
            | GameEvent::MouseRelease     { action, .. }
            | GameEvent::MouseEnter       { action, .. }
            | GameEvent::MouseLeave       { action, .. }
            | GameEvent::MouseOver        { action, .. }
            | GameEvent::MouseScroll      { action, .. }
            | GameEvent::MouseMove        { action, .. } => action,
            GameEvent::Custom { .. } => panic!("Custom events don't have actions"),
        }
    }

    pub fn custom_name(&self) -> Option<&str> {
        if let GameEvent::Custom { name, .. } = self { Some(name) } else { None }
    }
}

impl Clone for GameEvent {
    fn clone(&self) -> Self {
        match self {
            GameEvent::Collision { action, target } =>
                GameEvent::Collision { action: action.clone(), target: target.clone() },
            GameEvent::BoundaryCollision { action, target } =>
                GameEvent::BoundaryCollision { action: action.clone(), target: target.clone() },
            GameEvent::KeyPress { key, action, target } =>
                GameEvent::KeyPress { key: key.clone(), action: action.clone(), target: target.clone() },
            GameEvent::KeyRelease { key, action, target } =>
                GameEvent::KeyRelease { key: key.clone(), action: action.clone(), target: target.clone() },
            GameEvent::KeyHold { key, action, target } =>
                GameEvent::KeyHold { key: key.clone(), action: action.clone(), target: target.clone() },
            GameEvent::Tick { action, target } =>
                GameEvent::Tick { action: action.clone(), target: target.clone() },
            GameEvent::Custom { name, target } =>
                GameEvent::Custom { name: name.clone(), target: target.clone() },
            GameEvent::MousePress { action, target, button } =>
                GameEvent::MousePress { action: action.clone(), target: target.clone(), button: *button },
            GameEvent::MouseRelease { action, target, button } =>
                GameEvent::MouseRelease { action: action.clone(), target: target.clone(), button: *button },
            GameEvent::MouseEnter { action, target } =>
                GameEvent::MouseEnter { action: action.clone(), target: target.clone() },
            GameEvent::MouseLeave { action, target } =>
                GameEvent::MouseLeave { action: action.clone(), target: target.clone() },
            GameEvent::MouseOver { action, target } =>
                GameEvent::MouseOver { action: action.clone(), target: target.clone() },
            GameEvent::MouseScroll { action, target, axis } =>
                GameEvent::MouseScroll { action: action.clone(), target: target.clone(), axis: *axis },
            GameEvent::MouseMove { action, target } =>
                GameEvent::MouseMove { action: action.clone(), target: target.clone() },
        }
    }
}

impl std::fmt::Debug for GameEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameEvent::Collision { action, target } =>
                f.debug_struct("Collision").field("action", action).field("target", target).finish(),
            GameEvent::BoundaryCollision { action, target } =>
                f.debug_struct("BoundaryCollision").field("action", action).field("target", target).finish(),
            GameEvent::KeyPress { key, action, target } =>
                f.debug_struct("KeyPress").field("key", key).field("action", action).field("target", target).finish(),
            GameEvent::KeyRelease { key, action, target } =>
                f.debug_struct("KeyRelease").field("key", key).field("action", action).field("target", target).finish(),
            GameEvent::KeyHold { key, action, target } =>
                f.debug_struct("KeyHold").field("key", key).field("action", action).field("target", target).finish(),
            GameEvent::Tick { action, target } =>
                f.debug_struct("Tick").field("action", action).field("target", target).finish(),
            GameEvent::Custom { name, target } =>
                f.debug_struct("Custom").field("name", name).field("target", target).finish(),
            GameEvent::MousePress { action, target, button } =>
                f.debug_struct("MousePress").field("action", action).field("target", target).field("button", button).finish(),
            GameEvent::MouseRelease { action, target, button } =>
                f.debug_struct("MouseRelease").field("action", action).field("target", target).field("button", button).finish(),
            GameEvent::MouseEnter { action, target } =>
                f.debug_struct("MouseEnter").field("action", action).field("target", target).finish(),
            GameEvent::MouseLeave { action, target } =>
                f.debug_struct("MouseLeave").field("action", action).field("target", target).finish(),
            GameEvent::MouseOver { action, target } =>
                f.debug_struct("MouseOver").field("action", action).field("target", target).finish(),
            GameEvent::MouseScroll { action, target, axis } =>
                f.debug_struct("MouseScroll").field("action", action).field("target", target).field("axis", axis).finish(),
            GameEvent::MouseMove { action, target } =>
                f.debug_struct("MouseMove").field("action", action).field("target", target).finish(),
        }
    }
}