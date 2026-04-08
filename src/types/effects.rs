use prism::canvas::Color;

#[derive(Debug, Clone)]
pub struct GlowConfig {
    pub color: Color,
    pub width: f32,
}

#[derive(Debug, Clone, Default)]
pub struct HighlightEffect {
    pub tint: Option<Color>,
    pub glow: Option<GlowConfig>,
}