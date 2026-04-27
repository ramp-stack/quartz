use super::types::*;
use super::solver::SleepState;

// ── Pivot-aware geometry helpers for PhysicsBody ──────────────────────────────

/// World-space position of a body's normalised pivot point.
#[inline]
fn pivot_world(body: &PhysicsBody) -> (f32, f32) {
    (body.position.0 + body.size.0 * body.pivot.0,
     body.position.1 + body.size.1 * body.pivot.1)
}

/// Rotate a local offset around the body's pivot into world space.
#[inline]
fn local_to_world(body: &PhysicsBody, local: (f32, f32)) -> (f32, f32) {
    let (pw_x, pw_y) = pivot_world(body);
    if body.rotation == 0.0 { return (pw_x + local.0, pw_y + local.1); }
    let theta = body.rotation.to_radians();
    let cos_t = theta.cos();
    let sin_t = theta.sin();
    (pw_x + local.0 * cos_t - local.1 * sin_t,
     pw_y + local.0 * sin_t + local.1 * cos_t)
}

/// World-space centre of the body after applying rotation.
#[inline]
fn rotated_center(body: &PhysicsBody) -> (f32, f32) {
    local_to_world(body, (body.size.0 * (0.5 - body.pivot.0),
                          body.size.1 * (0.5 - body.pivot.1)))
}

/// Four world-space corners of the body after applying rotation around pivot.
#[inline]
fn corners_world(body: &PhysicsBody) -> [(f32, f32); 4] {
    let (px, py) = body.pivot;
    let local = [
        (-body.size.0 * px,          -body.size.1 * py),
        ( body.size.0 * (1.0 - px),  -body.size.1 * py),
        (-body.size.0 * px,           body.size.1 * (1.0 - py)),
        ( body.size.0 * (1.0 - px),   body.size.1 * (1.0 - py)),
    ];
    std::array::from_fn(|i| local_to_world(body, local[i]))
}

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

    // Preserve the narrowphase contact normal. collect_contacts() already
    // computes the correct surface normal for flat, sloped, and rotated
    // surfaces (including one-way checks/sign handling). Overriding with the
    // platform default normal flattens slope contacts and breaks align_to_slope.
    let (nx, ny) = (contact.nx, contact.ny);

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
    let (pw_x, pw_y) = pivot_world(plat);
    let theta = plat.rotation.to_radians();
    let cos_t = theta.cos();
    let sin_t = theta.sin();
    // Left and right edge endpoints of the top surface in world space.
    let left_wx  = local_to_world(plat, (-plat.size.0 * plat.pivot.0,       -plat.size.1 * plat.pivot.1));
    let right_wx = local_to_world(plat, ( plat.size.0 * (1.0 - plat.pivot.0), -plat.size.1 * plat.pivot.1));
    let _ = (pw_x, pw_y, cos_t, sin_t); // helpers computed above
    // Clamp world_x to the horizontal extent of the platform.
    let clamped_x = world_x.clamp(left_wx.0.min(right_wx.0), left_wx.0.max(right_wx.0));
    let edge_dx = right_wx.0 - left_wx.0;
    let t = if edge_dx.abs() < 0.001 { 0.5 } else { (clamped_x - left_wx.0) / edge_dx };
    left_wx.1 + t * (right_wx.1 - left_wx.1)
}

/// SAT-based penetration depth along a given normal.
/// Mirrors `penetration_depth()` in canvas.rs.
pub(super) fn penetration_depth(obj: &PhysicsBody, plat: &PhysicsBody, nx: f32, ny: f32) -> f32 {
    let (obj_cx,  obj_cy)  = rotated_center(obj);
    let (plat_cx, plat_cy) = rotated_center(plat);
    // AABB half-extents via corner sweep.
    let obj_corners  = corners_world(obj);
    let plat_corners = corners_world(plat);
    let obj_half_x  = (obj_corners.iter().map(|c|c.0).fold(f32::MIN,|a,b|a.max(b))
                     - obj_corners.iter().map(|c|c.0).fold(f32::MAX,|a,b|a.min(b))) * 0.5;
    let obj_half_y  = (obj_corners.iter().map(|c|c.1).fold(f32::MIN,|a,b|a.max(b))
                     - obj_corners.iter().map(|c|c.1).fold(f32::MAX,|a,b|a.min(b))) * 0.5;
    let plat_half_x = (plat_corners.iter().map(|c|c.0).fold(f32::MIN,|a,b|a.max(b))
                     - plat_corners.iter().map(|c|c.0).fold(f32::MAX,|a,b|a.min(b))) * 0.5;
    let plat_half_y = (plat_corners.iter().map(|c|c.1).fold(f32::MIN,|a,b|a.max(b))
                     - plat_corners.iter().map(|c|c.1).fold(f32::MAX,|a,b|a.min(b))) * 0.5;
    let obj_half  = obj_half_x  * nx.abs() + obj_half_y  * ny.abs();
    let plat_half = plat_half_x * nx.abs() + plat_half_y * ny.abs();
    let sep = (obj_cx - plat_cx) * nx + (obj_cy - plat_cy) * ny;
    let overlap = obj_half + plat_half - sep.abs();
    if overlap > 0.0 { overlap } else { 0.0 }
}

/// OBB-based solid collision resolution. Returns (dx, dy, face) where
/// face: 0=top, 1=bottom, 2=left, 3=right of the platform in local space.
/// Mirrors `resolve_solid_collision()` in canvas.rs.
pub(super) fn resolve_solid_collision(
    obj: &PhysicsBody,
    plat: &PhysicsBody,
) -> Option<(f32, f32, u8)> {
    let (plat_pivot_x, plat_pivot_y) = pivot_world(plat);
    let inv_theta = -plat.rotation.to_radians();
    let (cos_t, sin_t) = (inv_theta.cos(), inv_theta.sin());

    // Platform local extents from its pivot.
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

    let fwd_theta = plat.rotation.to_radians();
    let (cos_f, sin_f) = (fwd_theta.cos(), fwd_theta.sin());
    Some((
        (lnx * cos_f - lny * sin_f) * depth,
        (lnx * sin_f + lny * cos_f) * depth,
        face,
    ))
}

/// Circle-based solid collision resolution. Returns (dx, dy) push-out vector.
/// Mirrors `resolve_circle_collision()` in canvas.rs.
pub(super) fn resolve_circle_collision(
    obj: &PhysicsBody,
    plat: &PhysicsBody,
    radius: f32,
) -> Option<(f32, f32)> {
    let r = if radius <= 0.0 { plat.size.0.min(plat.size.1) * 0.5 } else { radius };
    let (obj_cx,  obj_cy)  = rotated_center(obj);
    let (plat_cx, plat_cy) = rotated_center(plat);
    let dx = obj_cx - plat_cx;
    let dy = obj_cy - plat_cy;
    let dist = (dx * dx + dy * dy).sqrt();
    let combined = r + (obj.size.0 + obj.size.1) * 0.25;
    if dist >= combined { return None; }
    if dist < 0.001 { return Some((0.0, -combined)); }
    let overlap = combined - dist;
    Some((dx / dist * overlap, dy / dist * overlap))
}
