use prism::event::{OnEvent, Event, TickEvent};
use prism::drawable::{Drawable, Component, SizedTree};
use prism::Context;
use prism::layout::{Area, SizeRequest};
use prism::canvas::{Image, ShapeType};
use prism::display::Opt;

use std::cell::Cell;

use crate::animation::AnimatedSprite;
use crate::value::{Value, ComparisonOperator, MathOperator};

#[derive(Debug, Clone)]
pub enum Target {
    ByName(String),
    ById(String),
    ByTag(String),
}

impl Target {
    pub fn name(s: impl Into<String>) -> Self {
        Target::ByName(s.into())
    }

    pub fn id(s: impl Into<String>) -> Self {
        Target::ById(s.into())
    }

    pub fn tag(s: impl Into<String>) -> Self {
        Target::ByTag(s.into())
    }
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

#[derive(Debug, Clone, Copy)]
pub struct Anchor {
    pub x: f32,
    pub y: f32,
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
    //===========================================
    //synful additions
    Compare(Value, ComparisonOperator, Value),
    VarExists(String),
    Grounded(Target),
}

#[derive(Clone, Debug)]
pub enum Action {
    ApplyMomentum {
        target: Target,
        value: (f32, f32),
    },
    SetMomentum {
        target: Target,
        value: (f32, f32),
    },
    Spawn {
        object: Box<GameObject>,
        location: Location,
    },
    SetResistance {
        target: Target,
        value: (f32, f32),
    },
    Remove {
        target: Target,
    },
    TransferMomentum {
        from: Target,
        to: Target,
        scale: f32,
    },
    SetAnimation {
        target: Target,
        animation_bytes: &'static [u8],
        fps: f32,
    },
    Teleport {
        target: Target,
        location: Location,
    },
    Show {
        target: Target,
    },
    Hide {
        target: Target,
    },
    Toggle {
        target: Target,
    },
    Conditional {
        condition: Condition,
        if_true: Box<Action>,
        if_false: Option<Box<Action>>,
    },
    Custom {
        name: String,
    },
    //==========================================
    //synful additions
    Multi(Vec<Action>),
    SetVar {
        name: String,
        value: Value,
    },
    ModVar {
        name: String,
        op: MathOperator,
        value: Value,
    },
    PlaySound {
        path: String,        
    },
    SetGravity {
        target: Target,
        value: f32,
    },
    SetSize {
        target: Target,
        value: (f32, f32),
    },
}

/// Mouse button identifier — extended if prism adds more buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollAxis {
    Up,
    Down,
    Left,
    Right,
}

impl GameEvent {
    pub fn is_key_press(&self) -> bool {
        matches!(self, GameEvent::KeyPress { .. })
    }

    pub fn is_key_release(&self) -> bool {
        matches!(self, GameEvent::KeyRelease { .. })
    }

    pub fn is_key_hold(&self) -> bool {
        matches!(self, GameEvent::KeyHold { .. })
    }

    pub fn is_tick(&self) -> bool {
        matches!(self, GameEvent::Tick { .. })
    }

    pub fn is_custom(&self) -> bool {
        matches!(self, GameEvent::Custom { .. })
    }

    pub fn is_mouse_press(&self) -> bool {
        matches!(self, GameEvent::MousePress { .. })
    }

    pub fn is_mouse_release(&self) -> bool {
        matches!(self, GameEvent::MouseRelease { .. })
    }

    pub fn is_mouse_enter(&self) -> bool {
        matches!(self, GameEvent::MouseEnter { .. })
    }

    pub fn is_mouse_leave(&self) -> bool {
        matches!(self, GameEvent::MouseLeave { .. })
    }

    pub fn is_mouse_over(&self) -> bool {
        matches!(self, GameEvent::MouseOver { .. })
    }

    pub fn is_mouse_scroll(&self) -> bool {
        matches!(self, GameEvent::MouseScroll { .. })
    }

    pub fn is_mouse_move(&self) -> bool {
        matches!(self, GameEvent::MouseMove { .. })
    }

    pub fn key(&self) -> Option<&prism::event::Key> {
        match self {
            GameEvent::KeyPress { key, .. }
            | GameEvent::KeyRelease { key, .. }
            | GameEvent::KeyHold { key, .. } => Some(key),
            _ => None,
        }
    }

    pub fn action(&self) -> &Action {
        match self {
            GameEvent::Collision { action, .. }
            | GameEvent::BoundaryCollision { action, .. }
            | GameEvent::KeyPress { action, .. }
            | GameEvent::KeyRelease { action, .. }
            | GameEvent::KeyHold { action, .. }
            | GameEvent::Tick { action, .. }
            | GameEvent::MousePress { action, .. }
            | GameEvent::MouseRelease { action, .. }
            | GameEvent::MouseEnter { action, .. }
            | GameEvent::MouseLeave { action, .. }
            | GameEvent::MouseOver { action, .. }
            | GameEvent::MouseScroll { action, .. }
            | GameEvent::MouseMove { action, .. } => action,
            GameEvent::Custom { .. } => panic!("Custom events don't have actions"),
        }
    }

    pub fn custom_name(&self) -> Option<&str> {
        if let GameEvent::Custom { name, .. } = self {
            Some(name)
        } else {
            None
        }
    }
}

impl Clone for GameEvent {
    fn clone(&self) -> Self {
        match self {
            GameEvent::Collision { action, target } => GameEvent::Collision {
                action: action.clone(),
                target: target.clone(),
            },
            GameEvent::BoundaryCollision { action, target } => GameEvent::BoundaryCollision {
                action: action.clone(),
                target: target.clone(),
            },
            GameEvent::KeyPress { key, action, target } => GameEvent::KeyPress {
                key: key.clone(),
                action: action.clone(),
                target: target.clone(),
            },
            GameEvent::KeyRelease { key, action, target } => GameEvent::KeyRelease {
                key: key.clone(),
                action: action.clone(),
                target: target.clone(),
            },
            GameEvent::KeyHold { key, action, target } => GameEvent::KeyHold {
                key: key.clone(),
                action: action.clone(),
                target: target.clone(),
            },
            GameEvent::Tick { action, target } => GameEvent::Tick {
                action: action.clone(),
                target: target.clone(),
            },
            GameEvent::Custom { name, target } => GameEvent::Custom {
                name: name.clone(),
                target: target.clone(),
            },
            GameEvent::MousePress { action, target, button } => GameEvent::MousePress {
                action: action.clone(),
                target: target.clone(),
                button: *button,
            },
            GameEvent::MouseRelease { action, target, button } => GameEvent::MouseRelease {
                action: action.clone(),
                target: target.clone(),
                button: *button,
            },
            GameEvent::MouseEnter { action, target } => GameEvent::MouseEnter {
                action: action.clone(),
                target: target.clone(),
            },
            GameEvent::MouseLeave { action, target } => GameEvent::MouseLeave {
                action: action.clone(),
                target: target.clone(),
            },
            GameEvent::MouseOver { action, target } => GameEvent::MouseOver {
                action: action.clone(),
                target: target.clone(),
            },
            GameEvent::MouseScroll { action, target, axis } => GameEvent::MouseScroll {
                action: action.clone(),
                target: target.clone(),
                axis: *axis,
            },
            GameEvent::MouseMove { action, target } => GameEvent::MouseMove {
                action: action.clone(),
                target: target.clone(),
            },
        }
    }
}

impl std::fmt::Debug for GameEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameEvent::Collision { action, target } => f
                .debug_struct("Collision")
                .field("action", action)
                .field("target", target)
                .finish(),
            GameEvent::BoundaryCollision { action, target } => f
                .debug_struct("BoundaryCollision")
                .field("action", action)
                .field("target", target)
                .finish(),
            GameEvent::KeyPress { key, action, target } => f
                .debug_struct("KeyPress")
                .field("key", key)
                .field("action", action)
                .field("target", target)
                .finish(),
            GameEvent::KeyRelease { key, action, target } => f
                .debug_struct("KeyRelease")
                .field("key", key)
                .field("action", action)
                .field("target", target)
                .finish(),
            GameEvent::KeyHold { key, action, target } => f
                .debug_struct("KeyHold")
                .field("key", key)
                .field("action", action)
                .field("target", target)
                .finish(),
            GameEvent::Tick { action, target } => f
                .debug_struct("Tick")
                .field("action", action)
                .field("target", target)
                .finish(),
            GameEvent::Custom { name, target } => f
                .debug_struct("Custom")
                .field("name", name)
                .field("target", target)
                .finish(),
            GameEvent::MousePress { action, target, button } => f
                .debug_struct("MousePress")
                .field("action", action)
                .field("target", target)
                .field("button", button)
                .finish(),
            GameEvent::MouseRelease { action, target, button } => f
                .debug_struct("MouseRelease")
                .field("action", action)
                .field("target", target)
                .field("button", button)
                .finish(),
            GameEvent::MouseEnter { action, target } => f
                .debug_struct("MouseEnter")
                .field("action", action)
                .field("target", target)
                .finish(),
            GameEvent::MouseLeave { action, target } => f
                .debug_struct("MouseLeave")
                .field("action", action)
                .field("target", target)
                .finish(),
            GameEvent::MouseOver { action, target } => f
                .debug_struct("MouseOver")
                .field("action", action)
                .field("target", target)
                .finish(),
            GameEvent::MouseScroll { action, target, axis } => f
                .debug_struct("MouseScroll")
                .field("action", action)
                .field("target", target)
                .field("axis", axis)
                .finish(),
            GameEvent::MouseMove { action, target } => f
                .debug_struct("MouseMove")
                .field("action", action)
                .field("target", target)
                .finish(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameObject {
    layout: prism::layout::Stack,
    pub id: String,
    pub tags: Vec<String>,
    drawable: Option<Box<dyn Drawable>>,
    pub animated_sprite: Option<AnimatedSprite>,
    pub size: (f32, f32),
    pub position: (f32, f32),
    pub momentum: (f32, f32),
    pub resistance: (f32, f32),
    pub gravity: f32,
    pub scaled_size: Cell<(f32, f32)>,
    pub is_platform: bool,
    pub visible: bool,
}

impl OnEvent for GameObject {}

impl Component for GameObject {
    fn children(&self) -> Vec<&dyn Drawable> {
        if self.visible {
            self.drawable.as_ref().map(|img| vec![img as &dyn Drawable]).unwrap_or_default()
        } else {
            vec![]
        }
    }

    fn children_mut(&mut self) -> Vec<&mut dyn Drawable> {
        if self.visible {
            self.drawable.as_mut().map(|img| vec![img as &mut dyn Drawable]).unwrap_or_default()
        } else {
            vec![]
        }
    }

    fn layout(&self) -> &dyn prism::layout::Layout {
        &self.layout
    }
}

impl GameObject {
    pub fn new(
        _ctx: &mut Context,
        id: String,
        drawable: Option<impl Drawable + 'static>,
        size: f32,
        position: (f32, f32),
        tags: Vec<String>,
        momentum: (f32, f32),
        resistance: (f32, f32),
        gravity: f32,
    ) -> Self {
        Self {
            layout: prism::layout::Stack::default(),
            id,
            tags,
            drawable: drawable.map(|d| Box::new(d) as Box<dyn Drawable>),
            animated_sprite: None,
            size: (size, size),
            position,
            momentum,
            resistance,
            gravity,
            scaled_size: Cell::new((size, size)),
            is_platform: false,
            visible: true,
        }
    }

    pub fn new_rect(
        _ctx: &mut Context,
        id: String,
        drawable: Option<impl Drawable + 'static>,
        size: (f32, f32),
        position: (f32, f32),
        tags: Vec<String>,
        momentum: (f32, f32),
        resistance: (f32, f32),
        gravity: f32,
    ) -> Self {
        Self {
            id,
            tags,
            drawable: drawable.map(|d| Box::new(d) as Box<dyn Drawable>),
            animated_sprite: None,
            size,
            position,
            momentum,
            resistance,
            gravity,
            scaled_size: Cell::new(size),
            is_platform: false,
            visible: true,
            layout: prism::layout::Stack::default(),
        }
    }

    pub fn with_animation(mut self, animated_sprite: AnimatedSprite) -> Self {
        self.animated_sprite = Some(animated_sprite);
        self
    }

    pub fn with_image(mut self, image: Image) -> Self {
        self.drawable = Some(Box::new(image));
        self
    }

    pub fn as_platform(mut self) -> Self {
        self.is_platform = true;
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_gravity(mut self, gravity: f32) -> Self {
        self.gravity = gravity;
        self
    }

    pub fn with_momentum(mut self, momentum: (f32, f32)) -> Self {
        self.momentum = momentum;
        self
    }

    pub fn with_resistance(mut self, resistance: (f32, f32)) -> Self {
        self.resistance = resistance;
        self
    }

    pub fn set_gravity(&mut self, gravity: f32) {
        self.gravity = gravity;
    }

    pub fn set_animation(&mut self, animated_sprite: AnimatedSprite) {
        self.animated_sprite = Some(animated_sprite);
    }

    pub fn set_image(&mut self, image: Image) {
        self.drawable = Some(Box::new(image));
    }

    pub fn update_position(&mut self) {
        self.position.0 += self.momentum.0;
        self.position.1 += self.momentum.1;
    }

    pub fn apply_gravity(&mut self) {
        self.momentum.1 += self.gravity;
    }

    pub fn apply_resistance(&mut self) {
        self.momentum.0 *= self.resistance.0;
        self.momentum.1 *= self.resistance.1;

        if self.momentum.0.abs() < 0.001 {
            self.momentum.0 = 0.0;
        }
        if self.momentum.1.abs() < 0.001 {
            self.momentum.1 = 0.0;
        }
    }

    pub fn update_animation(&mut self, delta_time: f32) {
        if let Some(sprite) = &mut self.animated_sprite {
            sprite.update(delta_time);
            let mut img = sprite.get_current_image();
            let scaled = self.scaled_size.get();
            img.shape = ShapeType::Rectangle(0.0, scaled, 0.0);
            self.drawable = Some(Box::new(img));
        }
    }

    pub fn update_image_shape(&mut self) {
        if let Some(drawable) = self.drawable.as_mut() {
            if let Some(ref mut img) = drawable.downcast_mut::<Image>() {
                let scaled = self.scaled_size.get();
                img.shape = ShapeType::Rectangle(0.0, scaled, 0.0);
            }
        }
    }

    pub fn check_boundary_collision(&self, canvas_size: (f32, f32)) -> bool {
        self.position.0 <= 0.0
            || self.position.0 + self.size.0 >= canvas_size.0
            || self.position.1 <= 0.0
            || self.position.1 + self.size.1 >= canvas_size.1
    }

    pub fn get_anchor_position(&self, anchor: Anchor) -> (f32, f32) {
        (
            self.position.0 + self.size.0 * anchor.x,
            self.position.1 + self.size.1 * anchor.y,
        )
    }


    pub fn contains_point(&self, point: (f32, f32)) -> bool {
        point.0 >= self.position.0
            && point.0 <= self.position.0 + self.size.0
            && point.1 >= self.position.1
            && point.1 <= self.position.1 + self.size.1
    }
}