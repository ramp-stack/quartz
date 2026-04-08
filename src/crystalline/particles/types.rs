#[derive(Clone, Debug)]
pub struct Particle {
    pub position: (f32, f32),
    pub velocity: (f32, f32),
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub color: (u8, u8, u8, u8),
    pub gravity_scale: f32,
    pub rotation: f32,
    pub collision_response: CollisionResponse,
}

#[derive(Clone, Debug)]
pub enum CollisionResponse {
    None,
    Bounce { elasticity: f32 },
    Die,
}

impl Default for CollisionResponse {
    fn default() -> Self {
        CollisionResponse::None
    }
}

#[derive(Clone, Debug)]
pub struct Emitter {
    pub name: String,
    pub origin: (f32, f32),
    pub rate: f32,
    pub lifetime: f32,
    pub velocity_base: (f32, f32),
    pub velocity_spread: (f32, f32),
    pub size: f32,
    pub color: (u8, u8, u8, u8),
    pub gravity_scale: f32,
    pub collision_response: CollisionResponse,
    /// Rotation in degrees applied to velocity_base when emitting particles.
    pub rotation: f32,
}

impl Emitter {
    pub fn fire(pos: (f32, f32)) -> Self {
        Self { name: "fire".into(), origin: pos, rate: 60.0, lifetime: 0.8,
            velocity_base: (0.0, -80.0), velocity_spread: (30.0, 20.0),
            size: 4.0, color: (255, 120, 20, 220), gravity_scale: -0.2,
            collision_response: CollisionResponse::Die, rotation: 0.0 }
    }
    pub fn smoke(pos: (f32, f32)) -> Self {
        Self { name: "smoke".into(), origin: pos, rate: 20.0, lifetime: 2.0,
            velocity_base: (0.0, -30.0), velocity_spread: (15.0, 10.0),
            size: 8.0, color: (140, 140, 140, 100), gravity_scale: -0.1,
            collision_response: CollisionResponse::None, rotation: 0.0 }
    }
    pub fn sparks(pos: (f32, f32)) -> Self {
        Self { name: "sparks".into(), origin: pos, rate: 80.0, lifetime: 0.4,
            velocity_base: (0.0, -120.0), velocity_spread: (80.0, 60.0),
            size: 2.0, color: (255, 220, 50, 255), gravity_scale: 0.8,
            collision_response: CollisionResponse::Bounce { elasticity: 0.6 }, rotation: 0.0 }
    }
    pub fn rain(canvas_width: f32) -> Self {
        Self { name: "rain".into(), origin: (canvas_width * 0.5, -20.0), rate: 200.0, lifetime: 2.5,
            velocity_base: (0.0, 300.0), velocity_spread: (canvas_width * 0.5, 40.0),
            size: 2.0, color: (100, 150, 255, 180), gravity_scale: 0.5,
            collision_response: CollisionResponse::Die, rotation: 0.0 }
    }
    pub fn snow(canvas_width: f32) -> Self {
        Self { name: "snow".into(), origin: (canvas_width * 0.5, -20.0), rate: 80.0, lifetime: 5.0,
            velocity_base: (0.0, 40.0), velocity_spread: (canvas_width * 0.5, 15.0),
            size: 3.0, color: (240, 245, 255, 200), gravity_scale: 0.05,
            collision_response: CollisionResponse::Die, rotation: 0.0 }
    }
    pub fn dust(pos: (f32, f32)) -> Self {
        Self { name: "dust".into(), origin: pos, rate: 15.0, lifetime: 0.6,
            velocity_base: (0.0, -10.0), velocity_spread: (20.0, 10.0),
            size: 3.0, color: (160, 130, 90, 120), gravity_scale: 0.1,
            collision_response: CollisionResponse::None, rotation: 0.0 }
    }
    pub fn explosion(pos: (f32, f32)) -> Self {
        Self { name: "explosion".into(), origin: pos, rate: 500.0, lifetime: 0.5,
            velocity_base: (0.0, 0.0), velocity_spread: (200.0, 200.0),
            size: 4.0, color: (255, 180, 50, 255), gravity_scale: 0.3,
            collision_response: CollisionResponse::Die, rotation: 0.0 }
    }
    pub fn trail(color: (u8, u8, u8, u8)) -> Self {
        Self { name: "trail".into(), origin: (0.0, 0.0), rate: 30.0, lifetime: 0.5,
            velocity_base: (0.0, 0.0), velocity_spread: (5.0, 5.0),
            size: 3.0, color, gravity_scale: 0.0,
            collision_response: CollisionResponse::None, rotation: 0.0 }
    }
    pub fn confetti(pos: (f32, f32)) -> Self {
        Self { name: "confetti".into(), origin: pos, rate: 100.0, lifetime: 2.0,
            velocity_base: (0.0, -60.0), velocity_spread: (100.0, 80.0),
            size: 5.0, color: (255, 100, 200, 255), gravity_scale: 0.4,
            collision_response: CollisionResponse::Bounce { elasticity: 0.3 }, rotation: 0.0 }
    }
    pub fn bubbles(pos: (f32, f32)) -> Self {
        Self { name: "bubbles".into(), origin: pos, rate: 12.0, lifetime: 3.0,
            velocity_base: (0.0, -40.0), velocity_spread: (15.0, 10.0),
            size: 6.0, color: (180, 220, 255, 150), gravity_scale: -0.15,
            collision_response: CollisionResponse::Bounce { elasticity: 0.2 }, rotation: 0.0 }
    }

    // -- Space particle presets -------------------------------------------

    pub fn thruster_exhaust(pos: (f32, f32)) -> Self {
        Self { name: "thruster".into(), origin: pos, rate: 80.0, lifetime: 0.4,
            velocity_base: (0.0, 60.0), velocity_spread: (25.0, 15.0),
            size: 3.0, color: (255, 180, 50, 200), gravity_scale: 0.0,
            collision_response: CollisionResponse::Die, rotation: 0.0 }
    }
    pub fn reentry_sparks(pos: (f32, f32)) -> Self {
        Self { name: "reentry".into(), origin: pos, rate: 120.0, lifetime: 0.25,
            velocity_base: (0.0, 0.0), velocity_spread: (80.0, 80.0),
            size: 2.0, color: (255, 100, 30, 255), gravity_scale: 0.0,
            collision_response: CollisionResponse::Die, rotation: 0.0 }
    }
    pub fn asteroid_debris(pos: (f32, f32)) -> Self {
        Self { name: "debris".into(), origin: pos, rate: 15.0, lifetime: 3.0,
            velocity_base: (0.0, 0.0), velocity_spread: (30.0, 30.0),
            size: 5.0, color: (140, 120, 100, 180), gravity_scale: 0.3,
            collision_response: CollisionResponse::Bounce { elasticity: 0.4 }, rotation: 0.0 }
    }
    pub fn solar_wind(_canvas_width: f32) -> Self {
        Self { name: "solar_wind".into(), origin: (0.0, 0.0), rate: 40.0, lifetime: 5.0,
            velocity_base: (20.0, 5.0), velocity_spread: (5.0, 3.0),
            size: 2.0, color: (255, 255, 200, 60), gravity_scale: 0.0,
            collision_response: CollisionResponse::None, rotation: 0.0 }
    }

    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// One-shot hull damage burst — small debris chunks flying outward.
    pub fn damage_burst(pos: (f32, f32)) -> Self {
        Self { name: "damage".into(), origin: pos, rate: 300.0, lifetime: 0.3,
            velocity_base: (0.0, 0.0), velocity_spread: (160.0, 160.0),
            size: 4.0, color: (200, 180, 160, 230), gravity_scale: 0.0,
            collision_response: CollisionResponse::None, rotation: 0.0 }
    }
}

// ── EmitterBuilder ───────────────────────────────────────────

pub struct EmitterBuilder {
    inner: Emitter,
}

impl EmitterBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            inner: Emitter {
                name: name.into(), origin: (0.0, 0.0), rate: 30.0, lifetime: 1.0,
                velocity_base: (0.0, 0.0), velocity_spread: (10.0, 10.0),
                size: 4.0, color: (255, 255, 255, 255), gravity_scale: 0.0,
                collision_response: CollisionResponse::None, rotation: 0.0,
            },
        }
    }
    pub fn origin(mut self, x: f32, y: f32) -> Self { self.inner.origin = (x, y); self }
    pub fn rate(mut self, r: f32) -> Self { self.inner.rate = r; self }
    pub fn lifetime(mut self, l: f32) -> Self { self.inner.lifetime = l; self }
    pub fn velocity(mut self, vx: f32, vy: f32) -> Self { self.inner.velocity_base = (vx, vy); self }
    pub fn spread(mut self, sx: f32, sy: f32) -> Self { self.inner.velocity_spread = (sx, sy); self }
    pub fn size(mut self, s: f32) -> Self { self.inner.size = s; self }
    pub fn color(mut self, r: u8, g: u8, b: u8, a: u8) -> Self { self.inner.color = (r, g, b, a); self }
    pub fn gravity_scale(mut self, g: f32) -> Self { self.inner.gravity_scale = g; self }
    pub fn collision(mut self, resp: CollisionResponse) -> Self { self.inner.collision_response = resp; self }
    pub fn build(self) -> Emitter { self.inner }
}
