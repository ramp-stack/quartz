use quartz::*;
use ramp::prism;

// ── Asset bytes ────────────────────────────────────────────────────────────────
static PLAYER_GIF:  &[u8] = include_bytes!("assets/player.gif");
static ENEMY_GIF:   &[u8] = include_bytes!("assets/enemy.gif");
static COIN_GIF:    &[u8] = include_bytes!("assets/coin.gif");
static BG_GIF:      &[u8] = include_bytes!("assets/bg.gif");
static JUMP_SFX:    &str  = "assets/jump.wav";
static COIN_SFX:    &str  = "assets/coin.wav";
static HIT_SFX:     &str  = "assets/hit.wav";

// ── World constants ────────────────────────────────────────────────────────────
const CW: f32 = 3840.0;
const CH: f32 = 2160.0;

const GROUND_Y:    f32 = 1750.0;
const PLAYER_SIZE: f32 = 160.0;
const ENEMY_SIZE:  f32 = 140.0;
const COIN_SIZE:   f32 = 80.0;
const PLATFORM_W:  f32 = 500.0;
const PLATFORM_H:  f32 = 60.0;

const GRAVITY:     f32 = 1.2;
const JUMP_FORCE:  f32 = -28.0;
const MOVE_SPEED:  f32 = 14.0;
const ENEMY_SPEED: f32 = 4.0;

pub struct MyApp;

impl MyApp {
    fn new(ctx: &mut Context, _assets: Assets) -> impl Drawable {
        let mut canvas = Canvas::new(ctx, CanvasMode::Landscape);
        build_scenes(ctx, &mut canvas);
        canvas.load_scene("menu");
        canvas
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// SCENE BUILDERS
// ══════════════════════════════════════════════════════════════════════════════

fn build_scenes(ctx: &mut Context, canvas: &mut Canvas) {
    canvas.add_scene(build_menu_scene(ctx));
    canvas.add_scene(build_game_scene(ctx));
    canvas.add_scene(build_gameover_scene(ctx));
}

// ── Menu scene ─────────────────────────────────────────────────────────────────
fn build_menu_scene(ctx: &mut Context) -> Scene {
    // Title text object
    let title = GameObject::new_rect(
        ctx,
        "title".into(),
        Some(Text::new(
            "QUARTZ DEMO",
            Font::default(),
            120.0,
            Color::WHITE,
            Align::Center,
        )),
        (CW * 0.6, 200.0),
        (CW * 0.2, CH * 0.25),
        vec!["ui".into()],
        (0.0, 0.0),
        (1.0, 1.0),
        0.0,
    );

    let subtitle = GameObject::new_rect(
        ctx,
        "subtitle".into(),
        Some(Text::new(
            "Press SPACE or click START to play",
            Font::default(),
            60.0,
            Color::from_rgb(200, 200, 255),
            Align::Center,
        )),
        (CW * 0.6, 100.0),
        (CW * 0.2, CH * 0.45),
        vec!["ui".into()],
        (0.0, 0.0),
        (1.0, 1.0),
        0.0,
    );

    // Clickable start button
    let start_btn = GameObject::new_rect(
        ctx,
        "start_btn".into(),
        Some(Text::new(
            "[ START ]",
            Font::default(),
            80.0,
            Color::from_rgb(100, 255, 100),
            Align::Center,
        )),
        (400.0, 120.0),
        (CW * 0.5 - 200.0, CH * 0.6),
        vec!["ui".into(), "button".into()],
        (0.0, 0.0),
        (1.0, 1.0),
        0.0,
    );

    Scene::new("menu")
        .with_object("title".into(),    title)
        .with_object("subtitle".into(), subtitle)
        .with_object("start_btn".into(), start_btn)
        // Space key → load game
        .with_event(
            GameEvent::KeyPress {
                key: Key::Named(NamedKey::Space),
                action: Action::Custom { name: "goto_game".into() },
                target: Target::tag("ui"),
            },
            Target::tag("ui"),
        )
        // Click start button → load game
        .with_event(
            GameEvent::MousePress {
                action: Action::Custom { name: "goto_game".into() },
                target: Target::name("start_btn"),
                button: Some(MouseButton::Left),
            },
            Target::name("start_btn"),
        )
        // Hover highlight
        .with_event(
            GameEvent::MouseEnter {
                action: Action::Show { target: Target::name("start_btn") },
                target: Target::name("start_btn"),
            },
            Target::name("start_btn"),
        )
        .on_enter(|canvas| {
            canvas.register_custom_event("goto_game".into(), |c| {
                c.load_scene("game");
            });
        })
}

// ── Game scene ─────────────────────────────────────────────────────────────────
fn build_game_scene(ctx: &mut Context) -> Scene {
    // ── Scrolling backgrounds (infinite scroll) ────────────────────────────
    let bg1 = GameObject::new_rect(
        ctx,
        "bg1".into(),
        Some(
            AnimatedSprite::new(BG_GIF, (CW, CH), 12.0)
                .unwrap()
                .get_current_image(),
        ),
        (CW, CH),
        (0.0, 0.0),
        vec!["scroll".into(), "bg".into()],
        (-3.0, 0.0),   // slow scroll
        (1.0, 1.0),
        0.0,
    )
    .with_animation(AnimatedSprite::new(BG_GIF, (CW, CH), 12.0).unwrap());

    let bg2 = GameObject::new_rect(
        ctx,
        "bg2".into(),
        Some(
            AnimatedSprite::new(BG_GIF, (CW, CH), 12.0)
                .unwrap()
                .get_current_image(),
        ),
        (CW, CH),
        (CW, 0.0),
        vec!["scroll".into(), "bg".into()],
        (-3.0, 0.0),
        (1.0, 1.0),
        0.0,
    )
    .with_animation(AnimatedSprite::new(BG_GIF, (CW, CH), 12.0).unwrap());

    // ── Ground platform ────────────────────────────────────────────────────
    let ground = GameObject::new_rect(
        ctx,
        "ground".into(),
        Some(Image {
            shape: ShapeType::Rectangle(0.0, (CW, 400.0), 0.0),
            image: solid_color_image(60, 40, 20),
            color: None,
        }),
        (CW, 400.0),
        (0.0, GROUND_Y),
        vec!["platform".into()],
        (0.0, 0.0),
        (1.0, 1.0),
        0.0,
    )
    .as_platform();

    // ── Floating platforms ─────────────────────────────────────────────────
    let plat_data: &[(f32, f32)] = &[
        (600.0,  1400.0),
        (1300.0, 1200.0),
        (2100.0, 1350.0),
        (2900.0, 1100.0),
        (3400.0, 1300.0),
    ];

    // ── Player ─────────────────────────────────────────────────────────────
    let player = GameObject::new(
        ctx,
        "player".into(),
        None::<Image>,
        PLAYER_SIZE,
        (400.0, GROUND_Y - PLAYER_SIZE),
        vec!["player".into()],
        (0.0, 0.0),
        (0.95, 1.0),  // x-resistance, no y-resistance (gravity handles it)
        GRAVITY,
    )
    .with_animation(AnimatedSprite::new(PLAYER_GIF, (PLAYER_SIZE, PLAYER_SIZE), 12.0).unwrap());

    // ── Enemies ────────────────────────────────────────────────────────────
    let enemy1 = make_enemy(ctx, "enemy1", 1800.0, GROUND_Y - ENEMY_SIZE);
    let enemy2 = make_enemy(ctx, "enemy2", 2800.0, GROUND_Y - ENEMY_SIZE);
    let enemy3 = make_enemy(ctx, "enemy3", 3300.0, 1300.0 - ENEMY_SIZE);

    // ── Coins ──────────────────────────────────────────────────────────────
    let coins: Vec<(String, GameObject)> = vec![
        (700.0,  1350.0), (750.0,  1350.0), (800.0,  1350.0),
        (1350.0, 1150.0), (1400.0, 1150.0),
        (2200.0, 1300.0), (2250.0, 1300.0),
        (3000.0, 1050.0), (3050.0, 1050.0), (3100.0, 1050.0),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (x, y))| {
        let name = format!("coin_{i}");
        let obj = GameObject::new(
            ctx,
            name.clone(),
            None::<Image>,
            COIN_SIZE,
            (x, y),
            vec!["coin".into()],
            (0.0, 0.0),
            (1.0, 1.0),
            0.0,
        )
        .with_animation(AnimatedSprite::new(COIN_GIF, (COIN_SIZE, COIN_SIZE), 10.0).unwrap());
        (name, obj)
    })
    .collect();

    // ── Score display ──────────────────────────────────────────────────────
    let score_display = GameObject::new_rect(
        ctx,
        "score".into(),
        Some(Text::new("Score: 0", Font::default(), 60.0, Color::WHITE, Align::Left)),
        (600.0, 80.0),
        (40.0, 40.0),
        vec!["hud".into()],
        (0.0, 0.0),
        (1.0, 1.0),
        0.0,
    );

    // ── Health bar (3 hearts shown as text) ───────────────────────────────
    let health_display = GameObject::new_rect(
        ctx,
        "health".into(),
        Some(Text::new("❤ ❤ ❤", Font::default(), 60.0, Color::from_rgb(255, 80, 80), Align::Left)),
        (400.0, 80.0),
        (40.0, 120.0),
        vec!["hud".into()],
        (0.0, 0.0),
        (1.0, 1.0),
        0.0,
    );

    // ── Build scene ────────────────────────────────────────────────────────
    let mut scene = Scene::new("game")
        .with_object("bg1".into(), bg1)
        .with_object("bg2".into(), bg2)
        .with_object("ground".into(), ground)
        .with_object("player".into(), player)
        .with_object("enemy1".into(), enemy1)
        .with_object("enemy2".into(), enemy2)
        .with_object("enemy3".into(), enemy3)
        .with_object("score_display".into(), score_display)
        .with_object("health_display".into(), health_display);

    // add floating platforms
    for (i, &(px, py)) in plat_data.iter().enumerate() {
        let plat = make_platform(ctx, &format!("plat_{i}"), px, py);
        scene = scene.with_object(format!("plat_{i}"), plat);
    }

    // add coins
    for (name, coin) in coins {
        scene = scene.with_object(name, coin);
    }

    // ── Key events on the scene ────────────────────────────────────────────

    // Jump — Space or Up arrow
    scene = scene
        .with_event(
            GameEvent::KeyPress {
                key: Key::Named(NamedKey::Space),
                action: Action::Custom { name: "player_jump".into() },
                target: Target::name("player"),
            },
            Target::name("player"),
        )
        .with_event(
            GameEvent::KeyPress {
                key: Key::Named(NamedKey::ArrowUp),
                action: Action::Custom { name: "player_jump".into() },
                target: Target::name("player"),
            },
            Target::name("player"),
        )
        // Move left (hold)
        .with_event(
            GameEvent::KeyHold {
                key: Key::Named(NamedKey::ArrowLeft),
                action: Action::ApplyMomentum {
                    target: Target::name("player"),
                    value: (-MOVE_SPEED, 0.0),
                },
                target: Target::name("player"),
            },
            Target::name("player"),
        )
        // Move right (hold)
        .with_event(
            GameEvent::KeyHold {
                key: Key::Named(NamedKey::ArrowRight),
                action: Action::ApplyMomentum {
                    target: Target::name("player"),
                    value: (MOVE_SPEED, 0.0),
                },
                target: Target::name("player"),
            },
            Target::name("player"),
        )
        // Coin collision → remove coin + play sound + increment score
        .with_event(
            GameEvent::Collision {
                action: Action::Custom { name: "collect_coin".into() },
                target: Target::tag("coin"),
            },
            Target::name("player"),
        )
        // Enemy collision → damage player
        .with_event(
            GameEvent::Collision {
                action: Action::Custom { name: "player_hit".into() },
                target: Target::tag("enemy"),
            },
            Target::name("player"),
        )
        // Pause with Escape
        .with_event(
            GameEvent::KeyPress {
                key: Key::Named(NamedKey::Escape),
                action: Action::Custom { name: "goto_menu".into() },
                target: Target::name("player"),
            },
            Target::name("player"),
        );

    // ── on_enter: wire up all the custom game logic ────────────────────────
    scene.on_enter(|canvas| {
        // ── State stored in closures ──────────────────────────────────────
        // (In a real app you'd use a shared Rc<RefCell<State>>)

        // Camera follows player across the wide world
        let mut cam = Camera::new((CW * 2.0, CH), (CW, CH));
        cam.follow(Some(Target::name("player")));
        cam.lerp_speed = 0.08;
        canvas.set_camera(cam);

        // Enemy patrol AI
        canvas.on_tick(|c| {
            for name in ["enemy1", "enemy2", "enemy3"] {
                if let Some(obj) = c.get_game_object_mut(name) {
                    // reverse when near world edges
                    if obj.position.0 <= 100.0 {
                        obj.momentum.0 = ENEMY_SPEED;
                    } else if obj.position.0 >= CW * 2.0 - ENEMY_SIZE - 100.0 {
                        obj.momentum.0 = -ENEMY_SPEED;
                    }
                    // start patrol if not moving
                    if obj.momentum.0 == 0.0 {
                        obj.momentum.0 = ENEMY_SPEED;
                    }
                }
            }
        });

        // Coin bob animation
        canvas.on_tick(|c| {
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f32();
            for i in 0..10 {
                let name = format!("coin_{i}");
                if let Some(obj) = c.get_game_object_mut(&name) {
                    // gentle vertical bob
                    let base_y = obj.position.1;
                    obj.position.1 = base_y + (t * 3.0 + i as f32).sin() * 0.4;
                }
            }
        });

        // Jump handler
        canvas.register_custom_event("player_jump".into(), |c| {
            if let Some(player) = c.get_game_object_mut("player") {
                // only jump when on or near the ground
                if player.momentum.1.abs() < 0.5 {
                    player.momentum.1 = JUMP_FORCE;
                    c.play_sound(JUMP_SFX);
                }
            }
        });

        // Coin collection
        let mut score = 0u32;
        canvas.register_custom_event("collect_coin".into(), move |c| {
            // find which coins collide with player and remove them
            let player_pos = c.get_game_object("player").map(|p| p.position).unwrap_or_default();
            let player_size = c.get_game_object("player").map(|p| p.size).unwrap_or_default();

            let to_remove: Vec<String> = (0..10)
                .map(|i| format!("coin_{i}"))
                .filter(|name| {
                    if let Some(coin) = c.get_game_object(name) {
                        // AABB overlap
                        coin.visible
                            && player_pos.0 < coin.position.0 + coin.size.0
                            && player_pos.0 + player_size.0 > coin.position.0
                            && player_pos.1 < coin.position.1 + coin.size.1
                            && player_pos.1 + player_size.1 > coin.position.1
                    } else {
                        false
                    }
                })
                .collect();

            for name in to_remove {
                c.run(Action::Hide { target: Target::name(&name) });
                score += 10;
                c.play_sound(COIN_SFX);
            }

            // Update score display
            if let Some(hud) = c.get_game_object_mut("score_display") {
                hud.set_image(Image {
                    shape: ShapeType::Rectangle(0.0, (600.0, 80.0), 0.0),
                    image: render_text_image(&format!("Score: {score}")),
                    color: None,
                });
            }
        });

        // Player hit
        let mut health = 3i32;
        canvas.register_custom_event("player_hit".into(), move |c| {
            // check actual overlap to avoid phantom collisions
            let hit = c.collision_between(&Target::name("player"), &Target::tag("enemy"));
            if hit {
                health -= 1;
                c.play_sound(HIT_SFX);
                // knockback
                c.run(Action::ApplyMomentum {
                    target: Target::name("player"),
                    value: (0.0, -18.0),
                });
                // Flash: hide then show
                c.run(Action::Conditional {
                    condition: Condition::Always,
                    if_true: Box::new(Action::Toggle { target: Target::name("player") }),
                    if_false: None,
                });
                // update HUD
                let hearts = match health.max(0) {
                    3 => "❤ ❤ ❤",
                    2 => "❤ ❤",
                    1 => "❤",
                    _ => "",
                };
                if let Some(hud) = c.get_game_object_mut("health_display") {
                    hud.set_image(Image {
                        shape: ShapeType::Rectangle(0.0, (400.0, 80.0), 0.0),
                        image: render_text_image(hearts),
                        color: None,
                    });
                }
                if health <= 0 {
                    c.load_scene("gameover");
                }
            }
        });

        // Goto menu
        canvas.register_custom_event("goto_menu".into(), |c| {
            c.load_scene("menu");
        });

        // Mouse click to shoot a projectile toward cursor
        canvas.on_mouse_press(|c, btn, pos| {
            if btn != MouseButton::Left { return; }
            if let Some(player) = c.get_game_object("player") {
                let px = player.position.0 + player.size.0 * 0.5;
                let py = player.position.1 + player.size.1 * 0.5;
                let dx = pos.0 - px;
                let dy = pos.1 - py;
                let len = (dx * dx + dy * dy).sqrt().max(1.0);
                let speed = 30.0;
                let bullet_id = format!("bullet_{}", rand_u32());
                let bullet = GameObject::new(
                    // we don't have ctx here — use a pre-built Image
                    // (in a real codebase you'd pass ctx differently)
                    &mut unsafe { std::mem::zeroed() },
                    bullet_id.clone(),
                    Some(Image {
                        shape: ShapeType::Rectangle(0.0, (24.0, 24.0), 0.0),
                        image: solid_color_image(255, 220, 0),
                        color: None,
                    }),
                    24.0,
                    (px, py),
                    vec!["bullet".into()],
                    (dx / len * speed, dy / len * speed),
                    (1.0, 1.0),
                    0.0,
                );
                c.run(Action::Spawn {
                    object: Box::new(bullet),
                    location: Location::at(px, py),
                });
                // Remove bullet on enemy collision
                c.add_event(
                    GameEvent::Collision {
                        action: Action::Remove { target: Target::name(&bullet_id) },
                        target: Target::tag("enemy"),
                    },
                    Target::name(&bullet_id),
                );
            }
        });

        // Scroll to zoom camera lerp speed (fun demo of scroll events)
        canvas.on_mouse_scroll(|c, delta| {
            if let Some(cam) = c.camera_mut() {
                cam.lerp_speed = (cam.lerp_speed + delta.1 * 0.01).clamp(0.01, 0.5);
            }
        });
    })
}

// ── Game-over scene ────────────────────────────────────────────────────────────
fn build_gameover_scene(ctx: &mut Context) -> Scene {
    let title = GameObject::new_rect(
        ctx,
        "go_title".into(),
        Some(Text::new(
            "GAME OVER",
            Font::default(),
            140.0,
            Color::from_rgb(255, 80, 80),
            Align::Center,
        )),
        (CW * 0.6, 200.0),
        (CW * 0.2, CH * 0.3),
        vec!["ui".into()],
        (0.0, 0.0),
        (1.0, 1.0),
        0.0,
    );

    let retry_btn = GameObject::new_rect(
        ctx,
        "retry_btn".into(),
        Some(Text::new(
            "[ RETRY ]",
            Font::default(),
            80.0,
            Color::from_rgb(100, 255, 100),
            Align::Center,
        )),
        (400.0, 120.0),
        (CW * 0.5 - 200.0, CH * 0.55),
        vec!["ui".into(), "button".into()],
        (0.0, 0.0),
        (1.0, 1.0),
        0.0,
    );

    let menu_btn = GameObject::new_rect(
        ctx,
        "menu_btn".into(),
        Some(Text::new(
            "[ MENU ]",
            Font::default(),
            80.0,
            Color::from_rgb(100, 180, 255),
            Align::Center,
        )),
        (400.0, 120.0),
        (CW * 0.5 - 200.0, CH * 0.70),
        vec!["ui".into(), "button".into()],
        (0.0, 0.0),
        (1.0, 1.0),
        0.0,
    );

    Scene::new("gameover")
        .with_object("go_title".into(), title)
        .with_object("retry_btn".into(), retry_btn)
        .with_object("menu_btn".into(),  menu_btn)
        .with_event(
            GameEvent::MousePress {
                action: Action::Custom { name: "goto_game".into() },
                target: Target::name("retry_btn"),
                button: Some(MouseButton::Left),
            },
            Target::name("retry_btn"),
        )
        .with_event(
            GameEvent::MousePress {
                action: Action::Custom { name: "goto_menu".into() },
                target: Target::name("menu_btn"),
                button: Some(MouseButton::Left),
            },
            Target::name("menu_btn"),
        )
        .with_event(
            GameEvent::MouseEnter {
                action: Action::Toggle { target: Target::tag("button") },
                target: Target::tag("button"),
            },
            Target::tag("button"),
        )
        .on_enter(|canvas| {
            canvas.register_custom_event("goto_game".into(), |c| c.load_scene("game"));
            canvas.register_custom_event("goto_menu".into(), |c| c.load_scene("menu"));
        })
}

// ══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ══════════════════════════════════════════════════════════════════════════════

fn make_enemy(ctx: &mut Context, id: &str, x: f32, y: f32) -> GameObject {
    GameObject::new(
        ctx,
        id.into(),
        None::<Image>,
        ENEMY_SIZE,
        (x, y),
        vec!["enemy".into()],
        (ENEMY_SPEED, 0.0),
        (0.99, 1.0),
        GRAVITY,
    )
    .with_animation(AnimatedSprite::new(ENEMY_GIF, (ENEMY_SIZE, ENEMY_SIZE), 8.0).unwrap())
}

fn make_platform(ctx: &mut Context, id: &str, x: f32, y: f32) -> GameObject {
    GameObject::new_rect(
        ctx,
        id.into(),
        Some(Image {
            shape: ShapeType::Rectangle(8.0, (PLATFORM_W, PLATFORM_H), 0.0),
            image: solid_color_image(80, 120, 60),
            color: None,
        }),
        (PLATFORM_W, PLATFORM_H),
        (x, y),
        vec!["platform".into()],
        (0.0, 0.0),
        (1.0, 1.0),
        0.0,
    )
    .as_platform()
}

/// Returns a 1×1 RGBA image filled with the given colour — used as a stand-in
/// where real art assets aren't provided.
fn solid_color_image(r: u8, g: u8, b: u8) -> image::RgbaImage {
    let mut img = image::RgbaImage::new(1, 1);
    img.put_pixel(0, 0, image::Rgba([r, g, b, 255]));
    img
}

/// Placeholder: returns a 1×1 image. Replace with real text rendering if
/// your prism build exposes a CPU text rasteriser.
fn render_text_image(_text: &str) -> image::RgbaImage {
    solid_color_image(255, 255, 255)
}

/// Cheap non-crypto random u32 for unique bullet IDs.
fn rand_u32() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos()
}

// ══════════════════════════════════════════════════════════════════════════════
// ENTRY POINT
// ══════════════════════════════════════════════════════════════════════════════

ramp::run! { |ctx: &mut Context, assets: Assets| { MyApp::new(ctx, assets) } }