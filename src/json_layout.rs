use std::collections::HashMap;
use crate::canvas::Canvas;
use crate::object::GameObject;
use crate::sprite::tint_overlay;
use crate::rounded_box::RoundedBox;
use prism::canvas::{Align, Color, Font, Span, Text};
use crate::Arc;

fn jstr<'a>(v: &'a serde_json::Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}
fn jf32(v: &serde_json::Value, k: &str, d: f32) -> f32 {
    v[k].as_f64().map(|x| x as f32).unwrap_or(d)
}
fn jbool(v: &serde_json::Value, k: &str, d: bool) -> bool {
    v[k].as_bool().unwrap_or(d)
}
fn jcolor(v: &serde_json::Value, k: &str, d: Color) -> Color {
    parse_color_arr(v.get(k).unwrap_or(&serde_json::Value::Null), d)
}
fn parse_color_arr(v: &serde_json::Value, d: Color) -> Color {
    match v.as_array() {
        Some(a) if a.len() == 4 => {
            let c = |i: usize| a[i].as_u64().unwrap_or(255) as u8;
            Color(c(0), c(1), c(2), c(3))
        }
        _ => d,
    }
}
fn jalign(v: &serde_json::Value, k: &str) -> Align {
    match v[k].as_str().unwrap_or("left") {
        "center" | "Center" => Align::Center,
        "right"  | "Right"  => Align::Right,
        _                   => Align::Left,
    }
}
fn jalign_str(s: &str) -> Align {
    match s {
        "center" | "Center" => Align::Center,
        "right"  | "Right"  => Align::Right,
        _                   => Align::Left,
    }
}

fn pick_font<'a>(def: &serde_json::Value, fonts: &'a HashMap<String, Arc<Font>>) -> &'a Arc<Font> {
    pick_font_by_name(jstr(def, "font"), fonts)
}
fn pick_font_by_name<'a>(name: &str, fonts: &'a HashMap<String, Arc<Font>>) -> &'a Arc<Font> {
    if !name.is_empty() {
        if let Some(f) = fonts.get(name) { return f; }
    }
    fonts.get("body")
        .or_else(|| fonts.values().next())
        .expect("[json_layout] fonts map is empty")
}

fn make_text_scaled(
    cv:     &Canvas,
    text:   &str,
    fs_ref: f32,
    color:  Color,
    align:  Align,
    max_w:  Option<f32>,
    font:   &Arc<Font>,
) -> Text {
    let scaled_fs = (fs_ref * cv.virtual_scale()).max(1.0);
    Text::new(
        vec![Span::new(text.to_string(), scaled_fs, Some(scaled_fs * 1.55), font.clone(), color, 0.0)],
        max_w,
        align,
        None,
    )
}


fn resolve_format(cv: &Canvas, template: &str) -> String {
    let mut out = String::with_capacity(template.len() * 2);
    let mut chars = template.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        if ch == '$' {
            let start = i + 1;
            let mut end = start;
            while let Some(&(j, c)) = chars.peek() {
                if c.is_alphanumeric() || c == '_' {
                    end = j + c.len_utf8();
                    chars.next();
                } else { break; }
            }
            let var_name = &template[start..end];
            if var_name.is_empty() { out.push('$'); }
            else {
                let val = cv.get_var(var_name)
                    .map(|v| v.to_display_string())
                    .unwrap_or_default();
                out.push_str(&val);
            }
        } else { out.push(ch); }
    }
    out
}


fn collect_ids(json_str: &str) -> Vec<String> {
    let root: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v, Err(_) => return vec![],
    };
    root["objects"].as_array()
        .map(|a| {
            a.iter()
                .filter_map(|d| d["id"].as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn init_vars(cv: &mut Canvas, json_str: &str) {
    let root: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v, Err(_) => return,
    };
    if let Some(vars) = root["vars"].as_object() {
        for (k, v) in vars {
            if !cv.has_var(k) {
                let val = match v {
                    serde_json::Value::String(s) => s.clone(),
                    o => o.to_string(),
                };
                cv.set_var(k.clone(), val);
            }
        }
    }
}


fn register_interactive(
    cv:              &mut Canvas,
    objects:         &[serde_json::Value],
    interactive_ids: &mut Vec<String>,
) {
    for def in objects {
        let id       = jstr(def, "id");
        let on_click = jstr(def, "on_click");
        let on_hover = jstr(def, "on_hover");
        if id.is_empty() { continue; }
        if !on_click.is_empty() {
            cv.set_var(format!("__jl_click_{id}"), on_click.to_string());
            if !interactive_ids.contains(&id.to_string()) {
                interactive_ids.push(id.to_string());
            }
        }
        if !on_hover.is_empty() {
            cv.set_var(format!("__jl_hover_{id}"), on_hover.to_string());
            if !interactive_ids.contains(&id.to_string()) {
                interactive_ids.push(id.to_string());
            }
        }
    }
}


pub fn setup_events(cv: &mut Canvas, events_path: &str, layout_paths: &[&str]) {
    let events_str = match std::fs::read_to_string(events_path) {
        Ok(s)  => s,
        Err(e) => { eprintln!("[json_layout] cannot read \"{events_path}\": {e}"); return; }
    };
    let root: serde_json::Value = match serde_json::from_str(&events_str) {
        Ok(v)  => v,
        Err(e) => { eprintln!("[json_layout] parse error in \"{events_path}\": {e}"); return; }
    };

    if let Some(vars) = root["vars"].as_object() {
        for (k, v) in vars {
            let val = match v {
                serde_json::Value::String(s) => s.clone(),
                o => o.to_string(),
            };
            cv.set_var(k.clone(), val);
        }
    }

    if let Some(reactions) = root["reactions"].as_array() {
        cv.set_var("__rx_count", reactions.len());
        for (n, rx) in reactions.iter().enumerate() {
            cv.set_var(format!("__rx_{n}_w"), jstr(rx, "watch").to_string());
            cv.set_var(format!("__rx_{n}_c"), jstr(rx, "condition").to_string());
            cv.set_var(format!("__rx_{n}_e"), jstr(rx, "equals").to_string());
            cv.set_var(format!("__rx_{n}_t"), jstr(rx, "target").to_string());
            if let Some(s) = rx.get("set")  { cv.set_var(format!("__rx_{n}_s"), s.to_string()); }
            if let Some(e) = rx.get("else") { cv.set_var(format!("__rx_{n}_x"), e.to_string()); }
            cv.set_var(format!("__rx_{n}_last"), String::new());
        }
    }

    let mut interactive_ids: Vec<String> = Vec::new();
    if let Some(objects) = root["objects"].as_array() {
        register_interactive(cv, objects, &mut interactive_ids);
    }
    for path in layout_paths {
        match std::fs::read_to_string(path) {
            Ok(s) => match serde_json::from_str::<serde_json::Value>(&s) {
                Ok(v) => {
                    if let Some(objects) = v["objects"].as_array() {
                        register_interactive(cv, objects, &mut interactive_ids);
                    }
                }
                Err(e) => eprintln!("[json_layout] parse error in \"{path}\": {e}"),
            },
            Err(e) => eprintln!("[json_layout] cannot read \"{path}\": {e}"),
        }
    }
    cv.set_var("__jl_ids", interactive_ids.join(","));

    let mut kp: Vec<String> = vec![];
    let mut kr: Vec<String> = vec![];
    let mut mp: Vec<String> = vec![];
    let mut mr: Vec<String> = vec![];
    let mut sc_list: Vec<String> = vec![];
    let mut mm: Vec<String> = vec![];

    if let Some(events) = root["events"].as_array() {
        for ev in events {
            let on  = jstr(ev, "on");
            let sv  = jstr(ev, "set_var");
            let val = jstr(ev, "value");
            let key = jstr(ev, "key");
            if sv.is_empty() { continue; }
            let entry = format!("{key}|{sv}|{val}");
            match on {
                "key_press"     => kp.push(entry),
                "key_release"   => kr.push(entry),
                "mouse_press"   => mp.push(format!("|{sv}|{val}")),
                "mouse_release" => mr.push(format!("|{sv}|{val}")),
                "scroll"        => sc_list.push(format!("|{sv}|{val}")),
                "mouse_move"    => mm.push(format!("|{sv}|{val}")),
                other           => eprintln!("[json_layout] unknown event \"{other}\""),
            }
        }
    }

    if !kp.is_empty()      { cv.set_var("__ev_kp", kp.join("\n")); }
    if !kr.is_empty()      { cv.set_var("__ev_kr", kr.join("\n")); }
    if !mp.is_empty()      { cv.set_var("__ev_mp", mp.join("\n")); }
    if !mr.is_empty()      { cv.set_var("__ev_mr", mr.join("\n")); }
    if !sc_list.is_empty() { cv.set_var("__ev_sc", sc_list.join("\n")); }
    if !mm.is_empty()      { cv.set_var("__ev_mm", mm.join("\n")); }


    cv.on_key_press(|cv, key| {
        let key_str = match key {
            prism::event::Key::Character(s) => s.to_string(),
            prism::event::Key::Named(n)     => format!("{n:?}"),
        };
        let Some(raw) = cv.get_var("__ev_kp") else { return };
        for line in raw.to_display_string().lines() {
            let p: Vec<&str> = line.splitn(3, '|').collect();
            if p.len() != 3 { continue; }
            let (filter, var, tmpl) = (p[0].to_lowercase(), p[1].to_string(), p[2].to_string());
            if filter != "any" && filter != key_str.to_lowercase() { continue; }
            cv.set_var(var, tmpl.replace("$key", &key_str));
        }
    });

    cv.on_key_release(|cv, key| {
        let key_str = match key {
            prism::event::Key::Character(s) => s.to_string(),
            prism::event::Key::Named(n)     => format!("{n:?}"),
        };
        let Some(raw) = cv.get_var("__ev_kr") else { return };
        for line in raw.to_display_string().lines() {
            let p: Vec<&str> = line.splitn(3, '|').collect();
            if p.len() != 3 { continue; }
            let (filter, var, tmpl) = (p[0].to_lowercase(), p[1].to_string(), p[2].to_string());
            if filter != "any" && filter != key_str.to_lowercase() { continue; }
            cv.set_var(var, tmpl.replace("$key", &key_str));
        }
    });

    cv.on_mouse_press(|cv, btn, (mx, my)| {
        let btn_str = match btn {
            crate::types::MouseButton::Left   => "left",
            crate::types::MouseButton::Right  => "right",
            crate::types::MouseButton::Middle => "middle",
        };
        if let Some(raw) = cv.get_var("__ev_mp") {
            for line in raw.to_display_string().lines() {
                let p: Vec<&str> = line.splitn(3, '|').collect();
                if p.len() != 3 { continue; }
                let (var, tmpl) = (p[1].to_string(), p[2].to_string());
                cv.set_var(var, tmpl
                    .replace("$btn", btn_str)
                    .replace("$x", &(mx as i32).to_string())
                    .replace("$y", &(my as i32).to_string()));
            }
        }
        if btn != crate::types::MouseButton::Left { return; }
        let Some(ids_val) = cv.get_var("__jl_ids") else { return };
        let ids_str = ids_val.to_display_string();
        for id in ids_str.split(',').filter(|s| !s.is_empty()) {
            let Some(vn_val) = cv.get_var(&format!("__jl_click_{id}")) else { continue };
            let vn = vn_val.to_display_string();
            if let Some(obj) = cv.get_game_object(id) {
                if !obj.visible { continue; }
                let (ox, oy) = obj.position;
                let (ow, oh) = obj.size;
                if mx >= ox && mx <= ox + ow && my >= oy && my <= oy + oh {
                    cv.set_var(vn, id.to_string());
                }
            }
        }
    });

    cv.on_mouse_release(|cv, btn, (mx, my)| {
        let btn_str = match btn {
            crate::types::MouseButton::Left   => "left",
            crate::types::MouseButton::Right  => "right",
            crate::types::MouseButton::Middle => "middle",
        };
        let Some(raw) = cv.get_var("__ev_mr") else { return };
        for line in raw.to_display_string().lines() {
            let p: Vec<&str> = line.splitn(3, '|').collect();
            if p.len() != 3 { continue; }
            let (var, tmpl) = (p[1].to_string(), p[2].to_string());
            cv.set_var(var, tmpl
                .replace("$btn", btn_str)
                .replace("$x", &(mx as i32).to_string())
                .replace("$y", &(my as i32).to_string()));
        }
    });

    cv.on_mouse_scroll(|cv, (_dx, dy)| {
        let dir = if dy > 0.0 { "up" } else { "down" };
        let Some(raw) = cv.get_var("__ev_sc") else { return };
        for line in raw.to_display_string().lines() {
            let p: Vec<&str> = line.splitn(3, '|').collect();
            if p.len() != 3 { continue; }
            let (var, tmpl) = (p[1].to_string(), p[2].to_string());
            cv.set_var(var, tmpl.replace("$dir", dir));
        }
    });

    cv.on_mouse_move(|cv, (mx, my)| {
        if let Some(raw) = cv.get_var("__ev_mm") {
            for line in raw.to_display_string().lines() {
                let p: Vec<&str> = line.splitn(3, '|').collect();
                if p.len() != 3 { continue; }
                let (var, tmpl) = (p[1].to_string(), p[2].to_string());
                cv.set_var(var, tmpl
                    .replace("$x", &(mx as i32).to_string())
                    .replace("$y", &(my as i32).to_string()));
            }
        }
        let Some(ids_val) = cv.get_var("__jl_ids") else { return };
        let ids_str = ids_val.to_display_string();
        for id in ids_str.split(',').filter(|s| !s.is_empty()) {
            let Some(vn_val) = cv.get_var(&format!("__jl_hover_{id}")) else { continue };
            let vn = vn_val.to_display_string();
            if let Some(obj) = cv.get_game_object(id) {
                if !obj.visible {
                    let cur = cv.get_var(&vn).map(|v| v.to_display_string()).unwrap_or_default();
                    if cur == id { cv.set_var(vn.clone(), String::new()); }
                    continue;
                }
                let (ox, oy) = obj.position;
                let (ow, oh) = obj.size;
                let hit = mx >= ox && mx <= ox + ow && my >= oy && my <= oy + oh;
                let cur = cv.get_var(&vn).map(|v| v.to_display_string()).unwrap_or_default();
                if hit  && cur != id { cv.set_var(vn.clone(), id.to_string()); }
                if !hit && cur == id { cv.set_var(vn.clone(), String::new()); }
            }
        }
    });
}

pub fn process_reactions(cv: &mut Canvas, fonts: &HashMap<String, Arc<Font>>) {
    let count = match cv.get_var("__rx_count") {
        Some(v) => {
            let s = v.to_display_string();
            s.trim_end_matches(".0").parse::<usize>().unwrap_or(0)
        }
        None => return,
    };
    if count == 0 { return; }

    struct Rx {
        watch:  String,
        cond:   String,
        equals: String,
        target: String,
        set:    String,
        els:    String,
        last:   String,
    }

    let mut rxs: Vec<Rx> = (0..count).map(|n| Rx {
        watch:  cv.get_var(&format!("__rx_{n}_w")).map(|v| v.to_display_string()).unwrap_or_default(),
        cond:   cv.get_var(&format!("__rx_{n}_c")).map(|v| v.to_display_string()).unwrap_or_default(),
        equals: cv.get_var(&format!("__rx_{n}_e")).map(|v| v.to_display_string()).unwrap_or_default(),
        target: cv.get_var(&format!("__rx_{n}_t")).map(|v| v.to_display_string()).unwrap_or_default(),
        set:    cv.get_var(&format!("__rx_{n}_s")).map(|v| v.to_display_string()).unwrap_or_default(),
        els:    cv.get_var(&format!("__rx_{n}_x")).map(|v| v.to_display_string()).unwrap_or_default(),
        last:   cv.get_var(&format!("__rx_{n}_last")).map(|v| v.to_display_string()).unwrap_or_default(),
    }).collect();

    let mut current_vals: HashMap<String, String> = HashMap::new();
    for rx in &rxs {
        if !rx.watch.is_empty() && !current_vals.contains_key(&rx.watch) {
            current_vals.insert(
                rx.watch.clone(),
                cv.get_var(&rx.watch).map(|v| v.to_display_string()).unwrap_or_default(),
            );
        }
    }

    for (n, rx) in rxs.iter_mut().enumerate() {
        if rx.watch.is_empty() || rx.target.is_empty() { continue; }
        let current = current_vals.get(&rx.watch).map(|s| s.as_str()).unwrap_or("");

        let matched = match rx.cond.as_str() {
            "not_empty" => !current.is_empty(),
            "empty"     => current.is_empty(),
            _           => current == rx.equals.as_str(),
        };
        let branch  = if matched { "1" } else { "0" };
        let dirty_key = format!("{current}\x00{branch}");
        if dirty_key == rx.last { continue; }
        rx.last = dirty_key.clone();
        cv.set_var(format!("__rx_{n}_last"), dirty_key);

        let raw = if matched { &rx.set } else { &rx.els };
        if raw.is_empty() { continue; }

        let props: serde_json::Value = match serde_json::from_str(raw) {
            Ok(v)  => v,
            Err(e) => {
                eprintln!("[json_layout reactions] parse error for rx target '{}': {e}", rx.target);
                continue;
            }
        };

        let resolved_text: Option<String> =
            if let Some(fmt) = props.get("text_format").and_then(|v| v.as_str()) {
                Some(resolve_format(cv, fmt))
            } else if let Some(vn) = props.get("text_from_var").and_then(|v| v.as_str()) {
                Some(cv.get_var(vn).map(|v| v.to_display_string()).unwrap_or_default())
            } else {
                None
            };

        let target = rx.target.clone();
        apply_props(cv, &target, &props, resolved_text.as_deref(), fonts);
    }
}

fn apply_props(
    cv:            &mut Canvas,
    target:        &str,
    props:         &serde_json::Value,
    resolved_text: Option<&str>,
    fonts:         &HashMap<String, Arc<Font>>,
) {
    if let Some(vis) = props.get("visible").and_then(|v| v.as_bool()) {
        if let Some(obj) = cv.get_game_object_mut(target) {
            obj.visible = vis;
        }
    }

    let has_color = props.get("color").is_some();
    let has_bc    = props.get("border_color").is_some();
    let has_bw    = props.get("border_width").is_some();

    if has_color || has_bc || has_bw {
        let color = parse_color_arr(
            props.get("color").unwrap_or(&serde_json::Value::Null),
            Color(10, 10, 10, 255),
        );
        let bc = parse_color_arr(
            props.get("border_color").unwrap_or(&serde_json::Value::Null),
            Color(50, 50, 50, 255),
        );
        let bw = props.get("border_width")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(0.0);

        // Scale border width by virtual_scale before querying the object size.
        let scaled_bw = bw * cv.virtual_scale();

        if let Some(obj) = cv.get_game_object_mut(target) {
            let (w, h) = obj.size;
            let img = if scaled_bw > 0.0 || has_bc {
                RoundedBox::new(w, h)
                    .radius((w.min(h) * 0.15).max(2.0))
                    .color(color)
                    .border(scaled_bw.max(1.0), bc)
                    .build()
            } else {
                tint_overlay(w, h, color)
            };
            obj.set_image(img);
        }
    }

    let text_str = resolved_text.or_else(|| props.get("text").and_then(|v| v.as_str()));
    if let Some(new_text) = text_str {
        let tc = parse_color_arr(
            props.get("text_color").unwrap_or(&serde_json::Value::Null),
            Color(255, 255, 255, 255),
        );
        let fs_px     = props.get("font_size").and_then(|v| v.as_f64()).map(|v| v as f32).unwrap_or(16.0);
        let font_name = props.get("font").and_then(|v| v.as_str()).unwrap_or("body");
        let font      = pick_font_by_name(font_name, fonts);
        let align     = props.get("align")
            .and_then(|v| v.as_str())
            .map(jalign_str)
            .unwrap_or(Align::Center);

        let d = make_text_scaled(cv, new_text, fs_px, tc, align, None, font);
        if let Some(obj) = cv.get_game_object_mut(target) {
            obj.set_drawable(Box::new(d));
        }
    }
}


fn spawn_object(cv: &mut Canvas, def: &serde_json::Value, fonts: &HashMap<String, Arc<Font>>) {
    let id   = jstr(def, "id");
    let kind = jstr(def, "kind");
    if id.is_empty() { return; }

    let scale   = cv.virtual_scale();
    let layer   = jf32(def, "layer", 5.0) as i32;
    let fill    = jbool(def, "fill_screen", false);
    let iz      = jbool(def, "ignore_zoom", false);
    let visible = jbool(def, "visible", true);
    let pin     = jstr(def, "pin");
    let pin_off_x = jf32(def, "pin_offset_x", 0.0) * scale;
    let pin_off_y = jf32(def, "pin_offset_y", 0.0) * scale;

    let (vw, vh) = cv.canvas_size();

    let raw_w = jf32(def, "w", 0.0);
    let raw_h = jf32(def, "h", 0.0);
    let mut w = if raw_w >= 10000.0 { vw } else { raw_w * scale };
    let mut h = if raw_h >= 10000.0 { vh } else { raw_h * scale };
    if fill || (w == 0.0 && h == 0.0 && kind == "rect") {
        w = vw;
        h = vh;
    }

    match kind {
        "rect" => {
            let color = jcolor(def, "color", Color(0, 0, 0, 255));
            let img   = tint_overlay(w, h, color);

            let mut b = GameObject::build(id)
                .size(w, h)
                .layer(layer)
                .image(img);

            b = apply_positioning(b, def, fill, iz, pin, pin_off_x, pin_off_y, w, h, vw, vh, scale);
            let mut obj = b.finish();
            obj.visible = visible;
            cv.add_game_object(id.to_string(), obj);
        }

        "rounded_rect" => {
            let color = jcolor(def, "color", Color(10, 10, 10, 255));
            let r     = (jf32(def, "radius", 8.0) * scale).min(h / 2.0).min(w / 2.0);
            let bw    = jf32(def, "border_width", 0.0) * scale;
            let bc    = jcolor(def, "border_color", Color(50, 50, 50, 255));

            let img = if bw > 0.0 {
                RoundedBox::new(w, h).radius(r).color(color).border(bw, bc).build()
            } else {
                RoundedBox::new(w, h).radius(r).color(color).build()
            };

            let mut b = GameObject::build(id)
                .size(w, h)
                .layer(layer)
                .image(img);

            b = apply_positioning(b, def, fill, iz, pin, pin_off_x, pin_off_y, w, h, vw, vh, scale);
            let mut obj = b.finish();
            obj.visible = visible;
            cv.add_game_object(id.to_string(), obj);
        }

        "text" => {
            let text_str = jstr(def, "text");
            let fs_px    = jf32(def, "font_size", 16.0);
            let color    = jcolor(def, "color", Color(240, 240, 245, 255));
            let align    = jalign(def, "align");
            let max_w    = if w > 0.0 { Some(w) } else { None };
            let font     = pick_font(def, fonts);

            let d = make_text_scaled(cv, text_str, fs_px, color, align, max_w, font);

            let (tw, th) = d.size();
            let obj_w = if tw > 0.0 { tw.max(w) } else { w };
            let obj_h = if th > 0.0 { th.max(h) } else { h };

            let mut b = GameObject::build(id)
                .size(obj_w, obj_h)
                .layer(layer);

            b = apply_positioning(b, def, fill, iz, pin, pin_off_x, pin_off_y, w, h, vw, vh, scale);
            let mut obj = b.finish();
            obj.set_drawable(Box::new(d));
            obj.visible = visible;
            cv.add_game_object(id.to_string(), obj);
        }

        other => eprintln!("[json_layout] unknown kind \"{other}\" (id \"{id}\")"),
    }
}


fn apply_positioning(
    mut b:       crate::object::GameObjectBuilder,
    def:         &serde_json::Value,
    fill:        bool,
    iz:          bool,
    pin:         &str,
    pin_off_x:   f32,
    pin_off_y:   f32,
    w:           f32,
    h:           f32,
    vw:          f32,
    vh:          f32,
    scale:       f32,
) -> crate::object::GameObjectBuilder {
    if fill {
        b = b.fill_screen();
        return b;
    }

    if !pin.is_empty() {
        b = match pin {
            "top_left"      => b.pin_top_left(pin_off_x, pin_off_y),
            "top_right"     => b.pin_top_right(pin_off_x, pin_off_y),
            "top_center"    => b.pin_top_center(pin_off_y).pin_offset(pin_off_x, 0.0),
            "bottom_left"   => b.pin_bottom_left(pin_off_x, pin_off_y),
            "bottom_right"  => b.pin_bottom_right(pin_off_x, pin_off_y),
            "bottom_center" => b.pin_bottom_center(pin_off_y).pin_offset(pin_off_x, 0.0),
            "center"        => b.pin_center().pin_offset(pin_off_x, pin_off_y),
            other           => {
                eprintln!("[json_layout] unknown pin \"{other}\" — defaulting to center");
                b.pin_center().pin_offset(pin_off_x, pin_off_y)
            }
        };
        return b;
    }

    if iz { b = b.ignore_zoom(); }

    let x = jf32(def, "x", 0.0) * scale;
    let y = jf32(def, "y", 0.0) * scale;

    if def.get("cx").is_some() {
        let cx = jf32(def, "cx", 0.0) * scale;
        b = b.center_at(vw / 2.0 + cx, y + h / 2.0);
    } else {
        b = b.position(x, y);
    }

    b
}


pub fn load_objects(cv: &mut Canvas, json_str: &str, fonts: &HashMap<String, Arc<Font>>) {
    let root: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v)  => v,
        Err(e) => { eprintln!("[json_layout] parse error: {e}"); return; }
    };
    let objects = match root["objects"].as_array() {
        Some(a) => a.clone(),
        None    => { eprintln!("[json_layout] missing \"objects\" array"); return; }
    };

    for def in &objects {
        spawn_object(cv, def, fonts);
    }

    init_vars(cv, json_str);
}

pub fn reload_objects(cv: &mut Canvas, json_str: &str, fonts: &HashMap<String, Arc<Font>>) {
    for id in collect_ids(json_str) { cv.remove_game_object(&id); }
    load_objects(cv, json_str, fonts);
}

pub fn load_objects_from_file(cv: &mut Canvas, path: &str, fonts: &HashMap<String, Arc<Font>>) {
    match std::fs::read_to_string(path) {
        Ok(s) => {
            cv.set_var("__current_layout", path.to_string());
            load_objects(cv, &s, fonts);
        }
        Err(e) => eprintln!("[json_layout] cannot read \"{path}\": {e}"),
    }
}

pub fn reload_objects_from_file(cv: &mut Canvas, path: &str, fonts: &HashMap<String, Arc<Font>>) {
    let prev_path = cv.get_var("__current_layout")
        .map(|v| v.to_display_string())
        .unwrap_or_default();

    if !prev_path.is_empty() {
        if let Ok(s) = std::fs::read_to_string(&prev_path) {
            for id in collect_ids(&s) { cv.remove_game_object(&id); }
        }
    }

    match std::fs::read_to_string(path) {
        Ok(s) => {
            cv.set_var("__current_layout", path.to_string());
            load_objects(cv, &s, fonts);
        }
        Err(e) => eprintln!("[json_layout] cannot read \"{path}\": {e}"),
    }
}