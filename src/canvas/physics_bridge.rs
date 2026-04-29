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

    // -- Grapple constraint system -----------------------------------------

    /// Attach a grapple constraint to a named game object.
    /// If the object already has a grapple, it is replaced.
    pub fn attach_grapple(&mut self, name: &str, mut grapple: crate::constraints::GrappleConstraint) {
        // If anchor_object is set, resolve its current position as initial anchor
        if let Some(anchor_name) = &grapple.anchor_object {
            if let Some(anchor_obj) = self.get_game_object(anchor_name) {
                grapple.anchor = (
                    anchor_obj.position.0 + anchor_obj.size.0 * 0.5,
                    anchor_obj.position.1 + anchor_obj.size.1 * 0.5,
                );
            }
        }
        self.grapple_constraints.insert(name.to_string(), grapple);
        // Wake the body so the grapple takes effect immediately
        self.wake_body(name);
    }

    /// Release (remove) the grapple from a named game object.
    pub fn release_grapple(&mut self, name: &str) {
        self.grapple_constraints.remove(name);
    }

    /// Check if an object has an active grapple attached.
    pub fn has_grapple(&self, name: &str) -> bool {
        self.grapple_constraints.get(name).map_or(false, |g| g.active)
    }

    /// Get a reference to an object's grapple constraint (if any).
    pub fn get_grapple(&self, name: &str) -> Option<&crate::constraints::GrappleConstraint> {
        self.grapple_constraints.get(name)
    }

    /// Mutable access to an object's grapple constraint (advanced).
    pub fn get_grapple_mut(&mut self, name: &str) -> Option<&mut crate::constraints::GrappleConstraint> {
        self.grapple_constraints.get_mut(name)
    }

    /// Enforce grapple constraints by applying position/velocity corrections
    /// directly to the store. Called AFTER the physics solver step so
    /// corrections override the solver's output (XPBD-style).
    pub(crate) fn enforce_grapple_constraints(&mut self) {
        if self.grapple_constraints.is_empty() {
            return;
        }

        // First, update anchors for grapples attached to objects
        let anchor_updates: Vec<(String, (f32, f32))> = self.grapple_constraints.iter()
            .filter_map(|(name, grapple)| {
                let anchor_name = grapple.anchor_object.as_ref()?;
                let anchor_obj = self.store.name_to_index.get(anchor_name.as_str())
                    .and_then(|&idx| self.store.objects.get(idx))?;
                Some((name.clone(), (
                    anchor_obj.position.0 + anchor_obj.size.0 * 0.5,
                    anchor_obj.position.1 + anchor_obj.size.1 * 0.5,
                )))
            })
            .collect();

        for (name, anchor_pos) in anchor_updates {
            if let Some(g) = self.grapple_constraints.get_mut(&name) {
                g.anchor = anchor_pos;
            }
        }

        // Solve each grapple and collect corrections
        struct GrappleCorr {
            idx: usize,
            position: Option<(f32, f32)>,
            velocity: Option<(f32, f32)>,
        }
        let mut corrections: Vec<GrappleCorr> = Vec::new();

        let names: Vec<String> = self.grapple_constraints.keys().cloned().collect();
        for name in &names {
            let idx = match self.store.name_to_index.get(name.as_str()) {
                Some(&i) => i,
                None => continue,
            };
            let obj = match self.store.objects.get(idx) {
                Some(o) => o,
                None => continue,
            };
            let obj_center = (
                obj.position.0 + obj.size.0 * 0.5,
                obj.position.1 + obj.size.1 * 0.5,
            );
            let obj_vel = obj.momentum;
            let half_w = obj.size.0 * 0.5;
            let half_h = obj.size.1 * 0.5;

            if let Some(grapple) = self.grapple_constraints.get_mut(name.as_str()) {
                let correction = grapple.solve(obj_center, obj_vel);
                if correction.applied() {
                    // Convert center position back to top-left corner
                    corrections.push(GrappleCorr {
                        idx,
                        position: correction.position.map(|(cx, cy)| (cx - half_w, cy - half_h)),
                        velocity: correction.velocity,
                    });
                }
            }
        }

        // Apply corrections directly to store objects
        for corr in corrections {
            if let Some(obj) = self.store.objects.get_mut(corr.idx) {
                if let Some(pos) = corr.position {
                    obj.position = pos;
                }
                if let Some(vel) = corr.velocity {
                    obj.momentum = vel;
                }
            }
        }
    }

    // -- Planet gravity injection (crystalline path) ------------------------

    pub(crate) fn inject_planet_gravity(&mut self) {
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
            if !obj.visible || obj.is_platform { continue; }

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

                if let Some((fx, fy, _pull)) = super::physics::compute_gravity_force(
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
                    1.0 - depth * super::physics::NESTED_GRAVITY_DAMPENING
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
            if result.fx != 0.0 || result.fy != 0.0 {
                if let Some(solver) = &mut self.crystalline {
                    solver.apply_force(result.idx, result.fx * 60.0, result.fy * 60.0);
                }
            }
            if let Some(obj) = self.store.objects.get_mut(result.idx) {
                obj.gravity_dominant_id = result.dominant_id;
            }
        }
    }

    // -- Gravity query methods ---

    /// Returns the object name of the planet currently exerting the dominant
    /// gravitational pull on the named object. None when outside all fields.
    pub fn get_dominant_planet(&self, name: &str) -> Option<&str> {
        self.get_game_object(name)
            .and_then(|obj| obj.gravity_dominant_id.as_deref())
    }

    /// Returns names of all planets currently within the gravity influence field
    /// of the named object.
    pub fn planets_in_range(&self, name: &str) -> Vec<&str> {
        let obj = match self.get_game_object(name) {
            Some(o) => o,
            None    => return Vec::new(),
        };
        let influence_mult = obj.gravity_influence_mult;
        let obj_cx = obj.position.0 + obj.size.0 * 0.5;
        let obj_cy = obj.position.1 + obj.size.1 * 0.5;

        self.store.objects.iter()
            .zip(self.store.names.iter())
            .filter_map(|(planet, planet_name)| {
                let radius = planet.planet_radius?;
                let pcx = planet.position.0 + planet.size.0 * 0.5;
                let pcy = planet.position.1 + planet.size.1 * 0.5;
                let dx = obj_cx - pcx;
                let dy = obj_cy - pcy;
                let dist_sq = dx * dx + dy * dy;
                let field_r = radius * influence_mult;
                if dist_sq <= field_r * field_r {
                    Some(planet_name.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// True if the named object is currently within the gravity field of any
    /// planet matching its gravity_target tag (or any planet if gravity_all_sources).
    pub fn is_in_gravity_field(&self, name: &str) -> bool {
        self.get_dominant_planet(name).is_some()
    }

    /// True if the named object is within the gravity field of ANY planet,
    /// regardless of gravity_target tag.
    pub fn is_in_any_gravity_field(&self, name: &str) -> bool {
        !self.planets_in_range(name).is_empty()
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

        // Enforce grapple constraints AFTER the solver step.
        // Position-level correction: projects objects back onto the rope
        // arc and strips outward radial velocity. Must happen after the
        // solver integrates forces/velocity so corrections override.
        self.enforce_grapple_constraints();

        // Step particle system.
        if let Some(ps) = &mut self.particle_system {
            let ps_result = ps.step(delta_time, None);
            self.last_particle_states = ps_result.particles;
        }

        // Build particle visuals for the render pipeline.
        // (Moved to events.rs tick loop, after apply_camera_transform,
        //  so particles use the current frame's zoom/scale.)

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

        // Use layout.scale (= base_scale * zoom) so particles match game-object sizing.
        let scale = self.layout.scale.get().max(0.001);

        // Apply camera offset so particles render in screen space.
        let (cam_x, cam_y) = self.active_camera
            .as_ref()
            .map(|c| c.position)
            .unwrap_or((0.0, 0.0));

        for ps in &self.last_particle_states {
            let (r, g, b, a) = ps.color;
            let s = ps.size * scale;
            self.particle_images.push(Image {
                shape: ShapeType::RoundedRectangle(ps.rotation, (s, s), 0.0, s * 0.5),
                image: Arc::clone(&white_pixel),
                color: Some(Color(r, g, b, a)),
            });
            self.layout.particle_offsets.push((
                ps.position.0 - cam_x,
                ps.position.1 - cam_y,
            ));
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
            pivot: obj.pivot,
        }
    }).collect()
}

/// Write physics step results back into game objects.
pub(crate) fn apply_physics_result(canvas: &mut Canvas, result: PhysicsStepResult) {
    for update in result.body_updates {
        let (size, has_slope, pivot) = if let Some(obj) = canvas.store.objects.get_mut(update.id) {
            obj.position = update.position;
            obj.momentum = update.momentum;
            obj.rotation = update.rotation;
            obj.rotation_momentum = update.rotation_momentum;
            obj.grounded = update.grounded;

            // ── Slope alignment ──────────────────────────────────
            // When an object has align_to_slope enabled and is grounded on
            // a surface, smoothly rotate it to match the slope's angle.
            // Skip objects that use planet auto_align (planet takes priority).
            if obj.align_to_slope && !obj.auto_align {
                if let Some((nx, ny)) = update.grounded_surface_normal {
                    // Target angle: slope normal points "up" from the
                    // surface, so the object's visual "up" should match.
                    // atan2(nx, -ny) gives degrees where flat = 0° and
                    // matches slope_auto_rotation(right-left, width) sign.
                    let target = nx.atan2(-ny).to_degrees();
                    let diff = shortest_angle(obj.rotation, target);
                    let speed = obj.align_to_slope_speed;
                    let step = diff.signum() * speed.min(diff.abs());
                    obj.rotation += step;
                } else if !obj.grounded {
                    // Airborne — ease back toward 0°.
                    let diff = shortest_angle(obj.rotation, 0.0);
                    let speed = obj.align_to_slope_speed * 0.5;
                    let step = diff.signum() * speed.min(diff.abs());
                    obj.rotation += step;
                }
            }

                // Sync the drawable's embedded rotation to the post-crystalline
                // obj.rotation so that rotation_adjusted_offset (which uses the
                // same rotation) and the drawn shape always agree.  Without this,
                // update_image_shape() was called in update_objects with the
                // pre-crystalline rotation, causing a one-step lag between the
                // offset compensation and the actual rendered shape rotation,
                // which produced continuous visual-center drift on rotated objects.
                if obj.animated_sprite.is_none() {
                    obj.update_image_shape();
                }

                (obj.size, obj.slope.is_some(), obj.pivot)
        } else {
            continue;
        };
        if let Some(offset) = canvas.layout.offsets.get_mut(update.id) {
            *offset = super::physics::rotation_adjusted_offset(
                update.position, size, update.rotation, has_slope, pivot,
            );
        }
    }
}

/// Shortest signed angular difference (degrees), result in [-180, 180].
fn shortest_angle(from: f32, to: f32) -> f32 {
    let d = (to - from).rem_euclid(360.0);
    if d > 180.0 { d - 360.0 } else { d }
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
