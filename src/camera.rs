use crate::Target;
use crate::entropy::Entropy;
use prism::canvas::Color;

// ── Flash Effect Configuration ────────────────────────────────────────────────

/// How the flash brightness evolves over time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlashMode {
    /// Starts at full intensity, fades to zero. (default, original behavior)
    FadeOut,
    /// Ramps up to peak brightness, then fades back down — looks like a camera flash.
    Pulse,
}

impl Default for FlashMode {
    fn default() -> Self { FlashMode::FadeOut }
}

/// Easing curve for the flash brightness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlashEase {
    /// Straight linear ramp.
    Linear,
    /// Smooth ease-in-out (sine curve).
    Smooth,
    /// Fast attack, slow exponential decay — punchy impact feel.
    Sharp,
}

impl Default for FlashEase {
    fn default() -> Self { FlashEase::Linear }
}

// ── Camera Effect Structs ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ShakeEffect {
    pub intensity:  f32,
    pub duration:   f32,
    pub elapsed:    f32,
    pub offset:     (f32, f32),
}

#[derive(Debug, Clone)]
pub struct FlashEffect {
    pub color:        Color,
    pub duration:     f32,
    pub elapsed:      f32,
    /// Peak alpha multiplier (0.0–1.0). Default 1.0 = full brightness.
    pub intensity:    f32,
    /// How brightness evolves over time.
    pub mode:         FlashMode,
    /// Easing curve shape.
    pub ease:         FlashEase,
    /// Seconds to hold at peak brightness before decay begins. Default 0.0.
    pub freeze_frame: f32,
}

#[derive(Debug, Clone)]
pub struct ZoomPunchEffect {
    pub amount:   f32,
    pub duration: f32,
    pub elapsed:  f32,
}

#[derive(Debug, Clone, Default)]
pub struct CameraEffects {
    pub shake:      Option<ShakeEffect>,
    pub flash:      Option<FlashEffect>,
    pub zoom_punch: Option<ZoomPunchEffect>,
    pub(crate) rng: Option<Entropy>,
}

impl CameraEffects {
    pub fn update(&mut self, dt: f32) {
        // ── Shake ──
        if let Some(ref mut s) = self.shake {
            s.elapsed += dt;
            if s.elapsed >= s.duration {
                self.shake = None;
            } else {
                let decay = 1.0 - (s.elapsed / s.duration);
                let rng = self.rng.get_or_insert_with(Entropy::new);
                s.offset = (
                    rng.range(-s.intensity, s.intensity) * decay,
                    rng.range(-s.intensity, s.intensity) * decay,
                );
            }
        }

        // ── Flash ──
        if let Some(ref mut f) = self.flash {
            f.elapsed += dt;
            if f.elapsed >= f.duration + f.freeze_frame {
                self.flash = None;
            }
        }

        // ── Zoom punch ──
        if let Some(ref mut z) = self.zoom_punch {
            z.elapsed += dt;
            if z.elapsed >= z.duration {
                self.zoom_punch = None;
            }
        }
    }

    pub fn shake_offset(&self) -> (f32, f32) {
        self.shake.as_ref().map_or((0.0, 0.0), |s| s.offset)
    }

    pub fn zoom_punch_amount(&self) -> f32 {
        if let Some(ref z) = self.zoom_punch {
            let t = (z.elapsed / z.duration).min(1.0);
            // Quick pop in, smooth decay out.
            let curve = 1.0 - (t * std::f32::consts::PI).sin().abs();
            z.amount * (1.0 - curve)
        } else {
            0.0
        }
    }

    pub fn flash_alpha(&self) -> f32 {
        self.flash.as_ref().map_or(0.0, |f| compute_flash_alpha(f))
    }

    pub fn flash_color(&self) -> Option<(Color, f32)> {
        self.flash.as_ref().map(|f| {
            (f.color, compute_flash_alpha(f))
        })
    }

    /// Returns the ready-to-render overlay color with baked-in alpha.
    /// Use this for automatic flash rendering.
    pub fn flash_overlay_color(&self) -> Option<Color> {
        self.flash.as_ref().map(|f| {
            let alpha = compute_flash_alpha(f);
            let a = (alpha * f.color.3 as f32).round().min(255.0).max(0.0) as u8;
            if a == 0 { return None; }
            Some(Color(f.color.0, f.color.1, f.color.2, a))
        }).flatten()
    }
}

// ── Flash alpha computation ───────────────────────────────────────────────────

fn compute_flash_alpha(f: &FlashEffect) -> f32 {
    let total = f.duration + f.freeze_frame;
    if total <= 0.0 { return 0.0; }
    let t_raw = (f.elapsed / total).min(1.0);

    // freeze_frame: hold at peak for freeze_frame seconds, then decay
    // t_eff is the decay progress (0.0 = peak, 1.0 = end) after the freeze
    let freeze_frac = f.freeze_frame / total;
    let t_eff = if t_raw <= freeze_frac {
        0.0  // still in freeze window — full peak
    } else {
        ((t_raw - freeze_frac) / (1.0 - freeze_frac)).min(1.0)
    };

    let raw_alpha = match f.mode {
        FlashMode::FadeOut => {
            // Original behavior: starts at 1.0, decays to 0.0
            1.0 - t_eff
        }
        FlashMode::Pulse => {
            // Ramp up to peak in first 25% of decay, then fade back down
            if t_eff < 0.25 {
                t_eff / 0.25
            } else {
                1.0 - ((t_eff - 0.25) / 0.75)
            }
        }
    };

    let eased = match f.ease {
        FlashEase::Linear => raw_alpha,
        FlashEase::Smooth => {
            // Smooth sine ease-in-out
            0.5 - 0.5 * (raw_alpha * std::f32::consts::PI).cos()
        }
        FlashEase::Sharp => {
            // Quadratic for fast attack, exponential decay feel
            raw_alpha * raw_alpha
        }
    };

    (eased * f.intensity).clamp(0.0, 1.0)
}

// ── Camera ────────────────────────────────────────────────────────────────────

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

    // ── Camera effects ────────────────────────────────────────────────────────
    pub effects: CameraEffects,

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
            effects:         CameraEffects::default(),
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

    // ── Camera effects API ────────────────────────────────────────────────────

    /// Start a camera shake. Intensity is in world-space pixels, duration in seconds.
    pub fn shake(&mut self, intensity: f32, duration: f32) {
        self.effects.shake = Some(ShakeEffect {
            intensity,
            duration,
            elapsed: 0.0,
            offset: (0.0, 0.0),
        });
    }

    /// Quick default shake — 6px intensity, 0.3s.
    pub fn quick_shake(&mut self) {
        self.shake(6.0, 0.3);
    }

    /// Screen flash. Color fades from full alpha to zero over `duration` seconds.
    /// Uses default settings: FadeOut mode, Linear ease, full intensity, no freeze.
    pub fn flash(&mut self, color: Color, duration: f32) {
        self.effects.flash = Some(FlashEffect {
            color,
            duration: duration.max(0.01),
            elapsed: 0.0,
            intensity: 1.0,
            mode: FlashMode::FadeOut,
            ease: FlashEase::Linear,
            freeze_frame: 0.0,
        });
    }

    /// Screen flash with full control over the effect.
    ///
    /// `mode`:         FadeOut (default) or Pulse (ramp-up then fade)
    /// `ease`:         Linear, Smooth (sine), or Sharp (fast attack)
    /// `intensity`:    Peak alpha multiplier (0.0–1.0, default 1.0)
    /// `freeze_frame`: Seconds to hold at peak before decay (default 0.0)
    pub fn flash_with(
        &mut self,
        color:        Color,
        duration:     f32,
        mode:         FlashMode,
        ease:         FlashEase,
        intensity:    f32,
        freeze_frame: f32,
    ) {
        self.effects.flash = Some(FlashEffect {
            color,
            duration: duration.max(0.01),
            elapsed: 0.0,
            intensity: intensity.clamp(0.0, 1.0),
            mode,
            ease,
            freeze_frame: freeze_frame.max(0.0),
        });
    }

    /// Zoom punch — a quick additive zoom burst that decays.
    /// `amount` is how much extra zoom (e.g. 0.15 = 15% pop).
    pub fn zoom_punch(&mut self, amount: f32, duration: f32) {
        self.effects.zoom_punch = Some(ZoomPunchEffect {
            amount,
            duration: duration.max(0.01),
            elapsed: 0.0,
        });
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