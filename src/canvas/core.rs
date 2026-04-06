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
    pub offsets:             Vec<(f32, f32)>,
    pub(crate) particle_offsets: Vec<(f32, f32)>,
    pub canvas_size:         Cell<(f32, f32)>,
    pub mode:                CanvasMode,
    pub scale:               Cell<f32>,
    pub safe_area_offset:    Cell<(f32, f32)>,
}

impl Layout for CanvasLayout {
    fn request_size(&self, _children: Vec<SizeRequest>) -> SizeRequest {
        SizeRequest::new(0.0, 0.0, f32::MAX, f32::MAX)
    }

    fn build(&self, size: (f32, f32), children: Vec<SizeRequest>) -> Vec<Area> {
        assert_eq!(
            self.offsets.len(),
            children.len(),
            "CanvasLayout: offset count must match child count"
        );

        let (scale, padding_x, padding_y, virtual_res) = match self.mode.virtual_resolution() {
            None => (1.0_f32, 0.0_f32, 0.0_f32, size),
            Some(vres) => {
                let s  = (size.0 / vres.0).min(size.1 / vres.1);
                let pw = (size.0 - vres.0 * s) / 2.0;
                let ph = (size.1 - vres.1 * s) / 2.0;
                (s, pw, ph, vres)
            }
        };

        self.scale.set(scale);
        self.safe_area_offset.set((padding_x, padding_y));
        self.canvas_size.set(virtual_res);

        self.offsets.iter().chain(self.particle_offsets.iter())
            .copied()
            .zip(children)
            .map(|(offset, child)| {
                let child_size = child.get((f32::MAX, f32::MAX));
                Area {
                    offset: (offset.0 * scale + padding_x, offset.1 * scale + padding_y),
                    size:   (child_size.0 * scale, child_size.1 * scale),
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
    pub(crate) crystalline:          Option<CrystallinePhysics>,
    pub(crate) particle_system:      Option<ParticleSystem>,
    pub(crate) last_particle_states: Vec<ParticleState>,
    pub(crate) particle_images:      Vec<Image>,
    pub(crate) image_cache:          HashMap<String, Image>,
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
        self.store.objects.iter().map(|o| o as &dyn Drawable)
            .chain(self.particle_images.iter().map(|i| i as &dyn Drawable))
            .collect()
    }

    fn children_mut(&mut self) -> Vec<&mut dyn Drawable> {
        self.store.objects.iter_mut().map(|o| o as &mut dyn Drawable)
            .chain(self.particle_images.iter_mut().map(|i| i as &mut dyn Drawable))
            .collect()
    }

    fn layout(&self) -> &dyn Layout {
        &self.layout
    }
}