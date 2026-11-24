//! Settings (O on the title, or the pause menu): sound, voice chat, accessibility, flashing lights and key bindings.
//! Saved in settings.json in the game's data folder.
//!  - Voice chat is OFF until you turn your microphone on yourself.
//!  - Flashing lights are OFF; turning them on takes four separate warnings, and PANIC turns them straight off.

use super::*;
use std::cell::RefCell;

/// Actions you can rebind: (name, the key the game checks for).
pub const ACTIONS: [(&str, KeyCode); 18] = [
    ("Walk forward", KeyCode::W), ("Walk back", KeyCode::S), ("Walk left", KeyCode::A), ("Walk right", KeyCode::D),
    ("Sprint", KeyCode::Space), ("Sneak", KeyCode::LeftShift), ("Hide", KeyCode::H), ("Use / talk", KeyCode::E),
    ("Hit", KeyCode::F), ("Kick", KeyCode::G), ("Drone", KeyCode::Q), ("2D / 3D map", KeyCode::Tab),
    ("Voice chat (hold)", KeyCode::V), ("Radio (hold)", KeyCode::R), ("PANIC", KeyCode::P), ("Sound on/off", KeyCode::M),
    ("Disguise", KeyCode::Z), ("Antidote", KeyCode::C),
];

/// Keys you can bind (Escape always stays Escape).
const KEYS: [KeyCode; 73] = [
    KeyCode::A, KeyCode::B, KeyCode::C, KeyCode::D, KeyCode::E, KeyCode::F, KeyCode::G, KeyCode::H, KeyCode::I, KeyCode::J,
    KeyCode::K, KeyCode::L, KeyCode::M, KeyCode::N, KeyCode::O, KeyCode::P, KeyCode::Q, KeyCode::R, KeyCode::S, KeyCode::T,
    KeyCode::U, KeyCode::V, KeyCode::W, KeyCode::X, KeyCode::Y, KeyCode::Z,
    KeyCode::Key0, KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4, KeyCode::Key5, KeyCode::Key6, KeyCode::Key7, KeyCode::Key8, KeyCode::Key9,
    KeyCode::F1, KeyCode::F2, KeyCode::F3, KeyCode::F4, KeyCode::F5, KeyCode::F6, KeyCode::F7, KeyCode::F8, KeyCode::F9, KeyCode::F10, KeyCode::F12,
    KeyCode::Space, KeyCode::Tab, KeyCode::Enter, KeyCode::Backspace, KeyCode::CapsLock,
    KeyCode::LeftShift, KeyCode::RightShift, KeyCode::LeftControl, KeyCode::RightControl, KeyCode::LeftAlt, KeyCode::RightAlt,
    KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right,
    KeyCode::Comma, KeyCode::Period, KeyCode::Slash, KeyCode::Semicolon, KeyCode::Apostrophe, KeyCode::LeftBracket,
    KeyCode::RightBracket, KeyCode::Minus, KeyCode::Equal, KeyCode::GraveAccent, KeyCode::Backslash,
];

fn key_name(k: KeyCode) -> String {
    let s = format!("{k:?}");
    s.strip_prefix("Key").filter(|d| d.len() == 1).map(String::from).unwrap_or(s)
}
fn key_from(name: &str) -> Option<KeyCode> { KEYS.iter().copied().find(|k| format!("{k:?}") == name) }

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub master: f32,
    pub ambience: f32,
    pub effects: f32,
    pub voices: f32,
    /// Your microphone (voice chat and the radio): off until you turn it on.
    pub mic_on: bool,
    pub hear_voice: bool,
    /// The AIs read their lines aloud (the computer's own speech; the infected sound corrupted).
    pub ai_voices: bool,
    pub captions: bool,
    pub big_text: bool,
    pub reduce_motion: bool,
    pub high_contrast: bool,
    pub jumpscares: bool,
    pub toggle_sprint: bool,
    pub toggle_sneak: bool,
    pub game_speed: f32,
    pub mouse_sens: f32,
    pub invert_y: bool,
    pub flashing: bool,
    /// Which browser opens the Helix pages ("" = your default browser).
    pub browser: String,
    /// Official builds download new releases by themselves (installed with U on the title, or when you quit).
    pub auto_update: bool,
    /// Deep meta (4.0): the infected know your computer (username, Wi-Fi, apps...). Off by default.
    pub deep_meta: bool,
    /// Streamer-safe: Deep meta never says names (username, computer, Wi-Fi, apps).
    pub streamer_safe: bool,
    /// Rebound keys: action name -> key name.
    pub binds: std::collections::HashMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { master: 1.0, ambience: 0.8, effects: 1.0, voices: 1.0, mic_on: false, hear_voice: true, ai_voices: true, captions: false,
            big_text: false, reduce_motion: false, high_contrast: false, jumpscares: true, toggle_sprint: false,
            toggle_sneak: false, game_speed: 1.0, mouse_sens: 1.0, invert_y: false, flashing: false, browser: String::new(), auto_update: true, deep_meta: false, streamer_safe: false, binds: Default::default() }
    }
}

fn file() -> std::path::PathBuf { platform::data_dir().join("settings.json") }

thread_local! {
    static S: RefCell<Settings> = RefCell::new(
        std::fs::read_to_string(file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default());
    /// True while you're playing: only then do rebound keys apply (menus keep their letters).
    static PLAYING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn get() -> Settings { S.with(|s| s.borrow().clone()) }
pub fn with<R>(f: impl FnOnce(&Settings) -> R) -> R { S.with(|s| f(&s.borrow())) }
fn change(f: impl FnOnce(&mut Settings)) {
    S.with(|s| {
        let mut s = s.borrow_mut();
        f(&mut s);
        let _ = std::fs::write(file(), serde_json::to_string_pretty(&*s).unwrap_or_default());
    });
}

pub fn set_playing(on: bool) { PLAYING.with(|p| p.set(on)); }

/// The key really pressed for a game key (a rebound one, while playing).
pub fn bound(k: KeyCode) -> KeyCode {
    if !PLAYING.with(|p| p.get()) { return k; }
    S.with(|s| {
        let s = s.borrow();
        ACTIONS.iter().find(|(_, d)| *d == k).and_then(|(name, _)| s.binds.get(*name)).and_then(|n| key_from(n)).unwrap_or(k)
    })
}

/// Turns flashing lights off (PANIC does this).
pub fn flashing_off() { change(|s| s.flashing = false); }

// ---------------- The settings screen ----------------

const TABS: [&str; 5] = ["Sound", "Voice chat", "Accessibility", "Flashing & web", "Controls"];

/// What the settings screen is doing: tab, row, waiting for a key to bind, the flashing warnings, where to go back.
#[derive(Default)]
pub struct Ui {
    pub tab: usize,
    pub row: usize,
    pub binding: Option<usize>,
    pub warning: Option<usize>,
    pub back_to_pause: bool,
    fresh: bool,
}

pub fn open(g: &mut Game, from_pause: bool) {
    g.settings_ui = Ui { back_to_pause: from_pause, ..Default::default() };
    g.screen = Screen::Settings;
}

enum Row { Slider(&'static str, f32, fn(&mut Settings, f32)), Toggle(&'static str, bool, fn(&mut Settings, bool), &'static str), Text(String), Browser(String) }

fn rows(tab: usize, s: &Settings) -> Vec<Row> {
    use Row::*;
    match tab {
        0 => vec![
            Slider("Master volume", s.master, |s, v| s.master = v),
            Slider("Ambience (hum, wind, sizzle, drone)", s.ambience, |s, v| s.ambience = v),
            Slider("Effects (steps, growls, stings)", s.effects, |s, v| s.effects = v),
            Slider("Voices (other players and the AIs)", s.voices, |s, v| s.voices = v),
        ],
        1 => vec![
            Toggle("My microphone", s.mic_on, |s, v| s.mic_on = v, "Off: nobody can hear you, even if you press V or R. On: they hear you only while you HOLD V (nearby) or R (radio)."),
            Toggle("Hear other players", s.hear_voice, |s, v| s.hear_voice = v, "Turn off to play online without hearing anyone's voice."),
            Toggle("AI voices", s.ai_voices, |s, v| s.ai_voices = v, "The characters read their lines aloud with your computer's own speech (offline). The infected sound corrupted."),
            Text("Your voice only ever goes to the players in your room, while you hold the key. Nothing is recorded or stored.".into()),
        ],
        2 => vec![
            Toggle("Captions for sounds", s.captions, |s, v| s.captions = v, "Shows sounds as text, with where they come from: [growl, left], [door], [radio]."),
            Toggle("Large text", s.big_text, |s, v| s.big_text = v, "Bigger speech bubbles, captions and messages."),
            Toggle("Reduce motion", s.reduce_motion, |s, v| s.reduce_motion = v, "No head bob, no screen shake."),
            Toggle("High-contrast infected", s.high_contrast, |s, v| s.high_contrast = v, "A yellow ! marker over every infected, so you don't need to tell the colours apart."),
            Toggle("Jumpscares", s.jumpscares, |s, v| s.jumpscares = v, "Off: no face fills the screen when you're caught."),
            Toggle("Sprint: toggle instead of hold", s.toggle_sprint, |s, v| s.toggle_sprint = v, "Press once to start sprinting, again to stop."),
            Toggle("Sneak: toggle instead of hold", s.toggle_sneak, |s, v| s.toggle_sneak = v, "Press once to sneak, again to walk."),
            Slider("Game speed", s.game_speed, |s, v| s.game_speed = v),
            Slider("Mouse sensitivity", s.mouse_sens / 2.0, |s, v| s.mouse_sens = v * 2.0),
            Toggle("Invert mouse up/down", s.invert_y, |s, v| s.invert_y = v, ""),
        ],
        3 => vec![
            Toggle("Flashing lights", s.flashing, |s, v| s.flashing = v, "Strobing jumpscares and lightning on the beach. Off by default. PANIC turns it off at once."),
            Toggle("Auto-update", s.auto_update, |s, v| s.auto_update = v, "Official downloads look for a new version when the game starts and get it in the background. It's installed when you press U on the title screen or when you quit. Builds you made yourself never update."),
            Toggle("Deep meta (4.0)", s.deep_meta, |s, v| s.deep_meta = v, "Needs Outside the window. The infected learn about your computer and say it: your username, your computer's name, your Wi-Fi, your battery, the apps and AIs you have. They only read and say things, never change anything, and it never leaves your computer. Offline games only."),
            Text("Clean up Deep meta: removes everything it made (calendar, reminders, notes, Desktop files) and brings back any deleted file (Enter)".into()),
            Toggle("Streamer-safe", s.streamer_safe, |s, v| s.streamer_safe = v, "Deep meta never says names: no username, computer name, Wi-Fi or app names (for recording and streaming)."),
            Browser(if s.browser.is_empty() { "Your default browser".into() } else { s.browser.clone() }),
        ],
        _ => {
            let mut v: Vec<Row> = ACTIONS.iter().map(|(name, d)| {
                let k = s.binds.get(*name).and_then(|n| key_from(n)).unwrap_or(*d);
                Text(format!("{name:<20} {}", key_name(k)))
            }).collect();
            v.push(Text("Reset all keys to the defaults".into()));
            v
        }
    }
}

const WARNINGS: [&str; 4] = [
    "WARNING 1 of 4\n\nThis turns on FLASHING LIGHTS: strobing jumpscares and lightning flashes.",
    "WARNING 2 of 4\n\nFlashing lights can trigger seizures in people with photosensitive epilepsy, even in people who have never had one before.",
    "WARNING 3 of 4\n\nStop playing at once if you feel dizzy, see differently, or your eyes or face twitch. Press P (PANIC) at any time: flashing turns off immediately.",
    "WARNING 4 of 4\n\nLast check. Are you sure you want flashing lights on?",
];

/// Draws the settings screen and handles its keys.
pub fn screen(g: &mut Game) {
    let s = get();
    let big = s.big_text;
    let soft = Color::new(0.65, 0.67, 0.75, 1.0);
    // The four flashing warnings, one at a time.
    if let Some(w) = g.settings_ui.warning {
        clear_background(Color::new(0.08, 0.0, 0.0, 1.0));
        draw_rectangle(-margin(), 0.0, 3.0, VH, Color::new(1.0, 0.4, 0.35, 0.8));
        draw_text(&format!("WARNING {} OF 4", w + 1), ui::LEFT, 120.0, 18.0, Color::new(1.0, 0.4, 0.35, 0.9));
        let mut y = 200.0;
        for (i, para) in WARNINGS[w].split("\n\n").enumerate() {
            for l in wrap(para, VW - 160.0, if i == 0 { 34.0 } else { 26.0 }) {
                draw_text(&l, 80.0, y, if i == 0 { 34.0 } else { 26.0 }, if i == 0 { Color::new(1.0, 0.4, 0.35, 1.0) } else { WHITE });
                y += if i == 0 { 44.0 } else { 34.0 };
            }
            y += 20.0;
        }
        let _ = soft;
        if w == 3 { ui::hints(&[("Enter", "YES, turn flashing lights on"), ("Esc", "no, keep them off")]); }
        else { ui::hints(&[("Enter", "I understand, continue"), ("Esc", "keep flashing lights off")]); }
        if pressed(KeyCode::Escape) { g.settings_ui.warning = None; }
        else if pressed(KeyCode::Enter) {
            if w == 3 { change(|s| s.flashing = true); g.settings_ui.warning = None; } else { g.settings_ui.warning = Some(w + 1); }
        }
        return;
    }
    ui::backdrop();
    ui::header("Settings", "");
    if let Some(t) = ui::tabs(&TABS, g.settings_ui.tab, 124.0) { g.settings_ui.tab = t; g.settings_ui.row = 0; }
    let list = rows(g.settings_ui.tab, &s);
    let row_h = if big { 40.0 } else { 34.0 };
    let size = if big { 26.0 } else { 22.0 };
    let mut desc = String::new();
    // Controls have more rows: scroll so the chosen one stays on screen.
    let first = g.settings_ui.row.saturating_sub(8);
    for (i, r) in list.iter().enumerate().skip(first).take(9) {
        let y = 200.0 + (i - first) as f32 * row_h;
        let on = i == g.settings_ui.row;
        ui::row(ui::LEFT - 14.0, y - row_h + 10.0, VW - 2.0 * ui::LEFT + 28.0, row_h - 2.0, on);
        if ui::hit(ui::LEFT - 14.0, y - row_h + 10.0, VW - 2.0 * ui::LEFT + 28.0, row_h - 2.0).1 { g.settings_ui.row = i; }
        let col = if on { WHITE } else { Color::new(0.82, 0.84, 0.9, 1.0) };
        match r {
            Row::Slider(label, v, _) => {
                draw_text(label, 70.0, y, size, col);
                ui::slider(*v, 560.0, y, 240.0);
            }
            Row::Toggle(label, v, _, d) => {
                draw_text(label, 70.0, y, size, col);
                ui::switch(*v, 600.0, y);
                if on { desc = d.to_string(); }
            }
            Row::Browser(b) => {
                draw_text("Helix pages open in", 70.0, y, size, col);
                draw_text(&format!("< {b} >"), 470.0, y, size, Color::new(0.85, 0.47, 0.32, 1.0));
                if on {
                    let n = platform::browsers().len();
                    desc = format!("When the game opens a page (the Helix Dynamics site, only with Outside the window allowed): your default browser, or one of the {n} browsers found on this computer. Left/Right to choose.");
                }
            }
            Row::Text(t) => {
                let waiting = g.settings_ui.tab == 4 && g.settings_ui.binding == Some(i);
                draw_text(&if waiting { format!("{}   press the new key... (Esc cancels)", ACTIONS[i].0) } else { t.clone() }, 70.0, y, size, if waiting { YELLOW } else { col });
            }
        }
    }
    if !desc.is_empty() {
        let lines = wrap(&desc, VW - 2.0 * ui::LEFT - 40.0, 17.0);
        let h = 18.0 + lines.len() as f32 * 21.0;
        ui::panel(ui::LEFT, VH - 64.0 - h, VW - 2.0 * ui::LEFT, h);
        for (k, l) in lines.iter().enumerate() { draw_text(l, ui::LEFT + 18.0, VH - 64.0 - h + 22.0 + k as f32 * 21.0, 17.0, soft); }
    }
    if g.settings_ui.tab == 4 { ui::hints(&[("↑↓", "choose"), ("Enter", "rebind"), ("Tab", "next section"), ("Esc", "back")]); }
    else { ui::hints(&[("↑↓", "choose"), ("←→", "change"), ("Enter", "switch"), ("Tab", "next section"), ("Esc", "back")]); }

    // Waiting for the key to bind.
    if let Some(i) = g.settings_ui.binding {
        // Not the Enter that started it.
        if g.settings_ui.fresh { g.settings_ui.fresh = false; return; }
        if pressed(KeyCode::Escape) { g.settings_ui.binding = None; return; }
        if let Some(k) = get_last_key_pressed().filter(|k| KEYS.contains(k)) {
            let name = ACTIONS[i].0;
            change(|s| {
                let current = |s: &Settings, n: &str, d: KeyCode| s.binds.get(n).and_then(|x| key_from(x)).unwrap_or(d);
                let old = current(s, name, ACTIONS[i].1);
                // A key that another action already uses: the two swap.
                if let Some((other, od)) = ACTIONS.iter().find(|(n, d)| *n != name && current(s, n, *d) == k) {
                    s.binds.insert(other.to_string(), format!("{old:?}"));
                    let _ = od;
                }
                s.binds.insert(name.to_string(), format!("{k:?}"));
            });
            g.settings_ui.binding = None;
        }
        return;
    }
    let n = list.len();
    if pressed(KeyCode::Down) { g.settings_ui.row = (g.settings_ui.row + 1) % n; }
    if pressed(KeyCode::Up) { g.settings_ui.row = (g.settings_ui.row + n - 1) % n; }
    if pressed(KeyCode::Tab) { g.settings_ui.tab = (g.settings_ui.tab + 1) % TABS.len(); g.settings_ui.row = 0; }
    let row = g.settings_ui.row.min(n - 1);
    let (left, right, enter) = (pressed(KeyCode::Left), pressed(KeyCode::Right), pressed(KeyCode::Enter) || pressed(KeyCode::Space));
    match &list[row] {
        Row::Slider(label, v, set) => {
            let step = if label.starts_with("Game speed") { 0.25 } else { 0.1 };
            let lo = if label.starts_with("Game speed") { 0.5 } else if label.starts_with("Mouse") { 0.1 } else { 0.0 };
            let nv = if left { (v - step).max(lo) } else if right { (v + step).min(1.0) } else { *v };
            if nv != *v { let set = *set; change(|s| set(s, (nv * 100.0).round() / 100.0)); g.sfx.apply_volumes(); }
        }
        Row::Toggle(label, v, set, _) => {
            if enter || left || right {
                if *label == "Flashing lights" && !v {
                    g.settings_ui.warning = Some(0);
                } else {
                    let set = *set;
                    let nv = !v;
                    change(|s| set(s, nv));
                }
            }
        }
        Row::Browser(cur) => {
            if left || right || enter {
                // "" (default) first, then every browser installed here.
                let mut all = vec![String::new()];
                all.extend(platform::browsers());
                let cur = if cur == "Your default browser" { String::new() } else { cur.clone() };
                let i = all.iter().position(|b| *b == cur).unwrap_or(0);
                let j = if left { (i + all.len() - 1) % all.len() } else { (i + 1) % all.len() };
                let pick = all[j].clone();
                change(|s| s.browser = pick);
            }
        }
        Row::Text(t) if g.settings_ui.tab == 3 && enter && t.starts_with("Clean up") => {
            let r = crate::spread::clean_up(g);
            g.status = r.clone();
            g.log.push(r);
        }
        Row::Text(_) if g.settings_ui.tab == 4 && enter => {
            if row == ACTIONS.len() { change(|s| s.binds.clear()); } else { g.settings_ui.binding = Some(row); g.settings_ui.fresh = true; }
        }
        _ => {}
    }
    if pressed(KeyCode::Escape) {
        g.screen = if g.settings_ui.back_to_pause { Screen::Paused } else { Screen::Title };
    }
}
