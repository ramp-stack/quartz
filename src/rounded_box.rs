use image::{RgbaImage, Rgba};
use prism::canvas::{Image, ShapeType, Color};
use std::sync::Arc;

/// Border style configuration
#[derive(Debug, Clone, Copy)]
pub struct BorderStyle {
    pub width: f32,
    pub color: Color,
    pub radius: Option<f32>,
}

impl BorderStyle {
    pub fn new(width: f32, color: Color) -> Self {
        Self { width, color, radius: None }
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }
}

/// Gradient direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientDirection {
    Horizontal,
    Vertical,
    Diagonal,
    DiagonalAlt,
    Radial,
}

/// Shadow configuration
#[derive(Debug, Clone, Copy)]
pub struct Shadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub color: Color,
}

impl Shadow {
    pub fn new(offset_x: f32, offset_y: f32, blur: f32, color: Color) -> Self {
        Self { offset_x, offset_y, blur, color }
    }

    pub fn soft() -> Self {
        Self { offset_x: 0.0, offset_y: 4.0, blur: 8.0, color: Color(0, 0, 0, 60) }
    }

    pub fn hard() -> Self {
        Self { offset_x: 0.0, offset_y: 2.0, blur: 0.0, color: Color(0, 0, 0, 100) }
    }

    pub fn elevated() -> Self {
        Self { offset_x: 0.0, offset_y: 8.0, blur: 16.0, color: Color(0, 0, 0, 40) }
    }
}

/// Corner radius specification
#[derive(Debug, Clone, Copy)]
pub enum CornerRadius {
    Uniform(f32),
    Individual(f32, f32, f32, f32),
}

impl CornerRadius {
    pub fn uniform(r: f32) -> Self {
        CornerRadius::Uniform(r)
    }

    pub fn individual(tl: f32, tr: f32, br: f32, bl: f32) -> Self {
        CornerRadius::Individual(tl, tr, br, bl)
    }

    pub fn top(r: f32) -> Self {
        CornerRadius::Individual(r, r, 0.0, 0.0)
    }

    pub fn bottom(r: f32) -> Self {
        CornerRadius::Individual(0.0, 0.0, r, r)
    }

    pub fn left(r: f32) -> Self {
        CornerRadius::Individual(r, 0.0, 0.0, r)
    }

    pub fn right(r: f32) -> Self {
        CornerRadius::Individual(0.0, r, r, 0.0)
    }

    fn get(&self, corner: usize) -> f32 {
        match self {
            CornerRadius::Uniform(r) => *r,
            CornerRadius::Individual(tl, tr, br, bl) => {
                match corner {
                    0 => *tl,
                    1 => *tr,
                    2 => *br,
                    3 => *bl,
                    _ => 0.0,
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoundedBoxBuilder {
    width: f32,
    height: f32,
    radius: CornerRadius,
    fill_color: Option<Color>,
    gradient: Option<(Color, Color, GradientDirection)>,
    border: Option<BorderStyle>,
    shadow: Option<Shadow>,
    antialiasing: bool,
}

impl RoundedBoxBuilder {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            radius: CornerRadius::Uniform(0.0),
            fill_color: Some(Color(255, 255, 255, 255)),
            gradient: None,
            border: None,
            shadow: None,
            antialiasing: true,
        }
    }

    pub fn radius(mut self, r: f32) -> Self {
        self.radius = CornerRadius::Uniform(r);
        self
    }

    pub fn corner_radii(mut self, tl: f32, tr: f32, br: f32, bl: f32) -> Self {
        self.radius = CornerRadius::Individual(tl, tr, br, bl);
        self
    }

    pub fn corners(mut self, radius: CornerRadius) -> Self {
        self.radius = radius;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.fill_color = Some(color);
        self.gradient = None;
        self
    }

    pub fn gradient(mut self, start: Color, end: Color, direction: GradientDirection) -> Self {
        self.gradient = Some((start, end, direction));
        self.fill_color = None;
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border = Some(BorderStyle::new(width, color));
        self
    }

    pub fn border_style(mut self, style: BorderStyle) -> Self {
        self.border = Some(style);
        self
    }

    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }

    pub fn no_antialiasing(mut self) -> Self {
        self.antialiasing = false;
        self
    }

    pub fn build(self) -> Image {
        RoundedBox::render(
            self.width,
            self.height,
            self.radius,
            self.fill_color,
            self.gradient,
            self.border,
            self.shadow,
            self.antialiasing,
        )
    }
}


pub struct RoundedBox;

impl RoundedBox {
    pub fn new(width: f32, height: f32) -> RoundedBoxBuilder {
        RoundedBoxBuilder::new(width, height)
    }

    pub fn solid(width: f32, height: f32, radius: f32, color: Color) -> Image {
        // Use efficient shape rendering - no pixel baking needed
        Image {
            shape: ShapeType::RoundedRectangle(0.0, (width, height), 0.0, radius),
            image: Arc::new(RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255]))),
            color: Some(color),
        }
    }

    pub fn bordered(
        width: f32, height: f32, radius: f32, fill: Color,
        border_width: f32, border_color: Color,
    ) -> Image {
        Self::new(width, height).radius(radius).color(fill)
            .border(border_width, border_color).build()
    }

    pub fn gradient(
        width: f32, height: f32, radius: f32,
        start: Color, end: Color, direction: GradientDirection,
    ) -> Image {
        Self::new(width, height).radius(radius)
            .gradient(start, end, direction).build()
    }

    pub fn card(width: f32, height: f32) -> Image {
        Self::new(width, height).radius(12.0)
            .color(Color(255, 255, 255, 255))
            .border(1.0, Color(220, 220, 220, 255))
            .shadow(Shadow::soft()).build()
    }

    pub fn button(width: f32, height: f32, color: Color) -> Image {
        Self::new(width, height).radius(8.0).color(color)
            .shadow(Shadow { offset_x: 0.0, offset_y: 2.0, blur: 4.0, color: Color(0, 0, 0, 30) })
            .build()
    }

    pub fn panel(width: f32, height: f32) -> Image {
        Self::new(width, height).radius(16.0)
            .color(Color(248, 249, 250, 255))
            .border(1.5, Color(200, 205, 210, 255)).build()
    }

    pub fn input(width: f32, height: f32) -> Image {
        Self::new(width, height).radius(6.0)
            .color(Color(255, 255, 255, 255))
            .border(1.0, Color(180, 180, 180, 255)).build()
    }

    pub fn tooltip(width: f32, height: f32) -> Image {
        Self::new(width, height).radius(4.0)
            .color(Color(30, 30, 30, 240))
            .shadow(Shadow::soft()).build()
    }

    pub fn modal(width: f32, height: f32) -> Image {
        Self::new(width, height).radius(20.0)
            .color(Color(255, 255, 255, 255))
            .shadow(Shadow::elevated()).build()
    }

    fn render(
        width: f32, height: f32, radius: CornerRadius,
        fill_color: Option<Color>,
        gradient: Option<(Color, Color, GradientDirection)>,
        border: Option<BorderStyle>,
        shadow: Option<Shadow>,
        antialiasing: bool,
    ) -> Image {
        let shadow_extend = shadow.as_ref().map_or(0.0, |s| {
            (s.offset_x.abs() + s.blur).max(s.offset_y.abs() + s.blur)
        });
        
        let canvas_w = (width + shadow_extend * 2.0).ceil() as u32;
        let canvas_h = (height + shadow_extend * 2.0).ceil() as u32;
        let mut img = RgbaImage::new(canvas_w, canvas_h);
        
        let ox = shadow_extend;
        let oy = shadow_extend;
        
        if let Some(s) = shadow {
            Self::draw_shadow(&mut img, ox, oy, width, height, radius, s);
        }
        
        if let Some((start, end, dir)) = gradient {
            Self::draw_gradient(&mut img, ox, oy, width, height, radius, start, end, dir, antialiasing);
        } else if let Some(color) = fill_color {
            Self::draw_solid(&mut img, ox, oy, width, height, radius, color, antialiasing);
        }
        
        if let Some(b) = border {
            Self::draw_border(&mut img, ox, oy, width, height, radius, b, antialiasing);
        }
        
        Image {
            shape: ShapeType::Rectangle(0.0, (width, height), 0.0),
            image: Arc::new(img),
            color: None,
        }
    }

    fn draw_solid(
        img: &mut RgbaImage, ox: f32, oy: f32, w: f32, h: f32,
        radius: CornerRadius, color: Color, aa: bool,
    ) {
        let Color(r, g, b, a) = color;
        for y in 0..h.ceil() as u32 {
            for x in 0..w.ceil() as u32 {
                let coverage = Self::rounded_rect_coverage(x as f32, y as f32, w, h, radius, aa);
                if coverage > 0.0 {
                    let final_alpha = (a as f32 * coverage) as u8;
                    let img_x = (ox + x as f32) as u32;
                    let img_y = (oy + y as f32) as u32;
                    if img_x < img.width() && img_y < img.height() {
                        img.put_pixel(img_x, img_y, Rgba([r, g, b, final_alpha]));
                    }
                }
            }
        }
    }

    fn draw_gradient(
        img: &mut RgbaImage, ox: f32, oy: f32, w: f32, h: f32,
        radius: CornerRadius, start: Color, end: Color,
        direction: GradientDirection, aa: bool,
    ) {
        for y in 0..h.ceil() as u32 {
            for x in 0..w.ceil() as u32 {
                let fx = x as f32;
                let fy = y as f32;
                let coverage = Self::rounded_rect_coverage(fx, fy, w, h, radius, aa);
                if coverage > 0.0 {
                    let t = match direction {
                        GradientDirection::Horizontal => fx / w,
                        GradientDirection::Vertical => fy / h,
                        GradientDirection::Diagonal => ((fx / w) + (fy / h)) / 2.0,
                        GradientDirection::DiagonalAlt => ((fx / w) + (1.0 - fy / h)) / 2.0,
                        GradientDirection::Radial => {
                            let dx = fx - w / 2.0;
                            let dy = fy - h / 2.0;
                            ((dx * dx + dy * dy).sqrt() / (w.max(h) / 2.0)).min(1.0)
                        }
                    };
                    let color = Self::lerp_color(start, end, t);
                    let Color(r, g, b, a) = color;
                    let final_alpha = (a as f32 * coverage) as u8;
                    let img_x = (ox + fx) as u32;
                    let img_y = (oy + fy) as u32;
                    if img_x < img.width() && img_y < img.height() {
                        img.put_pixel(img_x, img_y, Rgba([r, g, b, final_alpha]));
                    }
                }
            }
        }
    }

    fn draw_border(
        img: &mut RgbaImage, ox: f32, oy: f32, w: f32, h: f32,
        radius: CornerRadius, border: BorderStyle, aa: bool,
    ) {
        let Color(r, g, b, a) = border.color;
        let bw = border.width;
        for y in 0..h.ceil() as u32 {
            for x in 0..w.ceil() as u32 {
                let fx = x as f32;
                let fy = y as f32;
                let outer_coverage = Self::rounded_rect_coverage(fx, fy, w, h, radius, aa);
                let inner_coverage = Self::rounded_rect_coverage(
                    fx - bw, fy - bw, w - bw * 2.0, h - bw * 2.0, radius, aa,
                );
                let border_coverage = (outer_coverage - inner_coverage).max(0.0);
                if border_coverage > 0.0 {
                    let final_alpha = (a as f32 * border_coverage) as u8;
                    let img_x = (ox + fx) as u32;
                    let img_y = (oy + fy) as u32;
                    if img_x < img.width() && img_y < img.height() {
                        let existing = img.get_pixel(img_x, img_y);
                        let blended = Self::blend_pixels(*existing, Rgba([r, g, b, final_alpha]));
                        img.put_pixel(img_x, img_y, blended);
                    }
                }
            }
        }
    }

    fn draw_shadow(
        img: &mut RgbaImage, ox: f32, oy: f32, w: f32, h: f32,
        radius: CornerRadius, shadow: Shadow,
    ) {
        let Color(r, g, b, a) = shadow.color;
        let shadow_ox = ox + shadow.offset_x;
        let shadow_oy = oy + shadow.offset_y;
        
        for y in 0..h.ceil() as u32 {
            for x in 0..w.ceil() as u32 {
                let fx = x as f32;
                let fy = y as f32;
                let coverage = Self::rounded_rect_coverage(fx, fy, w, h, radius, true);
                
                if coverage > 0.0 && shadow.blur > 0.0 {
                    let blur_samples = (shadow.blur / 2.0).ceil() as i32;
                    for by in -blur_samples..=blur_samples {
                        for bx in -blur_samples..=blur_samples {
                            let dist = ((bx * bx + by * by) as f32).sqrt();
                            if dist > shadow.blur { continue; }
                            let blur_strength = 1.0 - (dist / shadow.blur);
                            let alpha = (a as f32 * coverage * blur_strength * 0.3) as u8;
                            if alpha > 0 {
                                let img_x = (shadow_ox + fx + bx as f32) as i32;
                                let img_y = (shadow_oy + fy + by as f32) as i32;
                                if img_x >= 0 && img_x < img.width() as i32 
                                    && img_y >= 0 && img_y < img.height() as i32 {
                                    let existing = img.get_pixel(img_x as u32, img_y as u32);
                                    let blended = Self::blend_pixels(*existing, Rgba([r, g, b, alpha]));
                                    img.put_pixel(img_x as u32, img_y as u32, blended);
                                }
                            }
                        }
                    }
                } else if coverage > 0.0 {
                    let final_alpha = (a as f32 * coverage) as u8;
                    let img_x = (shadow_ox + fx) as u32;
                    let img_y = (shadow_oy + fy) as u32;
                    if img_x < img.width() && img_y < img.height() {
                        let existing = img.get_pixel(img_x, img_y);
                        let blended = Self::blend_pixels(*existing, Rgba([r, g, b, final_alpha]));
                        img.put_pixel(img_x, img_y, blended);
                    }
                }
            }
        }
    }

    fn rounded_rect_coverage(
        x: f32, y: f32, w: f32, h: f32,
        radius: CornerRadius, antialiasing: bool,
    ) -> f32 {
        let corner = if x < radius.get(0) && y < radius.get(0) {
            Some((0, radius.get(0), x, y))
        } else if x > w - radius.get(1) && y < radius.get(1) {
            Some((1, radius.get(1), w - x, y))
        } else if x > w - radius.get(2) && y > h - radius.get(2) {
            Some((2, radius.get(2), w - x, h - y))
        } else if x < radius.get(3) && y > h - radius.get(3) {
            Some((3, radius.get(3), x, h - y))
        } else {
            None
        };
        
        if let Some((_, r, dx, dy)) = corner {
            if r == 0.0 {
                return if x >= 0.0 && x <= w && y >= 0.0 && y <= h { 1.0 } else { 0.0 };
            }
            let dist = ((r - dx).powi(2) + (r - dy).powi(2)).sqrt();
            if !antialiasing {
                return if dist <= r { 1.0 } else { 0.0 };
            }
            if dist < r - 0.5 {
                1.0
            } else if dist > r + 0.5 {
                0.0
            } else {
                1.0 - (dist - (r - 0.5))
            }
        } else {
            if x >= 0.0 && x <= w && y >= 0.0 && y <= h { 1.0 } else { 0.0 }
        }
    }

    fn lerp_color(start: Color, end: Color, t: f32) -> Color {
        let Color(r1, g1, b1, a1) = start;
        let Color(r2, g2, b2, a2) = end;
        Color(
            Self::lerp_u8(r1, r2, t),
            Self::lerp_u8(g1, g2, t),
            Self::lerp_u8(b1, b2, t),
            Self::lerp_u8(a1, a2, t),
        )
    }

    fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
        (a as f32 * (1.0 - t) + b as f32 * t) as u8
    }

    fn blend_pixels(bg: Rgba<u8>, fg: Rgba<u8>) -> Rgba<u8> {
        let alpha = fg[3] as f32 / 255.0;
        let inv_alpha = 1.0 - alpha;
        Rgba([
            (fg[0] as f32 * alpha + bg[0] as f32 * inv_alpha) as u8,
            (fg[1] as f32 * alpha + bg[1] as f32 * inv_alpha) as u8,
            (fg[2] as f32 * alpha + bg[2] as f32 * inv_alpha) as u8,
            ((fg[3] as f32 + bg[3] as f32 * inv_alpha) as u8).min(255),
        ])
    }
}

pub fn rounded_box(width: f32, height: f32, radius: f32, color: Color) -> Image {
    // Use ShapeType for efficient per-frame rendering
    Image {
        shape: ShapeType::RoundedRectangle(0.0, (width, height), 0.0, radius),
        image: Arc::new(RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255]))),
        color: Some(color),
    }
}

/// Create an outlined rounded box (transparent fill, border only)
pub fn rounded_box_outline(width: f32, height: f32, radius: f32, border_width: f32, border_color: Color) -> Image {
    RoundedBox::new(width, height)
        .radius(radius)
        .color(Color(0, 0, 0, 0))  // Transparent fill
        .border(border_width, border_color)
        .build()
}

pub fn rounded_box_bordered(
    width: f32, height: f32, radius: f32, fill: Color,
    border_width: f32, border_color: Color,
) -> Image {
    RoundedBox::bordered(width, height, radius, fill, border_width, border_color)
}

pub fn rounded_box_gradient(
    width: f32, height: f32, radius: f32,
    start: Color, end: Color, direction: GradientDirection,
) -> Image {
    RoundedBox::gradient(width, height, radius, start, end, direction)
}