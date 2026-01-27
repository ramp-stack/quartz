# Quartz 2D Game Engine

<p align="center">
  <img src="./logo/quartz.png" alt="Quartz Logo" width="400"/>
</p>

A Rust-based 2D game engine built on top of the Prism framework, designed for creating interactive games with physics, animations, and event-driven gameplay.

## Overview

Quartz provides a high-level abstraction for building 2D games with:

- **Canvas-based rendering** with automatic scaling and aspect ratio management
- **GameObject system** with physics (gravity, momentum, resistance) and platform support
- **Event-driven architecture** for collisions, keyboard input, and conditional logic
- **Animated sprite support** with GIF animation loading
- **Tag-based targeting** for flexible object management
- **Anchor-based positioning** for relative object placement
- **Visibility toggling** and conditional actions

## Core Concepts

### Canvas

The `Canvas` is your game world. It manages all game objects, handles layout scaling, and processes events.
```rust
let mut canvas = Canvas::new(ctx, CanvasMode::Landscape);
```

**Canvas Modes:**
- `CanvasMode::Landscape`: 3840×2160 virtual resolution (16:9)
- `CanvasMode::Portrait`: 2160×3840 virtual resolution (9:16)

The canvas automatically scales to fit any screen size while maintaining aspect ratio and letterboxing when necessary.

### GameObject

GameObjects are the entities in your game. Each has:

- **Identity**: Unique ID and tags for flexible targeting
- **Position & Size**: Location and dimensions in virtual coordinates
- **Physics**: Momentum, resistance, and gravity
- **Visuals**: Static images or animated sprites
- **Visibility**: Can be shown/hidden
- **Platform Properties**: Can act as platforms for other objects to stand on

**Creating GameObjects:**
```rust
// Square GameObject
let player = GameObject::new(
    ctx,
    "player".to_string(),      // id
    image,                      // Image
    100.0,                      // size (creates 100x100)
    (200.0, 300.0),            // position (x, y)
    vec!["player".to_string()], // tags
    (0.0, 0.0),                // initial momentum (x, y)
    (0.95, 0.95),              // resistance (x, y) - multiplied each frame
    0.5,                        // gravity - added to y momentum each frame
);

// Rectangular GameObject
let background = GameObject::new_rect(
    ctx,
    "bg".to_string(),
    bg_image,
    (3840.0, 2160.0),          // size (width, height)
    (0.0, 0.0),
    vec!["background".to_string()],
    (0.0, 0.0),
    (1.0, 1.0),
    0.0,
);

// Platform GameObject
let platform = GameObject::new_rect(ctx, ...)
    .as_platform();  // Makes object solid - other objects can stand on it
```

### Animated Sprites

Load GIF animations and attach them to GameObjects:
```rust
let animation = AnimatedSprite::new(
    gif_bytes,              // &[u8] - use include_bytes!()
    (50.0, 35.0),          // size in virtual coordinates
    12.0                    // frames per second
).expect("Failed to load animation");

let animated_object = GameObject::new_rect(ctx, ...)
    .with_animation(animation);
```

### Targeting System

Target objects in three ways:
```rust
Target::ById("enemy_1".to_string())         // Target by ID
Target::ByTag("enemies".to_string())        // Target all objects with tag
```

### Actions

Actions modify game objects:
```rust
// Add to existing momentum (for jumps, boosts)
Action::ApplyMomentum { 
    target: Target::ById("player".to_string()),
    value: (0.0, -10.0)  // negative y = upward
}

// Set momentum directly (for stopping, movement)
Action::SetMomentum { 
    target: Target::ByTag("bullets".to_string()),
    value: (5.0, 0.0)
}

// Change resistance (for ice, friction changes)
Action::SetResistance { 
    target: Target::ById("player".to_string()),
    value: (0.99, 0.99)  // less resistance = slides more
}

// Remove objects
Action::Remove { 
    target: Target::ByTag("enemies".to_string())
}

// Spawn new objects
Action::Spawn {
    object: Box::new(bullet),
    location: Location::AtTarget(Box::new(Target::ById("player".to_string())))
}

// Change animations
Action::SetAnimation {
    target: Target::ById("player".to_string()),
    animation_bytes: include_bytes!("../assets/running.gif"),
    fps: 16.0
}

// Toggle visibility
Action::Toggle {
    target: Target::ById("powerup".to_string())
}

// Teleport to position
Action::Teleport {
    target: Target::ById("player".to_string()),
    location: Location::OnTarget {
        target: Box::new(Target::ById("platform".to_string())),
        anchor: Anchor::TopCenter,
        offset: (0.0, -50.0)
    }
}

// Conditional actions
Action::Conditional {
    condition: Condition::IsVisible(Target::ById("key".to_string())),
    if_true: Box::new(Action::Remove { target: Target::ById("door".to_string()) }),
    if_false: None
}
```

### Conditions

Conditions check game state before executing actions:
```rust
// Check if object is visible
Condition::IsVisible(Target::ById("indicator".to_string()))

// Use in conditional actions
Action::Conditional {
    condition: Condition::IsVisible(Target::ById("power_mode".to_string())),
    if_true: Box::new(Action::SetMomentum { 
        target: Target::ById("player".to_string()),
        value: (20.0, 0.0)  // Fast movement
    }),
    if_false: Some(Box::new(Action::SetMomentum {
        target: Target::ById("player".to_string()),
        value: (5.0, 0.0)   // Normal movement
    }))
}
```

### Anchors

Position objects relative to other objects using anchors:
```rust
// Available anchors
Anchor::TopLeft
Anchor::TopCenter
Anchor::TopRight
Anchor::CenterLeft
Anchor::Center
Anchor::CenterRight
Anchor::BottomLeft
Anchor::BottomCenter
Anchor::BottomRight

// Example: Place companion to the right of player
Action::Teleport {
    target: Target::ById("companion".to_string()),
    location: Location::OnTarget {
        target: Box::new(Target::ById("player".to_string())),
        anchor: Anchor::BottomRight,  // Anchor to player's bottom-right
        offset: (50.0, 0.0)           // 50px to the right
    }
}
```

### Events

Connect game events to actions:
```rust
// Keyboard input
canvas.add_event(
    GameEvent::KeyPress {
        key: Key::Character("w".to_string().into()),
        action: Action::ApplyMomentum {
            target: Target::ById("player".to_string()),
            value: (0.0, -10.5)
        },
        target: Target::ById("player".to_string())
    },
    Target::ById("player".to_string())
);

// Collision between objects
canvas.add_event(
    GameEvent::Collision {
        action: Action::Remove {
            target: Target::ById("enemy".to_string())
        },
        target: Target::ById("bullet".to_string())
    },
    Target::ById("bullet".to_string())
);

// Boundary collision (hitting canvas edges)
canvas.add_event(
    GameEvent::BoundaryCollision {
        action: Action::SetMomentum {
            target: Target::ById("ball".to_string()),
            value: (0.0, 0.0)
        },
        target: Target::ById("ball".to_string())
    },
    Target::ById("ball".to_string())
);
```

### Custom Tick Updates

Run custom logic every frame using the `on_tick` callback:
```rust
let mut counter = 0u32;
canvas.on_tick(move |canvas| {
    counter += 1;
    
    // Toggle visibility every 300 frames (~5 seconds at 60fps)
    if counter >= 300 {
        counter = 0;
        if let Some(obj) = canvas.get_game_object_mut("powerup") {
            obj.visible = !obj.visible;
        }
    }
});
```

## Physics System

Quartz has a built-in physics system that runs every frame (60 FPS):

1. **Gravity**: Added to vertical momentum each frame
2. **Momentum**: Position updated by momentum values
3. **Resistance**: Momentum multiplied by resistance (friction/drag)
4. **Platform Collision**: Objects land on platforms and stop falling
5. **Collision Detection**: Automatic AABB collision detection between all objects

**Physics Tips:**
- Gravity of `1.2` with resistance `(0.98, 0.98)` works well for platformers
- Resistance of `(1.0, 1.0)` = no friction (objects never slow down)
- Resistance of `(0.0, 0.0)` = instant stop
- Negative momentum = movement left/up, positive = right/down
- Use `.as_platform()` to make objects solid for others to stand on

## Complete Example: Simple Platformer
```rust
use quartz::{Key, Context, Image, ShapeType, Canvas, GameObject, 
             Action, Target, GameEvent, CanvasMode, Location, 
             Condition, Anchor};
use ramp::prism;
use prism::drawable::Drawable;

pub struct MyApp;

impl MyApp {
    fn new(ctx: &mut Context) -> impl Drawable {
        let canvas_mode = CanvasMode::Landscape;
        let virtual_size = (3840.0, 2160.0);
        
        let player_size = 100.0;
        let ground_level = virtual_size.1 - 200.0;
        
        // Create green player block
        let player_image = Image {
            shape: ShapeType::Rectangle(0.0, (player_size, player_size), 0.0),
            image: image::RgbaImage::from_pixel(1, 1, 
                image::Rgba([0, 255, 0, 255])).into(),
            color: None
        };
        
        // Create ground platform
        let ground_image = Image {
            shape: ShapeType::Rectangle(0.0, (virtual_size.0, 50.0), 0.0),
            image: image::RgbaImage::from_pixel(1, 1, 
                image::Rgba([100, 100, 100, 255])).into(),
            color: None
        };
        
        // Create power indicator (red = off, green = on)
        let indicator_image = Image {
            shape: ShapeType::Rectangle(0.0, (50.0, 50.0), 0.0),
            image: image::RgbaImage::from_pixel(1, 1, 
                image::Rgba([255, 0, 0, 255])).into(),
            color: None
        };
        
        let mut canvas = Canvas::new(ctx, canvas_mode);
        
        // Add ground platform
        let ground = GameObject::new_rect(
            ctx,
            "ground".to_string(),
            ground_image,
            (virtual_size.0, 50.0),
            (0.0, ground_level),
            vec!["ground".to_string()],
            (0.0, 0.0),
            (1.0, 1.0),
            0.0,
        ).as_platform();  // Make it a platform
        canvas.add_game_object("ground".to_string(), ground);
        
        // Add player
        let player = GameObject::new(
            ctx,
            "player".to_string(),
            player_image,
            player_size,
            (400.0, ground_level - player_size),
            vec!["player".to_string()],
            (0.0, 0.0),
            (0.98, 0.98),
            1.2,  // Gravity
        );
        canvas.add_game_object("player".to_string(), player);
        
        // Add power indicator
        let indicator = GameObject::new(
            ctx,
            "power_indicator".to_string(),
            indicator_image,
            50.0,
            (100.0, 100.0),
            vec!["indicator".to_string()],
            (0.0, 0.0),
            (1.0, 1.0),
            0.0,
        );
        canvas.add_game_object("power_indicator".to_string(), indicator);
        
        // Jump on 'W' key
        canvas.add_event(
            GameEvent::KeyPress {
                key: Key::Character("w".to_string().into()),
                action: Action::ApplyMomentum {
                    target: Target::ById("player".to_string()),
                    value: (0.0, -30.0)
                },
                target: Target::ById("player".to_string())
            },
            Target::ById("player".to_string())
        );
        
        // Move left on 'A' key (only if power indicator visible)
        canvas.add_event(
            GameEvent::KeyPress {
                key: Key::Character("a".to_string().into()),
                action: Action::Conditional {
                    condition: Condition::IsVisible(
                        Target::ById("power_indicator".to_string())
                    ),
                    if_true: Box::new(Action::SetMomentum {
                        target: Target::ById("player".to_string()),
                        value: (-15.0, 0.0)
                    }),
                    if_false: None
                },
                target: Target::ById("player".to_string())
            },
            Target::ById("player".to_string())
        );
        
        // Stop horizontal movement on 'A' release
        canvas.add_event(
            GameEvent::KeyRelease {
                key: Key::Character("a".to_string().into()),
                action: Action::SetMomentum {
                    target: Target::ById("player".to_string()),
                    value: (0.0, 0.0)
                },
                target: Target::ById("player".to_string())
            },
            Target::ById("player".to_string())
        );
        
        // Toggle power indicator with 'P' key
        canvas.add_event(
            GameEvent::KeyPress {
                key: Key::Character("p".to_string().into()),
                action: Action::Toggle {
                    target: Target::ById("power_indicator".to_string())
                },
                target: Target::ById("player".to_string())
            },
            Target::ById("player".to_string())
        );
        
        canvas
    }
}

ramp::run!{|ctx: &mut Context| {
    MyApp::new(ctx)
}}
```

## API Reference

### Canvas Methods
```rust
// Create a new canvas
Canvas::new(ctx: &mut Context, mode: CanvasMode) -> Self

// Add a game object
canvas.add_game_object(name: String, object: GameObject)

// Remove a game object
canvas.remove_game_object(name: &str)

// Get object reference
canvas.get_game_object(name: &str) -> Option<&GameObject>
canvas.get_game_object_mut(name: &str) -> Option<&mut GameObject>

// Add event handler
canvas.add_event(event: GameEvent, target: Target)

// Execute an action
canvas.run(action: Action)

// Custom per-frame logic
canvas.on_tick<F>(callback: F) where F: FnMut(&mut Canvas) + 'static

// Check collision between targets
canvas.collision_between(target1: &Target, target2: &Target) -> bool

// Get canvas info
canvas.get_virtual_size() -> (f32, f32)
canvas.get_scale() -> f32
canvas.get_mode() -> CanvasMode
```

### GameObject Methods
```rust
// Create square GameObject
GameObject::new(
    ctx: &mut Context,
    id: String,
    image: Image,
    size: f32,
    position: (f32, f32),
    tags: Vec<String>,
    momentum: (f32, f32),
    resistance: (f32, f32),
    gravity: f32
) -> Self

// Create rectangular GameObject
GameObject::new_rect(
    ctx: &mut Context,
    id: String,
    image: Image,
    size: (f32, f32),
    position: (f32, f32),
    tags: Vec<String>,
    momentum: (f32, f32),
    resistance: (f32, f32),
    gravity: f32
) -> Self

// Make object a platform
object.as_platform() -> Self

// Add animation
object.with_animation(sprite: AnimatedSprite) -> Self

// Visibility control
object.visible = true/false

// Modify properties
object.set_gravity(gravity: f32)
```

### AnimatedSprite Methods
```rust
// Create from GIF bytes
AnimatedSprite::new(gif_bytes: &[u8], size: (f32, f32), fps: f32) -> Result<Self, String>

// Control animation
sprite.update(delta_time: f32)
sprite.set_fps(fps: f32)
sprite.reset()
sprite.set_frame(frame: usize)
sprite.frame_count() -> usize
sprite.get_current_image() -> Image
```

### Location Types
```rust
Location::Position((x, y))                              // Absolute position
Location::AtTarget(Box::new(target))                    // At target's position
Location::Between(Box::new(target1), Box::new(target2)) // Midpoint between targets
Location::OnTarget {                                    // Relative to target with anchor
    target: Box::new(target),
    anchor: Anchor::BottomRight,
    offset: (x, y)
}
```

### Condition Types
```rust
Condition::IsVisible(target)  // Check if target is visible
```

### Anchor Types
```rust
Anchor::TopLeft       // Top-left corner
Anchor::TopCenter     // Top edge, centered
Anchor::TopRight      // Top-right corner
Anchor::CenterLeft    // Left edge, vertically centered
Anchor::Center        // Center of object
Anchor::CenterRight   // Right edge, vertically centered
Anchor::BottomLeft    // Bottom-left corner
Anchor::BottomCenter  // Bottom edge, centered
Anchor::BottomRight   // Bottom-right corner
```

## How It Works

### Automatic Update Loop

Every frame (approximately 60 FPS), the Canvas automatically:

1. Runs custom `on_tick` callbacks
2. Updates all animated sprites
3. Applies gravity to momentum
4. Updates positions based on momentum
5. Applies resistance (friction/drag)
6. Checks for platform collisions and stops falling objects
7. Checks for collisions between all objects
8. Checks for boundary collisions
9. Triggers appropriate event handlers

### Coordinate System

- Origin (0, 0) is top-left
- X increases to the right
- Y increases downward
- Virtual coordinates scale automatically to actual screen size


## Architecture
```
Canvas (Game World)
  ├── CanvasLayout (Handles scaling and positioning)
  ├── GameObjects (Entities with physics and visuals)
  │     ├── Image (Static or from AnimatedSprite)
  │     ├── Physics (momentum, resistance, gravity)
  │     ├── Platform flag (solid for other objects)
  │     ├── Visibility flag
  │     └── Identity (id, tags)
  ├── Event System (Links triggers to actions)
  ├── Condition System (Checks state before actions)
  └── Tick Callbacks (Custom per-frame logic)
```

## License

Built on top of the Prism framework. Check your Prism license for usage terms.

---

**Created with ❤️ using Rust**