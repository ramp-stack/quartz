use super::*;

// Add this enum definition at the top of your module or in a separate types file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Left,
    Right,
    Up,
    Down,
}

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
        }
    }
    
    pub fn on_tick<F>(&mut self, callback: F) 
    where
        F: FnMut(&mut Canvas) + 'static,
    {
        self.tick_callbacks.push(Box::new(callback));
    }
    
    pub fn on_custom<F>(&mut self, event_name: impl Into<String>, handler: F)
    where
        F: FnMut(&mut Canvas) + 'static,
    {
        self.custom_event_handlers.insert(event_name.into(), Box::new(handler));
    }
    
    pub fn trigger_custom_event(&mut self, event_name: &str) {
        if let Some(mut handler) = self.custom_event_handlers.remove(event_name) {
            handler(self);
            self.custom_event_handlers.insert(event_name.to_string(), handler);
        }
    }
    
    pub fn get_mode(&self) -> CanvasMode {
        self.layout.mode
    }
    
    pub fn get_virtual_size(&self) -> (f32, f32) {
        self.layout.canvas_size.get()
    }
    
    pub fn get_scale(&self) -> f32 {
        self.layout.scale.get()
    }
    
    pub fn get_safe_area_offset(&self) -> (f32, f32) {
        self.layout.safe_area_offset.get()
    }
    
    pub fn get_size(&self) -> (f32, f32) {
        self.layout.canvas_size.get()
    }
    
    pub fn is_key_held(&self, key: &Key) -> bool {
        self.held_keys.contains(key)
    }
    
    pub fn show(&mut self, name: &str) {
        if let Some(&idx) = self.name_to_index.get(name) {
            if let Some(obj) = self.objects.get_mut(idx) {
                obj.visible = true;
            }
        }
    }
    
    pub fn hide(&mut self, name: &str) {
        if let Some(&idx) = self.name_to_index.get(name) {
            if let Some(obj) = self.objects.get_mut(idx) {
                obj.visible = false;
            }
        }
    }
    
    pub fn toggle_visibility(&mut self, name: &str) {
        if let Some(&idx) = self.name_to_index.get(name) {
            if let Some(obj) = self.objects.get_mut(idx) {
                obj.visible = !obj.visible;
            }
        }
    }
    
    pub fn is_visible(&self, name: &str) -> bool {
        if let Some(&idx) = self.name_to_index.get(name) {
            if let Some(obj) = self.objects.get(idx) {
                return obj.visible;
            }
        }
        false
    }
    
    pub fn add_game_object(&mut self, name: String, game_obj: GameObject) {
        let position = game_obj.position;
        let id = game_obj.id.clone();
        let tags = game_obj.tags.clone();
        
        let idx = self.objects.len();
        
        self.layout.offsets.push(position);
        self.name_to_index.insert(name.clone(), idx);
        self.id_to_index.insert(id.clone(), idx);
        
        for tag in tags {
            self.tag_to_indices.entry(tag).or_insert_with(Vec::new).push(idx);
        }
        
        self.object_names.push(name);
        self.objects.push(game_obj);
        self.object_events.push(Vec::new());
    }
    
    pub fn remove_game_object(&mut self, name: &str) {
        if let Some(&idx) = self.name_to_index.get(name) {
            let removed_name = self.object_names.remove(idx);
            let removed_obj = self.objects.remove(idx);
            self.layout.offsets.remove(idx);
            self.object_events.remove(idx);
            
            self.name_to_index.remove(&removed_name);
            self.id_to_index.remove(&removed_obj.id);
            
            for tag in &removed_obj.tags {
                if let Some(indices) = self.tag_to_indices.get_mut(tag) {
                    indices.retain(|&i| i != idx);
                }
            }
            
            for index in self.name_to_index.values_mut() {
                if *index > idx {
                    *index -= 1;
                }
            }
            
            for index in self.id_to_index.values_mut() {
                if *index > idx {
                    *index -= 1;
                }
            }
            
            for indices in self.tag_to_indices.values_mut() {
                for index in indices.iter_mut() {
                    if *index > idx {
                        *index -= 1;
                    }
                }
            }
        }
    }
    
    pub fn get_game_object(&self, name: &str) -> Option<&GameObject> {
        self.name_to_index.get(name)
            .and_then(|&idx| self.objects.get(idx))
    }
    
    pub fn get_game_object_mut(&mut self, name: &str) -> Option<&mut GameObject> {
        self.name_to_index.get(name).copied()
            .and_then(move |idx| self.objects.get_mut(idx))
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
                self.apply_to_targets(&target, |obj| {
                    obj.momentum.0 = value.0;
                    obj.momentum.1 = value.1;
                });
            }
            Action::SetResistance { target, value } => {
                self.apply_to_targets(&target, |obj| {
                    obj.resistance = value;
                });
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
                let mut total_momentum = (0.0, 0.0);
                let mut count = 0;
                
                for &idx in &from_indices {
                    if let Some(obj) = self.objects.get(idx) {
                        total_momentum.0 += obj.momentum.0;
                        total_momentum.1 += obj.momentum.1;
                        count += 1;
                    }
                }
                
                if count > 0 {
                    total_momentum.0 /= count as f32;
                    total_momentum.1 /= count as f32;
                    
                    let scaled_momentum = (total_momentum.0 * scale, total_momentum.1 * scale);
                    self.apply_to_targets(&to, |obj| {
                        obj.momentum.0 = scaled_momentum.0;
                        obj.momentum.1 = scaled_momentum.1;
                    });
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
            Action::SetPosition { target, location } => {
                let position = location.resolve_position(self);
                self.apply_to_targets(&target, |obj| {
                    obj.position = position;
                });
                let indices = self.get_target_indices(&target);
                for idx in indices {
                    self.layout.offsets[idx] = position;
                }
            }
            Action::Teleport { target, location } => {
                let position = location.resolve_position(self);
                self.apply_to_targets(&target, |obj| {
                    obj.position = position;
                });
                let indices = self.get_target_indices(&target);
                for idx in indices {
                    self.layout.offsets[idx] = position;
                }
            }
            Action::Show { target } => {
                self.apply_to_targets(&target, |obj| {
                    obj.visible = true;
                });
            }
            Action::Hide { target } => {
                self.apply_to_targets(&target, |obj| {
                    obj.visible = false;
                });
            }
            Action::Toggle { target } => {
                self.apply_to_targets(&target, |obj| {
                    obj.visible = !obj.visible;
                });
            }
            Action::Conditional { condition, if_true, if_false } => {
                if self.evaluate_condition(&condition) {
                    self.run(*if_true);
                } else if let Some(false_action) = if_false {
                    self.run(*false_action);
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
        
        for &idx1 in &indices1 {
            for &idx2 in &indices2 {
                if idx1 != idx2 && self.check_collision(idx1, idx2) {
                    return true;
                }
            }
        }
        
        false
    }
    
    pub fn handle_infinite_scroll(&mut self, direction: ScrollDirection) {
        let bg_indices = self.get_target_indices(&Target::ByTag("scroll".to_string()));
        
        if bg_indices.len() < 2 {
            return; 
        }
        
        match direction {
            ScrollDirection::Left => {
                for &idx in &bg_indices {
                    if let Some(obj) = self.objects.get(idx) {
                        let right_edge = obj.position.0 + obj.size.0;
                        
                        if right_edge <= -10.0 {
                            let mut max_right_edge = f32::MIN;
                            for &other_idx in &bg_indices {
                                if other_idx != idx {
                                    if let Some(other_obj) = self.objects.get(other_idx) {
                                        let other_right = other_obj.position.0 + other_obj.size.0;
                                        if other_right > max_right_edge {
                                            max_right_edge = other_right;
                                        }
                                    }
                                }
                            }
                            
                            if let Some(obj) = self.objects.get_mut(idx) {
                                obj.position.0 = max_right_edge;
                                self.layout.offsets[idx] = obj.position;
                            }
                        }
                    }
                }
            }
            ScrollDirection::Right => {
                for &idx in &bg_indices {
                    if let Some(obj) = self.objects.get(idx) {
                        let left_edge = obj.position.0;
                        let canvas_width = self.layout.canvas_size.get().0;
                        
                        if left_edge >= canvas_width + 10.0 {
                            let mut min_left_edge = f32::MAX;
                            for &other_idx in &bg_indices {
                                if other_idx != idx {
                                    if let Some(other_obj) = self.objects.get(other_idx) {
                                        let other_left = other_obj.position.0;
                                        if other_left < min_left_edge {
                                            min_left_edge = other_left;
                                        }
                                    }
                                }
                            }
                            
                            if let Some(obj) = self.objects.get_mut(idx) {
                                obj.position.0 = min_left_edge - obj.size.0;
                                self.layout.offsets[idx] = obj.position;
                            }
                        }
                    }
                }
            }
            ScrollDirection::Up => {
                for &idx in &bg_indices {
                    if let Some(obj) = self.objects.get(idx) {
                        let bottom_edge = obj.position.1 + obj.size.1;
                        
                        if bottom_edge <= -10.0 {
                            let mut max_bottom_edge = f32::MIN;
                            for &other_idx in &bg_indices {
                                if other_idx != idx {
                                    if let Some(other_obj) = self.objects.get(other_idx) {
                                        let other_bottom = other_obj.position.1 + other_obj.size.1;
                                        if other_bottom > max_bottom_edge {
                                            max_bottom_edge = other_bottom;
                                        }
                                    }
                                }
                            }
                            
                            if let Some(obj) = self.objects.get_mut(idx) {
                                obj.position.1 = max_bottom_edge;
                                self.layout.offsets[idx] = obj.position;
                            }
                        }
                    }
                }
            }
            ScrollDirection::Down => {
                for &idx in &bg_indices {
                    if let Some(obj) = self.objects.get(idx) {
                        let top_edge = obj.position.1;
                        let canvas_height = self.layout.canvas_size.get().1;
                        
                        if top_edge >= canvas_height + 10.0 {
                            let mut min_top_edge = f32::MAX;
                            for &other_idx in &bg_indices {
                                if other_idx != idx {
                                    if let Some(other_obj) = self.objects.get(other_idx) {
                                        let other_top = other_obj.position.1;
                                        if other_top < min_top_edge {
                                            min_top_edge = other_top;
                                        }
                                    }
                                }
                            }
                            
                            if let Some(obj) = self.objects.get_mut(idx) {
                                obj.position.1 = min_top_edge - obj.size.1;
                                self.layout.offsets[idx] = obj.position;
                            }
                        }
                    }
                }
            }
        }
    }
}