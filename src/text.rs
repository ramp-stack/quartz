use prism::canvas::{Text, Span, Align, Font, Color};

// ---------------------------------------------------------------------------
// SpanSpec / TextSpec
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SpanSpec {
    pub text:           String,
    pub font_size:      f32,
    pub line_height:    Option<f32>,
    pub font:           Font,
    pub color:          Color,
    pub letter_spacing: f32,
}

#[derive(Clone, Debug)]
pub struct TextSpec {
    pub spans: Vec<SpanSpec>,
    pub align: Align,
}

impl TextSpec {
    pub fn new(spans: Vec<SpanSpec>, align: Align) -> Self {
        Self { spans, align }
    }

    /// Build a prism Text with all font sizes multiplied by `scale`.
    pub fn build(&self, scale: f32) -> Text {
        let spans: Vec<Span> = self.spans.iter().map(|s| {
            Span::new(
                s.text.clone(),
                s.font_size      * scale,
                s.line_height.map(|lh| lh * scale),
                s.font.clone().into(),
                s.color,
                s.letter_spacing * scale,
            )
        }).collect();
        Text::new(spans, None, self.align.clone(), None)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a centred single-span TextSpec. Font size is in virtual pixels and
/// scales automatically with the canvas.
///
/// ```rust
/// obj.set_text(make_text("Score: 0", 38.0, &font, Color(120, 70, 30, 255)));
/// ```
pub fn make_text(text: impl Into<String>, font_size: f32, font: &Font, color: Color) -> TextSpec {
    make_text_aligned(text, font_size, font, color, Align::Center)
}

/// Like `make_text` but with explicit alignment.
///
/// ```rust
/// obj.set_text(make_text_aligned("Hi", 38.0, &font, Color(255,255,255,255), Align::Left));
/// ```
pub fn make_text_aligned(
    text: impl Into<String>,
    font_size: f32,
    font: &Font,
    color: Color,
    align: Align,
) -> TextSpec {
    TextSpec::new(vec![
        SpanSpec {
            text:           text.into(),
            font_size,
            line_height:    Some(font_size * 1.35),
            font:           font.clone(),
            color,
            letter_spacing: 0.0,
        }
    ], align)
}

/// Build a multi-span TextSpec from a list of (text, font_size, font, color) tuples.
pub fn make_text_multi(spans: Vec<(String, f32, Font, Color)>, align: Align) -> TextSpec {
    TextSpec::new(
        spans.into_iter().map(|(text, font_size, font, color)| SpanSpec {
            text,
            font_size,
            line_height:    Some(font_size * 1.35),
            font,
            color,
            letter_spacing: 0.0,
        }).collect(),
        align,
    )
}