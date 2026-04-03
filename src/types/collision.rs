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