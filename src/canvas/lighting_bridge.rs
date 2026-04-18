use super::core::Canvas;
use crate::lighting::{
    AmbientLight, LightEffect, LightSource, LightingConfig, LightingSystem,
};
use prism::canvas::{BloomSettings, Color};

impl Canvas {
    // ── Lifecycle ────────────────────────────────────────────

    /// Enable the lighting system with the given configuration.
    /// Automatically enables GPU features and bloom if not already active.
    pub fn enable_lighting(&mut self, config: LightingConfig) {
        self.enable_gpu_features();
        self.lighting = Some(LightingSystem::new(config));
        // Auto-enable bloom with low threshold so lit areas glow.
        if !self.bloom_enabled() {
            self.enable_bloom(BloomSettings {
                threshold: 0.25,
                strength: 0.8,
            });
        }
    }

    /// Disable the lighting system entirely.
    pub fn disable_lighting(&mut self) {
        self.lighting = None;
    }

    /// Returns `true` if the lighting system is active.
    pub fn has_lighting(&self) -> bool {
        self.lighting.is_some()
    }

    // ── Ambient ──────────────────────────────────────────────

    /// Set the global ambient light color and strength.
    pub fn set_ambient(&mut self, color: Color, strength: f32) {
        if let Some(ls) = &mut self.lighting {
            ls.set_ambient(AmbientLight { color, strength });
        }
    }

    /// Get the current ambient light, if lighting is enabled.
    pub fn ambient(&self) -> Option<&AmbientLight> {
        self.lighting.as_ref().map(|ls| ls.ambient())
    }

    // ── Light management ─────────────────────────────────────

    /// Add a light source. If a light with the same ID exists, it is replaced.
    pub fn add_light(&mut self, source: LightSource) {
        if let Some(ls) = &mut self.lighting {
            ls.add_light(source);
        }
    }

    /// Remove a light source by its ID.
    pub fn remove_light(&mut self, id: &str) {
        if let Some(ls) = &mut self.lighting {
            ls.remove_light(id);
        }
    }

    /// Get a light source by ID (immutable).
    pub fn get_light(&self, id: &str) -> Option<&LightSource> {
        self.lighting.as_ref().and_then(|ls| ls.get_light(id))
    }

    /// Get a light source by ID (mutable).
    pub fn get_light_mut(&mut self, id: &str) -> Option<&mut LightSource> {
        self.lighting.as_mut().and_then(|ls| ls.get_light_mut(id))
    }

    /// Set a light's position directly.
    pub fn set_light_position(&mut self, id: &str, x: f32, y: f32) {
        if let Some(light) = self.get_light_mut(id) {
            light.position = (x, y);
        }
    }

    /// Set a light's color.
    pub fn set_light_color(&mut self, id: &str, color: Color) {
        if let Some(light) = self.get_light_mut(id) {
            light.color = color;
        }
    }

    /// Set a light's intensity.
    pub fn set_light_intensity(&mut self, id: &str, intensity: f32) {
        if let Some(light) = self.get_light_mut(id) {
            light.intensity = intensity;
        }
    }

    /// Set a light's radius.
    pub fn set_light_radius(&mut self, id: &str, radius: f32) {
        if let Some(light) = self.get_light_mut(id) {
            light.radius = radius;
        }
    }

    /// Enable or disable a light.
    pub fn set_light_enabled(&mut self, id: &str, enabled: bool) {
        if let Some(light) = self.get_light_mut(id) {
            light.enabled = enabled;
        }
    }

    // ── Attachments ──────────────────────────────────────────

    /// Attach a light to a game object so it follows automatically.
    pub fn attach_light(&mut self, light_id: &str, object_name: &str, offset: (f32, f32)) {
        if let Some(ls) = &mut self.lighting {
            ls.attach_light(light_id, object_name, offset);
        }
    }

    /// Detach a light from its game object.
    pub fn detach_light(&mut self, light_id: &str) {
        if let Some(ls) = &mut self.lighting {
            ls.detach_light(light_id);
        }
    }

    // ── Effects ──────────────────────────────────────────────

    /// Set an animation effect on a light.
    pub fn set_light_effect(&mut self, id: &str, effect: LightEffect) {
        if let Some(light) = self.get_light_mut(id) {
            light.effect = Some(effect);
        }
    }

    /// Clear the animation effect from a light.
    pub fn clear_light_effect(&mut self, id: &str) {
        if let Some(light) = self.get_light_mut(id) {
            light.effect = None;
        }
    }

    // ── Glow-to-light sync ───────────────────────────────────

    /// Create a point light that matches the glow config of a game object.
    /// The light ID will be `"glow_{object_name}"` and is attached automatically.
    pub fn sync_glow_light(&mut self, object_name: &str) {
        let (position, glow_color, glow_width) = {
            let Some(obj) = self.get_game_object(object_name) else { return };
            let glow = obj.highlight.as_ref().and_then(|h| h.glow.as_ref());
            let Some(glow) = glow else { return };
            let cx = obj.position.0 + obj.size.0 / 2.0;
            let cy = obj.position.1 + obj.size.1 / 2.0;
            (
                (cx, cy),
                glow.color,
                glow.width,
            )
        };

        let light_id = format!("glow_{}", object_name);
        let source = LightSource::new(
            light_id.clone(),
            position,
            glow_color,
            glow_width * 20.0,
            0.6,
        );

        self.add_light(source);
        self.attach_light(&light_id, object_name, (0.0, 0.0));
    }

    // ── Internal: tick lighting each frame ────────────────────

    /// Called from the event loop to update attachments + effects.
    pub(crate) fn tick_lighting(&mut self, dt: f32) {
        if self.lighting.is_none() { return; }

        // Collect object center positions for attachment updates.
        let positions: std::collections::HashMap<String, (f32, f32)> = self
            .store
            .name_to_index
            .iter()
            .filter_map(|(name, &idx)| {
                let obj = self.store.objects.get(idx)?;
                let cx = obj.position.0 + obj.size.0 / 2.0;
                let cy = obj.position.1 + obj.size.1 / 2.0;
                Some((name.clone(), (cx, cy)))
            })
            .collect();

        let ls = self.lighting.as_mut().unwrap();
        ls.update_attachments(&positions);
        ls.tick_effects(dt, &mut self.entropy);
    }
}
