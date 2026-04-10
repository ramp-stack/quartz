use prism::canvas::{Color, Text};
use crate::object::GameObject;
use crate::value::{Expr, MathOp};
use crate::sound::SoundOptions;
use crate::crystalline::{PhysicsMaterial, PhysicsQuality, Emitter, CollisionResponse};
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

    // -- Material (crystalline) ---
    SetMaterial      { target: Target, material: PhysicsMaterial },
    SetElasticity    { target: Target, value: f32 },
    SetFriction      { target: Target, value: f32 },
    SetDensity       { target: Target, value: f32 },

    // -- Forces / impulses ---
    ApplyForce       { target: Target, fx: f32, fy: f32 },
    ApplyImpulse     { target: Target, ix: f32, iy: f32 },

    // -- Position (ball_swing feedback) ---
    SetPosition      { target: Target, x: f32, y: f32 },
    SetCameraRelative { target: Target, enabled: bool },

    // -- Body state ---
    WakeBody         { target: Target },
    FreezeBody       { target: Target },
    UnfreezeBody     { target: Target },

    // -- Per-body tuning ---
    SetCollisionLayer { target: Target, layer: u32 },

    // -- Global physics ---
    SetPhysicsQuality { quality: PhysicsQuality },
    EnableCrystalline,
    DisableCrystalline,

    // -- Particle lifecycle ---
    SpawnEmitter     { emitter: Emitter },
    RemoveEmitter    { name: String },
    AttachEmitter    { emitter_name: String, target: Target, location: Option<Location> },
    DetachEmitter    { emitter_name: String },

    // -- Emitter modification ---
    SetEmitterRate          { name: String, value: f32 },
    SetEmitterLifetime      { name: String, value: f32 },
    SetEmitterVelocity      { name: String, value: (f32, f32) },
    SetEmitterSpread        { name: String, value: (f32, f32) },
    SetEmitterSize          { name: String, value: f32 },
    SetEmitterColor         { name: String, value: (u8, u8, u8, u8) },
    SetEmitterGravityScale  { name: String, value: f32 },
    SetEmitterCollision     { name: String, value: CollisionResponse },
    SetEmitterRenderLayer   { name: String, value: i32 },

    // -- Render layer ---
    SetRenderLayer  { target: Target, layer: i32 },

    // -- Camera zoom ---
    SetZoom { value: f32 },
    AddZoom { value: f32 },

    // -- Planet gravity actions ---
    SetGravityStrength { target: Target, value: f32 },
    SetPlanetRadius    { target: Target, value: f32 },
    SetGravityTarget   { target: Target, tag: String },
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

    // -- Crystalline convenience constructors --
    pub fn set_material(target: Target, material: PhysicsMaterial) -> Self {
        Action::SetMaterial { target, material }
    }
    pub fn set_elasticity(target: Target, value: f32) -> Self { Action::SetElasticity { target, value } }
    pub fn set_friction(target: Target, value: f32) -> Self { Action::SetFriction { target, value } }
    pub fn set_density(target: Target, value: f32) -> Self { Action::SetDensity { target, value } }
    pub fn apply_force(target: Target, fx: f32, fy: f32) -> Self { Action::ApplyForce { target, fx, fy } }
    pub fn apply_impulse(target: Target, ix: f32, iy: f32) -> Self { Action::ApplyImpulse { target, ix, iy } }
    pub fn set_position(target: Target, x: f32, y: f32) -> Self { Action::SetPosition { target, x, y } }
    pub fn set_camera_relative(target: Target, enabled: bool) -> Self {
        Action::SetCameraRelative { target, enabled }
    }
    pub fn wake_body(target: Target) -> Self { Action::WakeBody { target } }
    pub fn freeze_body(target: Target) -> Self { Action::FreezeBody { target } }
    pub fn unfreeze_body(target: Target) -> Self { Action::UnfreezeBody { target } }
    pub fn set_collision_layer(target: Target, layer: u32) -> Self {
        Action::SetCollisionLayer { target, layer }
    }
    pub fn enable_crystalline() -> Self { Action::EnableCrystalline }
    pub fn disable_crystalline() -> Self { Action::DisableCrystalline }
    pub fn spawn_emitter(emitter: Emitter) -> Self { Action::SpawnEmitter { emitter } }
    pub fn remove_emitter(name: impl Into<String>) -> Self { Action::RemoveEmitter { name: name.into() } }
    pub fn attach_emitter(emitter_name: impl Into<String>, target: Target) -> Self {
        Action::AttachEmitter { emitter_name: emitter_name.into(), target, location: None }
    }
    pub fn attach_emitter_at(emitter_name: impl Into<String>, target: Target, location: Location) -> Self {
        Action::AttachEmitter { emitter_name: emitter_name.into(), target, location: Some(location) }
    }
    pub fn detach_emitter(emitter_name: impl Into<String>) -> Self {
        Action::DetachEmitter { emitter_name: emitter_name.into() }
    }
    pub fn set_emitter_rate(name: impl Into<String>, value: f32) -> Self {
        Action::SetEmitterRate { name: name.into(), value }
    }
    pub fn set_emitter_lifetime(name: impl Into<String>, value: f32) -> Self {
        Action::SetEmitterLifetime { name: name.into(), value }
    }
    pub fn set_emitter_velocity(name: impl Into<String>, vx: f32, vy: f32) -> Self {
        Action::SetEmitterVelocity { name: name.into(), value: (vx, vy) }
    }
    pub fn set_emitter_spread(name: impl Into<String>, sx: f32, sy: f32) -> Self {
        Action::SetEmitterSpread { name: name.into(), value: (sx, sy) }
    }
    pub fn set_emitter_size(name: impl Into<String>, value: f32) -> Self {
        Action::SetEmitterSize { name: name.into(), value }
    }
    pub fn set_emitter_color(name: impl Into<String>, r: u8, g: u8, b: u8, a: u8) -> Self {
        Action::SetEmitterColor { name: name.into(), value: (r, g, b, a) }
    }
    pub fn set_emitter_gravity_scale(name: impl Into<String>, value: f32) -> Self {
        Action::SetEmitterGravityScale { name: name.into(), value }
    }
    pub fn set_emitter_collision(name: impl Into<String>, value: CollisionResponse) -> Self {
        Action::SetEmitterCollision { name: name.into(), value }
    }
    pub fn set_emitter_render_layer(name: impl Into<String>, value: i32) -> Self {
        Action::SetEmitterRenderLayer { name: name.into(), value }
    }
    pub fn set_render_layer(target: Target, layer: i32) -> Self {
        Action::SetRenderLayer { target, layer }
    }
    pub fn set_zoom(value: f32) -> Self {
        Action::SetZoom { value }
    }
    pub fn add_zoom(value: f32) -> Self {
        Action::AddZoom { value }
    }
    pub fn set_gravity_strength(target: Target, value: f32) -> Self {
        Action::SetGravityStrength { target, value }
    }
    pub fn set_planet_radius(target: Target, value: f32) -> Self {
        Action::SetPlanetRadius { target, value }
    }
    pub fn set_gravity_target(target: Target, tag: impl Into<String>) -> Self {
        Action::SetGravityTarget { target, tag: tag.into() }
    }
}