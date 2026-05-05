use prism::canvas::{Image, ShapeType, Color};
use image::{RgbaImage, Rgba, AnimationDecoder, imageops};
use std::io::Cursor;
use prism::drawable::{Drawable, SizedTree, Rect};
use std::cell::RefCell;


pub fn solid_circle(size: f32, color: Color) -> Image {
    Image {
        shape: ShapeType::RoundedRectangle(0.0, (size, size), 0.0, size * 0.5),
        image: RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255])).into(),
        color: Some(color),
    }
}

pub fn solid_ellipse(w: f32, h: f32, color: Color) -> Image {
    Image {
        shape: ShapeType::Ellipse(0.0, (w, h), 0.0),
        image: RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255])).into(),
        color: Some(color),
    }
}

pub fn planet_image(radius: u32, r: u8, g: u8, b: u8, size: f32) -> Image {
    Image {
        shape: ShapeType::Rectangle(0.0, (size, size), 0.0),
        image: generate_planet_rgba(radius, r, g, b, 1.0).into(),
        color: None,
    }
}

pub fn planet_grayscale(radius: u32, size: f32) -> Image {
    Image {
        shape: ShapeType::Rectangle(0.0, (size, size), 0.0),
        image: generate_planet_rgba(radius, 255, 255, 255, 1.0).into(),
        color: None,
    }
}

pub fn with_tint(image: &Image, color: Color) -> Image {
    Image {
        shape: image.shape.clone(),
        image: image.image.clone(),
        color: Some(color),
    }
}

pub fn planet_atmosphere(radius: u32, r: u8, g: u8, b: u8, atmosphere: f32, size: f32) -> Image {
    let rf = radius as f32;
    let atm_px = rf * atmosphere.clamp(0.0, 1.0);
    let outer_r = rf + atm_px;
    let diameter = (outer_r * 2.0).ceil().max(1.0) as u32;
    let mut img = RgbaImage::new(diameter, diameter);
    let cx = outer_r;

    for py in 0..diameter {
        for px in 0..diameter {
            let dx = px as f32 - cx + 0.5;
            let dy = py as f32 - cx + 0.5;
            let dist = (dx * dx + dy * dy).sqrt();

            let (alpha, brightness) = if dist <= rf {
                let rim = ((rf - dist) / rf).min(1.0);
                (255u8, 0.7 + 0.3 * rim)
            } else if atm_px > 0.0 && dist <= rf + atm_px {
                let t = (dist - rf) / atm_px;
                let alpha = ((1.0 - t) * 180.0) as u8;
                (alpha, 0.6 + 0.15 * (1.0 - t))
            } else {
                continue;
            };

            img.put_pixel(px, py, Rgba([
                (r as f32 * brightness).min(255.0) as u8,
                (g as f32 * brightness).min(255.0) as u8,
                (b as f32 * brightness).min(255.0) as u8,
                alpha,
            ]));
        }
    }

    Image {
        shape: ShapeType::Rectangle(0.0, (size, size), 0.0),
        image: img.into(),
        color: None,
    }
}

pub fn glow_ring(w: f32, h: f32, ring_width: f32, corner_radius: f32, color: Color) -> Image {
    let total_w = w + 2.0 * ring_width;
    let total_h = h + 2.0 * ring_width;
    Image {
        shape: ShapeType::RoundedRectangle(
            ring_width,
            (total_w, total_h),
            0.0,
            corner_radius + ring_width * 0.5,
        ),
        image: RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255])).into(),
        color: Some(color),
    }
}

pub fn tint_overlay(w: f32, h: f32, color: Color) -> Image {
    Image {
        shape: ShapeType::Rectangle(0.0, (w, h), 0.0),
        image: RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255])).into(),
        color: Some(color),
    }
}

pub(crate) fn generate_planet_rgba(radius: u32, r: u8, g: u8, b: u8, brightness_scale: f32) -> RgbaImage {
    let diameter = radius * 2;
    let mut img = RgbaImage::new(diameter, diameter);
    let cx = radius as f32;
    let rf = radius as f32;

    for py in 0..diameter {
        for px in 0..diameter {
            let dx = px as f32 - cx + 0.5;
            let dy = py as f32 - cx + 0.5;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > rf { continue; }

            let rim = ((rf - dist) / rf).min(1.0);
            let brightness = (0.7 + 0.3 * rim) * brightness_scale;

            img.put_pixel(px, py, Rgba([
                (r as f32 * brightness).min(255.0) as u8,
                (g as f32 * brightness).min(255.0) as u8,
                (b as f32 * brightness).min(255.0) as u8,
                255,
            ]));
        }
    }

    img
}

pub fn load_image(bytes: &[u8]) -> Image {
    let rgba = image::load_from_memory(bytes)
        .expect("quartz: cannot decode image from bytes")
        .into_rgba8();
    let (w, h) = (rgba.width() as f32, rgba.height() as f32);
    make_image(rgba, w, h)
}

pub fn load_image_sized(bytes: &[u8], w: f32, h: f32) -> Image {
    let rgba = image::load_from_memory(bytes)
        .expect("quartz: cannot decode image from bytes")
        .into_rgba8();
    make_image(rgba, w, h)
}

pub fn load_animation(bytes: &[u8], size: (f32, f32), fps: f32) -> AnimatedSprite {
    AnimatedSprite::decode_vec(bytes.to_vec(), size, fps)
        .expect("quartz: failed to decode animation from bytes")
}

pub fn flip_horizontal(img: Image) -> Image {
    let (pixels, w, h) = extract(img);
    let flipped = imageops::flip_horizontal(&pixels);
    make_image(flipped, w, h)
}

pub fn flip_vertical(img: Image) -> Image {
    let (pixels, w, h) = extract(img);
    let flipped = imageops::flip_vertical(&pixels);
    make_image(flipped, w, h)
}

pub fn rotate_cw(img: Image) -> Image {
    let (pixels, w, h) = extract(img);
    let rotated = imageops::rotate270(&pixels);
    make_image(rotated, h, w)
}

pub fn rotate_ccw(img: Image) -> Image {
    let (pixels, w, h) = extract(img);
    let rotated = imageops::rotate90(&pixels);
    make_image(rotated, h, w)
}

pub fn rotate_180(img: Image) -> Image {
    let (pixels, w, h) = extract(img);
    let rotated = imageops::rotate180(&pixels);
    make_image(rotated, w, h)
}

fn extract(img: Image) -> (RgbaImage, f32, f32) {
    let (w, h) = match img.shape {
        ShapeType::Rectangle(_, size, _) => size,
        _ => panic!("image transform: expected a Rectangle shape"),
    };
    let pixels: RgbaImage = (*img.image).clone();
    (pixels, w, h)
}

pub(crate) fn make_image(pixels: RgbaImage, w: f32, h: f32) -> Image {
    Image {
        shape: ShapeType::Rectangle(0.0, (w, h), 0.0),
        image: pixels.into(),
        color: None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationDirection {
    Clockwise,
    CounterClockwise,
}

#[derive(Debug, Clone, Copy)]
pub struct RotationOptions {
    pub degrees:   f32,
    pub direction: RotationDirection,
}

impl RotationOptions {
    pub fn clockwise(degrees: f32) -> Self {
        Self { degrees, direction: RotationDirection::Clockwise }
    }
    pub fn counter_clockwise(degrees: f32) -> Self {
        Self { degrees, direction: RotationDirection::CounterClockwise }
    }
    pub fn degrees(degrees: f32) -> Self {
        Self { degrees, direction: RotationDirection::Clockwise }
    }
    pub(crate) fn to_radians(self) -> f32 {
        let r = self.degrees.to_radians();
        match self.direction {
            RotationDirection::Clockwise        =>  r,
            RotationDirection::CounterClockwise => -r,
        }
    }
}

impl Default for RotationOptions {
    fn default() -> Self {
        Self { degrees: 0.0, direction: RotationDirection::Clockwise }
    }
}

#[derive(Clone)]
pub struct AnimatedSprite {
    frames:                Vec<RgbaImage>,
    current_frame:         usize,
    frame_duration:        f32,
    time_since_last_frame: f32,
    size:                  (f32, f32),
    mirrored_h:            bool,
    mirrored_v:            bool,
    rotation:              RotationOptions,
}

impl AnimatedSprite {
    pub fn new(gif_bytes: &[u8], size: (f32, f32), fps: f32) -> Result<Self, String> {
        Self::decode_slice(gif_bytes, size, fps)
    }

    pub(crate) fn decode_vec(bytes: Vec<u8>, size: (f32, f32), fps: f32) -> Result<Self, String> {
        Self::decode_slice(&bytes, size, fps)
    }

    fn decode_slice(bytes: &[u8], size: (f32, f32), fps: f32) -> Result<Self, String> {
        let cursor  = Cursor::new(bytes);
        let decoder = image::codecs::gif::GifDecoder::new(cursor)
            .map_err(|e| format!("Failed to decode GIF: {}", e))?;
        let mut frames = Vec::new();
        for frame_result in decoder.into_frames() {
            let frame = frame_result
                .map_err(|e| format!("Failed to decode frame: {}", e))?;
            frames.push(frame.into_buffer());
        }
        if frames.is_empty() {
            return Err("GIF has no frames".to_string());
        }

        let tw = size.0.round().max(1.0) as u32;
        let th = size.1.round().max(1.0) as u32;
        frames = frames.into_iter().map(|f| {
            let fw = f.width();
            let fh = f.height();
            if fw == tw && fh == th { return f; }

            let scale = (tw as f32 / fw as f32).min(th as f32 / fh as f32);
            let rw = (fw as f32 * scale).round().max(1.0) as u32;
            let rh = (fh as f32 * scale).round().max(1.0) as u32;
            let resized = imageops::resize(&f, rw, rh, imageops::FilterType::Nearest);

            let mut canvas = RgbaImage::from_pixel(tw, th, image::Rgba([0, 0, 0, 0]));
            let ox = tw.saturating_sub(rw) / 2;
            let oy = th.saturating_sub(rh) / 2;
            imageops::overlay(&mut canvas, &resized, ox as i64, oy as i64);
            canvas
        }).collect();

        Ok(Self::from_frames(frames, size, fps))
    }

    pub fn from_frames(frames: Vec<RgbaImage>, size: (f32, f32), fps: f32) -> Self {
        assert!(!frames.is_empty(), "AnimatedSprite::from_frames requires at least one frame");
        Self {
            frames,
            current_frame:         0,
            frame_duration:        1.0 / fps,
            time_since_last_frame: 0.0,
            size,
            mirrored_h:            false,
            mirrored_v:            false,
            rotation:              RotationOptions::default(),
        }
    }

    pub fn fps(&self) -> f32 { 1.0 / self.frame_duration }

    pub fn update(&mut self, delta_time: f32) {
        self.time_since_last_frame += delta_time;
        while self.time_since_last_frame >= self.frame_duration {
            self.time_since_last_frame -= self.frame_duration;
            self.current_frame = (self.current_frame + 1) % self.frames.len();
        }
    }

    pub fn get_current_image(&self) -> Image {
        let mut pixels = self.frames[self.current_frame].clone();
        if self.mirrored_h { pixels = imageops::flip_horizontal(&pixels); }
        if self.mirrored_v { pixels = imageops::flip_vertical(&pixels); }
        Image {
            shape: ShapeType::Rectangle(0.0, self.size, self.rotation.to_radians()),
            image: pixels.into(),
            color: None,
        }
    }

    pub fn set_fps(&mut self, fps: f32) { self.frame_duration = 1.0 / fps; }

    pub fn reset(&mut self) {
        self.current_frame         = 0;
        self.time_since_last_frame = 0.0;
    }

    pub fn frame_count(&self) -> usize { self.frames.len() }

    pub fn set_frame(&mut self, frame: usize) {
        if frame < self.frames.len() {
            self.current_frame         = frame;
            self.time_since_last_frame = 0.0;
        }
    }

    pub fn mirror(&mut self)                         { self.mirrored_h = !self.mirrored_h; }
    pub fn set_mirrored(&mut self, v: bool)          { self.mirrored_h = v; }
    pub fn is_mirrored(&self) -> bool                { self.mirrored_h }
    pub fn mirror_vertical(&mut self)                { self.mirrored_v = !self.mirrored_v; }
    pub fn set_mirrored_vertical(&mut self, v: bool) { self.mirrored_v = v; }
    pub fn is_mirrored_vertical(&self) -> bool       { self.mirrored_v }

    pub fn set_rotation(&mut self, options: RotationOptions) { self.rotation = options; }

    pub fn rotate_by(&mut self, options: RotationOptions) {
        let new_rad = self.rotation.to_radians() + options.to_radians();
        self.rotation = RotationOptions {
            degrees:   new_rad.to_degrees(),
            direction: RotationDirection::Clockwise,
        };
    }

    pub fn clear_rotation(&mut self)      { self.rotation = RotationOptions::default(); }
    pub fn rotation_degrees(&self) -> f32 { self.rotation.to_radians().to_degrees() }

    pub fn rotate_90_cw(&mut self) {
        self.frames = self.frames.iter().map(|f| imageops::rotate270(f)).collect();
        self.size = (self.size.1, self.size.0);
    }

    pub fn rotate_90_ccw(&mut self) {
        self.frames = self.frames.iter().map(|f| imageops::rotate90(f)).collect();
        self.size = (self.size.1, self.size.0);
    }

    pub fn rotate_180(&mut self) {
        self.frames = self.frames.iter().map(|f| imageops::rotate180(f)).collect();
    }
}

impl std::fmt::Debug for AnimatedSprite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimatedSprite")
            .field("frame_count",    &self.frames.len())
            .field("current_frame",  &self.current_frame)
            .field("frame_duration", &self.frame_duration)
            .field("size",           &self.size)
            .field("mirrored_h",     &self.mirrored_h)
            .field("mirrored_v",     &self.mirrored_v)
            .field("rotation",       &self.rotation)
            .finish()
    }
}

pub fn star_field(width: u32, height: u32, star_count: u32, seed: u64) -> Image {
    let mut img = RgbaImage::from_pixel(width, height, Rgba([5, 5, 15, 255]));

    let mut state = seed.max(1);
    let mut next = || -> u64 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for _ in 0..star_count {
        let x = (next() % width as u64) as u32;
        let y = (next() % height as u64) as u32;
        let brightness = 100 + (next() % 156) as u8;
        let size_roll = next() % 100;
        let radius = if size_roll < 70 { 0 } else if size_roll < 92 { 1 } else { 2 };

        for dy in 0..=radius * 2 {
            for dx in 0..=radius * 2 {
                let px = x as i32 + dx as i32 - radius as i32;
                let py = y as i32 + dy as i32 - radius as i32;
                if px >= 0 && py >= 0 && (px as u32) < width && (py as u32) < height {
                    let dist = ((dx as f32 - radius as f32).powi(2)
                              + (dy as f32 - radius as f32).powi(2)).sqrt();
                    if dist <= radius as f32 + 0.5 {
                        let falloff = 1.0 - (dist / (radius as f32 + 1.0));
                        let b = (brightness as f32 * falloff).min(255.0) as u8;
                        img.put_pixel(px as u32, py as u32, Rgba([b, b, b.saturating_add(20), 255]));
                    }
                }
            }
        }
    }

    Image {
        shape: ShapeType::Rectangle(0.0, (width as f32, height as f32), 0.0),
        image: img.into(),
        color: None,
    }
}