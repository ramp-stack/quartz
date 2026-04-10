use prism::drawable::Drawable;
use prism::canvas::{Image, Color};
use prism::Context;
use crate::types::{CollisionMode, GlowConfig, HighlightEffect, collision_layers};
use crate::crystalline::PhysicsMaterial;
use std::cell::Cell;

use super::{GameObject, capture_asset_path};

pub struct GameObjectBuilder {
    pub(super) id:          String,
    pub(super) image:       Option<Image>,
    pub(super) image_path:  Option<String>,
    pub(super) image_mtime: Option<std::time::SystemTime>,
    pub(super) size:        (f32, f32),
    pub(super) position:    (f32, f32),
    pub(super) tags:        Vec<String>,
    pub(super) momentum:    (f32, f32),
    pub(super) resistance:  (f32, f32),
    pub(super) gravity:     f32,
    pub(super) is_platform: bool,
    pub layer:              i32,
    pub(super) rotation:    f32,
    pub(super) slope:       Option<(f32, f32)>,
    pub(super) one_way:     bool,
    pub(super) surface_velocity: Option<f32>,
    pub rotation_momentum:  f32,
    pub rotation_resistance: f32,
    pub(super) surface_normal: (f32, f32),
    pub(super) collision_mode: CollisionMode,
    pub(super) highlight: Option<HighlightEffect>,
    pub(super) material: PhysicsMaterial,
    pub(super) collision_layer: u32,
    pub(super) collision_mask: u32,
    pub(super) clipped: bool,
    pub(super) clip_origin: Option<(f32, f32)>,
    pub(super) planet_radius:        Option<f32>,
    pub(super) gravity_target:       Option<String>,
    pub(super) gravity_strength:     f32,
    pub(super) auto_align:           bool,
    pub(super) auto_align_speed:     f32,
    pub(super) auto_align_threshold: f32,
    pub(super) ignore_zoom:          bool,
}

impl GameObjectBuilder {
    pub fn layer(mut self, id: i32) -> Self { self.layer = id; self }

    pub fn image(mut self, image: Image) -> Self {
        let (path, mtime) = capture_asset_path();
        self.image_path  = path;
        self.image_mtime = mtime;
        self.image = Some(image);
        self
    }

    pub fn size(mut self, w: f32, h: f32) -> Self { self.size = (w, h); self }
    pub fn position(mut self, x: f32, y: f32) -> Self { self.position = (x, y); self }
    pub fn tag(mut self, tag: impl Into<String>) -> Self { self.tags.push(tag.into()); self }
    pub fn momentum(mut self, x: f32, y: f32) -> Self { self.momentum = (x, y); self }
    pub fn resistance(mut self, x: f32, y: f32) -> Self { self.resistance = (x, y); self }
    pub fn gravity(mut self, g: f32) -> Self { self.gravity = g; self }

    pub fn platform(mut self) -> Self {
        self.is_platform    = true;
        self.surface_normal = (0.0, -1.0);
        self
    }
    pub fn floor(self) -> Self { self.platform() }
    pub fn ceiling(mut self) -> Self {
        self.is_platform    = true;
        self.surface_normal = (0.0, 1.0);
        self
    }
    pub fn wall_left(mut self) -> Self {
        self.is_platform    = true;
        self.surface_normal = (1.0, 0.0);
        self
    }
    pub fn wall_right(mut self) -> Self {
        self.is_platform    = true;
        self.surface_normal = (-1.0, 0.0);
        self
    }
    pub fn surface(mut self, nx: f32, ny: f32) -> Self {
        self.is_platform = true;
        let len = (nx * nx + ny * ny).sqrt().max(0.001);
        self.surface_normal = (nx / len, ny / len);
        self
    }

    pub fn rotation(mut self, degrees: f32) -> Self { self.rotation = degrees; self }
    pub fn slope(mut self, left_offset: f32, right_offset: f32) -> Self {
        self.slope = Some((left_offset, right_offset)); self
    }
    pub fn slope_auto_rotation(mut self, left_offset: f32, right_offset: f32) -> Self {
        self.slope = Some((left_offset, right_offset));
        if self.size.0 != 0.0 {
            self.rotation = (right_offset - left_offset).atan2(self.size.0).to_degrees();
        }
        self
    }
    pub fn one_way(mut self) -> Self { self.one_way = true; self }
    pub fn surface_velocity(mut self, vx: f32) -> Self { self.surface_velocity = Some(vx); self }
    pub fn rotation_resistance(mut self, resistance: f32) -> Self {
        self.rotation_resistance = resistance.clamp(0.0, 1.0); self
    }
    pub fn solid(mut self) -> Self {
        self.collision_mode = CollisionMode::solid();
        self.is_platform = true;
        self
    }
    pub fn solid_circle(mut self, radius: f32) -> Self {
        self.collision_mode = CollisionMode::solid_circle(radius);
        self.is_platform = true;
        self
    }
    pub fn collision_mode(mut self, mode: CollisionMode) -> Self {
        match &mode {
            CollisionMode::NonPlatform => { self.is_platform = false; }
            CollisionMode::Surface | CollisionMode::Solid(_) => { self.is_platform = true; }
        }
        self.collision_mode = mode;
        self
    }
    pub fn highlight(mut self, effect: HighlightEffect) -> Self { self.highlight = Some(effect); self }
    pub fn glow(mut self, config: GlowConfig) -> Self {
        let mut effect = self.highlight.take().unwrap_or_default();
        effect.glow = Some(config);
        self.highlight = Some(effect);
        self
    }
    pub fn tint(mut self, color: Color) -> Self {
        let mut effect = self.highlight.take().unwrap_or_default();
        effect.tint = Some(color);
        self.highlight = Some(effect);
        self
    }
    pub fn material(mut self, mat: PhysicsMaterial) -> Self { self.material = mat; self }
    pub fn collision_layer(mut self, layer: u32) -> Self { self.collision_layer = layer; self }
    pub fn collision_mask(mut self, mask: u32) -> Self { self.collision_mask = mask; self }

    /// Clip children to this object's size bounds at draw time.
    pub fn clip(mut self) -> Self { self.clipped = true; self }

    /// Set a fixed clip origin independent of position.
    pub fn clip_origin(mut self, x: f32, y: f32) -> Self {
        self.clip_origin = Some((x, y));
        self
    }

    pub fn planet(mut self, radius: f32) -> Self {
        self.planet_radius = Some(radius.max(0.0));
        self.is_platform = true;
        self.collision_mode = CollisionMode::solid_circle(radius);
        self
    }
    pub fn gravity_target(mut self, tag: impl Into<String>) -> Self {
        self.gravity_target = Some(tag.into()); self
    }
    pub fn gravity_strength(mut self, strength: f32) -> Self {
        self.gravity_strength = strength.max(0.0); self
    }
    pub fn auto_align(mut self) -> Self { self.auto_align = true; self }
    pub fn auto_align_speed(mut self, speed: f32) -> Self {
        self.auto_align_speed = speed.max(0.0); self
    }
    pub fn auto_align_threshold(mut self, threshold: f32) -> Self {
        self.auto_align_threshold = threshold.max(0.0); self
    }
    /// Mark this object as zoom-independent (HUD elements, overlays, etc.).
    pub fn ignore_zoom(mut self) -> Self { self.ignore_zoom = true; self }
    pub fn gravity_well(mut self, radius: f32, strength: f32) -> Self {
        self.planet_radius = Some(radius.max(0.0));
        self.gravity_strength = strength.max(0.0);
        self.is_platform = false;
        self.collision_mode = CollisionMode::NonPlatform;
        self
    }

    pub fn elasticity(mut self, val: f32) -> Self { self.material.elasticity = val; self }
    pub fn friction(mut self, val: f32) -> Self { self.material.friction = val; self }
    pub fn density(mut self, val: f32) -> Self { self.material.density = val; self }
    pub fn bouncy(self) -> Self { self.material(PhysicsMaterial::bouncy()) }
    pub fn slippery(self) -> Self { self.material(PhysicsMaterial::ice()) }
    pub fn heavy(self) -> Self { self.material(PhysicsMaterial::metal()) }
    pub fn light(self) -> Self { self.material(PhysicsMaterial::feather()) }
    pub fn rubber(self) -> Self { self.material(PhysicsMaterial::rubber()) }
    pub fn static_object(self) -> Self { self.gravity(0.0).resistance(0.0, 0.0) }

    pub fn player_layer(self) -> Self {
        self.collision_layer(collision_layers::PLAYER).collision_mask(collision_layers::ALL)
    }
    pub fn enemy_layer(self) -> Self {
        self.collision_layer(collision_layers::ENEMY)
            .collision_mask(collision_layers::PLAYER | collision_layers::PROJECTILE | collision_layers::TERRAIN)
    }
    pub fn projectile_layer(self) -> Self {
        self.collision_layer(collision_layers::PROJECTILE)
            .collision_mask(collision_layers::ENEMY | collision_layers::TERRAIN)
    }
    pub fn no_collision(self) -> Self {
        self.collision_layer(collision_layers::NONE).collision_mask(collision_layers::NONE)
    }

    pub fn build(self, _ctx: &mut Context) -> GameObject { self.finish() }

    pub fn finish(self) -> GameObject {
        let size      = self.size;
        let highlight = self.highlight;
        let mut obj   = GameObject {
            layout:          prism::layout::Stack::default(),
            id:              self.id,
            tags:            self.tags,
            drawable:        self.image.map(|img| Box::new(img) as Box<dyn Drawable>),
            animated_sprite: None,
            size,
            position:        self.position,
            momentum:        self.momentum,
            resistance:      self.resistance,
            gravity:         self.gravity,
            scaled_size:     Cell::new(size),
            is_platform:     self.is_platform,
            visible:         true,
            layer:           self.layer,
            rotation:        self.rotation,
            slope:           self.slope,
            one_way:         self.one_way,
            surface_velocity: self.surface_velocity,
            rotation_momentum: 0.0,
            rotation_resistance: self.rotation_resistance,
            surface_normal:  self.surface_normal,
            collision_mode:  self.collision_mode,
            highlight:       None,
            glow_drawable:   None,
            tint_drawable:   None,
            grounded:        false,
            text_spec:       None,
            last_text_scale: Cell::new(0.0),
            image_path:      self.image_path,
            image_mtime:     self.image_mtime,
            animation_path:  None,
            animation_mtime: None,
            material:        self.material,
            collision_layer: self.collision_layer,
            collision_mask:  self.collision_mask,
            clipped:         self.clipped,
            clip_origin:     self.clip_origin,
            planet_radius:        self.planet_radius,
            gravity_target:       self.gravity_target.clone(),
            gravity_strength:     self.gravity_strength,
            auto_align:           self.auto_align,
            auto_align_speed:     self.auto_align_speed,
            auto_align_threshold: self.auto_align_threshold,
            ignore_zoom:          self.ignore_zoom,
        };
        if let Some(effect) = highlight { obj.set_highlight(effect); }
        obj
    }
}

impl GameObject {
    pub fn platform(id: impl Into<String>, w: f32, h: f32, pos: (f32, f32)) -> GameObjectBuilder {
        Self::build(id).size(w, h).position(pos.0, pos.1).platform().static_object()
            .collision_layer(collision_layers::TERRAIN)
    }

    pub fn trigger_zone(id: impl Into<String>, w: f32, h: f32, pos: (f32, f32)) -> GameObjectBuilder {
        Self::build(id).size(w, h).position(pos.0, pos.1).static_object()
            .no_collision().collision_layer(collision_layers::TRIGGER)
    }

    pub fn gravity_well(
        id: impl Into<String>, radius: f32, strength: f32,
        pos: (f32, f32), tag: impl Into<String>,
    ) -> GameObjectBuilder {
        Self::build(id)
            .size(radius * 2.0, radius * 2.0)
            .position(pos.0 - radius, pos.1 - radius)
            .tag(tag).gravity_well(radius, strength).static_object()
    }
}