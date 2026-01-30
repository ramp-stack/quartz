use prism::canvas::Image;
use prism::canvas::ShapeType;
use image::{RgbaImage, AnimationDecoder};
use std::io::Cursor;

#[derive(Clone)]
pub struct AnimatedSprite {
    frames: Vec<RgbaImage>,
    current_frame: usize,
    frame_duration: f32,
    time_since_last_frame: f32,
    size: (f32, f32),
}

impl AnimatedSprite {

    pub fn new(gif_bytes: &[u8], size: (f32, f32), fps: f32) -> Result<Self, String> {
        let cursor = Cursor::new(gif_bytes);
        let decoder = image::codecs::gif::GifDecoder::new(cursor)
            .map_err(|e| format!("Failed to decode GIF: {}", e))?;
        
        let frames_collection = decoder.into_frames();
        
        let mut frames = Vec::new();
        for frame_result in frames_collection {
            let frame = frame_result
                .map_err(|e| format!("Failed to decode frame: {}", e))?;
            frames.push(frame.into_buffer());
        }
        
        if frames.is_empty() {
            return Err("GIF has no frames".to_string());
        }
        
        let frame_duration = 1.0 / fps;
        
        Ok(Self {
            frames,
            current_frame: 0,
            frame_duration,
            time_since_last_frame: 0.0,
            size,
        })
    }
    
    pub fn update(&mut self, delta_time: f32) {
        self.time_since_last_frame += delta_time;
        
        while self.time_since_last_frame >= self.frame_duration {
            self.time_since_last_frame -= self.frame_duration;
            self.current_frame = (self.current_frame + 1) % self.frames.len();
        }
    }
    
    pub fn get_current_image(&self) -> Image {
        let current_frame_data = &self.frames[self.current_frame];
        
        Image {
            shape: ShapeType::Rectangle(0.0, self.size, 0.0),
            image: current_frame_data.clone().into(),
            color: None,
        }
    }
    
    pub fn set_fps(&mut self, fps: f32) {
        self.frame_duration = 1.0 / fps;
    }
    
    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.time_since_last_frame = 0.0;
    }
    
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
    
    pub fn set_frame(&mut self, frame: usize) {
        if frame < self.frames.len() {
            self.current_frame = frame;
            self.time_since_last_frame = 0.0;
        }
    }
}

impl std::fmt::Debug for AnimatedSprite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimatedSprite")
            .field("frame_count", &self.frames.len())
            .field("current_frame", &self.current_frame)
            .field("frame_duration", &self.frame_duration)
            .field("size", &self.size)
            .finish()
    }
}


