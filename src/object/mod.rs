mod builder;
mod geometry;

pub use builder::GameObjectBuilder;

use prism::event::OnEvent;
use prism::drawable::{Drawable, Component};
use prism::Context;
use prism::canvas::{Image, ShapeType, Color};
use crate::text::TextSpec;
use crate::sprite::{AnimatedSprite, reload_image_raw, LAST_ASSET_PATH};
use crate::types::{CollisionMode, GlowConfig, HighlightEffect};
use crate::crystalline::PhysicsMaterial;
use std::cell::Cell;

/// Reads and clears the thread-local path set by `load_image` / `load_animation`,
/// and records the file's current mtime alongside it.
pub(super) fn capture_asset_path() -> (Option<String>, Option<std::time::SystemTime>) {
    let path = LAST_ASSET_PATH.with(|p| p.borrow_mut().take());
    let mtime = path.as_ref()
        .and_then(|p| std::fs::metadata(p).ok())
        .and_then(|m| m.modified().ok());
    (path, mtime)
}

#[derive(Clone, Debug)]
pub struct GameObject {
    layout:              prism::layout::Stack,
    pub id:              String,
    pub tags:            Vec<String>,
    pub(crate) drawable: Option<Box<dyn Drawable>>,
    pub animated_sprite: Option<AnimatedSprite>,
    pub size:            (f32, f32),
    pub position:        (f32, f32),
    pub momentum:        (f32, f32),
    pub resistance:      (f32, f32),
    pub gravity:         f32,
    pub scaled_size:     Cell<(f32, f32)>,
    pub is_platform:     bool,
    pub visible:         bool,
    pub layer:           Option<u32>,
    pub(crate) text_spec:       Option<TextSpec>,
    pub(crate) last_text_scale: Cell<f32>,
    // hot-reload tracking — populated automatically when load_image / load_animation is used
    pub(crate) image_path:      Option<String>,
    pub(crate) animation_path:  Option<String>,
    pub(crate) image_mtime:     Option<std::time::SystemTime>,
    pub(crate) animation_mtime: Option<std::time::SystemTime>,
    pub rotation:            f32,
    pub slope:               Option<(f32, f32)>,
    pub one_way:             bool,
    pub surface_velocity:    Option<f32>,
    pub rotation_momentum:   f32,
    pub rotation_resistance: f32,
    pub surface_normal:      (f32, f32),
    pub collision_mode:      CollisionMode,
    pub highlight:           Option<HighlightEffect>,
    pub(crate) glow_drawable:    Option<Box<dyn Drawable>>,
    pub(crate) tint_drawable:    Option<Box<dyn Drawable>>,
    pub grounded:            bool,
    pub material:            PhysicsMaterial,
    pub collision_layer:     u32,
    pub collision_mask:      u32,

    // ── Planet gravity fields ──────────────────────────────────────
    pub planet_radius:       Option<f32>,
    pub gravity_target:      Option<String>,
    pub gravity_strength:    f32,

    // ── Auto-align fields ──────────────────────────────────────────
    pub auto_align:           bool,
    pub auto_align_speed:     f32,
    pub auto_align_threshold: f32,
}

impl OnEvent for GameObject {}

impl Component for GameObject {
    fn children(&self) -> Vec<&dyn Drawable> {
        if self.visible {
            let mut result = Vec::new();
            if let Some(d) = &self.drawable {
                result.push(d.as_ref() as &dyn Drawable);
            }
            if let Some(glow) = &self.glow_drawable {
                result.push(glow.as_ref() as &dyn Drawable);
            }
            if let Some(tint) = &self.tint_drawable {
                result.push(tint.as_ref() as &dyn Drawable);
            }
            result
        } else {
            vec![]
        }
    }

    fn children_mut(&mut self) -> Vec<&mut dyn Drawable> {
        if self.visible {
            let mut result = Vec::new();
            if let Some(d) = &mut self.drawable {
                result.push(d.as_mut() as &mut dyn Drawable);
            }
            if let Some(glow) = &mut self.glow_drawable {
                result.push(glow.as_mut() as &mut dyn Drawable);
            }
            if let Some(tint) = &mut self.tint_drawable {
                result.push(tint.as_mut() as &mut dyn Drawable);
            }
            result
        } else {
            vec![]
        }
    }

    fn layout(&self) -> &dyn prism::layout::Layout {
        &self.layout
    }
}

impl GameObject {
    pub fn build(id: impl Into<String>) -> GameObjectBuilder {
        GameObjectBuilder {
            id:          id.into(),
            image:       None,
            image_path:  None,
            image_mtime: None,
            size:        (100.0, 100.0),
            position:    (0.0, 0.0),
            tags:        vec![],
            momentum:    (0.0, 0.0),
            resistance:  (1.0, 1.0),
            gravity:     0.0,
            is_platform: false,
            layer:       None,
            rotation:    0.0,
            slope:       None,
            one_way:     false,
            surface_velocity: None,
            rotation_momentum: 0.0,
            rotation_resistance: 0.85,
            surface_normal: (0.0, -1.0),
            collision_mode: CollisionMode::Surface,
            highlight:     None,
            material:      PhysicsMaterial::default(),
            collision_layer: 0,
            collision_mask:  u32::MAX,
            planet_radius:        None,
            gravity_target:       None,
            gravity_strength:     1.0,
            auto_align:           false,
            auto_align_speed:     3.0,
            auto_align_threshold: 45.0,
        }
    }

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
        let (image_path, image_mtime) = if drawable.is_some() { capture_asset_path() } else { (None, None) };
        Self {
            layout:          prism::layout::Stack::default(),
            id,
            tags,
            drawable:        drawable.map(|d| Box::new(d) as Box<dyn Drawable>),
            animated_sprite: None,
            size:            (size, size),
            position,
            momentum,
            resistance,
            gravity,
            scaled_size:     Cell::new((size, size)),
            is_platform:     false,
            visible:         true,
            layer:           None,
            rotation:            0.0,
            slope:               None,
            one_way:             false,
            surface_velocity:    None,
            rotation_momentum:   0.0,
            rotation_resistance: 0.85,
            surface_normal:      (0.0, -1.0),
            collision_mode:     CollisionMode::Surface,
            highlight:          None,
            glow_drawable:      None,
            tint_drawable:      None,
            grounded:           false,
            text_spec:       None,
            last_text_scale: Cell::new(0.0),
            image_path,
            image_mtime,
            animation_path:  None,
            animation_mtime: None,
            material:        PhysicsMaterial::default(),
            collision_layer: 0,
            collision_mask:  u32::MAX,
            planet_radius:        None,
            gravity_target:       None,
            gravity_strength:     1.0,
            auto_align:           false,
            auto_align_speed:     3.0,
            auto_align_threshold: 45.0,
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
        let (image_path, image_mtime) = if drawable.is_some() { capture_asset_path() } else { (None, None) };
        Self {
            layout:          prism::layout::Stack::default(),
            id,
            tags,
            drawable:        drawable.map(|d| Box::new(d) as Box<dyn Drawable>),
            animated_sprite: None,
            size,
            position,
            momentum,
            resistance,
            gravity,
            scaled_size:     Cell::new(size),
            is_platform:     false,
            visible:         true,
            layer:           None,
            rotation:            0.0,
            slope:               None,
            one_way:             false,
            surface_velocity:    None,
            rotation_momentum:   0.0,
            rotation_resistance: 0.85,
            surface_normal:      (0.0, -1.0),
            collision_mode:      CollisionMode::Surface,
            highlight:          None,
            glow_drawable:      None,
            tint_drawable:      None,
            grounded:           false,
            text_spec:       None,
            last_text_scale: Cell::new(0.0),
            image_path,
            image_mtime,
            animation_path:  None,
            animation_mtime: None,
            material:        PhysicsMaterial::default(),
            collision_layer: 0,
            collision_mask:  u32::MAX,
            planet_radius:        None,
            gravity_target:       None,
            gravity_strength:     1.0,
            auto_align:           false,
            auto_align_speed:     3.0,
            auto_align_threshold: 45.0,
        }
    }

    pub fn with_animation(mut self, animated_sprite: AnimatedSprite) -> Self {
        let (path, mtime) = capture_asset_path();
        if path.is_some() {
            self.animation_path  = path;
            self.animation_mtime = mtime;
        }
        self.animated_sprite = Some(animated_sprite);
        self
    }

    pub fn with_image(mut self, image: Image) -> Self {
        let (path, mtime) = capture_asset_path();
        if path.is_some() {
            self.image_path  = path;
            self.image_mtime = mtime;
        }
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
        let (path, mtime) = capture_asset_path();
        if path.is_some() {
            self.animation_path  = path;
            self.animation_mtime = mtime;
        }
        self.animated_sprite = Some(animated_sprite);
    }

    pub fn set_image(&mut self, image: Image) {
        let (path, mtime) = capture_asset_path();
        if path.is_some() {
            self.image_path  = path;
            self.image_mtime = mtime;
        }
        self.text_spec = None;
        self.last_text_scale.set(0.0);
        self.drawable  = Some(Box::new(image));
    }

    pub fn set_text(&mut self, spec: TextSpec) {
        let text = spec.build(1.0);
        self.last_text_scale.set(0.0);
        self.text_spec = Some(spec);
        self.drawable  = Some(Box::new(text));
    }

    pub fn set_drawable(&mut self, drawable: Box<dyn prism::drawable::Drawable>) {
        self.text_spec = None;
        self.last_text_scale.set(0.0);
        self.drawable  = Some(drawable);
    }

    pub fn update_position(&mut self) {
        self.position.0 += self.momentum.0;
        self.position.1 += self.momentum.1;
    }

    pub fn apply_gravity(&mut self) {
        if self.gravity_target.is_none() {
            self.momentum.1 += self.gravity;
        }
    }

    pub fn apply_resistance(&mut self) {
        self.momentum.0 *= self.resistance.0;
        self.momentum.1 *= self.resistance.1;

        if self.momentum.0.abs() < 0.001 { self.momentum.0 = 0.0; }
        if self.momentum.1.abs() < 0.001 { self.momentum.1 = 0.0; }
    }

    pub fn update_animation(&mut self, delta_time: f32) {
        if let Some(sprite) = &mut self.animated_sprite {
            sprite.update(delta_time);
            let mut img = sprite.get_current_image();
            let scaled = self.scaled_size.get();
            img.shape = ShapeType::Rectangle(0.0, scaled, self.rotation);
            self.drawable = Some(Box::new(img));
        }
    }

    pub fn update_image_shape(&mut self) {
        let scaled = self.scaled_size.get();
        let rotation = self.rotation;

        let rescale = |img: &mut Image, rot: f32| {
            img.shape = match img.shape {
                ShapeType::Rectangle(stroke, prev, _) => {
                    let sx = if prev.0.abs() > f32::EPSILON { scaled.0 / prev.0 } else { 1.0 };
                    let sy = if prev.1.abs() > f32::EPSILON { scaled.1 / prev.1 } else { 1.0 };
                    let s = sx.min(sy);
                    ShapeType::Rectangle(stroke * s, scaled, rot)
                }
                ShapeType::Ellipse(stroke, prev, _) => {
                    let sx = if prev.0.abs() > f32::EPSILON { scaled.0 / prev.0 } else { 1.0 };
                    let sy = if prev.1.abs() > f32::EPSILON { scaled.1 / prev.1 } else { 1.0 };
                    let s = sx.min(sy);
                    ShapeType::Ellipse(stroke * s, scaled, rot)
                }
                ShapeType::RoundedRectangle(stroke, prev, _, corner_radius) => {
                    let sx = if prev.0.abs() > f32::EPSILON { scaled.0 / prev.0 } else { 1.0 };
                    let sy = if prev.1.abs() > f32::EPSILON { scaled.1 / prev.1 } else { 1.0 };
                    let s = sx.min(sy);
                    ShapeType::RoundedRectangle(stroke * s, scaled, rot, corner_radius * s)
                }
            };
        };

        if let Some(drawable) = self.drawable.as_mut() {
            if let Some(ref mut img) = drawable.downcast_mut::<Image>() {
                rescale(img, rotation);
            }
        }
        if let Some(glow) = self.glow_drawable.as_mut() {
            if let Some(ref mut img) = glow.downcast_mut::<Image>() {
                rescale(img, rotation);
            }
        }
        if let Some(tint) = self.tint_drawable.as_mut() {
            if let Some(ref mut img) = tint.downcast_mut::<Image>() {
                rescale(img, rotation);
            }
        }
    }

    /// Detect the main drawable's shape and return a matching ShapeType
    /// with the given stroke width and target size.
    fn highlight_shape(&self, stroke: f32, size: (f32, f32)) -> ShapeType {
        if let Some(drawable) = &self.drawable {
            if let Some(img) = drawable.downcast_ref::<Image>() {
                return match img.shape {
                    ShapeType::Ellipse(_, _, _) =>
                        ShapeType::Ellipse(stroke, size, 0.0),
                    ShapeType::RoundedRectangle(_, _, _, cr) =>
                        ShapeType::RoundedRectangle(stroke, size, 0.0, cr),
                    _ =>
                        ShapeType::Rectangle(stroke, size, 0.0),
                };
            }
        }
        ShapeType::Rectangle(stroke, size, 0.0)
    }

    fn rebuild_highlight_drawables(&mut self) {
        let size = self.scaled_size.get();
        match &self.highlight {
            Some(effect) => {
                self.glow_drawable = effect.glow.as_ref().map(|glow| {
                    let pixel = image::RgbaImage::from_pixel(1, 1, image::Rgba([255,255,255,255])).into();
                    let img = Image {
                        shape: self.highlight_shape(glow.width, size),
                        image: pixel,
                        color: Some(glow.color),
                    };
                    Box::new(img) as Box<dyn Drawable>
                });
                self.tint_drawable = effect.tint.map(|color| {
                    let pixel: std::sync::Arc<image::RgbaImage> = image::RgbaImage::from_pixel(1, 1, image::Rgba([255,255,255,255])).into();
                    let img = Image {
                        shape: self.highlight_shape(0.0, size),
                        image: pixel,
                        color: Some(color),
                    };
                    Box::new(img) as Box<dyn Drawable>
                });
            }
            None => {
                self.glow_drawable = None;
                self.tint_drawable = None;
            }
        }

        self.update_image_shape();
    }

    pub fn set_glow(&mut self, config: GlowConfig) {
        let mut effect = self.highlight.take().unwrap_or_default();
        effect.glow = Some(config);
        self.highlight = Some(effect);
        self.rebuild_highlight_drawables();
    }

    pub fn clear_glow(&mut self) {
        if let Some(effect) = &mut self.highlight {
            effect.glow = None;
            if effect.tint.is_none() {
                self.highlight = None;
            }
        }
        self.rebuild_highlight_drawables();
    }

    pub fn set_tint(&mut self, color: Color) {
        let mut effect = self.highlight.take().unwrap_or_default();
        effect.tint = Some(color);
        self.highlight = Some(effect);
        self.rebuild_highlight_drawables();
    }

    pub fn clear_tint(&mut self) {
        if let Some(effect) = &mut self.highlight {
            effect.tint = None;
            if effect.glow.is_none() {
                self.highlight = None;
            }
        }
        self.rebuild_highlight_drawables();
    }

    pub fn set_highlight(&mut self, effect: HighlightEffect) {
        if effect.tint.is_none() && effect.glow.is_none() {
            self.highlight = None;
        } else {
            self.highlight = Some(effect);
        }
        self.rebuild_highlight_drawables();
    }

    pub fn clear_highlight(&mut self) {
        self.highlight = None;
        self.rebuild_highlight_drawables();
    }

    pub(crate) fn update_text_scale(&mut self, scale: f32) {
        if self.text_spec.is_none() { return; }
        if (self.last_text_scale.get() - scale).abs() < 0.0001 { return; }

        if let Some(spec) = &mut self.text_spec {
            let text = spec.build(scale);
            self.drawable = Some(Box::new(text));
        }
        self.last_text_scale.set(scale);
    }

    // ── Hot-reload internals ─────────────────────────────────────────────────

    pub(crate) fn hot_reload_image(&mut self, path: &str) {
        let Ok(meta)  = std::fs::metadata(path) else { return };
        let Ok(mtime) = meta.modified()          else { return };
        if Some(mtime) == self.image_mtime { return; }

        let img = reload_image_raw(path, self.size);
        self.image_mtime = Some(mtime);
        self.text_spec   = None;
        self.last_text_scale.set(0.0);
        self.drawable    = Some(Box::new(img));
        println!("[hot-reload] image reloaded: {path}");
    }

    pub(crate) fn hot_reload_animation(&mut self, path: &str) {
        let Ok(meta)  = std::fs::metadata(path) else { return };
        let Ok(mtime) = meta.modified()          else { return };
        if Some(mtime) == self.animation_mtime { return; }

        let fps  = self.animated_sprite.as_ref().map(|s| s.fps()).unwrap_or(12.0);
        let size = self.size;
        match std::fs::read(path) {
            Ok(bytes) => match AnimatedSprite::decode_vec(bytes, size, fps) {
                Ok(sprite) => {
                    self.animated_sprite   = Some(sprite);
                    self.animation_mtime   = Some(mtime);
                    println!("[hot-reload] animation reloaded: {path}");
                }
                Err(e) => eprintln!("[hot-reload] failed to read '{path}': {e}"),
            },
            Err(e) => eprintln!("[hot-reload] failed to open '{path}': {e}"),
        }
    }
}
