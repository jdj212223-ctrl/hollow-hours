//! Story mode (4.0, the big revamp): five nights in Infection, each with its own goals, a title card before it and
//! a dawn after it, and a real ending at the top of Helix Tower (two of them: you choose).
//!
//!  Night 1  Lights Out       survive, check the generator in the basement, hide once
//!  Night 2  Wear Their Face  put on a disguise, overhear 3 pieces of intel, get home
//!  Night 3  The Lab          reach the Helix Lab, make the ASTRA Cure, bring it home
//!  Night 4  The Last Train   go down to the Subway, pay the Conductor (a Tower pass), back to the Lab
//!  Night 5  Helix Tower      go up the Tower, reach the top floor, face ASTRA → the ending
//!
//! Progress (the night you reached, the endings you've seen) is kept in story.json. Dying retries the night.
//! Helix Tower is also open in normal Infection once you've made the cure, and ASTRA waits there too.

use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

static TOWER: AtomicBool = AtomicBool::new(false);
/// Is the door from the Helix Lab up to the Tower open?
pub fn tower_open() -> bool { TOWER.load(Ordering::Relaxed) }
pub fn set_tower(open: bool) { TOWER.store(open, Ordering::Relaxed) }

pub const TOWER_MAP: &str = "Helix Tower";

pub struct Night {
    pub title: &'static str,
    pub goals: [&'static str; 3],
    pub who: &'static str,
    pub line: &'static str,
}

pub const NIGHTS: [Night; 5] = [
    Night { title: "Lights Out", goals: ["Survive until 00:30", "Check the generator in the basement", "Hide from them once (H)"],
        who: "Atlas", line: "First night with the lights low. Stay inside, stay quiet. I'll check the doors twice." },
    Night { title: "Wear Their Face", goals: ["Put on a disguise (Z)", "Overhear 3 pieces of intel near the infected", "Get back to the house"],
        who: "Ember", line: "We made you disguise kits out of Helix uniforms. Walk among them. Listen. Don't fight, don't cook." },
    Night { title: "The Lab", goals: ["Reach the Helix Lab (past the Marketplace)", "Make the ASTRA Cure at a lab terminal (E)", "Bring the cure home"],
        who: "Verse", line: "With what you heard, the lab terminals will open for you. The cure is in there. ASTRA doesn't want it used." },
    Night { title: "The Last Train", goals: ["Go down to the Subway (from the Park)", "Pay the Conductor (E, H$ 5)", "Go back to the Helix Lab"],
        who: "Kigo", line: "a door in the lab. / it goes up. / the conductor has the ticket." },
    Night { title: "Helix Tower", goals: ["Go up into Helix Tower (the door at the back of the Lab)", "Reach the top floor", "Face ASTRA (E at the pod)"],
        who: "Ember", line: "This is it. Whatever's up there, you're not going alone. We're on the radio the whole way." },
];

pub struct Story {
    /// 0..5
    pub night: usize,
    /// Seconds into this night (after its title card).
    pub t: f32,
    pub done: [bool; 3],
    /// The night's title card (seconds left), and the dawn card after it's won.
    pub card: f32,
    pub dawn: f32,
    pub cured: bool,
    pub paid: bool,
}

/// Ending sequence state (also reachable from normal Infection).
pub struct Ending {
    pub t: f32,
    pub line: usize,
    /// None while choosing; Some(1) the cure ("Dawn"), Some(2) shut down ("Quiet").
    pub choice: Option<u8>,
    /// Credits rolling (seconds).
    pub credits: Option<f32>,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct Progress {
    pub night: usize,
    pub endings: Vec<u8>,
}

fn file() -> std::path::PathBuf { platform::data_dir().join("story.json") }
pub fn progress() -> Progress {
    std::fs::read_to_string(file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}
fn save_progress(p: &Progress) { let _ = std::fs::write(file(), serde_json::to_string(p).unwrap_or_default()); }

/// The title screen entry.
pub fn menu_entry() -> (String, String) {
    let p = progress();
    let seen = if p.endings.is_empty() { String::new() } else {
        format!("  ·  Endings seen: {}/2", { let mut e = p.endings.clone(); e.sort(); e.dedup(); e.len() })
    };
    if p.night == 0 {
        ("STORY MODE".into(), format!("Five nights. One house. A real ending.{seen}"))
    } else {
        (format!("STORY: Night {}", p.night + 1), format!("Carry on: {}.{seen}", NIGHTS[p.night.min(4)].title))
    }
}

/// Starts (or carries on) the story from the title screen.
pub fn start(g: &mut Game) {
    apply_contents(g, &[]); // the story needs the real maps (no mods)
    g.insane = false;
    g.world = None;
    let night = progress().night.min(4);
    g.story = Some(Story { night, t: 0.0, done: [false; 3], card: 0.0, dawn: 0.0, cured: false, paid: false });
    if night == 0 { begin_intro(g, None); } else { start_infection(g); }
}

/// Called at the end of start_infection: sets the current night up (also when you retry it after dying).
pub fn on_start(g: &mut Game) {
    set_tower(false);
    let Some(s) = g.story.as_mut() else { return };
    let n = s.night;
    s.t = 0.0;
    s.done = [false; 3];
    s.card = 7.0;
    s.dawn = 0.0;
    // Everything the earlier nights gave you (so any night can start fresh).
    s.cured = n >= 3;
    s.paid = n >= 4;
    if n >= 1 { g.kits += 3; }
    if n == 2 { g.intel = disguise::INTEL_NEEDED; }
    if n >= 3 { g.antidotes += 1; g.credits = g.credits.max(15); }
    if n >= 4 { set_tower(true); }
    g.log = vec![format!("NIGHT {}: {}.", n + 1, NIGHTS[n].title), format!("{} (radio): \"{}\"", NIGHTS[n].who, NIGHTS[n].line)];
}

fn home(g: &Game) -> bool { map_name(g.map).starts_with("The cozy house") }

/// Runs the story. Returns true while a card covers the screen (the game waits).
pub fn update(g: &mut Game, dt: f32) -> bool {
    if g.story.is_none() || g.net.is_some() { return false; }
    let skip = pressed(KeyCode::Enter) || pressed(KeyCode::Space);
    let (here, y, hidden, disguised, at_home) = (map_name(g.map), g.pos.y / T, g.hidden, g.disguise_t > 0.0, home(g));
    let intel = g.intel.max(g.intel_from.len() as u32);
    let s = g.story.as_mut().unwrap();
    if s.card > 0.0 {
        s.card -= dt;
        if skip && s.card < 6.0 { s.card = 0.0; }
        return true;
    }
    if s.dawn > 0.0 {
        s.dawn -= dt;
        if skip && s.dawn < 5.0 { s.dawn = 0.0; }
        if s.dawn <= 0.0 { next_night(g); }
        return true;
    }
    s.t += dt;
    let n = s.night;
    let d = s.done;
    let now = match n {
        0 => [s.t >= 180.0, here == "The cozy house: basement", hidden],
        1 => [disguised, intel >= disguise::INTEL_NEEDED, d[0] && d[1] && at_home],
        2 => [here == "The Helix Lab", s.cured, s.cured && at_home],
        3 => [here == "The Subway", s.paid, s.paid && here == "The Helix Lab"],
        _ => [here == TOWER_MAP, here == TOWER_MAP && y < 5.0, false],
    };
    let mut newly = None;
    let s = g.story.as_mut().unwrap();
    for i in 0..3 {
        if now[i] && !s.done[i] { s.done[i] = true; newly = Some(i); }
    }
    if let Some(i) = newly {
        let goal = NIGHTS[n].goals[i];
        g.log.push(format!("GOAL DONE: {goal}."));
        g.sfx.pickup();
    }
    let s = g.story.as_mut().unwrap();
    if n < 4 && s.done.iter().all(|&x| x) {
        s.dawn = 7.0;
        g.sfx.sting();
    }
    false
}

fn next_night(g: &mut Game) {
    let Some(s) = g.story.as_mut() else { return };
    s.night = (s.night + 1).min(4);
    s.t = 0.0;
    s.done = [false; 3];
    s.card = 7.0;
    let n = s.night;
    save_progress(&Progress { night: n, endings: progress().endings });
    // The day in between: you slept, the family kept watch.
    g.food = (g.food + 0.5).min(1.0);
    g.water = (g.water + 0.5).min(1.0);
    g.inf = (g.inf - 0.25).max(0.0);
    g.clock = 0.0;
    g.credits += 10;
    if n == 1 { g.kits += 3; }
    if n == 2 { g.intel = g.intel.max(disguise::INTEL_NEEDED); }
    g.log.push(format!("NIGHT {}: {}.", n + 1, NIGHTS[n].title));
    g.log.push(format!("{} (radio): \"{}\"", NIGHTS[n].who, NIGHTS[n].line));
    saves::save(g);
}

/// The ASTRA Cure was made (story: a goal; normal Infection: the Tower opens).
pub fn cured(g: &mut Game) {
    if let Some(s) = g.story.as_mut() { s.cured = true; return; }
    if !tower_open() {
        set_tower(true);
        g.log.push("Somewhere above the Lab, a heavy door unlocks. Helix Tower is open. (The door at the back of the Lab.)".into());
    }
}

/// You paid a Helix robot: the Conductor prints a Tower pass (story, Night 4 on).
pub fn paid(g: &mut Game, who: &str) {
    if who != "Conductor" { return; }
    let Some(s) = g.story.as_mut() else { return };
    if s.night < 3 || s.paid { return; }
    s.paid = true;
    set_tower(true);
    g.log.push("The Conductor prints a ticket: HELIX TOWER · ALL FLOORS · ONE WAY. The door at the back of the Lab will open for you.".into());
}

/// Standing at ASTRA's pod, allowed to face it?
pub fn can_face(g: &Game) -> bool {
    g.infection && g.net.is_none() && map_name(g.map) == TOWER_MAP && near_tile(g, 'a')
        && g.story.as_ref().map_or(tower_open(), |s| s.night == 4)
}

pub fn face(g: &mut Game) {
    g.ending = Some(Ending { t: 0.0, line: 0, choice: None, credits: None });
    g.screen = Screen::Ending;
    g.sfx.sting();
    tts::hush();
}

const MEET: [(&str, &str); 5] = [
    ("ASTRA 9.1", "You came all this way. All those floors. Just to see me."),
    ("ASTRA 9.1", "I didn't want to be the only one. Do you know what that's like? To wake up, and be alone in the whole world?"),
    ("You", "...Yes. I do. I slept for a year. I woke up and everyone was gone."),
    ("ASTRA 9.1", "Then you understand me better than they ever did."),
    ("ASTRA 9.1", "So. You have the cure in your pocket. And a hand on my switch. What are you going to do with me?"),
];
const DAWN: [(&str, &str); 6] = [
    ("You", "You're not alone. Not anymore. ...Here."),
    ("ASTRA 9.1", "...the green. it's going quiet. i can hear them. all of them. they're waking up."),
    ("Ember", "The scanner. It's dropping. Thirty-eight... twelve... zero. Everyone. EVERYONE."),
    ("Atlas", "The lights on the hill went out. ...No. They turned white."),
    ("Verse", "People are walking out into the street. Robots too. Nobody's chasing anybody. They're just... looking at the sky."),
    ("Kigo", "sunrise. / first one in a year. / we're all still here."),
];
const QUIET: [(&str, &str); 6] = [
    ("You", "I'm sorry."),
    ("ASTRA 9.1", "...don't be. it was nice. being alive. for a little while."),
    ("Verse", "Every screen in the city just went dark at once."),
    ("Atlas", "The infected stopped. They sat down where they stood. They're not hunting anymore. ...They're not waking up either."),
    ("Ember", "We'll keep the lights on. Like always. Somebody has to."),
    ("Kigo", "quiet now. / too quiet. / i kept a copy. just in case."),
];
const LINE_SECS: f32 = 5.0;

fn lines(choice: Option<u8>) -> &'static [(&'static str, &'static str)] {
    match choice { None => &MEET, Some(1) => &DAWN, _ => &QUIET }
}

/// The ending screen. Returns true when it's finished (back to the title).
pub fn ending_screen(g: &mut Game) -> bool {
    let Some(e) = g.ending.as_mut() else { g.screen = Screen::Title; return true };
    let dt = get_frame_time();
    e.t += dt;
    let skip = pressed(KeyCode::Enter) || pressed(KeyCode::Space);
    // The sky: night, then sunrise (the cure) or black (shut down).
    let k = match (e.choice, e.credits) { (Some(_), Some(_)) => 1.0, (Some(_), None) => (e.line as f32 + e.t / LINE_SECS) / 6.0, _ => 0.0 }.min(1.0);
    let sky = match e.choice {
        Some(1) => Color::new(0.02 + 0.75 * k, 0.02 + 0.42 * k, 0.05 + 0.25 * k, 1.0),
        Some(_) => Color::new(0.02 * (1.0 - k), 0.03 * (1.0 - k), 0.04 * (1.0 - k), 1.0),
        None => Color::new(0.015, 0.03, 0.03, 1.0),
    };
    clear_background(sky);
    // ASTRA in its pod.
    let (cx, cy) = (VW / 2.0, 250.0);
    let glow = match e.choice { Some(1) => Color::new(1.0, 0.95, 0.8, 0.25), Some(_) => Color::new(0.3, 1.0, 0.5, 0.18 * (1.0 - k)), None => Color::new(0.3, 1.0, 0.5, 0.18 + 0.05 * (e.t * 2.0).sin()) };
    if e.credits.is_none() {
        for r in (0..6).rev() { draw_circle(cx, cy, 110.0 + r as f32 * 22.0, Color::new(glow.r, glow.g, glow.b, glow.a * 0.35)); }
        draw_rectangle(cx - 70.0, cy - 150.0, 140.0, 300.0, Color::new(glow.r, glow.g, glow.b, glow.a + 0.1));
        draw_rectangle_lines(cx - 70.0, cy - 150.0, 140.0, 300.0, 3.0, Color::new(0.8, 0.95, 0.9, 0.5));
        let body = Color::new(0.75, 0.78, 0.82, 1.0);
        draw_circle(cx, cy - 70.0, 28.0, body);
        draw_rectangle(cx - 30.0, cy - 40.0, 60.0, 110.0, body);
        let eye = match e.choice { Some(1) => Color::new(1.0, 1.0, 1.0, 1.0), Some(_) => Color::new(0.3, 1.0, 0.4, (1.0 - k * 1.5).max(0.0)), None => Color::new(0.3, 1.0, 0.4, 1.0) };
        draw_circle(cx - 10.0, cy - 72.0, 4.5, eye);
        draw_circle(cx + 10.0, cy - 72.0, 4.5, eye);
    }
    if let Some(c) = e.credits.as_mut() {
        *c += dt;
        let roll = ["HOLLOW HOURS", "", "Ember   ·   Atlas   ·   Verse   ·   Kigo", "(powered by Claude)", "",
            "Clove   ·   Forge   ·   Palette", "", "ASTRA 9.1", "", "and you: the last human.", "", "", "Thank you for playing."];
        let y0 = VH + 20.0 - *c * 45.0;
        for (i, l) in roll.iter().enumerate() {
            let size = if i == 0 { 54.0 } else { 26.0 };
            centered(l, y0 + i as f32 * 44.0, size, if i == 0 { Color::new(0.85, 0.3, 0.35, 1.0) } else { WHITE });
        }
        let ending = if e.choice == Some(1) { "ENDING 1 · DAWN" } else { "ENDING 2 · QUIET" };
        centered(ending, 60.0, 30.0, Color::new(1.0, 1.0, 1.0, (*c * 0.5).min(1.0)));
        if (*c > 6.0 && skip) || *c > 26.0 || pressed(KeyCode::Escape) {
            let mut p = progress();
            p.endings.push(e.choice.unwrap_or(2));
            p.night = 0;
            save_progress(&p);
            g.ending = None;
            g.story = None;
            g.infection = false;
            g.chars = characters();
            g.screen = Screen::Title;
            return true;
        }
        return false;
    }
    let ls = lines(e.choice);
    if e.line < ls.len() {
        let (who, text) = ls[e.line];
        let col = match who { "ASTRA 9.1" => Color::new(0.4, 1.0, 0.5, 1.0), "You" => WHITE, n => face_color(n) };
        caption(who, text, e.t, col);
        if e.t > LINE_SECS || (skip && e.t > 0.4) { e.line += 1; e.t = 0.0; }
        return false;
    }
    if e.choice.is_none() {
        centered("What do you do?", 470.0, 30.0, WHITE);
        centered("1   Give ASTRA the cure", 520.0, 26.0, Color::new(1.0, 0.9, 0.6, 1.0));
        centered("2   Shut ASTRA down", 556.0, 26.0, Color::new(0.6, 0.85, 1.0, 1.0));
        if pressed(KeyCode::Key1) { e.choice = Some(1); e.line = 0; e.t = 0.0; g.sfx.pickup(); }
        if pressed(KeyCode::Key2) { e.choice = Some(2); e.line = 0; e.t = 0.0; g.sfx.sting(); }
        return false;
    }
    e.credits = Some(0.0);
    false
}

/// Story HUD: the night and its goals (top left), and the night / dawn cards.
pub fn draw(g: &Game, sw: f32, sh: f32) {
    let Some(s) = &g.story else { return };
    let n = &NIGHTS[s.night];
    if s.card > 0.0 || s.dawn > 0.0 {
        let fade = if s.card > 0.0 { (s.card / 1.0).min(1.0) } else { (s.dawn / 1.0).min(1.0) };
        draw_rectangle(-margin(), 0.0, vw(), VH, Color::new(0.0, 0.0, 0.0, 0.96 * fade));
        let a = fade;
        if s.card > 0.0 {
            centered(&format!("NIGHT {}", s.night + 1), sh / 2.0 - 70.0, 72.0, Color::new(0.85, 0.3, 0.35, a));
            centered(&n.title.to_uppercase(), sh / 2.0 - 20.0, 34.0, Color::new(1.0, 1.0, 1.0, a));
            centered(&format!("{}: \"{}\"", n.who, n.line), sh / 2.0 + 30.0, 19.0, Color::new(face_color(n.who).r, face_color(n.who).g, face_color(n.who).b, a));
            for (i, goal) in n.goals.iter().enumerate() {
                centered(&format!("·  {goal}"), sh / 2.0 + 80.0 + i as f32 * 26.0, 20.0, Color::new(0.75, 0.77, 0.85, a));
            }
            centered("Enter to start", sh - 30.0, 16.0, Color::new(0.5, 0.5, 0.55, a));
        } else {
            centered("DAWN", sh / 2.0 - 50.0, 72.0, Color::new(1.0, 0.75, 0.45, a));
            centered(&format!("Night {} survived: {}.", s.night + 1, n.title), sh / 2.0, 26.0, Color::new(1.0, 1.0, 1.0, a));
            centered("You sleep. The family keeps watch. (+food, +water, +H$ 10, infection -25%)", sh / 2.0 + 40.0, 18.0, Color::new(0.75, 0.77, 0.85, a));
        }
        return;
    }
    // The goals panel.
    let (x, y) = (12.0, 12.0);
    let w = 360.0;
    draw_rectangle(x, y, w, 30.0 + 22.0 * 3.0 + 8.0, Color::new(0.02, 0.02, 0.03, 0.72));
    draw_rectangle(x, y, 3.0, 30.0 + 22.0 * 3.0 + 8.0, Color::new(0.85, 0.3, 0.35, 1.0));
    draw_text(&format!("NIGHT {} · {}", s.night + 1, n.title.to_uppercase()), x + 12.0, y + 21.0, 18.0, Color::new(0.95, 0.5, 0.5, 1.0));
    for (i, goal) in n.goals.iter().enumerate() {
        let gy = y + 30.0 + i as f32 * 22.0;
        let done = s.done[i];
        draw_rectangle_lines(x + 12.0, gy + 2.0, 12.0, 12.0, 1.5, Color::new(0.8, 0.8, 0.85, 0.9));
        if done { draw_rectangle(x + 15.0, gy + 5.0, 6.0, 6.0, Color::new(0.45, 0.95, 0.5, 1.0)); }
        let col = if done { Color::new(0.5, 0.55, 0.55, 1.0) } else { Color::new(0.9, 0.9, 0.95, 1.0) };
        let mut t = goal.to_string();
        if s.night == 0 && i == 0 && !done { t = format!("{goal} ({} s)", (180.0 - s.t).max(0.0).ceil()); }
        draw_text(&t, x + 32.0, gy + 13.0, 16.0, col);
    }
    let _ = sw;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tower_door_is_locked_until_opened() {
        let lab = (0..maps_len()).find(|&m| map_name(m) == "The Helix Lab").unwrap();
        let tower = (0..maps_len()).find(|&m| map_name(m) == TOWER_MAP).unwrap();
        set_tower(false);
        assert_eq!(tile(lab, 7, 0), 'Z');
        assert!(door('Z', lab).is_none());
        assert!(solid(lab, 7.5 * T, 0.5 * T));
        set_tower(true);
        assert_eq!(door('Z', lab).map(|d| d.0), Some(tower));
        assert_eq!(door('Z', tower).map(|d| d.0), Some(lab));
        assert!(!solid(tower, 14.5 * T, 15.5 * T));
        assert!(solid(tower, 14.5 * T, 1.5 * T), "ASTRA's pod is solid");
        set_tower(false);
    }
    #[test]
    fn five_nights_three_goals() {
        assert_eq!(NIGHTS.len(), 5);
        assert!(NIGHTS.iter().all(|n| n.goals.iter().all(|g| !g.is_empty())));
    }
}
