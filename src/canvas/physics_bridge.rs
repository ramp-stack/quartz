use super::core::Canvas;
use crate::crystalline::{
    CrystallinePhysics, ParticleSystem, PhysicsBody, PhysicsConfig,
    PhysicsMaterial, PhysicsQuality, PhysicsStepResult,
    CrystallineCollisionMode, CollisionShape as CrysCollisionShape,
    Emitter,
};
use crate::types::CollisionMode;
use prism::canvas::{Image, ShapeType, Color};

// ---------------------------------------------------------------------------
// PhysicsBridge — public API for the game developer
// ---------------------------------------------------------------------------

impl Canvas {
    // -- Enable / configure -----------------------------------------------

    /// Opt-in to Crystalline physics. Call once (e.g. in `on_enter`).
    /// After this, the tick loop will route physics through Crystalline
    /// instead of the legacy per-object gravity/resistance/position loop.
    pub fn enable_crystalline(&mut self) {
        self.enable_crystalline_with(PhysicsConfig::default());
    }

    /// Opt-in with a custom config.
    pub fn enable_crystalline_with(&mut self, config: PhysicsConfig) {
        let ps = ParticleSystem::new(2048).with_config(config.clone());
        self.crystalline = Some(CrystallinePhysics::with_config(config));
        self.particle_system = Some(ps);
    }

    /// Disable Crystalline and return to the legacy physics path.
    pub fn disable_crystalline(&mut self) {
        self.crystalline = None;
        self.particle_system = None;
        self.last_particle_states.clear();
        self.particle_images.clear();
        self.layout.particle_offsets.clear();
    }

    /// Returns `true` if Crystalline is currently enabled.
    pub fn has_crystalline(&self) -> bool {
        self.crystalline.is_some()
    }

    /// Apply a quality preset (adjusts config substeps + iterations).
    pub fn set_physics_quality(&mut self, quality: PhysicsQuality) {
        if let Some(solver) = &mut self.crystalline {
            solver.set_quality(quality);
        }
    }

    /// Direct access to the solver (advanced).
    pub fn crystalline_solver(&self) -> Option<&CrystallinePhysics> {
        self.crystalline.as_ref()
    }

    /// Mutable access to the solver (advanced).
    pub fn crystalline_solver_mut(&mut self) -> Option<&mut CrystallinePhysics> {
        self.crystalline.as_mut()
    }

    // -- Forces / impulses ------------------------------------------------

    /// Queue a continuous force on a body (applied over the timestep, scaled by dt).
    /// `name` is the GameObject id.
    pub fn apply_physics_force(&mut self, name: &str, fx: f32, fy: f32) {
        if let (Some(solver), Some(&idx)) = (
            &mut self.crystalline,
            self.store.name_to_index.get(name),
        ) {
            solver.apply_force(idx, fx, fy);
        }
    }

    /// Queue an instantaneous impulse on a body.
    pub fn apply_physics_impulse(&mut self, name: &str, ix: f32, iy: f32) {
        if let (Some(solver), Some(&idx)) = (
            &mut self.crystalline,
            self.store.name_to_index.get(name),
        ) {
            solver.apply_impulse(idx, ix, iy);
        }
    }

    /// Wake a sleeping body.
    pub fn wake_body(&mut self, name: &str) {
        if let (Some(solver), Some(&idx)) = (
            &mut self.crystalline,
            self.store.name_to_index.get(name),
        ) {
            solver.wake(idx);
        }
    }

    /// Check if a body is sleeping.
    pub fn is_body_sleeping(&self, name: &str) -> bool {
        if let (Some(solver), Some(&idx)) = (
            &self.crystalline,
            self.store.name_to_index.get(name),
        ) {
            solver.is_sleeping(idx)
        } else {
            false
        }
    }

    // -- Physics query helpers --------------------------------------------

    /// Get an object's current velocity.
    pub fn get_velocity(&self, name: &str) -> Option<(f32, f32)> {
        self.store.name_to_index.get(name)
            .and_then(|&idx| self.store.objects.get(idx))
            .map(|obj| obj.momentum)
    }

    /// Get an object's scalar speed (magnitude of velocity).
    pub fn get_speed(&self, name: &str) -> Option<f32> {
        self.get_velocity(name)
            .map(|(vx, vy)| (vx * vx + vy * vy).sqrt())
    }

    /// Check if a body is grounded (resting on a platform).
    pub fn is_body_grounded(&self, name: &str) -> bool {
        self.store.name_to_index.get(name)
            .and_then(|&idx| self.store.objects.get(idx))
            .map_or(false, |obj| obj.grounded)
    }

    /// Return names of all bodies whose center is within `radius` of `pos`.
    pub fn bodies_in_radius(&self, pos: (f32, f32), radius: f32) -> Vec<String> {
        let r2 = radius * radius;
        self.store.objects.iter().zip(self.store.names.iter())
            .filter(|(obj, _)| {
                let cx = obj.position.0 + obj.size.0 * 0.5;
                let cy = obj.position.1 + obj.size.1 * 0.5;
                let dx = cx - pos.0;
                let dy = cy - pos.1;
                dx * dx + dy * dy <= r2
            })
            .map(|(_, name)| name.clone())
            .collect()
    }

    // -- Area effects -----------------------------------------------------

    /// Apply a radial force pushing objects away from center.
    pub fn apply_radial_force(&mut self, center: (f32, f32), radius: f32, force: f32) {
        let r2 = radius * radius;
        let names: Vec<(String, f32, f32)> = self.store.objects.iter()
            .zip(self.store.names.iter())
            .filter_map(|(obj, name)| {
                let cx = obj.position.0 + obj.size.0 * 0.5;
                let cy = obj.position.1 + obj.size.1 * 0.5;
                let dx = cx - center.0;
                let dy = cy - center.1;
                let dist2 = dx * dx + dy * dy;
                if dist2 > 0.0 && dist2 <= r2 {
                    let dist = dist2.sqrt();
                    let falloff = 1.0 - dist / radius;
                    let scale = force * falloff / dist;
                    Some((name.clone(), dx * scale, dy * scale))
                } else {
                    None
                }
            }).collect();
        for (name, fx, fy) in names {
            self.apply_physics_force(&name, fx, fy);
        }
    }

    /// Apply a radial impulse (instant, for explosions).
    pub fn apply_radial_impulse(&mut self, center: (f32, f32), radius: f32, impulse: f32) {
        let r2 = radius * radius;
        let names: Vec<(String, f32, f32)> = self.store.objects.iter()
            .zip(self.store.names.iter())
            .filter_map(|(obj, name)| {
                let cx = obj.position.0 + obj.size.0 * 0.5;
                let cy = obj.position.1 + obj.size.1 * 0.5;
                let dx = cx - center.0;
                let dy = cy - center.1;
                let dist2 = dx * dx + dy * dy;
                if dist2 > 0.0 && dist2 <= r2 {
                    let dist = dist2.sqrt();
                    let falloff = 1.0 - dist / radius;
                    let scale = impulse * falloff / dist;
                    Some((name.clone(), dx * scale, dy * scale))
                } else {
                    None
                }
            }).collect();
        for (name, ix, iy) in names {
            self.apply_physics_impulse(&name, ix, iy);
        }
    }

    // -- Global physics control -------------------------------------------

    /// Set world gravity scale (multiplicative).
    pub fn set_gravity_scale(&mut self, scale: f32) {
        if let Some(solver) = &mut self.crystalline {
            solver.config.gravity_scale = scale;
        }
    }

    /// Get the current gravity scale.
    pub fn get_gravity_scale(&self) -> f32 {
        self.crystalline.as_ref().map_or(1.0, |s| s.config.gravity_scale)
    }

    // -- Material helpers -------------------------------------------------

    /// Set a game object's physics material.
    pub fn set_material(&mut self, name: &str, mat: PhysicsMaterial) {
        if let Some(obj) = self.get_game_object_mut(name) {
            obj.material = mat;
        }
    }

    // -- Particle system --------------------------------------------------

    /// Add an emitter to the particle system.
    pub fn add_emitter(&mut self, emitter: Emitter) {
        if let Some(ps) = &mut self.particle_system {
            ps.add_emitter(emitter);
        }
    }

    /// Remove a named emitter.
    pub fn remove_emitter(&mut self, name: &str) {
        if let Some(ps) = &mut self.particle_system {
            ps.remove_emitter(name);
        }
    }

    /// Attach an emitter to a game object so its origin follows each frame.
    pub fn attach_emitter_to(&mut self, emitter_name: &str, object_name: &str) {
        let key = format!("_emitter_bind_{}", emitter_name);
        self.game_vars.insert(key, crate::value::Value::Str(object_name.into()));
    }

    /// Returns particle positions from the last step (for rendering).
    pub fn particle_positions(&self) -> &[crate::crystalline::ParticleState] {
        &self.last_particle_states
    }

    /// Mutable access to the particle system (advanced).
    pub fn particle_system_mut(&mut self) -> Option<&mut ParticleSystem> {
        self.particle_system.as_mut()
    }

    /// Spawn a one-shot burst of particles from an emitter definition.
    /// Unlike `spawn_emitter`, this does NOT add a persistent emitter —
    /// particles are created immediately and then dissipate on their own.
    pub fn spawn_particle_burst(&mut self, emitter: &crate::crystalline::Emitter, count: usize) {
        if let Some(ps) = &mut self.particle_system {
            ps.spawn_burst_from_emitter(emitter, count);
        }
    }

    // -- Planet gravity injection (crystalline path) ------------------------

    pub(crate) fn inject_planet_gravity(&mut self) {
        let planets: Vec<(usize, f32, f32, f32, Vec<String>)> = self.store.objects
            .iter()
            .enumerate()
            .filter_map(|(idx, obj)| {
                let r = obj.planet_radius?;
                let cx = obj.position.0 + obj.size.0 * 0.5;
                let cy = obj.position.1 + obj.size.1 * 0.5;
                Some((idx, cx, cy, r, obj.tags.clone()))
            })
            .collect();

        for (obj_idx, obj) in self.store.objects.iter().enumerate() {
            let tag = match &obj.gravity_target {
                Some(t) => t,
                None    => continue,
            };
            if !obj.visible || obj.is_platform { continue; }

            let strength = obj.gravity_strength;
            let obj_cx   = obj.position.0 + obj.size.0 * 0.5;
            let obj_cy   = obj.position.1 + obj.size.1 * 0.5;

            let mut fx = 0.0_f32;
            let mut fy = 0.0_f32;

            for &(planet_idx, pcx, pcy, radius, ref tags) in &planets {
                if planet_idx == obj_idx { continue; }
                if !tags.contains(tag) { continue; }

                let dx   = pcx - obj_cx;
                let dy   = pcy - obj_cy;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < 1.0 { continue; }

                let pull = strength * radius / dist;
                fx += (dx / dist) * pull;
                fy += (dy / dist) * pull;
            }

            if fx != 0.0 || fy != 0.0 {
                if let Some(solver) = &mut self.crystalline {
                    solver.apply_force(obj_idx, fx * 60.0, fy * 60.0);
                }
            }
        }
    }

    // -- Crystalline tick step --------------------------------------------

    /// Full crystalline physics step: build body snapshot → run solver →
    /// apply results → particles → visuals → legacy collision events.
    /// Called from the tick loop when crystalline is enabled.
    pub(crate) fn run_crystalline_step(&mut self, delta_time: f32) {
        // Sync emitter origins to attached objects.
        sync_emitter_origins(self);

        // Inject planet gravity forces before stepping.
        self.inject_planet_gravity();

        // Build physics body snapshot from store.
        let bodies = build_physics_bodies(self);

        // Step solver.
        if let Some(solver) = &mut self.crystalline {
            let result = solver.step(delta_time, &bodies);
            apply_physics_result(self, result);
        }

        // Step particle system.
        if let Some(ps) = &mut self.particle_system {
            let ps_result = ps.step(delta_time, None);
            self.last_particle_states = ps_result.particles;
        }

        // Build particle visuals for the render pipeline.
        self.rebuild_particle_visuals();

        // Still run legacy collision events (Collision, BoundaryCollision GameEvents)
        // so the user's event-driven logic continues to work.
        self.handle_collisions();
    }

    // -- Particle visual rebuild ------------------------------------------

    /// Rebuild particle Image drawables from the last step's output.
    pub(crate) fn rebuild_particle_visuals(&mut self) {
        self.particle_images.clear();
        self.layout.particle_offsets.clear();
        self.particle_render_layers.clear();

        if self.last_particle_states.is_empty() {
            self.rebuild_render_order();
            return;
        }

        use std::sync::Arc;
        use image::{RgbaImage, Rgba};
        let white_pixel: Arc<RgbaImage> = Arc::new(
            RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255])),
        );

        let zoom = self.layout.zoom.get().max(0.01);
        for ps in &self.last_particle_states {
            let (r, g, b, a) = ps.color;
            let s = ps.size * zoom;
            self.particle_images.push(Image {
                shape: ShapeType::RoundedRectangle(ps.rotation, (s, s), 0.0, s * 0.5),
                image: Arc::clone(&white_pixel),
                color: Some(Color(r, g, b, a)),
            });
            self.layout.particle_offsets.push(ps.position);
            self.particle_render_layers.push(ps.render_layer);
        }

        self.rebuild_render_order();
    }

    /// Refresh sorted_offsets from the live offset arrays without re-sorting.
    /// Call this at the end of every tick so `build()` sees current positions.
    pub(crate) fn sync_sorted_offsets(&mut self) {
        use super::core::RenderSlot;
        for (i, slot) in self.render_order.iter().enumerate() {
            let off = match slot {
                RenderSlot::Object(obj_i) => {
                    self.layout.offsets.get(*obj_i).copied().unwrap_or((0.0, 0.0))
                }
                RenderSlot::Particle(p_i) => {
                    self.layout.particle_offsets.get(*p_i).copied().unwrap_or((0.0, 0.0))
                }
            };
            if let Some(s) = self.layout.sorted_offsets.get_mut(i) {
                *s = off;
            }
        }
    }

    /// Build sorted render_order + sorted_offsets from object & particle layers.
    pub(crate) fn rebuild_render_order(&mut self) {
        use super::core::RenderSlot;

        let obj_count = self.store.objects.len();
        let part_count = self.particle_images.len();

        let mut slots: Vec<(i32, usize, RenderSlot)> = Vec::with_capacity(obj_count + part_count);

        for i in 0..obj_count {
            let layer = self.store.objects[i].layer;
            slots.push((layer, i, RenderSlot::Object(i)));
        }
        for i in 0..part_count {
            let layer = self.particle_render_layers.get(i).copied().unwrap_or(0);
            // Use obj_count + i as secondary key so particles at the same layer
            // sort after objects (preserving backward-compatible default).
            slots.push((layer, obj_count + i, RenderSlot::Particle(i)));
        }

        // Stable sort by layer first, then by original insertion order.
        slots.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

        self.render_order.clear();
        self.layout.sorted_offsets.clear();
        self.layout.sorted_ignore_zoom.clear();

        for &(_, _, slot) in &slots {
            self.render_order.push(slot);
            match slot {
                RenderSlot::Object(i)   => {
                    let off = self.layout.offsets.get(i).copied().unwrap_or((0.0, 0.0));
                    self.layout.sorted_offsets.push(off);
                    let no_zoom = self.store.objects.get(i).map_or(false, |o| o.ignore_zoom);
                    self.layout.sorted_ignore_zoom.push(no_zoom);
                }
                RenderSlot::Particle(i) => {
                    let off = self.layout.particle_offsets.get(i).copied().unwrap_or((0.0, 0.0));
                    self.layout.sorted_offsets.push(off);
                    self.layout.sorted_ignore_zoom.push(false);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Internal bridge helpers (called from the tick loop)
// ---------------------------------------------------------------------------

/// Build a `Vec<PhysicsBody>` snapshot from the current object store.
pub(crate) fn build_physics_bodies(canvas: &Canvas) -> Vec<PhysicsBody> {
    canvas.store.objects.iter().enumerate().map(|(idx, obj)| {
        PhysicsBody {
            id: idx,
            position: obj.position,
            size: obj.size,
            momentum: obj.momentum,
            gravity: obj.gravity,
            resistance: obj.resistance,
            rotation: obj.rotation,
            rotation_momentum: obj.rotation_momentum,
            rotation_resistance: obj.rotation_resistance,
            is_platform: obj.is_platform,
            visible: obj.visible,
            collision_mode: convert_collision_mode(&obj.collision_mode),
            surface_normal: obj.surface_normal,
            slope: obj.slope,
            one_way: obj.one_way,
            surface_velocity: obj.surface_velocity,
            material: obj.material,
            collision_layer: obj.collision_layer,
            planet_radius: obj.planet_radius,
            gravity_target: obj.gravity_target.clone(),
        }
    }).collect()
}

/// Write physics step results back into game objects.
pub(crate) fn apply_physics_result(canvas: &mut Canvas, result: PhysicsStepResult) {
    for update in result.body_updates {
        let (size, has_slope) = if let Some(obj) = canvas.store.objects.get_mut(update.id) {
            obj.position = update.position;
            obj.momentum = update.momentum;
            obj.rotation = update.rotation;
            obj.rotation_momentum = update.rotation_momentum;
            obj.grounded = update.grounded;
            (obj.size, obj.slope.is_some())
        } else {
            continue;
        };
        if let Some(offset) = canvas.layout.offsets.get_mut(update.id) {
            *offset = super::physics::rotation_adjusted_offset(
                update.position, size, update.rotation, has_slope,
            );
        }
    }
}

/// Update emitter origins that are attached to game objects.
pub(crate) fn sync_emitter_origins(canvas: &mut Canvas) {
    let bindings: Vec<(String, String)> = canvas
        .game_vars
        .iter()
        .filter_map(|(k, v)| {
            if let Some(emitter_name) = k.strip_prefix("_emitter_bind_") {
                if let crate::value::Value::Str(obj_name) = v {
                    return Some((emitter_name.to_string(), obj_name.clone()));
                }
            }
            None
        })
        .collect();

    for (emitter_name, obj_name) in bindings {
        let obj_data = canvas
            .store
            .name_to_index
            .get(obj_name.as_str())
            .and_then(|&idx| canvas.store.objects.get(idx))
            .map(|obj| (obj.position, obj.size, obj.rotation));

        let (obj_pos, obj_size, obj_rotation) = match obj_data {
            Some(d) => d,
            None => continue,
        };

        let ps = match &mut canvas.particle_system {
            Some(ps) => ps,
            None => continue,
        };

        // Compute origin: use Location if set, otherwise center of object
        let origin = if let Some(loc) = canvas.emitter_locations.get(&emitter_name) {
            // Resolve anchor/offset relative to object center, then rotate
            let (local_x, local_y) = match loc {
                crate::types::Location::Position(pos) => *pos,
                crate::types::Location::OnTarget { anchor, offset, .. } => {
                    // Anchor is 0..1 over object size, offset in local px
                    let ax = (anchor.x - 0.5) * obj_size.0 + offset.0;
                    let ay = (anchor.y - 0.5) * obj_size.1 + offset.1;
                    (ax, ay)
                }
                crate::types::Location::Relative { offset, .. } => *offset,
                _ => (0.0, 0.0),
            };
            // Rotate local offset by object rotation
            let rad = obj_rotation.to_radians();
            let cos_r = rad.cos();
            let sin_r = rad.sin();
            let rx = local_x * cos_r - local_y * sin_r;
            let ry = local_x * sin_r + local_y * cos_r;
            let cx = obj_pos.0 + obj_size.0 * 0.5;
            let cy = obj_pos.1 + obj_size.1 * 0.5;
            (cx + rx, cy + ry)
        } else {
            // Default: center of object
            (obj_pos.0 + obj_size.0 * 0.5, obj_pos.1 + obj_size.1 * 0.5)
        };

        ps.set_emitter_origin(&emitter_name, origin);
        ps.set_emitter_rotation(&emitter_name, obj_rotation);
    }
}

/// Convert quartz `CollisionMode` to crystalline `CrystallineCollisionMode`.
fn convert_collision_mode(mode: &CollisionMode) -> CrystallineCollisionMode {
    match mode {
        CollisionMode::NonPlatform => CrystallineCollisionMode::NonPlatform,
        CollisionMode::Surface => CrystallineCollisionMode::Surface,
        CollisionMode::Solid(shape) => {
            let cs = match shape {
                crate::types::CollisionShape::Rectangle => CrysCollisionShape::Rectangle,
                crate::types::CollisionShape::Circle { radius } => {
                    CrysCollisionShape::Circle { radius: *radius }
                }
            };
            CrystallineCollisionMode::Solid(cs)
        }
    }
}
