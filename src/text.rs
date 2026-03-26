use prism::canvas::{Text, Span, Align, Font, Color};
use std::sync::{Arc, Mutex};
use std::time::Instant;

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
struct CacheEntry {
    span_keys: Vec<(String, Color)>,
    scale:     f32,
    text:      Text,
}

#[derive(Clone, Debug)]
pub struct TextSpec {
    pub spans: Vec<SpanSpec>,
    pub align: Align,
    cache: Arc<Mutex<Option<CacheEntry>>>,
}

impl TextSpec {
    pub fn new(spans: Vec<SpanSpec>, align: Align) -> Self {
        Self { spans, align, cache: Arc::new(Mutex::new(None)) }
    }

    pub fn build(&self, scale: f32) -> Text {
        let span_keys: Vec<(String, Color)> = self.spans
            .iter()
            .map(|s| (s.text.clone(), s.color))
            .collect();

        let mut guard = self.cache.lock().unwrap();

        if let Some(ref entry) = *guard {
            let scale_ok = (entry.scale - scale).abs() < 0.0001;
            let keys_ok  = entry.span_keys == span_keys;
            if scale_ok && keys_ok {
                return entry.text.clone();
            }
        }

        let start = Instant::now();

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

        let text = Text::new(spans, None, self.align.clone(), None);

        let elapsed = start.elapsed();
        println!("Text build took: {:.3?}", elapsed);

        *guard = Some(CacheEntry { span_keys, scale, text: text.clone() });
        text
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