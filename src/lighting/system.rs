use std::collections::HashMap;

use prism::canvas::PointLight;

use super::types::{
    AmbientLight, LightAttachment, LightEffect, LightSource, LightingConfig,
    color_to_rgb,
};

/// Owns all lights and produces the wgpu_canvas items each frame.
///
/// Modeled after `CrystallinePhysics` — a standalone system that the
/// Canvas owns and ticks each frame, but never touches the GPU directly.
#[derive(Clone, Debug)]
pub struct LightingSystem {
    pub config: LightingConfig,
    sources: HashMap<String, LightSource>,
    attachments: Vec<LightAttachment>,
}

impl LightingSystem {
    pub fn new(config: LightingConfig) -> Self {
        Self {
            config,
            sources: HashMap::new(),
            attachments: Vec::new(),
        }
    }

    // ── Light management ─────────────────────────────────────

    pub fn add_light(&mut self, source: LightSource) {
        self.sources.insert(source.id.clone(), source);
    }

    pub fn remove_light(&mut self, id: &str) {
        self.sources.remove(id);
        self.attachments.retain(|a| a.light_id != id);
    }

    pub fn get_light(&self, id: &str) -> Option<&LightSource> {
        self.sources.get(id)
    }

    pub fn get_light_mut(&mut self, id: &str) -> Option<&mut LightSource> {
        self.sources.get_mut(id)
    }

    pub fn light_count(&self) -> usize {
        self.sources.len()
    }

    pub fn active_light_count(&self) -> usize {
        self.sources.values().filter(|s| s.enabled).count()
    }

    pub fn clear_lights(&mut self) {
        self.sources.clear();
        self.attachments.clear();
    }

    // ── Ambient ──────────────────────────────────────────────

    pub fn set_ambient(&mut self, ambient: AmbientLight) {
        self.config.ambient = ambient;
    }

    pub fn ambient(&self) -> &AmbientLight {
        &self.config.ambient
    }

    // ── Attachments ──────────────────────────────────────────

    pub fn attach_light(&mut self, light_id: &str, object_name: &str, offset: (f32, f32)) {
        self.detach_light(light_id);
        self.attachments.push(LightAttachment {
            light_id: light_id.to_string(),
            object_name: object_name.to_string(),
            offset,
        });
    }

    pub fn detach_light(&mut self, light_id: &str) {
        self.attachments.retain(|a| a.light_id != light_id);
    }

    /// Update attached lights from object positions.
    /// `positions` maps object name → center position in logical coords.
    pub fn update_attachments(&mut self, positions: &HashMap<String, (f32, f32)>) {
        for attachment in &self.attachments {
            if let Some(&(cx, cy)) = positions.get(&attachment.object_name) {
                if let Some(light) = self.sources.get_mut(&attachment.light_id) {
                    light.position = (cx + attachment.offset.0, cy + attachment.offset.1);
                }
            }
        }
    }

    // ── Effects tick ─────────────────────────────────────────

    /// Advance all light effects by `dt` seconds.
    pub fn tick_effects(&mut self, dt: f32, entropy: &mut crate::entropy::Entropy) {
        for source in self.sources.values_mut() {
            let Some(effect) = source.effect.clone() else { continue };
            source.effect_time += dt;
            let t = source.effect_time;

            match &effect {
                LightEffect::Pulse { min_intensity, max_intensity, speed } => {
                    let phase = (t * speed * std::f32::consts::TAU).sin() * 0.5 + 0.5;
                    source.intensity = min_intensity + phase * (max_intensity - min_intensity);
                }
                LightEffect::Flicker { base_intensity, variance } => {
                    let noise = entropy.next() * 2.0 - 1.0;
                    source.intensity = (base_intensity + noise * variance).max(0.0);
                }
                LightEffect::ColorCycle { colors, speed } => {
                    if colors.len() >= 2 {
                        let total = colors.len() as f32;
                        let phase = (t * speed) % total;
                        let idx_a = phase.floor() as usize % colors.len();
                        let idx_b = (idx_a + 1) % colors.len();
                        let frac = phase.fract();
                        let a = &colors[idx_a];
                        let b = &colors[idx_b];
                        source.color = prism::canvas::Color(
                            lerp_u8(a.0, b.0, frac),
                            lerp_u8(a.1, b.1, frac),
                            lerp_u8(a.2, b.2, frac),
                            255,
                        );
                    }
                }
                LightEffect::FadeIn { target_intensity, duration } => {
                    let progress = (t / duration).min(1.0);
                    source.intensity = progress * target_intensity;
                    if progress >= 1.0 {
                        source.effect = None;
                        source.effect_time = 0.0;
                    }
                }
                LightEffect::FadeOut { duration } => {
                    let progress = (t / duration).min(1.0);
                    source.intensity = source.intensity * (1.0 - progress);
                    if progress >= 1.0 {
                        source.enabled = false;
                        source.effect = None;
                        source.effect_time = 0.0;
                    }
                }
            }
        }
    }

    // ── Emit items for wgpu_canvas ───────────────────────────

    /// Produce the `PointLight` array and ambient settings for this frame.
    /// Called from `Canvas::draw_pre()`.
    pub fn emit_lights(&self) -> (f32, f32, f32, f32, Vec<PointLight>) {
        let (ar, ag, ab) = self.config.ambient.as_rgb();

        let active_lights: Vec<PointLight> = self.sources
            .values()
            .filter(|s| s.enabled)
            .take(self.config.max_lights)
            .map(|s| s.to_point_light())
            .collect();

        (ar, ag, ab, self.config.ambient.strength, active_lights)
    }
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let v = a as f32 + (b as f32 - a as f32) * t;
    v.round().clamp(0.0, 255.0) as u8
}
