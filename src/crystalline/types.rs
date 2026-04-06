// ── Collision ────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum CollisionShape {
    Rectangle,
    Circle { radius: f32 },
}

impl Default for CollisionShape {
    fn default() -> Self {
        CollisionShape::Rectangle
    }
}

#[derive(Clone, Debug)]
pub enum CrystallineCollisionMode {
    NonPlatform,
    Surface,
    Solid(CollisionShape),
}

impl Default for CrystallineCollisionMode {
    fn default() -> Self {
        CrystallineCollisionMode::Surface
    }
}

// ── Material ─────────────────────────────────────────────────

/// Material properties for a physics body.
#[derive(Clone, Copy, Debug)]
pub struct PhysicsMaterial {
    /// Elasticity: how much kinetic energy is preserved after a bounce.
    /// 0.0 = perfectly inelastic (no bounce), 1.0 = perfectly elastic (full bounce).
    pub elasticity: f32,
    /// Surface friction coefficient.
    pub friction: f32,
    /// Mass contribution (density × area).
    pub density: f32,
}

impl Default for PhysicsMaterial {
    fn default() -> Self {
        Self {
            elasticity: 0.0,
            friction: 0.5,
            density: 1.0,
        }
    }
}

impl PhysicsMaterial {
    pub fn rubber()  -> Self { Self { elasticity: 0.8, friction: 0.9, density: 1.1 } }
    pub fn ice()     -> Self { Self { elasticity: 0.1, friction: 0.05, density: 0.9 } }
    pub fn metal()   -> Self { Self { elasticity: 0.3, friction: 0.4, density: 7.8 } }
    pub fn wood()    -> Self { Self { elasticity: 0.4, friction: 0.6, density: 0.6 } }
    pub fn stone()   -> Self { Self { elasticity: 0.2, friction: 0.7, density: 2.4 } }
    pub fn bouncy()  -> Self { Self { elasticity: 1.0, friction: 0.3, density: 0.5 } }
    pub fn sticky()  -> Self { Self { elasticity: 0.0, friction: 1.0, density: 1.0 } }
    pub fn glass()   -> Self { Self { elasticity: 0.5, friction: 0.2, density: 2.5 } }
    pub fn feather() -> Self { Self { elasticity: 0.3, friction: 0.1, density: 0.01 } }
}

// ── Physics Body (firewall input) ────────────────────────────

#[derive(Clone, Debug)]
pub struct PhysicsBody {
    pub id: usize,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub momentum: (f32, f32),
    pub gravity: f32,
    pub resistance: (f32, f32),
    pub rotation: f32,
    pub rotation_momentum: f32,
    pub rotation_resistance: f32,
    pub is_platform: bool,
    pub visible: bool,
    pub collision_mode: CrystallineCollisionMode,
    pub surface_normal: (f32, f32),
    pub slope: Option<(f32, f32)>,
    pub one_way: bool,
    pub surface_velocity: Option<f32>,
    pub material: PhysicsMaterial,
    /// Collision layer bitmask for dynamic-dynamic filtering.
    /// Two non-platform bodies only interact if `layer_a & layer_b != 0`.
    /// Default 0 = no dynamic-dynamic collision (labels, UI, decorations).
    pub collision_layer: u32,
}

// ── Physics Config ───────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct PhysicsConfig {
    pub fixed_dt:            f32,
    pub max_substeps:        u32,
    pub position_iterations: u32,
    pub gravity_scale:       f32,
    /// Particle gravity in Quartz-scale units (same scale as body.gravity).
    /// Internally converted to px/s² by multiplying by 60 (reference frame rate).
    pub particle_gravity:    f32,
    /// Maximum particle speed in px/s. Prevents tunnelling on high-gravity edge cases.
    pub particle_max_speed:  f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            fixed_dt:            1.0 / 240.0,
            max_substeps:        8,
            position_iterations: 6,
            gravity_scale:       1.0,
            particle_gravity:    0.5,
            particle_max_speed:  1200.0,
        }
    }
}

impl PhysicsConfig {
    pub fn platformer() -> Self {
        Self { fixed_dt: 1.0 / 240.0, max_substeps: 8, position_iterations: 6, gravity_scale: 1.0, particle_gravity: 0.5, particle_max_speed: 1200.0 }
    }
    pub fn floaty() -> Self {
        Self { fixed_dt: 1.0 / 240.0, max_substeps: 6, position_iterations: 4, gravity_scale: 0.3, particle_gravity: 0.15, particle_max_speed: 600.0 }
    }
    pub fn realistic() -> Self {
        Self { fixed_dt: 1.0 / 480.0, max_substeps: 16, position_iterations: 10, gravity_scale: 1.0, particle_gravity: 1.0, particle_max_speed: 2400.0 }
    }
    pub fn arcade() -> Self {
        Self { fixed_dt: 1.0 / 120.0, max_substeps: 4, position_iterations: 3, gravity_scale: 1.2, particle_gravity: 0.4, particle_max_speed: 800.0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsQuality {
    Low,
    Medium,
    High,
}

// ── Physics Step Result (firewall output) ────────────────────

pub struct PhysicsStepResult {
    pub body_updates: Vec<BodyUpdate>,
    pub collision_pairs: Vec<(usize, usize)>,
}

pub struct BodyUpdate {
    pub id: usize,
    pub position: (f32, f32),
    pub momentum: (f32, f32),
    pub rotation: f32,
    pub rotation_momentum: f32,
    pub grounded: bool,
}
