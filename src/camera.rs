use crate::Target;

#[derive(Debug, Clone)]
pub struct Camera {
    // ── Existing public fields (unchanged) ────────────────────────────────────
    pub position:      (f32, f32),
    pub world_size:    (f32, f32),
    pub lerp_speed:    f32,
    /// Currently displayed zoom level. Read by apply_camera_transform.
    /// Advancing via the smooth lerp each frame; direct assignment still works (snaps).
    pub zoom:          f32,

    // ── Smooth zoom fields ────────────────────────────────────────────────────
    /// Desired zoom. Lerped toward each frame. Setting `zoom` directly also
    /// snaps this so instant zoom (old behaviour) is preserved.
    pub zoom_target:      f32,
    /// How fast zoom lerps toward zoom_target. 0.1 = gentle, 0.3 = snappy.
    pub zoom_lerp_speed:  f32,
    /// Optional world-space point that stays fixed on screen when zoom changes.
    /// None = zoom anchors on the camera center (default, safe for most games).
    /// Some((wx, wy)) = zoom toward this world point (e.g. player position,
    /// mouse world position, or a fixed geometric anchor like the floor).
    pub zoom_anchor:      Option<(f32, f32)>,

    // ── Internal (unchanged) ──────────────────────────────────────────────────
    pub(crate) viewport_size:   (f32, f32),
    pub(crate) follow_target:   Option<Target>,
}

impl Camera {
    /// Existing constructor — signature and behaviour unchanged.
    pub fn new(world_size: (f32, f32), viewport_size: (f32, f32)) -> Self {
        Self {
            position:        (0.0, 0.0),
            world_size,
            viewport_size,
            follow_target:   None,
            lerp_speed:      0.10,
            zoom:            1.0,
            zoom_target:     1.0,
            zoom_lerp_speed: 0.12,
            zoom_anchor:     None,
        }
    }

    pub fn follow(&mut self, target: Option<Target>) {
        self.follow_target = target;
    }

    /// Instant snap to world point (existing behaviour unchanged).
    pub fn center_on(&mut self, wx: f32, wy: f32) {
        let (visible_w, visible_h) = self.visible_size();
        self.position.0 = (wx - visible_w * 0.5)
            .clamp(0.0, (self.world_size.0 - visible_w).max(0.0));
        self.position.1 = (wy - visible_h * 0.5)
            .clamp(0.0, (self.world_size.1 - visible_h).max(0.0));
    }

    /// Called by apply_camera_transform each frame — position lerp only.
    pub(crate) fn lerp_toward(&mut self, wx: f32, wy: f32) {
        let (visible_w, visible_h) = self.visible_size();
        let tx = (wx - visible_w * 0.5)
            .clamp(0.0, (self.world_size.0 - visible_w).max(0.0));
        self.position.0 += (tx - self.position.0) * self.lerp_speed;

        // When a zoom_anchor is active the Y position is controlled by the
        // anchor system — do not fight it with the follow lerp.
        if self.zoom_anchor.is_none() {
            let ty = (wy - visible_h * 0.5)
                .clamp(0.0, (self.world_size.1 - visible_h).max(0.0));
            self.position.1 += (ty - self.position.1) * self.lerp_speed;
        }
    }

    // ── Smooth zoom API ───────────────────────────────────────────────────────

    /// Set a desired zoom level; the camera lerps smoothly toward it.
    pub fn smooth_zoom(&mut self, target: f32) {
        self.zoom_target = target.max(0.01);
    }

    /// Zoom toward a world-space point by a multiplicative delta.
    /// Positive delta = zoom in; negative = zoom out.
    pub fn smooth_zoom_at(&mut self, delta: f32, world_anchor: (f32, f32)) {
        let factor = 1.0 + delta;
        let new_target = (self.zoom_target * factor).clamp(0.05, 30.0);
        self.zoom_anchor = Some(world_anchor);
        self.zoom_target = new_target;
    }

    /// Snap zoom instantly (existing direct-assignment behaviour, explicit).
    /// Also snaps zoom_target so the lerp doesn't fight it.
    pub fn snap_zoom(&mut self, value: f32) {
        let v = value.max(0.01);
        self.zoom        = v;
        self.zoom_target = v;
    }

    // ── Coordinate helpers ────────────────────────────────────────────────────

    /// Convert a virtual-screen position to world coordinates.
    pub fn screen_to_world(&self, screen: (f32, f32)) -> (f32, f32) {
        (
            self.position.0 + screen.0 / self.zoom,
            self.position.1 + screen.1 / self.zoom,
        )
    }

    /// Convert a world position to virtual-screen space.
    pub fn world_to_screen(&self, world: (f32, f32)) -> (f32, f32) {
        (
            (world.0 - self.position.0) * self.zoom,
            (world.1 - self.position.1) * self.zoom,
        )
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn visible_size(&self) -> (f32, f32) {
        (self.viewport_size.0 / self.zoom, self.viewport_size.1 / self.zoom)
    }

    /// Advance the zoom lerp. Called once per frame from apply_camera_transform.
    pub(crate) fn advance_zoom_lerp(&mut self) {
        let old_zoom = self.zoom;
        if (self.zoom - self.zoom_target).abs() > 0.0001 {
            self.zoom += (self.zoom_target - self.zoom) * self.zoom_lerp_speed;
            if (self.zoom - self.zoom_target).abs() < 0.001 {
                self.zoom = self.zoom_target;
            }
            self.adjust_position_for_zoom(old_zoom, self.zoom);
        }
    }

    /// Shift camera position so zoom_anchor stays at the same screen location.
    pub(crate) fn adjust_position_for_zoom(&mut self, old_zoom: f32, new_zoom: f32) {
        if (old_zoom - new_zoom).abs() < 0.00001 { return; }
        let has_explicit_anchor = self.zoom_anchor.is_some();
        let anchor = self.zoom_anchor.unwrap_or_else(|| {
            let (vw, vh) = self.viewport_size;
            (
                self.position.0 + vw / (2.0 * old_zoom),
                self.position.1 + vh / (2.0 * old_zoom),
            )
        });
        let ratio = old_zoom / new_zoom;
        self.position.0 = anchor.0 - (anchor.0 - self.position.0) * ratio;
        self.position.1 = anchor.1 - (anchor.1 - self.position.1) * ratio;

        // Only clamp when using the default center anchor; an explicit
        // anchor takes priority over world bounds (e.g. zoom-out with
        // a ground anchor requires negative position.y).
        if !has_explicit_anchor {
            let (visible_w, visible_h) = self.visible_size();
            self.position.0 = self.position.0.clamp(0.0, (self.world_size.0 - visible_w).max(0.0));
            self.position.1 = self.position.1.clamp(0.0, (self.world_size.1 - visible_h).max(0.0));
        }
    }
}