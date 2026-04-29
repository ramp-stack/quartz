use super::core::Canvas;
use crate::object;
use crate::types::{CollisionMode, CollisionShape, GameEvent, GravityFalloff, Target};

/// Shared constant. An object at exactly planet_radius × GRAVITY_INFLUENCE_MULT
/// is at the edge of the gravity field and receives zero pull.
pub(crate) const GRAVITY_INFLUENCE_MULT: f32 = 3.0;

/// How strongly non-dominant planet forces are dampened when an object is deep
/// inside the dominant (closest) planet's gravity field.  At the dominant
/// planet's surface the dampening factor reaches this value (0.0–1.0).
/// 0.9 means non-dominant forces are reduced to 10 % at the surface.
pub(crate) const NESTED_GRAVITY_DAMPENING: f32 = 0.9;

/// Compute the gravitational force vector from one planet onto one receiver.
///
/// Returns Some((fx, fy, pull_magnitude)) when the planet is in range,
/// None when the planet is outside the influence field or distance is negligible.
pub(crate) fn compute_gravity_force(
    obj_cx:             f32,
    obj_cy:             f32,
    obj_strength:       f32,
    obj_influence_mult: f32,
    obj_falloff:        GravityFalloff,
    planet_cx:          f32,
    planet_cy:          f32,
    planet_radius:      f32,
    planet_strength:    f32,
) -> Option<(f32, f32, f32)> {
    let dx   = planet_cx - obj_cx;
    let dy   = planet_cy - obj_cy;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 1.0 { return None; }

    let field_radius = planet_radius * obj_influence_mult;
    if dist > field_radius { return None; }

    let pull = match obj_falloff {
        GravityFalloff::Linear => {
            obj_strength * planet_strength * planet_radius / dist
        }
        GravityFalloff::InverseSquare => {
            obj_strength * planet_strength * (planet_radius * planet_radius) / (dist * dist)
        }
    };

    let nx = dx / dist;
    let ny = dy / dist;
    Some((nx * pull, ny * pull, pull))
}

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
        self.apply_directional_gravity();

        let scale = self.layout.scale.get();
        let has_crystalline = self.crystalline.is_some();

        // ignore_zoom objects need base_scale (without zoom) for their
        // shape/text sizing so it matches what build() applies to them.
        let zoom = self.layout.zoom.get().max(0.01);
        let base_scale = if zoom.abs() > f32::EPSILON { scale / zoom } else { scale };

        for (idx, obj) in self.store.objects.iter_mut().enumerate() {
            obj.grounded = false;
            let obj_scale = if obj.ignore_zoom { base_scale } else { scale };
            obj.scaled_size.set((obj.size.0 * obj_scale, obj.size.1 * obj_scale));
            obj.render_scale.set(obj_scale);
            obj.update_animation(delta_time);

            if obj.visible {
                if !has_crystalline {
                    obj.apply_gravity();
                    obj.update_position();
                    obj.apply_resistance();
                    obj.apply_rotation_momentum();
                }                
                if obj.animated_sprite.is_none() {
                    obj.update_image_shape();
                }
                self.layout.offsets[idx] = rotation_adjusted_offset(
                    obj.position,
                    obj.size,
                    obj.rotation,
                    obj.slope.is_some(),
                    obj.pivot,
                );
            }
        }

        self.handle_infinite_scroll();
    }

    pub(crate) fn apply_camera_transform(&mut self) {
        let mut cam = match self.active_camera.take() {
            Some(c) => c,
            None => return,
        };

        // Advance the zoom lerp (always, even without a follow target).
        cam.advance_zoom_lerp();

        if let Some(target) = cam.follow_target.clone() {
            if let Some(&idx) = self.store.get_indices(&target).first() {
                if let Some(obj) = self.store.objects.get(idx) {
                    let cx = obj.position.0 + obj.size.0 * 0.5;
                    let cy = obj.position.1 + obj.size.1 * 0.5;
                    cam.lerp_toward(cx, cy);
                }
            }
        }

        // Advance camera effects (shake, flash, zoom punch).
        cam.effects.update(1.0 / 60.0);

        // Additive offsets from effects — not fed back into cam.position/zoom.
        let shake_offset = cam.effects.shake_offset();
        let zoom_punch   = cam.effects.zoom_punch_amount();
        let effective_zoom = (cam.zoom + zoom_punch).max(0.01);

        let (cam_x, cam_y) = (cam.position.0 + shake_offset.0, cam.position.1 + shake_offset.1);
        for (idx, obj) in self.store.objects.iter().enumerate() {
            let adj = rotation_adjusted_offset(
                obj.position,
                obj.size,
                obj.rotation,
                obj.slope.is_some(),
                obj.pivot,
            );

            if let Some(pin) = &obj.screen_pin {
                let (vw, vh) = self.layout.canvas_size.get();
                let px = vw * pin.anchor.0 + pin.offset.0 - obj.size.0 * pin.anchor.0;
                let py = vh * pin.anchor.1 + pin.offset.1 - obj.size.1 * pin.anchor.1;
                let safe = self.layout.safe_area_offset.get();
                self.layout.offsets[idx] = (px + safe.0, py + safe.1);
            } else if obj.ignore_zoom {
                self.layout.offsets[idx] = adj;
            } else {
                self.layout.offsets[idx] = (adj.0 - cam_x, adj.1 - cam_y);
            }
        }

        // Particle offsets are handled by rebuild_particle_visuals (runs after this).

        // Propagate camera zoom to the layout so build() scales everything.
        self.layout.zoom.set(effective_zoom);

        // ── Auto flash overlay ────────────────────────────────────────────────
        // The engine manages an internal fullscreen object for camera flash.
        // This way cam.flash() "just works" without user-side wiring.
        self.drive_flash_overlay(&cam);

        self.active_camera = Some(cam);
    }

    /// Internal name for the auto-managed flash overlay object.
    const FLASH_OVERLAY_NAME: &'static str = "__quartz_flash_overlay";

    /// Create or update the internal flash overlay so `cam.flash()` renders
    /// automatically without requiring user-side wiring.
    fn drive_flash_overlay(&mut self, cam: &crate::camera::Camera) {
        let overlay_color = cam.effects.flash_overlay_color();

        if let Some(color) = overlay_color {
            let (vw, vh) = self.layout.canvas_size.get();

            if let Some(&idx) = self.store.name_to_index.get(Self::FLASH_OVERLAY_NAME) {
                // Update existing overlay
                let obj = &mut self.store.objects[idx];
                obj.visible = true;
                obj.size = (vw, vh);
                let img = crate::sprite::tint_overlay(vw, vh, color);
                obj.set_image(img);
            } else {
                // Create the overlay object on first flash
                let img = crate::sprite::tint_overlay(vw, vh, color);
                let mut obj = crate::object::GameObject::build(Self::FLASH_OVERLAY_NAME)
                    .position(0.0, 0.0)
                    .size(vw, vh)
                    .image(img)
                    .layer(i32::MAX)
                    .ignore_zoom()
                    .finish();
                obj.collision_mode = crate::types::CollisionMode::NonPlatform;
                self.add_game_object(Self::FLASH_OVERLAY_NAME.to_string(), obj);
            }
        } else {
            // No active flash — hide the overlay if it exists
            if let Some(&idx) = self.store.name_to_index.get(Self::FLASH_OVERLAY_NAME) {
                self.store.objects[idx].visible = false;
            }
        }
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
            let (nx, ny) = match &plat.collision_mode {
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

            let adj = rotation_adjusted_offset(
                obj.position,
                obj.size,
                obj.rotation,
                obj.slope.is_some(),
                obj.pivot,
            );
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

    // ── Planet gravity (legacy path) ─────────────────────────────────────

    pub(crate) fn apply_directional_gravity(&mut self) {
        if self.crystalline.is_some() { return; }

        struct PlanetSnapshot {
            id:       String,
            cx:       f32,
            cy:       f32,
            radius:   f32,
            strength: f32,
            tags:     Vec<String>,
        }
        let planets: Vec<PlanetSnapshot> = self.store.objects.iter().map(|obj| {
            let cx = obj.position.0 + obj.size.0 * 0.5;
            let cy = obj.position.1 + obj.size.1 * 0.5;
            PlanetSnapshot {
                id:       obj.id.clone(),
                cx, cy,
                radius:   obj.planet_radius.unwrap_or(0.0),
                strength: obj.gravity_strength,
                tags:     obj.tags.clone(),
            }
        })
        .filter(|p| p.radius > 0.0)
        .collect();

        struct GravityResult {
            idx:         usize,
            fx:          f32,
            fy:          f32,
            dominant_id: Option<String>,
        }
        let mut results: Vec<GravityResult> = Vec::new();

        for (obj_idx, obj) in self.store.objects.iter().enumerate() {
            if !obj.visible { continue; }

            let tag_filter: Option<&str> = if obj.gravity_all_sources {
                None
            } else {
                match &obj.gravity_target {
                    Some(t) => Some(t.as_str()),
                    None    => continue,
                }
            };

            let obj_strength       = obj.gravity_strength;
            let obj_influence_mult = obj.gravity_influence_mult;
            let obj_falloff        = obj.gravity_falloff;
            let obj_cx             = obj.position.0 + obj.size.0 * 0.5;
            let obj_cy             = obj.position.1 + obj.size.1 * 0.5;

            let mut total_fx       = 0.0_f32;
            let mut total_fy       = 0.0_f32;
            let mut dominant_id: Option<String> = None;

            // -- First pass: collect per-planet forces and find dominant ------
            struct PlanetForce {
                planet_idx: usize,
                fx:  f32,
                fy:  f32,
                surface_dist: f32,
                planet_radius: f32,
            }
            let mut planet_forces: Vec<PlanetForce> = Vec::new();
            let mut dom_pf_idx: Option<usize> = None;
            let mut dom_surface_dist = f32::MAX;

            for (pi, planet) in planets.iter().enumerate() {
                if planet.radius <= 0.0 { continue; }

                if let Some(tag) = tag_filter {
                    if !planet.tags.iter().any(|t| t == tag) { continue; }
                }

                if let Some((fx, fy, _pull)) = compute_gravity_force(
                    obj_cx, obj_cy,
                    obj_strength, obj_influence_mult, obj_falloff,
                    planet.cx, planet.cy, planet.radius, planet.strength,
                ) {
                    let dx = obj_cx - planet.cx;
                    let dy = obj_cy - planet.cy;
                    let surface_dist = (dx * dx + dy * dy).sqrt() - planet.radius;
                    if surface_dist < dom_surface_dist {
                        dom_surface_dist = surface_dist;
                        dom_pf_idx = Some(planet_forces.len());
                    }
                    planet_forces.push(PlanetForce {
                        planet_idx: pi, fx, fy, surface_dist, planet_radius: planet.radius,
                    });
                }
            }

            // -- Second pass: accumulate with nested-field dampening ----------
            let dampening_mult = if let Some(di) = dom_pf_idx {
                let dom_r = planet_forces[di].planet_radius;
                let max_field_surf = dom_r * (obj_influence_mult - 1.0);
                if max_field_surf > 0.0 {
                    let depth = 1.0 - (dom_surface_dist / max_field_surf).clamp(0.0, 1.0);
                    1.0 - depth * NESTED_GRAVITY_DAMPENING
                } else {
                    1.0
                }
            } else {
                1.0
            };

            for (i, pf) in planet_forces.iter().enumerate() {
                if Some(i) == dom_pf_idx {
                    total_fx += pf.fx;
                    total_fy += pf.fy;
                    dominant_id = Some(planets[pf.planet_idx].id.clone());
                } else {
                    total_fx += pf.fx * dampening_mult;
                    total_fy += pf.fy * dampening_mult;
                }
            }

            results.push(GravityResult {
                idx: obj_idx,
                fx: total_fx,
                fy: total_fy,
                dominant_id,
            });
        }

        for result in results {
            let obj = &mut self.store.objects[result.idx];
            if result.fx != 0.0 || result.fy != 0.0 {
                obj.momentum.0 += result.fx;
                obj.momentum.1 += result.fy;
            }
            obj.gravity_dominant_id = result.dominant_id;
        }
    }

    pub(crate) fn handle_planet_landings(&mut self) {
        if self.crystalline.is_some() { return; }

        let planets: Vec<(String, f32, f32, f32)> = self.store.objects
            .iter()
            .filter_map(|obj| {
                let r  = obj.planet_radius?;
                let cx = obj.position.0 + obj.size.0 * 0.5;
                let cy = obj.position.1 + obj.size.1 * 0.5;
                Some((obj.id.clone(), cx, cy, r))
            })
            .collect();

        let mut corrections: Vec<(usize, f32, f32, f32, f32)> = Vec::new();

        'outer: for (obj_idx, obj) in self.store.objects.iter().enumerate() {
            if !obj.visible || obj.planet_radius.is_some() { continue; }

            let obj_cx = obj.position.0 + obj.size.0 * 0.5;
            let obj_cy = obj.position.1 + obj.size.1 * 0.5;

            // Try the dominant planet first — it is the physically correct landing
            // target when multiple planet surfaces are near each other.
            let dominant = obj.gravity_dominant_id.as_deref();
            let ordered: Vec<&(String, f32, f32, f32)> = {
                let mut v: Vec<&_> = planets.iter()
                    .filter(|(id, _, _, _)| Some(id.as_str()) == dominant)
                    .collect();
                v.extend(planets.iter()
                    .filter(|(id, _, _, _)| Some(id.as_str()) != dominant));
                v
            };

            for (_, planet_cx, planet_cy, radius) in ordered {
                let dx   = obj_cx - planet_cx;
                let dy   = obj_cy - planet_cy;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist >= *radius || dist < 0.01 { continue; }

                let nx = dx / dist;
                let ny = dy / dist;
                let new_pos_x = planet_cx + nx * radius - obj.size.0 * 0.5;
                let new_pos_y = planet_cy + ny * radius - obj.size.1 * 0.5;
                corrections.push((obj_idx, new_pos_x, new_pos_y, nx, ny));
                continue 'outer;
            }
        }

        for (obj_idx, new_x, new_y, nx, ny) in corrections {
            let obj = &mut self.store.objects[obj_idx];

            let inward = obj.momentum.0 * (-nx) + obj.momentum.1 * (-ny);
            if inward > 0.0 {
                obj.momentum.0 += nx * inward;
                obj.momentum.1 += ny * inward;
            }

            obj.position.0 = new_x;
            obj.position.1 = new_y;

            let adj = rotation_adjusted_offset(
                obj.position,
                obj.size,
                obj.rotation,
                obj.slope.is_some(),
                obj.pivot,
            );
            if let Some(offset) = self.layout.offsets.get_mut(obj_idx) {
                *offset = adj;
            }
        }
    }

    pub(crate) fn apply_auto_align(&mut self) {
        let planets_tagged: Vec<(String, Vec<String>, f32, f32, f32)> = self.store.objects
            .iter()
            .filter_map(|obj| {
                let r = obj.planet_radius?;
                let cx = obj.position.0 + obj.size.0 * 0.5;
                let cy = obj.position.1 + obj.size.1 * 0.5;
                Some((obj.id.clone(), obj.tags.clone(), cx, cy, r))
            })
            .collect();

        let mut adjustments: Vec<(usize, f32)> = Vec::new();

        for (obj_idx, obj) in self.store.objects.iter().enumerate() {
            if !obj.auto_align || !obj.visible { continue; }

            let obj_cx = obj.position.0 + obj.size.0 * 0.5;
            let obj_cy = obj.position.1 + obj.size.1 * 0.5;
            let speed  = obj.auto_align_speed;
            let thresh = obj.auto_align_threshold;
            let obj_influence_mult = obj.gravity_influence_mult;

            // Resolve target planet and its radius for depth scaling.
            // Primary: use gravity_dominant_id (set by gravity pass when in-field).
            // Fallback: nearest planet within the gravity field.
            let planet_info: Option<(f32, f32, f32)> = if let Some(dom_id) = &obj.gravity_dominant_id {
                planets_tagged.iter()
                    .find(|(id, _, _, _, _)| id == dom_id)
                    .map(|(_, _, pcx, pcy, pr)| (*pcx, *pcy, *pr))
            } else {
                None
            };

            let (planet_cx, planet_cy, planet_r) = match planet_info {
                Some(p) => p,
                None => {
                    // Fallback: find nearest planet within effective gravity range.
                    let nearest = if let Some(target_tag) = &obj.gravity_target {
                        planets_tagged.iter()
                            .filter(|(_, tags, _, _, _)| tags.iter().any(|t| t == target_tag))
                            .map(|(_, _, pcx, pcy, pr)| {
                                let dx = pcx - obj_cx;
                                let dy = pcy - obj_cy;
                                let dist = (dx * dx + dy * dy).sqrt();
                                (dist, *pcx, *pcy, *pr)
                            })
                            .filter(|(d, _, _, pr)| *d > 0.01 && *d <= *pr * obj_influence_mult)
                            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                    } else if obj.gravity_all_sources {
                        planets_tagged.iter()
                            .map(|(_, _, pcx, pcy, pr)| {
                                let dx = pcx - obj_cx;
                                let dy = pcy - obj_cy;
                                let dist = (dx * dx + dy * dy).sqrt();
                                (dist, *pcx, *pcy, *pr)
                            })
                            .filter(|(d, _, _, pr)| *d > 0.01 && *d <= *pr * obj_influence_mult)
                            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                    } else {
                        continue;
                    };
                    match nearest {
                        Some((_, pcx, pcy, pr)) => (pcx, pcy, pr),
                        None => continue,
                    }
                }
            };

            let dx = obj_cx - planet_cx;
            let dy = obj_cy - planet_cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let target_angle = dy.atan2(dx).to_degrees() + 90.0;
            let diff = shortest_angle_diff(obj.rotation, target_angle);

            if diff.abs() > thresh { continue; }

            // Scale alignment strength by how deep into the gravity field
            // the object is.  depth_raw goes from 0.0 at the field edge to
            // 1.0 at the planet surface.  auto_align_min_depth controls the
            // threshold: the object must be at least that fraction into the
            // field before any alignment applies.  Above the threshold the
            // strength ramps linearly to 1.0 at the surface.
            let field_radius = planet_r * obj_influence_mult;
            let min_depth = obj.auto_align_min_depth;
            let depth_factor = if field_radius > planet_r {
                let depth_raw = ((field_radius - dist) / (field_radius - planet_r)).clamp(0.0, 1.0);
                if depth_raw < min_depth {
                    0.0
                } else if min_depth >= 1.0 {
                    1.0
                } else {
                    (depth_raw - min_depth) / (1.0 - min_depth)
                }
            } else {
                1.0
            };

            let push = diff.signum() * speed.min(diff.abs()) * depth_factor;
            adjustments.push((obj_idx, push));
        }

        for (idx, push) in adjustments {
            self.store.objects[idx].rotation_momentum += push;
        }
    }
}

fn shortest_angle_diff(from: f32, to: f32) -> f32 {
    let diff = (to - from).rem_euclid(360.0);
    if diff > 180.0 { diff - 360.0 } else { diff }
}

// ── Free helpers ─────────────────────────────────────────────────────────────

pub(crate) fn rotation_adjusted_offset(
    position:  (f32, f32),
    size:      (f32, f32),
    rotation:  f32,
    has_slope: bool,
    pivot:     (f32, f32),
) -> (f32, f32) {
    if rotation == 0.0 || has_slope { return position; }

    let theta = rotation.to_radians();
    let cos_t = theta.cos();
    let sin_t = theta.sin();

    // World position of the rotation origin.
    let pw_x = position.0 + size.0 * pivot.0;
    let pw_y = position.1 + size.1 * pivot.1;

    // Pivot-to-centre vector rotated into world space.
    let dx = size.0 * (0.5 - pivot.0);
    let dy = size.1 * (0.5 - pivot.1);
    let cx = pw_x + dx * cos_t - dy * sin_t;
    let cy = pw_y + dx * sin_t + dy * cos_t;

    // AABB half-extents of the rotated rectangle around its centre.
    let hw = size.0 * cos_t.abs() * 0.5 + size.1 * sin_t.abs() * 0.5;
    let hh = size.0 * sin_t.abs() * 0.5 + size.1 * cos_t.abs() * 0.5;

    // Top-left of the AABB — the layout offset consumed by the renderer.
    (cx - hw, cy - hh)
}

fn rotated_aabb(obj: &object::GameObject) -> (f32, f32, f32, f32) {
    if obj.rotation == 0.0 {
        return (obj.position.0, obj.position.1, obj.size.0, obj.size.1);
    }
    // Delegates to corners_world() — the single source of truth for the sweep.
    let world_corners = obj.corners_world();
    let min_x = world_corners.iter().map(|c| c.0).fold(f32::MAX, |a, b| a.min(b));
    let max_x = world_corners.iter().map(|c| c.0).fold(f32::MIN, |a, b| a.max(b));
    let min_y = world_corners.iter().map(|c| c.1).fold(f32::MAX, |a, b| a.min(b));
    let max_y = world_corners.iter().map(|c| c.1).fold(f32::MIN, |a, b| a.max(b));
    (min_x, min_y, max_x - min_x, max_y - min_y)
}

fn rotated_surface_y(plat: &object::GameObject, world_x: f32) -> f32 {
    // Rotation origin is the pivot world position.
    let (pw_x, pw_y) = plat.pivot_world();
    let theta = plat.rotation.to_radians();
    let cos_t = theta.cos();
    let sin_t = theta.sin();

    // Clamp world_x to the horizontal extent of the rotated top edge.
    let left_wx  = pw_x + (-plat.size.0 * plat.pivot.0) * cos_t
                        - (-plat.size.1 * plat.pivot.1) * sin_t;
    let right_wx = pw_x + ( plat.size.0 * (1.0 - plat.pivot.0)) * cos_t
                        - (-plat.size.1 * plat.pivot.1) * sin_t;
    let clamped_x = world_x.clamp(left_wx.min(right_wx), left_wx.max(right_wx));

    // Exact surface y: solve ex(t) = clamped_x for t, substitute into ey(t).
    let cos_safe = if cos_t.abs() < 0.001 { 0.001_f32.copysign(cos_t) } else { cos_t };
    let offset_x = clamped_x - pw_x + plat.size.1 * plat.pivot.1 * sin_t;
    let local_along = offset_x / cos_safe;
    pw_y + local_along * sin_t - plat.size.1 * plat.pivot.1 * cos_t
}

fn penetration_depth(obj: &object::GameObject, plat: &object::GameObject, nx: f32, ny: f32) -> f32 {
    let (obj_cx,  obj_cy)  = obj.rotated_center();
    let (plat_cx, plat_cy) = plat.rotated_center();
    let sep = ((obj_cx - plat_cx) * nx + (obj_cy - plat_cy) * ny).abs();

    let obj_half = {
        let (_, _, w, h) = rotated_aabb(obj);
        (w * nx.abs() + h * ny.abs()) * 0.5
    };
    let plat_half = {
        let (_, _, w, h) = rotated_aabb(plat);
        (w * nx.abs() + h * ny.abs()) * 0.5
    };
    let overlap = obj_half + plat_half - sep;
    if overlap > 0.0 { overlap } else { 0.0 }
}

fn resolve_solid_collision(
    obj: &object::GameObject, plat: &object::GameObject,
) -> Option<(f32, f32, u8)> {
    // Rotation origin for the platform frame is the platform's pivot.
    let (plat_pivot_x, plat_pivot_y) = plat.pivot_world();

    // Object's world-space rotated centre (used for the centre-based local
    // transform below — kept for backward compat but corners are authoritative).
    let (obj_cx, obj_cy) = obj.rotated_center();
    let _ = (obj_cx, obj_cy); // used in corner loop below via pivot/rotation

    // Inverse rotation for platform local frame.
    let inv_theta = -plat.rotation.to_radians();
    let (cos_t, sin_t) = (inv_theta.cos(), inv_theta.sin());

    // Platform local extents from its pivot (asymmetric for non-centre pivots).
    let plat_left  = -plat.size.0 * plat.pivot.0;
    let plat_right =  plat.size.0 * (1.0 - plat.pivot.0);
    let plat_top   = -plat.size.1 * plat.pivot.1;
    let plat_bot   =  plat.size.1 * (1.0 - plat.pivot.1);

    // Project all four of obj's world corners into the platform frame.
    let (opx, opy) = obj.pivot;
    let obj_pivot_world_x = obj.position.0 + obj.size.0 * opx;
    let obj_pivot_world_y = obj.position.1 + obj.size.1 * opy;
    let obj_self_theta = obj.rotation.to_radians();
    let (sc, ss) = (obj_self_theta.cos(), obj_self_theta.sin());

    let obj_local_corners = [
        (-obj.size.0 * opx,          -obj.size.1 * opy),
        ( obj.size.0 * (1.0 - opx),  -obj.size.1 * opy),
        (-obj.size.0 * opx,           obj.size.1 * (1.0 - opy)),
        ( obj.size.0 * (1.0 - opx),   obj.size.1 * (1.0 - opy)),
    ];

    let obj_corners_in_plat_frame: [(f32, f32); 4] = std::array::from_fn(|i| {
        let (lx, ly) = obj_local_corners[i];
        let wx = obj_pivot_world_x + lx * sc - ly * ss;
        let wy = obj_pivot_world_y + lx * ss + ly * sc;
        let rx = wx - plat_pivot_x;
        let ry = wy - plat_pivot_y;
        (rx * cos_t - ry * sin_t, rx * sin_t + ry * cos_t)
    });

    let obj_min_x = obj_corners_in_plat_frame.iter().map(|c| c.0).fold(f32::MAX, |a, b| a.min(b));
    let obj_max_x = obj_corners_in_plat_frame.iter().map(|c| c.0).fold(f32::MIN, |a, b| a.max(b));
    let obj_min_y = obj_corners_in_plat_frame.iter().map(|c| c.1).fold(f32::MAX, |a, b| a.min(b));
    let obj_max_y = obj_corners_in_plat_frame.iter().map(|c| c.1).fold(f32::MIN, |a, b| a.max(b));

    // Per-face penetration depths (positive = overlapping).
    let ox_neg = obj_max_x - plat_left;
    let ox_pos = plat_right - obj_min_x;
    let oy_neg = obj_max_y - plat_top;
    let oy_pos = plat_bot  - obj_min_y;

    let overlap_x = ox_neg.min(ox_pos);
    let overlap_y = oy_neg.min(oy_pos);
    if overlap_x <= 0.0 || overlap_y <= 0.0 { return None; }

    let (depth_x, local_nx) = if ox_neg < ox_pos { (ox_neg, -1.0_f32) } else { (ox_pos,  1.0_f32) };
    let (depth_y, local_ny) = if oy_neg < oy_pos { (oy_neg, -1.0_f32) } else { (oy_pos,  1.0_f32) };

    let (depth, lnx, lny, face) = if overlap_x < overlap_y {
        (depth_x, local_nx, 0.0_f32, if local_nx < 0.0 { 2u8 } else { 3u8 })
    } else {
        (depth_y, 0.0_f32, local_ny, if local_ny < 0.0 { 0u8 } else { 1u8 })
    };

    // Rotate the local normal back into world space.
    let fwd_theta = plat.rotation.to_radians();
    let (cos_f, sin_f) = (fwd_theta.cos(), fwd_theta.sin());
    Some((
        (lnx * cos_f - lny * sin_f) * depth,
        (lnx * sin_f + lny * cos_f) * depth,
        face,
    ))
}

fn resolve_circle_collision(
    obj: &object::GameObject, plat: &object::GameObject, radius: &f32,
) -> Option<(f32, f32)> {
    let r = if *radius <= 0.0 { plat.size.0.min(plat.size.1) * 0.5 } else { *radius };
    let (obj_cx,  obj_cy)  = obj.rotated_center();
    let (plat_cx, plat_cy) = plat.rotated_center();
    let dx = obj_cx - plat_cx;
    let dy = obj_cy - plat_cy;
    let dist = (dx * dx + dy * dy).sqrt();
    let combined = r + (obj.size.0 + obj.size.1) * 0.25;
    if dist >= combined { return None; }
    if dist < 0.001 { return Some((0.0, -combined)); }
    let overlap = combined - dist;
    Some((dx / dist * overlap, dy / dist * overlap))
}