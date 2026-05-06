use super::super::broadphase::AabbPairFinder;
use super::types::*;
use super::super::types::PhysicsConfig;
use rand::Rng;

/// Output for the host to render.
#[derive(Clone)]
pub struct ParticleState {
    pub position: (f32, f32),
    pub size: f32,
    pub color: (u8, u8, u8, u8),
    pub rotation: f32,
    pub render_layer: i32,
    pub shape: super::types::ParticleShape,
}

pub struct ParticleStepResult {
    pub particles: Vec<ParticleState>,
}

#[derive(Clone)]
pub struct ParticleSystem {
    particles:            Vec<Particle>,
    emitters:             Vec<Emitter>,
    max_particles:        usize,
    config:               PhysicsConfig,
    /// Fractional accumulator per emitter so sub-frame spawn rates work.
    emit_accum:           Vec<f32>,
    /// Previous-frame origin per emitter — used for sub-frame position
    /// interpolation when `emitter.interpolate_position` is true.
    emitter_prev_origins: Vec<(f32, f32)>,
}

impl ParticleSystem {
    pub fn new(max_particles: usize) -> Self {
        Self {
            particles:            Vec::with_capacity(max_particles),
            emitters:             Vec::new(),
            max_particles,
            config:               PhysicsConfig::default(),
            emit_accum:           Vec::new(),
            emitter_prev_origins: Vec::new(),
        }
    }

    pub fn with_config(mut self, config: PhysicsConfig) -> Self {
        self.config = config;
        self
    }

    pub fn set_config(&mut self, config: PhysicsConfig) {
        self.config = config;
    }

    pub fn add_emitter(&mut self, emitter: Emitter) {
        let origin = emitter.origin;
        self.emitters.push(emitter);
        self.emit_accum.push(0.0);
        self.emitter_prev_origins.push(origin);
    }

    pub fn remove_emitter(&mut self, name: &str) {
        if let Some(idx) = self.emitters.iter().position(|e| e.name == name) {
            self.emitters.remove(idx);
            if idx < self.emit_accum.len() {
                self.emit_accum.remove(idx);
            }
            if idx < self.emitter_prev_origins.len() {
                self.emitter_prev_origins.remove(idx);
            }
        }
    }

    pub fn has_emitter(&self, name: &str) -> bool {
        self.emitters.iter().any(|e| e.name == name)
    }

    /// Update an emitter's origin (called by host each frame for attached emitters).
    pub fn set_emitter_origin(&mut self, name: &str, origin: (f32, f32)) {
        if let Some(e) = self.emitters.iter_mut().find(|e| e.name == name) {
            e.origin = origin;
        }
    }

    /// Update an emitter's rotation in degrees.
    pub fn set_emitter_rotation(&mut self, name: &str, rotation: f32) {
        if let Some(e) = self.emitters.iter_mut().find(|e| e.name == name) {
            e.rotation = rotation;
        }
    }

    pub fn set_emitter_rate(&mut self, name: &str, rate: f32) {
        if let Some(e) = self.emitters.iter_mut().find(|e| e.name == name) {
            e.rate = rate;
        }
    }

    pub fn set_emitter_lifetime(&mut self, name: &str, lifetime: f32) {
        if let Some(e) = self.emitters.iter_mut().find(|e| e.name == name) {
            e.lifetime = lifetime;
        }
    }

    pub fn set_emitter_velocity(&mut self, name: &str, velocity: (f32, f32)) {
        if let Some(e) = self.emitters.iter_mut().find(|e| e.name == name) {
            e.velocity_base = velocity;
        }
    }

    pub fn set_emitter_spread(&mut self, name: &str, spread: (f32, f32)) {
        if let Some(e) = self.emitters.iter_mut().find(|e| e.name == name) {
            e.velocity_spread = spread;
        }
    }

    pub fn set_emitter_size(&mut self, name: &str, size: f32) {
        if let Some(e) = self.emitters.iter_mut().find(|e| e.name == name) {
            e.size = size;
        }
    }

    pub fn set_emitter_color(&mut self, name: &str, color: (u8, u8, u8, u8)) {
        if let Some(e) = self.emitters.iter_mut().find(|e| e.name == name) {
            e.color = color;
        }
    }

    pub fn set_emitter_gravity_scale(&mut self, name: &str, gravity_scale: f32) {
        if let Some(e) = self.emitters.iter_mut().find(|e| e.name == name) {
            e.gravity_scale = gravity_scale;
        }
    }

    pub fn set_emitter_collision(&mut self, name: &str, response: CollisionResponse) {
        if let Some(e) = self.emitters.iter_mut().find(|e| e.name == name) {
            e.collision_response = response;
        }
    }

    pub fn set_emitter_render_layer(&mut self, name: &str, layer: i32) {
        if let Some(e) = self.emitters.iter_mut().find(|e| e.name == name) {
            e.render_layer = layer;
        }
    }

    pub fn set_emitter_size_end(&mut self, name: &str, size_end: f32) {
        if let Some(e) = self.emitters.iter_mut().find(|e| e.name == name) {
            e.size_end = size_end;
        }
    }

    pub fn set_emitter_color_end(&mut self, name: &str, color_end: Option<(u8, u8, u8, u8)>) {
        if let Some(e) = self.emitters.iter_mut().find(|e| e.name == name) {
            e.color_end = color_end;
        }
    }

    pub fn set_emitter_shape(&mut self, name: &str, shape: super::types::ParticleShape) {
        if let Some(e) = self.emitters.iter_mut().find(|e| e.name == name) {
            e.shape = shape;
        }
    }

    pub fn set_emitter_align_to_velocity(&mut self, name: &str, value: bool) {
        if let Some(e) = self.emitters.iter_mut().find(|e| e.name == name) {
            e.align_to_velocity = value;
        }
    }

    pub fn set_emitter_interpolate_position(&mut self, name: &str, value: bool) {
        if let Some(e) = self.emitters.iter_mut().find(|e| e.name == name) {
            e.interpolate_position = value;
        }
    }

    pub fn spawn_burst(&mut self, position: (f32, f32), count: usize, template: Particle) {
        let remaining = self.max_particles.saturating_sub(self.particles.len());
        for _ in 0..count.min(remaining) {
            let mut p = template.clone();
            p.position = position;
            self.particles.push(p);
        }
    }

    /// Spawn a one-shot burst of particles using an emitter's spread parameters.
    /// Each particle gets randomized velocity within the emitter's spread range.
    pub fn spawn_burst_from_emitter(&mut self, emitter: &super::types::Emitter, count: usize) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let remaining = self.max_particles.saturating_sub(self.particles.len());
        for _ in 0..count.min(remaining) {
            let vx = emitter.velocity_base.0
                + (rng.r#gen::<f32>() - 0.5) * emitter.velocity_spread.0;
            let vy = emitter.velocity_base.1
                + (rng.r#gen::<f32>() - 0.5) * emitter.velocity_spread.1;
            self.particles.push(super::types::Particle {
                position: emitter.origin,
                velocity: (vx, vy),
                life: emitter.lifetime,
                max_life: emitter.lifetime,
                size: emitter.size,
                size_end: emitter.size_end,
                color: emitter.color,
                color_end: emitter.color_end,
                gravity_scale: emitter.gravity_scale,
                rotation: 0.0,
                align_to_velocity: emitter.align_to_velocity,
                collision_response: emitter.collision_response.clone(),
                render_layer: emitter.render_layer,
                shape: emitter.shape.clone(),
            });
        }
    }

    /// Main particle step. Broadphase is optional (pass None to skip collision).
    pub fn step(
        &mut self,
        dt: f32,
        broadphase: Option<&AabbPairFinder>,
    ) -> ParticleStepResult {
        let mut rng = rand::thread_rng();

        // Keep prev_origins in sync with emitters vec length.
        self.emit_accum.resize(self.emitters.len(), 0.0);
        self.emitter_prev_origins.resize(
            self.emitters.len(),
            (0.0, 0.0),
        );

        // Emit new particles from each emitter (fractional accumulator).
        for (ei, emitter) in self.emitters.iter().enumerate() {
            self.emit_accum[ei] += emitter.rate * dt;
            let to_spawn = self.emit_accum[ei] as usize;
            self.emit_accum[ei] -= to_spawn as f32;

            if to_spawn == 0 {
                // Still update prev_origin so interpolation is ready next frame.
                self.emitter_prev_origins[ei] = emitter.origin;
                continue;
            }

            let prev = self.emitter_prev_origins[ei];
            let curr = emitter.origin;

            // Pre-compute rotation transform for this emitter.
            let rad     = emitter.rotation.to_radians();
            let cos_r   = rad.cos();
            let sin_r   = rad.sin();

            for i in 0..to_spawn {
                if self.particles.len() >= self.max_particles {
                    break;
                }

                // Sub-frame position interpolation: distribute spawn positions
                // evenly along the path travelled this frame so gaps are filled
                // even at high speeds.  When interpolation is off every particle
                // spawns at the current origin (original behaviour).
                let spawn_pos = if emitter.interpolate_position {
                    let t = (i as f32 + 0.5) / to_spawn as f32;
                    (
                        prev.0 + (curr.0 - prev.0) * t,
                        prev.1 + (curr.1 - prev.1) * t,
                    )
                } else {
                    curr
                };

                let raw_vx = emitter.velocity_base.0
                    + (rng.r#gen::<f32>() - 0.5) * emitter.velocity_spread.0;
                let raw_vy = emitter.velocity_base.1
                    + (rng.r#gen::<f32>() - 0.5) * emitter.velocity_spread.1;
                // Rotate velocity by emitter rotation.
                let vx = raw_vx * cos_r - raw_vy * sin_r;
                let vy = raw_vx * sin_r + raw_vy * cos_r;

                self.particles.push(Particle {
                    position: spawn_pos,
                    velocity: (vx, vy),
                    life: emitter.lifetime,
                    max_life: emitter.lifetime,
                    size: emitter.size,
                    size_end: emitter.size_end,
                    color: emitter.color,
                    color_end: emitter.color_end,
                    gravity_scale: emitter.gravity_scale,
                    rotation: 0.0,
                    align_to_velocity: emitter.align_to_velocity,
                    collision_response: emitter.collision_response.clone(),
                    render_layer: emitter.render_layer,
                    shape: emitter.shape.clone(),
                });
            }

            self.emitter_prev_origins[ei] = curr;
        }

        // Simulate
        // Convert particle_gravity from Quartz-scale units to px/s²
        // Quartz applies gravity per frame at ~60fps, so multiply by 60
        let grav_accel = self.config.particle_gravity * 60.0;
        for p in &mut self.particles {
            p.velocity.1 += grav_accel * p.gravity_scale * dt;
            p.position.0 += p.velocity.0 * dt;
            p.position.1 += p.velocity.1 * dt;
            p.life -= dt;

            // Speed clamp — prevents tunnelling on high-gravity edge cases
            let speed_sq = p.velocity.0 * p.velocity.0 + p.velocity.1 * p.velocity.1;
            let max_sq   = self.config.particle_max_speed * self.config.particle_max_speed;
            if speed_sq > max_sq {
                let scale = self.config.particle_max_speed / speed_sq.sqrt().max(0.001);
                p.velocity.0 *= scale;
                p.velocity.1 *= scale;
            }

            // Optional world collision — per-particle response
            if let Some(bp) = broadphase {
                if bp.query_point(p.position.0, p.position.1).is_some() {
                    match &p.collision_response {
                        CollisionResponse::None => {}
                        CollisionResponse::Bounce { elasticity } => {
                            p.velocity.0 *= -elasticity;
                            p.velocity.1 *= -elasticity;
                        }
                        CollisionResponse::Die => {
                            p.life = 0.0;
                        }
                    }
                }
            }
        }

        // Cull dead particles
        self.particles.retain(|p| p.life > 0.0);

        // Build render output — apply per-particle lerps and velocity alignment.
        ParticleStepResult {
            particles: self
                .particles
                .iter()
                .map(|p| {
                    // Normalised age 0.0 (just spawned) → 1.0 (about to die).
                    let age = if p.max_life > 0.0 {
                        1.0 - (p.life / p.max_life).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };

                    // Size lerp: -1.0 means no lerp (keep spawn size).
                    let rendered_size = if p.size_end >= 0.0 {
                        p.size * (1.0 - age) + p.size_end * age
                    } else {
                        p.size
                    };

                    // Colour lerp.
                    let rendered_color = if let Some(end) = p.color_end {
                        fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
                            (a as f32 + (b as f32 - a as f32) * t).round().clamp(0.0, 255.0) as u8
                        }
                        (
                            lerp_u8(p.color.0, end.0, age),
                            lerp_u8(p.color.1, end.1, age),
                            lerp_u8(p.color.2, end.2, age),
                            lerp_u8(p.color.3, end.3, age),
                        )
                    } else {
                        p.color
                    };

                    // Rotation: velocity-aligned or fixed.
                    let rendered_rotation = if p.align_to_velocity {
                        let speed_sq = p.velocity.0 * p.velocity.0
                            + p.velocity.1 * p.velocity.1;
                        if speed_sq > 0.01 {
                            p.velocity.1.atan2(p.velocity.0).to_degrees()
                        } else {
                            p.rotation
                        }
                    } else {
                        p.rotation
                    };

                    ParticleState {
                        position: p.position,
                        size: rendered_size,
                        color: rendered_color,
                        rotation: rendered_rotation,
                        render_layer: p.render_layer,
                        shape: p.shape.clone(),
                    }
                })
                .collect(),
        }
    }
}
