# Quartz 2D Game Engine

<p align="center">
  <img src="./logo/quartz.png" alt="Quartz Logo" width="400"/>
</p>

<p align="center">
  A Rust 2D game engine for building fullscreen, event-driven applications — from games to tools — with a clean object model, live physics, and hot-reloadable everything.
</p>

---

## What it is

Quartz sits on top of Prism and gives you a `Canvas` you can fill with `GameObject`s, wire up with events and callbacks, and run at a fixed 60 Hz tick. It handles the loop, the input, the physics, the camera, and the renderer. You handle the logic.

It is designed for projects where you own the full screen and want direct control over every pixel — IDE shells, game prototypes, creative tools, dashboards, simulations.

---

## Concepts

**Canvas** is the root. Everything lives on it — objects, variables, scenes, callbacks, and physics. You create one with a mode that sets the coordinate system: `Landscape` (3840×2160 virtual), `Portrait` (2160×3840 virtual), or `Fullscreen` (actual window size, no letterboxing).

**GameObjects** are the primitive. Each has a position, size, drawable, optional physics, tags, and a render layer. They are built with a fluent builder and added to the canvas by name.

**Canvas variables** are a typed key-value store on the canvas — the idiomatic way to share state between callbacks without reaching for external globals or `Arc<Mutex<_>>`.

**The tick loop** runs at ~60 Hz and fires in a fixed order each frame: `on_update` callbacks → input events → object update (gravity, position, animation) → physics step → camera transform.

---

## Capabilities

**Rendering** — GameObjects with images, animated sprites, multi-span mixed-style text, tint, glow, and scissor clipping.

**Input** — Mouse (press, release, move, scroll) and keyboard callbacks with full modifier key support.

**Physics** — Crystalline, an impulse-based rigid body solver. Configurable presets (platformer, arcade, realistic). Particle emitters with named presets (fire, sparks, smoke, explosion, and more). Emitters support per-particle shape (`Circle`, `Ellipse`, `Square`, `Rect`, `Soft`), size and color animation over lifetime, velocity-aligned rotation, and sub-frame position interpolation to fill gaps at high speed. Planet gravity with linear and inverse-square falloff.

**Camera** — Follows a target object with configurable lerp speed. Smooth zoom with cursor-anchored pivot. World↔screen coordinate conversion.

**Text** — Multi-span text with per-span font, size, color, and line height. Auto-scaled font sizes for virtual resolution. Word wrap.

**Hot Reload** — File watchers poll every 0.5 s. Images and animations reload automatically. Raw byte and typed source-parsed watchers for configs and settings structs.

**Scenes** — Named scene graphs with enter and exit hooks for swapping full object/event graphs at runtime.

**Object Pools** — Pre-allocated pools for zero-allocation spawning of bullets, particles, or any frequently reused entity.

**Audio** — `play_sound` with volume, pitch, pan, looping, and fade controls.

**Alignment & Screen Pinning** — `center_at(cx, cy)` on the builder positions objects by centre rather than top-left. `screen_space()` / `pin_*(anchor, offset)` builder methods pin HUD objects to normalised viewport anchors (`pin_top_left`, `pin_top_center`, `pin_bottom_right`, etc.) — the engine repositions them automatically every frame, so manual per-tick `obj.position = ...` reassignments are gone. `fill_screen()` places fullscreen overlays at (0, 0) with a single call. `rotate_around_center()` / `with_pivot(px, py)` let you change the rotation pivot without jitter. For slope-facing behavior, use `align_to_slope()` / `align_to_slope_speed(...)` on builders and `Action::set_align_to_slope(...)` / `Action::set_align_to_slope_speed(...)` at runtime.

**Shared State** — `Shared<T>`, a lightweight `Rc<RefCell<T>>` wrapper with a `.changed()` flag for coordinating state across closures.

**Entropy** — Seeded RNG with range, chance, pick, and position helpers.

**Lerp** — Bounded interpolator with tick, nudge, and snap.

---

## Getting started

```rust
// src/lib.rs
use quartz::*;

struct App;

impl App {
    fn new(ctx: &mut Context, assets: &Assets) -> Scene {
        let mut cv = Canvas::new(ctx, CanvasMode::Landscape);
        // build objects, register callbacks, wire up physics
        cv.into_scene()
    }
}

ramp::run!(App);
```

> **API reference is in active development** and is maintained separately from this README. See the internal API spec for current methods.

---

## License

See `LICENSE` for details.
