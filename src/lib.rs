use prism::event::{OnEvent, Event, TickEvent, KeyboardEvent, KeyboardState};
use prism::drawable::Component;
use prism::layout::{Area, SizeRequest, Layout};
use std::collections::{HashMap, HashSet};
use std::cell::Cell;
use prism::drawable::SizedTree;

pub use prism::Context;

pub use prism::canvas::{ShapeType, Image};
pub use prism::event::Key;

mod game_object;
mod animation;
mod apis;

pub use game_object::{GameObject, Action, Target, Location, GameEvent, Condition, Anchor};
pub use animation::AnimatedSprite;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CanvasMode {
    Landscape, 
    Portrait,  
}

impl CanvasMode {
    fn aspect_ratio(&self) -> f32 {
        match self {
            CanvasMode::Landscape => 16.0 / 9.0,
            CanvasMode::Portrait => 9.0 / 16.0,
        }
    }
    
    fn virtual_resolution(&self) -> (f32, f32) {
        match self {
            CanvasMode::Landscape => (3840.0, 2160.0),
            CanvasMode::Portrait => (2160.0, 3840.0),
        }
    }
}

#[derive(Debug)]
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
        
        let virtual_res = self.mode.virtual_resolution();
        
        let scale_x = size.0 / virtual_res.0;
        let scale_y = size.1 / virtual_res.1;
        
        let scale = scale_x.min(scale_y);
        
        let canvas_width = virtual_res.0 * scale;
        let canvas_height = virtual_res.1 * scale;
    
        let padding_x = (size.0 - canvas_width) / 2.0;
        let padding_y = (size.1 - canvas_height) / 2.0;
        
        self.scale.set(scale);
        self.safe_area_offset.set((padding_x, padding_y));
        self.canvas_size.set(virtual_res);
        
        self.offsets.iter().copied().zip(children).map(|(offset, child)| {
            let child_size = child.get((f32::MAX, f32::MAX));
            
            Area {
                offset: (
                    offset.0 * scale + padding_x,
                    offset.1 * scale + padding_y
                ),
                size: (
                    child_size.0 * scale,
                    child_size.1 * scale
                )
            }
        }).collect()
    }
}

#[derive(Component)]
pub struct Canvas {
    layout: CanvasLayout,
    objects: Vec<GameObject>,
    #[skip] object_names: Vec<String>,
    #[skip] name_to_index: HashMap<String, usize>,
    #[skip] id_to_index: HashMap<String, usize>,
    #[skip] object_events: Vec<Vec<GameEvent>>,
    #[skip] tag_to_indices: HashMap<String, Vec<usize>>,
    #[skip] held_keys: HashSet<Key>,
    #[skip] tick_callbacks: Vec<Box<dyn FnMut(&mut Canvas) + 'static>>,
    #[skip] custom_event_handlers: HashMap<String, Box<dyn FnMut(&mut Canvas) + 'static>>,
}

impl std::fmt::Debug for Canvas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Canvas")
            .field("layout", &self.layout)
            .field("objects", &self.objects)
            .field("object_names", &self.object_names)
            .field("name_to_index", &self.name_to_index)
            .field("id_to_index", &self.id_to_index)
            .field("object_events", &self.object_events)
            .field("tag_to_indices", &self.tag_to_indices)
            .field("held_keys", &self.held_keys)
            .field("tick_callbacks", &format!("<{} callbacks>", self.tick_callbacks.len()))
            .field("custom_event_handlers", &format!("<{} handlers>", self.custom_event_handlers.len()))
            .finish()
    }
}

impl OnEvent for Canvas {
    fn on_event(&mut self, _ctx: &mut Context, _tree: &SizedTree, event: Box<dyn Event>) -> Vec<Box<dyn Event>> {
        if let Some(KeyboardEvent { state, key }) = event.downcast_ref() {
            match state {
                KeyboardState::Pressed => {
                    let is_new_press = !self.held_keys.contains(key);
                    
                    if is_new_press {
                        self.held_keys.insert(key.clone());
                        
                        for idx in 0..self.objects.len() {
                            if let Some(events) = self.object_events.get(idx).cloned() {
                                for game_event in events {
                                    if let GameEvent::KeyPress { key: event_key, action, target: _ } = game_event {
                                        if &event_key == key {
                                            self.run(action);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                KeyboardState::Released => {
                    self.held_keys.remove(key);
                    
                    for idx in 0..self.objects.len() {
                        if let Some(events) = self.object_events.get(idx).cloned() {
                            for game_event in events {
                                if let GameEvent::KeyRelease { key: event_key, action, target: _ } = game_event {
                                    if &event_key == key {
                                        self.run(action);
                                    }
                                }
                            }
                        }
                    }
                }
                KeyboardState::Repeated => {
                }
            }
        }
        
        if let Some(_tick) = event.downcast_ref::<TickEvent>() {
            const DELTA_TIME: f32 = 0.016; 
            
            let scale = self.layout.scale.get();
            
            let mut callbacks = std::mem::take(&mut self.tick_callbacks);
            for callback in &mut callbacks {
                callback(self);
            }
            self.tick_callbacks = callbacks;
            
            for idx in 0..self.objects.len() {
                if let Some(events) = self.object_events.get(idx).cloned() {
                    for game_event in events {
                        if let GameEvent::KeyHold { key: event_key, action, target: _ } = game_event {
                            if self.held_keys.contains(&event_key) {
                                self.run(action);
                            }
                        }
                    }
                }
            }
            
            for idx in 0..self.objects.len() {
                if let Some(events) = self.object_events.get(idx).cloned() {
                    for game_event in events {
                        if let GameEvent::Tick { action, target: _ } = game_event {
                            self.run(action);
                        }
                    }
                }
            }
            
            for idx in 0..self.objects.len() {
                if let Some(events) = self.object_events.get(idx).cloned() {
                    for game_event in events {
                        if let GameEvent::Custom { name, target: _ } = game_event {
                            self.trigger_custom_event(&name);
                        }
                    }
                }
            }
            
            for idx in 0..self.objects.len() {
                if let Some(game_obj) = self.objects.get_mut(idx) {
                    let scaled_size = (game_obj.size.0 * scale, game_obj.size.1 * scale);
                    game_obj.scaled_size.set(scaled_size);
                    
                    game_obj.update_animation(DELTA_TIME);
                    
                    if game_obj.animated_sprite.is_none() {
                        game_obj.update_image_shape();
                    }
                    
                    if game_obj.visible {
                        game_obj.apply_gravity();
                        game_obj.update_position();
                        game_obj.apply_resistance();
                        self.layout.offsets[idx] = game_obj.position;
                    }
                }
            }
            
            self.handle_infinite_scroll();
            
            for i in 0..self.objects.len() {
                for j in 0..self.objects.len() {
                    if i == j {
                        continue;
                    }
                    
                    let is_platform = self.objects.get(j).map(|obj| obj.is_platform).unwrap_or(false);
                    if !is_platform {
                        continue;
                    }
                    
                    let is_visible = self.objects.get(i).map(|obj| obj.visible).unwrap_or(false);
                    if !is_visible {
                        continue;
                    }
                    
                    if self.check_collision(i, j) {
                        let (platform_pos, platform_size) = if let Some(platform) = self.objects.get(j) {
                            (platform.position, platform.size)
                        } else {
                            continue;
                        };
                        
                        if let Some(obj) = self.objects.get_mut(i) {
                            let obj_bottom = obj.position.1 + obj.size.1;
                            let platform_top = platform_pos.1;
                            
                            if obj.momentum.1 > 0.0 && obj_bottom > platform_top {
                                obj.position.1 = platform_top - obj.size.1;
                                obj.momentum.1 = 0.0; 
                                self.layout.offsets[i] = obj.position;
                            }
                        }
                    }
                }
            }
            
            for i in 0..self.objects.len() {
                for j in (i + 1)..self.objects.len() {
                    if self.check_collision(i, j) {
                        self.trigger_collision_events(i);
                        self.trigger_collision_events(j);
                    }
                }
            }
            
            let canvas_size = self.layout.canvas_size.get();
            let mut boundary_collisions = Vec::new();
            for idx in 0..self.objects.len() {
                if let Some(obj) = self.objects.get(idx) {
                    if obj.check_boundary_collision(canvas_size) {
                        boundary_collisions.push(idx);
                    }
                }
            }
            
            for idx in boundary_collisions {
                self.trigger_boundary_collision_events(idx);
            }
        }

        vec![event]
    }
}

impl Canvas {
    fn check_collision(&self, idx1: usize, idx2: usize) -> bool {
        let obj1 = match self.objects.get(idx1) {
            Some(obj) => obj,
            None => return false,
        };
        let obj2 = match self.objects.get(idx2) {
            Some(obj) => obj,
            None => return false,
        };
        
        if !obj1.visible || !obj2.visible {
            return false;
        }
        
        let obj1_right = obj1.position.0 + obj1.size.0;
        let obj1_bottom = obj1.position.1 + obj1.size.1;
        let obj2_right = obj2.position.0 + obj2.size.0;
        let obj2_bottom = obj2.position.1 + obj2.size.1;
        
        obj1.position.0 < obj2_right &&
        obj1_right > obj2.position.0 &&
        obj1.position.1 < obj2_bottom &&
        obj1_bottom > obj2.position.1
    }
    
    fn evaluate_condition(&self, condition: &Condition) -> bool {
        match condition {
            Condition::Always => true,
            Condition::KeyHeld(key) => self.held_keys.contains(key),
            Condition::KeyNotHeld(key) => !self.held_keys.contains(key),
            Condition::Collision(target) => {
                let indices = self.get_target_indices(target);
                for &idx1 in &indices {
                    for idx2 in 0..self.objects.len() {
                        if idx1 != idx2 && self.check_collision(idx1, idx2) {
                            return true;
                        }
                    }
                }
                false
            }
            Condition::NoCollision(target) => {
                !self.evaluate_condition(&Condition::Collision(target.clone()))
            }
            Condition::And(cond1, cond2) => {
                self.evaluate_condition(cond1) && self.evaluate_condition(cond2)
            }
            Condition::Or(cond1, cond2) => {
                self.evaluate_condition(cond1) || self.evaluate_condition(cond2)
            }
            Condition::Not(cond) => {
                !self.evaluate_condition(cond)
            }
            Condition::IsVisible(target) => {
                let indices = self.get_target_indices(target);
                indices.iter().any(|&idx| {
                    self.objects.get(idx).map(|obj| obj.visible).unwrap_or(false)
                })
            }
            Condition::IsHidden(target) => {
                let indices = self.get_target_indices(target);
                indices.iter().any(|&idx| {
                    self.objects.get(idx).map(|obj| !obj.visible).unwrap_or(true)
                })
            }
        }
    }
    
    fn trigger_collision_events(&mut self, idx: usize) {
        if let Some(events) = self.object_events.get(idx).cloned() {
            for event in events {
                if let GameEvent::Collision { action, target: _ } = event {
                    self.run(action);
                }
            }
        }
    }
    
    fn trigger_boundary_collision_events(&mut self, idx: usize) {
        if let Some(events) = self.object_events.get(idx).cloned() {
            let mut actions_to_run = Vec::new();
            for event in events {
                if let GameEvent::BoundaryCollision { action, target: _ } = event {
                    actions_to_run.push(action);
                }
            }
            
            for action in actions_to_run {
                self.run(action);
            }
        }
    }
    
    fn apply_to_targets<F>(&mut self, target: &Target, mut f: F)
    where
        F: FnMut(&mut GameObject),
    {
        let indices = self.get_target_indices(target);
        for idx in indices {
            if let Some(obj) = self.objects.get_mut(idx) {
                f(obj);
            }
        }
    }
    
    fn get_target_indices(&self, target: &Target) -> Vec<usize> {
        match target {
            Target::ByName(name) => {
                self.name_to_index.get(name)
                    .map(|&idx| vec![idx])
                    .unwrap_or_else(Vec::new)
            }
            Target::ById(id) => {
                self.id_to_index.get(id)
                    .map(|&idx| vec![idx])
                    .unwrap_or_else(Vec::new)
            }
            Target::ByTag(tag) => {
                self.tag_to_indices.get(tag).cloned().unwrap_or_else(Vec::new)
            }
        }
    }
    
    fn get_target_names(&self, target: &Target) -> Vec<String> {
        let indices = self.get_target_indices(target);
        indices.iter()
            .filter_map(|&idx| self.object_names.get(idx))
            .cloned()
            .collect()
    }
}

impl Location {
    fn resolve_position(&self, canvas: &Canvas) -> (f32, f32) {
        match self {
            Location::Position(pos) => *pos,
            Location::AtTarget(target) => {
                if let Some(idx) = canvas.get_target_indices(target).first() {
                    if let Some(obj) = canvas.objects.get(*idx) {
                        obj.position
                    } else {
                        (0.0, 0.0)
                    }
                } else {
                    (0.0, 0.0)
                }
            }
            Location::Between(target1, target2) => {
                let pos1 = canvas.get_target_indices(target1).first()
                    .and_then(|&idx| canvas.objects.get(idx))
                    .map(|obj| obj.position)
                    .unwrap_or((0.0, 0.0));
                let pos2 = canvas.get_target_indices(target2).first()
                    .and_then(|&idx| canvas.objects.get(idx))
                    .map(|obj| obj.position)
                    .unwrap_or((0.0, 0.0));
                ((pos1.0 + pos2.0) / 2.0, (pos1.1 + pos2.1) / 2.0)
            }
            Location::Relative { target, offset } => {
                if let Some(idx) = canvas.get_target_indices(target).first() {
                    if let Some(obj) = canvas.objects.get(*idx) {
                        (obj.position.0 + offset.0, obj.position.1 + offset.1)
                    } else {
                        *offset
                    }
                } else {
                    *offset
                }
            }
            Location::OnTarget { target, anchor, offset } => {
                if let Some(idx) = canvas.get_target_indices(target).first() {
                    if let Some(obj) = canvas.objects.get(*idx) {
                        let anchor_pos = obj.get_anchor_position(*anchor);
                        (anchor_pos.0 + offset.0, anchor_pos.1 + offset.1)
                    } else {
                        *offset
                    }
                } else {
                    *offset
                }
            }
        }
    }
}


