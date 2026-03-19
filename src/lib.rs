use prism::event::{OnEvent, Event, TickEvent, KeyboardEvent, KeyboardState, MouseEvent, MouseState};
use prism::drawable::Component;
use prism::layout::{Area, SizeRequest, Layout};
use std::collections::{HashMap, HashSet};
use std::cell::Cell;
use prism::drawable::SizedTree;

pub use prism::Context;

pub use prism::canvas::{ShapeType, Image, Text, Span, Align, Font, Color};
pub use prism::event::{Key, NamedKey};

mod game_object;
mod animation;
mod apis;
mod scene;
mod camera;
mod mouse;
mod value;

pub use game_object::{
    GameObject, Action, Target, Location, GameEvent, Condition, Anchor,
    MouseButton, ScrollAxis,
};
pub use animation::AnimatedSprite;
pub use scene::{Scene, SceneManager};
pub use camera::Camera;
pub use mouse::{MouseCallback, MouseMoveCallback, MouseScrollCallback};
pub use value::{Value, ComparisonOperator, MathOperator};
pub use value::{compare_operands, resolve_value, apply_op};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CanvasMode {
    Landscape,
    Portrait,
    Fullscreen,
}

impl CanvasMode {
    fn aspect_ratio(&self) -> f32 {
        match self {
            CanvasMode::Landscape => 16.0 / 9.0,
            CanvasMode::Portrait => 9.0 / 16.0,
            CanvasMode::Fullscreen => 1.0,
        }
    }

    fn virtual_resolution(&self) -> Option<(f32, f32)> {
        match self {
            CanvasMode::Landscape => Some((3840.0, 2160.0)),
            CanvasMode::Portrait  => Some((2160.0, 3840.0)),
            CanvasMode::Fullscreen => None,
        }
    }
}


#[derive(Debug, Clone)]
pub struct CanvasLayout {
    offsets: Vec<(f32, f32)>,
    canvas_size: Cell<(f32, f32)>,
    mode: CanvasMode,
    scale: Cell<f32>,
    safe_area_offset: Cell<(f32, f32)>,
}

impl Layout for CanvasLayout {
    fn request_size(&self, _children: Vec<SizeRequest>) -> SizeRequest {
        SizeRequest::new(0.0, 0.0, f32::MAX, f32::MAX)
    }

    fn build(&self, size: (f32, f32), children: Vec<SizeRequest>) -> Vec<Area> {
        if self.offsets.len() != children.len() {
            panic!("CanvasLayout does not have the same number of offsets as children!");
        }

        let (scale, padding_x, padding_y, virtual_res) = match self.mode.virtual_resolution() {
            None => {
                (1.0_f32, 0.0_f32, 0.0_f32, size)
            }
            Some(vres) => {
                let s = (size.0 / vres.0).min(size.1 / vres.1);
                let pw = (size.0 - vres.0 * s) / 2.0;
                let ph = (size.1 - vres.1 * s) / 2.0;
                (s, pw, ph, vres)
            }
        };

        self.scale.set(scale);
        self.safe_area_offset.set((padding_x, padding_y));
        self.canvas_size.set(virtual_res);

        self.offsets
            .iter()
            .copied()
            .zip(children)
            .map(|(offset, child)| {
                let child_size = child.get((f32::MAX, f32::MAX));
                Area {
                    offset: (
                        offset.0 * scale + padding_x,
                        offset.1 * scale + padding_y,
                    ),
                    size: (child_size.0 * scale, child_size.1 * scale),
                }
            })
            .collect()
    }
}


#[derive(Component, Clone)]
pub struct Canvas {
    layout: CanvasLayout,
    objects: Vec<GameObject>,
    #[skip] object_names: Vec<String>,
    #[skip] name_to_index: HashMap<String, usize>,
    #[skip] id_to_index: HashMap<String, usize>,
    #[skip] object_events: Vec<Vec<GameEvent>>,
    #[skip] tag_to_indices: HashMap<String, Vec<usize>>,
    #[skip] held_keys: HashSet<Key>,
    #[skip] tick_callbacks: Vec<Box<dyn EventCallback>>,
    #[skip] custom_event_handlers: HashMap<String, Box<dyn EventCallback>>,
    #[skip] key_press_callbacks: Vec<Box<dyn Callback>>,
    #[skip] key_release_callbacks: Vec<Box<dyn Callback>>,
    #[skip] scene_manager: SceneManager,
    #[skip] active_camera: Option<Camera>,
    #[skip] mouse_position: Option<(f32, f32)>,
    #[skip] hovered_indices: HashSet<usize>,
    #[skip] mouse_press_callbacks: Vec<Box<dyn MouseCallback>>,
    #[skip] mouse_release_callbacks: Vec<Box<dyn MouseCallback>>,
    #[skip] mouse_move_callbacks: Vec<Box<dyn MouseMoveCallback>>,
    #[skip] mouse_scroll_callbacks: Vec<Box<dyn MouseScrollCallback>>,
    #[skip] pub game_vars: HashMap<String, Value>,
}

impl std::fmt::Debug for Canvas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Canvas")
            .field("layout", &self.layout)
            .field("objects", &self.objects)
            .field("mouse_position", &self.mouse_position)
            .field("hovered_indices", &self.hovered_indices)
            .finish()
    }
}


impl Canvas {
    pub fn on_key_press(&mut self, cb: impl FnMut(&mut Canvas, &Key) + Clone + 'static) {
        self.key_press_callbacks.push(Box::new(cb));
    }

    pub fn on_key_release(&mut self, cb: impl FnMut(&mut Canvas, &Key) + Clone + 'static) {
        self.key_release_callbacks.push(Box::new(cb));
    }

    pub fn set_camera(&mut self, camera: Camera) {
        self.active_camera = Some(camera);
    }

    pub fn clear_camera(&mut self) {
        self.active_camera = None;
    }

    pub fn camera(&self) -> Option<&Camera> {
        self.active_camera.as_ref()
    }

    pub fn camera_mut(&mut self) -> Option<&mut Camera> {
        self.active_camera.as_mut()
    }

    fn check_collision(obj1: &GameObject, obj2: &GameObject) -> bool {
        if !obj1.visible || !obj2.visible {
            return false;
        }

        let obj1_right = obj1.position.0 + obj1.size.0;
        let obj1_bottom = obj1.position.1 + obj1.size.1;
        let obj2_right = obj2.position.0 + obj2.size.0;
        let obj2_bottom = obj2.position.1 + obj2.size.1;

        obj1.position.0 < obj2_right
            && obj1_right > obj2.position.0
            && obj1.position.1 < obj2_bottom
            && obj1_bottom > obj2.position.1
    }
}


impl OnEvent for Canvas {
    fn on_event(
        &mut self,
        _ctx: &mut Context,
        _tree: &SizedTree,
        event: Box<dyn Event>,
    ) -> Vec<Box<dyn Event>> {
        if let Some(KeyboardEvent { state, key }) = event.downcast_ref() {
            match state {
                KeyboardState::Pressed if self.held_keys.insert(key.clone()) => {
                    println!("key {key:?}");

                    let key_clone = key.clone();
                    let mut callbacks = std::mem::take(&mut self.key_press_callbacks);
                    for cb in callbacks.iter_mut() {
                        cb(self, &key_clone);
                    }
                    self.key_press_callbacks = callbacks;

                    self.process_key_events(key, GameEvent::is_key_press);
                }
                KeyboardState::Released => {
                    self.held_keys.remove(key);

                    let key_clone = key.clone();
                    let mut callbacks = std::mem::take(&mut self.key_release_callbacks);
                    for cb in callbacks.iter_mut() {
                        cb(self, &key_clone);
                    }
                    self.key_release_callbacks = callbacks;

                    self.process_key_events(key, GameEvent::is_key_release);
                }
                _ => {}
            }
        }

        if let Some(mouse_evt) = event.downcast_ref::<MouseEvent>() {
            self.handle_mouse_event(mouse_evt.clone());
        }

        if let Some(_tick) = event.downcast_ref::<TickEvent>() {
            const DELTA_TIME: f32 = 0.016;

            let mut callbacks = std::mem::take(&mut self.tick_callbacks);
            callbacks.iter_mut().for_each(|cb| cb(self));
            self.tick_callbacks = callbacks;

            let held_keys = self.held_keys.clone();
            self.process_all_events(GameEvent::is_key_hold, |e| {
                e.key().map_or(false, |key| held_keys.contains(key))
            });

            self.process_all_events(GameEvent::is_tick, |_| true);

            if let Some(pos) = self.mouse_position {
                let virtual_pos = self.screen_to_virtual(pos);
                self.process_mouse_over_events(virtual_pos);
            }

            let custom_event_names: Vec<String> = (0..self.objects.len())
                .filter_map(|idx| self.object_events.get(idx))
                .flatten()
                .filter_map(|e| {
                    if GameEvent::is_custom(e) {
                        e.custom_name().map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .collect();

            for name in custom_event_names {
                if let Some(mut handler) = self.custom_event_handlers.remove(&name) {
                    handler(self);
                    self.custom_event_handlers.insert(name, handler);
                }
            }

            self.update_objects(DELTA_TIME);
            self.handle_collisions();
        }

        vec![event]
    }
}

impl Canvas {
    pub(crate) fn screen_to_virtual(&self, screen_pos: (f32, f32)) -> (f32, f32) {
        let scale = self.layout.scale.get();
        let (pad_x, pad_y) = self.layout.safe_area_offset.get();
        if scale == 0.0 {
            return screen_pos;
        }
        (
            (screen_pos.0 - pad_x) / scale,
            (screen_pos.1 - pad_y) / scale,
        )
    }
}

impl Canvas {
    fn process_key_events<F>(&mut self, key: &Key, predicate: F)
    where
        F: Fn(&GameEvent) -> bool,
    {
        let actions: Vec<_> = (0..self.objects.len())
            .filter_map(|idx| self.object_events.get(idx))
            .flatten()
            .filter(|e| predicate(e) && e.key() == Some(key))
            .map(|e| e.action().clone())
            .collect();

        actions.into_iter().for_each(|action| self.run(action));
    }

    fn process_all_events<F, P>(&mut self, predicate: F, should_run: P)
    where
        F: Fn(&GameEvent) -> bool,
        P: Fn(&GameEvent) -> bool,
    {
        let actions: Vec<_> = (0..self.objects.len())
            .filter_map(|idx| self.object_events.get(idx))
            .flatten()
            .filter(|e| predicate(e) && should_run(e))
            .map(|e| e.action().clone())
            .collect();

        actions.into_iter().for_each(|action| self.run(action));
    }

    fn update_objects(&mut self, delta_time: f32) {
        let scale = self.layout.scale.get();

        for (idx, obj) in self.objects.iter_mut().enumerate() {
            obj.scaled_size.set((obj.size.0 * scale, obj.size.1 * scale));
            obj.update_animation(delta_time);

            if obj.animated_sprite.is_none() {
                obj.update_image_shape();
            }

            if obj.visible {
                obj.apply_gravity();
                obj.update_position();
                obj.apply_resistance();
                self.layout.offsets[idx] = obj.position;
            }
        }

        self.handle_infinite_scroll();
        self.apply_camera_transform();
    }

    fn apply_camera_transform(&mut self) {
        let mut cam = match self.active_camera.take() {
            Some(c) => c,
            None => return,
        };

        if let Some(target) = cam.follow_target.clone() {
            if let Some(idx) = self.get_target_indices(&target).first().copied() {
                if let Some(obj) = self.objects.get(idx) {
                    let cx = obj.position.0 + obj.size.0 * 0.5;
                    let cy = obj.position.1 + obj.size.1 * 0.5;
                    cam.lerp_toward(cx, cy);
                }
            }
        }

        let cam_x = cam.position.0;
        let cam_y = cam.position.1;
        for (idx, obj) in self.objects.iter().enumerate() {
            self.layout.offsets[idx] = (obj.position.0 - cam_x, obj.position.1 - cam_y);
        }

        self.active_camera = Some(cam);
    }

    fn handle_collisions(&mut self) {
        let mut adjustments = Vec::new();
        let mut collision_pairs = Vec::new();

        for i in 0..self.objects.len() {
            if !self.objects[i].visible { continue; }
            for j in (i + 1)..self.objects.len() {
                if !self.objects[j].visible { continue; }

                let obj1 = &self.objects[i];
                let obj2 = &self.objects[j];

                if Self::check_collision(obj1, obj2) {
                    if obj2.is_platform && obj1.momentum.1 > 0.0 {
                        let obj1_bottom = obj1.position.1 + obj1.size.1;
                        if obj1_bottom > obj2.position.1 {
                            adjustments.push((i, obj2.position.1 - obj1.size.1));
                        }
                    } else if obj1.is_platform && obj2.momentum.1 > 0.0 {
                        let obj2_bottom = obj2.position.1 + obj2.size.1;
                        if obj2_bottom > obj1.position.1 {
                            adjustments.push((j, obj1.position.1 - obj2.size.1));
                        }
                    }
                    if !obj1.is_platform && !obj2.is_platform {
                        collision_pairs.push((i, j));
                    }
                }
            }
        }

        for (idx, new_y) in adjustments {
            self.objects[idx].position.1 = new_y;
            self.objects[idx].momentum.1 = 0.0;
            let cam_offset = self.active_camera.as_ref().map(|c| c.position).unwrap_or((0.0, 0.0));
            self.layout.offsets[idx] = (
                self.objects[idx].position.0 - cam_offset.0,
                self.objects[idx].position.1 - cam_offset.1,
            );
        }

        for (i, j) in collision_pairs {
            self.trigger_collision_events(i);
            self.trigger_collision_events(j);
        }
    }

    fn evaluate_condition(&self, condition: &Condition) -> bool {
        match condition {
            Condition::Always => true,
            Condition::KeyHeld(key) => self.held_keys.contains(key),
            Condition::KeyNotHeld(key) => !self.held_keys.contains(key),
            Condition::Collision(target) => {
                self.get_target_indices(target).iter().any(|&idx1| {
                    (0..self.objects.len()).any(|idx2| {
                        if idx1 == idx2 { return false; }
                        match (self.objects.get(idx1), self.objects.get(idx2)) {
                            (Some(obj1), Some(obj2)) => Self::check_collision(obj1, obj2),
                            _ => false,
                        }
                    })
                })
            }
            Condition::NoCollision(target) => !self.evaluate_condition(&Condition::Collision(target.clone())),
            Condition::And(c1, c2) => self.evaluate_condition(c1) && self.evaluate_condition(c2),
            Condition::Or(c1, c2) => self.evaluate_condition(c1) || self.evaluate_condition(c2),
            Condition::Not(c) => !self.evaluate_condition(c),
            Condition::IsVisible(target) => {
                self.get_target_indices(target).iter()
                    .any(|&idx| self.objects.get(idx).map_or(false, |obj| obj.visible))
            }
            Condition::IsHidden(target) => {
                self.get_target_indices(target).iter()
                    .any(|&idx| self.objects.get(idx).map_or(true, |obj| !obj.visible))
            }
            //=================================================
            //synful additions
            Condition::Compare(left, op, right) => {
                match (resolve_value(left, &self.game_vars), resolve_value(right, &self.game_vars)) {
                    (Some(l), Some(r)) => compare_operands(&l, op, &r).unwrap_or(false),
                    _ => false,
                }
            }
            Condition::VarExists(name) => self.game_vars.contains_key(name),
            Condition::Grounded(target) => {
                self.get_target_indices(target).iter().any(|&idx| {
                    if let Some(obj) = self.objects.get(idx) {
                        let obj_bottom = obj.position.1 + obj.size.1;
                        self.objects.iter().any(|other| {
                            other.is_platform && (obj_bottom - other.position.1).abs() < 2.0 && obj.position.0 + obj.size.0 > other.position.0 && obj.position.0 < other.position.0 + other.size.0 && obj.momentum.1 >= 0.0
                        })
                    } else {
                        false                        
                    }
                })
            }
        }
    }

    fn trigger_collision_events(&mut self, idx: usize) {
        let actions: Vec<_> = self.object_events.get(idx)
            .into_iter()
            .flatten()
            .filter_map(|e| if let GameEvent::Collision { action, .. } = e { Some(action.clone()) } else { None })
            .collect();
        actions.into_iter().for_each(|action| self.run(action));
    }

    fn trigger_boundary_collision_events(&mut self, idx: usize) {
        let actions: Vec<_> = self.object_events.get(idx)
            .into_iter()
            .flatten()
            .filter_map(|e| if let GameEvent::BoundaryCollision { action, .. } = e { Some(action.clone()) } else { None })
            .collect();
        actions.into_iter().for_each(|action| self.run(action));
    }

    pub fn register_custom_event<F>(&mut self, name: String, handler: F)
    where
        F: FnMut(&mut Canvas) + Clone + 'static,
    {
        self.custom_event_handlers.insert(name, Box::new(handler));
    }

    fn apply_to_targets<F>(&mut self, target: &Target, mut f: F)
    where
        F: FnMut(&mut GameObject),
    {
        let indices = self.get_target_indices(target);
        for idx in indices {
            if let Some(obj) = self.objects.get_mut(idx) { f(obj); }
        }
    }

    fn get_target_indices(&self, target: &Target) -> Vec<usize> {
        match target {
            Target::ByName(name) => self.name_to_index.get(name).map(|&idx| vec![idx]).unwrap_or_default(),
            Target::ById(id) => self.id_to_index.get(id).map(|&idx| vec![idx]).unwrap_or_default(),
            Target::ByTag(tag) => self.tag_to_indices.get(tag).cloned().unwrap_or_default(),
        }
    }

    fn get_target_names(&self, target: &Target) -> Vec<String> {
        self.get_target_indices(target).iter()
            .filter_map(|&idx| self.object_names.get(idx).cloned())
            .collect()
    }
}

impl Location {
    fn resolve_position(&self, canvas: &Canvas) -> (f32, f32) {
        match self {
            Location::Position(pos) => *pos,
            Location::AtTarget(target) => {
                canvas.get_target_indices(target).first()
                    .and_then(|&idx| canvas.objects.get(idx))
                    .map(|obj| obj.position)
                    .unwrap_or((0.0, 0.0))
            }
            Location::Between(target1, target2) => {
                let pos1 = canvas.get_target_indices(target1).first()
                    .and_then(|&idx| canvas.objects.get(idx))
                    .map(|obj| obj.position).unwrap_or((0.0, 0.0));
                let pos2 = canvas.get_target_indices(target2).first()
                    .and_then(|&idx| canvas.objects.get(idx))
                    .map(|obj| obj.position).unwrap_or((0.0, 0.0));
                ((pos1.0 + pos2.0) / 2.0, (pos1.1 + pos2.1) / 2.0)
            }
            Location::Relative { target, offset } => {
                canvas.get_target_indices(target).first()
                    .and_then(|&idx| canvas.objects.get(idx))
                    .map(|obj| (obj.position.0 + offset.0, obj.position.1 + offset.1))
                    .unwrap_or(*offset)
            }
            Location::OnTarget { target, anchor, offset } => {
                canvas.get_target_indices(target).first()
                    .and_then(|&idx| canvas.objects.get(idx))
                    .map(|obj| {
                        let anchor_pos = obj.get_anchor_position(*anchor);
                        (anchor_pos.0 + offset.0, anchor_pos.1 + offset.1)
                    })
                    .unwrap_or(*offset)
            }
        }
    }
}


pub trait EventCallback: FnMut(&mut Canvas) + 'static {
    fn clone_box(&self) -> Box<dyn EventCallback>;
}
impl PartialEq for dyn EventCallback {
    fn eq(&self, _: &Self) -> bool { true }
}
impl<F> EventCallback for F where F: FnMut(&mut Canvas) + Clone + 'static {
    fn clone_box(&self) -> Box<dyn EventCallback> { Box::new(self.clone()) }
}
impl Clone for Box<dyn EventCallback> {
    fn clone(&self) -> Self { self.as_ref().clone_box() }
}
impl std::fmt::Debug for dyn EventCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "Clonable Closure") }
}

pub trait Callback: FnMut(&mut Canvas, &Key) + 'static {
    fn clone_box(&self) -> Box<dyn Callback>;
}
impl PartialEq for dyn Callback {
    fn eq(&self, _: &Self) -> bool { true }
}
impl<F> Callback for F where F: FnMut(&mut Canvas, &Key) + Clone + 'static {
    fn clone_box(&self) -> Box<dyn Callback> { Box::new(self.clone()) }
}
impl Clone for Box<dyn Callback> {
    fn clone(&self) -> Self { self.as_ref().clone_box() }
}
impl std::fmt::Debug for dyn Callback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "Clonable Closure") }
}