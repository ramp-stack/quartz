use prism::drawable::{Component, Drawable};
use prism::layout::{Area, SizeRequest, Layout};
use std::cell::Cell;
use std::collections::HashMap;

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
}