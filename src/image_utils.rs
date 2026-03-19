use prism::canvas::{Image, ShapeType};

pub fn load_image(path: &str) -> Image {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|e| panic!("Failed to read image '{}': {}", path, e));
    let img = image::load_from_memory(&bytes)
        .unwrap_or_else(|e| panic!("Failed to decode image '{}': {}", path, e))
        .to_rgba8();
    let (w, h) = (img.width() as f32, img.height() as f32);
    Image {
        shape: ShapeType::Rectangle(0.0, (w, h), 0.0),
        image: img.into(),
        color: None,
    }
}


pub fn load_image_sized(path: &str, w: f32, h: f32) -> Image {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|e| panic!("Failed to read image '{}': {}", path, e));
    let img = image::load_from_memory(&bytes)
        .unwrap_or_else(|e| panic!("Failed to decode image '{}': {}", path, e))
        .to_rgba8();
    Image {
        shape: ShapeType::Rectangle(0.0, (w, h), 0.0),
        image: img.into(),
        color: None,
    }
}