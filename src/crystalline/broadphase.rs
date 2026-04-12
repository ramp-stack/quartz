use super::types::PhysicsBody;
use std::collections::HashSet;

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

    /// Union of two AABBs (smallest AABB containing both).
    pub fn union(a: &Aabb, b: &Aabb) -> Aabb {
        Aabb {
            min_x: a.min_x.min(b.min_x),
            min_y: a.min_y.min(b.min_y),
            max_x: a.max_x.max(b.max_x),
            max_y: a.max_y.max(b.max_y),
        }
    }
}

// ── BVH / Dynamic AABB Tree broadphase ───────────────────────
// Top-down BVH with median-split on longest axis.
// O(n log n) build, O(n log n + k) pair query, O(log n) point query.
// Rebuilt every substep (same pattern as spatial hash before it).
// No external dependencies.

const NULL_NODE: u32 = u32::MAX;

#[derive(Clone, Copy)]
struct BvhNode {
    aabb: Aabb,
    body_id: usize,  // usize::MAX for internal nodes
    left:  u32,      // NULL_NODE for leaves
    right: u32,      // NULL_NODE for leaves
}

impl BvhNode {
    fn is_leaf(&self) -> bool {
        self.left == NULL_NODE
    }
}

#[derive(Clone)]
pub struct AabbPairFinder {
    nodes: Vec<BvhNode>,
    root: u32,
    /// Bodies removed mid-frame (rare). Skipped in queries; tree rebuilt next substep.
    removed: HashSet<usize>,
}

impl AabbPairFinder {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: NULL_NODE,
            removed: HashSet::new(),
        }
    }

    // ── Build ────────────────────────────────────────────────

    /// Recursively build a BVH subtree. Returns the index of the root node.
    fn build(&mut self, items: &mut [(usize, Aabb)]) -> u32 {
        if items.is_empty() {
            return NULL_NODE;
        }

        // Leaf
        if items.len() == 1 {
            let idx = self.nodes.len() as u32;
            self.nodes.push(BvhNode {
                aabb: items[0].1,
                body_id: items[0].0,
                left: NULL_NODE,
                right: NULL_NODE,
            });
            return idx;
        }

        // Determine split axis from combined AABB
        let mut combined = items[0].1;
        for &(_, aabb) in items.iter().skip(1) {
            combined = Aabb::union(&combined, &aabb);
        }
        let dx = combined.max_x - combined.min_x;
        let dy = combined.max_y - combined.min_y;

        // Partition at the median on the longest axis (O(n) nth-element)
        let mid = items.len() / 2;
        if dx >= dy {
            items.select_nth_unstable_by(mid, |a, b| {
                let ca = a.1.min_x + a.1.max_x;
                let cb = b.1.min_x + b.1.max_x;
                ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            items.select_nth_unstable_by(mid, |a, b| {
                let ca = a.1.min_y + a.1.max_y;
                let cb = b.1.min_y + b.1.max_y;
                ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        let (left_items, right_items) = items.split_at_mut(mid);
        let left  = self.build(left_items);
        let right = self.build(right_items);

        // Tight AABB from children (may be tighter than the pre-split combined)
        let node_aabb = Aabb::union(
            &self.nodes[left as usize].aabb,
            &self.nodes[right as usize].aabb,
        );

        let idx = self.nodes.len() as u32;
        self.nodes.push(BvhNode {
            aabb: node_aabb,
            body_id: usize::MAX,
            left,
            right,
        });
        idx
    }

    // ── Public API (unchanged signatures) ────────────────────

    /// Rebuild from current body positions. Called once per substep.
    pub fn rebuild(&mut self, bodies: &[PhysicsBody]) {
        self.nodes.clear();
        self.removed.clear();

        let mut items: Vec<(usize, Aabb)> = bodies
            .iter()
            .filter(|b| b.visible)
            .map(|b| (b.id, Aabb::from_body(b)))
            .collect();

        self.root = self.build(&mut items);
    }

    /// Rebuild with speculative margin: each AABB is expanded by the body's
    /// momentum (velocity proxy) so fast-moving objects are never missed.
    /// This is the "speculative contacts" broadphase from Box2D 3.0 / Unity.
    pub fn rebuild_speculative(&mut self, bodies: &[PhysicsBody], dt: f32) {
        self.nodes.clear();
        self.removed.clear();

        let mut items: Vec<(usize, Aabb)> = Vec::new();
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
            items.push((body.id, aabb));
        }

        self.root = self.build(&mut items);
    }

    /// Return all overlapping (id_a, id_b) pairs where id_a < id_b.
    /// Dual-tree traversal — each pair found at most once by construction
    /// (no HashSet dedup needed).
    pub fn query_pairs(&self) -> Vec<(usize, usize)> {
        let mut pairs = Vec::new();
        if self.root != NULL_NODE {
            self.self_query(self.root, &mut pairs);
        }
        pairs
    }

    /// Remove a body from the broadphase (e.g. destroyed mid-frame).
    /// Marks it for skip; tree is rebuilt next substep anyway.
    pub fn remove(&mut self, body_id: usize) {
        self.removed.insert(body_id);
    }

    /// Point query: which body (if any) contains this point? O(log n) tree walk.
    pub fn query_point(&self, px: f32, py: f32) -> Option<usize> {
        if self.root == NULL_NODE {
            return None;
        }
        self.point_walk(self.root, px, py)
    }

    // ── Internal traversal ───────────────────────────────────

    /// Recurse into both children, then cross-test left vs right.
    fn self_query(&self, idx: u32, pairs: &mut Vec<(usize, usize)>) {
        let node = &self.nodes[idx as usize];
        if node.is_leaf() {
            return;
        }
        self.self_query(node.left, pairs);
        self.self_query(node.right, pairs);
        self.cross_query(node.left, node.right, pairs);
    }

    /// Test every leaf in subtree `a` against every leaf in subtree `b`,
    /// pruning whenever their AABBs don't overlap.
    fn cross_query(&self, a_idx: u32, b_idx: u32, pairs: &mut Vec<(usize, usize)>) {
        let a = &self.nodes[a_idx as usize];
        let b = &self.nodes[b_idx as usize];

        if !a.aabb.overlaps(&b.aabb) {
            return; // entire subtrees culled
        }

        if a.is_leaf() && b.is_leaf() {
            let id_a = a.body_id;
            let id_b = b.body_id;
            if id_a != id_b
                && !self.removed.contains(&id_a)
                && !self.removed.contains(&id_b)
            {
                let (lo, hi) = if id_a < id_b { (id_a, id_b) } else { (id_b, id_a) };
                pairs.push((lo, hi));
            }
            return;
        }

        if a.is_leaf() {
            self.cross_query(a_idx, b.left, pairs);
            self.cross_query(a_idx, b.right, pairs);
        } else if b.is_leaf() {
            self.cross_query(a.left, b_idx, pairs);
            self.cross_query(a.right, b_idx, pairs);
        } else {
            // Both internal — split the one with the larger AABB
            let a_size = (a.aabb.max_x - a.aabb.min_x) * (a.aabb.max_y - a.aabb.min_y);
            let b_size = (b.aabb.max_x - b.aabb.min_x) * (b.aabb.max_y - b.aabb.min_y);
            if a_size >= b_size {
                self.cross_query(a.left, b_idx, pairs);
                self.cross_query(a.right, b_idx, pairs);
            } else {
                self.cross_query(a_idx, b.left, pairs);
                self.cross_query(a_idx, b.right, pairs);
            }
        }
    }

    fn point_walk(&self, idx: u32, px: f32, py: f32) -> Option<usize> {
        let node = &self.nodes[idx as usize];
        if !node.aabb.contains_point(px, py) {
            return None;
        }
        if node.is_leaf() {
            if !self.removed.contains(&node.body_id) {
                return Some(node.body_id);
            }
            return None;
        }
        if let Some(id) = self.point_walk(node.left, px, py) {
            return Some(id);
        }
        self.point_walk(node.right, px, py)
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
