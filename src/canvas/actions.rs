use super::core::Canvas;
use prism::event::Key;
use prism::event::NamedKey;
use crate::store::ObjectStore;
use crate::input::{InputState, MouseState, CallbackStore};
use crate::scene::SceneManager;
use crate::entropy::Entropy;
use crate::object::GameObject;
use crate::sprite::AnimatedSprite;
use crate::sound::{SoundOptions, SoundHandle, spawn_sound};
use crate::camera::Camera;
use crate::file_watcher;
use crate::value::{Value, Expr, resolve_expr, apply_op};
use crate::expr::{parse_action, parse_condition};
use crate::types::{
    Action, Condition,
    Target, Location,
    CollisionMode, CollisionShape,
    GlowConfig,
};
use super::core::CanvasLayout;
use super::core::CanvasMode;
use std::cell::Cell;
use std::collections::HashMap;

impl Canvas {
    pub fn new(_ctx: &mut prism::Context, mode: CanvasMode) -> Self {
        let virtual_res = mode.virtual_resolution().unwrap_or((0.0, 0.0));
        Self {
            layout: CanvasLayout {
                offsets:          Vec::new(),
                canvas_size:      Cell::new(virtual_res),
                mode,
                scale:            Cell::new(1.0),
                safe_area_offset: Cell::new((0.0, 0.0)),
            },
            store:            ObjectStore::new(),
            input:            InputState::new(),
            mouse:            MouseState::new(),
            callbacks:        CallbackStore::new(),
            scene_manager:    SceneManager::new(),
            active_camera:    None,
            entropy:          Entropy::new(),
            hot_reload_timer: 0.0,
            file_watchers:    Vec::new(),
            game_vars:        HashMap::new(),
            paused:           false,
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
            Action::SetVar { name, value } => {
                if let Some(resolved) = resolve_expr(&value, &self.game_vars) {
                    self.game_vars.insert(name, resolved);
                }
            }
            Action::ModVar { name, op, operand } => {
                if let Some(current) = self.game_vars.get(&name).cloned() {
                    if let Some(resolved) = resolve_expr(&operand, &self.game_vars) {
                        if let Some(new_val) = apply_op(&current, &resolved, &op) {
                            self.game_vars.insert(name, new_val);
                        }
                    }
                }
            }
            Action::Multi(actions) => {
                for action in actions {
                    self.run(action);
                }
            }
            Action::PlaySound { path, options } => {
                self.play_sound_with(&path, options);
            }
            Action::SetGravity { target, value } => {
                self.store.apply_to_targets(&target, |obj| obj.gravity = value);
            }
            Action::SetSize { target, value } => {
                let scale = self.layout.scale.get();
                let indices = self.store.get_indices(&target);
                for idx in indices {
                    if let Some(obj) = self.store.objects.get_mut(idx) {
                        obj.size = value;
                        obj.scaled_size.set((value.0 * scale, value.1 * scale));
                        obj.update_image_shape();
                    }
                }
            }
            Action::AddTag { target, tag } => {
                let indices = self.store.get_indices(&target);
                for idx in indices {
                    if let Some(obj) = self.store.objects.get_mut(idx) {
                        if !obj.tags.contains(&tag) {
                            obj.tags.push(tag.clone());
                            self.store.tag_to_indices.entry(tag.clone()).or_default().push(idx);
                        }
                    }
                }
            }
            Action::RemoveTag { target, tag } => {
                let indices = self.store.get_indices(&target);
                for idx in indices {
                    if let Some(obj) = self.store.objects.get_mut(idx) {
                        obj.tags.retain(|t| t != &tag);
                    }
                    if let Some(v) = self.store.tag_to_indices.get_mut(&tag) {
                        v.retain(|&i| i != idx);
                    }
                }
            }
            Action::SetText { target, text } => {
                let indices = self.store.get_indices(&target);
                for idx in indices {
                    if let Some(obj) = self.store.objects.get_mut(idx) {
                        obj.set_drawable(Box::new(text.clone()));
                    }
                }
            }
            Action::Expr(src) => {
                match parse_action(&src) {
                    Ok(actions) => {
                        for action in actions {
                            self.run(action);
                        }
                    }
                    Err(e) => {
                        debug_assert!(false,
                            "[Action::Expr] parse error in \"{src}\": {e}\n\
                            Use Action::expr() to catch this at setup time.");
                    }
                }
            }
            Action::SetRotation { target, value } => {
                self.store.apply_to_targets(&target, |obj| obj.rotation = value);
            }
            Action::SetSlope { target, left_offset, right_offset, auto_rotate } => {
                let indices = self.store.get_indices(&target);
                for idx in indices {
                    if let Some(obj) = self.store.objects.get_mut(idx) {
                        obj.slope = Some((left_offset, right_offset));
                        if auto_rotate {
                            obj.rotation = obj.rotation_from_slope();
                        }
                    }
                }
            }
            Action::AddRotation { target, value } => {
                self.store.apply_to_targets(&target, |obj| {
                    obj.rotation += value;
                });
            }
            Action::ApplyRotation { target, value } => {
                self.store.apply_to_targets(&target, |obj| {
                    obj.rotation_momentum += value;
                });
            }
            Action::SetSurfaceNormal { target, nx, ny } => {
                let len = (nx * nx + ny * ny).sqrt().max(0.001);
                let (nx, ny) = (nx / len, ny / len);
                self.store.apply_to_targets(&target, |obj| obj.surface_normal = (nx, ny));
            }
            Action::SetCollisionMode { target, mode } => {
                let indices = self.store.get_indices(&target);
                for idx in indices {
                    if let Some(obj) = self.store.objects.get_mut(idx) {
                        obj.collision_mode = mode.clone();
                        match mode {
                            CollisionMode::NonPlatform => { obj.is_platform = false; }
                            CollisionMode::Surface | CollisionMode::Solid(_) => {
                                obj.is_platform = true;
                            }
                        }
                    }
                }
            }
            Action::SetGlow { target, color, width } => {
                let indices = self.store.get_indices(&target);
                for idx in indices {
                    if let Some(obj) = self.store.objects.get_mut(idx) {
                        obj.set_glow(GlowConfig { color, width });
                    }
                }
            }
            Action::ClearGlow { target } => {
                let indices = self.store.get_indices(&target);
                for idx in indices {
                    if let Some(obj) = self.store.objects.get_mut(idx) {
                        obj.clear_glow();
                    }
                }
            }
            Action::SetTint { target, color } => {
                let indices = self.store.get_indices(&target);
                for idx in indices {
                    if let Some(obj) = self.store.objects.get_mut(idx) {
                        obj.set_tint(color);
                    }
                }
            }
            Action::ClearTint { target } => {
                let indices = self.store.get_indices(&target);
                for idx in indices {
                    if let Some(obj) = self.store.objects.get_mut(idx) {
                        obj.clear_tint();
                    }
                }
            }
        }
    }


    pub fn add_event(&mut self, event: crate::types::GameEvent, target: Target) {
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


    pub fn set_camera(&mut self, camera: Camera)         { self.active_camera = Some(camera); }
    pub fn clear_camera(&mut self)                       { self.active_camera = None; }
    pub fn camera(&self)     -> Option<&Camera>          { self.active_camera.as_ref() }
    pub fn camera_mut(&mut self) -> Option<&mut Camera>  { self.active_camera.as_mut() }


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


    pub(crate) fn evaluate_condition(&self, condition: &Condition) -> bool {
        use crate::value::compare_operands;
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
            Condition::Compare(left, op, right) => {
                match (
                    resolve_expr(left, &self.game_vars),
                    resolve_expr(right, &self.game_vars),
                ) {
                    (Some(l), Some(r)) => compare_operands(&l, &r, op).unwrap_or(false),
                    _ => false,
                }
            }
            Condition::VarExists(name) => self.game_vars.contains_key(name.as_str()),
            Condition::Grounded(target) => {
                self.store.get_indices(target).iter().any(|&idx| {
                    self.store.objects.get(idx).map_or(false, |obj| obj.grounded)
                })
            }
            Condition::Expr(src) => {
                match parse_condition(src) {
                    Ok(condition) => self.evaluate_condition(&condition),
                    Err(e) => {
                        debug_assert!(false,
                            "[Condition::Expr] parse error in \"{src}\": {e}\n\
                            Use Condition::expr() to catch this at setup time.");
                        false
                    }
                }
            }
            Condition::HasTag(target, tag) => {
                self.store.get_indices(target).iter().any(|&idx| {
                    self.store.objects.get(idx).map_or(false, |obj| obj.tags.contains(tag))
                })
            }
        }
    }


    pub fn set_var(&mut self, name: impl Into<String>, value: impl Into<Value>) {
        self.game_vars.insert(name.into(), value.into());
    }

    pub fn get_var(&self, name: &str) -> Option<Value> {
        self.game_vars.get(name).cloned()
    }

    pub fn has_var(&self, name: &str) -> bool {
        self.game_vars.contains_key(name)
    }

    pub fn remove_var(&mut self, name: &str) {
        self.game_vars.remove(name);
    }

    pub fn resolve(&self, expr: &Expr) -> Option<Value> {
        resolve_expr(expr, &self.game_vars)
    }

    pub fn modify_var(&mut self, name: &str, f: impl FnOnce(Value) -> Value) {
        if let Some(val) = self.game_vars.remove(name) {
            self.game_vars.insert(name.to_string(), f(val));
        }
    }

    pub fn get_u8(&self, name: &str) -> u8 {
        match self.game_vars.get(name) {
            Some(Value::U8(v)) => *v,
            Some(other) => panic!("game_var '{name}' expected U8 but found {}", value_type_name(other)),
            None => panic!("game_var '{name}' expected U8 but key was missing"),
        }
    }

    pub fn get_u32(&self, name: &str) -> u32 {
        match self.game_vars.get(name) {
            Some(Value::U32(v)) => *v,
            Some(other) => panic!("game_var '{name}' expected U32 but found {}", value_type_name(other)),
            None => panic!("game_var '{name}' expected U32 but key was missing"),
        }
    }

    pub fn get_i32(&self, name: &str) -> i32 {
        match self.game_vars.get(name) {
            Some(Value::I32(v)) => *v,
            Some(other) => panic!("game_var '{name}' expected I32 but found {}", value_type_name(other)),
            None => panic!("game_var '{name}' expected I32 but key was missing"),
        }
    }

    pub fn get_f32(&self, name: &str) -> f32 {
        match self.game_vars.get(name) {
            Some(Value::F32(v)) => *v,
            Some(other) => panic!("game_var '{name}' expected F32 but found {}", value_type_name(other)),
            None => panic!("game_var '{name}' expected F32 but key was missing"),
        }
    }

    pub fn get_bool(&self, name: &str) -> bool {
        match self.game_vars.get(name) {
            Some(Value::Bool(v)) => *v,
            Some(other) => panic!("game_var '{name}' expected Bool but found {}", value_type_name(other)),
            None => panic!("game_var '{name}' expected Bool but key was missing"),
        }
    }

    pub fn get_usize(&self, name: &str) -> usize {
        match self.game_vars.get(name) {
            Some(Value::Usize(v)) => *v,
            Some(other) => panic!("game_var '{name}' expected Usize but found {}", value_type_name(other)),
            None => panic!("game_var '{name}' expected Usize but key was missing"),
        }
    }

    pub fn get_str(&self, name: &str) -> &str {
        match self.game_vars.get(name) {
            Some(Value::Str(v)) => v.as_str(),
            Some(other) => panic!("game_var '{name}' expected Str but found {}", value_type_name(other)),
            None => panic!("game_var '{name}' expected Str but key was missing"),
        }
    }

    pub fn modify_u8(&mut self, name: &str, f: impl FnOnce(u8) -> u8) {
        match self.game_vars.get(name).cloned() {
            Some(Value::U8(n)) => { self.game_vars.insert(name.to_string(), Value::U8(f(n))); }
            Some(other) => panic!("game_var '{name}' expected U8 for modify but found {}", value_type_name(&other)),
            None => panic!("game_var '{name}' expected U8 for modify but key was missing"),
        }
    }

    pub fn modify_u32(&mut self, name: &str, f: impl FnOnce(u32) -> u32) {
        match self.game_vars.get(name).cloned() {
            Some(Value::U32(n)) => { self.game_vars.insert(name.to_string(), Value::U32(f(n))); }
            Some(other) => panic!("game_var '{name}' expected U32 for modify but found {}", value_type_name(&other)),
            None => panic!("game_var '{name}' expected U32 for modify but key was missing"),
        }
    }

    pub fn modify_i32(&mut self, name: &str, f: impl FnOnce(i32) -> i32) {
        match self.game_vars.get(name).cloned() {
            Some(Value::I32(n)) => { self.game_vars.insert(name.to_string(), Value::I32(f(n))); }
            Some(other) => panic!("game_var '{name}' expected I32 for modify but found {}", value_type_name(&other)),
            None => panic!("game_var '{name}' expected I32 for modify but key was missing"),
        }
    }

    pub fn modify_f32(&mut self, name: &str, f: impl FnOnce(f32) -> f32) {
        match self.game_vars.get(name).cloned() {
            Some(Value::F32(n)) => { self.game_vars.insert(name.to_string(), Value::F32(f(n))); }
            Some(other) => panic!("game_var '{name}' expected F32 for modify but found {}", value_type_name(&other)),
            None => panic!("game_var '{name}' expected F32 for modify but key was missing"),
        }
    }

    pub fn modify_bool(&mut self, name: &str, f: impl FnOnce(bool) -> bool) {
        match self.game_vars.get(name).cloned() {
            Some(Value::Bool(n)) => { self.game_vars.insert(name.to_string(), Value::Bool(f(n))); }
            Some(other) => panic!("game_var '{name}' expected Bool for modify but found {}", value_type_name(&other)),
            None => panic!("game_var '{name}' expected Bool for modify but key was missing"),
        }
    }

    pub fn modify_usize(&mut self, name: &str, f: impl FnOnce(usize) -> usize) {
        match self.game_vars.get(name).cloned() {
            Some(Value::Usize(n)) => { self.game_vars.insert(name.to_string(), Value::Usize(f(n))); }
            Some(other) => panic!("game_var '{name}' expected Usize for modify but found {}", value_type_name(&other)),
            None => panic!("game_var '{name}' expected Usize for modify but key was missing"),
        }
    }

    pub fn modify_str(&mut self, name: &str, f: impl FnOnce(String) -> String) {
        match self.game_vars.get(name).cloned() {
            Some(Value::Str(s)) => { self.game_vars.insert(name.to_string(), Value::Str(f(s))); }
            Some(other) => panic!("game_var '{name}' expected Str for modify but found {}", value_type_name(&other)),
            None => panic!("game_var '{name}' expected Str for modify but key was missing"),
        }
    }


    pub fn pause(&mut self)       { self.paused = true; }
    pub fn resume(&mut self)      { self.paused = false; }
    pub fn is_paused(&self) -> bool { self.paused }
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::I8(_)    => "I8",
        Value::U8(_)    => "U8",
        Value::I16(_)   => "I16",
        Value::U16(_)   => "U16",
        Value::I32(_)   => "I32",
        Value::U32(_)   => "U32",
        Value::I64(_)   => "I64",
        Value::U64(_)   => "U64",
        Value::F32(_)   => "F32",
        Value::F64(_)   => "F64",
        Value::Usize(_) => "Usize",
        Value::Bool(_)  => "Bool",
        Value::Str(_)   => "Str",
    }
}