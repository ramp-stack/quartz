use prism::canvas::{Image, ShapeType};
use image::{RgbaImage, AnimationDecoder, imageops};
use std::cell::RefCell;
use std::io::Cursor;

thread_local! {
    pub(crate) static LAST_ASSET_PATH: RefCell<Option<String>> = RefCell::new(None);
}

pub fn load_image(path: &str) -> Image {
    LAST_ASSET_PATH.with(|p| *p.borrow_mut() = Some(path.to_string()));
    let rgba = image::open(path)
        .unwrap_or_else(|_| panic!("quartz: cannot open image '{}'", path))
        .into_rgba8();
    let (w, h) = (rgba.width() as f32, rgba.height() as f32);
    make_image(rgba, w, h)
}

pub fn load_image_sized(path: &str, w: f32, h: f32) -> Image {
    LAST_ASSET_PATH.with(|p| *p.borrow_mut() = Some(path.to_string()));
    let rgba = image::open(path)
        .unwrap_or_else(|_| panic!("quartz: cannot open image '{}'", path))
        .into_rgba8();
    make_image(rgba, w, h)
}

/// Load an animated GIF from a file path. Registers the path automatically
/// for hot-reloading — no extra setup required.
pub fn load_animation(path: &str, size: (f32, f32), fps: f32) -> AnimatedSprite {
    LAST_ASSET_PATH.with(|p| *p.borrow_mut() = Some(path.to_string()));
    let bytes = std::fs::read(path)
        .unwrap_or_else(|_| panic!("quartz: cannot read animation '{}'", path));
    AnimatedSprite::decode_vec(bytes, size, fps)
        .unwrap_or_else(|e| panic!("quartz: failed to decode animation '{}': {}", path, e))
}

/// Re-read an image from disk at a fixed size without touching the thread-local.
/// Used internally by hot-reload; not part of the public API.
pub(crate) fn reload_image_raw(path: &str, size: (f32, f32)) -> Image {
    let rgba = image::open(path)
        .unwrap_or_else(|_| panic!("quartz: cannot open image '{}'", path))
        .into_rgba8();
    make_image(rgba, size.0, size.1)
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
    /// Create from static bytes (e.g. `include_bytes!`). Does not register for hot-reload.
    pub fn new(gif_bytes: &[u8], size: (f32, f32), fps: f32) -> Result<Self, String> {
        Self::decode_slice(gif_bytes, size, fps)
    }

    /// Create from an owned Vec<u8>. Used by `load_animation` and hot-reload internally.
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