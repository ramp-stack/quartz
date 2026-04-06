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
}

pub struct ParticleStepResult {
    pub particles: Vec<ParticleState>,
}

#[derive(Clone)]
pub struct ParticleSystem {
    particles:     Vec<Particle>,
    emitters:      Vec<Emitter>,
    max_particles: usize,
    config:        PhysicsConfig,
    /// Fractional accumulator per emitter so sub-frame spawn rates work.
    emit_accum:    Vec<f32>,
}

impl ParticleSystem {
    pub fn new(max_particles: usize) -> Self {
        Self {
            particles:     Vec::with_capacity(max_particles),
            emitters:      Vec::new(),
            max_particles,
            config:        PhysicsConfig::default(),
            emit_accum:    Vec::new(),
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
        self.emitters.push(emitter);
        self.emit_accum.push(0.0);
    }

    pub fn remove_emitter(&mut self, name: &str) {
        if let Some(idx) = self.emitters.iter().position(|e| e.name == name) {
            self.emitters.remove(idx);
            if idx < self.emit_accum.len() {
                self.emit_accum.remove(idx);
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

    pub fn spawn_burst(&mut self, position: (f32, f32), count: usize, template: Particle) {
        let remaining = self.max_particles.saturating_sub(self.particles.len());
        for _ in 0..count.min(remaining) {
            let mut p = template.clone();
            p.position = position;
            self.particles.push(p);
        }
    }

    /// Main particle step. Broadphase is optional (pass None to skip collision).
    pub fn step(
        &mut self,
        dt: f32,
        broadphase: Option<&AabbPairFinder>,
    ) -> ParticleStepResult {
        let mut rng = rand::thread_rng();

        // Emit new particles from each emitter (fractional accumulator)
        // Ensure accum vec matches emitter count
        self.emit_accum.resize(self.emitters.len(), 0.0);
        for (ei, emitter) in self.emitters.iter().enumerate() {
            self.emit_accum[ei] += emitter.rate * dt;
            let to_spawn = self.emit_accum[ei] as usize;
            self.emit_accum[ei] -= to_spawn as f32;
            for _ in 0..to_spawn {
                if self.particles.len() >= self.max_particles {
                    break;
                }
                let vx = emitter.velocity_base.0
                    + (rng.r#gen::<f32>() - 0.5) * emitter.velocity_spread.0;
                let vy = emitter.velocity_base.1
                    + (rng.r#gen::<f32>() - 0.5) * emitter.velocity_spread.1;
                self.particles.push(Particle {
                    position: emitter.origin,
                    velocity: (vx, vy),
                    life: emitter.lifetime,
                    max_life: emitter.lifetime,
                    size: emitter.size,
                    color: emitter.color,
                    gravity_scale: emitter.gravity_scale,
                    rotation: 0.0,
                    collision_response: emitter.collision_response.clone(),
                });
            }
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

        // Build output
        ParticleStepResult {
            particles: self
                .particles
                .iter()
                .map(|p| ParticleState {
                    position: p.position,
                    size: p.size,
                    color: p.color,
                    rotation: p.rotation,
                })
                .collect(),
        }
    }
}
