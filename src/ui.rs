//! The 4.0 look for every menu (the big UI revamp): one backdrop, one header, one way to show a chosen row, text boxes,
//! switches, sliders and key hints at the bottom. Same colours as the 3.0 title: near-black, Helix red, Claude orange.

use super::*;

pub const ACCENT: Color = Color::new(0.85, 0.3, 0.35, 1.0);
pub const WARM: Color = Color::new(0.85, 0.47, 0.32, 1.0);
pub const INK: Color = Color::new(0.9, 0.9, 0.95, 1.0);
pub const SOFT: Color = Color::new(0.62, 0.63, 0.7, 1.0);
pub const DIM: Color = Color::new(0.42, 0.43, 0.5, 1.0);
pub const GOOD: Color = Color::new(0.45, 0.95, 0.5, 1.0);
pub const BAD: Color = Color::new(0.95, 0.5, 0.45, 1.0);
pub const LEFT: f32 = 64.0;

fn a(c: Color, k: f32) -> Color { Color::new(c.r, c.g, c.b, c.a * k) }

/// The menu background: a dark room with drifting dust and a faint red glow from below.
pub fn backdrop() {
    let t = get_time() as f32;
    clear_background(Color::new(0.03, 0.025, 0.035, 1.0));
    let (x0, w) = (-margin(), vw());
    for k in 0..12 {
        let f = k as f32 / 12.0;
        draw_rectangle(x0, VH * (0.55 + f * 0.45), w, VH * 0.04 + 1.0, Color::new(0.35, 0.05, 0.08, 0.035 * f));
    }
    for i in 0..40u32 {
        let h = i.wrapping_mul(2654435761);
        let x = x0 + ((h % 1000) as f32 / 1000.0 * w + t * (4.0 + (i % 5) as f32)) % w;
        let y = ((h / 1000 % 1000) as f32 / 1000.0 * VH - t * (3.0 + (i % 3) as f32)).rem_euclid(VH);
        draw_circle(x, y, 1.0 + (i % 3) as f32 * 0.5, Color::new(1.0, 0.9, 0.8, 0.05 + 0.04 * ((t + i as f32).sin() * 0.5 + 0.5)));
    }
    // A thin accent line down the left edge, like the title.
    draw_rectangle(x0, 0.0, 3.0, VH, a(ACCENT, 0.6));
}

/// A darkened game behind a menu (pause, game over).
pub fn dim_over_game(k: f32) {
    draw_rectangle(-margin(), 0.0, vw(), VH, Color::new(0.01, 0.0, 0.02, k));
}

/// Big title, a small "HOLLOW HOURS" over it, and a line under it.
pub fn header(title: &str, sub: &str) {
    draw_text("HOLLOW HOURS", LEFT, 46.0, 15.0, a(ACCENT, 0.8));
    draw_text(title, LEFT - 2.0, 96.0, 50.0, INK);
    let w = measure_text(title, None, 50, 1.0).width;
    draw_rectangle(LEFT, 108.0, w.min(VW - 2.0 * LEFT), 2.0, ACCENT);
    if !sub.is_empty() {
        for (i, l) in wrap(sub, VW - 2.0 * LEFT, 18.0).iter().take(2).enumerate() {
            draw_text(l, LEFT, 136.0 + i as f32 * 22.0, 18.0, SOFT);
        }
    }
}

/// A centered header (pause, game over, consent).
pub fn header_center(title: &str, sub: &str, y: f32, color: Color) {
    centered(title, y, 60.0, color);
    let w = measure_text(title, None, 60, 1.0).width.min(VW - 120.0);
    draw_rectangle(VW / 2.0 - w / 2.0, y + 14.0, w, 2.0, a(color, 0.7));
    if !sub.is_empty() { centered(sub, y + 46.0, 19.0, SOFT); }
}

/// A card.
pub fn panel(x: f32, y: f32, w: f32, h: f32) {
    draw_rectangle(x, y, w, h, Color::new(0.07, 0.065, 0.085, 0.88));
    draw_rectangle_lines(x, y, w, h, 1.0, Color::new(1.0, 1.0, 1.0, 0.08));
    draw_rectangle(x, y, 3.0, h, a(ACCENT, 0.8));
}

/// A list row's highlight (the chosen one glows, with a bar on the left).
pub fn row(x: f32, y: f32, w: f32, h: f32, on: bool) {
    if on {
        let pulse = 0.85 + 0.15 * (get_time() as f32 * 3.0).sin();
        for k in 0..8 {
            let f = k as f32 / 8.0;
            draw_rectangle(x + w * f, y, w / 8.0 + 1.0, h, Color::new(WARM.r, WARM.g, WARM.b, 0.28 * (1.0 - f) * pulse));
        }
        draw_rectangle(x, y, 4.0, h, WARM);
    } else {
        draw_rectangle(x, y, w, h, Color::new(1.0, 1.0, 1.0, 0.025));
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
        let ry = y + i as f32 * 50.0;
        let (over, c) = hit(x, ry, w, 42.0);
        if over { hover = Some(i); click |= c; }
        row(x, ry, w, 42.0, i == sel);
        draw_text(label, x + 22.0, ry + 29.0, 26.0, if i == sel { WHITE } else { SOFT });
    }
    (hover, click)
}

/// A text box: its label over it, the text, a blinking cursor when focused. `hidden` shows dots.
pub fn field(label: &str, value: &str, focused: bool, hidden: bool, x: f32, y: f32, w: f32) {
    draw_text(&label.to_uppercase(), x, y, 14.0, if focused { WARM } else { DIM });
    let by = y + 8.0;
    draw_rectangle(x, by, w, 44.0, Color::new(1.0, 1.0, 1.0, if focused { 0.09 } else { 0.04 }));
    draw_rectangle(x, by + 42.0, w, 2.0, if focused { WARM } else { Color::new(1.0, 1.0, 1.0, 0.15) });
    let shown = if hidden { "•".repeat(value.chars().count()) } else { value.to_string() };
    let shown = if hidden && measure_text(&shown, None, 24, 1.0).width < 1.0 { "*".repeat(value.chars().count()) } else { shown };
    let cursor = if focused && (get_time() * 2.0) as i32 % 2 == 0 { "_" } else { "" };
    draw_text(&format!("{shown}{cursor}"), x + 12.0, by + 30.0, 24.0, INK);
}

/// An ON/OFF switch.
pub fn switch(on: bool, x: f32, y: f32) {
    let (w, h) = (46.0, 22.0);
    draw_rectangle(x, y - h + 4.0, w, h, if on { a(GOOD, 0.35) } else { Color::new(1.0, 1.0, 1.0, 0.1) });
    draw_rectangle_lines(x, y - h + 4.0, w, h, 1.0, if on { GOOD } else { DIM });
    let kx = if on { x + w - h + 2.0 } else { x + 2.0 };
    draw_rectangle(kx, y - h + 6.0, h - 4.0, h - 4.0, if on { GOOD } else { SOFT });
    draw_text(if on { "ON" } else { "OFF" }, x + w + 10.0, y - 1.0, 18.0, if on { GOOD } else { BAD });
}

/// A slider 0..1 with its percent.
pub fn slider(v: f32, x: f32, y: f32, w: f32) {
    let v = v.clamp(0.0, 1.0);
    draw_rectangle(x, y - 10.0, w, 6.0, Color::new(1.0, 1.0, 1.0, 0.12));
    draw_rectangle(x, y - 10.0, w * v, 6.0, WARM);
    draw_circle(x + w * v, y - 7.0, 7.0, WHITE);
    draw_text(&format!("{}%", (v * 100.0).round() as i32), x + w + 14.0, y - 1.0, 18.0, INK);
}

/// Key hints along the bottom: [Enter] play  [Esc] back ...
pub fn hints(keys: &[(&str, &str)]) {
    let y = VH - 26.0;
    draw_rectangle(-margin(), y - 24.0, vw(), 50.0, Color::new(0.0, 0.0, 0.0, 0.35));
    let mut x = LEFT;
    for (k, label) in keys {
        let kw = measure_text(k, None, 15, 1.0).width + 14.0;
        draw_rectangle(x, y - 15.0, kw, 22.0, Color::new(1.0, 1.0, 1.0, 0.1));
        draw_rectangle(x, y + 5.0, kw, 2.0, Color::new(1.0, 1.0, 1.0, 0.18));
        draw_text(k, x + 7.0, y + 1.0, 15.0, INK);
        x += kw + 8.0;
        draw_text(label, x, y + 1.0, 16.0, SOFT);
        x += measure_text(label, None, 16, 1.0).width + 22.0;
        if x > VW - 120.0 { break; }
    }
}

/// A message (errors in orange, the rest soft), just above the hints.
pub fn status(msg: &str) {
    if msg.is_empty() { return; }
    let w = measure_text(msg, None, 18, 1.0).width + 28.0;
    draw_rectangle(LEFT, VH - 92.0, w, 32.0, Color::new(1.0, 0.6, 0.3, 0.12));
    draw_rectangle(LEFT, VH - 92.0, 3.0, 32.0, Color::new(1.0, 0.7, 0.4, 1.0));
    draw_text(msg, LEFT + 14.0, VH - 70.0, 18.0, Color::new(1.0, 0.8, 0.55, 1.0));
}

/// Tabs (settings, servers): returns the tab clicked, if any.
pub fn tabs(names: &[&str], sel: usize, y: f32) -> Option<usize> {
    let mut x = LEFT;
    let mut clicked = None;
    for (i, t) in names.iter().enumerate() {
        let w = measure_text(t, None, 18, 1.0).width + 28.0;
        let (over, c) = hit(x, y, w, 32.0);
        if c { clicked = Some(i); }
        if i == sel {
            draw_rectangle(x, y, w, 32.0, a(WARM, 0.9));
        } else {
            draw_rectangle(x, y, w, 32.0, Color::new(1.0, 1.0, 1.0, if over { 0.1 } else { 0.04 }));
        }
        draw_text(t, x + 14.0, y + 22.0, 18.0, if i == sel { WHITE } else { SOFT });
        x += w + 6.0;
    }
    clicked
}
