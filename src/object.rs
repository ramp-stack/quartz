use prism::event::OnEvent;
use prism::drawable::{Drawable, Component, SizedTree};
use prism::Context;
use prism::layout::{Area, SizeRequest};
use prism::canvas::{Image, ShapeType, Color};
use crate::text::TextSpec;
use crate::sprite::AnimatedSprite;
use crate::types::{CollisionMode, Anchor, GlowConfig, HighlightEffect};
use std::cell::Cell;

#[derive(Clone, Debug)]
pub struct GameObject {
    layout:           prism::layout::Stack,
    pub id:           String,
    pub tags:         Vec<String>,
    drawable:         Option<Box<dyn Drawable>>,
    pub animated_sprite: Option<AnimatedSprite>,
    pub size:         (f32, f32),
    pub position:     (f32, f32),
    pub momentum:     (f32, f32),
    pub resistance:   (f32, f32),
    pub gravity:      f32,
    pub scaled_size:  Cell<(f32, f32)>,
    pub is_platform:  bool,
    pub visible:      bool,
    pub layer:        Option<u32>,
    pub rotation:            f32,
    pub slope:               Option<(f32, f32)>,
    pub one_way:             bool,
    pub surface_velocity:    Option<f32>,
    pub rotation_momentum:   f32,
    pub rotation_resistance: f32,
    pub surface_normal:      (f32, f32),
    pub collision_mode:      CollisionMode,
    pub highlight:    Option<HighlightEffect>,
    glow_drawable: Option<Box<dyn Drawable>>,
    tint_drawable: Option<Box<dyn Drawable>>,
    pub grounded:            bool,
    text_spec:        Option<TextSpec>,
    last_text_scale:  Cell<f32>, 
}

impl OnEvent for GameObject {}

impl Component for GameObject {
    fn children(&self) -> Vec<&dyn Drawable> {
        if self.visible {
            let mut result = Vec::new();
            // Main drawable
            if let Some(d) = &self.drawable {
                result.push(d.as_ref() as &dyn Drawable);
            }
            // Glow ring renders on top of main drawable
            if let Some(glow) = &self.glow_drawable {
                result.push(glow.as_ref() as &dyn Drawable);
            }
            // Tint overlay renders on top of everything
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
        self.momentum.1 += self.gravity;
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

        // Keep all image-based drawables synchronized to the current screen scale
        // in the same frame they are rebuilt to avoid one-frame oversize flashes.
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

        if let Some(spec) = &self.text_spec {
            let text = spec.build(scale);
            self.drawable = Some(Box::new(text));
        }
        self.last_text_scale.set(scale);
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

    pub fn apply_rotation_momentum(&mut self) {
        if self.rotation_momentum == 0.0 { return; }
        self.rotation += self.rotation_momentum;
        self.rotation_momentum *= self.rotation_resistance;
        if self.rotation_momentum.abs() < 0.01 {
            self.rotation_momentum = 0.0;
        }
        if self.is_platform {
            self.sync_rotation_normal();
        }
    }

    pub fn sync_rotation_normal(&mut self) {
        let theta = self.rotation.to_radians();
        self.surface_normal = (theta.sin(), -theta.cos());
    }

    pub fn slope_surface_y(&self, world_x: f32) -> f32 {
        match self.slope {
            None => self.position.1,
            Some((left_offset, right_offset)) => {
                if self.size.0 == 0.0 { return self.position.1; }
                let t = ((world_x - self.position.0) / self.size.0).clamp(0.0, 1.0);
                self.position.1 + left_offset + (right_offset - left_offset) * t
            }
        }
    }

    pub fn rotation_from_slope(&self) -> f32 {
        match self.slope {
            None => 0.0,
            Some((left_offset, right_offset)) => {
                (right_offset - left_offset).atan2(self.size.0).to_degrees()
            }
        }
    }

    pub fn surface_normal_at(&self, _world_x: f32) -> (f32, f32) {
        match self.slope {
            None => self.surface_normal,
            Some((left_offset, right_offset)) => {
                let w = self.size.0;
                if w < 0.01 { return (0.0, -1.0); }
                let rise = right_offset - left_offset;
                let len  = (rise * rise + w * w).sqrt();
                (rise / len, -w / len)
            }
        }
    }

    pub fn slope_aabb(&self) -> (f32, f32, f32, f32) {
        match self.slope {
            None => (self.position.0, self.position.1, self.size.0, self.size.1),
            Some((left_off, right_off)) => {
                let left_y = self.position.1 + left_off;
                let right_y = self.position.1 + right_off;
                let top = left_y.min(right_y);
                let bottom = left_y.max(right_y) + self.size.1;
                (self.position.0, top, self.size.0, bottom - top)
            }
        }
    }
}

pub struct GameObjectBuilder {
    id:          String,
    image:       Option<Image>,
    size:        (f32, f32),
    position:    (f32, f32),
    tags:        Vec<String>,
    momentum:    (f32, f32),
    resistance:  (f32, f32),
    gravity:     f32,
    is_platform: bool,
    pub layer:   Option<u32>,
    rotation:    f32,
    slope:       Option<(f32, f32)>,
    one_way:     bool,
    surface_velocity: Option<f32>,
    pub rotation_momentum: f32,
    pub rotation_resistance: f32,
    surface_normal: (f32, f32),
    collision_mode: CollisionMode,
    highlight: Option<HighlightEffect>,
}

impl GameObjectBuilder {
    pub fn layer(mut self, id: u32) -> Self {
        self.layer = Some(id);
        self
    }

    pub fn image(mut self, image: Image) -> Self {
        self.image = Some(image);
        self
    }

    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = (w, h);
        self
    }

    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.position = (x, y);
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn momentum(mut self, x: f32, y: f32) -> Self {
        self.momentum = (x, y);
        self
    }

    pub fn resistance(mut self, x: f32, y: f32) -> Self {
        self.resistance = (x, y);
        self
    }

    pub fn gravity(mut self, g: f32) -> Self {
        self.gravity = g;
        self
    }

    pub fn platform(mut self) -> Self {
        self.is_platform    = true;
        self.surface_normal = (0.0, -1.0);
        self
    }

    pub fn floor(mut self) -> Self {
        self.platform()
    }

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

    pub fn rotation(mut self, degrees: f32) -> Self {
        self.rotation = degrees;
        self
    }

    pub fn slope(mut self, left_offset: f32, right_offset: f32) -> Self {
        self.slope = Some((left_offset, right_offset));
        self
    }

    pub fn slope_auto_rotation(mut self, left_offset: f32, right_offset: f32) -> Self {
        self.slope = Some((left_offset, right_offset));
        if self.size.0 != 0.0 {
            self.rotation = (right_offset - left_offset).atan2(self.size.0).to_degrees();
        }
        self
    }

    pub fn one_way(mut self) -> Self {
        self.one_way = true;
        self
    }

    pub fn surface_velocity(mut self, vx: f32) -> Self {
        self.surface_velocity = Some(vx);
        self
    }

    pub fn rotation_resistance(mut self, resistance: f32) -> Self {
        self.rotation_resistance = resistance.clamp(0.0, 1.0);
        self
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

    pub fn highlight(mut self, effect: HighlightEffect) -> Self {
        self.highlight = Some(effect);
        self
    }

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

    pub fn build(self, _ctx: &mut Context) -> GameObject {
        self.finish()
    }

    pub fn finish(self) -> GameObject {
        let size = self.size;
        let highlight = self.highlight;
        let mut obj = GameObject {
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
        };
        if let Some(effect) = highlight {
            obj.set_highlight(effect);
        }
        obj
    }
}