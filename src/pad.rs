//! Controllers and touch screens. Both become the game's keys, depending on the screen, so every screen works
//! without a keyboard.
//!
//! Controllers (Xbox, PlayStation, Switch Pro, Steam Deck / Legion Go through Steam Input...):
//!   Title:      A = Infection   X = Normal   Y = the story   RB = server list   LB = log in
//!   Playing:    left stick = walk   right stick = look (3D)   A = talk / use   X = hide   Y = 2D/3D map
//!               LT or LB = sneak (hold)   L3 (click the left stick) = sprint (hold)   RB = voice   RT = radio
//!               D-pad up = drone   D-pad down = sound on/off   View/Select = PANIC
//!   Menus:      A = OK / Enter   B = back / Esc   D-pad = up/down
//!   Typing:     the Steam keyboard (Steam + X on a Steam Deck / Legion Go), A = send, B = cancel
//!
//! Touch (Android phones and tablets; HOLLOW_TOUCH=1 tries it with the mouse on a computer):
//!   Playing:    drag on the left half = walk (a joystick appears where you touch), drag on the right half = look,
//!               buttons for use, hide, sprint, sneak, 2D/3D, panic and menu
//!   Elsewhere:  big buttons for every choice; the on-screen keyboard opens where you type

#[cfg(not(any(target_os = "android", target_os = "ios")))]
use gilrs::{Axis, Button, Gilrs};
use macroquad::prelude::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Title,
    Play,
    Typing,
    Mac,
    Consent,
    Menu,
}

#[derive(Clone, Copy)]
enum Role {
    Button(usize),
    Stick(Vec2),
    Look(Vec2),
}

#[derive(Default)]
struct State {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    gilrs: Option<Gilrs>,
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    tried: bool,
    name: Option<String>,
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    prev: HashSet<Button>,
    pressed_keys: HashSet<KeyCode>,
    held_keys: HashSet<KeyCode>,
    left: Vec2,
    right: Vec2,
    // ---- touch ----
    touch_on: bool,
    touches: HashMap<u64, (Role, Vec2)>,
    look: Vec2,
    keyboard: bool,
    online: bool,
}

thread_local! {
    static PAD: RefCell<State> = RefCell::new(State::default());
}

/// Touch controls are on: on Android, or when HOLLOW_TOUCH is set (to try them with a mouse).
pub fn touch_mode() -> bool {
    cfg!(any(target_os = "android", target_os = "ios")) || std::env::var("HOLLOW_TOUCH").is_ok()
}

// ---------------- Controllers ----------------

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn dead(v: f32) -> f32 {
    if v.abs() < 0.2 { 0.0 } else { (v - 0.2 * v.signum()) / 0.8 }
}

/// Which keys a button stands for on this kind of screen.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn keys_for(mode: Mode, b: Button) -> &'static [KeyCode] {
    use Button::{DPadDown, DPadLeft, DPadUp, East, LeftThumb, LeftTrigger, LeftTrigger2, North, RightTrigger, RightTrigger2, Select, South, Start, West};
    use KeyCode as K;
    match (mode, b) {
        (Mode::Title, South) | (Mode::Title, Start) => &[K::Key4],
        (Mode::Title, West) => &[K::Key2],
        (Mode::Title, North) => &[K::I],
        (Mode::Title, RightTrigger) => &[K::S],
        (Mode::Title, LeftTrigger) => &[K::L],
        (Mode::Play, South) => &[K::E],
        (Mode::Play, West) => &[K::H],
        (Mode::Play, North) => &[K::Tab],
        (Mode::Play, DPadDown) => &[K::M],
        (Mode::Play, DPadUp) => &[K::Q],
        (Mode::Play, Select) => &[K::P],
        (Mode::Play, Start) => &[K::Escape],
        (Mode::Play, LeftTrigger) | (Mode::Play, LeftTrigger2) => &[K::LeftShift],
        (Mode::Play, LeftThumb) => &[K::Space],
        (Mode::Play, RightTrigger) => &[K::V],
        (Mode::Play, RightTrigger2) => &[K::R],
        (Mode::Typing, South) => &[K::Enter],
        (Mode::Typing, East) => &[K::Escape],
        (Mode::Typing, DPadUp) => &[K::Up],
        (Mode::Typing, DPadDown) => &[K::Down],
        (Mode::Mac, South) => &[K::Enter],
        (Mode::Mac, East) => &[K::Escape],
        (Mode::Mac, North) => &[K::Tab],
        (Mode::Mac, DPadUp) => &[K::Up],
        (Mode::Mac, DPadDown) => &[K::Down],
        (Mode::Mac, DPadLeft) | (Mode::Mac, West) => &[K::Backspace],
        (Mode::Consent, South) => &[K::Y],
        (Mode::Consent, East) => &[K::N],
        (Mode::Menu, South) | (Mode::Menu, Start) => &[K::Enter, K::Space],
        (Mode::Menu, East) => &[K::Escape],
        (Mode::Menu, DPadUp) => &[K::Up],
        (Mode::Menu, DPadDown) => &[K::Down],
        _ => &[],
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn read_controller(p: &mut State, mode: Mode) {
    if !p.tried {
        p.tried = true;
        p.gilrs = Gilrs::new().ok();
    }
    let Some(gilrs) = p.gilrs.as_mut() else { return };
    while gilrs.next_event().is_some() {}
    // The first connected controller.
    let found = gilrs.gamepads().find(|(_, g)| g.is_connected()).map(|(_, g)| {
        let buttons: HashSet<Button> = [
            Button::South, Button::East, Button::West, Button::North, Button::Start, Button::Select,
            Button::LeftTrigger, Button::LeftTrigger2, Button::RightTrigger, Button::RightTrigger2, Button::LeftThumb,
            Button::DPadUp, Button::DPadDown, Button::DPadLeft, Button::DPadRight,
        ].into_iter().filter(|b| g.is_pressed(*b)).collect();
        let left = vec2(dead(g.value(Axis::LeftStickX)), dead(g.value(Axis::LeftStickY)));
        let right = vec2(dead(g.value(Axis::RightStickX)), dead(g.value(Axis::RightStickY)));
        (g.name().to_string(), buttons, left, right)
    });
    let (name, buttons, left, right) = match found {
        Some((n, b, l, r)) => (Some(n), b, l, r),
        None => (None, HashSet::new(), Vec2::ZERO, Vec2::ZERO),
    };
    if std::env::var("HOLLOW_PAD_DEBUG").is_ok() && (buttons != p.prev || name != p.name) {
        eprintln!("controller: {name:?} buttons {buttons:?} left {left} right {right}");
    }
    let newly: Vec<Button> = buttons.difference(&p.prev).copied().collect();
    p.pressed_keys.extend(newly.iter().flat_map(|b| keys_for(mode, *b).iter().copied()));
    p.held_keys.extend(buttons.iter().flat_map(|b| keys_for(mode, *b).iter().copied()));
    p.prev = buttons;
    p.name = name;
    p.left = left;
    p.right = right;
}

// ---------------- Touch ----------------

/// An on-screen button: where it is (canvas coordinates), its label, the key it stands for, and whether it counts
/// while held (sprint, sneak, voice) or only when tapped.
struct Btn {
    rect: Rect,
    label: &'static str,
    key: KeyCode,
    hold: bool,
}

fn buttons(mode: Mode, online: bool) -> Vec<Btn> {
    use KeyCode as K;
    let (l, r) = (-crate::margin(), crate::VW + crate::margin());
    let wh = crate::MH as f32 * crate::T;
    let b = |x: f32, y: f32, w: f32, h: f32, label, key, hold| Btn { rect: Rect::new(x, y, w, h), label, key, hold };
    match mode {
        Mode::Play => {
            let mut v = vec![
                b(r - 130.0, wh - 130.0, 110.0, 110.0, "USE", K::E, false),
                b(r - 250.0, wh - 90.0, 100.0, 70.0, "HIDE", K::H, false),
                b(r - 130.0, wh - 230.0, 110.0, 80.0, "SPRINT", K::Space, true),
                b(r - 250.0, wh - 190.0, 100.0, 80.0, "SNEAK", K::LeftShift, true),
                b(r - 110.0, 54.0, 90.0, 54.0, "2D/3D", K::Tab, false),
                b(r - 210.0, 54.0, 90.0, 54.0, "PANIC", K::P, false),
                b(l + 20.0, 54.0, 90.0, 54.0, "MENU", K::Escape, false),
                b(r - 310.0, 54.0, 90.0, 54.0, "DRONE", K::Q, false),
            ];
            if online {
                v.push(b(r - 370.0, wh - 90.0, 100.0, 70.0, "TALK", K::V, true));
                v.push(b(r - 370.0, wh - 180.0, 100.0, 70.0, "RADIO", K::R, true));
            }
            v
        }
        Mode::Title => {
            let x = r - 300.0;
            [("INFECTION", K::Key4), ("THE STORY", K::I), ("NORMAL", K::Key2), ("BRAVE", K::Key1), ("ONLINE", K::S), ("LOG IN / OUT", K::L)]
                .into_iter().enumerate().map(|(i, (label, key))| b(x, 110.0 + i as f32 * 72.0, 270.0, 60.0, label, key, false)).collect()
        }
        Mode::Typing => vec![
            b(r - 200.0, 20.0, 180.0, 60.0, "OK / SEND", K::Enter, false),
            b(r - 390.0, 20.0, 170.0, 60.0, "CANCEL", K::Escape, false),
            b(r - 560.0, 20.0, 150.0, 60.0, "NEXT", K::Tab, false),
            b(r - 680.0, 20.0, 100.0, 60.0, "UP", K::Up, false),
            b(r - 800.0, 20.0, 100.0, 60.0, "DOWN", K::Down, false),
        ],
        Mode::Mac => vec![
            b(r - 170.0, crate::VH - 80.0, 150.0, 60.0, "LEAVE", K::Escape, false),
            b(r - 340.0, crate::VH - 80.0, 150.0, 60.0, "SWITCH APP", K::Tab, false),
            b(r - 470.0, crate::VH - 80.0, 110.0, 60.0, "OPEN", K::Enter, false),
            b(r - 590.0, crate::VH - 80.0, 100.0, 60.0, "BACK", K::Backspace, false),
            b(r - 700.0, crate::VH - 80.0, 90.0, 60.0, "UP", K::Up, false),
            b(r - 810.0, crate::VH - 80.0, 90.0, 60.0, "DOWN", K::Down, false),
        ],
        Mode::Consent => vec![
            b(crate::VW / 2.0 - 260.0, crate::VH - 120.0, 240.0, 80.0, "ALLOW", K::Y, false),
            b(crate::VW / 2.0 + 20.0, crate::VH - 120.0, 240.0, 80.0, "KEEP IT INSIDE", K::N, false),
        ],
        Mode::Menu => vec![
            b(r - 200.0, crate::VH - 90.0, 180.0, 70.0, "OK", K::Enter, false),
            b(l + 20.0, crate::VH - 90.0, 180.0, 70.0, "BACK", K::Escape, false),
            b(r - 320.0, crate::VH - 90.0, 100.0, 70.0, "UP", K::Up, false),
            b(r - 440.0, crate::VH - 90.0, 100.0, 70.0, "DOWN", K::Down, false),
        ],
    }
}

/// A touch (or the mouse, when trying touch on a computer) in canvas coordinates.
fn to_canvas(p: Vec2) -> Vec2 {
    let (s, ox, oy) = crate::canvas_fit();
    vec2((p.x - ox) / s - crate::margin(), (p.y - oy) / s)
}

fn read_touch(p: &mut State, mode: Mode) {
    p.look = Vec2::ZERO;
    let dpi = macroquad::miniquad::window::dpi_scale();
    let mut list: Vec<(u64, TouchPhase, Vec2)> = touches().into_iter().map(|t| (t.id, t.phase, to_canvas(t.position / dpi))).collect();
    if !cfg!(any(target_os = "android", target_os = "ios")) {
        // Trying it on a computer: the mouse is one finger.
        let m = to_canvas(Vec2::from(mouse_position()));
        if is_mouse_button_pressed(MouseButton::Left) { list.push((0, TouchPhase::Started, m)); }
        else if is_mouse_button_down(MouseButton::Left) { list.push((0, TouchPhase::Moved, m)); }
        else if is_mouse_button_released(MouseButton::Left) { list.push((0, TouchPhase::Ended, m)); }
    }
    let btns = buttons(mode, p.online);
    let wh = crate::MH as f32 * crate::T;
    for (id, phase, pos) in list {
        match phase {
            TouchPhase::Started => {
                let role = if let Some(i) = btns.iter().position(|b| b.rect.contains(pos)) {
                    p.pressed_keys.insert(btns[i].key);
                    Role::Button(i)
                } else if mode == Mode::Play && pos.y < wh && pos.x < crate::VW / 2.0 {
                    Role::Stick(pos)
                } else if mode == Mode::Play && pos.y < wh {
                    Role::Look(pos)
                } else {
                    continue;
                };
                p.touches.insert(id, (role, pos));
            }
            TouchPhase::Moved | TouchPhase::Stationary => {
                if let Some((role, last)) = p.touches.get_mut(&id) {
                    if let Role::Look(prev) = role {
                        p.look += pos - *prev;
                        *prev = pos;
                    }
                    *last = pos;
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => { p.touches.remove(&id); }
        }
    }
    // Held buttons and the joystick.
    let mut held = vec![];
    let mut stick = None;
    for (role, pos) in p.touches.values() {
        match role {
            Role::Button(i) => if let Some(b) = btns.get(*i) { if b.hold { held.push(b.key); } },
            Role::Stick(origin) => {
                let v = (*pos - *origin) / 70.0;
                let v = if v.length() > 1.0 { v.normalize() } else { v };
                stick = Some(vec2(v.x, -v.y));
            }
            Role::Look(_) => {}
        }
    }
    p.held_keys.extend(held);
    if let Some(s) = stick { p.left = s; }
    // The on-screen keyboard: open while there's something to type.
    let want_keyboard = matches!(mode, Mode::Typing | Mode::Mac);
    if want_keyboard != p.keyboard {
        p.keyboard = want_keyboard;
        macroquad::miniquad::window::show_keyboard(want_keyboard);
    }
}

/// Reads the controller and the touch screen once per frame (call before anything checks keys).
pub fn update(mode: Mode, online: bool) {
    PAD.with(|p| {
        let mut p = p.borrow_mut();
        p.pressed_keys.clear();
        p.held_keys.clear();
        p.left = Vec2::ZERO;
        p.right = Vec2::ZERO;
        p.online = online;
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        read_controller(&mut p, mode);
        if touch_mode() {
            if !p.touch_on {
                p.touch_on = true;
                // Touches shouldn't also act as mouse clicks (the mouse grabs the camera in 3D).
                simulate_mouse_with_touch(false);
            }
            read_touch(&mut p, mode);
        }
    });
}

/// Draws the touch buttons and the joystick over the screen (canvas coordinates).
pub fn draw(mode: Mode) {
    if !touch_mode() { return; }
    PAD.with(|p| {
        let p = p.borrow();
        let btns = buttons(mode, p.online);
        let held: Vec<usize> = p.touches.values().filter_map(|(r, _)| if let Role::Button(i) = r { Some(*i) } else { None }).collect();
        for (i, b) in btns.iter().enumerate() {
            let on = held.contains(&i);
            draw_rectangle(b.rect.x, b.rect.y, b.rect.w, b.rect.h, Color::new(1.0, 1.0, 1.0, if on { 0.32 } else { 0.12 }));
            draw_rectangle_lines(b.rect.x, b.rect.y, b.rect.w, b.rect.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.45));
            let size = if b.rect.h >= 80.0 { 22.0 } else { 18.0 };
            let w = measure_text(b.label, None, size as u16, 1.0).width;
            draw_text(b.label, b.rect.x + (b.rect.w - w) / 2.0, b.rect.y + b.rect.h / 2.0 + size / 3.0, size, WHITE);
        }
        for (role, pos) in p.touches.values() {
            if let Role::Stick(o) = role {
                draw_circle_lines(o.x, o.y, 70.0, 3.0, Color::new(1.0, 1.0, 1.0, 0.4));
                let d = (*pos - *o).clamp_length_max(70.0);
                draw_circle(o.x + d.x, o.y + d.y, 26.0, Color::new(1.0, 1.0, 1.0, 0.35));
            }
        }
    });
}

/// This key was "pressed" by a controller button or a touch button this frame.
pub fn pressed(k: KeyCode) -> bool { PAD.with(|p| p.borrow().pressed_keys.contains(&k)) }
/// A controller or touch button standing for this key is held.
pub fn down(k: KeyCode) -> bool { PAD.with(|p| p.borrow().held_keys.contains(&k)) }
/// The sticks: (left = walk, right = look). Up is +y. The touch joystick is the left stick.
pub fn sticks() -> (Vec2, Vec2) { PAD.with(|p| { let p = p.borrow(); (p.left, p.right) }) }
/// How far a finger dragged to look around this frame (canvas pixels).
pub fn look_drag() -> Vec2 { PAD.with(|p| p.borrow().look) }
/// The connected controller's name, if there is one.
pub fn name() -> Option<String> { PAD.with(|p| p.borrow().name.clone()) }
