use super::types::*;
use super::solver::SleepState;

// ── Contact structs ─────────────────────────────────────────────────────

/// A narrowphase contact: obj penetrating plat, with correction and normal.
pub(super) struct Contact {
    pub(super) obj_idx: usize,
    pub(super) plat_idx: usize,
    pub(super) dx: f32,
    pub(super) dy: f32,
    pub(super) nx: f32,
    pub(super) ny: f32,
}

/// A narrowphase contact between two dynamic (non-platform) bodies.
pub(super) struct DynamicContact {
    pub(super) idx_a: usize,
    pub(super) idx_b: usize,
    /// Separation vector: push A by +dx,+dy and B by -dx,-dy (scaled by mass).
    pub(super) dx: f32,
    pub(super) dy: f32,
    /// Contact normal from A towards B.
    pub(super) nx: f32,
    pub(super) ny: f32,
}

// ── Contact solvers ─────────────────────────────────────────────────────

/// Apply one contact's position correction + velocity response.
/// Position correction is scaled by `correction_scale` (1/iterations) so
/// the total correction across all iterations sums to the full delta.
/// Velocity response (bounce, friction, conveyor) is applied only on the
/// final iteration (`apply_velocity = true`) to prevent compounding.
pub(super) fn solve_contact(
    bodies: &mut [PhysicsBody],
    grounded: &mut [bool],
    grounded_normals: &mut [Option<(f32, f32)>],
    contact: &Contact,
    sleep_states: &[SleepState],
    correction_scale: f32,
    apply_velocity: bool,
) {
    let plat = &bodies[contact.plat_idx];

    let (nx, ny) = match &plat.collision_mode {
        CrystallineCollisionMode::Surface => {
            let (mut nx, mut ny) = plat.surface_normal;
            if plat.rotation != 0.0 && plat.slope.is_none() && ny > 0.0 {
                nx = -nx;
                ny = -ny;
            }
            (nx, ny)
        }
        _ => (contact.nx, contact.ny),
    };

    let surf_vel = plat.surface_velocity;

    // Read material from both bodies for combined response
    let plat_mat = plat.material;
    let obj_mat = bodies[contact.obj_idx].material;

    // Combined elasticity: geometric mean, clamped to [0, 1] to conserve energy.
    // Values > 1.0 inject energy per bounce → objects escape to infinity.
    let combined_elasticity = (obj_mat.elasticity * plat_mat.elasticity).sqrt().min(1.0);

    // Combined friction: geometric mean
    let combined_friction = (obj_mat.friction * plat_mat.friction).sqrt();

    let obj = &mut bodies[contact.obj_idx];

    // ── Position correction (scaled so N iterations sum to full delta) ─
    obj.position.0 += contact.dx * correction_scale;
    obj.position.1 += contact.dy * correction_scale;

    // Grounded detection (every iteration — just a flag + surface normal)
    if ny < -0.3 {
        grounded[contact.obj_idx] = true;
        grounded_normals[contact.obj_idx] = Some((nx, ny));
    }

    // ── Velocity response (final iteration only to prevent compounding) ─
    if apply_velocity {
        // Normal speed (inward component)
        let vn = obj.momentum.0 * (-nx) + obj.momentum.1 * (-ny);
        if vn > 0.0 {
            // Remove inward component
            obj.momentum.0 += nx * vn;
            obj.momentum.1 += ny * vn;

            // Apply elasticity: bounce = normal_speed × elasticity
            if combined_elasticity > 0.001 {
                obj.momentum.0 += nx * vn * combined_elasticity;
                obj.momentum.1 += ny * vn * combined_elasticity;
            }
        }

        // Tangential friction
        if combined_friction > 0.001 {
            // Tangent direction (perpendicular to normal)
            let tx = -ny;
            let ty = nx;
            let vt = obj.momentum.0 * tx + obj.momentum.1 * ty;

            // Coulomb friction: clamp tangential impulse to µ × normal impulse
            let max_friction = combined_friction * vn.abs();
            let friction_impulse = vt.clamp(-max_friction, max_friction);
            obj.momentum.0 -= tx * friction_impulse;
            obj.momentum.1 -= ty * friction_impulse;
        }

        // Surface velocity (conveyor belts)
        if let Some(vx) = surf_vel {
            obj.momentum.0 += -ny * vx;
            obj.momentum.1 += nx * vx;
        }
    }

    // Wake sleeping body on collision
    if let Some(state) = sleep_states.get(contact.obj_idx) {
        if state.sleeping {
            // Can't mutate sleep_states here — wake will be handled by
            // post-step velocity check (collision adds momentum → not at rest)
        }
    }
}

/// Resolve a collision between two dynamic (non-platform) bodies.
/// Correction and impulse are split by inverse mass ratio (from density × area).
pub(super) fn solve_dynamic_contact(
    bodies: &mut [PhysicsBody],
    dc: &DynamicContact,
    correction_scale: f32,
    apply_velocity: bool,
) {
    // Compute inverse masses from material density × area
    let area_a = bodies[dc.idx_a].size.0 * bodies[dc.idx_a].size.1;
    let area_b = bodies[dc.idx_b].size.0 * bodies[dc.idx_b].size.1;
    let mass_a = bodies[dc.idx_a].material.density * area_a;
    let mass_b = bodies[dc.idx_b].material.density * area_b;
    let inv_a = if mass_a > 0.001 { 1.0 / mass_a } else { 0.0 };
    let inv_b = if mass_b > 0.001 { 1.0 / mass_b } else { 0.0 };
    let inv_total = inv_a + inv_b;
    if inv_total < 0.0001 { return; }

    let ratio_a = inv_a / inv_total; // lighter body moves more
    let ratio_b = inv_b / inv_total;

    // ── Position correction (scaled by 1/iterations) ───────────
    bodies[dc.idx_a].position.0 += dc.dx * ratio_a * correction_scale;
    bodies[dc.idx_a].position.1 += dc.dy * ratio_a * correction_scale;
    bodies[dc.idx_b].position.0 -= dc.dx * ratio_b * correction_scale;
    bodies[dc.idx_b].position.1 -= dc.dy * ratio_b * correction_scale;

    // ── Velocity response (final iteration only) ───────────────
    if apply_velocity {
        let nx = dc.nx;
        let ny = dc.ny;

        // Relative velocity along normal (A towards B)
        let rel_vn = (bodies[dc.idx_a].momentum.0 - bodies[dc.idx_b].momentum.0) * (-nx)
                   + (bodies[dc.idx_a].momentum.1 - bodies[dc.idx_b].momentum.1) * (-ny);

        if rel_vn > 0.0 {
            // Combined elasticity: geometric mean
            let e = (bodies[dc.idx_a].material.elasticity * bodies[dc.idx_b].material.elasticity).sqrt().min(1.0);
            let j = rel_vn * (1.0 + e) / inv_total;

            bodies[dc.idx_a].momentum.0 += nx * j * inv_a;
            bodies[dc.idx_a].momentum.1 += ny * j * inv_a;
            bodies[dc.idx_b].momentum.0 -= nx * j * inv_b;
            bodies[dc.idx_b].momentum.1 -= ny * j * inv_b;

            // Tangential friction
            let combined_friction = (bodies[dc.idx_a].material.friction * bodies[dc.idx_b].material.friction).sqrt();
            if combined_friction > 0.001 {
                let tx = -ny;
                let ty = nx;
                let rel_vt = (bodies[dc.idx_a].momentum.0 - bodies[dc.idx_b].momentum.0) * tx
                           + (bodies[dc.idx_a].momentum.1 - bodies[dc.idx_b].momentum.1) * ty;
                let max_f = combined_friction * (j * inv_total).abs();
                let fi = rel_vt.clamp(-max_f, max_f);
                bodies[dc.idx_a].momentum.0 -= tx * fi * inv_a;
                bodies[dc.idx_a].momentum.1 -= ty * fi * inv_a;
                bodies[dc.idx_b].momentum.0 += tx * fi * inv_b;
                bodies[dc.idx_b].momentum.1 += ty * fi * inv_b;
            }
        }
    }
}

// ── Collision helper functions (ported from Quartz canvas.rs / object.rs) ──

/// Surface Y for a sloped body at a given world X.
/// Mirrors `GameObject::slope_surface_y()` in object.rs.
pub(super) fn slope_surface_y(body: &PhysicsBody, world_x: f32) -> f32 {
    match body.slope {
        None => body.position.1,
        Some((left_offset, right_offset)) => {
            if body.size.0 == 0.0 {
                return body.position.1;
            }
            let t = ((world_x - body.position.0) / body.size.0).clamp(0.0, 1.0);
            body.position.1 + left_offset + (right_offset - left_offset) * t
        }
    }
}

/// Surface normal at a given world X.
/// Mirrors `GameObject::surface_normal_at()` in object.rs.
pub(super) fn surface_normal_at(body: &PhysicsBody, _world_x: f32) -> (f32, f32) {
    match body.slope {
        None => body.surface_normal,
        Some((left_offset, right_offset)) => {
            let w = body.size.0;
            if w < 0.01 {
                return (0.0, -1.0);
            }
            let rise = right_offset - left_offset;
            let len = (rise * rise + w * w).sqrt();
            (rise / len, -w / len)
        }
    }
}

/// Surface Y for a rotated (non-slope) platform at a given world X.
/// Mirrors `rotated_surface_y()` in canvas.rs.
pub(super) fn rotated_surface_y(plat: &PhysicsBody, world_x: f32) -> f32 {
    let cx = plat.position.0 + plat.size.0 * 0.5;
    let cy = plat.position.1 + plat.size.1 * 0.5;
    let theta = plat.rotation.to_radians();
    let cos_t = theta.cos();
    let sin_t = theta.sin();
    let half_w = plat.size.0 * 0.5;
    let half_h = plat.size.1 * 0.5;
    let dx = (world_x - cx).clamp(-half_w, half_w);
    let cos_abs = cos_t.abs().max(0.001);
    let cos_safe = if cos_t.abs() < 0.001 { 0.001 } else { cos_t };
    cy + dx * sin_t / cos_safe - half_h / cos_abs
}

/// SAT-based penetration depth along a given normal.
/// Mirrors `penetration_depth()` in canvas.rs.
pub(super) fn penetration_depth(obj: &PhysicsBody, plat: &PhysicsBody, nx: f32, ny: f32) -> f32 {
    let obj_cx = obj.position.0 + obj.size.0 * 0.5;
    let obj_cy = obj.position.1 + obj.size.1 * 0.5;
    let plat_cx = plat.position.0 + plat.size.0 * 0.5;
    let plat_cy = plat.position.1 + plat.size.1 * 0.5;

    let obj_half = (obj.size.0 * nx.abs() + obj.size.1 * ny.abs()) * 0.5;
    let plat_half = (plat.size.0 * nx.abs() + plat.size.1 * ny.abs()) * 0.5;

    let sep = (obj_cx - plat_cx) * nx + (obj_cy - plat_cy) * ny;
    let overlap = obj_half + plat_half - sep.abs();
    if overlap > 0.0 {
        overlap
    } else {
        0.0
    }
}

/// OBB-based solid collision resolution. Returns (dx, dy, face) where
/// face: 0=top, 1=bottom, 2=left, 3=right of the platform in local space.
/// Mirrors `resolve_solid_collision()` in canvas.rs.
pub(super) fn resolve_solid_collision(
    obj: &PhysicsBody,
    plat: &PhysicsBody,
) -> Option<(f32, f32, u8)> {
    let plat_cx = plat.position.0 + plat.size.0 * 0.5;
    let plat_cy = plat.position.1 + plat.size.1 * 0.5;
    let obj_cx = obj.position.0 + obj.size.0 * 0.5;
    let obj_cy = obj.position.1 + obj.size.1 * 0.5;

    let theta = -plat.rotation.to_radians();
    let cos_t = theta.cos();
    let sin_t = theta.sin();

    let rel_x = obj_cx - plat_cx;
    let rel_y = obj_cy - plat_cy;
    let local_x = rel_x * cos_t - rel_y * sin_t;
    let local_y = rel_x * sin_t + rel_y * cos_t;

    let half_pw = plat.size.0 * 0.5;
    let half_ph = plat.size.1 * 0.5;
    let half_ow = obj.size.0 * 0.5;
    let half_oh = obj.size.1 * 0.5;

    let overlap_x = (half_pw + half_ow) - local_x.abs();
    let overlap_y = (half_ph + half_oh) - local_y.abs();

    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return None;
    }

    let mut candidates: Vec<(f32, f32, f32, u8)> = Vec::with_capacity(4);

    if local_y < 0.0 {
        candidates.push((overlap_y, 0.0, -1.0, 0));
    }
    if local_y >= 0.0 {
        candidates.push((overlap_y, 0.0, 1.0, 1));
    }
    if local_x < 0.0 {
        candidates.push((overlap_x, -1.0, 0.0, 2));
    }
    if local_x >= 0.0 {
        candidates.push((overlap_x, 1.0, 0.0, 3));
    }

    if candidates.is_empty() {
        return None;
    }

    let &(depth, local_nx, local_ny, face) = candidates
        .iter()
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();

    // Rotate normal back to world space using the platform's forward rotation
    let theta = plat.rotation.to_radians();
    let cos_t = theta.cos();
    let sin_t = theta.sin();
    let world_nx = local_nx * cos_t - local_ny * sin_t;
    let world_ny = local_nx * sin_t + local_ny * cos_t;

    Some((world_nx * depth, world_ny * depth, face))
}

/// Circle-based solid collision resolution. Returns (dx, dy) push-out vector.
/// Mirrors `resolve_circle_collision()` in canvas.rs.
pub(super) fn resolve_circle_collision(
    obj: &PhysicsBody,
    plat: &PhysicsBody,
    radius: f32,
) -> Option<(f32, f32)> {
    let r = if radius <= 0.0 {
        plat.size.0.min(plat.size.1) * 0.5
    } else {
        radius
    };

    let plat_cx = plat.position.0 + plat.size.0 * 0.5;
    let plat_cy = plat.position.1 + plat.size.1 * 0.5;
    let obj_cx = obj.position.0 + obj.size.0 * 0.5;
    let obj_cy = obj.position.1 + obj.size.1 * 0.5;

    let dx = obj_cx - plat_cx;
    let dy = obj_cy - plat_cy;
    let dist = (dx * dx + dy * dy).sqrt();

    let obj_half = (obj.size.0 + obj.size.1) * 0.25;
    let combined = r + obj_half;

    if dist >= combined {
        return None;
    }

    if dist < 0.001 {
        return Some((0.0, -(combined)));
    }

    let overlap = combined - dist;
    let nx = dx / dist;
    let ny = dy / dist;

    Some((nx * overlap, ny * overlap))
}
