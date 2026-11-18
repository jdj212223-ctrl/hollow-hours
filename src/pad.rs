//! Controller support (Xbox, PlayStation, Switch Pro, Steam Deck / Legion Go through Steam Input...).
//! Buttons become the game's keys, depending on the screen, so every screen works with a pad:
//!
//!   Title:      A = Infection   X = Normal   Y = the story   RB = server list   LB = log in
//!   Playing:    left stick = walk   right stick = look (3D)   A = talk / use   X = hide   Y = 2D/3D map
//!               LT or LB or L3 = sneak (hold)   RB = voice (hold)   RT = radio (hold)
//!               D-pad down = sound on/off   View/Select = PANIC
//!   Menus:      A = OK / Enter   B = back / Esc   D-pad = up/down
//!   Typing:     the Steam keyboard (Steam + X on a Steam Deck / Legion Go), A = send, B = cancel

use gilrs::{Axis, Button, Gilrs};
use macroquad::prelude::{vec2, KeyCode, Vec2};
use std::cell::RefCell;
use std::collections::HashSet;

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Title,
    Play,
    Typing,
    Mac,
    Consent,
    Menu,
}

#[derive(Default)]
struct State {
    gilrs: Option<Gilrs>,
    tried: bool,
    name: Option<String>,
    down_buttons: HashSet<Button>,
    prev: HashSet<Button>,
    pressed_keys: HashSet<KeyCode>,
    held_keys: HashSet<KeyCode>,
    left: Vec2,
    right: Vec2,
}

thread_local! {
    static PAD: RefCell<State> = RefCell::new(State::default());
}

fn dead(v: f32) -> f32 {
    if v.abs() < 0.2 { 0.0 } else { (v - 0.2 * v.signum()) / 0.8 }
}

/// Which keys a button stands for on this kind of screen.
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
        (Mode::Play, Select) => &[K::P],
        (Mode::Play, Start) => &[K::Escape],
        (Mode::Play, LeftTrigger) | (Mode::Play, LeftTrigger2) | (Mode::Play, LeftThumb) => &[K::LeftShift],
        (Mode::Play, RightTrigger) => &[K::V],
        (Mode::Play, RightTrigger2) => &[K::R],
        (Mode::Typing, South) => &[K::Enter],
        (Mode::Typing, East) => &[K::Escape],
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

/// Reads the controller once per frame (call before anything checks keys).
pub fn update(mode: Mode) {
    PAD.with(|p| {
        let mut p = p.borrow_mut();
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
        let newly: Vec<Button> = buttons.difference(&p.prev).copied().collect();
        p.pressed_keys = newly.iter().flat_map(|b| keys_for(mode, *b).iter().copied()).collect();
        p.held_keys = buttons.iter().flat_map(|b| keys_for(mode, *b).iter().copied()).collect();
        p.prev = buttons.clone();
        p.down_buttons = buttons;
        p.name = name;
        p.left = left;
        p.right = right;
    });
}

/// This key was "pressed" by a controller button this frame.
pub fn pressed(k: KeyCode) -> bool { PAD.with(|p| p.borrow().pressed_keys.contains(&k)) }
/// A controller button standing for this key is held.
pub fn down(k: KeyCode) -> bool { PAD.with(|p| p.borrow().held_keys.contains(&k)) }
/// The sticks: (left = walk, right = look). Up is +y.
pub fn sticks() -> (Vec2, Vec2) { PAD.with(|p| { let p = p.borrow(); (p.left, p.right) }) }
/// The connected controller's name, if there is one.
pub fn name() -> Option<String> { PAD.with(|p| p.borrow().name.clone()) }
