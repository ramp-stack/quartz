use super::action::Action;
use super::target::Target;

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
    Collision {
        action: Action,
        target: Target,
    },
    BoundaryCollision {
        action: Action,
        target: Target,
    },
    KeyPress {
        key: prism::event::Key,
        action: Action,
        target: Target,
    },
    KeyRelease {
        key: prism::event::Key,
        action: Action,
        target: Target,
    },
    KeyHold {
        key: prism::event::Key,
        action: Action,
        target: Target,
    },
    Tick {
        action: Action,
        target: Target,
    },
    Custom {
        name: String,
        target: Target,
    },
    MousePress {
        action: Action,
        target: Target,
        button: Option<MouseButton>,
    },
    MouseRelease {
        action: Action,
        target: Target,
        button: Option<MouseButton>,
    },
    MouseEnter {
        action: Action,
        target: Target,
    },
    MouseLeave {
        action: Action,
        target: Target,
    },
    MouseOver {
        action: Action,
        target: Target,
    },
    MouseScroll {
        action: Action,
        target: Target,
        axis: Option<ScrollAxis>,
    },
    MouseMove {
        action: Action,
        target: Target,
    },
}

impl GameEvent {
    pub fn is_key_press(&self) -> bool    { matches!(self, GameEvent::KeyPress    { .. }) }
    pub fn is_key_release(&self) -> bool  { matches!(self, GameEvent::KeyRelease  { .. }) }
    pub fn is_key_hold(&self) -> bool     { matches!(self, GameEvent::KeyHold     { .. }) }
    pub fn is_tick(&self) -> bool         { matches!(self, GameEvent::Tick        { .. }) }
    pub fn is_custom(&self) -> bool       { matches!(self, GameEvent::Custom      { .. }) }
    pub fn is_mouse_press(&self) -> bool  { matches!(self, GameEvent::MousePress  { .. }) }
    pub fn is_mouse_release(&self) -> bool{ matches!(self, GameEvent::MouseRelease{ .. }) }
    pub fn is_mouse_enter(&self) -> bool  { matches!(self, GameEvent::MouseEnter  { .. }) }
    pub fn is_mouse_leave(&self) -> bool  { matches!(self, GameEvent::MouseLeave  { .. }) }
    pub fn is_mouse_over(&self) -> bool   { matches!(self, GameEvent::MouseOver   { .. }) }
    pub fn is_mouse_scroll(&self) -> bool { matches!(self, GameEvent::MouseScroll { .. }) }
    pub fn is_mouse_move(&self) -> bool   { matches!(self, GameEvent::MouseMove   { .. }) }

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