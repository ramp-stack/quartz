use prism::drawable::{Component, Drawable, SizedTree};
use prism::layout::{Area, SizeRequest, Layout};
use prism::canvas::BloomSettings;
use std::cell::Cell;
use std::collections::HashMap;

use wgpu_canvas::abi::{EnvelopePayload, FrameEnvelope};
use wgpu_canvas::capabilities::CapabilitySnapshot;

use prism::canvas::Image;
use crate::store::ObjectStore;
use crate::input::{InputState, MouseState, CallbackStore};
use crate::scene::SceneManager;
use crate::camera::Camera;
use crate::entropy::Entropy;
use crate::file_watcher;
use crate::value::Value;
use crate::crystalline::{CrystallinePhysics, ParticleSystem, ParticleState};
use crate::constraints::GrappleConstraint;
use crate::lighting::LightingSystem;


#[derive(Clone, Copy, Debug)]
pub(crate) enum RenderSlot {
    Object(usize),
    Particle(usize),
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CanvasMode {
    Landscape,
    Portrait,
    Fullscreen,
}

impl CanvasMode {
    pub(crate) fn aspect_ratio(&self) -> f32 {
        match self {
            CanvasMode::Landscape  => 16.0 / 9.0,
            CanvasMode::Portrait   => 9.0  / 16.0,
            CanvasMode::Fullscreen => 1.0,
        }
    }

    pub(crate) fn virtual_resolution(&self) -> Option<(f32, f32)> {
        match self {
            CanvasMode::Landscape  => Some((3840.0, 2160.0)),
            CanvasMode::Portrait   => Some((2160.0, 3840.0)),
            CanvasMode::Fullscreen => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CanvasLayout {
    pub offsets:                  Vec<(f32, f32)>,
    pub(crate) particle_offsets:  Vec<(f32, f32)>,
    pub(crate) sorted_offsets:    Vec<(f32, f32)>,
    pub canvas_size:              Cell<(f32, f32)>,
    pub mode:                     CanvasMode,
    pub scale:                    Cell<f32>,
    pub safe_area_offset:         Cell<(f32, f32)>,
    pub(crate) zoom:              Cell<f32>,
    pub(crate) sorted_ignore_zoom: Vec<bool>,
    /// Actual window size in physical pixels, updated each frame by build().
    pub(crate) actual_size:       Cell<(f32, f32)>,
}

impl Layout for CanvasLayout {
    fn request_size(&self, _children: Vec<SizeRequest>) -> SizeRequest {
        SizeRequest::new(0.0, 0.0, f32::MAX, f32::MAX)
    }

    fn build(&self, size: (f32, f32), children: Vec<SizeRequest>) -> Vec<Area> {
        assert_eq!(
            self.sorted_offsets.len(),
            children.len(),
            "CanvasLayout: sorted_offsets count must match child count"
        );

        // Store the actual window size so virtual_scale() can use it.
        self.actual_size.set(size);

        let (base_scale, padding_x, padding_y, virtual_res) = match self.mode.virtual_resolution() {
            None => (1.0_f32, 0.0_f32, 0.0_f32, size),
            Some(vres) => {
                let s  = (size.0 / vres.0).min(size.1 / vres.1);
                let pw = (size.0 - vres.0 * s) / 2.0;
                let ph = (size.1 - vres.1 * s) / 2.0;
                (s, pw, ph, vres)
            }
        };

        let zoom = self.zoom.get().max(0.01);
        let scale = base_scale * zoom;

        self.scale.set(scale);
        self.safe_area_offset.set((padding_x, padding_y));
        self.canvas_size.set(virtual_res);

        self.sorted_offsets.iter()
            .copied()
            .zip(self.sorted_ignore_zoom.iter().copied())
            .zip(children)
            .map(|((offset, no_zoom), child)| {
                let s = if no_zoom { base_scale } else { scale };
                let child_size = child.get((f32::MAX, f32::MAX));
                Area {
                    offset: (offset.0 * s + padding_x, offset.1 * s + padding_y),
                    size:   (child_size.0 * s, child_size.1 * s),
                }
            }).collect()
    }
}

// ── Canvas ───────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct PostState {
    pub bloom: Option<BloomSettings>,
}

#[derive(Clone, Debug, Default)]
pub struct GpuFeatureState {
    pub post: PostState,
}

#[derive(Clone)]
pub struct Canvas {
    pub(crate) layout:           CanvasLayout,
    pub(crate) store:            ObjectStore,
    pub(crate) input:            InputState,
    pub        mouse:            MouseState,
    pub(crate) callbacks:        CallbackStore,
    pub(crate) scene_manager:    SceneManager,
    pub(crate) active_camera:    Option<Camera>,
    pub        entropy:          Entropy,
    pub(crate) hot_reload_timer: f32,
    pub(crate) file_watchers:    Vec<file_watcher::FileWatcher>,
    pub        game_vars:        HashMap<String, Value>,
    pub(crate) paused:           bool,
    pub(crate) crystalline:               Option<CrystallinePhysics>,
    pub(crate) particle_system:           Option<ParticleSystem>,
    pub(crate) last_particle_states:      Vec<ParticleState>,
    pub(crate) particle_images:           Vec<Image>,
    pub(crate) image_cache:               crate::assets::ImageCache,
    pub(crate) emitter_locations:         HashMap<String, crate::types::Location>,
    pub(crate) particle_render_layers:    Vec<i32>,
    pub(crate) render_order:              Vec<RenderSlot>,
    /// Per-object grapple constraints. Key = game object name.
    pub(crate) grapple_constraints:       HashMap<String, GrappleConstraint>,
    pub(crate) gpu_features:              Option<GpuFeatureState>,
    pub(crate) lighting:                  Option<LightingSystem>,

    /// WGSL shader sources accumulated via `register_shader_source()`.
    /// Each entry is `(id, label, wgsl_source)`.
    /// Emitted as `EnvelopePayload::RegisterShader` items in `draw_pre`.
    pub(crate) pending_shader_sources: Vec<(String, String, String)>,

    /// Optional capability snapshot set from outside (e.g., from a startup helper).
    /// Quartz reads this before emitting commands to avoid unsupported payloads.
    /// If `None`, Quartz assumes safe conservative defaults.
    pub(crate) capability_snapshot: Option<CapabilitySnapshot>,

    /// Active post-processing override: `(shader_id, params)`.
    /// Emitted as `EnvelopePayload::PostOverride` in `draw_pre` each frame.
    /// Set by `enable_bloom`, custom post shaders, etc.
    pub(crate) active_post_override: Option<(String, Vec<f32>)>,
}

impl std::fmt::Debug for Canvas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Canvas")
            .field("layout",         &self.layout)
            .field("store",          &self.store)
            .field("mouse_position", &self.mouse.position)
            .finish()
    }
}

impl Component for Canvas {
    fn children(&self) -> Vec<&dyn Drawable> {
        self.render_order.iter().map(|slot| match slot {
            RenderSlot::Object(i)   => &self.store.objects[*i] as &dyn Drawable,
            RenderSlot::Particle(i) => &self.particle_images[*i] as &dyn Drawable,
        }).collect()
    }

    fn children_mut(&mut self) -> Vec<&mut dyn Drawable> {
        let order = self.render_order.clone();
        let mut obj_slots: Vec<Option<&mut dyn Drawable>> = self.store.objects.iter_mut()
            .map(|o| Some(o as &mut dyn Drawable)).collect();
        let mut part_slots: Vec<Option<&mut dyn Drawable>> = self.particle_images.iter_mut()
            .map(|i| Some(i as &mut dyn Drawable)).collect();
        order.iter().map(|slot| match slot {
            RenderSlot::Object(i)   => obj_slots[*i].take().unwrap(),
            RenderSlot::Particle(i) => part_slots[*i].take().unwrap(),
        }).collect()
    }

    fn layout(&self) -> &dyn Layout {
        &self.layout
    }

    fn draw_pre(
        &self,
        _sized: &SizedTree,
        _offset: prism::drawable::Offset,
        _bound: prism::drawable::Rect,
    ) -> Vec<(prism::canvas::Area, prism::canvas::Item)> {
        let full_area = prism::canvas::Area { offset: (0.0, 0.0), bounds: None };
        let mut out: Vec<(prism::canvas::Area, prism::canvas::Item)> = vec![];

        // ── Shader registration (Option B) ─────────────────────────────────
        // Emit any pending shader sources as RegisterShader envelopes.
        if !self.pending_shader_sources.is_empty() {
            let payloads: Vec<EnvelopePayload> = self.pending_shader_sources.iter()
                .map(|(id, label, src)| EnvelopePayload::RegisterShader {
                    id: id.clone(),
                    label: label.clone(),
                    wgsl_source: src.clone(),
                })
                .collect();
            out.push((full_area, prism::canvas::Item::FrameEnvelope(FrameEnvelope::new(payloads))));
        }

        // ── Post-processing via PostOverride ────────────────────────────────
        // If bloom (or a custom post shader) is active, emit the PostOverride
        // envelope so DynamicPostRenderer picks it up.
        if let Some(ref override_cmd) = self.active_post_override {
            out.push((full_area, prism::canvas::Item::FrameEnvelope(FrameEnvelope::new(vec![
                EnvelopePayload::PostOverride {
                    shader_id: override_cmd.0.clone(),
                    params: override_cmd.1.clone(),
                },
            ]))));
        }

        // ── Lighting (Options A + C) ────────────────────────────────────────
        let Some(ls) = &self.lighting else {
            return out;
        };

        let (ar, ag, ab, strength, mut lights) = ls.emit_lights();

        // Transform world-space positions to screen-logical space.
        let scale = self.layout.scale.get();
        let (pad_x, pad_y) = self.layout.safe_area_offset.get();
        let (cam_x, cam_y) = if let Some(cam) = &self.active_camera {
            let shake = cam.effects.shake_offset();
            (cam.position.0 + shake.0, cam.position.1 + shake.1)
        } else {
            (0.0, 0.0)
        };

        for light in &mut lights {
            light.position = (
                (light.position.0 - cam_x) * scale + pad_x,
                (light.position.1 - cam_y) * scale + pad_y,
            );
            light.radius *= scale;
        }

        // Collect shadow occluders from visible platform objects.
        let occluders: Vec<prism::canvas::ShadowOccluder> = self.store.objects.iter()
            .filter(|obj| obj.visible && obj.is_platform)
            .map(|obj| prism::canvas::ShadowOccluder {
                position: (
                    (obj.position.0 - cam_x) * scale + pad_x,
                    (obj.position.1 - cam_y) * scale + pad_y,
                ),
                size: (obj.size.0 * scale, obj.size.1 * scale),
            })
            .collect();

        // Item 1: SetLights carries ambient and occluders only.
        // Individual point lights are emitted as PatchLight envelopes below.
        out.push((
            full_area,
            prism::canvas::Item::SetLights {
                ambient_rgb: (ar, ag, ab),
                ambient_strength: strength,
                lights: vec![],
                occluders,
            },
        ));

        // Item 2: FrameEnvelope with one PatchLight per active light.
        let light_payloads: Vec<EnvelopePayload> = lights.iter().enumerate().map(|(i, l)| {
            EnvelopePayload::PatchLight {
                id: format!("__quartz_light_{}", i),
                position: l.position,
                color: l.color,
                intensity: l.intensity,
                radius: l.radius,
            }
        }).collect();

        if !light_payloads.is_empty() {
            out.push((full_area, prism::canvas::Item::FrameEnvelope(FrameEnvelope::new(light_payloads))));
        }

        out
    }

    fn draw_post(
        &self,
        _sized: &SizedTree,
        _offset: prism::drawable::Offset,
        _bound: prism::drawable::Rect,
    ) -> Vec<(prism::canvas::Area, prism::canvas::Item)> {
        // Post-processing is now handled via PostOverride envelopes in draw_pre.
        // Item::PostBloom is no longer emitted.
        vec![]
    }
}