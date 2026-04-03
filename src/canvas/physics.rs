use super::core::Canvas;
use crate::object;
use crate::types::{CollisionMode, CollisionShape, GameEvent, Target};

impl Canvas {
    pub(crate) fn check_collision(o1: &object::GameObject, o2: &object::GameObject) -> bool {
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

        ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
    }

    pub(crate) fn trigger_collision_events(&mut self, idx: usize) {
        let actions: Vec<_> = self.store.events.get(idx).into_iter().flatten()
            .filter_map(|e| {
                if let GameEvent::Collision { action, .. } = e { Some(action.clone()) } else { None }
            })
            .collect();
        actions.into_iter().for_each(|a| self.run(a));
    }

    pub(crate) fn trigger_boundary_collision_events(&mut self, idx: usize) {
        let actions: Vec<_> = self.store.events.get(idx).into_iter().flatten()
            .filter_map(|e| {
                if let GameEvent::BoundaryCollision { action, .. } = e { Some(action.clone()) } else { None }
            })
            .collect();
        actions.into_iter().for_each(|a| self.run(a));
    }

    pub(crate) fn update_objects(&mut self, delta_time: f32) {
        let scale = self.layout.scale.get();

        for (idx, obj) in self.store.objects.iter_mut().enumerate() {
            obj.grounded = false;
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
        let mut collision_pairs: Vec<(usize, usize)> = Vec::new();

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

                match &plat.collision_mode {
                    CollisionMode::NonPlatform => { continue; }
                    CollisionMode::Solid(shape) => {
                        let result = match shape {
                            CollisionShape::Rectangle => {
                                resolve_solid_collision(obj, plat).map(|(dx, dy, _)| (dx, dy))
                            }
                            CollisionShape::Circle { radius } => {
                                resolve_circle_collision(obj, plat, radius)
                            }
                        };
                        if let Some((dx, dy)) = result {
                            let dist = (dx * dx + dy * dy).sqrt().max(0.001);
                            let nx = dx / dist;
                            let ny = dy / dist;
                            let approach = obj.momentum.0 * (-nx) + obj.momentum.1 * (-ny);
                            if approach > 0.0 {
                                adjustments.push((obj_idx, dx, dy, plat_idx));
                            }
                        }
                        continue;
                    }
                    CollisionMode::Surface => {}
                }

                let (mut nx, mut ny) = plat.surface_normal_at(obj_center_x);
                if plat.rotation != 0.0 && plat.slope.is_none() && ny > 0.0 {
                    nx = -nx; ny = -ny;
                }

                let approach_speed = obj.momentum.0 * (-nx) + obj.momentum.1 * (-ny);
                if approach_speed <= 0.0 { continue; }

                if plat.one_way {
                    if plat.slope.is_some() {
                        let prev_bottom = (obj.position.1 + obj.size.1) - obj.momentum.1;
                        let prev_cx = obj_center_x - obj.momentum.0;
                        if prev_bottom > plat.slope_surface_y(prev_cx) + 2.0 { continue; }
                    } else {
                        let obj_cx = obj.position.0 + obj.size.0 * 0.5;
                        let obj_cy = obj.position.1 + obj.size.1 * 0.5;
                        let plat_cx = plat.position.0 + plat.size.0 * 0.5;
                        let plat_cy = plat.position.1 + plat.size.1 * 0.5;
                        let prev_rel_x = (obj_cx - obj.momentum.0) - plat_cx;
                        let prev_rel_y = (obj_cy - obj.momentum.1) - plat_cy;
                        if !(prev_rel_x * nx + prev_rel_y * ny > 0.0) { continue; }
                    }
                }

                let (dx, dy) = if plat.slope.is_some() {
                    let surface_y = plat.slope_surface_y(obj_center_x);
                    if obj.position.1 + obj.size.1 <= surface_y { continue; }
                    let prev_bottom = (obj.position.1 + obj.size.1) - obj.momentum.1;
                    let prev_cx = obj_center_x - obj.momentum.0;
                    if prev_bottom > plat.slope_surface_y(prev_cx) + 20.0 { continue; }
                    (0.0, (surface_y - obj.size.1) - obj.position.1)
                } else if plat.rotation != 0.0 {
                    let surface_y = rotated_surface_y(plat, obj_center_x);
                    let obj_bottom = obj.position.1 + obj.size.1;
                    if obj_bottom <= surface_y { continue; }
                    let prev_bottom = obj_bottom - obj.momentum.1;
                    let prev_cx = obj_center_x - obj.momentum.0;
                    if prev_bottom > rotated_surface_y(plat, prev_cx) + 20.0 { continue; }
                    (0.0, (surface_y - obj.size.1) - obj.position.1)
                } else {
                    let depth = penetration_depth(obj, plat, nx, ny);
                    if depth <= 0.0 { continue; }
                    (nx * depth, ny * depth)
                };

                adjustments.push((obj_idx, dx, dy, plat_idx));
            }
        }

        let cam_off = self.active_camera.as_ref().map(|c| c.position).unwrap_or((0.0, 0.0));

        for (obj_idx, dx, dy, plat_idx) in adjustments {
            let plat = &self.store.objects[plat_idx];
            let (mut nx, mut ny) = match &plat.collision_mode {
                CollisionMode::Surface => {
                    let (mut nx, mut ny) = plat.surface_normal;
                    if plat.rotation != 0.0 && plat.slope.is_none() && ny > 0.0 { nx = -nx; ny = -ny; }
                    (nx, ny)
                }
                _ => {
                    let dist = (dx * dx + dy * dy).sqrt().max(0.001);
                    (dx / dist, dy / dist)
                }
            };

            let surf_vel = plat.surface_velocity;
            let obj = &mut self.store.objects[obj_idx];

            let inward_speed = obj.momentum.0 * (-nx) + obj.momentum.1 * (-ny);
            if inward_speed > 0.0 {
                obj.momentum.0 += nx * inward_speed;
                obj.momentum.1 += ny * inward_speed;
            }

            obj.position.0 += dx;
            obj.position.1 += dy;
            if ny < -0.3 { obj.grounded = true; }

            let adj = rotation_adjusted_offset(obj.position, obj.size, obj.rotation, obj.slope.is_some());
            self.layout.offsets[obj_idx] = (adj.0 - cam_off.0, adj.1 - cam_off.1);

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

    pub(crate) fn process_hot_reloads(&mut self, delta_time: f32) {
        self.hot_reload_timer += delta_time;
        if self.hot_reload_timer < 0.5 { return; }
        self.hot_reload_timer = 0.0;

        for obj in self.store.objects.iter_mut() {
            let image_path = obj.image_path.clone();
            let anim_path  = obj.animation_path.clone();
            if let Some(path) = image_path { obj.hot_reload_image(&path); }
            if let Some(path) = anim_path  { obj.hot_reload_animation(&path); }
        }

        let changed: Vec<(usize, Vec<u8>)> = self.file_watchers
            .iter_mut()
            .enumerate()
            .filter_map(|(i, w)| {
                let meta  = std::fs::metadata(&w.path).ok()?;
                let mtime = meta.modified().ok()?;
                if Some(mtime) == w.mtime { return None; }
                w.mtime = Some(mtime);
                match std::fs::read(&w.path) {
                    Ok(bytes) => Some((i, bytes)),
                    Err(e)    => { eprintln!("[hot-reload] read '{}': {e}", w.path); None }
                }
            })
            .collect();

        for (i, bytes) in changed {
            let mut watcher = self.file_watchers[i].clone();
            watcher.handler.call(self, &bytes);
            self.file_watchers[i] = watcher;
            println!("[hot-reload] file reloaded: {}", self.file_watchers[i].path);
        }
    }
}

// ── Free helpers ─────────────────────────────────────────────────────────────

pub(crate) fn rotation_adjusted_offset(
    position: (f32, f32), size: (f32, f32), rotation: f32, has_slope: bool,
) -> (f32, f32) {
    if rotation == 0.0 || has_slope { return position; }
    let theta = rotation.to_radians();
    let cos_t = theta.cos().abs();
    let sin_t = theta.sin().abs();
    let new_w = size.0 * cos_t + size.1 * sin_t;
    let new_h = size.0 * sin_t + size.1 * cos_t;
    (position.0 - (new_w - size.0) * 0.5, position.1 - (new_h - size.1) * 0.5)
}

fn rotated_aabb(obj: &object::GameObject) -> (f32, f32, f32, f32) {
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
    let dx = (world_x - cx).clamp(-plat.size.0 * 0.5, plat.size.0 * 0.5);
    let cos_safe = if cos_t.abs() < 0.001 { 0.001 } else { cos_t };
    let cos_abs  = cos_t.abs().max(0.001);
    cy + dx * sin_t / cos_safe - plat.size.1 * 0.5 / cos_abs
}

fn penetration_depth(obj: &object::GameObject, plat: &object::GameObject, nx: f32, ny: f32) -> f32 {
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

fn resolve_solid_collision(
    obj: &object::GameObject, plat: &object::GameObject,
) -> Option<(f32, f32, u8)> {
    let plat_cx = plat.position.0 + plat.size.0 * 0.5;
    let plat_cy = plat.position.1 + plat.size.1 * 0.5;
    let obj_cx  = obj.position.0  + obj.size.0  * 0.5;
    let obj_cy  = obj.position.1  + obj.size.1  * 0.5;
    let theta = -plat.rotation.to_radians();
    let (cos_t, sin_t) = (theta.cos(), theta.sin());
    let rel_x = obj_cx - plat_cx;
    let rel_y = obj_cy - plat_cy;
    let local_x = rel_x * cos_t - rel_y * sin_t;
    let local_y = rel_x * sin_t + rel_y * cos_t;
    let overlap_x = (plat.size.0 * 0.5 + obj.size.0 * 0.5) - local_x.abs();
    let overlap_y = (plat.size.1 * 0.5 + obj.size.1 * 0.5) - local_y.abs();
    if overlap_x <= 0.0 || overlap_y <= 0.0 { return None; }
    let mut candidates: Vec<(f32, f32, f32, u8)> = Vec::with_capacity(4);
    if local_y < 0.0  { candidates.push((overlap_y,  0.0, -1.0, 0)); }
    if local_y >= 0.0 { candidates.push((overlap_y,  0.0,  1.0, 1)); }
    if local_x < 0.0  { candidates.push((overlap_x, -1.0,  0.0, 2)); }
    if local_x >= 0.0 { candidates.push((overlap_x,  1.0,  0.0, 3)); }
    if candidates.is_empty() { return None; }
    let &(depth, local_nx, local_ny, face) = candidates.iter()
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();
    let theta = plat.rotation.to_radians();
    let (cos_t, sin_t) = (theta.cos(), theta.sin());
    Some((
        (local_nx * cos_t - local_ny * sin_t) * depth,
        (local_nx * sin_t + local_ny * cos_t) * depth,
        face,
    ))
}

fn resolve_circle_collision(
    obj: &object::GameObject, plat: &object::GameObject, radius: &f32,
) -> Option<(f32, f32)> {
    let r = if *radius <= 0.0 { plat.size.0.min(plat.size.1) * 0.5 } else { *radius };
    let dx = (obj.position.0 + obj.size.0 * 0.5) - (plat.position.0 + plat.size.0 * 0.5);
    let dy = (obj.position.1 + obj.size.1 * 0.5) - (plat.position.1 + plat.size.1 * 0.5);
    let dist = (dx * dx + dy * dy).sqrt();
    let combined = r + (obj.size.0 + obj.size.1) * 0.25;
    if dist >= combined { return None; }
    if dist < 0.001 { return Some((0.0, -combined)); }
    let overlap = combined - dist;
    Some((dx / dist * overlap, dy / dist * overlap))
}