use prism::canvas::Color;

// ── Light Type ───────────────────────────────────────────────

/// The kind of light source. Point is the standard 2D radial light.
/// Spot and Directional are reserved for future expansion.
#[derive(Clone, Debug, PartialEq)]
pub enum LightType {
    Point,
    Spot {
        direction: f32,
        cone_angle: f32,
    },
    Directional {
        direction: (f32, f32),
    },
}

impl Default for LightType {
    fn default() -> Self {
        LightType::Point
    }
}

// ── Light Source ──────────────────────────────────────────────

/// A single light source in the scene. Converts to a wgpu_canvas
/// `PointLight` at emit time — Quartz never touches the GPU directly.
#[derive(Clone, Debug)]
pub struct LightSource {
    pub id: String,
    pub light_type: LightType,
    pub position: (f32, f32),
    pub color: Color,
    pub radius: f32,
    pub intensity: f32,
    pub enabled: bool,
    pub casts_shadows: bool,
    pub effect: Option<LightEffect>,
    pub(crate) effect_time: f32,
}

impl LightSource {
    pub fn new(id: impl Into<String>, position: (f32, f32), color: Color, radius: f32, intensity: f32) -> Self {
        Self {
            id: id.into(),
            light_type: LightType::Point,
            position,
            color,
            radius,
            intensity,
            enabled: true,
            casts_shadows: true,
            effect: None,
            effect_time: 0.0,
        }
    }

    // ── Presets ──────────────────────────────────────────────

    pub fn torch(position: (f32, f32)) -> Self {
        Self::new("torch", position, Color(255, 180, 80, 255), 300.0, 0.8)
    }

    pub fn campfire(position: (f32, f32)) -> Self {
        let mut s = Self::new("campfire", position, Color(255, 140, 40, 255), 450.0, 1.0);
        s.effect = Some(LightEffect::Flicker { base_intensity: 1.0, variance: 0.15 });
        s
    }

    pub fn moonlight(position: (f32, f32)) -> Self {
        Self::new("moonlight", position, Color(160, 180, 255, 255), 800.0, 0.3)
    }

    pub fn neon(position: (f32, f32), color: Color) -> Self {
        Self::new("neon", position, color, 150.0, 1.2)
    }

    pub fn lantern(position: (f32, f32)) -> Self {
        let mut s = Self::new("lantern", position, Color(255, 220, 150, 255), 350.0, 0.7);
        s.effect = Some(LightEffect::Pulse { min_intensity: 0.6, max_intensity: 0.8, speed: 1.5 });
        s
    }

    pub fn spotlight(position: (f32, f32), direction: f32, cone_angle: f32) -> Self {
        Self {
            light_type: LightType::Spot { direction, cone_angle },
            ..Self::new("spotlight", position, Color(255, 255, 240, 255), 500.0, 1.0)
        }
    }

    pub fn sun(direction: (f32, f32)) -> Self {
        Self {
            light_type: LightType::Directional { direction },
            casts_shadows: true,
            ..Self::new("sun", (0.0, 0.0), Color(255, 248, 220, 255), f32::MAX, 0.6)
        }
    }

    /// Convenience: set a unique ID on a preset.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Convenience: set shadow casting.
    pub fn with_shadows(mut self, casts: bool) -> Self {
        self.casts_shadows = casts;
        self
    }

    /// Convenience: attach an effect.
    pub fn with_effect(mut self, effect: LightEffect) -> Self {
        self.effect = Some(effect);
        self
    }

    /// Convert this high-level source to a wgpu_canvas PointLight.
    pub(crate) fn to_point_light(&self) -> prism::canvas::PointLight {
        let (r, g, b) = color_to_rgb(&self.color);

        let kind = match &self.light_type {
            LightType::Point => wgpu_canvas::LightKind::Point,

            LightType::Spot { direction, cone_angle } => wgpu_canvas::LightKind::Spot {
                direction: (direction.cos(), direction.sin()),
                cone_half_angle: cone_angle / 2.0,
            },

            LightType::Directional { direction } => wgpu_canvas::LightKind::Directional {
                direction: *direction,
            },
        };

        prism::canvas::PointLight {
            position: self.position,
            color: (r, g, b),
            radius: self.radius,
            intensity: self.intensity,
            kind,
        }
    }
}

// ── Ambient Light ────────────────────────────────────────────

/// Global ambient illumination that applies uniformly to all lit surfaces.
#[derive(Clone, Debug)]
pub struct AmbientLight {
    pub color: Color,
    pub strength: f32,
}

impl Default for AmbientLight {
    fn default() -> Self {
        Self {
            color: Color(255, 255, 255, 255),
            strength: 1.0,
        }
    }
}

impl AmbientLight {
    pub fn dark() -> Self {
        Self { color: Color(10, 10, 25, 255), strength: 0.06 }
    }

    pub fn dim() -> Self {
        Self { color: Color(80, 80, 120, 255), strength: 0.2 }
    }

    pub fn bright() -> Self {
        Self { color: Color(255, 255, 255, 255), strength: 0.8 }
    }

    pub(crate) fn as_rgb(&self) -> (f32, f32, f32) {
        color_to_rgb(&self.color)
    }
}

// ── Lighting Config ──────────────────────────────────────────

/// Top-level configuration for the lighting system.
#[derive(Clone, Debug)]
pub struct LightingConfig {
    pub ambient: AmbientLight,
    pub max_lights: usize,
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            ambient: AmbientLight::default(),
            max_lights: 64,
        }
    }
}

impl LightingConfig {
    /// Night mode preset: very dark ambient, allow up to 64 lights.
    pub fn night() -> Self {
        Self {
            ambient: AmbientLight::dark(),
            max_lights: 64,
        }
    }

    /// Indoor preset: dim ambient.
    pub fn indoor() -> Self {
        Self {
            ambient: AmbientLight::dim(),
            max_lights: 64,
        }
    }

    /// Daytime outdoor preset: bright ambient, fewer dramatic lights needed.
    pub fn day() -> Self {
        Self {
            ambient: AmbientLight::bright(),
            max_lights: 32,
        }
    }
}

// ── Light Attachment ─────────────────────────────────────────

/// Bind a light to a game object so it follows automatically.
#[derive(Clone, Debug)]
pub struct LightAttachment {
    pub light_id: String,
    pub object_name: String,
    pub offset: (f32, f32),
}

// ── Light Effects ────────────────────────────────────────────

/// Per-light animation effects ticked by the lighting system.
#[derive(Clone, Debug, PartialEq)]
pub enum LightEffect {
    Pulse {
        min_intensity: f32,
        max_intensity: f32,
        speed: f32,
    },
    Flicker {
        base_intensity: f32,
        variance: f32,
    },
    ColorCycle {
        colors: Vec<Color>,
        speed: f32,
    },
    FadeIn {
        target_intensity: f32,
        duration: f32,
    },
    FadeOut {
        duration: f32,
    },
}

// ── Helpers ──────────────────────────────────────────────────

pub(crate) fn color_to_rgb(c: &Color) -> (f32, f32, f32) {
    (c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0)
}
