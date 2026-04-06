use super::broadphase::AabbPairFinder;
use super::contacts::*;
use super::types::*;

/// Threshold for considering a body "at rest" (sleeping candidate).
const SLEEP_VELOCITY_THRESHOLD: f32 = 0.1;
/// Frames a body must be below threshold before sleeping.
const SLEEP_FRAMES_REQUIRED: u16 = 60;

/// Per-body sleeping state, tracked across frames.
#[derive(Clone, Debug)]
pub struct SleepState {
    pub sleeping: bool,
    pub frames_at_rest: u16,
}

impl Default for SleepState {
    fn default() -> Self {
        Self {
            sleeping: false,
            frames_at_rest: 0,
        }
    }
}

#[derive(Clone)]
pub struct CrystallinePhysics {
    pub config: PhysicsConfig,
    accumulator: f32,
    broadphase: AabbPairFinder,
    /// Sleeping state per body ID. Grows as needed.
    sleep_states: Vec<SleepState>,
    /// Pending one-shot forces/impulses to apply next step.
    pending_forces: Vec<(usize, f32, f32)>,
    pending_impulses: Vec<(usize, f32, f32)>,
}

impl CrystallinePhysics {
    pub fn new() -> Self {
        Self::with_config(PhysicsConfig::default())
    }

    pub fn with_config(config: PhysicsConfig) -> Self {
        Self {
            config,
            accumulator: 0.0,
            broadphase: AabbPairFinder::new(),
            sleep_states: Vec::new(),
            pending_forces: Vec::new(),
            pending_impulses: Vec::new(),
        }
    }

    /// Queue a continuous force (applied over the timestep, scaled by dt).
    pub fn apply_force(&mut self, body_id: usize, fx: f32, fy: f32) {
        self.pending_forces.push((body_id, fx, fy));
        self.wake(body_id);
    }

    /// Queue an instantaneous impulse (added directly to momentum).
    pub fn apply_impulse(&mut self, body_id: usize, ix: f32, iy: f32) {
        self.pending_impulses.push((body_id, ix, iy));
        self.wake(body_id);
    }

    /// Wake a sleeping body (e.g. when a force or collision hits it).
    pub fn wake(&mut self, body_id: usize) {
        if let Some(state) = self.sleep_states.get_mut(body_id) {
            state.sleeping = false;
            state.frames_at_rest = 0;
        }
    }

    /// Check if a body is currently sleeping.
    pub fn is_sleeping(&self, body_id: usize) -> bool {
        self.sleep_states
            .get(body_id)
            .map_or(false, |s| s.sleeping)
    }

    /// Main entry point. Called once per frame with raw delta time.
    pub fn step(&mut self, delta: f32, bodies: &[PhysicsBody]) -> PhysicsStepResult {
        self.accumulator += delta;
        let fixed = self.config.fixed_dt;
        let mut steps = 0u32;

        // Working copy
        let mut working: Vec<PhysicsBody> = bodies.to_vec();
        let mut all_collisions: Vec<(usize, usize)> = Vec::new();
        let mut grounded: Vec<bool> = vec![false; working.len()];

        // Ensure sleep_states covers all bodies
        if self.sleep_states.len() < working.len() {
            self.sleep_states
                .resize(working.len(), SleepState::default());
        }

        // Apply pending impulses (instant, before integration)
        for (id, ix, iy) in self.pending_impulses.drain(..) {
            if let Some(body) = working.iter_mut().find(|b| b.id == id) {
                body.momentum.0 += ix;
                body.momentum.1 += iy;
            }
        }

        while self.accumulator >= fixed && steps < self.config.max_substeps {
            self.substep(&mut working, fixed, &mut all_collisions, &mut grounded);
            self.accumulator -= fixed;
            steps += 1;
        }

        // Drain forces (they were applied each substep)
        self.pending_forces.clear();

        // Update sleep states
        for (idx, body) in working.iter().enumerate() {
            if body.is_platform || !body.visible {
                continue;
            }
            if let Some(state) = self.sleep_states.get_mut(idx) {
                let speed =
                    body.momentum.0 * body.momentum.0 + body.momentum.1 * body.momentum.1;
                if speed < SLEEP_VELOCITY_THRESHOLD * SLEEP_VELOCITY_THRESHOLD
                    && body.rotation_momentum.abs() < SLEEP_VELOCITY_THRESHOLD
                {
                    state.frames_at_rest = state.frames_at_rest.saturating_add(1);
                    if state.frames_at_rest >= SLEEP_FRAMES_REQUIRED {
                        state.sleeping = true;
                    }
                } else {
                    state.frames_at_rest = 0;
                    state.sleeping = false;
                }
            }
        }

        // Build result
        let body_updates = working
            .iter()
            .enumerate()
            .map(|(idx, body)| BodyUpdate {
                id: body.id,
                position: body.position,
                momentum: body.momentum,
                rotation: body.rotation,
                rotation_momentum: body.rotation_momentum,
                grounded: grounded.get(idx).copied().unwrap_or(false),
            })
            .collect();

        all_collisions.sort();
        all_collisions.dedup();

        PhysicsStepResult {
            body_updates,
            collision_pairs: all_collisions,
        }
    }

    fn substep(
        &mut self,
        bodies: &mut [PhysicsBody],
        dt: f32,
        collisions: &mut Vec<(usize, usize)>,
        grounded: &mut [bool],
    ) {
        // Grounded is accumulated across substeps — reset happens once
        // in step(), not per substep. Any contact within any substep
        // that detects a floor normal keeps grounded = true.

        // Normalize to 60 fps reference frame so behaviour is substep-count invariant
        let frame_scale = dt * 60.0;

        // 1. Integrate: apply gravity, momentum, resistance, pending forces
        for (idx, body) in bodies.iter_mut().enumerate() {
            if !body.visible || body.is_platform {
                continue;
            }
            // Skip sleeping bodies
            if self
                .sleep_states
                .get(idx)
                .map_or(false, |s| s.sleeping)
            {
                continue;
            }

            // Apply continuous forces (F × dt added to momentum)
            for &(fid, fx, fy) in &self.pending_forces {
                if fid == body.id {
                    body.momentum.0 += fx * dt;
                    body.momentum.1 += fy * dt;
                }
            }

            body.momentum.1 += body.gravity * self.config.gravity_scale * frame_scale;
            body.position.0 += body.momentum.0 * frame_scale;
            body.position.1 += body.momentum.1 * frame_scale;
            body.momentum.0 *= body.resistance.0.powf(frame_scale);
            body.momentum.1 *= body.resistance.1.powf(frame_scale);
            if body.momentum.0.abs() < 0.001 {
                body.momentum.0 = 0.0;
            }
            if body.momentum.1.abs() < 0.001 {
                body.momentum.1 = 0.0;
            }

            // Rotation
            body.rotation += body.rotation_momentum * frame_scale;
            body.rotation_momentum *= body.rotation_resistance.powf(frame_scale);
            if body.rotation_momentum.abs() < 0.001 {
                body.rotation_momentum = 0.0;
            }
        }

        // 2. Broadphase — speculative expansion by velocity × dt
        self.broadphase.rebuild_speculative(bodies, dt);
        let pairs = self.broadphase.query_pairs();

        // 3. Collect contact manifolds (narrowphase)
        let (contacts, dynamic_contacts) = self.collect_contacts(bodies, &pairs, collisions);

        // Wake any sleeping body involved in a dynamic-dynamic collision
        for dc in &dynamic_contacts {
            if let Some(s) = self.sleep_states.get_mut(dc.idx_a) {
                if s.sleeping {
                    s.sleeping = false;
                    s.frames_at_rest = 0;
                }
            }
            if let Some(s) = self.sleep_states.get_mut(dc.idx_b) {
                if s.sleeping {
                    s.sleeping = false;
                    s.frames_at_rest = 0;
                }
            }
        }

        // 4. XPBD-style iterative position correction
        //    Multiple passes reduce jitter and produce stable stacking.
        let iterations = self.config.position_iterations.max(1);
        let correction_scale = 1.0 / iterations as f32;
        for iter in 0..iterations {
            let is_last = iter == iterations - 1;
            for contact in &contacts {
                solve_contact(bodies, grounded, contact, &self.sleep_states, correction_scale, is_last);
            }
            for dc in &dynamic_contacts {
                solve_dynamic_contact(bodies, dc, correction_scale, is_last);
            }
        }
    }

    /// Narrowphase: build contact list from broadphase pairs.
    fn collect_contacts(
        &self,
        bodies: &[PhysicsBody],
        pairs: &[(usize, usize)],
        collisions: &mut Vec<(usize, usize)>,
    ) -> (Vec<Contact>, Vec<DynamicContact>) {
        let mut contacts = Vec::new();
        let mut dynamic_contacts = Vec::new();

        for &(id_a, id_b) in pairs {
            let idx_a = match bodies.iter().position(|b| b.id == id_a) {
                Some(i) => i,
                None => continue,
            };
            let idx_b = match bodies.iter().position(|b| b.id == id_b) {
                Some(i) => i,
                None => continue,
            };

            let a_is_platform = bodies[idx_a].is_platform;
            let b_is_platform = bodies[idx_b].is_platform;

            // Two non-platforms: resolve dynamic-dynamic collision
            if !a_is_platform && !b_is_platform {
                collisions.push((id_a, id_b));

                // Collision layer filter: both must have non-zero overlapping bits
                let layer_a = bodies[idx_a].collision_layer;
                let layer_b = bodies[idx_b].collision_layer;
                if layer_a == 0 || layer_b == 0 || (layer_a & layer_b) == 0 {
                    continue;
                }

                // AABB overlap test for two dynamic bodies
                let a = &bodies[idx_a];
                let b = &bodies[idx_b];
                let a_cx = a.position.0 + a.size.0 * 0.5;
                let a_cy = a.position.1 + a.size.1 * 0.5;
                let b_cx = b.position.0 + b.size.0 * 0.5;
                let b_cy = b.position.1 + b.size.1 * 0.5;

                let overlap_x = (a.size.0 + b.size.0) * 0.5 - (a_cx - b_cx).abs();
                let overlap_y = (a.size.1 + b.size.1) * 0.5 - (a_cy - b_cy).abs();

                if overlap_x > 0.0 && overlap_y > 0.0 {
                    // Minimum penetration axis
                    let (dx, dy, nx, ny) = if overlap_x < overlap_y {
                        let sign = if a_cx < b_cx { -1.0 } else { 1.0 };
                        (sign * overlap_x, 0.0, sign, 0.0)
                    } else {
                        let sign = if a_cy < b_cy { -1.0 } else { 1.0 };
                        (0.0, sign * overlap_y, 0.0, sign)
                    };

                    // Check approach: only resolve if bodies are moving towards each other
                    let rel_vx = a.momentum.0 - b.momentum.0;
                    let rel_vy = a.momentum.1 - b.momentum.1;
                    let approach = rel_vx * (-nx) + rel_vy * (-ny);
                    if approach > -0.01 {
                        dynamic_contacts.push(DynamicContact {
                            idx_a, idx_b, dx, dy, nx, ny,
                        });
                    }
                }
                continue;
            }

            let (obj_idx, plat_idx) = if b_is_platform && !a_is_platform {
                (idx_a, idx_b)
            } else if a_is_platform && !b_is_platform {
                (idx_b, idx_a)
            } else {
                continue;
            };

            collisions.push((id_a, id_b));

            let obj = &bodies[obj_idx];
            let plat = &bodies[plat_idx];
            let obj_center_x = obj.position.0 + obj.size.0 * 0.5;

            match &plat.collision_mode {
                CrystallineCollisionMode::NonPlatform => continue,

                CrystallineCollisionMode::Solid(shape) => {
                    let result = match shape {
                        CollisionShape::Rectangle => resolve_solid_collision(obj, plat)
                            .map(|(dx, dy, _face)| (dx, dy)),
                        CollisionShape::Circle { radius } => {
                            resolve_circle_collision(obj, plat, *radius)
                        }
                    };
                    if let Some((dx, dy)) = result {
                        let dist = (dx * dx + dy * dy).sqrt().max(0.001);
                        let nx = dx / dist;
                        let ny = dy / dist;
                        let approach = obj.momentum.0 * (-nx) + obj.momentum.1 * (-ny);
                        if approach > 0.0 {
                            contacts.push(Contact {
                                obj_idx,
                                plat_idx,
                                dx,
                                dy,
                                nx,
                                ny,
                            });
                        }
                    }
                    continue;
                }

                CrystallineCollisionMode::Surface => {
                    // Fall through to surface resolution
                }
            }

            // ── Surface mode resolution ──────────────────────────

            let (mut nx, mut ny) = surface_normal_at(plat, obj_center_x);

            if plat.rotation != 0.0 && plat.slope.is_none() && ny > 0.0 {
                nx = -nx;
                ny = -ny;
            }

            let approach_speed = obj.momentum.0 * (-nx) + obj.momentum.1 * (-ny);
            if approach_speed <= 0.0 {
                continue;
            }

            // One-way platform logic
            if plat.one_way {
                if plat.slope.is_some() {
                    let prev_bottom = (obj.position.1 + obj.size.1) - obj.momentum.1;
                    let prev_cx = obj_center_x - obj.momentum.0;
                    let surface_at_prev = slope_surface_y(plat, prev_cx);
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
                    if !was_outside {
                        continue;
                    }
                }
            }

            // Compute correction delta
            let (cdx, cdy) = if plat.slope.is_some() {
                let surface_y = slope_surface_y(plat, obj_center_x);
                if obj.position.1 + obj.size.1 <= surface_y {
                    continue;
                }
                const SLOPE_TOLERANCE: f32 = 20.0;
                let prev_bottom = (obj.position.1 + obj.size.1) - obj.momentum.1;
                let prev_cx = obj_center_x - obj.momentum.0;
                let surface_prev = slope_surface_y(plat, prev_cx);
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
                if depth <= 0.0 {
                    continue;
                }
                (nx * depth, ny * depth)
            };

            contacts.push(Contact {
                obj_idx,
                plat_idx,
                dx: cdx,
                dy: cdy,
                nx,
                ny,
            });
        }

        (contacts, dynamic_contacts)
    }

    pub fn set_config(&mut self, config: PhysicsConfig) {
        self.config = config;
    }

    pub fn set_quality(&mut self, preset: PhysicsQuality) {
        match preset {
            PhysicsQuality::Low => {
                self.config.max_substeps        = 3;
                self.config.position_iterations = 3;
            }
            PhysicsQuality::Medium => {
                self.config.max_substeps        = 8;
                self.config.position_iterations = 6;
            }
            PhysicsQuality::High => {
                self.config.max_substeps        = 12;
                self.config.position_iterations = 10;
            }
        }
    }
}

