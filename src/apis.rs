use super::*;
use rodio::{Decoder, OutputStreamBuilder, Sink};
use std::fs::File;
use std::io::BufReader;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};

impl Canvas {
    pub fn new(_ctx: &mut Context, mode: CanvasMode) -> Self {
        let virtual_res = mode.virtual_resolution();
        Self {
            layout: CanvasLayout {
                offsets: Vec::new(),
                canvas_size: Cell::new(virtual_res),
                mode,
                scale: Cell::new(1.0),
                safe_area_offset: Cell::new((0.0, 0.0)),
            },
            objects: Vec::new(),
            object_names: Vec::new(),
            name_to_index: HashMap::new(),
            id_to_index: HashMap::new(),
            object_events: Vec::new(),
            tag_to_indices: HashMap::new(),
            held_keys: HashSet::new(),
            tick_callbacks: Vec::new(),
            custom_event_handlers: HashMap::new(),
            key_press_callbacks: Vec::new(),
            key_release_callbacks: Vec::new(),
        }
    }
    
    pub fn add_game_object(&mut self, name: String, game_obj: GameObject) {
        let position = game_obj.position;
        let id = game_obj.id.clone();
        let tags = game_obj.tags.clone();
        
        let idx = self.objects.len();
        
        self.layout.offsets.push(position);
        self.name_to_index.insert(name.clone(), idx);
        self.id_to_index.insert(id, idx);
        
        for tag in tags {
            self.tag_to_indices.entry(tag).or_insert_with(Vec::new).push(idx);
        }
        
        self.object_names.push(name);
        self.objects.push(game_obj);
        self.object_events.push(Vec::new());
    }
    
    pub fn remove_game_object(&mut self, name: &str) {
        if let Some(&idx) = self.name_to_index.get(name) {
            let removed_obj = self.objects.remove(idx);
            let removed_name = self.object_names.remove(idx);
            
            self.layout.offsets.remove(idx);
            self.object_events.remove(idx);
            
            self.name_to_index.remove(&removed_name);
            self.id_to_index.remove(&removed_obj.id);
            
            for tag in &removed_obj.tags {
                if let Some(indices) = self.tag_to_indices.get_mut(tag) {
                    indices.retain(|&i| i != idx);
                }
            }
            
            self.name_to_index.values_mut().for_each(|i| if *i > idx { *i -= 1 });
            self.id_to_index.values_mut().for_each(|i| if *i > idx { *i -= 1 });
            self.tag_to_indices.values_mut().for_each(|indices| {
                indices.iter_mut().for_each(|i| if *i > idx { *i -= 1 });
            });
        }
    }
    
    pub fn run(&mut self, action: Action) {
        match action {
            Action::ApplyMomentum { target, value } => {
                self.apply_to_targets(&target, |obj| {
                    obj.momentum.0 += value.0;
                    obj.momentum.1 += value.1;
                });
            }
            Action::SetMomentum { target, value } => {  
                self.apply_to_targets(&target, |obj| obj.momentum = value);
            }
            Action::SetResistance { target, value } => {
                self.apply_to_targets(&target, |obj| obj.resistance = value);
            }
            Action::Remove { target } => {
                let names = self.get_target_names(&target);
                for name in names {
                    self.remove_game_object(&name);
                }
            }
            Action::Spawn { object, location } => {
                let position = location.resolve_position(self);
                let mut new_obj = *object;
                new_obj.position = position;
                let name = format!("spawned_{}", new_obj.id);
                self.add_game_object(name, new_obj);
            }
            Action::TransferMomentum { from, to, scale } => {
                let from_indices = self.get_target_indices(&from);
                let (total_momentum, count) = from_indices.iter()
                    .filter_map(|&idx| self.objects.get(idx))
                    .fold(((0.0, 0.0), 0), |(acc, cnt), obj| {
                        ((acc.0 + obj.momentum.0, acc.1 + obj.momentum.1), cnt + 1)
                    });
                
                if count > 0 {
                    let avg_momentum = (total_momentum.0 / count as f32, total_momentum.1 / count as f32);
                    let scaled_momentum = (avg_momentum.0 * scale, avg_momentum.1 * scale);
                    self.apply_to_targets(&to, |obj| obj.momentum = scaled_momentum);
                }
            }
            Action::SetAnimation { target, animation_bytes, fps } => {
                let indices = self.get_target_indices(&target);
                for idx in indices {
                    if let Some(obj) = self.objects.get_mut(idx) {
                        if let Ok(new_animation) = AnimatedSprite::new(animation_bytes, obj.size, fps) {
                            obj.set_animation(new_animation);
                        }
                    }
                }
            }
            Action::Teleport { target, location } => {
                let position = location.resolve_position(self);
                let indices = self.get_target_indices(&target);
                for idx in indices {
                    if let Some(obj) = self.objects.get_mut(idx) {
                        obj.position = position;
                        self.layout.offsets[idx] = position;
                    }
                }
            }
            Action::Show { target } => {
                self.apply_to_targets(&target, |obj| obj.visible = true);
            }
            Action::Hide { target } => {
                self.apply_to_targets(&target, |obj| obj.visible = false);
            }
            Action::Toggle { target } => {
                self.apply_to_targets(&target, |obj| obj.visible = !obj.visible);
            }
            Action::Conditional { condition, if_true, if_false } => {
                if self.evaluate_condition(&condition) {
                    self.run(*if_true);
                } else if let Some(false_action) = if_false {
                    self.run(*false_action);
                }
            }
            Action::Custom { name } => {
                if let Some(mut handler) = self.custom_event_handlers.remove(&name) {
                    handler(self);
                    self.custom_event_handlers.insert(name, handler);
                }
            }
        }
    }
    
    pub fn add_event(&mut self, event: GameEvent, target: Target) {
        let indices = self.get_target_indices(&target);
        for idx in indices {
            if let Some(events) = self.object_events.get_mut(idx) {
                events.push(event.clone());
            }
        }
    }
    
    pub fn collision_between(&self, target1: &Target, target2: &Target) -> bool {
        let indices1 = self.get_target_indices(target1);
        let indices2 = self.get_target_indices(target2);
        
        indices1.iter().any(|&idx1| {
            indices2.iter().any(|&idx2| {
                if idx1 == idx2 {
                    return false;
                }
                
                match (self.objects.get(idx1), self.objects.get(idx2)) {
                    (Some(obj1), Some(obj2)) => Self::check_collision(obj1, obj2),
                    _ => false,
                }
            })
        })
    }

    pub fn objects_in_radius(&self, game_object: &GameObject, radius_px: f32) -> Vec<&GameObject> {
        let center_x = game_object.position.0 + game_object.size.0 / 2.0;
        let center_y = game_object.position.1 + game_object.size.1 / 2.0;
        
        let radius_squared = radius_px * radius_px;
        
        self.objects.iter()
            .filter(|obj| {
                if obj.id == game_object.id {
                    return false;
                }
                
                if !obj.visible {
                    return false;
                }
                
                let obj_center_x = obj.position.0 + obj.size.0 / 2.0;
                let obj_center_y = obj.position.1 + obj.size.1 / 2.0;
                
                let dx = obj_center_x - center_x;
                let dy = obj_center_y - center_y;
                let distance_squared = dx * dx + dy * dy;
                
                distance_squared <= radius_squared
            })
            .collect()
    }
    
    pub fn handle_infinite_scroll(&mut self) {
        let bg_indices = self.get_target_indices(&Target::ByTag("scroll".to_string()));
        
        if bg_indices.len() < 2 {
            return; 
        }
        
        for &idx in &bg_indices {
            if let Some(obj) = self.objects.get(idx) {
                let right_edge = obj.position.0 + obj.size.0;
                
                if right_edge <= -10.0 {
                    let max_right_edge = bg_indices.iter()
                        .filter(|&&other_idx| other_idx != idx)
                        .filter_map(|&other_idx| self.objects.get(other_idx))
                        .map(|other_obj| other_obj.position.0 + other_obj.size.0)
                        .fold(f32::MIN, f32::max);
                    
                    if let Some(obj) = self.objects.get_mut(idx) {
                        obj.position.0 = max_right_edge;
                        self.layout.offsets[idx] = obj.position;
                    }
                }
            }
        }
    }

    pub fn get_virtual_size(&self) -> (f32, f32) {
        self.layout.canvas_size.get()
    }
    
    pub fn on_tick<F>(&mut self, callback: F)
    where
        F: FnMut(&mut Canvas) + 'static,
    {
        self.tick_callbacks.push(Box::new(callback));
    }
    
    pub fn get_game_object(&self, name: &str) -> Option<&GameObject> {
        self.name_to_index.get(name)
            .and_then(|&idx| self.objects.get(idx))
    }
    
    pub fn get_game_object_mut(&mut self, name: &str) -> Option<&mut GameObject> {
        self.name_to_index.get(name).copied()
            .and_then(move |idx| self.objects.get_mut(idx))
    }
    
    pub fn is_key_held(&self, key: &Key) -> bool {
        self.held_keys.contains(key)
    }
    
    pub fn play_sound(&self, file_path: &str) {
        let path = file_path.to_string();
        std::thread::spawn(move || {
            if let Ok(file) = File::open(&path) {
                if let Ok(source) = Decoder::try_from(file) {
                    if let Ok(stream_handle) = OutputStreamBuilder::open_default_stream() {
                        let sink = Sink::connect_new(stream_handle.mixer());
                        sink.append(source);
                        sink.sleep_until_end();
                    }
                }
            }
        });
    }
}