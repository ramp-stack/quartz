use prism::canvas::{Text, Span, Align, Font, Color};
use std::cell::{Cell, RefCell};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

#[derive(Clone, Debug, PartialEq)]
pub struct SpanSpec {
    pub text:           String,
    pub font_size:      f32,
    pub line_height:    Option<f32>,
    pub font:           Font,
    pub color:          Color,
    pub letter_spacing: f32,
}

#[derive(Debug, Clone)]
struct TextCache {
    text:         Text,
    scale:        f32,
    content_hash: u64,
}

#[derive(Clone, Debug)]
pub struct TextSpec {
    pub spans: Vec<SpanSpec>,
    pub align: Align,
    cache:     RefCell<Option<TextCache>>,
    dirty:     Cell<bool>,
    old: Vec<SpanSpec>,
    old_text: Option<Text>,
}

impl TextSpec {
    pub fn new(spans: Vec<SpanSpec>, align: Align) -> Self {
        Self { spans: spans.clone(), align, cache: RefCell::new(None), dirty: Cell::new(true), old: spans, old_text: None }
    }

    /// Called by the engine every tick via `update_text_scale`. Always returns
    /// a Text (cached when nothing changed), so the engine stays happy.
    pub fn build(&self, scale: f32) -> Text {
        Self::build_text(&self.spans, &self.align, scale)
    }

    /// Mark this spec as needing `set_text` to be called again.
    pub fn mark_dirty(&self) {
        self.dirty.set(true);
    }

    /// Returns true (and clears the flag) if `set_text` should be called.
    /// Returns false on quiet frames — caller must skip `set_text` entirely.
    pub fn take_dirty(&self) -> bool {
        let was = self.dirty.get();
        self.dirty.set(false);
        was
    }

    fn content_hash(&self) -> u64 {
        let mut h = DefaultHasher::new();
        for s in &self.spans {
            s.text.hash(&mut h);
            s.font_size.to_bits().hash(&mut h);
            s.letter_spacing.to_bits().hash(&mut h);
            s.line_height.map(f32::to_bits).hash(&mut h);
            format!("{:?}{:?}", s.font, s.color).hash(&mut h);
        }
        format!("{:?}", self.align).hash(&mut h);
        h.finish()
    }

    fn build_text(spans: &[SpanSpec], align: &Align, scale: f32) -> Text {
        let prism_spans: Vec<Span> = spans.iter().map(|s| {
            Span::new(
                s.text.clone(),
                s.font_size      * scale,
                s.line_height.map(|lh| lh * scale),
                s.font.clone().into(),
                s.color,
                s.letter_spacing * scale,
            )
        }).collect();
        Text::new(prism_spans, None, align.clone(), None)
    }
}

pub fn make_text(text: impl Into<String>, font_size: f32, font: &Font, color: Color) -> TextSpec {
    make_text_aligned(text, font_size, font, color, Align::Center)
}

pub fn make_text_aligned(
    text:      impl Into<String>,
    font_size: f32,
    font:      &Font,
    color:     Color,
    align:     Align,
) -> TextSpec {
    TextSpec::new(vec![SpanSpec {
        text:           text.into(),
        font_size,
        line_height:    Some(font_size * 1.35),
        font:           font.clone(),
        color,
        letter_spacing: 0.0,
    }], align)
}

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