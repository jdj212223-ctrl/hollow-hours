//! The look for every screen (4.0, the second UI revamp): "cozy to corrupt". The whole interface starts warm (lamp light, cream text,
//! soft rounded cards, like the cozy house) and gets infected along with you: the colours drain into sick green,
//! the text glitches, scanlines and tears creep in, and when every Claude AI is corrupted it bleeds red.
//! `set_rot` (once a frame, from main) says how corrupted things are; every helper here follows it, and
//! `overlay` (drawn over everything, last) adds the vignette, scanlines and tears. Reduce motion stops the shaking,
//! and the tears only flash with flashing lights allowed.

use super::*;
use std::cell::Cell;

pub const ACCENT: Color = Color::new(0.92, 0.42, 0.36, 1.0);
pub const WARM: Color = Color::new(0.96, 0.66, 0.36, 1.0);
pub const INK: Color = Color::new(0.97, 0.93, 0.86, 1.0);
pub const SOFT: Color = Color::new(0.74, 0.68, 0.63, 1.0);
pub const DIM: Color = Color::new(0.50, 0.45, 0.43, 1.0);
pub const GOOD: Color = Color::new(0.55, 0.92, 0.55, 1.0);
pub const BAD: Color = Color::new(0.98, 0.52, 0.45, 1.0);
pub const LEFT: f32 = 64.0;

const SICK: Color = Color::new(0.42, 0.98, 0.38, 1.0);
const SICK_INK: Color = Color::new(0.78, 0.98, 0.76, 1.0);
const BLOOD: Color = Color::new(0.95, 0.06, 0.08, 1.0);

thread_local! {
    static ROT: Cell<f32> = const { Cell::new(0.0) };
    static BLEED: Cell<f32> = const { Cell::new(0.0) };
}

/// How corrupted the interface is (0 cozy .. 1 fully infected), and whether every Claude AI is gone (blood red).
/// Eases toward the target so it creeps instead of jumping.
pub fn set_rot(target: f32, all_gone: bool, dt: f32) {
    let k = (dt * 1.5).min(1.0);
    ROT.with(|r| r.set(r.get() + (target.clamp(0.0, 1.0) - r.get()) * k));
    BLEED.with(|b| b.set(b.get() + (if all_gone { 1.0 } else { 0.0 } - b.get()) * k));
}
pub fn rot() -> f32 { ROT.with(|r| r.get()) }
fn bleed() -> f32 { BLEED.with(|b| b.get()) }

fn a(c: Color, k: f32) -> Color { Color::new(c.r, c.g, c.b, c.a * k) }
pub fn mix(x: Color, y: Color, k: f32) -> Color {
    let k = k.clamp(0.0, 1.0);
    Color::new(x.r + (y.r - x.r) * k, x.g + (y.g - x.g) * k, x.b + (y.b - x.b) * k, x.a + (y.a - x.a) * k)
}
/// A colour as the infection sees it: drained toward green, then toward blood red.
pub fn tint(c: Color) -> Color {
    let g = mix(c, Color::new(c.r * 0.35 + SICK.r * 0.4, c.g * 0.35 + SICK.g * 0.6, c.b * 0.35 + SICK.b * 0.4, c.a), rot() * 0.85);
    mix(g, Color::new(BLOOD.r, BLOOD.g * 0.5 + g.g * 0.1, BLOOD.b, c.a), bleed() * 0.8)
}
pub fn accent() -> Color { mix(mix(ACCENT, SICK, rot()), BLOOD, bleed()) }
pub fn warm() -> Color { mix(mix(WARM, SICK, rot()), BLOOD, bleed()) }
pub fn ink() -> Color { mix(mix(INK, SICK_INK, rot() * 0.8), Color::new(1.0, 0.82, 0.8, 1.0), bleed()) }
pub fn soft() -> Color { tint(SOFT) }
pub fn dim() -> Color { tint(DIM) }
fn bg() -> Color { mix(mix(Color::new(0.075, 0.05, 0.05, 1.0), Color::new(0.02, 0.07, 0.03, 1.0), rot()), Color::new(0.09, 0.0, 0.01, 1.0), bleed()) }

fn calm() -> bool { settings::with(|s| s.reduce_motion) }

/// A small shake (more as it rots); none with reduce motion.
pub fn jitter(seed: f32) -> Vec2 {
    let r = rot().max(bleed());
    if r < 0.35 || calm() { return Vec2::ZERO; }
    let t = (get_time() as f32 * 14.0).floor() + seed * 7.3;
    let n = |v: f32| ((v * 12.9898).sin() * 43758.547).fract() - 0.5;
    if n(t * 1.7) > 0.5 - r * 0.45 { vec2(n(t) * 6.0 * r, n(t + 3.1) * 2.0 * r) } else { Vec2::ZERO }
}

/// The game's pixel font has no arrow symbols: spell them out.
pub fn arrows(s: &str) -> String {
    s.replace("↑↓", "Up/Down").replace("←→", "Left/Right").replace('↑', "Up").replace('↓', "Down").replace('←', "Left").replace('→', "Right")
}

/// Text that glitches as it rots: now and then letters flip to blocks and symbols.
pub fn glitch(s: &str) -> String {
    let s = &arrows(s);
    let r = rot().max(bleed() * 0.6);
    if r < 0.3 { return s.to_string(); }
    let t = (get_time() as f32 * 9.0).floor();
    const JUNK: [char; 8] = ['#', '%', '&', '$', '@', '?', '/', '\\'];
    s.chars().enumerate().map(|(i, c)| {
        let h = (((i as f32 + t * 3.7) * 78.233).sin() * 43758.547).fract().abs();
        if c != ' ' && h < (r - 0.3) * 0.06 { JUNK[(h * 1000.0) as usize % JUNK.len()] } else { c }
    }).collect()
}

/// Text in the current look: glitched, and split into red and green ghosts as it rots.
pub fn text(s: &str, x: f32, y: f32, size: f32, c: Color) {
    let s = glitch(s);
    let r = rot().max(bleed());
    let j = jitter(x + y);
    if r > 0.45 && !calm() {
        let o = (r - 0.45) * 4.0;
        draw_text(&s, x + j.x - o, y + j.y, size, Color::new(1.0, 0.1, 0.2, 0.35 * c.a));
        draw_text(&s, x + j.x + o, y + j.y, size, Color::new(0.2, 1.0, 0.4, 0.35 * c.a));
    }
    draw_text(&s, x + j.x, y + j.y, size, c);
}

/// A rounded box (no overlapping parts, so see-through colours stay even).
pub fn rrect(x: f32, y: f32, w: f32, h: f32, r: f32, c: Color) {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    if r < 1.0 { draw_rectangle(x, y, w, h, c); return; }
    draw_rectangle(x + r, y, w - 2.0 * r, h, c);
    draw_rectangle(x, y + r, r, h - 2.0 * r, c);
    draw_rectangle(x + w - r, y + r, r, h - 2.0 * r, c);
    use std::f32::consts::{FRAC_PI_2, PI};
    const N: usize = 6;
    for (cx, cy, a0) in [(x + r, y + r, PI), (x + w - r, y + r, -FRAC_PI_2), (x + w - r, y + h - r, 0.0), (x + r, y + h - r, FRAC_PI_2)] {
        for k in 0..N {
            let (p, q) = (a0 + k as f32 / N as f32 * FRAC_PI_2, a0 + (k + 1) as f32 / N as f32 * FRAC_PI_2);
            draw_triangle(vec2(cx, cy), vec2(cx + r * p.cos(), cy + r * p.sin()), vec2(cx + r * q.cos(), cy + r * q.sin()), c);
        }
    }
}
/// A rounded outline.
pub fn rrect_lines(x: f32, y: f32, w: f32, h: f32, r: f32, th: f32, c: Color) {
    let r = r.min(w / 2.0).min(h / 2.0);
    draw_line(x + r, y, x + w - r, y, th, c);
    draw_line(x + r, y + h, x + w - r, y + h, th, c);
    draw_line(x, y + r, x, y + h - r, th, c);
    draw_line(x + w, y + r, x + w, y + h - r, th, c);
    let arc = |cx: f32, cy: f32, a0: f32| {
        for k in 0..6 {
            let (p, q) = (a0 + k as f32 / 6.0 * std::f32::consts::FRAC_PI_2, a0 + (k + 1) as f32 / 6.0 * std::f32::consts::FRAC_PI_2);
            draw_line(cx + r * p.cos(), cy + r * p.sin(), cx + r * q.cos(), cy + r * q.sin(), th, c);
        }
    };
    use std::f32::consts::PI;
    arc(x + r, y + r, PI); arc(x + w - r, y + r, -PI / 2.0); arc(x + w - r, y + h - r, 0.0); arc(x + r, y + h - r, PI / 2.0);
}

/// A soft glow behind something (a few stacked, fading rounded boxes).
fn glow(x: f32, y: f32, w: f32, h: f32, c: Color, k: f32) {
    for i in 1..5 {
        let g = i as f32 * 3.0;
        rrect(x - g, y - g, w + 2.0 * g, h + 2.0 * g, 10.0 + g, a(c, k * 0.05 * (5 - i) as f32));
    }
}

/// The menu background: a lamp-lit room (warm glow, floating dust like in lamp light). As it rots the light
/// turns green, the dust becomes spores rising, and dark veins creep in from the corners.
pub fn backdrop() {
    let t = get_time() as f32;
    clear_background(bg());
    let (x0, w) = (-margin(), vw());
    // The lamp's glow from the lower left, and a cooler window light from the top right.
    for k in 0..28 {
        let f = k as f32 / 28.0;
        draw_poly(x0 + w * 0.18, VH * 0.95, 96, VH * (1.25 - f), 0.0, a(tint(Color::new(0.95, 0.5, 0.22, 1.0)), 0.009));
    }
    for k in 0..8 {
        let f = k as f32 / 8.0;
        draw_poly(x0 + w * 0.9, -VH * 0.1, 96, VH * (0.8 - f * 0.6), 0.0, a(tint(Color::new(0.45, 0.5, 0.85, 1.0)), 0.012));
    }
    // Dust in the lamp light (spores as it rots: they rise faster and glow green).
    let r = rot();
    for i in 0..56u32 {
        let h = i.wrapping_mul(2654435761);
        let sp = 1.0 + r * 3.0;
        let x = x0 + ((h % 1000) as f32 / 1000.0 * w + t * (3.0 + (i % 5) as f32) * (1.0 - r)).rem_euclid(w);
        let y = ((h / 1000 % 1000) as f32 / 1000.0 * VH - t * (2.0 + (i % 3) as f32) * sp).rem_euclid(VH);
        let tw = 0.5 + 0.5 * (t * 1.3 + i as f32).sin();
        draw_circle(x, y, 0.8 + (i % 3) as f32 * 0.6 + r, a(tint(Color::new(1.0, 0.85, 0.6, 1.0)), 0.05 + 0.08 * tw));
    }
    // Veins from the corners.
    if r > 0.15 || bleed() > 0.1 {
        let vc = a(mix(Color::new(0.1, 0.5, 0.12, 1.0), BLOOD, bleed()), 0.35 * r.max(bleed()));
        for (cx, cy, dir) in [(x0, 0.0, 1.0f32), (x0 + w, VH, -1.0)] {
            for v in 0..7u32 {
                let (mut px, mut py) = (cx, cy);
                let ang = 0.15 + v as f32 * 0.2;
                let len = 40.0 + 260.0 * r.max(bleed()) * (0.5 + 0.5 * ((v * 7) as f32).sin().abs());
                for s in 0..10 {
                    let wob = ((s as f32 + v as f32 * 3.0) * 1.7 + t * 0.4).sin() * 0.35;
                    let nx = px + dir * (ang + wob).cos() * len / 10.0;
                    let ny = py + dir * (ang + wob).sin() * len / 10.0;
                    draw_line(px, py, nx, ny, 2.5 - s as f32 * 0.2, vc);
                    px = nx; py = ny;
                }
            }
        }
    }
    // A warm accent seam down the left edge.
    draw_rectangle(x0, 0.0, 3.0, VH, a(accent(), 0.7));
    draw_rectangle(x0 + 3.0, 0.0, 10.0, VH, a(accent(), 0.06));
}

/// A darkened game behind a menu (pause, game over), with a frosted warm edge.
pub fn dim_over_game(k: f32) {
    draw_rectangle(-margin(), 0.0, vw(), VH, a(bg(), k));
    for i in 0..10 {
        let f = i as f32 / 10.0;
        draw_rectangle(-margin(), VH * (0.7 + f * 0.3), vw(), VH * 0.03 + 1.0, a(tint(Color::new(0.6, 0.25, 0.1, 1.0)), 0.04 * f));
    }
}

/// Big title, a small "HOLLOW HOURS" with a lamp-dot over it, and a fading line under it.
pub fn header(title: &str, sub: &str) {
    let pulse = 0.6 + 0.4 * (get_time() as f32 * 2.0).sin();
    draw_circle(LEFT + 4.0, 41.0, 4.0, a(warm(), pulse));
    draw_circle(LEFT + 4.0, 41.0, 8.0, a(warm(), 0.15 * pulse));
    text("HOLLOW HOURS", LEFT + 16.0, 46.0, 15.0, a(accent(), 0.9));
    text(title, LEFT, 98.0, 52.0, a(Color::new(0.0, 0.0, 0.0, 1.0), 0.5)); // shadow
    text(title, LEFT - 2.0, 96.0, 52.0, ink());
    let w = measure_text(title, None, 52, 1.0).width.min(VW - 2.0 * LEFT);
    for k in 0..16 {
        let f = k as f32 / 16.0;
        draw_rectangle(LEFT + w * f * 1.4, 108.0, w * 1.4 / 16.0 + 1.0, 2.0, a(accent(), 1.0 - f));
    }
    if !sub.is_empty() {
        for (i, l) in wrap(sub, VW - 2.0 * LEFT, 18.0).iter().take(2).enumerate() {
            text(l, LEFT, 138.0 + i as f32 * 22.0, 18.0, soft());
        }
    }
}

/// A centered header (pause, game over, consent).
pub fn header_center(title: &str, sub: &str, y: f32, color: Color) {
    let w = measure_text(title, None, 60, 1.0).width.min(VW - 120.0);
    glow(VW / 2.0 - w / 2.0, y - 44.0, w, 54.0, tint(color), 0.6);
    text(title, VW / 2.0 - w / 2.0 + 2.0, y + 2.0, 60.0, a(BLACK, 0.5));
    text(title, VW / 2.0 - w / 2.0, y, 60.0, tint(color));
    for k in 0..20 {
        let f = (k as f32 / 20.0 - 0.5).abs() * 2.0;
        draw_rectangle(VW / 2.0 - w / 2.0 + w * k as f32 / 20.0, y + 14.0, w / 20.0 + 1.0, 2.0, a(tint(color), 0.8 * (1.0 - f)));
    }
    if !sub.is_empty() {
        let sw = measure_text(sub, None, 19, 1.0).width;
        text(sub, VW / 2.0 - sw / 2.0, y + 46.0, 19.0, soft());
    }
}

/// A card: soft shadow, rounded, a lamp-warm top edge.
pub fn panel(x: f32, y: f32, w: f32, h: f32) {
    rrect(x + 3.0, y + 5.0, w, h, 12.0, Color::new(0.0, 0.0, 0.0, 0.35));
    rrect(x, y, w, h, 12.0, a(mix(Color::new(0.13, 0.095, 0.09, 1.0), Color::new(0.04, 0.1, 0.05, 1.0), rot()), 0.93));
    rrect_lines(x, y, w, h, 12.0, 1.0, a(ink(), 0.09));
    draw_rectangle(x + 12.0, y, w - 24.0, 2.0, a(warm(), 0.55));
}

/// A list row's highlight: the chosen one is a warm glowing pill with a ›, the others a faint line.
pub fn row(x: f32, y: f32, w: f32, h: f32, on: bool) {
    let j = jitter(y);
    let (x, y) = (x + j.x, y + j.y);
    if on {
        let pulse = 0.85 + 0.15 * (get_time() as f32 * 3.0).sin();
        glow(x, y, w, h, warm(), 0.7 * pulse);
        rrect(x, y, w, h, h / 2.6, a(warm(), 0.22 * pulse));
        rrect_lines(x, y, w, h, h / 2.6, 1.0, a(warm(), 0.6));
        let (cx, cy, k) = (x + w - 18.0, y + h / 2.0, h * 0.16);
        draw_triangle(vec2(cx - k, cy - k * 1.6), vec2(cx - k, cy + k * 1.6), vec2(cx + k * 1.2, cy), warm());
    } else {
        draw_rectangle(x + 10.0, y + h, w - 20.0, 1.0, a(ink(), 0.05));
    }
}

/// Is the mouse over this box, and was it clicked this frame?
pub fn hit(x: f32, y: f32, w: f32, h: f32) -> (bool, bool) {
    let (mx, my) = virtual_mouse();
    let over = mx >= x && mx <= x + w && my >= y && my <= y + h;
    (over, over && is_mouse_button_pressed(MouseButton::Left))
}

/// A list of choices (the pause menu, game over): returns (row under the mouse, clicked).
pub fn choices(items: &[&str], sel: usize, x: f32, y: f32, w: f32) -> (Option<usize>, bool) {
    let mut hover = None;
    let mut click = false;
    for (i, label) in items.iter().enumerate() {
        let ry = y + i as f32 * 52.0;
        let (over, c) = hit(x, ry, w, 42.0);
        if over { hover = Some(i); click |= c; }
        row(x, ry, w, 42.0, i == sel);
        text(label, x + 24.0, ry + 29.0, 26.0, if i == sel { ink() } else { soft() });
    }
    (hover, click)
}

/// A text box: its label over it, the text, a blinking cursor when focused. `hidden` shows dots.
pub fn field(label: &str, value: &str, focused: bool, hidden: bool, x: f32, y: f32, w: f32) {
    text(&label.to_uppercase(), x + 4.0, y, 14.0, if focused { warm() } else { dim() });
    let by = y + 8.0;
    if focused { glow(x, by, w, 44.0, warm(), 0.5); }
    rrect(x, by, w, 44.0, 10.0, a(ink(), if focused { 0.10 } else { 0.05 }));
    rrect_lines(x, by, w, 44.0, 10.0, 1.0, if focused { warm() } else { a(ink(), 0.14) });
    let shown = if hidden { "•".repeat(value.chars().count()) } else { value.to_string() };
    let shown = if hidden && measure_text(&shown, None, 24, 1.0).width < 1.0 { "*".repeat(value.chars().count()) } else { shown };
    let cursor = if focused && (get_time() * 2.0) as i32 % 2 == 0 { "_" } else { "" };
    draw_text(&format!("{shown}{cursor}"), x + 14.0, by + 30.0, 24.0, ink());
}

/// An ON/OFF switch: a pill with a round knob.
pub fn switch(on: bool, x: f32, y: f32) {
    let (w, h) = (48.0, 24.0);
    let top = y - h + 4.0;
    let c = if on { tint(GOOD) } else { dim() };
    rrect(x, top, w, h, h / 2.0, if on { a(c, 0.35) } else { a(ink(), 0.08) });
    rrect_lines(x, top, w, h, h / 2.0, 1.0, c);
    let kx = if on { x + w - h / 2.0 } else { x + h / 2.0 };
    if on { draw_circle(kx, top + h / 2.0, h / 2.0 + 3.0, a(c, 0.2)); }
    draw_circle(kx, top + h / 2.0, h / 2.0 - 3.0, if on { c } else { soft() });
    text(if on { "ON" } else { "OFF" }, x + w + 10.0, y - 1.0, 18.0, if on { c } else { tint(BAD) });
}

/// A slider 0..1 with its percent.
pub fn slider(v: f32, x: f32, y: f32, w: f32) {
    let v = v.clamp(0.0, 1.0);
    rrect(x, y - 10.0, w, 6.0, 3.0, a(ink(), 0.12));
    rrect(x, y - 10.0, (w * v).max(6.0), 6.0, 3.0, warm());
    draw_circle(x + w * v, y - 7.0, 11.0, a(warm(), 0.2));
    draw_circle(x + w * v, y - 7.0, 7.0, ink());
    text(&format!("{}%", (v * 100.0).round() as i32), x + w + 16.0, y - 1.0, 18.0, ink());
}

/// A key drawn as a little keycap (a lighter top, a darker lip).
pub fn keycap(k: &str, x: f32, y: f32, size: f32) -> f32 {
    let k = &arrows(k);
    let kw = measure_text(k, None, size as u16, 1.0).width + 14.0;
    let h = size + 8.0;
    rrect(x, y - h + 6.0, kw, h, 5.0, a(BLACK, 0.45));
    rrect(x, y - h + 3.0, kw, h - 2.0, 5.0, a(ink(), 0.14));
    rrect_lines(x, y - h + 3.0, kw, h - 2.0, 5.0, 1.0, a(ink(), 0.2));
    draw_text(k, x + 7.0, y - 3.0, size, WHITE);
    kw
}

/// Key hints along the bottom: [Enter] play  [Esc] back ...
pub fn hints(keys: &[(&str, &str)]) {
    let y = VH - 24.0;
    for i in 0..8 {
        let f = i as f32 / 8.0;
        draw_rectangle(-margin(), y - 30.0 + f * 54.0, vw(), 7.0, a(BLACK, 0.06 + f * 0.06));
    }
    let mut x = LEFT;
    for (k, label) in keys {
        x += keycap(k, x, y, 15.0) + 8.0;
        text(label, x, y - 2.0, 16.0, soft());
        x += measure_text(label, None, 16, 1.0).width + 24.0;
        if x > VW - 120.0 { break; }
    }
}

/// A message toast (errors warm, the rest soft), just above the hints.
pub fn status(msg: &str) {
    if msg.is_empty() { return; }
    let w = measure_text(msg, None, 18, 1.0).width + 32.0;
    let c = tint(Color::new(1.0, 0.72, 0.42, 1.0));
    glow(LEFT, VH - 94.0, w, 34.0, c, 0.5);
    rrect(LEFT, VH - 94.0, w, 34.0, 17.0, a(c, 0.14));
    rrect_lines(LEFT, VH - 94.0, w, 34.0, 17.0, 1.0, a(c, 0.6));
    text(msg, LEFT + 16.0, VH - 71.0, 18.0, c);
}

/// Tabs (settings, servers): pills; returns the tab clicked, if any.
pub fn tabs(names: &[&str], sel: usize, y: f32) -> Option<usize> {
    let mut x = LEFT;
    let mut clicked = None;
    for (i, t) in names.iter().enumerate() {
        let w = measure_text(t, None, 18, 1.0).width + 30.0;
        let (over, c) = hit(x, y, w, 32.0);
        if c { clicked = Some(i); }
        if i == sel {
            glow(x, y, w, 32.0, warm(), 0.6);
            rrect(x, y, w, 32.0, 16.0, a(warm(), 0.9));
            text(t, x + 15.0, y + 22.0, 18.0, Color::new(0.12, 0.06, 0.04, 1.0));
        } else {
            rrect(x, y, w, 32.0, 16.0, a(ink(), if over { 0.12 } else { 0.05 }));
            text(t, x + 15.0, y + 22.0, 18.0, soft());
        }
        x += w + 8.0;
    }
    clicked
}

/// A HUD meter: small caps label, a rounded bar with a glow, and the number.
pub fn meter(x: f32, y: f32, label: &str, v: f32, color: Color, shake: f32) {
    let off = if shake > 0.5 && !calm() { rand::gen_range(-shake, shake) * 2.0 } else { 0.0 };
    let v = v.clamp(0.0, 1.0);
    text(&label.to_uppercase(), x + off, y - 2.0, 14.0, soft());
    let (bx, by, bw) = (x + 100.0, y - 12.0, 150.0);
    rrect(bx, by, bw, 9.0, 4.5, a(ink(), 0.08));
    if v > 0.0 {
        rrect(bx - 2.0, by - 2.0, bw * v + 4.0, 13.0, 6.5, a(color, 0.16));
        rrect(bx, by, (bw * v).max(9.0), 9.0, 4.5, color);
        draw_rectangle(bx + 4.0, by + 1.5, (bw * v - 8.0).max(0.0), 1.5, a(WHITE, 0.25));
    }
    draw_text(&format!("{}", (v * 100.0).round() as i32), bx + bw + 10.0, y - 2.0, 15.0, ink());
}

/// A speech bubble (rounded, with a little tail toward whoever is talking).
pub fn bubble_box(x: f32, y: f32, w: f32, h: f32, tail_x: f32) {
    rrect(x + 2.0, y + 4.0, w, h, 10.0, a(BLACK, 0.4));
    rrect(x, y, w, h, 10.0, a(mix(Color::new(0.14, 0.1, 0.09, 1.0), Color::new(0.04, 0.1, 0.05, 1.0), rot()), 0.95));
    rrect_lines(x, y, w, h, 10.0, 1.5, a(warm(), 0.45));
    let tx = tail_x.clamp(x + 14.0, x + w - 14.0);
    draw_triangle(vec2(tx - 8.0, y + h), vec2(tx + 8.0, y + h), vec2(tx, y + h + 10.0), a(mix(Color::new(0.14, 0.1, 0.09, 1.0), Color::new(0.04, 0.1, 0.05, 1.0), rot()), 0.95));
}

/// Drawn last, over every screen: a soft vignette; as it rots, a green cast, scanlines, a slow roll bar and
/// (with flashing lights allowed) sudden tears; blood red when every Claude AI is corrupted.
pub fn overlay() {
    let (x0, w) = (-margin(), vw());
    let r = rot();
    let b = bleed();
    let t = get_time() as f32;
    // Vignette: darker edges, tinted by the infection.
    let vc = mix(mix(Color::new(0.05, 0.02, 0.0, 1.0), Color::new(0.0, 0.08, 0.01, 1.0), r), Color::new(0.25, 0.0, 0.0, 1.0), b);
    for i in 0..10 {
        let f = i as f32 / 10.0;
        let k = 0.035 * (1.0 - f) * (1.0 + r * 1.5 + b * 2.0);
        let e = 6.0 + f * 46.0;
        draw_rectangle(x0, 0.0, w, e, a(vc, k));
        draw_rectangle(x0, VH - e, w, e, a(vc, k));
        draw_rectangle(x0, 0.0, e, VH, a(vc, k));
        draw_rectangle(x0 + w - e, 0.0, e, VH, a(vc, k));
    }
    if r > 0.05 {
        draw_rectangle(x0, 0.0, w, VH, Color::new(0.1, 0.9, 0.2, 0.05 * r));
        let mut y = 0.0;
        while y < VH { draw_rectangle(x0, y, w, 1.0, Color::new(0.0, 0.0, 0.0, 0.13 * r)); y += 3.0; }
        if !calm() {
            let ry = (t * 60.0).rem_euclid(VH + 80.0) - 40.0;
            draw_rectangle(x0, ry, w, 40.0, Color::new(0.6, 1.0, 0.6, 0.025 * r));
        }
    }
    if b > 0.05 {
        let beat = if calm() { 0.6 } else { 0.6 + 0.4 * ((t * 2.2).sin().max(0.0)).powf(6.0) };
        draw_rectangle(x0, 0.0, w, VH, Color::new(0.8, 0.0, 0.02, 0.12 * b * beat));
    }
    let tears = settings::with(|s| s.flashing) && !calm();
    if tears && r.max(b) > 0.4 {
        let n = |v: f32| ((v * 12.9898).sin() * 43758.547).fract().abs();
        let tick = (t * 12.0).floor();
        if n(tick) < (r.max(b) - 0.4) * 0.5 {
            for k in 0..3 {
                let ty = n(tick + k as f32) * VH;
                let th = 2.0 + n(tick * 2.0 + k as f32) * 10.0;
                let c = if b > 0.5 { Color::new(1.0, 0.1, 0.1, 0.25) } else { Color::new(0.5, 1.0, 0.5, 0.22) };
                draw_rectangle(x0 + (n(tick + 9.0) - 0.5) * 30.0, ty, w, th, c);
            }
        }
    }
}
