// ── Constraints ──────────────────────────────────────────────
//
// High-level constraint system for Quartz. Sits on top of
// crystalline (uses the same (f32, f32) convention) but lives
// outside the solver to keep crystalline focused on low-level
// physics.
//
// Provides: distance constraints, spring constraints, and
// grapple/swinging mechanics with game-friendly defaults.

// ── Math helpers ─────────────────────────────────────────────

/// Solve a distance constraint between two points.
/// Returns the corrective impulse to apply to the dynamic point (pos_a).
///
/// - `pos_a` / `vel_a` — the dynamic object's position & velocity
/// - `pos_b` — the anchor (static point or another object's center)
/// - `rest_length` — desired distance between points
/// - `stiffness` — 0.0..=1.0, how rigidly the constraint is enforced
/// - `damping` — 0.0..=1.0, how quickly oscillation decays
pub fn solve_distance_constraint(
    pos_a: (f32, f32),
    pos_b: (f32, f32),
    rest_length: f32,
    stiffness: f32,
    damping: f32,
    vel_a: (f32, f32),
) -> (f32, f32) {
    let dx = pos_a.0 - pos_b.0;
    let dy = pos_a.1 - pos_b.1;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 0.001 {
        return (0.0, 0.0);
    }

    let nx = dx / dist;
    let ny = dy / dist;

    // Spring force: pulls toward rest length
    let stretch = dist - rest_length;
    let spring_force = -stiffness * stretch;

    // Damping force: opposes radial velocity
    let radial_vel = vel_a.0 * nx + vel_a.1 * ny;
    let damp_force = -damping * radial_vel;

    let total = spring_force + damp_force;
    (nx * total, ny * total)
}

// ── Distance Constraint ──────────────────────────────────────

/// A simple distance constraint between two points.
/// Can be used for ropes, tethers, rigid links, etc.
#[derive(Clone, Debug)]
pub struct DistanceConstraint {
    pub anchor: (f32, f32),
    pub rest_length: f32,
    pub stiffness: f32,
    pub damping: f32,
    pub active: bool,
}

impl Default for DistanceConstraint {
    fn default() -> Self {
        Self {
            anchor: (0.0, 0.0),
            rest_length: 100.0,
            stiffness: 0.5,
            damping: 0.1,
            active: true,
        }
    }
}

impl DistanceConstraint {
    pub fn new(anchor: (f32, f32), rest_length: f32) -> Self {
        Self {
            anchor,
            rest_length,
            ..Default::default()
        }
    }

    pub fn with_stiffness(mut self, stiffness: f32) -> Self {
        self.stiffness = stiffness.clamp(0.0, 1.0);
        self
    }

    pub fn with_damping(mut self, damping: f32) -> Self {
        self.damping = damping.clamp(0.0, 1.0);
        self
    }

    /// Compute the corrective impulse for this frame.
    pub fn solve(&self, pos: (f32, f32), vel: (f32, f32)) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }
        solve_distance_constraint(pos, self.anchor, self.rest_length, self.stiffness, self.damping, vel)
    }
}

// ── Spring Constraint ────────────────────────────────────────

/// A spring connecting two points. Softer than a distance constraint —
/// suitable for bouncy tethers, bungee cords, elastic connections.
#[derive(Clone, Debug)]
pub struct SpringConstraint {
    pub anchor: (f32, f32),
    pub rest_length: f32,
    /// Spring constant (force per unit displacement). Higher = stiffer.
    pub spring_k: f32,
    /// Damping coefficient. Higher = less oscillation.
    pub damp_k: f32,
    pub active: bool,
}

impl Default for SpringConstraint {
    fn default() -> Self {
        Self {
            anchor: (0.0, 0.0),
            rest_length: 100.0,
            spring_k: 200.0,
            damp_k: 10.0,
            active: true,
        }
    }
}

impl SpringConstraint {
    pub fn new(anchor: (f32, f32), rest_length: f32) -> Self {
        Self {
            anchor,
            rest_length,
            ..Default::default()
        }
    }

    pub fn with_spring_k(mut self, k: f32) -> Self {
        self.spring_k = k.max(0.0);
        self
    }

    pub fn with_damp_k(mut self, k: f32) -> Self {
        self.damp_k = k.max(0.0);
        self
    }

    /// Compute the spring force for this frame.
    pub fn solve(&self, pos: (f32, f32), vel: (f32, f32)) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }
        let dx = pos.0 - self.anchor.0;
        let dy = pos.1 - self.anchor.1;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < 0.001 {
            return (0.0, 0.0);
        }

        let nx = dx / dist;
        let ny = dy / dist;

        let stretch = dist - self.rest_length;
        let spring_f = -self.spring_k * stretch;

        let radial_vel = vel.0 * nx + vel.1 * ny;
        let damp_f = -self.damp_k * radial_vel;

        let total = spring_f + damp_f;
        (nx * total, ny * total)
    }
}

// ── Grapple Constraint ───────────────────────────────────────

/// The swing direction bias for a grapple. Controls which
/// directions the player can swing freely in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SwingBias {
    /// No directional bias — swing freely in all directions.
    None,
    /// Favor horizontal swing (reduce vertical pull slightly).
    Horizontal,
    /// Favor vertical swing (reduce horizontal pull slightly).
    Vertical,
}

impl Default for SwingBias {
    fn default() -> Self { SwingBias::None }
}

/// Result of solving a grapple constraint for one frame.
///
/// Contains the corrected position and velocity when the rope is
/// taut (object exceeded rope length). When the object is within
/// the rope length, both fields are `None` — no correction needed.
#[derive(Clone, Debug)]
pub struct GrappleCorrection {
    /// Corrected absolute position (center of the object, projected
    /// back onto the rope arc). `None` if within rope length.
    pub position: Option<(f32, f32)>,
    /// Corrected absolute velocity (outward radial component stripped,
    /// tangential component optionally damped). `None` if within rope length.
    pub velocity: Option<(f32, f32)>,
}

impl GrappleCorrection {
    /// No correction needed — object is within rope length.
    pub fn none() -> Self {
        Self { position: None, velocity: None }
    }

    /// Whether any correction was applied.
    pub fn applied(&self) -> bool {
        self.position.is_some()
    }
}

/// A grapple/swinging constraint. Designed for pendulum-style
/// swinging mechanics (grappling hooks, rope swings, web-slingers).
///
/// The grapple acts as a one-sided distance constraint: it only
/// pulls the object back when it exceeds the rope length, never
/// pushes when the object is closer than the rope length.
///
/// Uses **position-level correction** (XPBD-style) rather than
/// spring forces, so the rope feels rigid by default. Stiffness
/// controls how much of the correction is applied each frame:
/// `1.0` = fully rigid (default), `0.5` = half correction per
/// frame (elastic/bouncy).
///
/// Attach to a static world anchor or to another game object.
#[derive(Clone, Debug)]
pub struct GrappleConstraint {
    /// World-space anchor point (updated each frame if attached to an object).
    pub anchor: (f32, f32),
    /// Name of the object the grapple is attached to (if any).
    /// When set, anchor is updated to that object's center each frame.
    pub anchor_object: Option<String>,
    /// Rope length — max distance before the constraint kicks in.
    pub length: f32,
    /// How rigidly the rope enforces the distance.
    /// 1.0 = fully rigid (hard position projection, like a real rope).
    /// 0.0 = no enforcement. Values in between create elastic/bouncy ropes.
    pub stiffness: f32,
    /// Tangential velocity damping per frame. Controls how quickly the
    /// swing loses energy. 0.0 = no damping (perpetual swing),
    /// 1.0 = all tangential velocity stripped instantly.
    /// Typical values: 0.001–0.05 for natural-feeling swing.
    pub damping: f32,
    /// Optional maximum swing speed (px/frame-unit). 0 = unlimited.
    pub max_swing_speed: f32,
    /// Whether the rope auto-shortens when the object is closer than `length`.
    pub auto_shorten: bool,
    /// Swing direction bias.
    pub swing_bias: SwingBias,
    /// Whether this grapple is currently active.
    pub active: bool,
}

impl Default for GrappleConstraint {
    fn default() -> Self {
        Self {
            anchor: (0.0, 0.0),
            anchor_object: None,
            length: 200.0,
            stiffness: 0.8,
            damping: 0.05,
            max_swing_speed: 0.0,
            auto_shorten: false,
            swing_bias: SwingBias::None,
            active: true,
        }
    }
}

impl GrappleConstraint {
    /// Create a grapple attached to a fixed world point.
    pub fn at_point(anchor: (f32, f32), length: f32) -> Self {
        Self {
            anchor,
            length: length.max(1.0),
            ..Default::default()
        }
    }

    /// Create a grapple attached to a named game object.
    pub fn to_object(object_name: impl Into<String>, length: f32) -> Self {
        Self {
            anchor_object: Some(object_name.into()),
            length: length.max(1.0),
            ..Default::default()
        }
    }

    pub fn with_stiffness(mut self, stiffness: f32) -> Self {
        self.stiffness = stiffness.clamp(0.0, 1.0);
        self
    }

    pub fn with_damping(mut self, damping: f32) -> Self {
        self.damping = damping.clamp(0.0, 1.0);
        self
    }

    pub fn with_max_swing_speed(mut self, speed: f32) -> Self {
        self.max_swing_speed = speed.max(0.0);
        self
    }

    pub fn with_auto_shorten(mut self) -> Self {
        self.auto_shorten = true;
        self
    }

    pub fn with_swing_bias(mut self, bias: SwingBias) -> Self {
        self.swing_bias = bias;
        self
    }

    /// Compute the grapple correction for this frame.
    ///
    /// The grapple is **one-sided**: it only constrains when the object
    /// exceeds `self.length` from the anchor. When the object is closer,
    /// no correction is applied (it's a rope, not a rod).
    ///
    /// Uses **position-level correction**: projects the object back onto
    /// the rope arc and strips the outward radial velocity component.
    /// This produces a rigid, inextensible rope feel (like the classic
    /// pendulum/grapple-hook mechanic) rather than a springy tether.
    ///
    /// If `auto_shorten` is true, the rope length shrinks to match the
    /// current distance when the object is closer.
    pub fn solve(&mut self, obj_pos: (f32, f32), obj_vel: (f32, f32)) -> GrappleCorrection {
        if !self.active {
            return GrappleCorrection::none();
        }

        let dx = obj_pos.0 - self.anchor.0;
        let dy = obj_pos.1 - self.anchor.1;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < 0.001 {
            return GrappleCorrection::none();
        }

        // Auto-shorten: reduce rope length when object is closer
        if self.auto_shorten && dist < self.length {
            self.length = dist;
        }

        // One-sided: only constrain when beyond rope length
        if dist <= self.length {
            return GrappleCorrection::none();
        }

        let nx = dx / dist;
        let ny = dy / dist;

        // ── Position correction: project back onto the rope arc ──────
        //
        // Target position is on the circle of radius `length` centered
        // at the anchor, in the current direction from anchor to object.
        let target_x = self.anchor.0 + nx * self.length;
        let target_y = self.anchor.1 + ny * self.length;

        // Stiffness interpolates between current position (0.0) and
        // fully projected position (1.0).
        let corrected_x = obj_pos.0 + (target_x - obj_pos.0) * self.stiffness;
        let corrected_y = obj_pos.1 + (target_y - obj_pos.1) * self.stiffness;

        // ── Velocity correction: decompose into radial + tangential ──
        let tangent_x = -ny;
        let tangent_y = nx;

        let radial_vel = obj_vel.0 * nx + obj_vel.1 * ny;
        let tangent_vel = obj_vel.0 * tangent_x + obj_vel.1 * tangent_y;

        // Strip outward radial velocity (moving away from anchor).
        // At stiffness=1.0, ALL outward radial velocity is removed.
        // At lower stiffness, proportionally less is removed.
        let corrected_radial = if radial_vel > 0.0 {
            radial_vel * (1.0 - self.stiffness)
        } else {
            // Moving inward (toward anchor) — allow it
            radial_vel
        };

        // Apply tangential damping (slight energy loss per frame for
        // natural swing decay).
        let damped_tangent = tangent_vel * (1.0 - self.damping);

        // Reconstruct velocity from corrected radial + damped tangential
        let mut new_vx = nx * corrected_radial + tangent_x * damped_tangent;
        let mut new_vy = ny * corrected_radial + tangent_y * damped_tangent;

        // Apply swing bias
        match self.swing_bias {
            SwingBias::None => {}
            SwingBias::Horizontal => {
                // Blend vertical velocity correction (reduce vertical pull)
                new_vy = obj_vel.1 + (new_vy - obj_vel.1) * 0.6;
            }
            SwingBias::Vertical => {
                // Blend horizontal velocity correction (reduce horizontal pull)
                new_vx = obj_vel.0 + (new_vx - obj_vel.0) * 0.6;
            }
        }

        // Speed cap
        if self.max_swing_speed > 0.0 {
            let speed = (new_vx * new_vx + new_vy * new_vy).sqrt();
            if speed > self.max_swing_speed {
                let scale = self.max_swing_speed / speed;
                new_vx *= scale;
                new_vy *= scale;
            }
        }

        GrappleCorrection {
            position: Some((corrected_x, corrected_y)),
            velocity: Some((new_vx, new_vy)),
        }
    }
}

// ── Presets ──────────────────────────────────────────────────

impl GrappleConstraint {
    /// Standard grappling hook — rigid rope, moderate damping.
    pub fn grappling_hook(anchor: (f32, f32), length: f32) -> Self {
        Self::at_point(anchor, length)
            .with_stiffness(0.9)
            .with_damping(0.05)
    }

    /// Web-slinger style — slightly elastic, fast swing.
    pub fn web_swing(anchor: (f32, f32), length: f32) -> Self {
        Self::at_point(anchor, length)
            .with_stiffness(0.7)
            .with_damping(0.02)
            .with_swing_bias(SwingBias::Horizontal)
    }

    /// Bungee cord — very elastic, high damping.
    pub fn bungee(anchor: (f32, f32), length: f32) -> Self {
        Self::at_point(anchor, length)
            .with_stiffness(0.3)
            .with_damping(0.15)
    }

    /// Rigid tether — no stretch, strong pull-back.
    pub fn rigid_tether(anchor: (f32, f32), length: f32) -> Self {
        Self::at_point(anchor, length)
            .with_stiffness(1.0)
            .with_damping(0.1)
    }

    /// Wrecking ball — heavy, slow, powerful.
    pub fn wrecking_ball(anchor: (f32, f32), length: f32) -> Self {
        Self::at_point(anchor, length)
            .with_stiffness(0.95)
            .with_damping(0.02)
            .with_max_swing_speed(800.0)
    }
}
