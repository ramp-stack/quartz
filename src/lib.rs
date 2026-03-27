use prism::event::{OnEvent, Event, TickEvent, KeyboardEvent};
use prism::drawable::{Component, Drawable, SizedTree};
use prism::layout::{Area, SizeRequest, Layout};
use std::cell::Cell;
use std::collections::HashMap;

pub use prism::Context;
pub use prism::canvas::{ShapeType, Image, Text, Span, Align, Font, Color};
pub use prism::event::{Key, NamedKey};


// ---------------------------------------------------------------------------
// Modules
// ---------------------------------------------------------------------------

pub mod value;
pub mod entropy;
pub mod lerp;
pub mod object;
pub mod types;
pub mod sprite;
pub mod text;

pub mod sound;
pub mod scene;
pub mod camera;
pub mod store;
pub mod input;
pub mod canvas;
pub mod expr;

// ---------------------------------------------------------------------------
// Re-exports
// ---------------------------------------------------------------------------

pub use types::{
    Action, Condition, GameEvent,
    Target, Location, Anchor,
    MouseButton, ScrollAxis,
};
pub use object::{GameObject, GameObjectBuilder};
pub use sprite::{
    AnimatedSprite, RotationOptions, RotationDirection,
    load_image, load_image_sized,
    flip_horizontal, flip_vertical,
    rotate_cw, rotate_ccw, rotate_180,
};
pub use scene::{Scene, SceneManager};
pub use camera::Camera;
pub use store::ObjectStore;
pub use input::{InputState, Callback, MouseState, MouseCallback, MouseMoveCallback, MouseScrollCallback, CallbackStore, EventCallback};
pub use sound::{SoundOptions, SoundHandle};
pub use expr::{parse_condition, parse_action};
pub use entropy::Entropy;
pub use text::{TextSpec, SpanSpec, make_text, make_text_aligned, make_text_multi};
pub use lerp::Lerp;
 
pub use value::{
    Expr,
    Value,
    MathOp,
    CompOp,
    resolve_expr,
    apply_op,
    compare_operands,
};
 

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
    pub offsets:          Vec<(f32, f32)>,
    pub canvas_size:      Cell<(f32, f32)>,
    pub mode:             CanvasMode,
    pub scale:            Cell<f32>,
    pub safe_area_offset: Cell<(f32, f32)>,
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

        self.offsets.iter().copied().zip(children).map(|(offset, child)| {
            let child_size = child.get((f32::MAX, f32::MAX));
            Area {
                offset: (offset.0 * scale + padding_x, offset.1 * scale + padding_y),
                size:   (child_size.0 * scale, child_size.1 * scale),
            }
        }).collect()
    }
}

#[derive(Clone)]
pub struct Canvas {
    pub(crate) layout:        CanvasLayout,
    pub(crate) store:         ObjectStore,
    pub(crate) input:         InputState,
    pub        mouse:         MouseState,
    pub(crate) callbacks:     CallbackStore,
    pub(crate) scene_manager: SceneManager,
    pub(crate) active_camera: Option<Camera>,
    pub        entropy:       Entropy,
    pub        game_vars:     HashMap<String, value::Value>,
    paused:                   bool,
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
        self.store.objects.iter().map(|o| o as &dyn Drawable).collect()
    }

    fn children_mut(&mut self) -> Vec<&mut dyn Drawable> {
        self.store.objects.iter_mut().map(|o| o as &mut dyn Drawable).collect()
    }

    fn layout(&self) -> &dyn Layout {
        &self.layout
    }
}

impl OnEvent for Canvas {
    fn on_event(
        &mut self,
        _ctx:  &mut Context,
        _tree: &SizedTree,
        event: Box<dyn Event>,
    ) -> Vec<Box<dyn Event>> {
        if let Some(kb_evt) = event.downcast_ref::<KeyboardEvent>() {
            self.handle_keyboard_event(kb_evt);
        }

        if let Some(mouse_evt) = event.downcast_ref::<prism::event::MouseEvent>() {
            self.handle_mouse_event(mouse_evt.clone());
        }

        if let Some(_tick) = event.downcast_ref::<TickEvent>() {
            if self.paused { return vec![event]; }
            const DELTA_TIME: f32 = 0.016;

            // User tick callbacks
            let mut tick_cbs = std::mem::take(&mut self.callbacks.tick);
            tick_cbs.iter_mut().for_each(|cb| cb(self));
            self.callbacks.tick = tick_cbs;

            self.process_held_key_events();
            self.process_all_tick_events();

            // MouseOver events
            if let Some(pos) = self.mouse.position {
                let vpos = self.screen_to_virtual(pos);
                self.process_mouse_over_events(vpos);
            }

            // Custom event handlers attached to objects
            let custom_names: Vec<String> = self.store.events.iter()
                .flatten()
                .filter_map(|e| {
                    if GameEvent::is_custom(e) {
                        e.custom_name().map(str::to_string)
                    } else {
                        None
                    }
                })
                .collect();

            for name in custom_names {
                if let Some(mut handler) = self.callbacks.custom.remove(&name) {
                    handler(self);
                    self.callbacks.custom.insert(name, handler);
                }
            }

            self.update_objects(DELTA_TIME);
            self.handle_collisions();

            let canvas_size = self.layout.canvas_size.get();
            let boundary_indices: Vec<usize> = self.store.objects.iter()
                .enumerate()
                .filter(|(_, obj)| obj.visible && obj.check_boundary_collision(canvas_size))
                .map(|(i, _)| i)
                .collect();
            for idx in boundary_indices {
                self.trigger_boundary_collision_events(idx);
            }
        }

        vec![event]
    }
}

impl Canvas {
    pub fn canvas_size(&self) -> (f32, f32) {
        self.layout.canvas_size.get()
    }

    pub(crate) fn screen_to_virtual(&self, screen_pos: (f32, f32)) -> (f32, f32) {
        let scale = self.layout.scale.get();
        let (pad_x, pad_y) = self.layout.safe_area_offset.get();
        if scale == 0.0 { return screen_pos; }
        ((screen_pos.0 - pad_x) / scale, (screen_pos.1 - pad_y) / scale)
    }

    fn process_all_tick_events(&mut self) {
        let actions: Vec<_> = self.store.events.iter()
            .flatten()
            .filter(|e| GameEvent::is_tick(e))
            .map(|e| e.action().clone())
            .collect();
        actions.into_iter().for_each(|a| self.run(a));
    }
}