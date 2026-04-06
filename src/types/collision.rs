#[derive(Debug, Clone)]
pub enum CollisionShape {
    Rectangle,
    Circle { radius: f32 },
}

impl Default for CollisionShape {
    fn default() -> Self { CollisionShape::Rectangle }
}

impl CollisionShape {
    pub fn circle(radius: f32) -> Self { CollisionShape::Circle { radius } }
    pub fn circle_auto() -> Self { CollisionShape::Circle { radius: 0.0 } }
}

#[derive(Debug, Clone)]
pub enum CollisionMode {
    NonPlatform,
    Surface,
    Solid(CollisionShape),
}

impl Default for CollisionMode {
    fn default() -> Self { CollisionMode::Surface }
}

impl CollisionMode {
    pub fn non_platform() -> Self { CollisionMode::NonPlatform }
    pub fn solid() -> Self { CollisionMode::Solid(CollisionShape::Rectangle) }
    pub fn solid_circle(radius: f32) -> Self { CollisionMode::Solid(CollisionShape::circle(radius)) }
}

pub mod collision_layers {
    pub const NONE:       u32 = 0;
    pub const DEFAULT:    u32 = 1 << 0;
    pub const PLAYER:     u32 = 1 << 1;
    pub const ENEMY:      u32 = 1 << 2;
    pub const PROJECTILE: u32 = 1 << 3;
    pub const PICKUP:     u32 = 1 << 4;
    pub const TRIGGER:    u32 = 1 << 5;
    pub const TERRAIN:    u32 = 1 << 6;
    pub const PARTICLE:   u32 = 1 << 7;
    pub const ALL:        u32 = u32::MAX;
}