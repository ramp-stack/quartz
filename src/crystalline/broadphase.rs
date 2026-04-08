use super::types::PhysicsBody;

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl Aabb {
    /// Build an AABB from a PhysicsBody.
    /// Handles slopes and rotated platforms the same way Quartz's
    /// `check_collision` does: expanded bounding box for rotated platforms,
    /// slope-aware AABB for sloped platforms.
    pub fn from_body(body: &PhysicsBody) -> Self {
        if body.is_platform && body.slope.is_some() {
            let (x, y, w, h) = slope_aabb(body);
            Self {
                min_x: x,
                min_y: y,
                max_x: x + w,
                max_y: y + h,
            }
        } else if body.is_platform && body.rotation != 0.0 {
            let (x, y, w, h) = rotated_aabb(body);
            Self {
                min_x: x,
                min_y: y,
                max_x: x + w,
                max_y: y + h,
            }
        } else {
            Self {
                min_x: body.position.0,
                min_y: body.position.1,
                max_x: body.position.0 + body.size.0,
                max_y: body.position.1 + body.size.1,
            }
        }
    }

    pub fn overlaps(&self, other: &Aabb) -> bool {
        self.min_x < other.max_x
            && self.max_x > other.min_x
            && self.min_y < other.max_y
            && self.max_y > other.min_y
    }

    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.min_x && px <= self.max_x
            && py >= self.min_y && py <= self.max_y
    }
}

/// Flat O(n²) broadphase. Upgrade path: spatial hash → BVH/DBVT.
/// API surface stays the same regardless of backing structure.
#[derive(Clone)]
pub struct AabbPairFinder {
    aabbs: Vec<(usize, Aabb)>,
}

impl AabbPairFinder {
    pub fn new() -> Self {
        Self {
            aabbs: Vec::new(),
        }
    }

    /// Rebuild from current body positions. Called once per substep.
    pub fn rebuild(&mut self, bodies: &[PhysicsBody]) {
        self.aabbs.clear();
        for body in bodies {
            if body.visible {
                self.aabbs.push((body.id, Aabb::from_body(body)));
            }
        }
    }

    /// Rebuild with speculative margin: each AABB is expanded by the body's
    /// momentum (velocity proxy) so fast-moving objects are never missed.
    /// This is the "speculative contacts" broadphase from Box2D 3.0 / Unity.
    pub fn rebuild_speculative(&mut self, bodies: &[PhysicsBody], dt: f32) {
        self.aabbs.clear();
        for body in bodies {
            if !body.visible {
                continue;
            }
            let mut aabb = Aabb::from_body(body);
            if !body.is_platform {
                let vx = body.momentum.0 * dt;
                let vy = body.momentum.1 * dt;
                if vx > 0.0 {
                    aabb.max_x += vx;
                } else {
                    aabb.min_x += vx;
                }
                if vy > 0.0 {
                    aabb.max_y += vy;
                } else {
                    aabb.min_y += vy;
                }
            }
            self.aabbs.push((body.id, aabb));
        }
    }

    /// Return all overlapping (id_a, id_b) pairs where id_a < id_b.
    pub fn query_pairs(&self) -> Vec<(usize, usize)> {
        let mut pairs = Vec::new();
        for i in 0..self.aabbs.len() {
            for j in (i + 1)..self.aabbs.len() {
                if self.aabbs[i].1.overlaps(&self.aabbs[j].1) {
                    let a = self.aabbs[i].0;
                    let b = self.aabbs[j].0;
                    pairs.push((a.min(b), a.max(b)));
                }
            }
        }
        pairs
    }

    /// Remove a body from the broadphase (e.g. destroyed mid-frame).
    pub fn remove(&mut self, body_id: usize) {
        self.aabbs.retain(|(id, _)| *id != body_id);
    }

    /// Point query: which body (if any) contains this point?
    pub fn query_point(&self, px: f32, py: f32) -> Option<usize> {
        self.aabbs
            .iter()
            .find(|(_, aabb)| aabb.contains_point(px, py))
            .map(|(id, _)| *id)
    }
}

// ── AABB helpers (ported from Quartz) ────────────────────────

/// Expanded AABB for a rotated platform.
fn rotated_aabb(body: &PhysicsBody) -> (f32, f32, f32, f32) {
    if body.rotation == 0.0 {
        return (body.position.0, body.position.1, body.size.0, body.size.1);
    }
    let theta = body.rotation.to_radians();
    let cos_t = theta.cos().abs();
    let sin_t = theta.sin().abs();
    let w = body.size.0 * cos_t + body.size.1 * sin_t;
    let h = body.size.0 * sin_t + body.size.1 * cos_t;
    let cx = body.position.0 + body.size.0 * 0.5;
    let cy = body.position.1 + body.size.1 * 0.5;
    (cx - w * 0.5, cy - h * 0.5, w, h)
}

/// Slope-aware AABB for a sloped platform.
fn slope_aabb(body: &PhysicsBody) -> (f32, f32, f32, f32) {
    match body.slope {
        None => (body.position.0, body.position.1, body.size.0, body.size.1),
        Some((left_off, right_off)) => {
            let left_y = body.position.1 + left_off;
            let right_y = body.position.1 + right_off;
            let top = left_y.min(right_y);
            let bottom = left_y.max(right_y) + body.size.1;
            (body.position.0, top, body.size.0, bottom - top)
        }
    }
}
