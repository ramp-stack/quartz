use super::*;
use crate::sound::spawn_sound;
use std::cell::Cell;

impl Canvas {
    pub fn new(_ctx: &mut Context, mode: CanvasMode) -> Self {
        let virtual_res = mode.virtual_resolution().unwrap_or((0.0, 0.0));
        Self {
            layout: CanvasLayout {
                offsets:          Vec::new(),
                canvas_size:      Cell::new(virtual_res),
                mode,
                scale:            Cell::new(1.0),
                safe_area_offset: Cell::new((0.0, 0.0)),
            },
            store:         ObjectStore::new(),
            input:         InputState::new(),
            mouse:         MouseState::new(),
            callbacks:     CallbackStore::new(),
            scene_manager: SceneManager::new(),
            active_camera: None,
            entropy:       Entropy::new(),
        }
    }

    pub fn key(&self, name: &str) -> bool {
        let k = match name {
            "left"  => Key::Named(NamedKey::ArrowLeft),
            "right" => Key::Named(NamedKey::ArrowRight),
            "up"    => Key::Named(NamedKey::ArrowUp),
            "down"  => Key::Named(NamedKey::ArrowDown),
            "space" => Key::Named(NamedKey::Space),
            "enter" => Key::Named(NamedKey::Enter),
            "tab"   => Key::Named(NamedKey::Tab),
            "del"   => Key::Named(NamedKey::Delete),
            other   => Key::Character(other.into()),
        };
        self.input.held_keys.contains(&k)
    }

    pub fn add_game_object(&mut self, name: String, obj: GameObject) {
        let position = obj.position;
        self.layout.offsets.push(position);
        self.store.add(name, obj);
    }

    pub fn remove_game_object(&mut self, name: &str) {
        if let Some(&idx) = self.store.name_to_index.get(name) {
            self.mouse.hovered_indices.remove(&idx);
            let updated: std::collections::HashSet<usize> = self.mouse.hovered_indices
                .drain()
                .map(|i| if i > idx { i - 1 } else { i })
                .collect();
            self.mouse.hovered_indices = updated;

            self.layout.offsets.remove(idx);
            self.store.remove(name);
        }
    }

    pub fn get_game_object(&self, name: &str) -> Option<&GameObject> {
        self.store.name_to_index.get(name).and_then(|&i| self.store.objects.get(i))
    }

    pub fn get_game_object_mut(&mut self, name: &str) -> Option<&mut GameObject> {
        self.store.name_to_index.get(name).copied()
            .and_then(move |i| self.store.objects.get_mut(i))
    }

    pub fn run(&mut self, action: Action) {
        match action {
            Action::ApplyMomentum { target, value } => {
                self.store.apply_to_targets(&target, |obj| {
                    obj.momentum.0 += value.0;
                    obj.momentum.1 += value.1;
                });
            }
            Action::SetMomentum { target, value } => {
                self.store.apply_to_targets(&target, |obj| obj.momentum = value);
            }
            Action::SetResistance { target, value } => {
                self.store.apply_to_targets(&target, |obj| obj.resistance = value);
            }
            Action::Remove { target } => {
                let names = self.store.get_names(&target);
                for name in names { self.remove_game_object(&name); }
            }
            Action::Spawn { object, location } => {
                let position = location.resolve_position(&self.store);
                let mut new_obj = *object;
                new_obj.position = position;
                let name = format!("spawned_{}", new_obj.id);
                self.add_game_object(name, new_obj);
            }
            Action::TransferMomentum { from, to, scale } => {
                let from_indices = self.store.get_indices(&from);
                let (total, count) = from_indices.iter()
                    .filter_map(|&i| self.store.objects.get(i))
                    .fold(((0.0_f32, 0.0_f32), 0usize), |(acc, cnt), obj| {
                        ((acc.0 + obj.momentum.0, acc.1 + obj.momentum.1), cnt + 1)
                    });

                if count > 0 {
                    let scaled = (total.0 / count as f32 * scale, total.1 / count as f32 * scale);
                    self.store.apply_to_targets(&to, |obj| obj.momentum = scaled);
                }
            }
            Action::SetAnimation { target, animation_bytes, fps } => {
                let indices = self.store.get_indices(&target);
                for idx in indices {
                    if let Some(obj) = self.store.objects.get_mut(idx) {
                        if let Ok(sprite) = AnimatedSprite::new(animation_bytes, obj.size, fps) {
                            obj.set_animation(sprite);
                        }
                    }
                }
            }
            Action::Teleport { target, location } => {
                let position = location.resolve_position(&self.store);
                let indices = self.store.get_indices(&target);
                for idx in indices {
                    if let Some(obj) = self.store.objects.get_mut(idx) {
                        obj.position = position;
                        self.layout.offsets[idx] = position;
                    }
                }
            }
            Action::Show   { target } => self.store.apply_to_targets(&target, |obj| obj.visible = true),
            Action::Hide   { target } => self.store.apply_to_targets(&target, |obj| obj.visible = false),
            Action::Toggle { target } => self.store.apply_to_targets(&target, |obj| obj.visible = !obj.visible),
            Action::Conditional { condition, if_true, if_false } => {
                if self.evaluate_condition(&condition) {
                    self.run(*if_true);
                } else if let Some(fa) = if_false {
                    self.run(*fa);
                }
            }
            Action::Custom { name } => {
                if let Some(mut handler) = self.callbacks.custom.remove(&name) {
                    handler(self);
                    self.callbacks.custom.insert(name, handler);
                }
            }
        }
    }

    pub fn add_event(&mut self, event: GameEvent, target: Target) {
        let indices = self.store.get_indices(&target);
        for idx in indices {
            if let Some(events) = self.store.events.get_mut(idx) {
                events.push(event.clone());
            }
        }
    }

    pub fn on_update<F>(&mut self, callback: F)
    where
        F: FnMut(&mut Canvas) + Clone + 'static,
    {
        self.callbacks.tick.push(Box::new(callback));
    }

    pub fn register_custom_event<F>(&mut self, name: String, handler: F)
    where
        F: FnMut(&mut Canvas) + Clone + 'static,
    {
        self.callbacks.custom.insert(name, Box::new(handler));
    }

    pub fn set_camera(&mut self, camera: Camera) { self.active_camera = Some(camera); }
    pub fn clear_camera(&mut self)               { self.active_camera = None; }
    pub fn camera(&self)     -> Option<&Camera>      { self.active_camera.as_ref() }
    pub fn camera_mut(&mut self) -> Option<&mut Camera> { self.active_camera.as_mut() }

    pub fn collision_between(&self, t1: &Target, t2: &Target) -> bool {
        let i1 = self.store.get_indices(t1);
        let i2 = self.store.get_indices(t2);
        i1.iter().any(|&a| {
            i2.iter().any(|&b| {
                if a == b { return false; }
                match (self.store.objects.get(a), self.store.objects.get(b)) {
                    (Some(o1), Some(o2)) => Self::check_collision(o1, o2),
                    _ => false,
                }
            })
        })
    }

    pub fn objects_in_radius(&self, game_object: &GameObject, radius_px: f32) -> Vec<&GameObject> {
        let cx = game_object.position.0 + game_object.size.0 / 2.0;
        let cy = game_object.position.1 + game_object.size.1 / 2.0;
        let r2 = radius_px * radius_px;

        self.store.objects.iter().filter(|obj| {
            if obj.id == game_object.id || !obj.visible { return false; }
            let dx = obj.position.0 + obj.size.0 / 2.0 - cx;
            let dy = obj.position.1 + obj.size.1 / 2.0 - cy;
            dx * dx + dy * dy <= r2
        }).collect()
    }

    pub fn get_virtual_size(&self) -> (f32, f32) {
        self.layout.canvas_size.get()
    }

    pub fn play_sound(&self, file_path: &str) -> SoundHandle {
        spawn_sound(file_path, SoundOptions::default())
    }

    pub fn play_sound_with(&self, file_path: &str, options: SoundOptions) -> SoundHandle {
        spawn_sound(file_path, options)
    }

    pub(crate) fn check_collision(o1: &GameObject, o2: &GameObject) -> bool {
        if !o1.visible || !o2.visible { return false; }
        o1.position.0 < o2.position.0 + o2.size.0
            && o1.position.0 + o1.size.0 > o2.position.0
            && o1.position.1 < o2.position.1 + o2.size.1
            && o1.position.1 + o1.size.1 > o2.position.1
    }

    pub(crate) fn evaluate_condition(&self, condition: &Condition) -> bool {
        match condition {
            Condition::Always => true,
            Condition::KeyHeld(k)    =>  self.input.held_keys.contains(k),
            Condition::KeyNotHeld(k) => !self.input.held_keys.contains(k),
            Condition::Collision(t) => {
                self.store.get_indices(t).iter().any(|&i| {
                    (0..self.store.objects.len()).any(|j| {
                        if i == j { return false; }
                        match (self.store.objects.get(i), self.store.objects.get(j)) {
                            (Some(a), Some(b)) => Self::check_collision(a, b),
                            _ => false,
                        }
                    })
                })
            }
            Condition::NoCollision(t) => !self.evaluate_condition(&Condition::Collision(t.clone())),
            Condition::And(c1, c2) => self.evaluate_condition(c1) && self.evaluate_condition(c2),
            Condition::Or(c1, c2)  => self.evaluate_condition(c1) || self.evaluate_condition(c2),
            Condition::Not(c)      => !self.evaluate_condition(c),
            Condition::IsVisible(t) => self.store.get_indices(t).iter()
                .any(|&i| self.store.objects.get(i).map_or(false, |o| o.visible)),
            Condition::IsHidden(t)  => self.store.get_indices(t).iter()
                .any(|&i| self.store.objects.get(i).map_or(true,  |o| !o.visible)),
        }
    }

    pub(crate) fn trigger_collision_events(&mut self, idx: usize) {
        let actions: Vec<_> = self.store.events.get(idx).into_iter().flatten()
            .filter_map(|e| if let GameEvent::Collision { action, .. } = e { Some(action.clone()) } else { None })
            .collect();
        actions.into_iter().for_each(|a| self.run(a));
    }

    pub(crate) fn trigger_boundary_collision_events(&mut self, idx: usize) {
        let actions: Vec<_> = self.store.events.get(idx).into_iter().flatten()
            .filter_map(|e| if let GameEvent::BoundaryCollision { action, .. } = e { Some(action.clone()) } else { None })
            .collect();
        actions.into_iter().for_each(|a| self.run(a));
    }

    pub(crate) fn update_objects(&mut self, delta_time: f32) {
        let scale = self.layout.scale.get();

        for (idx, obj) in self.store.objects.iter_mut().enumerate() {
            obj.scaled_size.set((obj.size.0 * scale, obj.size.1 * scale));
            obj.update_animation(delta_time);

            if obj.animated_sprite.is_none() {
                obj.update_image_shape();
            }

            obj.update_text_scale(scale);

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

    pub(crate) fn apply_camera_transform(&mut self) {
        let mut cam = match self.active_camera.take() {
            Some(c) => c,
            None => return,
        };

        if let Some(target) = cam.follow_target.clone() {
            if let Some(&idx) = self.store.get_indices(&target).first() {
                if let Some(obj) = self.store.objects.get(idx) {
                    let cx = obj.position.0 + obj.size.0 * 0.5;
                    let cy = obj.position.1 + obj.size.1 * 0.5;
                    cam.lerp_toward(cx, cy);
                }
            }
        }

        let (cam_x, cam_y) = cam.position;
        for (idx, obj) in self.store.objects.iter().enumerate() {
            self.layout.offsets[idx] = (obj.position.0 - cam_x, obj.position.1 - cam_y);
        }

        self.active_camera = Some(cam);
    }

    pub(crate) fn handle_collisions(&mut self) {
        let mut adjustments: Vec<(usize, f32)> = Vec::new();
        let mut collision_pairs: Vec<(usize, usize)> = Vec::new();

        let n = self.store.objects.len();
        for i in 0..n {
            if !self.store.objects[i].visible { continue; }
            for j in (i + 1)..n {
                if !self.store.objects[j].visible { continue; }

                let o1 = &self.store.objects[i];
                let o2 = &self.store.objects[j];

                if Self::check_collision(o1, o2) {
                    if o2.is_platform && o1.momentum.1 > 0.0 {
                        if o1.position.1 + o1.size.1 > o2.position.1 {
                            adjustments.push((i, o2.position.1 - o1.size.1));
                        }
                    } else if o1.is_platform && o2.momentum.1 > 0.0 {
                        if o2.position.1 + o2.size.1 > o1.position.1 {
                            adjustments.push((j, o1.position.1 - o2.size.1));
                        }
                    }
                    if !o1.is_platform && !o2.is_platform {
                        collision_pairs.push((i, j));
                    }
                }
            }
        }

        let cam_off = self.active_camera.as_ref().map(|c| c.position).unwrap_or((0.0, 0.0));
        for (idx, new_y) in adjustments {
            self.store.objects[idx].position.1 = new_y;
            self.store.objects[idx].momentum.1 = 0.0;
            self.layout.offsets[idx] = (
                self.store.objects[idx].position.0 - cam_off.0,
                new_y - cam_off.1,
            );
        }

        for (i, j) in collision_pairs {
            self.trigger_collision_events(i);
            self.trigger_collision_events(j);
        }
    }

    pub(crate) fn handle_infinite_scroll(&mut self) {
        let bg_indices = self.store.get_indices(&Target::ByTag("scroll".to_string()));
        if bg_indices.len() < 2 { return; }

        for &idx in &bg_indices {
            if let Some(obj) = self.store.objects.get(idx) {
                if obj.position.0 + obj.size.0 <= -10.0 {
                    let max_right = bg_indices.iter()
                        .filter(|&&other| other != idx)
                        .filter_map(|&other| self.store.objects.get(other))
                        .map(|o| o.position.0 + o.size.0)
                        .fold(f32::MIN, f32::max);

                    if let Some(obj) = self.store.objects.get_mut(idx) {
                        obj.position.0 = max_right;
                        self.layout.offsets[idx] = obj.position;
                    }
                }
            }
        }
    }
}

impl Location {
    pub(crate) fn resolve_position(&self, store: &ObjectStore) -> (f32, f32) {
        match self {
            Location::Position(pos) => *pos,
            Location::AtTarget(t) => {
                store.get_indices(t).first()
                    .and_then(|&i| store.objects.get(i))
                    .map(|o| o.position)
                    .unwrap_or((0.0, 0.0))
            }
            Location::Between(t1, t2) => {
                let p1 = store.get_indices(t1).first()
                    .and_then(|&i| store.objects.get(i)).map(|o| o.position).unwrap_or((0.0, 0.0));
                let p2 = store.get_indices(t2).first()
                    .and_then(|&i| store.objects.get(i)).map(|o| o.position).unwrap_or((0.0, 0.0));
                ((p1.0 + p2.0) / 2.0, (p1.1 + p2.1) / 2.0)
            }
            Location::Relative { target, offset } => {
                store.get_indices(target).first()
                    .and_then(|&i| store.objects.get(i))
                    .map(|o| (o.position.0 + offset.0, o.position.1 + offset.1))
                    .unwrap_or(*offset)
            }
            Location::OnTarget { target, anchor, offset } => {
                store.get_indices(target).first()
                    .and_then(|&i| store.objects.get(i))
                    .map(|o| {
                        let ap = o.get_anchor_position(*anchor);
                        (ap.0 + offset.0, ap.1 + offset.1)
                    })
                    .unwrap_or(*offset)
            }
        }
    }
}