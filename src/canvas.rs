use super::*;
use crate::sound::spawn_sound;
use crate::value::{Value, Expr, resolve_expr, apply_op, compare_operands, MathOp};
use crate::expr::{parse_action, parse_condition};
use std::cell::Cell;
use std::collections::HashMap;

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
            game_vars:     HashMap::new(),
            paused:        false,
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

        let (ax, ay, aw, ah) = if o1.is_platform && o1.slope.is_some() {
            o1.slope_aabb()
        } else if o1.is_platform && o1.rotation != 0.0 {
            rotated_aabb(o1)
        } else {
            (o1.position.0, o1.position.1, o1.size.0, o1.size.1)
        };

        let (bx, by, bw, bh) = if o2.is_platform && o2.slope.is_some() {
            o2.slope_aabb()
        } else if o2.is_platform && o2.rotation != 0.0 {
            rotated_aabb(o2)
        } else {
            (o2.position.0, o2.position.1, o2.size.0, o2.size.1)
        };

        ax < bx + bw
            && ax + aw > bx
            && ay < by + bh
            && ay + ah > by
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
                    if let Some(obj) = self.store.objects.get(idx) {
                        let obj_bottom = obj.position.1 + obj.size.1;
                        let obj_center_x = obj.position.0 + obj.size.0 * 0.5;
                        self.store.objects.iter().any(|other| {
                            if !other.is_platform { return false; }
                            let (_, eff_ny) = other.surface_normal_at(obj_center_x);
                            if eff_ny >= -0.3 { return false; }
                            if obj.position.0 + obj.size.0 <= other.position.0 { return false; }
                            if obj.position.0 >= other.position.0 + other.size.0 { return false; }
                            if obj.momentum.1 < 0.0 { return false; }
                            let surface_y = other.slope_surface_y(obj_center_x);
                            (obj_bottom - surface_y).abs() < 2.0
                        })
                    } else {
                        false
                    }
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
                obj.apply_rotation_momentum();
                self.layout.offsets[idx] = rotation_adjusted_offset(
                    obj.position, obj.size, obj.rotation, obj.slope.is_some(),
                );
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
            let adj = rotation_adjusted_offset(
                obj.position, obj.size, obj.rotation, obj.slope.is_some(),
            );
            self.layout.offsets[idx] = (adj.0 - cam_x, adj.1 - cam_y);
        }

        self.active_camera = Some(cam);
    }

    pub(crate) fn handle_collisions(&mut self) {
        let mut adjustments: Vec<(usize, f32, f32, usize)> = Vec::new();
        let mut collision_pairs: Vec<(usize, usize)>       = Vec::new();

        let n = self.store.objects.len();
        for i in 0..n {
            if !self.store.objects[i].visible { continue; }
            for j in (i + 1)..n {
                if !self.store.objects[j].visible { continue; }

                let o1 = &self.store.objects[i];
                let o2 = &self.store.objects[j];

                if !Self::check_collision(o1, o2) { continue; }

                if !o1.is_platform && !o2.is_platform {
                    collision_pairs.push((i, j));
                    continue;
                }

                let (obj_idx, plat_idx) = if o2.is_platform && !o1.is_platform {
                    (i, j)
                } else if o1.is_platform && !o2.is_platform {
                    (j, i)
                } else {
                    continue;
                };

                let obj  = &self.store.objects[obj_idx];
                let plat = &self.store.objects[plat_idx];

                let obj_center_x = obj.position.0 + obj.size.0 * 0.5;

                let (mut nx, mut ny) = plat.surface_normal_at(obj_center_x);

                // For rotating platforms (non-slope), always treat the upper
                // side as the collideable surface. If the tracked normal
                // points downward (ny > 0), the platform has rotated past
                // 90° — flip the normal so it points upward again.
                if plat.rotation != 0.0 && plat.slope.is_none() && ny > 0.0 {
                    nx = -nx;
                    ny = -ny;
                }

                let approach_speed = obj.momentum.0 * (-nx) + obj.momentum.1 * (-ny);
                if approach_speed <= 0.0 { continue; }

                if plat.one_way {
                    if plat.slope.is_some() {
                        let prev_bottom = (obj.position.1 + obj.size.1) - obj.momentum.1;
                        let prev_cx = obj_center_x - obj.momentum.0;
                        let surface_at_prev = plat.slope_surface_y(prev_cx);
                        if prev_bottom > surface_at_prev + 2.0 {
                            continue;
                        }
                    } else {
                        let obj_cx = obj.position.0 + obj.size.0 * 0.5;
                        let obj_cy = obj.position.1 + obj.size.1 * 0.5;
                        let plat_cx = plat.position.0 + plat.size.0 * 0.5;
                        let plat_cy = plat.position.1 + plat.size.1 * 0.5;
                        let prev_rel_x = (obj_cx - obj.momentum.0) - plat_cx;
                        let prev_rel_y = (obj_cy - obj.momentum.1) - plat_cy;
                        let was_outside = prev_rel_x * nx + prev_rel_y * ny > 0.0;
                        if !was_outside { continue; }
                    }
                }

                let (dx, dy) = if plat.slope.is_some() {
                    let surface_y = plat.slope_surface_y(obj_center_x);
                    if obj.position.1 + obj.size.1 <= surface_y {
                        continue;
                    }
                    const SLOPE_TOLERANCE: f32 = 20.0;
                    let prev_bottom = (obj.position.1 + obj.size.1) - obj.momentum.1;
                    let prev_cx = obj_center_x - obj.momentum.0;
                    let surface_prev = plat.slope_surface_y(prev_cx);
                    if prev_bottom > surface_prev + SLOPE_TOLERANCE {
                        continue;
                    }
                    (0.0, (surface_y - obj.size.1) - obj.position.1)
                } else if plat.rotation != 0.0 {
                    let surface_y = rotated_surface_y(plat, obj_center_x);
                    let obj_bottom = obj.position.1 + obj.size.1;
                    if obj_bottom <= surface_y {
                        continue;
                    }
                    const ROT_TOLERANCE: f32 = 20.0;
                    let prev_bottom = obj_bottom - obj.momentum.1;
                    let prev_cx = obj_center_x - obj.momentum.0;
                    let surface_at_prev = rotated_surface_y(plat, prev_cx);
                    if prev_bottom > surface_at_prev + ROT_TOLERANCE {
                        continue;
                    }
                    (0.0, (surface_y - obj.size.1) - obj.position.1)
                } else {
                    let depth = penetration_depth(obj, plat, nx, ny);
                    if depth <= 0.0 { continue; }
                    (nx * depth, ny * depth)
                };

                adjustments.push((obj_idx, dx, dy, plat_idx));
            }
        }

        let cam_off = self.active_camera.as_ref()
            .map(|c| c.position)
            .unwrap_or((0.0, 0.0));

        for (obj_idx, dx, dy, plat_idx) in adjustments {
            let plat = &self.store.objects[plat_idx];
            let (mut nx, mut ny) = plat.surface_normal;
            let surf_vel  = plat.surface_velocity;

            // Mirror the approach-check flip: rotated platforms always
            // use the upward-facing normal for momentum cancellation.
            if plat.rotation != 0.0 && plat.slope.is_none() && ny > 0.0 {
                nx = -nx;
                ny = -ny;
            }

            let obj = &mut self.store.objects[obj_idx];

            let inward_speed = obj.momentum.0 * (-nx) + obj.momentum.1 * (-ny);
            if inward_speed > 0.0 {
                obj.momentum.0 += nx * inward_speed;
                obj.momentum.1 += ny * inward_speed;
            }

            obj.position.0 += dx;
            obj.position.1 += dy;

            let adj = rotation_adjusted_offset(
                obj.position, obj.size, obj.rotation, obj.slope.is_some(),
            );
            self.layout.offsets[obj_idx] = (
                adj.0 - cam_off.0,
                adj.1 - cam_off.1,
            );

            if let Some(vx) = surf_vel {
                self.store.objects[obj_idx].momentum.0 += -ny * vx;
                self.store.objects[obj_idx].momentum.1 +=  nx * vx;
            }
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

    // ── game_vars accessors ──────────────────────────────────────────

    pub fn set_var(&mut self, key: impl Into<String>, val: impl Into<value::Value>) {
        self.game_vars.insert(key.into(), val.into());
    }

    pub fn get_var(&self, key: &str) -> Option<&value::Value> {
        self.game_vars.get(key)
    }

    pub fn has_var(&self, key: &str) -> bool {
        self.game_vars.contains_key(key)
    }

    pub fn remove_var(&mut self, key: &str) -> Option<value::Value> {
        self.game_vars.remove(key)
    }

    pub fn resolve(&self, key: &str) -> value::Value {
        self.game_vars.get(key).cloned().unwrap_or(value::Value::F32(0.0))
    }

    pub fn modify_var(&mut self, key: &str, op: value::MathOp, rhs: value::Value) {
        let current = self.resolve(key);
        let result  = value::apply_op(&current, &rhs, &op);
        if let Some(val) = result {
            self.game_vars.insert(key.to_string(), val);
        }
    }

    pub fn get_f32(&self, key: &str) -> f32 {
        match self.game_vars.get(key) {
            Some(value::Value::F32(v)) => *v,
            Some(value::Value::F64(v)) => *v as f32,
            Some(value::Value::I32(v)) => *v as f32,
            _ => 0.0,
        }
    }

    pub fn get_f64(&self, key: &str) -> f64 {
        match self.game_vars.get(key) {
            Some(value::Value::F64(v)) => *v,
            Some(value::Value::F32(v)) => *v as f64,
            Some(value::Value::I32(v)) => *v as f64,
            _ => 0.0,
        }
    }

    pub fn get_i32(&self, key: &str) -> i32 {
        match self.game_vars.get(key) {
            Some(value::Value::I32(v)) => *v,
            Some(value::Value::F32(v)) => *v as i32,
            _ => 0,
        }
    }

    pub fn get_bool(&self, key: &str) -> bool {
        match self.game_vars.get(key) {
            Some(value::Value::Bool(v)) => *v,
            _ => false,
        }
    }

    pub fn get_str(&self, key: &str) -> String {
        match self.game_vars.get(key) {
            Some(value::Value::Str(s)) => s.clone(),
            Some(v) => format!("{:?}", v),
            None => String::new(),
        }
    }

    // ── pause / resume ───────────────────────────────────────────────

    pub fn pause(&mut self)  { self.paused = true; }
    pub fn resume(&mut self) { self.paused = false; }
    pub fn is_paused(&self) -> bool { self.paused }
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

// ── Free helper functions ────────────────────────────────────────────

fn rotation_adjusted_offset(
    position: (f32, f32),
    size: (f32, f32),
    rotation: f32,
    has_slope: bool,
) -> (f32, f32) {
    if rotation == 0.0 || has_slope {
        return position;
    }
    let theta = rotation.to_radians();
    let cos_t = theta.cos().abs();
    let sin_t = theta.sin().abs();
    let new_w = size.0 * cos_t + size.1 * sin_t;
    let new_h = size.0 * sin_t + size.1 * cos_t;
    let dx = (new_w - size.0) * 0.5;
    let dy = (new_h - size.1) * 0.5;
    (position.0 - dx, position.1 - dy)
}

fn rotated_aabb(obj: &object::GameObject) -> (f32, f32, f32, f32) {
    if obj.rotation == 0.0 {
        return (obj.position.0, obj.position.1, obj.size.0, obj.size.1);
    }
    let theta = obj.rotation.to_radians();
    let cos_t = theta.cos().abs();
    let sin_t = theta.sin().abs();
    let w = obj.size.0 * cos_t + obj.size.1 * sin_t;
    let h = obj.size.0 * sin_t + obj.size.1 * cos_t;
    let cx = obj.position.0 + obj.size.0 * 0.5;
    let cy = obj.position.1 + obj.size.1 * 0.5;
    (cx - w * 0.5, cy - h * 0.5, w, h)
}

fn rotated_surface_y(plat: &object::GameObject, world_x: f32) -> f32 {
    let cx = plat.position.0 + plat.size.0 * 0.5;
    let cy = plat.position.1 + plat.size.1 * 0.5;
    let theta = plat.rotation.to_radians();
    let cos_t = theta.cos();
    let sin_t = theta.sin();
    let half_w = plat.size.0 * 0.5;
    let half_h = plat.size.1 * 0.5;
    let dx = (world_x - cx).clamp(-half_w, half_w);
    let cos_abs = cos_t.abs().max(0.001);
    let cos_safe = if cos_t.abs() < 0.001 {
        0.001
    } else {
        cos_t
    };
    // Always returns the Y of whichever edge is currently on top:
    // Original top edge when |rotation| < 90, original bottom edge otherwise.
    cy + dx * sin_t / cos_safe - half_h / cos_abs
}

fn penetration_depth(
    obj: &object::GameObject,
    plat: &object::GameObject,
    nx: f32,
    ny: f32,
) -> f32 {
    let obj_cx  = obj.position.0  + obj.size.0  * 0.5;
    let obj_cy  = obj.position.1  + obj.size.1  * 0.5;
    let plat_cx = plat.position.0 + plat.size.0 * 0.5;
    let plat_cy = plat.position.1 + plat.size.1 * 0.5;

    let obj_half  = (obj.size.0  * nx.abs() + obj.size.1  * ny.abs()) * 0.5;
    let plat_half = (plat.size.0 * nx.abs() + plat.size.1 * ny.abs()) * 0.5;

    let sep = (obj_cx - plat_cx) * nx + (obj_cy - plat_cy) * ny;
    let overlap = obj_half + plat_half - sep.abs();
    if overlap > 0.0 { overlap } else { 0.0 }
}