//! Drawing the game: the world (2D and the 3D view's overlays), speech bubbles, the HUD and its meters.
//! Split out of main.rs in 4.0.

use super::*;

// ---------------- Drawing helpers ----------------

pub(crate) fn wrap(text: &str, width: f32, size: f32) -> Vec<String> {
    let mut lines = vec![];
    let mut line = String::new();
    for w in text.split_whitespace() {
        let test = if line.is_empty() { w.to_string() } else { format!("{line} {w}") };
        if measure_text(&test, None, size as u16, 1.0).width > width && !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            line = w.to_string();
        } else {
            line = test;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

pub(crate) fn bubble(text: &str, via: &str, x: f32, y: f32) {
    let fs = if settings::with(|s| s.big_text) { 24.0 } else { 18.0 };
    let w = fs / 18.0 * 280.0;
    let lines = wrap(text, w - 20.0, fs);
    let h = lines.len() as f32 * fs + if via.is_empty() { 10.0 } else { 24.0 };
    let bx = (x - w / 2.0).clamp(4.0, VW - w - 4.0);
    let by = (y - h - 26.0).max(4.0);
    draw_rectangle(bx, by, w, h, Color::new(0.06, 0.06, 0.09, 0.92));
    draw_rectangle_lines(bx, by, w, h, 2.0, Color::new(1.0, 1.0, 1.0, 0.35));
    for (i, l) in lines.iter().enumerate() {
        draw_text(l, bx + 10.0, by + fs + 2.0 + i as f32 * fs, fs, WHITE);
    }
    if !via.is_empty() {
        draw_text(via, bx + 10.0, by + h - 7.0, 13.0, Color::new(0.6, 0.65, 0.75, 1.0));
    }
}

pub(crate) fn meter(x: f32, y: f32, label: &str, v: f32, color: Color, glitch: f32) {
    // 4.0 HUD: a small caps label, a slim bar with a soft glow, and the number.
    let off = if glitch > 0.5 { rand::gen_range(-glitch, glitch) * 2.0 } else { 0.0 };
    let v = v.clamp(0.0, 1.0);
    draw_text(&label.to_uppercase(), x + off, y - 2.0, 14.0, Color::new(0.72, 0.74, 0.8, 1.0));
    let (bx, by, bw) = (x + 100.0, y - 11.0, 150.0);
    draw_rectangle(bx, by, bw, 8.0, Color::new(1.0, 1.0, 1.0, 0.08));
    draw_rectangle(bx, by - 2.0, bw * v, 12.0, Color::new(color.r, color.g, color.b, 0.18));
    draw_rectangle(bx, by, bw * v, 8.0, color);
    draw_text(&format!("{}", (v * 100.0).round() as i32), bx + bw + 8.0, y - 2.0, 15.0, Color::new(0.85, 0.86, 0.9, 1.0));
}

/// A soft round light: transparent in the middle, dark at the edge (used for darkness and the fear vignette).
pub(crate) fn light_texture() -> Texture2D {
    let n = 256u16;
    let mut img = Image::gen_image_color(n, n, Color::new(0.0, 0.0, 0.0, 0.0));
    for y in 0..n {
        for x in 0..n {
            let d = vec2(x as f32 - 128.0, y as f32 - 128.0).length() / 128.0;
            let a = ((d - 0.35) / 0.65).clamp(0.0, 1.0);
            img.set_pixel(x as u32, y as u32, Color::new(0.0, 0.0, 0.02, a * a * (3.0 - 2.0 * a)));
        }
    }
    Texture2D::from_image(&img)
}

/// The game is drawn on a fixed 960x660 canvas, then scaled to fill the window (full screen too), keeping its shape
/// with black bars at the sides or top. VW x VH is that canvas.
pub(crate) fn draw_world(g: &Game, light: &Texture2D, view: &RenderTarget) {
    let (floor, wall, m_name, drained) = (map_floor(g.map), map_wall(g.map), map_name(g.map), map_drained(g.map));
    let shake = if g.fear > 0.75 && !settings::with(|s| s.reduce_motion) { rand::gen_range(-1.5, 1.5) } else { 0.0 };
    let (sw, sh) = (MW as f32 * T, MH as f32 * T);
    let cam = view3d::camera(g, None);
    // Where something on this map shows up on screen: its tile spot in 2D, projected from 3D in first person.
    let at = |p: Vec2, height: f32| -> Option<Vec2> {
        if g.view3d { view3d::project(&cam, vec3(p.x / T, height, p.y / T)) } else { Some(p) }
    };
    if g.view3d {
        draw_texture_ex(&view.texture, -margin(), 0.0, WHITE, DrawTextureParams { dest_size: Some(vec2(vw(), sh)), flip_y: true, ..Default::default() });
        // Fear closes in at the edges of your eyes.
        let r = (1.25 - g.fear * 0.45) * sw;
        draw_texture_ex(light, sw / 2.0 - r / 2.0, sh / 2.0 - r / 2.0, Color::new(1.0, 1.0, 1.0, 0.55 + g.fear * 0.4),
            DrawTextureParams { dest_size: Some(vec2(r, r)), ..Default::default() });
        let edge = Color::new(0.0, 0.0, 0.02, 0.55 + g.fear * 0.4);
        let (lx, ly) = (sw / 2.0 - r / 2.0, sh / 2.0 - r / 2.0);
        let (left, right) = (-margin(), sw + margin());
        if ly > 0.0 { draw_rectangle(left, 0.0, right - left, ly, edge); draw_rectangle(left, ly + r, right - left, sh - ly - r, edge); }
        if lx > left { draw_rectangle(left, ly.max(0.0), lx - left, r.min(sh), edge); }
        if lx + r < right { draw_rectangle(lx + r, ly.max(0.0), right - lx - r, r.min(sh), edge); }
        // Names over heads (only the ones close enough to see).
        for c in g.chars.iter().filter(|c| c.map == g.map && c.pos.distance(g.pos) < 9.0 * T) {
            if let Some(p) = at(c.pos, 1.4) {
                draw_text(c.name, p.x - measure_text(c.name, None, 16, 1.0).width / 2.0, p.y, 16.0, Color::new(1.0, 1.0, 1.0, 0.85));
            }
        }
        for r in g.remotes.iter().filter(|r| r.map == g.map) {
            if let Some(p) = at(r.pos, 1.4) {
                let label = format!("{} {}%", r.name, (r.inf * 100.0) as i32);
                draw_text(&label, p.x - measure_text(&label, None, 15, 1.0).width / 2.0, p.y, 15.0, Color::new(0.75, 0.85, 1.0, 1.0));
            }
        }
        // A small crosshair.
        draw_circle(sw / 2.0, sh / 2.0, 2.0, Color::new(1.0, 1.0, 1.0, 0.5));
        if g.screen == Screen::Play && !g.grabbed && !pad::touch_mode() {
            draw_text("click to look around with the mouse  ·  Tab = 2D map", sw / 2.0 - 190.0, sh - 14.0, 16.0, Color::new(0.8, 0.8, 0.85, 0.7));
        }
    } else {
        // The sides of a wide screen: the map's walls carry on.
        if margin() > 0.0 { draw_rectangle(-margin(), 0.0, vw(), sh, Color::new(map_wall(g.map).r * 0.5, map_wall(g.map).g * 0.5, map_wall(g.map).b * 0.5, 1.0)); }
        // Tiles.
        for y in 0..MH {
            for x in 0..MW {
                let (px, py) = (x as f32 * T + shake, y as f32 * T + shake);
                let c = tile(g.map, x as i32, y as i32);
                let col = match c {
                    '#' => wall,
                    'b' => Color::new(0.42, 0.3, 0.22, 1.0),
                    '*' => Color::new(0.12, 0.3, 0.14, 1.0),          // a tree
                    'n' => Color::new(0.45, 0.32, 0.2, 1.0),          // the park bench
                    't' => Color::new(0.15, 0.17, 0.19, 1.0),         // a lab terminal
                    'o' => Color::new(0.2, 0.45, 0.35, 1.0),          // ASTRA's pod
                    'a' => Color::new(0.25, 0.75, 0.45, 1.0),         // ASTRA, in its pod (Helix Tower)
                    'c' => Color::new(0.45, 0.2, 0.2, 1.0),           // a couch / lounger
                    'k' => Color::new(0.7, 0.7, 0.72, 1.0),           // a counter
                    'f' => Color::new(0.88, 0.9, 0.92, 1.0),          // a fridge
                    'p' => Color::new(0.2, 0.45, 0.22, 1.0),          // a plant
                    'h' => Color::new(0.55, 0.4, 0.28, 1.0),          // a shop shelf
                    'v' => Color::new(0.2, 0.45, 0.75, 1.0),          // a vending machine
                    'q' => Color::new(0.5, 0.5, 0.52, 1.0),           // a pillar
                    'u' => Color::new(0.85, 0.3, 0.3, 1.0),           // a beach umbrella
                    'e' => Color::new(0.45, 0.45, 0.47, 1.0),         // a rock
                    'j' => Color::new(0.08, 0.2, 0.3, 1.0),           // a pond
                    'i' => Color::new(0.08, 0.09, 0.11, 1.0),         // a server rack
                    'z' => Color::new(0.6, 0.45, 0.28, 1.0),          // boxes
                    'r' => view3d::rug(g.map),                 // a rug / path / safety line
                    'x' => Color::new(floor.r * 0.7, floor.g * 0.7, floor.b * 0.7, 1.0), // debris
                    'Z' if !story::tower_open() => Color::new(0.45, 0.12, 0.12, 1.0), // the Tower door, sealed
                    '=' => Color::new(0.18, 0.17, 0.16, 1.0),         // subway tracks
                    'y' => Color::new(0.88, 0.88, 0.92, 1.0),         // your bed
                    'g' => Color::new(0.35, 0.38, 0.3, 1.0),          // the generator
                    'w' => Color::new(0.3, 0.45, 0.7, 1.0),           // Kigo's lookout window
                    '~' if !g.on(1) => Color::new(0.6, 0.62, 0.6, 1.0), // Water phobia off: no water at all (calm tiles)
                    '~' if drained => Color::new(0.55, 0.78, 0.8, 1.0), // the drained pool
                    '~' => Color::new(0.05, 0.12 + 0.03 * (get_time() as f32 + x as f32 * 0.3).sin(), 0.25, 1.0),
                    ',' => Color::new(floor.r * 1.1, floor.g * 1.05, floor.b * 0.9, 1.0),
                    c if c.is_ascii_uppercase() => Color::new(0.9, 0.85, 0.5, 1.0),
                    _ => floor,
                };
                draw_rectangle(px, py, T, T, col);
                if drained && c != '#' && (x + y) % 2 == 0 { draw_rectangle(px, py, T, T, Color::new(1.0, 1.0, 1.0, 0.05)); }
                if c == 'L' { draw_circle(px + T / 2.0, py + T / 2.0, 7.0, Color::new(1.0, 0.85, 0.5, 1.0)); }
                if c.is_ascii_uppercase() {
                    match passage::tile_kind(g.map, x as i32, y as i32) {
                        Some(passage::Kind::Stairs { up }) => {
                            // Stairs: steps getting lighter (up) or darker (down).
                            draw_rectangle(px, py, T, T, Color::new(0.4, 0.28, 0.18, 1.0));
                            for k in 0..4 {
                                let v = if up { 0.6 + k as f32 * 0.12 } else { 0.9 - k as f32 * 0.2 };
                                draw_rectangle(px + 2.0, py + 2.0 + k as f32 * 7.0, T - 4.0, 5.0, Color::new(0.5 * v, 0.36 * v, 0.24 * v, 1.0));
                            }
                        }
                        Some(passage::Kind::Door) => {
                            let open = passage::door_open(g, x as i32, y as i32);
                            draw_rectangle(px, py, T, T, Color::new(0.38 * (1.0 - open * 0.6), 0.25 * (1.0 - open * 0.6), 0.16 * (1.0 - open * 0.6), 1.0));
                            draw_circle(px + T * 0.72, py + T / 2.0, 3.0, Color::new(0.85, 0.72, 0.35, 1.0));
                        }
                        _ => {}
                    }
                }
            if c == 't' { draw_rectangle(px + 5.0, py + 6.0, T - 10.0, 12.0, Color::new(0.3, 0.95, 0.5, 0.8)); }
            if c == '=' { for k in [8.0, 22.0] { draw_rectangle(px, py + k, T, 3.0, Color::new(0.55, 0.55, 0.58, 1.0)); } }
            if c == '*' { draw_circle(px + T / 2.0, py + T / 2.0, T * 0.55, Color::new(0.15, 0.38, 0.17, 1.0)); }
            if c == 's' {
                draw_rectangle(px, py, T, T, Color::new(0.2, 0.2, 0.22, 1.0));
                let hot = g.fry_t > 0.0;
                for (bx, by) in [(8.0, 8.0), (24.0, 8.0), (8.0, 24.0), (24.0, 24.0)] {
                    draw_circle(px + bx, py + by, 5.0, if hot { Color::new(1.0, 0.45, 0.1, 1.0) } else { Color::new(0.35, 0.35, 0.37, 1.0) });
                }
            }
            if c == 'm' {
                draw_rectangle(px, py, T, T, Color::new(0.3, 0.22, 0.16, 1.0));
                draw_rectangle(px + 4.0, py + 6.0, 16.0, 10.0, Color::new(0.85, 0.47, 0.32, 1.0));
                draw_rectangle(px + 22.0, py + 8.0, 7.0, 7.0, Color::new(0.78, 0.79, 0.82, 1.0));
            }
                if c == 'd' && g.on(2) {
                    // A doll that turns its eyes toward you.
                    let (cx, cy) = (px + T / 2.0, py + T / 2.0);
                    draw_circle(cx, cy, 9.0, Color::new(0.95, 0.85, 0.85, 1.0));
                    let look = (g.pos - vec2(cx, cy)).normalize_or_zero() * 2.5;
                    draw_circle(cx - 3.0 + look.x, cy - 1.0 + look.y, 1.6, BLACK);
                    draw_circle(cx + 3.0 + look.x, cy - 1.0 + look.y, 1.6, BLACK);
                }
            }
        }
        // Supplies (infection mode): red apples (food), blue bottles (water).
        for (m, p, water) in &g.items {
            if *m != g.map { continue; }
            if *water {
                draw_rectangle(p.x - 4.0, p.y - 8.0, 8.0, 15.0, Color::new(0.35, 0.65, 1.0, 1.0));
                draw_rectangle(p.x - 2.0, p.y - 11.0, 4.0, 4.0, Color::new(0.8, 0.85, 0.95, 1.0));
            } else {
                draw_circle(p.x, p.y, 6.0, Color::new(0.85, 0.15, 0.15, 1.0));
                draw_line(p.x, p.y - 6.0, p.x + 2.0, p.y - 10.0, 2.0, Color::new(0.3, 0.6, 0.2, 1.0));
            }
        }
        // Characters.
        for c in g.chars.iter().filter(|c| c.map == g.map) {
            if c.kind { draw_circle_lines(c.pos.x + shake, c.pos.y + shake, 16.0, 2.0, g.rules.halo); }
            let disguised = c.mimic && c.crazy <= 0.0;
            let painted = c.disguise_t > 0.0; // a kind AI in disguise: green paint, green eyes, but its halo stays
        let body = if (c.infected && !disguised) || painted { Color::new(c.color.r * 0.45, c.color.g * 0.6 + 0.2, c.color.b * 0.45, 1.0) } else { c.color };
            draw_circle(c.pos.x + shake, c.pos.y + shake, 12.0, body);
            let flicker = disguised && (get_time() as f32 * 3.1 + c.pos.x).sin() > 0.985;
            let eye = if painted { g.rules.eye } else if disguised && !flicker { BLACK } else if c.infected { if c.stun > 0.0 { GRAY } else { g.rules.eye } } else if c.crazy > 0.0 { RED } else { BLACK };
            let look = (g.pos - c.pos).normalize_or_zero() * 2.0;
            draw_circle(c.pos.x - 4.0 + look.x, c.pos.y - 2.0 + look.y, 2.4, eye);
            draw_circle(c.pos.x + 4.0 + look.x, c.pos.y - 2.0 + look.y, 2.4, eye);
            draw_text(c.name, c.pos.x - measure_text(c.name, None, 16, 1.0).width / 2.0, c.pos.y + 26.0, 16.0, WHITE);
        }
        // The other humans in the room.
        for r in g.remotes.iter().filter(|r| r.map == g.map) {
            let col = if r.lc { Color::new(0.4, 0.95, 0.35, 1.0) } else { Color::new(0.85, 0.9, 1.0, if r.hid { 0.35 } else { 1.0 }) };
            draw_circle(r.pos.x, r.pos.y, 11.0, col);
            draw_circle_lines(r.pos.x, r.pos.y, 11.0, 2.0, Color::new(0.3, 0.5, 1.0, 1.0));
            let label = format!("{} {}%", r.name, (r.inf * 100.0) as i32);
            draw_text(&label, r.pos.x - measure_text(&label, None, 15, 1.0).width / 2.0, r.pos.y - 16.0, 15.0, Color::new(0.75, 0.85, 1.0, 1.0));
        }
        drone::draw_2d(g);
        combat::draw_items_2d(g);
        // You (see-through while hiding).
        let you = if g.hidden { Color::new(0.95, 0.95, 0.98, 0.35) } else if g.disguise_t > 0.0 { Color::new(0.55, 0.85, 0.5, 1.0) } else { Color::new(0.95, 0.95, 0.98, 1.0) };
        draw_circle(g.pos.x, g.pos.y, 11.0, you);
        draw_circle_lines(g.pos.x, g.pos.y, 11.0, 2.0, BLACK);
        // The tornado: a grey spinning funnel (a dashed circle where it will touch down during the warning).
        if let Some(t) = &g.tornado {
            if t.warn > 0.0 {
                let x = t.pos.x.clamp(T, (MW as f32 - 1.0) * T);
                for k in 0..12 {
                    let a = k as f32 / 12.0 * std::f32::consts::TAU + get_time() as f32;
                    draw_circle(x + a.cos() * g.rules.tornado_radius * T, t.pos.y + a.sin() * g.rules.tornado_radius * T, 3.0, Color::new(0.8, 0.8, 0.85, 0.6));
                }
            } else {
                for k in 0..6 {
                    let r = g.rules.tornado_radius * (1.0 - k as f32 * 0.14) * T;
                    let a = get_time() as f32 * (4.0 + k as f32) + k as f32;
                    draw_circle_lines(t.pos.x + a.cos() * 4.0, t.pos.y + a.sin() * 4.0, r, 3.0, Color::new(0.55, 0.55, 0.6, 0.75));
                }
            }
        }

        // Darkness: the room is dark except a soft circle of light around you (smaller when you're scared).
        let dark = if g.on(0) { map_dark(g.map) } else { 0.1 };
        let radius = (7.5 - g.fear * 3.0) * T;
        let a = (dark + g.fear * 0.25).min(0.97);
        let vp = drone::view_pos(g);
        let (lx, ly) = (vp.x - radius, vp.y - radius);
        let dim = Color::new(0.0, 0.0, 0.02, a);
        draw_texture_ex(light, lx, ly, Color::new(1.0, 1.0, 1.0, a), DrawTextureParams { dest_size: Some(vec2(radius * 2.0, radius * 2.0)), ..Default::default() });
        // Fill everything outside the light's square, without overlapping (overlaps showed up as darker bands).
        let (top, bottom) = (ly.max(0.0), (ly + radius * 2.0).min(sh));
        draw_rectangle(0.0, 0.0, sw, top, dim);
        draw_rectangle(0.0, bottom, sw, sh - bottom, dim);
        draw_rectangle(0.0, top, lx.max(0.0), bottom - top, dim);
        draw_rectangle(lx + radius * 2.0, top, (sw - (lx + radius * 2.0)).max(0.0), bottom - top, dim);
    }
    // Infection mode: your noise as a ring (they hear you inside it), and the world turning green as you turn.
    if g.infection {
        if g.noise > 0.05 && !g.lost_control && !g.view3d {
            draw_circle_lines(g.pos.x, g.pos.y, g.noise * g.rules.hearing * T, 2.0, Color::new(1.0, 1.0, 1.0, 0.25 * g.noise));
        }
        if g.view3d && !g.lost_control {
            // Your noise as a little bar under the crosshair (they hear you when it's high).
            draw_rectangle(sw / 2.0 - 40.0, sh / 2.0 + 24.0, 80.0 * g.noise, 3.0, Color::new(1.0, 1.0, 1.0, 0.5));
            if g.sneaking { draw_text("sneaking", sw / 2.0 - 26.0, sh / 2.0 + 44.0, 14.0, Color::new(0.7, 0.8, 1.0, 0.9)); }
        } else if g.sneaking && !g.lost_control { draw_text("sneaking", g.pos.x - 26.0, g.pos.y + 28.0, 14.0, Color::new(0.7, 0.8, 1.0, 0.9)); }
        if g.inf > 0.0 { draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.1, 0.8, 0.1, g.inf * 0.18)); }
        if g.lost_control {
            draw_rectangle(0.0, 0.0, sw, 44.0, Color::new(0.0, 0.0, 0.0, 0.7));
            draw_text("YOU ARE INFECTED: you can see, but you can't move. It's walking toward the kind ones.", 16.0, 28.0, 20.0, Color::new(0.4, 1.0, 0.35, 1.0));
        }
        for c in g.chars.iter().filter(|c| c.map == g.map && c.claude && !c.infected && c.infect > 0.0) {
            if let Some(p) = at(c.pos, 1.55) {
                draw_rectangle(p.x - 14.0, p.y - 22.0 * (!g.view3d) as i32 as f32, 28.0 * c.infect, 4.0, Color::new(0.35, 0.95, 0.3, 1.0));
            }
        }
    }
    // Liminality: everything goes a little grey and too quiet.
    if g.liminality > 0.3 { draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.6, 0.65, 0.7, (g.liminality - 0.3) * 0.25)); }
    // Discomfort: a faint wrong-coloured tint.
    if g.discomfort > 0.2 { draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.6, 0.0, 0.3, g.discomfort * 0.12)); }

    // Who's talking: a label over their head (same map), and a banner for anyone on the radio.
    let now = get_time();
    for r in &g.remotes {
        if let Some(radio) = g.voice.is_talking(&r.id, now) {
            if r.map == g.map {
                if let Some(p) = at(r.pos, 1.7) {
                    let t = if radio { "(radio)" } else { "(talking)" };
                    draw_text(t, p.x - measure_text(t, None, 15, 1.0).width / 2.0, p.y - if g.view3d { 0.0 } else { 34.0 }, 15.0, Color::new(0.55, 1.0, 0.6, 1.0));
                }
            }
            if radio {
                let t = format!("RADIO  {}", r.name);
                draw_rectangle(10.0, 10.0, measure_text(&t, None, 18, 1.0).width + 20.0, 28.0, Color::new(0.0, 0.0, 0.0, 0.7));
                draw_text(&t, 20.0, 30.0, 18.0, Color::new(1.0, 0.75, 0.3, 1.0));
            }
        }
    }
    if let Some(radio) = g.voice.talking {
        let t = if radio { "ON THE RADIO: everyone hears you" } else { "TALKING: players near you hear you" };
        let w = measure_text(t, None, 18, 1.0).width + 20.0;
        draw_rectangle(sw / 2.0 - w / 2.0, 10.0, w, 28.0, Color::new(0.0, 0.0, 0.0, 0.7));
        draw_text(t, sw / 2.0 - w / 2.0 + 10.0, 30.0, 18.0, if radio { Color::new(1.0, 0.75, 0.3, 1.0) } else { Color::new(0.55, 1.0, 0.6, 1.0) });
    }

    // Speech bubbles (drawn over the dark so you can read them).
    for c in g.chars.iter().filter(|c| c.map == g.map && c.bubble_t > 0.0) {
        if let Some(p) = at(c.pos, 1.3) { bubble(&c.bubble, &c.via, p.x, p.y); }
    }
    let prompt = if let (true, Some(i)) = (g.infection, disguise::robot_to_pay(g)) { Some(format!("E: pay {} H$ {}{}", g.chars[i].name, disguise::PAY, if g.disguise_t > 0.0 && !g.intel_from.contains(&g.chars[i].name) { " (receipt = intel)" } else { "" })) }
        else if story::can_face(g) { Some("E: face ASTRA".to_string()) }
        else if disguise::lab_ready(g) { Some("E: synthesize the ASTRA Cure (stay 15 s)".to_string()) }
        else if near_tile(g, 'Z') && !story::tower_open() && map_name(g.map) == "The Helix Lab" { Some(if g.story.is_some() { "Helix Tower: SEALED. (The Conductor in the Subway has the pass.)" } else { "Helix Tower: SEALED. (It opens once the ASTRA Cure is made.)" }.to_string()) }
        else if lore::at_drawer(g) && g.note.is_none() { Some("E: open the drawer".to_string()) }
        else if let Some(i) = nearest(g, 2.2) { Some(format!("E: talk to {}", g.chars[i].name)) }
        else if near_stove(g) { Some(if g.fry_t > 0.0 { format!("Frying... {:.0} s (don't walk away)", g.fry_t.ceil()) } else if let Some(c) = czech::best(&g.pantry) { format!("E: cook {}", czech::label(&c)) } else { "The stove (no ingredients: the Marketplace, or a grocery bag from the Helix Store)".to_string() }) }
        else if near_mac(g) { Some("E: use the Mac Studio".to_string()) }
        else if near_tile(g, 'y') { Some(if g.resting { "Resting (move or E to get up)".to_string() } else { "E: rest in your bed".to_string() }) }
        else if lore::near(g).is_some() && g.note.is_none() { Some("E: read".to_string()) } else { None };
    if let (Some(t), Screen::Play) = (prompt, g.screen) {
        if g.view3d { draw_text(&t, sw / 2.0 - measure_text(&t, None, 20, 1.0).width / 2.0, sh / 2.0 + 70.0, 20.0, YELLOW); }
        else { draw_text(&t, g.pos.x - 40.0, g.pos.y - 22.0, 16.0, YELLOW); }
    }

    if g.ai_t < 6.0 {
        let t = "Claude is playing  (your keys always win  ·  P = panic)";
        draw_rectangle(sw - measure_text(t, None, 16, 1.0).width - 24.0, 6.0, measure_text(t, None, 16, 1.0).width + 16.0, 24.0, Color::new(0.85, 0.47, 0.32, 0.9));
        draw_text(t, sw - measure_text(t, None, 16, 1.0).width - 16.0, 23.0, 16.0, WHITE);
    }
    // Flashing lights (only if turned on through the four warnings): lightning, and a strobe when a jumpscare starts.
    let (flashing, jumpscares, high_contrast, big) = settings::with(|s| (s.flashing, s.jumpscares, s.high_contrast, s.big_text));
    if flashing && g.flash_t > 0.0 {
        draw_rectangle(-margin(), 0.0, vw(), sh, Color::new(0.9, 0.92, 1.0, (g.flash_t / 0.15).min(1.0) * 0.8));
    }
    if flashing && jumpscares && g.scare_t > 0.45 && ((g.scare_t * 16.0) as i32) % 2 == 0 {
        draw_rectangle(-margin(), 0.0, vw(), sh, Color::new(1.0, 0.1, 0.1, 0.55));
    }
    // High-contrast infected: a yellow ! over every infected you can see (shape, not colour).
    if high_contrast {
        for c in g.chars.iter().filter(|c| c.map == g.map && c.infected && c.stun < combat::SHUT_DOWN / 2.0 && !(c.mimic && c.crazy <= 0.0)) {
            if let Some(p) = at(c.pos, 1.75) {
                let y = if g.view3d { p.y } else { p.y - 30.0 };
                draw_triangle(vec2(p.x, y - 22.0), vec2(p.x - 13.0, y), vec2(p.x + 13.0, y), Color::new(1.0, 0.85, 0.0, 1.0));
                draw_text("!", p.x - 3.0, y - 3.0, 20.0, BLACK);
            }
        }
    }
    // Going through a door / up the stairs: a short blink between places.
    let f = passage::fade(g);
    if f > 0.0 { draw_rectangle(-margin(), 0.0, vw(), sh, Color::new(0.0, 0.0, 0.0, f)); }
    // Captions (accessibility): what you'd hear, as text.
    let caps = g.sfx.recent_captions();
    for (k, c) in caps.iter().rev().enumerate() {
        let size = if big { 24.0 } else { 18.0 };
        let w = measure_text(c, None, size as u16, 1.0).width + 16.0;
        let y = sh - 50.0 - k as f32 * (size + 10.0);
        draw_rectangle(sw / 2.0 - w / 2.0, y - size, w, size + 8.0, Color::new(0.0, 0.0, 0.0, 0.75));
        draw_text(c, sw / 2.0 - w / 2.0 + 8.0, y, size, WHITE);
    }
    // Jumpscare: a face fills the screen for a moment (one quick fade; it strobes only with flashing lights on).
    if g.scare_t > 0.0 && jumpscares {
        let k = (g.scare_t / 0.7).min(1.0);
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.85 * k));
        draw_circle(sw / 2.0, sh / 2.0, 220.0 * (1.2 - k * 0.2), Color::new(g.scare_face.r, g.scare_face.g, g.scare_face.b, k));
        draw_circle(sw / 2.0 - 70.0, sh / 2.0 - 30.0, 26.0, Color::new(1.0, 0.0, 0.0, k));
        draw_circle(sw / 2.0 + 70.0, sh / 2.0 - 30.0, 26.0, Color::new(1.0, 0.0, 0.0, k));
    }

    if g.weak_t > 0.0 && g.screen == Screen::Play {
        let t = format!("{}  {:.0} s", if g.water <= 0.0 { "DEHYDRATED: find water" } else { "STARVING: find food" }, (60.0 - g.weak_t).max(0.0).ceil());
        let w = measure_text(&t, None, 24, 1.0).width;
        draw_text(&t, sw / 2.0 - w / 2.0, 70.0, 24.0, Color::new(1.0, 0.3, 0.25, 0.6 + 0.4 * (get_time() as f32 * 4.0).sin().abs()));
    }
    if g.infection && (g.drone.is_some() || g.drone_charge < 0.999) && g.screen == Screen::Play {
        let (b, flying) = g.drone.as_ref().map(|d| (d.battery, true)).unwrap_or((g.drone_charge, false));
        let label = if flying { format!("DRONE {}%  (Q to bring it back)", (b * 100.0) as i32) } else { format!("drone charging {}%", (b * 100.0) as i32) };
        draw_text(&label, 14.0, sh - 26.0, 16.0, Color::new(0.5, 0.85, 1.0, 0.9));
        draw_rectangle(14.0, sh - 20.0, 120.0, 5.0, Color::new(0.0, 0.0, 0.0, 0.5));
        draw_rectangle(14.0, sh - 20.0, 120.0 * b, 5.0, Color::new(0.4, 0.8, 1.0, 0.9));
    }
    if g.fry_t > 0.0 {
        draw_rectangle(sw / 2.0 - 80.0, sh - 34.0, 160.0, 6.0, Color::new(0.0, 0.0, 0.0, 0.5));
        draw_rectangle(sw / 2.0 - 80.0, sh - 34.0, 160.0 * (1.0 - g.fry_t / g.fry_max.max(0.1)).clamp(0.0, 1.0), 6.0, Color::new(1.0, 0.55, 0.15, 0.9));
    }
    if g.infection && g.screen == Screen::Play {
        let t = format!("H$ {}", g.credits);
        draw_text(&t, sw - measure_text(&t, None, 16, 1.0).width - 14.0, sh - 28.0, 16.0, Color::new(1.0, 0.85, 0.35, 0.9));
    }
    if czech::total(&g.pantry) > 0 && g.infection {
        let t = czech::summary(&g.pantry);
        draw_text(&t, sw - measure_text(&t, None, 16, 1.0).width - 14.0, sh - 10.0, 16.0, Color::new(1.0, 0.7, 0.45, 0.9));
    }
    if g.resting {
        let breathe = 0.86 + 0.05 * (get_time() as f32 * 0.9).sin();
        draw_rectangle(-margin(), 0.0, vw(), sh, Color::new(0.0, 0.0, 0.01, breathe));
        let t = format!("Resting...   infection {}%   (move or E to get up)", (g.inf * 100.0) as i32);
        draw_text(&t, sw / 2.0 - measure_text(&t, None, 20, 1.0).width / 2.0, sh / 2.0, 20.0, Color::new(0.7, 0.72, 0.8, 0.9));
    }
    combat::draw_hud(g, sw, sh);
    disguise::draw_hud(g, sw, sh);
    lore::draw(g, sw, sh);
    // Stamina (only while it isn't full).
    if g.stamina < 0.999 && g.screen == Screen::Play {
        let col = if g.winded { Color::new(0.95, 0.35, 0.3, 0.9) } else { Color::new(0.95, 0.9, 0.6, 0.85) };
        draw_rectangle(sw / 2.0 - 80.0, sh - 18.0, 160.0, 6.0, Color::new(0.0, 0.0, 0.0, 0.5));
        draw_rectangle(sw / 2.0 - 80.0, sh - 18.0, 160.0 * g.stamina, 6.0, col);
        if g.winded { draw_text("winded", sw / 2.0 - 22.0, sh - 24.0, 15.0, col); }
    }
    // HUD.
    let y0 = sh;
    draw_rectangle(-margin(), y0, vw(), HUD, g.rules.hud);
    // 4.0: a thin accent line on top of the HUD (green while you're infected enough to worry, red otherwise).
    let accent = if g.infection && g.inf > 0.5 { Color::new(0.35, 0.95, 0.3, 0.8) } else { Color::new(0.85, 0.3, 0.35, 0.8) };
    draw_rectangle(-margin(), y0, vw(), 2.0, accent);
    draw_rectangle(-margin(), y0 + 2.0, vw(), 6.0, Color::new(accent.r, accent.g, accent.b, 0.07));
    if !g.infection { meter(14.0, y0 + 24.0, "Fear", g.fear, Color::new(0.9, 0.2, 0.2, 1.0), 0.0); }
    if g.infection {
        meter(14.0, y0 + 24.0, "Infection", g.inf, Color::new(0.35, 0.95, 0.3, 1.0), if g.inf > 0.5 { g.inf * 2.0 } else { 0.0 });
        meter(14.0, y0 + 48.0, "Food", g.food, Color::new(0.9, 0.55, 0.2, 1.0), 0.0);
        meter(14.0, y0 + 72.0, "Water", g.water, Color::new(0.3, 0.6, 1.0, 1.0), 0.0);
    } else {
        meter(14.0, y0 + 48.0, "Discomfort", g.discomfort, Color::new(0.75, 0.3, 0.75, 1.0), g.discomfort * 3.0);
        meter(14.0, y0 + 72.0, "Liminality", g.liminality, Color::new(0.55, 0.7, 0.85, 1.0), 0.0);
    }
    if g.infection {
        let infected = g.chars.iter().filter(|c| c.infected).count();
        draw_text(&format!("INFECTION · last human · {} min {} s · {} infected", (g.survived / 60.0) as i32, g.survived as i32 % 60, infected),
            sw - 470.0, y0 + 48.0, 16.0, Color::new(0.5, 0.95, 0.4, 1.0));
        if g.hidden { draw_text("HIDDEN (H to come out)", sw - 470.0, y0 + 24.0, 18.0, Color::new(0.8, 0.85, 1.0, 1.0)); }
        if let Some(n) = &g.net {
            let status = if !n.connected { format!("room {}: connecting...", n.room) } else {
                format!("room {} · {} players · {}", n.room, n.players, if n.is_host() { "you're the host" } else { "guest" }) };
            draw_text(&status, sw - 250.0, y0 + 24.0, 16.0, Color::new(0.55, 0.75, 1.0, 1.0));
        }
        if g.tornado.as_ref().is_some_and(|t| t.warn > 0.0) && (get_time() * 2.0) as i32 % 2 == 0 {
            draw_text("TORNADO WARNING", sw / 2.0 - 120.0, 40.0, 32.0, Color::new(1.0, 0.85, 0.2, 1.0));
        }
    }
    let courage = match g.courage { Courage::Brave => "Brave", Courage::Normal => "Normal", Courage::Phobias => "Phobias" };
    draw_text(&format!("{}  ·  {}  ·  {}", m_name, g.time_text(), courage), 320.0, y0 + 24.0, 18.0, Color::new(0.85, 0.85, 0.9, 1.0));
    // Recent events, cut to fit so they never run into the status text on the right.
    let max_w = if g.infection { sw - 480.0 - 320.0 } else { sw - 400.0 - 320.0 };
    for (i, l) in g.log.iter().rev().take(2).enumerate() {
        let mut t = l.clone();
        while t.len() > 3 && measure_text(&t, None, 16, 1.0).width > max_w {
            t.pop();
            while !t.is_char_boundary(t.len()) { t.pop(); }
            if !t.ends_with("...") { t.truncate(t.trim_end_matches('.').len()); }
        }
        if &t != l { t = format!("{}...", t.trim_end_matches('.')); }
        draw_text(&t, 320.0, y0 + 48.0 + i as f32 * 20.0, 16.0, Color::new(0.65, 0.68, 0.75, 1.0));
    }
    draw_text(if pad::name().is_some() { "stick · L3 sprint · A use · B hit · D-right kick · X hide · LT sneak · D-up drone · View PANIC" } else if g.infection { if g.net.is_some() { "WASD · Space sprint · Shift sneak · H hide · E talk · V voice · R radio · P PANIC" } else if g.view3d { "WASD · mouse · Space sprint · Shift sneak · H hide · E use · F hit · G kick · Q drone · P PANIC" } else { "WASD · Space sprint · Shift sneak · H hide · E use · F hit · G kick · Q drone · P PANIC" } } else { "WASD move · E talk · P PANIC (everything off) · Esc menu" },
        sw - 380.0, y0 + 72.0, 14.0, Color::new(0.5, 0.52, 0.6, 1.0));
    if g.vest.enabled { draw_text("vest ON", sw - 70.0, y0 + 24.0, 16.0, Color::new(0.4, 0.9, 0.5, 1.0)); }
    story::draw(g, sw, sh);
}

pub(crate) fn draw_phobias(g: &Game) {
    ui::backdrop();
    ui::header("Your phobias", "ON = the world uses it to scare you. OFF = it never happens. Switch off anything you can't handle.");
    for (i, (name, what)) in PHOBIAS.iter().enumerate() {
        let on = g.phobia[i];
        let y = 190.0 + i as f32 * 52.0;
        ui::row(ui::LEFT, y, VW - 2.0 * ui::LEFT, 44.0, false);
        draw_text(&format!("{}", i + 1), ui::LEFT + 16.0, y + 29.0, 22.0, ui::DIM);
        draw_text(name, ui::LEFT + 50.0, y + 29.0, 24.0, ui::INK);
        draw_text(what, ui::LEFT + 260.0, y + 28.0, 18.0, ui::SOFT);
        ui::switch(on, VW - ui::LEFT - 110.0, y + 32.0);
    }
    ui::hints(&[("1-6", "switch"), ("Enter", "start"), ("Esc", "back")]);
}

