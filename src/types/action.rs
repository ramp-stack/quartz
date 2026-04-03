use prism::canvas::{Color, Text};
use crate::object::GameObject;
use crate::value::{Expr, MathOp};
use crate::sound::SoundOptions;
use super::targeting::{Target, Location};
use super::collision::CollisionMode;
use super::condition::Condition;

#[derive(Clone, Debug)]
pub enum Action {
    ApplyMomentum { target: Target, value: (f32, f32) },
    SetMomentum   { target: Target, value: (f32, f32) },
    Spawn         { object: Box<GameObject>, location: Location },
    SetResistance { target: Target, value: (f32, f32) },
    Remove        { target: Target },
    TransferMomentum { from: Target, to: Target, scale: f32 },
    SetAnimation  { target: Target, animation_bytes: &'static [u8], fps: f32 },
    Teleport      { target: Target, location: Location },
    Show          { target: Target },
    Hide          { target: Target },
    Toggle        { target: Target },
    Conditional   { condition: Condition, if_true: Box<Action>, if_false: Option<Box<Action>> },
    Custom        { name: String },
    SetVar        { name: String, value: Expr },
    ModVar        { name: String, op: MathOp, operand: Expr },
    Multi(Vec<Action>),
    PlaySound     { path: String, options: SoundOptions },
    SetGravity    { target: Target, value: f32 },
    SetSize       { target: Target, value: (f32, f32) },
    AddTag        { target: Target, tag: String },
    RemoveTag     { target: Target, tag: String },
    SetText       { target: Target, text: Text },
    Expr(String),
    SetRotation   { target: Target, value: f32 },
    SetSlope      { target: Target, left_offset: f32, right_offset: f32, auto_rotate: bool },
    AddRotation   { target: Target, value: f32 },
    ApplyRotation { target: Target, value: f32 },
    SetSurfaceNormal { target: Target, nx: f32, ny: f32 },
    SetCollisionMode { target: Target, mode: CollisionMode },
    SetGlow       { target: Target, color: Color, width: f32 },
    ClearGlow     { target: Target },
    SetTint       { target: Target, color: Color },
    ClearTint     { target: Target },
}

impl Action {
    pub fn expr(s: impl Into<String>) -> Self { Action::Expr(s.into()) }

    pub fn expr_checked(s: impl Into<String>) -> Result<Self, String> {
        let src = s.into();
        crate::expr::parse_action(&src)?;
        Ok(Action::Expr(src))
    }

    pub fn when(cond: Condition, if_true: Action, if_false: Option<Action>) -> Self {
        Action::Conditional { condition: cond, if_true: Box::new(if_true), if_false: if_false.map(Box::new) }
    }
    pub fn when_if(cond: Condition, if_true: Action) -> Self {
        Action::Conditional { condition: cond, if_true: Box::new(if_true), if_false: None }
    }
    pub fn when_else(cond: Condition, if_true: Action, if_false: Action) -> Self {
        Action::Conditional { condition: cond, if_true: Box::new(if_true), if_false: Some(Box::new(if_false)) }
    }
    pub fn multi(actions: Vec<Action>) -> Self { Action::Multi(actions) }
    pub fn set_var(name: impl Into<String>, value: impl Into<Expr>) -> Self {
        Action::SetVar { name: name.into(), value: value.into() }
    }
    pub fn apply_momentum(target: Target, x: f32, y: f32) -> Self {
        Action::ApplyMomentum { target, value: (x, y) }
    }
    pub fn apply_rotation(target: Target, value: f32) -> Self { Action::ApplyRotation { target, value } }
    pub fn set_rotation(target: Target, value: f32) -> Self   { Action::SetRotation { target, value } }
    pub fn add_rotation(target: Target, value: f32) -> Self   { Action::AddRotation { target, value } }
    pub fn show(target: Target)   -> Self { Action::Show { target } }
    pub fn hide(target: Target)   -> Self { Action::Hide { target } }
    pub fn toggle(target: Target) -> Self { Action::Toggle { target } }
    pub fn remove(target: Target) -> Self { Action::Remove { target } }
    pub fn spawn(object: GameObject, location: Location) -> Self {
        Action::Spawn { object: Box::new(object), location }
    }
    pub fn teleport(target: Target, location: Location) -> Self {
        Action::Teleport { target, location }
    }
    pub fn set_momentum(target: Target, x: f32, y: f32) -> Self {
        Action::SetMomentum { target, value: (x, y) }
    }
    pub fn set_resistance(target: Target, x: f32, y: f32) -> Self {
        Action::SetResistance { target, value: (x, y) }
    }
    pub fn set_gravity(target: Target, value: f32) -> Self { Action::SetGravity { target, value } }
    pub fn transfer_momentum(from: Target, to: Target, scale: f32) -> Self {
        Action::TransferMomentum { from, to, scale }
    }
    pub fn set_size(target: Target, width: f32, height: f32) -> Self {
        Action::SetSize { target, value: (width, height) }
    }
    pub fn add_tag(target: Target, tag: impl Into<String>) -> Self {
        Action::AddTag { target, tag: tag.into() }
    }
    pub fn remove_tag(target: Target, tag: impl Into<String>) -> Self {
        Action::RemoveTag { target, tag: tag.into() }
    }
    pub fn set_text(target: Target, text: Text) -> Self { Action::SetText { target, text } }
    pub fn play_sound(path: impl Into<String>) -> Self {
        Action::PlaySound { path: path.into(), options: SoundOptions::default() }
    }
    pub fn play_sound_with_options(path: impl Into<String>, options: SoundOptions) -> Self {
        Action::PlaySound { path: path.into(), options }
    }
    pub fn set_animation(target: Target, animation_bytes: &'static [u8], fps: f32) -> Self {
        Action::SetAnimation { target, animation_bytes, fps }
    }
    pub fn set_slope(target: Target, left: f32, right: f32, auto_rotate: bool) -> Self {
        Action::SetSlope { target, left_offset: left, right_offset: right, auto_rotate }
    }
    pub fn set_surface_normal(target: Target, nx: f32, ny: f32) -> Self {
        Action::SetSurfaceNormal { target, nx, ny }
    }
    pub fn mod_var(name: impl Into<String>, op: MathOp, operand: impl Into<Expr>) -> Self {
        Action::ModVar { name: name.into(), op, operand: operand.into() }
    }
    pub fn custom(name: impl Into<String>) -> Self { Action::Custom { name: name.into() } }
    pub fn set_collision_mode(target: Target, mode: CollisionMode) -> Self {
        Action::SetCollisionMode { target, mode }
    }
    pub fn set_glow(target: Target, color: Color, width: f32) -> Self {
        Action::SetGlow { target, color, width }
    }
    pub fn clear_glow(target: Target) -> Self { Action::ClearGlow { target } }
    pub fn set_tint(target: Target, color: Color) -> Self { Action::SetTint { target, color } }
    pub fn clear_tint(target: Target) -> Self { Action::ClearTint { target } }
}