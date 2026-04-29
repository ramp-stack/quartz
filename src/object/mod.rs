mod builder;
mod geometry;

pub use builder::GameObjectBuilder;

use prism::event::{OnEvent, Event};
use prism::drawable::{Drawable, SizedTree, RequestTree, Offset, Rect, Size};
use prism::layout::{SizeRequest, Area};
use prism::Context;
use prism::canvas::{Image, ShapeType, Color};
use crate::sprite::{AnimatedSprite, reload_image_raw, LAST_ASSET_PATH};
use crate::types::{CollisionMode, GlowConfig, GravityFalloff, HighlightEffect};
use crate::crystalline::PhysicsMaterial;
use wgpu_canvas::{Area as CanvasArea, Item as CanvasItem};
use std::cell::Cell;

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
    pub render_scale:    Cell<f32>,
    pub is_platform:     bool,
    pub visible:         bool,
    pub layer:           i32,
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
    pub ped:                 bool,
    pub _origin:             Option<(f32, f32)>,
    pub _size:               Option<(f32, f32)>,
    pub planet_radius:       Option<f32>,
    pub gravity_target:      Option<String>,
    pub gravity_strength:    f32,
    pub gravity_influence_mult: f32,
    pub gravity_falloff:     GravityFalloff,
    pub gravity_all_sources: bool,
    pub gravity_dominant_id: Option<String>,
    pub gravity_identity:    Option<String>,
    pub auto_align:          bool,
    pub auto_align_speed:    f32,
    pub auto_align_threshold: f32,
    pub auto_align_min_depth: f32,
    pub ignore_zoom:         bool,
}

impl OnEvent for GameObject {}

impl GameObject {
    fn active_children(&self) -> Vec<&dyn Drawable> {
        if !self.visible { return vec![]; }
        let mut v: Vec<&dyn Drawable> = Vec::new();
        if let Some(d) = &self.drawable      { v.push(d.as_ref()); }
        if let Some(g) = &self.glow_drawable  { v.push(g.as_ref()); }
        if let Some(t) = &self.tint_drawable  { v.push(t.as_ref()); }
        v
    }

    fn active_children_mut(&mut self) -> Vec<&mut dyn Drawable> {
        if !self.visible { return vec![]; }
        let mut v: Vec<&mut dyn Drawable> = Vec::new();
        if let Some(d) = &mut self.drawable      { v.push(d.as_mut()); }
        if let Some(g) = &mut self.glow_drawable  { v.push(g.as_mut()); }
        if let Some(t) = &mut self.tint_drawable  { v.push(t.as_mut()); }
        v
    }

    fn reported_size(&self) -> Size {
        if self.ped { self._size.unwrap_or(self.size) } else { self.size }
    }

    fn clip_rect(&self, poffset: Offset) -> Rect {
        let s = self.render_scale.get();
        let (cx, cy) = self._origin
            .map(|(x, y)| (x * s, y * s))
            .unwrap_or(poffset);
        let (cw, ch) = self._size
            .map(|(w, h)| (w * s, h * s))
            .unwrap_or(self.size);
        // return as (x, y, w, h) to match renderer expectation
        (cx, cy, cw, ch)
    }
}

impl Drawable for GameObject {
    fn request_size(&self) -> RequestTree {
        let child_requests = self.active_children()
            .into_iter()
            .map(Drawable::request_size)
            .collect();
        RequestTree(SizeRequest::fixed(self.reported_size()), child_requests)
    }

    fn build(&self, size: Size, request: RequestTree) -> SizedTree {
        let own_size   = request.0.get(size);
        let child_size = self.size;
        SizedTree(
            own_size,
            self.active_children()
                .into_iter()
                .zip(request.1)
                .map(|(child, branch)| {
                    let built = child.build(child_size, branch);
                    ((0.0_f32, 0.0_f32), built)
                })
                .collect(),
        )
    }

    fn draw(
        &self,
        sized:   &SizedTree,
        poffset: Offset,
        bound:   Rect,
    ) -> Vec<(CanvasArea, CanvasItem)> {
        if !self.visible { return vec![]; }

        let bound = if self.ped {
            let cr = self.clip_rect(poffset);
            // both bound and cr are (x, y, w, h)
            let x0 = bound.0.max(cr.0);
            let y0 = bound.1.max(cr.1);
            let x1 = (bound.0 + bound.2).min(cr.0 + cr.2);
            let y1 = (bound.1 + bound.3).min(cr.1 + cr.3);
            if x1 <= x0 || y1 <= y0 { return vec![]; }
            (x0, y0, x1 - x0, y1 - y0)
        } else {
            bound
        };

        sized.1.iter()
            .zip(self.active_children())
            .flat_map(|((offset, branch), child)| {
                let child_offset = (poffset.0 + offset.0, poffset.1 + offset.1);
                child.draw(branch, child_offset, bound)
            })
            .collect()
    }

    fn event(&mut self, ctx: &mut Context, sized: &SizedTree, event: Box<dyn Event>) {
        let areas: Vec<Area> = sized.1.iter()
            .map(|(o, branch)| Area { offset: *o, size: branch.0 })
            .collect();
        let events = OnEvent::on_event(self, ctx, sized, event);
        for ev in events {
            for (slot, (child, (_, branch))) in
                ev.pass(ctx, &areas)
                    .into_iter()
                    .zip(self.active_children_mut().into_iter().zip(sized.1.iter()))
            {
                if let Some(e) = slot { child.event(ctx, branch, e); }
            }
        }
    }
}

impl GameObject {
    pub fn build(id: impl Into<String>) -> GameObjectBuilder {
        GameObjectBuilder {
            id: id.into(), image: None, image_path: None, image_mtime: None,
            size: (100.0, 100.0), position: (0.0, 0.0), tags: vec![],
            momentum: (0.0, 0.0), resistance: (1.0, 1.0), gravity: 0.0,
            is_platform: false, layer: 0, rotation: 0.0, slope: None,
            one_way: false, surface_velocity: None, rotation_momentum: 0.0,
            rotation_resistance: 0.85, surface_normal: (0.0, -1.0),
            collision_mode: CollisionMode::Surface, highlight: None,
            material: PhysicsMaterial::default(), collision_layer: 0,
            collision_mask: u32::MAX, clipped: false, clip_origin: None, clip_size: None,
            planet_radius: None, gravity_target: None, gravity_strength: 1.0,
            gravity_influence_mult: 3.0, gravity_falloff: GravityFalloff::default(),
            gravity_all_sources: false, gravity_identity: None,
            auto_align: false, auto_align_speed: 3.0, auto_align_threshold: 45.0,
            auto_align_min_depth: 0.3,
            ignore_zoom: false,
        }
    }

    fn default_fields(
        size: (f32, f32),
        image_path:  Option<String>,
        image_mtime: Option<std::time::SystemTime>,
    ) -> Self {
        Self {
            layout: prism::layout::Stack::default(),
            id: String::new(), tags: vec![], drawable: None, animated_sprite: None,
            size, position: (0.0, 0.0), momentum: (0.0, 0.0),
            resistance: (1.0, 1.0), gravity: 0.0,
            scaled_size: Cell::new(size),
            render_scale: Cell::new(1.0),
            is_platform: false, visible: true, layer: 0,
            rotation: 0.0, slope: None, one_way: false, surface_velocity: None,
            rotation_momentum: 0.0, rotation_resistance: 0.85,
            surface_normal: (0.0, -1.0), collision_mode: CollisionMode::Surface,
            highlight: None, glow_drawable: None, tint_drawable: None, grounded: false,
            image_path, image_mtime, animation_path: None, animation_mtime: None,
            material: PhysicsMaterial::default(), collision_layer: 0,
            collision_mask: u32::MAX, ped: false, _origin: None, _size: None,
            planet_radius: None, gravity_target: None, gravity_strength: 1.0,
            gravity_influence_mult: 3.0, gravity_falloff: GravityFalloff::default(),
            gravity_all_sources: false, gravity_dominant_id: None,
            gravity_identity: None,
            auto_align: false, auto_align_speed: 3.0, auto_align_threshold: 45.0,
            auto_align_min_depth: 0.3,
            ignore_zoom: false,
        }
    }

    pub fn new(
        _ctx: &mut Context, id: String, drawable: Option<impl Drawable + 'static>,
        size: f32, position: (f32, f32), tags: Vec<String>,
        momentum: (f32, f32), resistance: (f32, f32), gravity: f32,
    ) -> Self {
        let (image_path, image_mtime) = if drawable.is_some() { capture_asset_path() } else { (None, None) };
        let mut s = Self::default_fields((size, size), image_path, image_mtime);
        s.id = id; s.tags = tags; s.position = position;
        s.momentum = momentum; s.resistance = resistance; s.gravity = gravity;
        s.drawable = drawable.map(|d| Box::new(d) as Box<dyn Drawable>);
        s
    }

    pub fn new_rect(
        _ctx: &mut Context, id: String, drawable: Option<impl Drawable + 'static>,
        size: (f32, f32), position: (f32, f32), tags: Vec<String>,
        momentum: (f32, f32), resistance: (f32, f32), gravity: f32,
    ) -> Self {
        let (image_path, image_mtime) = if drawable.is_some() { capture_asset_path() } else { (None, None) };
        let mut s = Self::default_fields(size, image_path, image_mtime);
        s.id = id; s.tags = tags; s.position = position;
        s.momentum = momentum; s.resistance = resistance; s.gravity = gravity;
        s.drawable = drawable.map(|d| Box::new(d) as Box<dyn Drawable>);
        s
    }

    pub fn with_animation(mut self, animated_sprite: AnimatedSprite) -> Self {
        let (path, mtime) = capture_asset_path();
        if path.is_some() { self.animation_path = path; self.animation_mtime = mtime; }
        self.animated_sprite = Some(animated_sprite);
        self
    }

    pub fn with_image(mut self, image: Image) -> Self {
        let (path, mtime) = capture_asset_path();
        if path.is_some() { self.image_path = path; self.image_mtime = mtime; }
        self.drawable = Some(Box::new(image));
        self
    }

    pub fn as_platform(mut self)                              -> Self { self.is_platform = true; self }
    pub fn with_tag(mut self, tag: impl Into<String>)         -> Self { self.tags.push(tag.into()); self }
    pub fn with_tags(mut self, tags: Vec<String>)             -> Self { self.tags = tags; self }
    pub fn with_gravity(mut self, gravity: f32)               -> Self { self.gravity = gravity; self }
    pub fn with_momentum(mut self, momentum: (f32, f32))      -> Self { self.momentum = momentum; self }
    pub fn with_resistance(mut self, resistance: (f32, f32))  -> Self { self.resistance = resistance; self }
    pub fn clip(mut self)                                      -> Self { self.ped = true; self }

    pub fn set_gravity(&mut self, gravity: f32) { self.gravity = gravity; }

    pub fn set_animation(&mut self, animated_sprite: AnimatedSprite) {
        let (path, mtime) = capture_asset_path();
        if path.is_some() { self.animation_path = path; self.animation_mtime = mtime; }
        self.animated_sprite = Some(animated_sprite);
    }

    pub fn set_image(&mut self, image: Image) {
        let (path, mtime) = capture_asset_path();
        if path.is_some() { self.image_path = path; self.image_mtime = mtime; }
        self.drawable = Some(Box::new(image));
    }

    pub fn set_drawable(&mut self, drawable: Box<dyn prism::drawable::Drawable>) {
        self.drawable = Some(drawable);
    }

    pub fn set_clip(&mut self, clip: bool)                          { self.ped     = clip; }
    pub fn set_clip_origin(&mut self, origin: Option<(f32, f32)>)  { self._origin = origin; }
    pub fn set_clip_size(&mut self, size: Option<(f32, f32)>)      { self._size   = size; }

    pub fn update_position(&mut self) {
        self.position.0 += self.momentum.0;
        self.position.1 += self.momentum.1;
    }

    pub fn apply_gravity(&mut self) {
        if self.gravity_target.is_none() { self.momentum.1 += self.gravity; }
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
        let scaled   = self.scaled_size.get();
        let rotation = self.rotation;
        let rescale = |img: &mut Image, rot: f32| {
            img.shape = match img.shape {
                ShapeType::Rectangle(stroke, prev, _) => {
                    let s = (scaled.0 / prev.0.max(f32::EPSILON)).min(scaled.1 / prev.1.max(f32::EPSILON));
                    ShapeType::Rectangle(stroke * s, scaled, rot)
                }
                ShapeType::Ellipse(stroke, prev, _) => {
                    let s = (scaled.0 / prev.0.max(f32::EPSILON)).min(scaled.1 / prev.1.max(f32::EPSILON));
                    ShapeType::Ellipse(stroke * s, scaled, rot)
                }
                ShapeType::RoundedRectangle(stroke, prev, _, cr) => {
                    let s = (scaled.0 / prev.0.max(f32::EPSILON)).min(scaled.1 / prev.1.max(f32::EPSILON));
                    ShapeType::RoundedRectangle(stroke * s, scaled, rot, cr * s)
                }
            };
        };
        if let Some(d) = self.drawable.as_mut()      { if let Some(i) = d.downcast_mut::<Image>() { rescale(i, rotation); } }
        if let Some(d) = self.glow_drawable.as_mut() { if let Some(i) = d.downcast_mut::<Image>() { rescale(i, rotation); } }
        if let Some(d) = self.tint_drawable.as_mut() { if let Some(i) = d.downcast_mut::<Image>() { rescale(i, rotation); } }
    }

    fn highlight_shape(&self, stroke: f32, size: (f32, f32)) -> ShapeType {
        if let Some(d) = &self.drawable {
            if let Some(img) = d.downcast_ref::<Image>() {
                return match img.shape {
                    ShapeType::Ellipse(..)              => ShapeType::Ellipse(stroke, size, 0.0),
                    ShapeType::RoundedRectangle(.., cr) => ShapeType::RoundedRectangle(stroke, size, 0.0, cr),
                    _                                   => ShapeType::Rectangle(stroke, size, 0.0),
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
                    let pixel = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255])).into();
                    Box::new(Image {
                        shape: self.highlight_shape(glow.width, size),
                        image: pixel,
                        color: Some(glow.color),
                    }) as Box<dyn Drawable>
                });
                self.tint_drawable = effect.tint.map(|color| {
                    let pixel: std::sync::Arc<image::RgbaImage> =
                        image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255])).into();
                    Box::new(Image {
                        shape: self.highlight_shape(0.0, size),
                        image: pixel,
                        color: Some(color),
                    }) as Box<dyn Drawable>
                });
            }
            None => { self.glow_drawable = None; self.tint_drawable = None; }
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
        if let Some(e) = &mut self.highlight { e.glow = None; if e.tint.is_none() { self.highlight = None; } }
        self.rebuild_highlight_drawables();
    }
    pub fn set_tint(&mut self, color: Color) {
        let mut effect = self.highlight.take().unwrap_or_default();
        effect.tint = Some(color);
        self.highlight = Some(effect);
        self.rebuild_highlight_drawables();
    }
    pub fn clear_tint(&mut self) {
        if let Some(e) = &mut self.highlight { e.tint = None; if e.glow.is_none() { self.highlight = None; } }
        self.rebuild_highlight_drawables();
    }
    pub fn set_highlight(&mut self, effect: HighlightEffect) {
        if effect.tint.is_none() && effect.glow.is_none() { self.highlight = None; }
        else { self.highlight = Some(effect); }
        self.rebuild_highlight_drawables();
    }
    pub fn clear_highlight(&mut self) {
        self.highlight = None;
        self.rebuild_highlight_drawables();
    }

    pub(crate) fn hot_reload_image(&mut self, path: &str) {
        let Ok(meta)  = std::fs::metadata(path) else { return };
        let Ok(mtime) = meta.modified()          else { return };
        if Some(mtime) == self.image_mtime { return; }
        let img = reload_image_raw(path, self.size);
        self.image_mtime = Some(mtime);
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
                    self.animated_sprite  = Some(sprite);
                    self.animation_mtime  = Some(mtime);
                    println!("[hot-reload] animation reloaded: {path}");
                }
                Err(e) => eprintln!("[hot-reload] failed to read '{path}': {e}"),
            },
            Err(e) => eprintln!("[hot-reload] failed to open '{path}': {e}"),
        }
    }
}