# Quartz Reintegration Guide — synful_slopes → main

> Line-by-line plan for restoring all missing features from the synful_slopes
> branch into the current refactored quartz main branch.
>
> **Source of truth:** `quartz_synful_slopes/src/` (old, feature-complete)
> **Target:**          `quartz/src/` (current main, partially stripped)
>
> Date: 2026-03-26

---

## Table of Contents

1. [Executive Summary — What's Missing](#1-executive-summary)
2. [Phase 1 — game_vars on Canvas](#2-phase-1--game_vars-on-canvas)
3. [Phase 2 — expr.rs Parser Module](#3-phase-2--exprrs-parser-module)
4. [Phase 3 — Action Variants](#4-phase-3--action-variants)
5. [Phase 4 — Condition Variants](#5-phase-4--condition-variants)
6. [Phase 5 — Action::run() Wiring in canvas.rs](#6-phase-5--actionrun-wiring-in-canvasrs)
7. [Phase 6 — Condition::evaluate_condition() Wiring](#7-phase-6--conditionevaluate_condition-wiring)
8. [Phase 7 — GameObject Fields (rotation/slope/surface)](#8-phase-7--gameobject-fields)
9. [Phase 8 — GameObjectBuilder Extensions](#9-phase-8--gameobjectbuilder-extensions)
10. [Phase 9 — Collision System (slopes/rotation)](#10-phase-9--collision-system)
11. [Phase 10 — BoundaryCollision Wiring](#11-phase-10--boundarycollision-wiring)
12. [Phase 11 — Pause/Resume](#12-phase-11--pauseresume)
13. [Phase 12 — Re-exports and lib.rs Updates](#13-phase-12--re-exports-and-librs-updates)
14. [Phase 13 — Helper Methods on Action/Condition](#14-phase-13--helper-methods)
15. [Phase 14 — Verification](#15-phase-14--verification)
16. [Dependency Graph](#16-dependency-graph)

---

## 1. Executive Summary

### What current `quartz/src/` HAS (working):
- Canvas struct with ObjectStore, InputState, MouseState, CallbackStore, SceneManager, Camera, Entropy
- 13 Action variants: ApplyMomentum, SetMomentum, Spawn, SetResistance, Remove, TransferMomentum, SetAnimation, Teleport, Show, Hide, Toggle, Conditional, Custom
- 10 Condition variants: Always, KeyHeld, KeyNotHeld, Collision, NoCollision, And, Or, Not, IsVisible, IsHidden
- 14 GameEvent variants (all wired except BoundaryCollision)
- Full mouse event system (press/release/enter/leave/over/scroll/move)
- Value/Expr types defined in value.rs (standalone — not connected to Canvas)
- AnimatedSprite with rotation support
- Lerp, Entropy, Text/SpanSpec, SoundOptions

### What current `quartz/src/` is MISSING (was in synful_slopes):

#### Entirely Missing Systems:
| System | Description |
|--------|-------------|
| **game_vars** | `HashMap<String, Value>` on Canvas + set_var/get_var/has_var/remove_var/resolve/modify_var + 13 type-specific getters + 12 type-specific modifiers |
| **expr.rs** | Full expression parser (Lexer → Parser → Actions/Conditions from strings) |
| **paused field** | Canvas.paused + pause()/resume()/is_paused() + tick-loop guard |

#### Missing Action Variants (15 variants absent):
| Action | What It Does |
|--------|-------------|
| `SetVar { name, value: Expr }` | Set a game variable by resolving an expression |
| `ModVar { name, op: MathOp, operand: Expr }` | Modify a game variable with a math operation |
| `Multi(Vec<Action>)` | Execute multiple actions in sequence |
| `PlaySound { path, options }` | Play a sound as a declarative action |
| `SetGravity { target, value }` | Set gravity on target objects |
| `SetSize { target, value }` | Resize target objects (updates scaled_size + shape) |
| `AddTag { target, tag }` | Add a tag to target objects at runtime |
| `RemoveTag { target, tag }` | Remove a tag from target objects at runtime |
| `SetText { target, text }` | Set drawable text on target objects |
| `Expr(String)` | Parse and execute an expression string as actions |
| `SetRotation { target, value }` | Set absolute rotation on target objects |
| `SetSlope { target, left, right, auto_rotate }` | Configure slope geometry |
| `AddRotation { target, value }` | Add to current rotation |
| `ApplyRotation { target, value }` | Add rotation momentum |
| `SetSurfaceNormal { target, nx, ny }` | Set surface normal for collision response |

#### Missing Condition Variants (5 variants absent):
| Condition | What It Does |
|-----------|-------------|
| `Compare(Expr, CompOp, Expr)` | Compare two resolved expressions |
| `VarExists(String)` | Check if a game variable exists |
| `Grounded(Target)` | Check if object is resting on a platform surface |
| `Expr(String)` | Parse and evaluate a condition string at runtime |
| `HasTag(Target, String)` | Check if target object has a specific tag |

#### Missing GameObject Fields (7 fields absent):
| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `rotation` | `f32` | `0.0` | Visual rotation in degrees |
| `slope` | `Option<(f32, f32)>` | `None` | (left_offset, right_offset) for slope platforms |
| `one_way` | `bool` | `false` | Allow pass-through from below |
| `surface_velocity` | `Option<f32>` | `None` | Conveyor belt tangent speed |
| `rotation_momentum` | `f32` | `0.0` | Angular velocity |
| `rotation_resistance` | `f32` | `0.0` | Angular friction |
| `surface_normal` | `(f32, f32)` | `(0.0, -1.0)` | Unit vector for collision response |

#### Missing GameObjectBuilder Methods:
| Method | What It Does |
|--------|-------------|
| `rotation(degrees)` | Set initial rotation |
| `slope(left, right)` | Set slope offsets |
| `slope_auto_rotation(left, right)` | Set slope with auto-computed rotation |
| `one_way()` | Mark as one-way platform |
| `surface_velocity(vx)` | Set conveyor belt speed |
| `rotation_resistance(r)` | Set angular friction |
| `floor()` | Alias for platform() |
| `ceiling()` | Platform with normal (0, 1) |
| `wall_left()` | Platform with normal (1, 0) |
| `wall_right()` | Platform with normal (-1, 0) |
| `surface(nx, ny)` | Custom surface normal (auto-normalized) |

#### Missing GameObject Methods:
| Method | What It Does |
|--------|-------------|
| `apply_rotation_momentum()` | rotation += rotation_momentum, apply resistance |
| `sync_rotation_normal()` | Recompute surface_normal from rotation angle |
| `slope_surface_y(world_x)` | Interpolate height along a slope |
| `rotation_from_slope()` | Compute angle from slope offsets |
| `surface_normal_at(world_x)` | Geometric normal (slope-aware) |
| `slope_aabb()` | Axis-aligned bounding box of slope |

#### Collision System Gaps:
- Current: simple AABB + platform top-landing only
- Missing: slope collision, rotation-aware surfaces, one-way platforms, surface velocity transfer, normal-based push resolution

---

## 2. Phase 1 — game_vars on Canvas

**WHY:** game_vars is the backbone of the expression system. Every Expr resolves
variables from this HashMap. Every SetVar/ModVar/Compare/VarExists action and
condition reads or writes to it. Without it, the entire expression system has
no storage layer.

**WHERE:** `quartz/src/lib.rs` (Canvas struct) + `quartz/src/canvas.rs` (methods)

### Step 1.1 — Add the field to Canvas struct

**File:** `quartz/src/lib.rs`

In the `Canvas` struct definition, add two new fields:

```rust
// BEFORE (current):
pub struct Canvas {
    pub(crate) layout:        CanvasLayout,
    pub(crate) store:         ObjectStore,
    pub(crate) input:         InputState,
    pub        mouse:         MouseState,
    pub(crate) callbacks:     CallbackStore,
    pub(crate) scene_manager: SceneManager,
    pub(crate) active_camera: Option<Camera>,
    pub        entropy:       Entropy,
}

// AFTER:
pub struct Canvas {
    pub(crate) layout:        CanvasLayout,
    pub(crate) store:         ObjectStore,
    pub(crate) input:         InputState,
    pub        mouse:         MouseState,
    pub(crate) callbacks:     CallbackStore,
    pub(crate) scene_manager: SceneManager,
    pub(crate) active_camera: Option<Camera>,
    pub        entropy:       Entropy,
    pub        game_vars:     HashMap<String, Value>,  // ← NEW
    paused:                   bool,                    // ← NEW
}
```

**ALSO:** Add `use std::collections::HashMap;` and `use crate::value::Value;` to
the imports at the top of lib.rs.

### Step 1.2 — Initialize in Canvas::new()

**File:** `quartz/src/canvas.rs`

In `Canvas::new()`, add the two new fields to the constructed Self:

```rust
// Add to the Self { ... } block:
    game_vars: HashMap::new(),
    paused:    false,
```

**ALSO:** Add `use std::collections::HashMap;` and `use crate::value::{Value, Expr, resolve_expr, apply_op, compare_operands, MathOp};` to the imports at the top of canvas.rs.

### Step 1.3 — Add game_vars methods to Canvas

**File:** `quartz/src/canvas.rs`

Add a new `impl Canvas` block (or append to a existing one) with all
variable access methods. These go on Canvas because game_vars lives on Canvas,
not ObjectStore:

```rust
// ── game_vars API ──────────────────────────────────────────

pub fn set_var(&mut self, name: impl Into<String>, value: impl Into<Value>) {
    self.game_vars.insert(name.into(), value.into());
}

pub fn get_var(&self, name: &str) -> Option<Value> {
    self.game_vars.get(name).cloned()
}

pub fn has_var(&self, name: &str) -> bool {
    self.game_vars.contains_key(name)
}

pub fn remove_var(&mut self, name: &str) {
    self.game_vars.remove(name);
}

pub fn resolve(&self, expr: &Expr) -> Option<Value> {
    resolve_expr(expr, &self.game_vars)
}

pub fn modify_var(&mut self, name: &str, f: impl FnOnce(Value) -> Value) {
    if let Some(val) = self.game_vars.remove(name) {
        self.game_vars.insert(name.to_string(), f(val));
    }
}
```

Then add the type-specific getters (get_u8, get_i32, get_f32, get_bool,
get_str, etc.) and type-specific modifiers (modify_u8, modify_i32, etc.).
These are convenience wrappers — see synful_slopes/src/apis.rs lines 681–780
for the exact implementations. Each one is ~3 lines.

**WHY type-specific helpers exist:** Without them, every caller has to
`match canvas.get_var("score") { Some(Value::I32(v)) => v, _ => 0 }`.
The helpers reduce this to `canvas.get_i32("score").unwrap_or(0)`.

---

## 3. Phase 2 — expr.rs Parser Module

**WHY:** The expression parser enables `Action::Expr("score += 1")` and
`Condition::Expr("score > 10 && lives > 0")` — string-based scripting at
runtime. This is how the game editor will let users write logic without
Rust code. It's also how `parse_action` and `parse_condition` work.

**WHERE:** New file `quartz/src/expr.rs`

### Step 2.1 — Create the file

Copy the *entire* contents of `quartz_synful_slopes/src/expr.rs` into a new
file at `quartz/src/expr.rs`. The file is ~350 lines containing:

- `Token` enum (all lexer tokens: Int, Float, Bool, Str, Ident, operators)
- `Lexer` struct with `tokenize()` (converts string → tokens)
- `Parser` struct with:
  - `parse_expr()` → `parse_add()` → `parse_mul()` → `parse_unary()` → `parse_primary()` (expression precedence ladder)
  - `parse_condition()` → `parse_or()` → `parse_and()` → `parse_not()` → `parse_compare()` (condition tree)
  - `parse_stmts()` → `parse_stmt()` (semicolon-separated variable assignments)
- Two public entry points:
  - `pub fn parse_condition(input: &str) -> Result<Condition, String>`
  - `pub fn parse_action(input: &str) -> Result<Vec<Action>, String>`

**IMPORTANT:** The imports at the top of expr.rs reference `Action` and
`Condition` from `crate`. These types must already have the `SetVar`, `ModVar`,
`Compare`, and `Expr` variants before expr.rs can compile. This means
**Phase 3 and Phase 4 must be done before or simultaneously with Phase 2.**

### Step 2.2 — Register the module

**File:** `quartz/src/lib.rs`

Add the module declaration with the other mod statements:

```rust
pub mod expr;
```

### Step 2.3 — Re-export the public functions

**File:** `quartz/src/lib.rs`

Add to the re-exports section:

```rust
pub use expr::{parse_condition, parse_action};
```

---

## 4. Phase 3 — Action Variants

**WHY:** The current Action enum has 13 variants. synful_slopes had 28.
The 15 missing variants represent all the declarative features that make
quartz usable as a game engine (variable manipulation, sound playback,
physics control, tag management, text setting, expression scripting,
rotation/slope/surface control).

**WHERE:** `quartz/src/types.rs` (Action enum definition)

### Step 3.1 — Add all missing variants to the Action enum

After the existing `Custom { name: String }` variant, add:

```rust
    // ── Variable system (requires game_vars on Canvas) ──
    SetVar {
        name:  String,
        value: Expr,
    },
    ModVar {
        name:    String,
        op:      MathOp,
        operand: Expr,
    },

    // ── Batch execution ──
    Multi(Vec<Action>),

    // ── Sound ──
    PlaySound {
        path:    String,
        options: SoundOptions,
    },

    // ── Physics / object mutation ──
    SetGravity {
        target: Target,
        value:  f32,
    },
    SetSize {
        target: Target,
        value:  (f32, f32),
    },

    // ── Tag mutation ──
    AddTag {
        target: Target,
        tag:    String,
    },
    RemoveTag {
        target: Target,
        tag:    String,
    },

    // ── Visual ──
    SetText {
        target: Target,
        text:   Text,
    },

    // ── Expression scripting ──
    Expr(String),

    // ── Rotation / slope / surface ──
    SetRotation {
        target: Target,
        value:  f32,
    },
    SetSlope {
        target:       Target,
        left_offset:  f32,
        right_offset: f32,
        auto_rotate:  bool,
    },
    AddRotation {
        target: Target,
        value:  f32,
    },
    ApplyRotation {
        target: Target,
        value:  f32,
    },
    SetSurfaceNormal {
        target: Target,
        nx:     f32,
        ny:     f32,
    },
```

### Step 3.2 — Add required imports to types.rs

```rust
use crate::value::{Expr, MathOp};
use crate::sound::SoundOptions;
use prism::canvas::Text;
```

### Step 3.3 — Update the Clone/Debug impls for GameEvent

The `GameEvent::Clone` and `GameEvent::Debug` manual implementations reference
`action` on the existing variants. The return type of `action()` remains the
same so no changes there. However, if you derive Clone on GameEvent instead of
manual impl, ensure Action's new variants all derive Clone (they do — Expr,
SoundOptions, Value, MathOp, Text all derive Clone).

### Step 3.4 — Update GameEvent::action() if needed

The `action()` method on GameEvent already handles all variants that have an
`action` field. Since no new GameEvent variants are being added in this phase,
the existing match arms remain correct. The new Action variants are just new
*values* that the existing `action` field can hold.

---

## 5. Phase 4 — Condition Variants

**WHY:** The current Condition enum has 10 variants. synful_slopes had 15.
The 5 missing variants enable expression-based comparisons, variable existence
checks, ground detection, string-parsed conditions, and runtime tag queries.

**WHERE:** `quartz/src/types.rs` (Condition enum definition)

### Step 4.1 — Add all missing variants

After `IsHidden(Target)`, add:

```rust
    // ── Expression-based ──
    Compare(Expr, CompOp, Expr),
    VarExists(String),

    // ── Physics ──
    Grounded(Target),

    // ── Runtime string parsing ──
    Expr(String),

    // ── Tag query ──
    HasTag(Target, String),
```

### Step 4.2 — Add required imports

```rust
use crate::value::{Expr, CompOp};  // add CompOp to the existing Expr import
```

---

## 6. Phase 5 — Action::run() Wiring in canvas.rs

**WHY:** Adding variants to the enum is not enough — the `Canvas::run()`
method has a `match action { ... }` that must handle every variant. Without
a match arm, the compiler will error (non-exhaustive match). More importantly,
the match arm is where the actual *behavior* lives.

**WHERE:** `quartz/src/canvas.rs`, in `pub fn run(&mut self, action: Action)`

### Step 5.1 — Add match arms for all 15 new Action variants

After the existing `Action::Custom { name }` arm, add these arms **in order**:

```rust
Action::SetVar { name, value } => {
    // Resolve the Expr against game_vars, then store the result.
    // If the Expr references a missing variable, the set is silently skipped
    // (same behavior as synful_slopes — no partial writes).
    if let Some(resolved) = resolve_expr(&value, &self.game_vars) {
        self.game_vars.insert(name, resolved);
    }
}
Action::ModVar { name, op, operand } => {
    // Read current value, resolve operand, apply math op, write back.
    // If the var doesn't exist or types don't match, nothing happens.
    if let Some(current) = self.game_vars.get(&name).cloned() {
        if let Some(resolved) = resolve_expr(&operand, &self.game_vars) {
            if let Some(new_val) = apply_op(&current, &resolved, &op) {
                self.game_vars.insert(name, new_val);
            }
        }
    }
}
Action::Multi(actions) => {
    // Execute a list of actions in order. This is how you bundle
    // multiple side effects into a single event response.
    for action in actions {
        self.run(action);
    }
}
Action::PlaySound { path, options } => {
    // Delegate to the existing sound system.
    self.play_sound_with(&path, options);
}
Action::SetGravity { target, value } => {
    // Directly set the gravity field on all matching objects.
    self.store.apply_to_targets(&target, |obj| obj.gravity = value);
}
Action::SetSize { target, value } => {
    // Update size + scaled_size + redraw the image shape.
    // Must also update scaled_size because the rendering pipeline
    // reads scaled_size, not size, for actual draw dimensions.
    let scale = self.layout.scale.get();
    let indices = self.store.get_indices(&target);
    for idx in indices {
        if let Some(obj) = self.store.objects.get_mut(idx) {
            obj.size = value;
            obj.scaled_size.set((value.0 * scale, value.1 * scale));
            obj.update_image_shape();
        }
    }
}
Action::AddTag { target, tag } => {
    // Add a tag at runtime. Must also update the ObjectStore's
    // tag_to_indices map so Target::ByTag lookups find this object.
    let indices = self.store.get_indices(&target);
    for idx in indices {
        if let Some(obj) = self.store.objects.get_mut(idx) {
            if !obj.tags.contains(&tag) {
                obj.tags.push(tag.clone());
                self.store.tag_to_indices
                    .entry(tag.clone())
                    .or_default()
                    .push(idx);
            }
        }
    }
}
Action::RemoveTag { target, tag } => {
    // Remove a tag at runtime. Must also clean up tag_to_indices.
    let indices = self.store.get_indices(&target);
    for idx in indices {
        if let Some(obj) = self.store.objects.get_mut(idx) {
            obj.tags.retain(|t| t != &tag);
        }
        if let Some(v) = self.store.tag_to_indices.get_mut(&tag) {
            v.retain(|&i| i != idx);
        }
    }
}
Action::SetText { target, text } => {
    // Replace the drawable on target objects with a Text drawable.
    let indices = self.store.get_indices(&target);
    for idx in indices {
        if let Some(obj) = self.store.objects.get_mut(idx) {
            obj.set_drawable(text.clone());
        }
    }
}
Action::Expr(src) => {
    // Parse a string into Action(s) at runtime and execute them.
    // parse_action returns Vec<Action> because a single string can
    // contain semicolon-separated statements like "score += 1; lives -= 1".
    match parse_action(&src) {
        Ok(actions) => {
            for action in actions {
                self.run(action);
            }
        }
        Err(e) => {
            // In debug builds, panic so the developer sees the parse error.
            // In release, silently skip (game shouldn't crash from bad scripts).
            debug_assert!(false,
                "[Action::Expr] parse error in \"{src}\": {e}\n\
                 Use Action::expr() to catch this at setup time.");
        }
    }
}
Action::SetRotation { target, value } => {
    self.store.apply_to_targets(&target, |obj| obj.rotation = value);
}
Action::SetSlope { target, left_offset, right_offset, auto_rotate } => {
    let indices = self.store.get_indices(&target);
    for idx in indices {
        if let Some(obj) = self.store.objects.get_mut(idx) {
            obj.slope = Some((left_offset, right_offset));
            if auto_rotate {
                obj.rotation = obj.rotation_from_slope();
            }
        }
    }
}
Action::AddRotation { target, value } => {
    self.store.apply_to_targets(&target, |obj| obj.rotation += value);
}
Action::ApplyRotation { target, value } => {
    // This adds to rotation_momentum, not rotation directly.
    // The actual rotation change happens in update_objects() each tick.
    self.store.apply_to_targets(&target, |obj| obj.rotation_momentum += value);
}
Action::SetSurfaceNormal { target, nx, ny } => {
    // Auto-normalize the input vector to ensure it's a unit vector.
    let len = (nx * nx + ny * ny).sqrt().max(0.001);
    let (nx, ny) = (nx / len, ny / len);
    self.store.apply_to_targets(&target, |obj| obj.surface_normal = (nx, ny));
}
```

### Step 5.2 — Add the parse_action import

At the top of canvas.rs, add:
```rust
use crate::expr::parse_action;
```

(The value imports from Step 1.2 should already cover resolve_expr, apply_op, etc.)

---

## 7. Phase 6 — Condition::evaluate_condition() Wiring

**WHY:** Same reason as Actions — the Condition enum now has 5 new variants
that need match arms in `evaluate_condition()`.

**WHERE:** `quartz/src/canvas.rs`, in `pub(crate) fn evaluate_condition()`

### Step 6.1 — Add match arms for all 5 new Condition variants

After the existing `Condition::IsHidden` arm, add:

```rust
Condition::Compare(left, op, right) => {
    // Resolve both Exprs against game_vars, then compare.
    // If either side fails to resolve (missing var, type mismatch),
    // the condition evaluates to false — never panics.
    match (
        resolve_expr(left, &self.game_vars),
        resolve_expr(right, &self.game_vars),
    ) {
        (Some(l), Some(r)) => compare_operands(&l, &r, op).unwrap_or(false),
        _ => false,
    }
}
Condition::VarExists(name) => {
    // Simple containment check on the HashMap.
    self.game_vars.contains_key(name.as_str())
}
Condition::Grounded(target) => {
    // Check if the target object is resting on any platform's surface.
    // "Grounded" = bottom of object is within 2px of a platform's
    // surface AND the object is not moving upward (momentum.y >= 0)
    // AND the platform's surface faces upward (effective ny < -0.3).
    self.store.get_indices(target).iter().any(|&idx| {
        if let Some(obj) = self.store.objects.get(idx) {
            let obj_bottom = obj.position.1 + obj.size.1;
            let obj_center_x = obj.position.0 + obj.size.0 * 0.5;
            self.store.objects.iter().any(|other| {
                if !other.is_platform { return false; }
                let (_, eff_ny) = other.surface_normal_at(obj_center_x);
                if eff_ny >= -0.3 { return false; }
                if obj.position.0 + obj.size.0 <= other.position.0 { return false; }
                if obj.position.0 >= other.position.0 + other.size.0 { return false; }
                if obj.momentum.1 < 0.0 { return false; }
                let surface_y = other.slope_surface_y(obj_center_x);
                (obj_bottom - surface_y).abs() < 2.0
            })
        } else {
            false
        }
    })
}
Condition::Expr(src) => {
    // Parse a condition string at runtime and evaluate it.
    match parse_condition(src) {
        Ok(condition) => self.evaluate_condition(&condition),
        Err(e) => {
            debug_assert!(false,
                "[Condition::Expr] parse error in \"{src}\": {e}\n\
                 Use Condition::expr() to catch this at setup time.");
            false
        }
    }
}
Condition::HasTag(target, tag) => {
    // Check if any object matching the target has the given tag.
    self.store.get_indices(target).iter().any(|&idx| {
        self.store.objects.get(idx)
            .map_or(false, |obj| obj.tags.contains(tag))
    })
}
```

### Step 6.2 — Add the parse_condition import

```rust
use crate::expr::parse_condition;
```

---

## 8. Phase 7 — GameObject Fields

**WHY:** The rotation/slope/surface fields on GameObject are required by:
- `Action::SetRotation/SetSlope/AddRotation/ApplyRotation/SetSurfaceNormal`
- `Condition::Grounded` (reads surface_normal, slope, is_platform)
- The upgraded collision system (Phase 9)
- `GameObjectBuilder` methods (Phase 8)

Without these fields, none of the above will compile.

**WHERE:** `quartz/src/object.rs`

### Step 7.1 — Add fields to the GameObject struct

After `pub layer: Option<u32>`, add:

```rust
    pub rotation:             f32,               // visual rotation in degrees
    pub slope:                Option<(f32, f32)>, // (left_offset, right_offset) for slope platforms
    pub one_way:              bool,               // one-way platform (pass through from below)
    pub surface_velocity:     Option<f32>,         // conveyor belt tangent speed
    pub rotation_momentum:    f32,               // angular velocity (degrees/tick)
    pub rotation_resistance:  f32,               // angular friction multiplier
    pub surface_normal:       (f32, f32),         // unit vector for collision response
```

### Step 7.2 — Initialize in all constructors

In every place that creates a `GameObject { ... }` (there are 3: `new()`,
`new_rect()`, and `GameObjectBuilder::finish()`), add the defaults:

```rust
    rotation:            0.0,
    slope:               None,
    one_way:             false,
    surface_velocity:    None,
    rotation_momentum:   0.0,
    rotation_resistance: 0.0,
    surface_normal:      (0.0, -1.0),  // default: surface faces up
```

**NOTE:** For `GameObjectBuilder::finish()`, these defaults must be overridden
by the builder fields added in Phase 8.

### Step 7.3 — Add new methods to GameObject

After the existing methods, add:

```rust
/// Apply angular velocity and angular friction each tick.
pub fn apply_rotation_momentum(&mut self) {
    if self.rotation_momentum.abs() < 0.001 { return; }
    self.rotation += self.rotation_momentum;
    self.rotation_momentum *= self.rotation_resistance;
    if self.rotation_momentum.abs() < 0.001 {
        self.rotation_momentum = 0.0;
    }
}

/// Recompute surface_normal from the current rotation angle.
/// Call after changing rotation if the object is a platform.
pub fn sync_rotation_normal(&mut self) {
    let rad = self.rotation.to_radians();
    self.surface_normal = (rad.sin(), -rad.cos());
}

/// For slope platforms: interpolate the surface Y at a given world X.
/// Returns the Y coordinate of the slope surface at world_x.
pub fn slope_surface_y(&self, world_x: f32) -> f32 {
    let (left_off, right_off) = self.slope.unwrap_or((0.0, 0.0));
    let left_y  = self.position.1 + left_off;
    let right_y = self.position.1 + right_off;
    let t = ((world_x - self.position.0) / self.size.0).clamp(0.0, 1.0);
    left_y + (right_y - left_y) * t
}

/// Compute the rotation angle implied by the slope offsets.
pub fn rotation_from_slope(&self) -> f32 {
    let (left_off, right_off) = self.slope.unwrap_or((0.0, 0.0));
    let rise = right_off - left_off;
    let run  = self.size.0;
    rise.atan2(run).to_degrees()
}

/// Effective surface normal at a world X position (slope-aware).
/// For non-slope objects, returns self.surface_normal directly.
pub fn surface_normal_at(&self, world_x: f32) -> (f32, f32) {
    if let Some((left_off, right_off)) = self.slope {
        let rise = right_off - left_off;
        let run  = self.size.0;
        let len  = (rise * rise + run * run).sqrt().max(0.001);
        // Tangent is (run, rise), normal is (-rise, run) normalized
        (-rise / len, -run.abs() / len)
    } else {
        self.surface_normal
    }
}

/// Axis-aligned bounding box that encompasses the slope geometry.
pub fn slope_aabb(&self) -> (f32, f32, f32, f32) {
    let (left_off, right_off) = self.slope.unwrap_or((0.0, 0.0));
    let min_y = self.position.1 + left_off.min(right_off);
    let max_y = self.position.1 + self.size.1 + left_off.max(right_off);
    (self.position.0, min_y, self.position.0 + self.size.0, max_y)
}
```

---

## 9. Phase 8 — GameObjectBuilder Extensions

**WHY:** The builder pattern is the primary way users create GameObjects.
Without builder methods for the new fields, users would have to create the
object and then mutate fields directly, which defeats the builder pattern.

**WHERE:** `quartz/src/object.rs`

### Step 8.1 — Add fields to GameObjectBuilder

```rust
pub struct GameObjectBuilder {
    // ... existing fields ...
    rotation:             f32,
    slope:                Option<(f32, f32)>,
    one_way:              bool,
    surface_velocity:     Option<f32>,
    rotation_momentum:    f32,
    rotation_resistance:  f32,
    surface_normal:       (f32, f32),
}
```

Initialize in `GameObject::build()`:
```rust
    rotation:            0.0,
    slope:               None,
    one_way:             false,
    surface_velocity:    None,
    rotation_momentum:   0.0,
    rotation_resistance: 0.0,
    surface_normal:      (0.0, -1.0),
```

### Step 8.2 — Add builder methods

```rust
pub fn rotation(mut self, degrees: f32) -> Self {
    self.rotation = degrees;
    self
}

pub fn slope(mut self, left_offset: f32, right_offset: f32) -> Self {
    self.slope = Some((left_offset, right_offset));
    self
}

pub fn slope_auto_rotation(mut self, left_offset: f32, right_offset: f32) -> Self {
    self.slope = Some((left_offset, right_offset));
    let rise = right_offset - left_offset;
    let run  = self.size.0;  // NOTE: size must be set before calling this
    self.rotation = rise.atan2(run).to_degrees();
    self
}

pub fn one_way(mut self) -> Self {
    self.one_way = true;
    self
}

pub fn surface_velocity(mut self, vx: f32) -> Self {
    self.surface_velocity = Some(vx);
    self
}

pub fn rotation_resistance(mut self, resistance: f32) -> Self {
    self.rotation_resistance = resistance;
    self
}

/// Alias for platform() — clearer name for floor surfaces.
pub fn floor(self) -> Self {
    self.platform()  // normal = (0, -1), already set as default
}

/// Platform that faces downward — for ceilings.
pub fn ceiling(mut self) -> Self {
    self.is_platform = true;
    self.surface_normal = (0.0, 1.0);
    self
}

/// Platform that pushes rightward — left wall.
pub fn wall_left(mut self) -> Self {
    self.is_platform = true;
    self.surface_normal = (1.0, 0.0);
    self
}

/// Platform that pushes leftward — right wall.
pub fn wall_right(mut self) -> Self {
    self.is_platform = true;
    self.surface_normal = (-1.0, 0.0);
    self
}

/// Platform with a custom surface normal (auto-normalized).
pub fn surface(mut self, nx: f32, ny: f32) -> Self {
    self.is_platform = true;
    let len = (nx * nx + ny * ny).sqrt().max(0.001);
    self.surface_normal = (nx / len, ny / len);
    self
}
```

### Step 8.3 — Update finish() to use the new builder fields

In `GameObjectBuilder::finish()`, replace the fixed defaults with the
builder's stored values:

```rust
    rotation:            self.rotation,
    slope:               self.slope,
    one_way:             self.one_way,
    surface_velocity:    self.surface_velocity,
    rotation_momentum:   self.rotation_momentum,
    rotation_resistance: self.rotation_resistance,
    surface_normal:      if self.is_platform { self.surface_normal } else { (0.0, -1.0) },
```

### Step 8.4 — Update platform() builder method

The existing `platform()` method just sets `is_platform = true`. It should
also ensure the surface_normal is (0, -1) (upward-facing):

```rust
pub fn platform(mut self) -> Self {
    self.is_platform = true;
    self.surface_normal = (0.0, -1.0);
    self
}
```

---

## 10. Phase 9 — Collision System

**WHY:** The current handle_collisions() in canvas.rs only does simple AABB
platform-top-landing. synful_slopes had a much more sophisticated system:
slope-aware collision, rotation-aware surfaces, one-way platform filtering,
surface velocity transfer, and normal-based push resolution. This is what
makes slopes, walls, ceilings, and conveyor belts work.

**WHERE:** `quartz/src/canvas.rs`

### Step 9.1 — Add helper functions

Before the `impl Canvas` block, add two free functions that the collision
system needs:

```rust
/// Compute surface Y for a rotated (non-slope) platform at a given world X.
fn rotated_surface_y(plat: &GameObject, world_x: f32) -> f32 {
    let cx = plat.position.0 + plat.size.0 * 0.5;
    let cy = plat.position.1 + plat.size.1 * 0.5;
    let rad = plat.rotation.to_radians();
    let dx = world_x - cx;
    cy - (plat.size.1 * 0.5) + dx * rad.sin()
}

/// Compute penetration depth of obj into plat along normal (nx, ny).
fn penetration_depth(obj: &GameObject, plat: &GameObject, nx: f32, ny: f32) -> f32 {
    // Project overlap onto normal axis
    let (ox, oy, ow, oh) = (obj.position.0, obj.position.1, obj.size.0, obj.size.1);
    let (px, py, pw, ph) = (plat.position.0, plat.position.1, plat.size.0, plat.size.1);

    if nx.abs() > ny.abs() {
        // Primarily horizontal normal
        if nx > 0.0 { px + pw - ox } else { ox + ow - px }
    } else {
        // Primarily vertical normal
        if ny > 0.0 { py + ph - oy } else { oy + oh - py }
    }
}

/// Adjust render offset for rotated/sloped objects.
fn rotation_adjusted_offset(
    position: (f32, f32),
    _size: (f32, f32),
    _rotation: f32,
    _is_slope: bool,
) -> (f32, f32) {
    // For now, just return position. Visual rotation is handled by
    // the image shape's rotation parameter, not by offset adjustment.
    position
}
```

### Step 9.2 — Replace handle_collisions()

Replace the current `handle_collisions()` method with the upgraded version
from synful_slopes. The key differences from current:

1. **Normal-based push resolution** instead of just top-landing
2. **Approach check**: only resolve if object is moving into the surface
3. **One-way filtering**: skip collision if object came from behind
4. **Slope surface interpolation**: use `slope_surface_y()` for slope platforms
5. **Rotation surface**: use `rotated_surface_y()` for rotated platforms
6. **Surface velocity transfer**: push object tangentially on conveyor belts
7. **Inward momentum cancellation**: remove the component of momentum going into the surface

See the exact source code in the synful_slopes audit (handle_collisions in
apis.rs, ~100 lines). Copy it verbatim — it references:
- `Self::check_collision(o1, o2)` (already exists)
- `plat.surface_normal_at(x)` (added in Phase 7)
- `plat.slope_surface_y(x)` (added in Phase 7)
- `plat.one_way` (added in Phase 7)
- `plat.surface_velocity` (added in Phase 7)
- `rotated_surface_y()` (added in Step 9.1)
- `penetration_depth()` (added in Step 9.1)
- `rotation_adjusted_offset()` (added in Step 9.1)

### Step 9.3 — Update update_objects() for rotation momentum

In `update_objects()`, add a call to `apply_rotation_momentum()` in the
per-object loop, after `apply_resistance()`:

```rust
// Current:
    obj.apply_resistance();

// Becomes:
    obj.apply_resistance();
    obj.apply_rotation_momentum();
```

---

## 11. Phase 10 — BoundaryCollision Wiring

**WHY:** `trigger_boundary_collision_events()` exists but is never called.
It should be called in the tick loop after `update_objects()` for each object
that has BoundaryCollision events registered.

**WHERE:** `quartz/src/lib.rs` (OnEvent impl, tick section) or `quartz/src/canvas.rs`

### Step 10.1 — Add a check_and_fire_boundary_collisions method

```rust
pub(crate) fn check_and_fire_boundary_collisions(&mut self) {
    let canvas_size = self.layout.canvas_size.get();
    let indices_with_boundary_events: Vec<usize> = self.store.events
        .iter()
        .enumerate()
        .filter(|(_, events)| events.iter().any(|e| matches!(e, GameEvent::BoundaryCollision { .. })))
        .map(|(idx, _)| idx)
        .collect();

    for idx in indices_with_boundary_events {
        if let Some(obj) = self.store.objects.get(idx) {
            if obj.check_boundary_collision(canvas_size) {
                self.trigger_boundary_collision_events(idx);
            }
        }
    }
}
```

### Step 10.2 — Wire it into the tick loop

In `quartz/src/lib.rs`, in the `OnEvent` impl, after `self.handle_collisions();`:

```rust
    self.handle_collisions();
    self.check_and_fire_boundary_collisions();  // ← NEW
```

---

## 12. Phase 11 — Pause/Resume

**WHY:** A paused Canvas skips the entire tick loop — no physics updates, no
event processing, no callbacks. This is essential for pause menus.

**WHERE:** `quartz/src/lib.rs` (Canvas struct already has `paused: bool` from Phase 1)

### Step 11.1 — Add methods

**File:** `quartz/src/canvas.rs`

```rust
pub fn pause(&mut self)      { self.paused = true; }
pub fn resume(&mut self)     { self.paused = false; }
pub fn is_paused(&self) -> bool { self.paused }
```

### Step 11.2 — Guard the tick loop

**File:** `quartz/src/lib.rs`, in the `OnEvent` impl, at the start of the tick
handling section:

```rust
// Current:
if let Some(_tick) = event.downcast_ref::<TickEvent>() {

// Add right after:
    if self.paused { return vec![event]; }
```

---

## 13. Phase 12 — Re-exports and lib.rs Updates

**WHY:** Users of the quartz crate need to be able to import the new types
without reaching into submodules.

**WHERE:** `quartz/src/lib.rs`

### Step 12.1 — Add new re-exports

```rust
// Value system (already partially exported — add the missing ones)
pub use value::{
    Expr, Value, MathOp, CompOp,
    resolve_expr, apply_op, compare_operands,
};

// Expression parser
pub use expr::{parse_condition, parse_action};
```

The `Expr`, `Value`, `MathOp`, `CompOp`, `resolve_expr`, `apply_op`, and
`compare_operands` are already re-exported. Just add `parse_condition` and
`parse_action` once `expr.rs` exists.

### Step 12.2 — Add HashMap import to lib.rs

```rust
use std::collections::HashMap;
use crate::value::Value;
```

(Needed for the `game_vars: HashMap<String, Value>` field on Canvas.)

---

## 14. Phase 13 — Helper Methods on Action and Condition

**WHY:** synful_slopes had convenience constructors on Action and Condition
that make the API much more ergonomic. For example:
- `Action::set_var("score", 0i32)` instead of `Action::SetVar { name: "score".to_string(), value: Expr::i32(0) }`
- `Action::when(cond, action)` instead of `Action::Conditional { condition: cond, if_true: Box::new(action), if_false: None }`
- `Condition::expr("score > 10")` instead of `Condition::Expr("score > 10".to_string())`

**WHERE:** `quartz/src/types.rs`

### Step 13.1 — Add impl Action helpers

```rust
impl Action {
    pub fn apply_momentum(target: Target, value: (f32, f32)) -> Self {
        Action::ApplyMomentum { target, value }
    }
    pub fn set_momentum(target: Target, value: (f32, f32)) -> Self {
        Action::SetMomentum { target, value }
    }
    pub fn set_resistance(target: Target, value: (f32, f32)) -> Self {
        Action::SetResistance { target, value }
    }
    pub fn remove(target: Target) -> Self {
        Action::Remove { target }
    }
    pub fn spawn(object: GameObject, location: Location) -> Self {
        Action::Spawn { object: Box::new(object), location }
    }
    pub fn transfer_momentum(from: Target, to: Target, scale: f32) -> Self {
        Action::TransferMomentum { from, to, scale }
    }
    pub fn set_animation(target: Target, animation_bytes: &'static [u8], fps: f32) -> Self {
        Action::SetAnimation { target, animation_bytes, fps }
    }
    pub fn teleport(target: Target, location: Location) -> Self {
        Action::Teleport { target, location }
    }
    pub fn show(target: Target) -> Self { Action::Show { target } }
    pub fn hide(target: Target) -> Self { Action::Hide { target } }
    pub fn toggle(target: Target) -> Self { Action::Toggle { target } }
    pub fn custom(name: impl Into<String>) -> Self {
        Action::Custom { name: name.into() }
    }
    pub fn when(condition: Condition, action: Action) -> Self {
        Action::Conditional { condition, if_true: Box::new(action), if_false: None }
    }
    pub fn when_else(condition: Condition, if_true: Action, if_false: Action) -> Self {
        Action::Conditional {
            condition,
            if_true: Box::new(if_true),
            if_false: Some(Box::new(if_false)),
        }
    }
    pub fn multi(actions: Vec<Action>) -> Self { Action::Multi(actions) }
    pub fn play_sound(path: impl Into<String>) -> Self {
        Action::PlaySound { path: path.into(), options: SoundOptions::default() }
    }
    pub fn play_sound_with_options(path: impl Into<String>, options: SoundOptions) -> Self {
        Action::PlaySound { path: path.into(), options }
    }
    pub fn set_gravity(target: Target, value: f32) -> Self {
        Action::SetGravity { target, value }
    }
    pub fn set_size(target: Target, value: (f32, f32)) -> Self {
        Action::SetSize { target, value }
    }
    pub fn add_tag(target: Target, tag: impl Into<String>) -> Self {
        Action::AddTag { target, tag: tag.into() }
    }
    pub fn remove_tag(target: Target, tag: impl Into<String>) -> Self {
        Action::RemoveTag { target, tag: tag.into() }
    }
    pub fn set_var(name: impl Into<String>, value: impl Into<Expr>) -> Self {
        Action::SetVar { name: name.into(), value: value.into() }
    }
    pub fn mod_var(name: impl Into<String>, op: MathOp, operand: impl Into<Expr>) -> Self {
        Action::ModVar { name: name.into(), op, operand: operand.into() }
    }
    pub fn expr(src: impl Into<String>) -> Result<Self, String> {
        let src = src.into();
        crate::expr::parse_action(&src)?;  // validate at construction time
        Ok(Action::Expr(src))
    }
    pub fn set_rotation(target: Target, value: f32) -> Self {
        Action::SetRotation { target, value }
    }
    pub fn set_slope(target: Target, left_offset: f32, right_offset: f32, auto_rotate: bool) -> Self {
        Action::SetSlope { target, left_offset, right_offset, auto_rotate }
    }
    pub fn add_rotation(target: Target, value: f32) -> Self {
        Action::AddRotation { target, value }
    }
    pub fn apply_rotation(target: Target, value: f32) -> Self {
        Action::ApplyRotation { target, value }
    }
    pub fn set_surface_normal(target: Target, nx: f32, ny: f32) -> Self {
        Action::SetSurfaceNormal { target, nx, ny }
    }
}
```

### Step 13.2 — Add impl Condition helpers

```rust
impl Condition {
    pub fn expr(src: impl Into<String>) -> Result<Self, String> {
        let src = src.into();
        crate::expr::parse_condition(&src)?;  // validate at construction time
        Ok(Condition::Expr(src))
    }

    pub fn and(self, other: Condition) -> Self {
        Condition::And(Box::new(self), Box::new(other))
    }
    pub fn or(self, other: Condition) -> Self {
        Condition::Or(Box::new(self), Box::new(other))
    }
    pub fn not(self) -> Self {
        Condition::Not(Box::new(self))
    }
}
```

---

## 15. Phase 14 — Verification

After all phases are complete, verify the integration:

### Compile check
```bash
cd quartz && cargo check
```

All 15 new Action variants and 5 new Condition variants must be handled
in `run()` and `evaluate_condition()` respectively, or the compiler will
error with non-exhaustive match.

### synful_testing update
Update `quartz/synful_testing/src/lib.rs` to exercise the new features:
- `canvas.set_var("score", 0i32)` / `canvas.get_i32("score")`
- `Action::SetVar { name: "score".into(), value: Expr::i32(0) }`
- `Action::ModVar { name: "score".into(), op: MathOp::Add, operand: Expr::i32(1) }`
- `Action::Multi(vec![...])` bundling multiple actions
- `Action::expr("score += 1")` string-based scripting
- `Condition::Compare(Expr::var("score"), CompOp::Gt, Expr::i32(10))`
- `Condition::VarExists("score".into())`
- `Condition::Grounded(Target::id("player"))`
- `Condition::expr("score > 10")`
- `Condition::HasTag(Target::id("player"), "grounded".into())`
- `Action::SetGravity`, `Action::SetSize`, `Action::AddTag`, `Action::RemoveTag`
- `Action::SetRotation`, `Action::AddRotation`, `Action::ApplyRotation`
- Builder: `.slope()`, `.one_way()`, `.ceiling()`, `.wall_left()`, `.surface_velocity()`

---

## 16. Dependency Graph

```
Phase 1 (game_vars)
    ↓
Phase 2 (expr.rs) ← requires Phase 3 + 4 (so Action/Condition have Expr variants)
    ↓
Phase 3 (Action variants) ← can be done with Phase 4 simultaneously
Phase 4 (Condition variants)
    ↓
Phase 5 (run() wiring) ← requires Phase 1 + 2 + 3
Phase 6 (evaluate_condition() wiring) ← requires Phase 1 + 2 + 4
    ↓
Phase 7 (GameObject fields) ← independent of 1-6, but needed by Phase 9
    ↓
Phase 8 (Builder extensions) ← requires Phase 7
    ↓
Phase 9 (Collision system) ← requires Phase 7 + 8
    ↓
Phase 10 (BoundaryCollision) ← requires existing code only
Phase 11 (Pause/Resume) ← requires Phase 1 (paused field)
Phase 12 (Re-exports) ← requires Phase 2
Phase 13 (Helper methods) ← requires Phase 2 + 3 + 4
    ↓
Phase 14 (Verification) ← all phases complete
```

### Recommended execution order (minimizes blocked work):

1. **Phase 7** — GameObject fields (no deps)
2. **Phase 8** — Builder extensions (needs Phase 7)
3. **Phase 1** — game_vars + paused on Canvas
4. **Phase 3 + 4 together** — Action + Condition variants
5. **Phase 2** — expr.rs (needs Phase 3 + 4)
6. **Phase 5 + 6 together** — run() + evaluate_condition() wiring
7. **Phase 9** — Collision system upgrade
8. **Phase 10** — BoundaryCollision wiring
9. **Phase 11** — Pause/Resume
10. **Phase 12** — Re-exports
11. **Phase 13** — Helper methods
12. **Phase 14** — Verify everything compiles and run synful_testing

---

## Quick Reference — Files Modified

| File | Phases |
|------|--------|
| `quartz/src/lib.rs` | 1, 2, 10, 11, 12 |
| `quartz/src/canvas.rs` | 1, 5, 6, 9 |
| `quartz/src/types.rs` | 3, 4, 13 |
| `quartz/src/object.rs` | 7, 8 |
| `quartz/src/expr.rs` | 2 (new file) |
| `quartz/synful_testing/src/lib.rs` | 14 |
