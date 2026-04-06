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

    // -- Crystalline tick step --------------------------------------------

    /// Full crystalline physics step: build body snapshot → run solver →
    /// apply results → particles → visuals → legacy collision events.
    /// Called from the tick loop when crystalline is enabled.
    pub(crate) fn run_crystalline_step(&mut self, delta_time: f32) {
        // Sync emitter origins to attached objects.
        sync_emitter_origins(self);

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

        if self.last_particle_states.is_empty() {
            return;
        }

        use std::sync::Arc;
        use image::{RgbaImage, Rgba};
        let white_pixel: Arc<RgbaImage> = Arc::new(
            RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255])),
        );

        for ps in &self.last_particle_states {
            let (r, g, b, a) = ps.color;
            let s = ps.size;
            self.particle_images.push(Image {
                shape: ShapeType::RoundedRectangle(ps.rotation, (s, s), 0.0, s * 0.5),
                image: Arc::clone(&white_pixel),
                color: Some(Color(r, g, b, a)),
            });
            self.layout.particle_offsets.push(ps.position);
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
        let origin = canvas
            .store
            .name_to_index
            .get(obj_name.as_str())
            .and_then(|&idx| canvas.store.objects.get(idx))
            .map(|obj| {
                (
                    obj.position.0 + obj.size.0 * 0.5,
                    obj.position.1 + obj.size.1 * 0.5,
                )
            });
        if let (Some(ps), Some(origin)) = (&mut canvas.particle_system, origin) {
            ps.set_emitter_origin(&emitter_name, origin);
        }
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
