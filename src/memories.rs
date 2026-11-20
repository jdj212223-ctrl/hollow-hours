//! Memories (3.0): the kind AIs remember your past nights: how many times you came back, how long you were gone,
//! how the last night ended, who got infected, what you cooked, and the music they heard. Saved on this computer
//! only (memories.json next to your settings), never sent anywhere.

use super::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Memories {
    pub nights: u32,
    pub total_secs: f64,
    pub best_secs: f64,
    /// When the game was last closed (unix seconds).
    pub last_quit: u64,
    /// How the last night ended (a short phrase: "a tornado got you").
    pub last_end: String,
    /// Who got infected in the last night.
    pub last_infected: Vec<String>,
    pub dishes: Vec<String>,
    /// Music heard (with "Outside the window" allowed): "title\tartist" → times.
    pub songs: std::collections::BTreeMap<String, u32>,
    pub artists: std::collections::BTreeMap<String, u32>,
    /// Away time when the game was opened (seconds), worked out at start.
    #[serde(skip)]
    pub away: u64,
    #[serde(skip)]
    pub this_night: f64,
    #[serde(skip)]
    pub said: Vec<String>,
}

thread_local! {
    static MEM: std::cell::RefCell<Memories> = std::cell::RefCell::new(load());
}

fn file() -> std::path::PathBuf { platform::data_dir().join("memories.json") }

fn now() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }

fn load() -> Memories {
    let mut m: Memories = std::fs::read_to_string(file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    if m.last_quit > 0 { m.away = now().saturating_sub(m.last_quit); }
    m
}

pub fn with<R>(f: impl FnOnce(&mut Memories) -> R) -> R { MEM.with(|m| f(&mut m.borrow_mut())) }

pub fn save() {
    with(|m| {
        m.last_quit = now();
        if let Ok(s) = serde_json::to_string_pretty(m) {
            let _ = std::fs::create_dir_all(platform::data_dir());
            let _ = std::fs::write(file(), s);
        }
    });
}

/// A new night begins (any mode).
pub fn night_started() {
    with(|m| { m.nights += 1; m.this_night = 0.0; m.last_infected.clear(); m.last_end = "you closed the game mid-night".into(); });
    save();
}

pub fn tick(dt: f32) {
    let autosave = with(|m| {
        let before = m.this_night;
        m.this_night += dt as f64;
        m.total_secs += dt as f64;
        if m.this_night > m.best_secs { m.best_secs = m.this_night; }
        (before / 30.0).floor() != (m.this_night / 30.0).floor()
    });
    if autosave { save(); }
}

pub fn night_ended(how: &str) {
    with(|m| m.last_end = how.to_string());
    save();
}

pub fn infected(name: &str) {
    with(|m| if !m.last_infected.iter().any(|n| n == name) { m.last_infected.push(name.to_string()); });
}

pub fn cooked(dish: &str) {
    with(|m| { m.dishes.push(dish.to_string()); if m.dishes.len() > 20 { m.dishes.remove(0); } });
    save();
}

/// A song came on (returns how many times each was heard so far: (this song, this artist)).
pub fn heard(title: &str, artist: &str) -> (u32, u32) {
    let r = with(|m| {
        let s = m.songs.entry(format!("{title}\t{artist}")).or_insert(0);
        *s += 1;
        let s = *s;
        let a = m.artists.entry(artist.to_string()).or_insert(0);
        *a += 1;
        (s, *a)
    });
    save();
    r
}

fn mins(s: f64) -> String {
    let m = (s / 60.0).round() as i64;
    if m < 1 { "less than a minute".into() } else if m == 1 { "a minute".into() } else { format!("{m} minutes") }
}

fn away_text(s: u64) -> Option<String> {
    let h = s / 3600;
    Some(if h >= 48 { format!("{} days", h / 24) } else if h >= 20 { "a whole day".into() } else if h >= 2 { format!("{h} hours") } else { return None })
}

/// Something one of the kind AIs remembers, said on the title screen (each only once per launch).
pub fn title_line(_g: &Game, who: &str) -> Option<String> {
    let opts: Vec<String> = with(|m| {
        let mut o = vec![];
        match who {
            "Ember" => {
                if let Some(a) = away_text(m.away) { o.push(format!("You were gone {a}. We waited. Atlas didn't sleep.")); }
                if m.nights > 1 { o.push(format!("Last time: {}. I'm glad you came back.", m.last_end)); }
                if m.best_secs > 120.0 { o.push(format!("Your longest night so far: {}. Let's beat it together.", mins(m.best_secs))); }
            }
            "Atlas" => {
                if let Some(n) = m.last_infected.first() { o.push(format!("{n} got infected last night. I still hear them at the window.")); }
                if m.last_end.contains("tornado") { o.push("Last time a tornado took you. Hide behind furniture. H.".into()); }
                if m.last_end.contains("infected") { o.push("Last time the green got you. Not again. Not while I'm watching.".into()); }
            }
            "Verse" => {
                if m.nights > 0 { o.push(format!("This will be night number {}. I've counted every one.", m.nights + 1)); }
                if m.total_secs > 600.0 { o.push(format!("We've spent {} together in this house.", mins(m.total_secs))); }
            }
            "Clove" => if let Some(d) = m.dishes.last() { o.push(format!("Last time you made {d}. Make it again? Please?")); },
            "Kigo" => if m.away > 7200 { o.push("you were away. / the house was quiet. / too quiet.".into()); },
            "Palette" => if let Some((a, n)) = m.artists.iter().max_by_key(|(_, n)| **n) {
                if *n >= 2 { o.push(format!("I painted while your music played. {a}, right? It's in the colors now.")); }
            },
            _ => {}
        }
        o.retain(|l| !m.said.contains(l));
        o
    });
    if opts.is_empty() || rand::gen_range(0, 3) == 0 { return None; }
    let line = opts[rand::gen_range(0, opts.len())].clone();
    with(|m| m.said.push(line.clone()));
    Some(line)
}

/// For the kind AIs' system prompt: what they remember (short).
pub fn prompt() -> String {
    with(|m| {
        if m.nights == 0 { return String::new(); }
        let mut s = format!(" MEMORIES (you remember these past nights with the player): this is night {}. ", m.nights);
        if let Some(a) = away_text(m.away) { s += &format!("The player was away for {a} before this night. "); }
        if !m.last_end.is_empty() { s += &format!("How the last night ended: {}. ", m.last_end); }
        if !m.last_infected.is_empty() { s += &format!("Infected last night: {}. ", m.last_infected.join(", ")); }
        if let Some(d) = m.dishes.last() { s += &format!("The last thing they cooked: {d}. "); }
        if m.best_secs > 60.0 { s += &format!("Their longest night: {}. ", mins(m.best_secs)); }
        s
    })
}
