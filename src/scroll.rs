use crate::canvas::Canvas;
use crate::file_watcher::Shared;

// ───────────────────────────────────────────────────────────────────────────────
//  ScrollConfig
// ───────────────────────────────────────────────────────────────────────────────

/// Tuning knobs shared by both scroll axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollConfig {
    /// Velocity added per scroll-event unit.  Default 5.5.
    pub accel: f32,
    /// Velocity multiplier applied every tick (decay < 1.0).  Default 0.10.
    pub friction: f32,
    /// Maximum scroll velocity (absolute).  Default 90.0.
    pub max_vel: f32,
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self { accel: 5.5, friction: 0.10, max_vel: 90.0 }
    }
}

impl ScrollConfig {
    /// Tuned for code editor use — slower acceleration, more friction, lower top speed.
    pub fn editor() -> Self {
        Self { accel: 2.5, friction: 0.18, max_vel: 40.0 }
    }

    /// Snappier, higher top speed — good for long lists or file trees.
    pub fn fast() -> Self {
        Self { accel: 9.0, friction: 0.08, max_vel: 150.0 }
    }

    /// Sluggish, low top speed — good for image viewers or zoomed canvases.
    pub fn slow() -> Self {
        Self { accel: 3.0, friction: 0.15, max_vel: 45.0 }
    }
}

// ───────────────────────────────────────────────────────────────────────────────
//  AxisScroll  (internal)
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct AxisScroll {
    pub offset:       f32,
    pub vel:          f32,
    pub intent:       f32,
    pub content_len:  f32,
    pub viewport_len: f32,
}

impl AxisScroll {
    fn new() -> Self {
        Self { offset: 0.0, vel: 0.0, intent: 0.0, content_len: 0.0, viewport_len: 0.0 }
    }

    fn push(&mut self, cfg: &ScrollConfig, units: f32) {
        let dir      = units.signum();
        let target   = (units.abs() * cfg.accel).min(cfg.max_vel) * dir;
        // If already moving faster than the new target in the same direction,
        // keep the existing velocity — don't pile on. Otherwise snap to target.
        let same_dir = self.vel.signum() == dir;
        self.vel = if same_dir && self.vel.abs() >= target.abs() {
            self.vel
        } else {
            target
        };
    }

    fn tick(&mut self, cfg: &ScrollConfig) {
        if self.intent != 0.0 {
            self.vel = self.intent;
        }
        self.offset += self.vel;
        self.vel *= 1.0 - cfg.friction;
        if self.vel.abs() < 0.01 { self.vel = 0.0; }
        let max_offset = (self.content_len - self.viewport_len).max(0.0);
        self.offset = self.offset.clamp(0.0, max_offset);
    }
}

// ───────────────────────────────────────────────────────────────────────────────
//  ScrollState
// ───────────────────────────────────────────────────────────────────────────────

/// Physics state for a scrollable region. Both axes are independent.
///
/// Call [`tick`] once per frame in `on_update`, then read [`offset_x`] /
/// [`offset_y`]. Drive velocity from input handlers via [`push_x`] / [`push_y`]
/// and [`set_intent_x`] / [`set_intent_y`].
///
/// [`tick`]: ScrollState::tick
/// [`offset_x`]: ScrollState::offset_x
/// [`offset_y`]: ScrollState::offset_y
/// [`push_x`]: ScrollState::push_x
/// [`push_y`]: ScrollState::push_y
/// [`set_intent_x`]: ScrollState::set_intent_x
/// [`set_intent_y`]: ScrollState::set_intent_y
#[derive(Debug, Clone)]
pub struct ScrollState {
    cfg: ScrollConfig,
    x:   AxisScroll,
    y:   AxisScroll,
}

impl ScrollState {
    pub fn new(cfg: ScrollConfig) -> Self {
        Self { cfg, x: AxisScroll::new(), y: AxisScroll::new() }
    }

    // ── sizing ────────────────────────────────────────────────────────────────

    pub fn set_content_size(&mut self, w: f32, h: f32) {
        self.x.content_len  = w;
        self.y.content_len  = h;
    }

    pub fn set_viewport_size(&mut self, w: f32, h: f32) {
        self.x.viewport_len = w;
        self.y.viewport_len = h;
    }

    // ── impulse drivers ───────────────────────────────────────────────────────

    /// Instantaneous horizontal velocity impulse (e.g. raw `dx` from scroll event).
    pub fn push_x(&mut self, units: f32) { self.x.push(&self.cfg, units); }

    /// Instantaneous vertical velocity impulse (e.g. raw `dy` from scroll event).
    pub fn push_y(&mut self, units: f32) { self.y.push(&self.cfg, units); }

    // ── sustained intent ──────────────────────────────────────────────────────

    /// Sustained horizontal velocity applied every tick until cleared. Pass `0.0` to stop.
    pub fn set_intent_x(&mut self, vel: f32) { self.x.intent = vel; }

    /// Sustained vertical velocity applied every tick until cleared. Pass `0.0` to stop.
    pub fn set_intent_y(&mut self, vel: f32) { self.y.intent = vel; }

    /// Clear sustained intent on both axes.
    pub fn clear_intent(&mut self) {
        self.x.intent = 0.0;
        self.y.intent = 0.0;
    }

    // ── direct velocity access ────────────────────────────────────────────────

    pub fn set_vel_x(&mut self, v: f32) { self.x.vel = v; }
    pub fn set_vel_y(&mut self, v: f32) { self.y.vel = v; }
    pub fn vel_x(&self) -> f32 { self.x.vel }
    pub fn vel_y(&self) -> f32 { self.y.vel }

    /// The configured maximum velocity — use as magnitude for `set_intent_*`.
    pub fn max_vel(&self) -> f32 { self.cfg.max_vel }

    // ── tick ──────────────────────────────────────────────────────────────────

    /// Advance scroll physics by one tick. Call once per frame in `on_update`.
    pub fn tick(&mut self) {
        self.x.tick(&self.cfg);
        self.y.tick(&self.cfg);
    }

    // ── read ──────────────────────────────────────────────────────────────────

    pub fn offset_x(&self) -> f32 { self.x.offset }
    pub fn offset_y(&self) -> f32 { self.y.offset }

    // ── jump / reset ──────────────────────────────────────────────────────────

    /// Instantly jump to an offset without animation.
    pub fn jump_to(&mut self, x: f32, y: f32) {
        let max_x = (self.x.content_len - self.x.viewport_len).max(0.0);
        let max_y = (self.y.content_len - self.y.viewport_len).max(0.0);
        self.x.offset = x.clamp(0.0, max_x);
        self.y.offset = y.clamp(0.0, max_y);
        self.x.vel    = 0.0;
        self.y.vel    = 0.0;
    }

    /// Reset both axes to zero with no velocity.
    pub fn reset(&mut self) { self.jump_to(0.0, 0.0); }
}

// ───────────────────────────────────────────────────────────────────────────────
//  ScrollView
// ───────────────────────────────────────────────────────────────────────────────

/// A scrollable region that owns a [`Shared<ScrollState>`] and wires up
/// `on_mouse_scroll` automatically via [`mount`].
///
/// All other input (keyboard intent, drag-edge) is driven by the caller through
/// [`state`].
///
/// # Example
/// ```rust
/// // setup
/// let scroll = ScrollView::new(x, y, w, h, ScrollConfig::editor());
/// scroll.set_content_size(max_line_width, total_lines * line_height);
/// scroll.mount(cv);
///
/// // on_update
/// scroll.tick();
/// let gs = scroll.offset_y();
/// let hs = scroll.offset_x();
///
/// // from key / drag-edge handlers
/// scroll.state().get_mut().set_intent_y(-scroll.state().get().max_vel());
/// ```
///
/// [`mount`]: ScrollView::mount
/// [`state`]: ScrollView::state
#[derive(Clone)]
pub struct ScrollView {
    state: Shared<ScrollState>,
    x:     Shared<f32>,
    y:     Shared<f32>,
    w:     Shared<f32>,
    h:     Shared<f32>,
}

impl ScrollView {
    /// Create a new `ScrollView`.
    ///
    /// `x`, `y`, `w`, `h` are the screen-space bounds of the scrollable region.
    /// Call [`set_bounds`] if they can change at runtime.
    ///
    /// [`set_bounds`]: ScrollView::set_bounds
    pub fn new(x: f32, y: f32, w: f32, h: f32, cfg: ScrollConfig) -> Self {
        let mut state = ScrollState::new(cfg);
        state.set_viewport_size(w, h);
        Self {
            state: Shared::new(state),
            x:     Shared::new(x),
            y:     Shared::new(y),
            w:     Shared::new(w),
            h:     Shared::new(h),
        }
    }

    // ── bounds ────────────────────────────────────────────────────────────────

    /// Update the screen-space bounds and viewport size together.
    pub fn set_bounds(&self, x: f32, y: f32, w: f32, h: f32) {
        *self.x.get_mut() = x;
        *self.y.get_mut() = y;
        *self.w.get_mut() = w;
        *self.h.get_mut() = h;
        self.state.get_mut().set_viewport_size(w, h);
    }

    // ── content size ──────────────────────────────────────────────────────────

    pub fn set_content_size(&self, content_w: f32, content_h: f32) {
        self.state.get_mut().set_content_size(content_w, content_h);
    }

    // ── state access ──────────────────────────────────────────────────────────

    /// Direct access to the underlying `Shared<ScrollState>`.
    pub fn state(&self) -> &Shared<ScrollState> { &self.state }

    // ── tick + read ───────────────────────────────────────────────────────────

    /// Advance physics. Call once per frame inside `on_update`.
    pub fn tick(&self) { self.state.get_mut().tick(); }

    pub fn offset_x(&self) -> f32 { self.state.get().offset_x() }
    pub fn offset_y(&self) -> f32 { self.state.get().offset_y() }

    // ── mount ─────────────────────────────────────────────────────────────────

    /// Register the `on_mouse_scroll` handler on the canvas.
    /// Call once during component setup.
    pub fn mount(&self, cv: &mut Canvas) {
        let state = self.state.clone();
        let lx    = self.x.clone();
        let ly    = self.y.clone();
        let lw    = self.w.clone();
        let lh    = self.h.clone();

        cv.on_mouse_scroll(move |cv, (dx, dy)| {
            if let Some((mx, my)) = cv.mouse_position() {
                let ex = *lx.get(); let ey = *ly.get();
                let ew = *lw.get(); let eh = *lh.get();
                if mx < ex || mx > ex + ew || my < ey || my > ey + eh { return; }
            } else { return; }

            let mut st = state.get_mut();

            if dy != 0.0 {
                let dir    = if dy > 0.0 { 1.0f32 } else { -1.0 };
                let cfg    = st.cfg;
                let target = (dy.abs() * cfg.accel).min(cfg.max_vel) * dir;
                let cur    = st.y.vel;
                st.y.vel = if cur.signum() == dir && cur.abs() >= target.abs() { cur } else { target };
            }

            if dx != 0.0 {
                let dir     = if dx > 0.0 { 1.0f32 } else { -1.0 };
                let cfg     = st.cfg;
                let target  = (dx.abs() * cfg.accel).min(cfg.max_vel) * dir;
                let cur_vel = st.x.vel;
                let cur_pos = st.x.offset;
                let h_max   = (st.x.content_len - st.x.viewport_len).max(0.0);
                let new_vel = if cur_vel.signum() == dir && cur_vel.abs() >= target.abs() { cur_vel } else { target };
                st.x.vel = if cur_pos <= 0.0 && new_vel < 0.0 { 0.0 }
                      else if h_max > 0.0 && cur_pos >= h_max && new_vel > 0.0 { 0.0 }
                      else { new_vel };
            }
        });
    }
}

// ───────────────────────────────────────────────────────────────────────────────
//  Canvas convenience
// ───────────────────────────────────────────────────────────────────────────────

impl Canvas {
    /// Mount a `ScrollView` — equivalent to `view.mount(self)`.
    pub fn register_scroll_view(&mut self, view: &ScrollView) {
        view.mount(self);
    }
}