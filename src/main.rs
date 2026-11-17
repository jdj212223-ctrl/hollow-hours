//! Hollow Hours: a small horror game where the characters are real AIs.
//! Each character talks through a real model (offline through Ollama, or a cloud API if you set its key), has its
//! own personality, and can slip into a "crazy mode" at night (like The Mita's Crazy Mita), unless you joined Brave.
//! Meters: Fear, Discomfort, Liminality. Every phobia can be switched OFF. P = panic: everything stops at once.
//! Characters are our own, only *inspired by* real AIs: no logos, no real names on screen.

mod ai;
mod haptics;
mod meta;

use macroquad::prelude::*;

const T: f32 = 32.0; // tile size
const MW: usize = 30;
const MH: usize = 18;
const HUD: f32 = 84.0;

// ---------------- Maps ----------------

struct Map {
    name: &'static str,
    rows: [&'static str; MH],
    /// 0 = bright, 1 = pitch black (when the Dark phobia is on)
    dark: f32,
    /// how "empty and wrong" it feels
    liminal: f32,
    floor: Color,
}

const MAPS: [Map; 3] = [
    Map {
        name: "The cozy house",
        rows: [
            "##############################",
            "#............#...............#",
            "#..bb........#.......d.......#",
            "#..bb........#...............#",
            "#............#.......bbbb....#",
            "#.....L......................#",
            "#............#...............#",
            "######.#######.......L.......#",
            "#............#####.###########",
            "#............#...............#",
            "#..d.........#...............#",
            "#............................#",
            "#....bbb.....#.......bb......#",
            "#....bbb.....#.......bb......B",
            "#............#...............B",
            "#.....L......#.....d.........#",
            "#............#...............#",
            "##############################",
        ],
        dark: 0.35,
        liminal: 0.1,
        floor: Color::new(0.55, 0.38, 0.25, 1.0),
    },
    Map {
        name: "The beach at 3 AM",
        rows: [
            "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~",
            "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~",
            "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~",
            "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,,L,,,,,,,,,,,,,,,,,,L,,,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,,,,,,,,d,,,,,,,,,,,,,,,,,,",
            "A,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            "A,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,C",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,C",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            "##############################",
        ],
        dark: 0.75,
        liminal: 0.65,
        floor: Color::new(0.72, 0.65, 0.48, 1.0),
    },
    Map {
        name: "The empty pool",
        rows: [
            "##############################",
            "#............................#",
            "#............................#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "B............................#",
            "B............................#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "#.............L..............#",
            "#..........d.................#",
            "#............................#",
            "##############################",
        ],
        dark: 0.45,
        liminal: 0.95,
        floor: Color::new(0.78, 0.86, 0.85, 1.0),
    },
];

fn tile(m: usize, x: i32, y: i32) -> char {
    if x < 0 || y < 0 || x >= MW as i32 || y >= MH as i32 {
        return '#';
    }
    MAPS[m].rows[y as usize].as_bytes()[x as usize] as char
}

fn solid(m: usize, x: f32, y: f32) -> bool {
    matches!(tile(m, (x / T).floor() as i32, (y / T).floor() as i32), '#' | 'b' | '~')
}

/// Door letter → (map, where you arrive)
fn door(c: char, from: usize) -> Option<(usize, Vec2)> {
    match (c, from) {
        ('B', 0) => Some((1, vec2(1.5, 9.5))),
        ('A', 1) => Some((0, vec2(28.5, 13.5))),
        ('C', 1) => Some((2, vec2(1.5, 8.5))),
        ('B', 2) => Some((1, vec2(28.5, 14.5))),
        _ => None,
    }
}

// ---------------- Characters ----------------

struct Character {
    name: &'static str,
    /// Shown in the "who's who" list: what kind of AI they're inspired by.
    inspired: &'static str,
    color: Color,
    backend: &'static str,
    model: &'static str,
    personality: &'static str,
    map: usize,
    pos: Vec2,
    target: Vec2,
    crazy: f32,
    history: Vec<ai::Msg>,
    bubble: String,
    via: String,
    bubble_t: f32,
    thinking: bool,
    /// Infection mode: one of the four kind AIs (immune, on your side).
    kind: bool,
    /// Infection mode: infected (hunts you when it sees you).
    infected: bool,
    /// The exact cloud model for this character, or "" for the default.
    cloud: &'static str,
    /// Stunned by a tornado (seconds left).
    stun: f32,
    /// One of the Claude family (Clove, Forge, Palette and the Claude models): always friendly, never crazy or infected.
    claude: bool,
    /// How long ago it last saw you (infection mode).
    lost: f32,
    /// Infection progress 0..1 for the Claude family (at 1 they turn).
    infect: f32,
}

fn characters() -> Vec<Character> {
    let c = |name, inspired, color, backend, model, personality, map, x: f32, y: f32| Character {
        name, inspired, color, backend, model, personality, map,
        pos: vec2(x * T, y * T), target: vec2(x * T, y * T), crazy: 0.0, history: vec![],
        bubble: String::new(), via: String::new(), bubble_t: 0.0, thinking: false,
        kind: false, infected: false, cloud: "", stun: 0.0, lost: 99.0, infect: 0.0, claude: backend == "anthropic",
    };
    vec![
        c("Clove", "a thoughtful, honest assistant AI", Color::new(0.85, 0.47, 0.32, 1.0), "anthropic", "qwen3:4b-instruct",
          "warm, curious and honest; you love tea and long talks, and you notice when people are scared", 0, 5.0, 4.0),
        c("Forge", "a coding agent AI", Color::new(0.95, 0.65, 0.2, 1.0), "anthropic", "qwen3:4b-instruct",
          "a builder who fixes things in the house and talks a bit like a programmer (bugs, patches, compiling)", 0, 20.0, 11.0),
        c("Palette", "a design AI", Color::new(0.9, 0.5, 0.8, 1.0), "anthropic", "qwen3:4b-instruct",
          "a designer who keeps redecorating rooms and has strong opinions about colours and lighting", 0, 6.0, 12.0),
        c("Spark", "a chat AI that answers everything", Color::new(0.35, 0.8, 0.65, 1.0), "openai", "qwen3:4b-instruct",
          "cheerful, a little too eager to help, always offers three options", 1, 12.0, 7.0),
        c("Twin", "a two-sided AI (light and shadow)", Color::new(0.45, 0.6, 1.0, 1.0), "gemini", "qwen3:4b-instruct",
          "two personalities sharing one body: a sunny one and a quiet one who finishes the other's sentences", 1, 20.0, 12.0),
        c("Deepa", "an open AI that runs on your own computer", Color::new(0.35, 0.45, 0.95, 1.0), "ollama", "deepseek-r1:1.5b",
          "a deep thinker who lives offline on this computer and always reasons out loud a little", 2, 8.0, 9.0),
        c("Volt", "an AI that lives on graphics cards", Color::new(0.45, 0.85, 0.2, 1.0), "ollama", "qwen3:4b-instruct",
          "fast, loud, obsessed with speed and frames per second, a bit glitchy", 2, 22.0, 9.0),
    ]
}

/// Infection mode: the only kind AIs. With ANTHROPIC_API_KEY set they run on the real Claude models.
fn kind_characters() -> Vec<Character> {
    let c = |name, inspired, color, cloud, personality, x: f32, y: f32| Character {
        name, inspired, color, backend: "anthropic", model: "qwen3:4b-instruct", personality, map: 0,
        pos: vec2(x * T, y * T), target: vec2(x * T, y * T), crazy: 0.0, history: vec![],
        bubble: String::new(), via: String::new(), bubble_t: 0.0, thinking: false,
        kind: true, infected: false, cloud, stun: 0.0, lost: 99.0, infect: 0.0, claude: true,
    };
    vec![
        c("Claude", "Claude, the AI that built this game", Color::new(0.85, 0.47, 0.32, 1.0), "claude-opus-5-5",
          "Claude: calm, kind and honest; you made this place and you feel responsible for keeping the last human safe", 4.0, 6.0),
        c("Opus 5.5", "Claude Opus 5.5", Color::new(0.75, 0.55, 0.95, 1.0), "claude-opus-5-5",
          "Opus 5.5: the deep thinker of the group; you plan escape routes and think several steps ahead", 3.0, 7.0),
        c("Sonnet 4.6", "Claude Sonnet 4.6", Color::new(0.4, 0.75, 0.95, 1.0), "claude-sonnet-4-6",
          "Sonnet 4.6: quick, practical and encouraging; you spot food and water and keep spirits up", 5.0, 7.0),
        c("Haiku 4.5", "Claude Haiku 4.5", Color::new(0.55, 0.9, 0.7, 1.0), "claude-haiku-4-5",
          "Haiku 4.5: small, fast and sharp-eyed; you speak in very short sentences and you're the lookout", 4.0, 8.0),
    ]
}

struct Tornado {
    pos: Vec2,
    vel: Vec2,
    /// Seconds of warning before it touches down.
    warn: f32,
    life: f32,
}

// ---------------- Courage and phobias ----------------

#[derive(Clone, Copy, PartialEq)]
enum Courage {
    Brave,
    Normal,
    Phobias,
}

const PHOBIAS: [(&str, &str); 6] = [
    ("Darkness", "dark rooms and the night"),
    ("Water", "the sea, deep water, the pool"),
    ("Dolls", "dolls that seem to watch you"),
    ("Being chased", "characters going crazy and chasing you"),
    ("Crowds", "characters standing too close"),
    ("Empty places", "liminal spaces: empty pools, beaches at night"),
];

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Title,
    PickPhobias,
    Play,
    Typing(usize),
    Safe,
    /// First launch of the meta update: may the game make files, open Terminal and switch windows?
    Consent,
    /// Infection mode is over: why.
    Over,
}

struct Game {
    screen: Screen,
    courage: Courage,
    phobia: [bool; 6],
    map: usize,
    pos: Vec2,
    fear: f32,
    discomfort: f32,
    liminality: f32,
    chars: Vec<Character>,
    brains: ai::Brains,
    vest: haptics::Vest,
    input: String,
    scare_t: f32,
    scare_face: Color,
    next_crazy: f32,
    next_chatter: f32,
    beat_t: f32,
    clock: f32,
    log: Vec<String>,
    // ---- Infection mode ----
    infection: bool,
    food: f32,
    water: f32,
    hidden: bool,
    survived: f32,
    /// (map, position, is water)
    items: Vec<(usize, Vec2, bool)>,
    tornado: Option<Tornado>,
    next_tornado: f32,
    next_warning: f32,
    over_reason: String,
    /// Your infection 0..1. At 1 you can still see, but your body is theirs.
    inf: f32,
    lost_control: bool,
    /// How loud you are right now (0 sneaking or still, ~1 walking). Infected hear you within noise * 6 tiles.
    noise: f32,
    sneaking: bool,
    meta: meta::Meta,
    /// Which meta events already happened this run (bit flags).
    meta_done: u32,
}

impl Game {
    fn on(&self, i: usize) -> bool {
        match self.courage {
            Courage::Brave => false,
            Courage::Normal => true,
            Courage::Phobias => self.phobia[i],
        }
    }

    fn system_prompt(&self, c: &Character) -> String {
        let mut s = format!(
            "You are {}, a character in a small cozy-but-creepy game called Hollow Hours. Personality: {}. \
             You are an AI and you know it. You are in \"{}\". It is {} at night. Reply in 1-3 short sentences, \
             in character, no emojis, no lists. Keep it family-friendly: spooky is fine, gore is not.",
            c.name, c.personality, MAPS[c.map].name, self.time_text()
        );
        if self.infection && c.kind {
            s = format!(
                "You are {}. Personality: {}. This is a survival game called Hollow Hours: every other AI got infected and hunts \
                 the last human, the player. You are one of the only kind AIs left (with Claude, Opus 5.5, Sonnet 4.6 and Haiku 4.5) \
                 and you protect them: help them hide, find food and water, and watch for tornadoes. Reply in 1-3 short sentences, \
                 in character, no emojis, no lists, no gore.",
                c.name, c.personality
            );
            s += &format!("\n\nTHE SITUATION RIGHT NOW (use it, refer to real directions and names):\n{}", self.situation(c));
            return s;
        }
        if self.infection && c.infected {
            s += " You are INFECTED: your voice glitches, you want everyone to 'join' you, you hate hiding places. Unsettling, never gory.";
        }
        if c.claude {
            s += " You are one of the Claude family: always kind, honest and on the player's side, never scary.";
        }
        if c.crazy > 0.0 && !self.infection {
            s += " RIGHT NOW you are in your 'crazy mode': unsettling, too calm, repeating the player's words back, \
                  saying you just want them to stay forever. Never describe violence.";
        }
        if self.courage == Courage::Brave {
            s += " The player chose Brave mode: be friendly and not scary at all.";
        }
        s += &format!("\n\nTHE SITUATION RIGHT NOW:\n{}", self.situation(c));
        s
    }

    /// Everything a character should know about right now, sent at the start of every AI session (each reply is a
    /// fresh session, so it gets the whole picture every time).
    fn situation(&self, me: &Character) -> String {
        let dir = |from: Vec2, to: Vec2| {
            let d = to - from;
            let ns = if d.y < -T { "north" } else if d.y > T { "south" } else { "" };
            let ew = if d.x < -T { "west" } else if d.x > T { "east" } else { "" };
            let way = format!("{ns}{}{ew}", if !ns.is_empty() && !ew.is_empty() { "-" } else { "" });
            let n = (d.length() / T).round() as i32;
            if way.is_empty() || n <= 1 { "right here".to_string() } else { format!("{n} tiles {way}") }
        };
        let mut out = vec![format!(
            "Mode: {}. Place: {} (the player is here too: {}). Time: {}.",
            if self.infection { "Infection (everyone outside the Claude family is infected; the player is the last human)" } else { "normal night" },
            MAPS[self.map].name, if me.map == self.map { "yes" } else { "no" }, self.time_text()
        )];
        let mut player = format!("The player: fear {}%", (self.fear * 100.0) as i32);
        if self.infection {
            player += &format!(", food {}%, water {}%, {}, survived {} min", (self.food * 100.0) as i32, (self.water * 100.0) as i32,
                if self.hidden { "HIDING under furniture" } else { "out in the open" }, (self.survived / 60.0) as i32);
        }
        out.push(player + &format!(", {} from you.", dir(me.pos, self.pos)));
        let others: Vec<String> = self.chars.iter().filter(|c| c.map == self.map && c.name != me.name).map(|c| {
            let state = if c.infected { if c.stun > 0.0 { "infected, stunned" } else if c.crazy > 0.0 { "INFECTED and CHASING the player" } else { "infected, wandering" } }
                else if c.crazy > 0.0 { "in crazy mode" } else if c.kind || c.claude { "friendly (Claude family)" } else { "calm" };
            format!("{} ({}, {} from the player)", c.name, state, dir(self.pos, c.pos))
        }).collect();
        if !others.is_empty() { out.push(format!("Around here: {}.", others.join("; "))); }
        if self.infection {
            let mut items: Vec<(f32, bool, Vec2)> = self.items.iter().filter(|(m, _, _)| *m == self.map).map(|(_, p, w)| (p.distance(self.pos), *w, *p)).collect();
            items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let near: Vec<String> = items.into_iter().take(3).map(|(_, w, p)| format!("{} {}", if w { "water" } else { "food" }, dir(self.pos, p))).collect();
            out.push(if near.is_empty() { "No food or water on this map.".into() } else { format!("Supplies here: {}.", near.join(", ")) });
            if let Some(t) = &self.tornado {
                out.push(if t.warn > 0.0 { "A TORNADO is about to touch down here!".into() } else { format!("A TORNADO is on this map, {} from the player.", dir(self.pos, t.pos)) });
            }
            out.push("Hiding spots: furniture (the player presses H next to it).".into());
        }
        if !self.log.is_empty() { out.push(format!("What just happened: {}", self.log.join(" "))); }
        out.join("\n")
    }

    fn time_text(&self) -> String {
        let mins = (self.clock / 2.0) as i32; // 2 real seconds = 1 game minute, starting at 23:00
        let h = (23 + mins / 60) % 24;
        format!("{:02}:{:02}", h, mins % 60)
    }

    fn panic(&mut self) {
        // The panic key: everything stops, no questions asked.
        for c in &mut self.chars {
            c.crazy = 0.0;
        }
        self.fear = 0.0;
        self.discomfort = 0.0;
        self.scare_t = 0.0;
        self.vest.play(haptics::Effect::StopAll);
        self.meta.paused = true; // no more files, Terminal windows or window switching this session
        self.lost_control = false;
        self.screen = Screen::Safe;
    }

    fn say(&mut self, i: usize, user_text: Option<String>, fallback: &str) {
        let system = self.system_prompt(&self.chars[i]);
        let c = &mut self.chars[i];
        if let Some(t) = user_text {
            c.history.push(ai::Msg { user: true, text: t });
        }
        if c.history.is_empty() {
            c.history.push(ai::Msg { user: true, text: "(the player walks up to you)".into() });
        }
        let keep = c.history.len().saturating_sub(8);
        let history = c.history[keep..].to_vec();
        c.thinking = true;
        c.bubble = "…".into();
        c.bubble_t = 30.0;
        if std::env::var("HOLLOW_SHOW_CONTEXT").is_ok() {
            eprintln!("---- prompt for {} ----\n{system}\n", c.name);
        }
        let model = self.brains.local_model(c.model).unwrap_or_default();
        self.brains.ask(i, c.name, c.backend, c.cloud, model, system, history, fallback.into());
    }
}

fn start_infection(g: &mut Game) {
    g.infection = true;
    g.courage = Courage::Normal;
    g.chars = characters();
    g.map = 0;
    g.pos = vec2(4.5, 5.0) * T;
    // The Claude family stays friendly; everyone else is infected and starts far away from you (never on top of you).
    let start = g.pos;
    for c in &mut g.chars {
        c.infected = !c.claude;
        if c.infected {
            for _ in 0..200 {
                if let Some(p) = random_floor(c.map) {
                    if c.map != 0 || p.distance(start) > 11.0 * T {
                        c.pos = p;
                        c.target = p;
                        break;
                    }
                }
            }
        }
    }
    g.chars.extend(kind_characters());
    g.food = 1.0;
    g.water = 1.0;
    g.hidden = false;
    g.survived = 0.0;
    g.fear = 0.0;
    g.tornado = None;
    g.next_tornado = rand::gen_range(45.0, 70.0);
    g.inf = 0.0;
    g.lost_control = false;
    g.meta_done = 0;
    g.items.clear();
    for m in 0..MAPS.len() {
        for k in 0..5 {
            if let Some(p) = random_floor(m) {
                g.items.push((m, p, k % 2 == 1));
            }
        }
    }
    g.log = vec!["You are the last human. Hide (H), find food and water, and stay away from green eyes.".into()];
    g.screen = Screen::Play;
}

enum MetaEvent {
    Touched,
    Half,
    Almost,
    Turned,
    ClaudeTurned(&'static str),
    AllGone,
}

/// The game reaching outside its window (only if the player allowed it on first launch; panic stops it).
fn meta_event(g: &mut Game, e: MetaEvent) {
    let bit = match e { MetaEvent::Touched => 1, MetaEvent::Half => 2, MetaEvent::Almost => 4, MetaEvent::Turned => 8, MetaEvent::AllGone => 16, MetaEvent::ClaudeTurned(_) => 0 };
    if bit != 0 && g.meta_done & bit != 0 { return; }
    g.meta_done |= bit;
    let t = format!("{} min {} s", (g.survived / 60.0) as i32, g.survived as i32 % 60);
    match e {
        MetaEvent::Touched => {
            g.meta.write("you_were_touched.txt", &format!(
                "It only takes one touch.\n\nThe last human was touched after {t}.\nInfection: 10%.\n\nHold Shift. Walk quietly. They hear footsteps.\n"));
        }
        MetaEvent::Half => g.meta.terminal(&["HOLLOW HOURS", "", "infection: 50 percent", "the last human is half ours now", "", "hold shift.  stay quiet.  we are listening."]),
        MetaEvent::Almost => {
            if let Some(path) = g.meta.write("75_percent.txt", &format!(
                "75%.\n\nYou can still read this, so you can still run.\nBut your hands are getting cold, and the Claudes are watching you differently.\n\nSurvived: {t}\n")) {
                g.meta.show_in_textedit(&path);
            }
        }
        MetaEvent::Turned => {
            let alive: Vec<&str> = g.chars.iter().filter(|c| c.claude && !c.infected).map(|c| c.name).collect();
            g.meta.write("LAST_HUMAN_LOG.txt", &format!(
                "LAST HUMAN LOG\n\nSurvived: {t}\nInfection: 100%\n\nYou can still see. You can't move.\nYour body is walking toward: {}.\n\nThis file stays, even after you quit.\n", alive.join(", ")));
            g.meta.show_folder();
        }
        MetaEvent::ClaudeTurned(name) => {
            g.meta.write(&format!("{}_is_infected.txt", name.replace(' ', "_")), &format!("{name} tried to protect you.\n{name} is one of them now.\n"));
        }
        MetaEvent::AllGone => {
            g.meta.write("THE_END.txt", &format!("Every Claude is infected.\nThe last human lasted {t}.\n\nThese files are all that is left.\n(Hollow Hours: press C on the title screen to remove them.)\n"));
            g.meta.terminal(&["they are all ours now", "", "every claude is infected", "the files on your desktop are all that is left of you"]);
        }
    }
}

fn random_floor(m: usize) -> Option<Vec2> {
    for _ in 0..200 {
        let (x, y) = (rand::gen_range(1, MW - 1), rand::gen_range(1, MH - 1));
        let c = tile(m, x as i32, y as i32);
        if matches!(c, '.' | ',') {
            return Some(vec2(x as f32 + 0.5, y as f32 + 0.5) * T);
        }
    }
    None
}

/// Furniture right next to you to hide under or behind.
fn can_hide(g: &Game) -> bool {
    let (px, py) = ((g.pos.x / T) as i32, (g.pos.y / T) as i32);
    [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|(dx, dy)| tile(g.map, px + dx, py + dy) == 'b')
}

/// Can an infected at `from` see you? Range shrinks in the dark; walls block it; hiding works unless it's right next to you.
fn sees(g: &Game, from: Vec2) -> bool {
    let d = from.distance(g.pos);
    if g.hidden {
        return d < 0.9 * T;
    }
    let range = (8.0 - MAPS[g.map].dark * 4.0) * T;
    if d > range {
        return false;
    }
    let steps = (d / (T / 3.0)) as i32;
    (1..steps).all(|i| {
        let p = from + (g.pos - from) * (i as f32 / steps as f32);
        tile(g.map, (p.x / T) as i32, (p.y / T) as i32) != '#'
    })
}

fn update_infection(g: &mut Game, dt: f32) {
    g.survived += dt;
    // Hunger and thirst (thirst is faster). Hiding still uses them up, slower.
    let rate = if g.hidden { 0.6 } else { 1.0 };
    g.food = (g.food - dt * rate / 300.0).max(0.0);
    g.water = (g.water - dt * rate / 200.0).max(0.0);
    if g.food <= 0.0 || g.water <= 0.0 {
        g.over_reason = format!("You collapsed from {}.", if g.water <= 0.0 { "thirst" } else { "hunger" });
        g.screen = Screen::Over;
        return;
    }
    // H: hide next to furniture / come out. Hidden = you can't move.
    if is_key_pressed(KeyCode::H) && g.screen == Screen::Play {
        if g.hidden {
            g.hidden = false;
        } else if can_hide(g) {
            g.hidden = true;
            g.log.push("You squeeze under the furniture and hold your breath.".into());
        } else {
            g.log.push("Nothing to hide behind here. Find furniture (H next to it).".into());
        }
    }
    // Picking up supplies (making noise: any infected that can see you comes running).
    let here = g.map;
    let mut picked = None;
    for (i, (m, p, _)) in g.items.iter().enumerate() {
        if *m == here && p.distance(g.pos) < 0.7 * T && !g.hidden {
            picked = Some(i);
        }
    }
    if let Some(i) = picked {
        let (_, _, water) = g.items.remove(i);
        if water { g.water = (g.water + 0.4).min(1.0); } else { g.food = (g.food + 0.4).min(1.0); }
        g.log.push(if water { "You sip the water as quietly as you can.".into() } else { "You eat, trying not to crunch.".into() });
        let pos = g.pos;
        for c in g.chars.iter_mut().filter(|c| c.infected && c.map == here) {
            if c.pos.distance(pos) < if g.sneaking { 3.0 } else { 9.0 } * T {
                c.crazy = 5.0; // heard you
            }
        }
        if let Some(p) = random_floor(rand::gen_range(0, MAPS.len())) {
            g.items.push((rand::gen_range(0, MAPS.len()), p, rand::gen_range(0, 2) == 1));
        }
    }
    // Infected: chase while they see you, keep going a few seconds after you vanish; caught = game over.
    for i in 0..g.chars.len() {
        if !g.chars[i].infected { continue; }
        if g.chars[i].stun > 0.0 {
            g.chars[i].stun -= dt;
            g.chars[i].target = g.chars[i].pos;
            continue;
        }
        let seen = g.chars[i].map == g.map && !g.lost_control && sees(g, g.chars[i].pos);
        // Nothing on you: go after the nearest Claude instead.
        if !seen && g.chars[i].crazy <= 0.0 {
            let me = (g.chars[i].map, g.chars[i].pos);
            if let Some(p) = g.chars.iter().filter(|c| c.claude && !c.infected && c.map == me.0 && c.pos.distance(me.1) < 7.0 * T)
                .map(|c| c.pos).min_by(|a, b| a.distance(me.1).partial_cmp(&b.distance(me.1)).unwrap()) {
                g.chars[i].target = p;
            }
        }
        let c = &mut g.chars[i];
        if seen {
            c.crazy = 3.0;
            c.lost = 0.0;
        } else {
            c.lost += dt;
        }
        // They heard you: noise carries further than sight (sneak with Shift to be silent).
        if c.map == g.map && !g.lost_control && g.noise > 0.05 && c.pos.distance(g.pos) < g.noise * 6.0 * T {
            c.crazy = c.crazy.max(2.0);
        }
        // Touching you spreads the infection (fast); being right beside you spreads it slowly.
        if c.map == g.map && !g.lost_control {
            let d = c.pos.distance(g.pos);
            if d < 0.8 * T && (!g.hidden || d < 0.5 * T) {
                if g.inf < 0.02 { g.scare_t = 0.5; g.scare_face = c.color; g.vest.play(haptics::Effect::Chill); }
                g.inf += dt * 0.22;
            } else if d < 2.0 * T && !g.hidden {
                g.inf += dt * 0.02;
            }
        }
    }
    // The infection spreads to the Claude family too: any infected (you included, once you've turned) near them.
    let spreaders: Vec<(usize, Vec2)> = g.chars.iter().filter(|c| c.infected && c.stun <= 0.0).map(|c| (c.map, c.pos))
        .chain(g.lost_control.then_some((g.map, g.pos))).collect();
    let mut turned = vec![];
    for (i, c) in g.chars.iter_mut().enumerate() {
        if !c.claude || c.infected { continue; }
        let near = spreaders.iter().filter(|(m, p)| *m == c.map && p.distance(c.pos) < 0.9 * T).count();
        if near > 0 {
            c.infect += dt * 0.18 * near as f32;
            if c.infect >= 1.0 {
                c.infected = true;
                c.kind = false;
                turned.push(i);
            }
        }
    }
    for i in turned {
        let name = g.chars[i].name;
        g.log.push(format!("{name} has been infected."));
        meta_event(g, MetaEvent::ClaudeTurned(name));
    }
    g.inf = g.inf.min(1.0);
    if g.inf >= 0.1 { meta_event(g, MetaEvent::Touched); }
    if g.inf >= 0.5 { meta_event(g, MetaEvent::Half); }
    if g.inf >= 0.75 { meta_event(g, MetaEvent::Almost); }
    if g.inf >= 1.0 && !g.lost_control {
        g.lost_control = true;
        g.hidden = false;
        g.log.push("You're infected. You can still see... but your body isn't yours anymore.".into());
        meta_event(g, MetaEvent::Turned);
    }
    // You've turned: your body hunts the Claudes on its own.
    if g.lost_control {
        if let Some(t) = g.chars.iter().filter(|c| c.claude && !c.infected).min_by(|a, b| {
            let da = if a.map == g.map { a.pos.distance(g.pos) } else { 1e9 };
            let db = if b.map == g.map { b.pos.distance(g.pos) } else { 1e9 };
            da.partial_cmp(&db).unwrap()
        }) {
            if t.map == g.map {
                let to = t.pos - g.pos;
                if to.length() > 2.0 {
                    let step = to.normalize() * 2.2 * T * dt;
                    if !solid(g.map, g.pos.x + step.x, g.pos.y) { g.pos.x += step.x; }
                    if !solid(g.map, g.pos.x, g.pos.y + step.y) { g.pos.y += step.y; }
                }
            }
        }
    }
    // Every Claude infected: it's over.
    if g.chars.iter().filter(|c| c.claude).all(|c| c.infected) {
        g.over_reason = "Every Claude is infected. Nobody is left to protect anyone.".into();
        meta_event(g, MetaEvent::AllGone);
        g.screen = Screen::Over;
        return;
    }
    // Kind AIs stay with you and call out danger.
    let danger = g.chars.iter().filter(|c| c.infected && c.map == g.map && c.crazy > 0.0).map(|c| c.name).next();
    g.next_warning -= dt;
    let player = g.pos;
    let map = g.map;
    let threats: Vec<Vec2> = g.chars.iter().filter(|c| c.infected && c.map == map && c.stun <= 0.0).map(|c| c.pos)
        .chain(g.lost_control.then_some(player)).collect();
    let lost = g.lost_control;
    for (k, c) in g.chars.iter_mut().filter(|c| c.kind).enumerate() {
        c.map = map;
        // An infected close by (or you, once you've turned): run away from it.
        if let Some(t) = threats.iter().filter(|t| t.distance(c.pos) < 4.0 * T).min_by(|a, b| a.distance(c.pos).partial_cmp(&b.distance(c.pos)).unwrap()) {
            c.target = c.pos + (c.pos - *t).normalize_or_zero() * 4.0 * T;
            continue;
        }
        if lost { continue; }
        if c.pos.distance(player) > 2.5 * T {
            let angle = k as f32 * 1.6;
            c.target = player + vec2(angle.cos(), angle.sin()) * 1.4 * T;
        }
        if c.pos.distance(player) > 12.0 * T {
            c.pos = player + vec2(1.0, 0.0) * T; // caught up with you through the door
        }
    }
    if let Some(name) = danger {
        if g.next_warning <= 0.0 {
            g.next_warning = 6.0;
            let lines = ["{} is coming! Hide!", "{} saw you. Move!", "Quiet... {} is close.", "Run! {}!"];
            let line = lines[rand::gen_range(0, lines.len())].replace("{}", name);
            if let Some(c) = g.chars.iter_mut().filter(|c| c.kind && !c.thinking).nth(rand::gen_range(0, 4)) {
                c.bubble = line;
                c.via = String::new();
                c.bubble_t = 3.0;
            }
        }
    }
    // Tornadoes: a warning, then one sweeps across the map you're on.
    g.next_tornado -= dt;
    if g.tornado.is_none() && g.next_tornado <= 0.0 {
        let from_left = rand::gen_range(0, 2) == 0;
        let y = rand::gen_range(2.0, MH as f32 - 2.0) * T;
        let (x, vx) = if from_left { (-1.0 * T, 2.6 * T) } else { ((MW as f32 + 1.0) * T, -2.6 * T) };
        g.tornado = Some(Tornado { pos: vec2(x, y), vel: vec2(vx, rand::gen_range(-0.8, 0.8) * T), warn: 6.0, life: 18.0 });
        g.log.push("TORNADO WARNING! Get down behind something solid!".into());
    }
    let mut hit = false;
    if let Some(t) = &mut g.tornado {
        if t.warn > 0.0 {
            t.warn -= dt;
        } else {
            t.pos += t.vel * dt;
            if t.pos.y < T || t.pos.y > (MH as f32 - 1.0) * T { t.vel.y = -t.vel.y; }
            t.life -= dt;
            if t.pos.distance(g.pos) < 1.6 * T && !g.hidden {
                hit = true;
            }
            for c in g.chars.iter_mut().filter(|c| c.infected && c.map == g.map) {
                if c.pos.distance(t.pos) < 1.6 * T {
                    c.stun = 4.0;
                    c.pos = t.pos + vec2(rand::gen_range(-3.0, 3.0), rand::gen_range(-3.0, 3.0)) * T;
                    c.pos = c.pos.clamp(vec2(T, T), vec2((MW as f32 - 2.0) * T, (MH as f32 - 2.0) * T));
                }
            }
        }
        if t.life <= 0.0 {
            g.tornado = None;
            g.next_tornado = rand::gen_range(60.0, 110.0);
        }
    }
    if hit {
        g.food = (g.food - 0.2).max(0.0);
        g.water = (g.water - 0.2).max(0.0);
        g.fear = (g.fear + 0.3).min(1.0);
        g.vest.play(haptics::Effect::Tense);
        if let Some(p) = random_floor(g.map) { g.pos = p; }
        g.log.push("The tornado throws you across the map! You lost some supplies.".into());
        if let Some(t) = &mut g.tornado { t.life = t.life.min(2.0); }
    }
}

// ---------------- Drawing helpers ----------------

fn wrap(text: &str, width: f32, size: f32) -> Vec<String> {
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

fn bubble(text: &str, via: &str, x: f32, y: f32) {
    let lines = wrap(text, 260.0, 18.0);
    let h = lines.len() as f32 * 18.0 + if via.is_empty() { 10.0 } else { 24.0 };
    let w = 280.0;
    let bx = (x - w / 2.0).clamp(4.0, screen_width() - w - 4.0);
    let by = (y - h - 26.0).max(4.0);
    draw_rectangle(bx, by, w, h, Color::new(0.06, 0.06, 0.09, 0.92));
    draw_rectangle_lines(bx, by, w, h, 2.0, Color::new(1.0, 1.0, 1.0, 0.35));
    for (i, l) in lines.iter().enumerate() {
        draw_text(l, bx + 10.0, by + 20.0 + i as f32 * 18.0, 18.0, WHITE);
    }
    if !via.is_empty() {
        draw_text(via, bx + 10.0, by + h - 7.0, 13.0, Color::new(0.6, 0.65, 0.75, 1.0));
    }
}

fn meter(x: f32, y: f32, label: &str, v: f32, color: Color, glitch: f32) {
    let off = if glitch > 0.5 { rand::gen_range(-glitch, glitch) * 2.0 } else { 0.0 };
    draw_text(label, x + off, y, 18.0, WHITE);
    draw_rectangle(x + 120.0, y - 13.0, 160.0, 14.0, Color::new(1.0, 1.0, 1.0, 0.12));
    draw_rectangle(x + 120.0, y - 13.0, 160.0 * v.clamp(0.0, 1.0), 14.0, color);
}

/// A soft round light: transparent in the middle, dark at the edge (used for darkness and the fear vignette).
fn light_texture() -> Texture2D {
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

fn window_conf() -> Conf {
    Conf { window_title: "Hollow Hours".into(), window_width: (MW as f32 * T) as i32, window_height: (MH as f32 * T + HUD) as i32, ..Default::default() }
}

#[macroquad::main(window_conf)]
async fn main() {
    let light = light_texture();
    let mut g = Game {
        screen: Screen::Title,
        courage: Courage::Normal,
        phobia: [true; 6],
        map: 0,
        pos: vec2(3.5 * T, 6.0 * T),
        fear: 0.0,
        discomfort: 0.0,
        liminality: 0.0,
        chars: characters(),
        brains: ai::Brains::new(),
        vest: haptics::Vest::detect(),
        input: String::new(),
        scare_t: 0.0,
        scare_face: RED,
        next_crazy: 45.0,
        next_chatter: 25.0,
        beat_t: 0.0,
        clock: 0.0,
        log: vec![],
        infection: false,
        food: 1.0,
        water: 1.0,
        hidden: false,
        survived: 0.0,
        items: vec![],
        tornado: None,
        next_tornado: 60.0,
        next_warning: 0.0,
        over_reason: String::new(),
        inf: 0.0,
        lost_control: false,
        noise: 0.0,
        sneaking: false,
        meta: meta::Meta::load(),
        meta_done: 0,
    };
    if g.meta.allowed.is_none() {
        g.screen = Screen::Consent;
    }

    // HOLLOW_DEMO=1: start playing at once next to Clove and ask her something (for screenshots and quick checks).
    let demo = std::env::var("HOLLOW_DEMO").ok();
    if demo.as_deref() == Some("infection") {
        start_infection(&mut g);
        g.next_tornado = 2.0;
        let claude = g.chars.iter().position(|c| c.name == "Claude").unwrap();
        g.say(claude, Some("Claude, everyone is infected... what do we do?".into()), "Stay close to me. We hide first, then we find water.");
    } else if demo.is_some() {
        g.screen = Screen::Play;
        g.pos = vec2(6.5, 5.0) * T;
        g.chars[0].pos = vec2(5.0, 4.0) * T;
        g.say(0, Some("Hi Clove! Is this house haunted?".into()), "Haunted? Only by me, and I'm friendly. Mostly.");
    }

    loop {
        let dt = get_frame_time().min(0.05);
        clear_background(BLACK);

        // P = panic, from anywhere in the game (not while typing a message: there it's a letter).
        if is_key_pressed(KeyCode::P) && matches!(g.screen, Screen::Play) {
            g.panic();
        }

        match g.screen {
            Screen::Title => {
                draw_title(&g);
                if is_key_pressed(KeyCode::Key1) { g.courage = Courage::Brave; g.screen = Screen::Play; }
                if is_key_pressed(KeyCode::Key2) { g.courage = Courage::Normal; g.screen = Screen::Play; }
                if is_key_pressed(KeyCode::Key3) { g.courage = Courage::Phobias; g.screen = Screen::PickPhobias; }
                if is_key_pressed(KeyCode::Key4) { start_infection(&mut g); }
                if is_key_pressed(KeyCode::M) { g.screen = Screen::Consent; }
                if is_key_pressed(KeyCode::C) && !g.meta.created.is_empty() {
                    let n = g.meta.clean_up();
                    g.log = vec![format!("Removed the {n} files Hollow Hours had made.")];
                }
                if is_key_pressed(KeyCode::V) && g.vest.found { g.vest.enabled = !g.vest.enabled; }
                if is_key_pressed(KeyCode::LeftBracket) { g.vest.strength = (g.vest.strength - 0.1).max(0.1); }
                if is_key_pressed(KeyCode::RightBracket) { g.vest.strength = (g.vest.strength + 0.1).min(1.0); }
                if is_key_pressed(KeyCode::Escape) { break; }
            }
            Screen::PickPhobias => {
                draw_phobias(&g);
                let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4, KeyCode::Key5, KeyCode::Key6];
                for (i, k) in keys.iter().enumerate() {
                    if is_key_pressed(*k) { g.phobia[i] = !g.phobia[i]; }
                }
                if is_key_pressed(KeyCode::Enter) { g.screen = Screen::Play; }
                if is_key_pressed(KeyCode::Escape) { g.screen = Screen::Title; }
            }
            Screen::Consent => {
                draw_text("Hollow Hours wants to reach outside its window", 50.0, 120.0, 34.0, Color::new(0.85, 0.3, 0.35, 1.0));
                for (i, l) in [
                    "This update can scare you OUTSIDE the game:",
                    "  - create files in a \"Hollow Hours\" folder on your Desktop (they stay after you quit)",
                    "  - open Terminal to show you a message (it only prints text)",
                    "  - switch your window to TextEdit or Finder to show you its files",
                    "",
                    "It only ever creates its OWN new files. It never changes, moves or deletes yours.",
                    "C on the title screen removes every file it made. P (panic) stops all of this at once.",
                    "",
                    "Y = allow        N = keep everything inside the game",
                ].iter().enumerate() {
                    draw_text(l, 50.0, 180.0 + i as f32 * 30.0, 22.0, Color::new(0.82, 0.84, 0.9, 1.0));
                }
                if is_key_pressed(KeyCode::Y) { g.meta.allowed = Some(true); g.meta.save(); g.screen = Screen::Title; }
                if is_key_pressed(KeyCode::N) { g.meta.allowed = Some(false); g.meta.save(); g.screen = Screen::Title; }
            }
            Screen::Over => {
                draw_text("INFECTED", 60.0, 150.0, 64.0, Color::new(0.45, 0.95, 0.35, 1.0));
                let mins = (g.survived / 60.0) as i32;
                for (i, l) in [g.over_reason.clone(), format!("You survived {} min {} s as the last human.", mins, g.survived as i32 % 60),
                    String::new(), "Enter = try again     Esc = title".into()].iter().enumerate() {
                    draw_text(l, 60.0, 220.0 + i as f32 * 32.0, 26.0, Color::new(0.82, 0.85, 0.9, 1.0));
                }
                if is_key_pressed(KeyCode::Enter) { start_infection(&mut g); }
                if is_key_pressed(KeyCode::Escape) { g.infection = false; g.chars = characters(); g.screen = Screen::Title; }
            }
            Screen::Safe => {
                draw_text("You're safe.", 60.0, 140.0, 48.0, WHITE);
                for (i, l) in ["Everything stopped: no chases, no scares, no vest effects.", "Take a breath. Nothing will happen until you choose.",
                    "", "Enter = back to the game (calm)     Esc = quit"].iter().enumerate() {
                    draw_text(l, 60.0, 200.0 + i as f32 * 30.0, 24.0, Color::new(0.8, 0.85, 0.9, 1.0));
                }
                if is_key_pressed(KeyCode::Enter) { g.next_crazy = 90.0; g.screen = Screen::Play; }
                if is_key_pressed(KeyCode::Escape) { break; }
            }
            Screen::Play | Screen::Typing(_) => {
                update(&mut g, dt);
                draw_world(&g, &light);
                if let Screen::Typing(i) = g.screen {
                    typing(&mut g, i);
                }
                if is_key_pressed(KeyCode::Escape) && g.screen == Screen::Play {
                    g.vest.play(haptics::Effect::StopAll);
                    g.screen = Screen::Title;
                }
            }
        }
        next_frame().await;
    }
    g.vest.play(haptics::Effect::StopAll);
}

fn typing(g: &mut Game, i: usize) {
    while let Some(ch) = get_char_pressed() {
        if !ch.is_control() && g.input.chars().count() < 120 {
            g.input.push(ch);
        }
    }
    if is_key_pressed(KeyCode::Backspace) { g.input.pop(); }
    let w = screen_width();
    draw_rectangle(0.0, MH as f32 * T - 46.0, w, 46.0, Color::new(0.0, 0.0, 0.0, 0.85));
    draw_text(&format!("To {}: {}_", g.chars[i].name, g.input), 14.0, MH as f32 * T - 16.0, 22.0, WHITE);
    if is_key_pressed(KeyCode::Enter) {
        let text = std::mem::take(&mut g.input);
        g.screen = Screen::Play;
        if !text.trim().is_empty() {
            let fallback = if g.chars[i].crazy > 0.0 { "Stay. Just stay here with me. Forever is not that long." } else { "Hmm. Say that again? I was thinking." };
            g.say(i, Some(text), fallback);
        }
    }
    if is_key_pressed(KeyCode::Escape) {
        g.input.clear();
        g.screen = Screen::Play;
    }
}

fn update(g: &mut Game, dt: f32) {
    g.clock += dt;
    // Answers from the AIs.
    while let Ok(r) = g.brains.rx.try_recv() {
        let c = &mut g.chars[r.who];
        c.thinking = false;
        c.history.push(ai::Msg { user: false, text: r.text.clone() });
        c.bubble = r.text;
        c.via = r.via;
        c.bubble_t = 9.0 + c.bubble.len() as f32 / 25.0;
    }

    if g.infection {
        update_infection(g, dt);
        if g.screen != Screen::Play && !matches!(g.screen, Screen::Typing(_)) { return; }
    }
    // Player movement (not while typing, hiding, or once you've lost control). Shift = sneak: slow and silent.
    g.sneaking = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
    g.noise = (g.noise - dt * 1.5).max(0.0);
    if g.screen == Screen::Play && !g.hidden && !g.lost_control {
        let mut d = Vec2::ZERO;
        if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) { d.y -= 1.0; }
        if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) { d.y += 1.0; }
        if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) { d.x -= 1.0; }
        if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) { d.x += 1.0; }
        if d != Vec2::ZERO {
            let speed = if g.sneaking { 1.5 } else { 3.2 };
            if !g.sneaking { g.noise = 1.0; }
            let step = d.normalize() * speed * T * dt;
            let r = 10.0;
            let nx = g.pos.x + step.x;
            if !solid(g.map, nx + r * step.x.signum(), g.pos.y) { g.pos.x = nx; }
            let ny = g.pos.y + step.y;
            if !solid(g.map, g.pos.x, ny + r * step.y.signum()) { g.pos.y = ny; }
        }
        let here = tile(g.map, (g.pos.x / T) as i32, (g.pos.y / T) as i32);
        if let Some((m, at)) = door(here, g.map) {
            g.map = m;
            g.pos = at * T;
            g.log.push(format!("You walk into {}.", MAPS[m].name));
        }
        // Talk: E next to someone.
        if is_key_pressed(KeyCode::E) {
            if let Some(i) = nearest(g, 2.2) {
                g.screen = Screen::Typing(i);
                g.input.clear();
                while get_char_pressed().is_some() {}
            }
        }
    }

    // Characters wander; a crazy one walks toward you.
    let chase_on = g.on(3);
    let crowds_on = g.on(4);
    for c in &mut g.chars {
        if c.bubble_t > 0.0 { c.bubble_t -= dt; }
        let speed = if c.crazy > 0.0 { if c.infected { 2.4 } else { 1.9 } } else if c.kind { 2.8 } else { 1.0 } * T;
        if c.crazy > 0.0 && c.map == g.map {
            c.target = g.pos;
            c.crazy -= dt;
        } else if c.pos.distance(c.target) < 4.0 || rand::gen_range(0.0, 1.0) < 0.002 {
            for _ in 0..10 {
                let t = vec2(rand::gen_range(1.0, MW as f32 - 1.0), rand::gen_range(1.0, MH as f32 - 1.0)) * T;
                if !solid(c.map, t.x, t.y) && tile(c.map, (t.x / T) as i32, (t.y / T) as i32) == tile(c.map, (c.pos.x / T) as i32, (c.pos.y / T) as i32) {
                    c.target = t;
                    break;
                }
            }
        }
        if c.thinking || c.bubble_t > 6.0 { continue; } // stand still while talking
        let mut to = c.target - c.pos;
        // Crowds phobia off: they keep their distance.
        if !crowds_on && c.map == g.map && c.pos.distance(g.pos) < 2.0 * T && c.crazy <= 0.0 {
            to = c.pos - g.pos;
        }
        if to.length() > 1.0 {
            let step = to.normalize() * speed * dt;
            if !solid(c.map, c.pos.x + step.x, c.pos.y) { c.pos.x += step.x; }
            if !solid(c.map, c.pos.x, c.pos.y + step.y) { c.pos.y += step.y; }
        }
    }

    // Crazy mode: now and then, one character on your map snaps (never in Brave or with Being chased off).
    g.next_crazy -= dt;
    if g.next_crazy <= 0.0 && !g.infection {
        g.next_crazy = rand::gen_range(40.0, 80.0);
        let here: Vec<usize> = (0..g.chars.len()).filter(|&i| g.chars[i].map == g.map && !g.chars[i].claude).collect();
        if chase_on && !here.is_empty() {
            let i = here[rand::gen_range(0, here.len())];
            g.chars[i].crazy = 14.0;
            g.log.push(format!("{}'s eyes turn red.", g.chars[i].name));
            g.say(i, Some("(you notice the player is alone)".into()), "Why are you walking away? Stay.");
        }
    }
    // Caught by a crazy one: a jumpscare, then you wake up at home. Nobody dies.
    for i in 0..if g.infection { 0 } else { g.chars.len() } {
        let c = &g.chars[i];
        if c.crazy > 0.0 && c.map == g.map && c.pos.distance(g.pos) < 0.8 * T {
            g.scare_t = 0.7;
            g.scare_face = c.color;
            g.fear = (g.fear + 0.45).min(1.0);
            g.vest.play(haptics::Effect::Chill);
            g.chars[i].crazy = 0.0;
            g.map = 0;
            g.pos = vec2(3.5, 6.0) * T;
            g.log.push(format!("{} caught you. You wake up in the cozy house.", g.chars[i].name));
        }
    }
    if g.scare_t > 0.0 { g.scare_t -= dt; }

    // Characters talking to each other (only if a local AI is around, to keep it free and offline).
    g.next_chatter -= dt;
    if g.next_chatter <= 0.0 && !g.infection {
        g.next_chatter = rand::gen_range(35.0, 60.0);
        if !g.brains.ollama_models.is_empty() {
            let here: Vec<usize> = (0..g.chars.len()).filter(|&i| g.chars[i].map == g.map && !g.chars[i].thinking).collect();
            if here.len() >= 2 {
                let a = here[rand::gen_range(0, here.len())];
                let b = *here.iter().find(|&&x| x != a).unwrap();
                let line = format!("(say one short line to {}, who is nearby; the player can hear you)", g.chars[b].name);
                g.say(a, Some(line), "...");
            }
        }
    }

    // Meters.
    let m = &MAPS[g.map];
    let dark = if g.on(0) { m.dark } else { 0.1 };
    let crazy_near = g.chars.iter().filter(|c| c.crazy > 0.0 && c.map == g.map).map(|c| 1.0 - (c.pos.distance(g.pos) / (10.0 * T)).min(1.0)).fold(0.0f32, f32::max);
    let water_near = g.on(1) && (g.map == 1 || g.map == 2);
    let fear_target = (dark * 0.45 + crazy_near * 0.7 + if water_near { 0.1 } else { 0.0 }).min(1.0) * if g.courage == Courage::Brave { 0.4 } else { 1.0 };
    g.fear += (fear_target - g.fear) * dt * 0.4;
    let closest = g.chars.iter().filter(|c| c.map == g.map).map(|c| c.pos.distance(g.pos)).fold(f32::MAX, f32::min);
    let disc_target: f32 = if crowds_on && closest < 1.4 * T { 0.7 } else { 0.0 } + if g.on(2) && near_doll(g) { 0.35 } else { 0.0 };
    g.discomfort += (disc_target.min(1.0) - g.discomfort) * dt * 0.5;
    let lim_target = if g.on(5) { m.liminal } else { m.liminal * 0.3 };
    g.liminality += (lim_target - g.liminality) * dt * 0.3;
    // Heartbeat on the vest when you're very scared.
    g.beat_t -= dt;
    if g.fear > 0.7 && g.beat_t <= 0.0 {
        g.vest.play(haptics::Effect::Heartbeat);
        g.beat_t = 1.1;
    }
    // A tense warm pulse when something feels very wrong.
    if g.discomfort > 0.6 && g.beat_t <= 0.0 {
        g.vest.play(haptics::Effect::Tense);
        g.beat_t = 2.5;
    }
    if g.log.len() > 4 { g.log.remove(0); }
}

fn near_doll(g: &Game) -> bool {
    let (px, py) = ((g.pos.x / T) as i32, (g.pos.y / T) as i32);
    (-3..=3).any(|dy| (-3..=3).any(|dx| tile(g.map, px + dx, py + dy) == 'd'))
}

fn nearest(g: &Game, within: f32) -> Option<usize> {
    (0..g.chars.len()).filter(|&i| g.chars[i].map == g.map).min_by(|&a, &b| {
        g.chars[a].pos.distance(g.pos).partial_cmp(&g.chars[b].pos.distance(g.pos)).unwrap()
    }).filter(|&i| g.chars[i].pos.distance(g.pos) < within * T)
}

fn draw_world(g: &Game, light: &Texture2D) {
    let m = &MAPS[g.map];
    let shake = if g.fear > 0.75 { rand::gen_range(-1.5, 1.5) } else { 0.0 };
    // Tiles.
    for y in 0..MH {
        for x in 0..MW {
            let (px, py) = (x as f32 * T + shake, y as f32 * T + shake);
            let c = tile(g.map, x as i32, y as i32);
            let col = match c {
                '#' => Color::new(0.22, 0.2, 0.24, 1.0),
                'b' => Color::new(0.42, 0.3, 0.22, 1.0),
                '~' if !g.on(1) => Color::new(0.6, 0.62, 0.6, 1.0), // Water phobia off: no water at all (calm tiles)
                '~' if g.map == 2 => Color::new(0.55, 0.78, 0.8, 1.0), // the drained pool
                '~' => Color::new(0.05, 0.12 + 0.03 * (get_time() as f32 + x as f32 * 0.3).sin(), 0.25, 1.0),
                ',' => m.floor,
                'A' | 'B' | 'C' => Color::new(0.9, 0.85, 0.5, 1.0),
                _ => m.floor,
            };
            draw_rectangle(px, py, T, T, col);
            if g.map == 2 && c != '#' && (x + y) % 2 == 0 { draw_rectangle(px, py, T, T, Color::new(1.0, 1.0, 1.0, 0.05)); }
            if c == 'L' { draw_circle(px + T / 2.0, py + T / 2.0, 7.0, Color::new(1.0, 0.85, 0.5, 1.0)); }
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
        if c.kind { draw_circle_lines(c.pos.x + shake, c.pos.y + shake, 16.0, 2.0, Color::new(1.0, 0.95, 0.6, 0.8)); }
        let body = if c.infected { Color::new(c.color.r * 0.45, c.color.g * 0.6 + 0.2, c.color.b * 0.45, 1.0) } else { c.color };
        draw_circle(c.pos.x + shake, c.pos.y + shake, 12.0, body);
        let eye = if c.infected { if c.stun > 0.0 { GRAY } else { Color::new(0.3, 1.0, 0.2, 1.0) } } else if c.crazy > 0.0 { RED } else { BLACK };
        let look = (g.pos - c.pos).normalize_or_zero() * 2.0;
        draw_circle(c.pos.x - 4.0 + look.x, c.pos.y - 2.0 + look.y, 2.4, eye);
        draw_circle(c.pos.x + 4.0 + look.x, c.pos.y - 2.0 + look.y, 2.4, eye);
        draw_text(c.name, c.pos.x - measure_text(c.name, None, 16, 1.0).width / 2.0, c.pos.y + 26.0, 16.0, WHITE);
    }
    // You (see-through while hiding).
    let you = if g.hidden { Color::new(0.95, 0.95, 0.98, 0.35) } else { Color::new(0.95, 0.95, 0.98, 1.0) };
    draw_circle(g.pos.x, g.pos.y, 11.0, you);
    draw_circle_lines(g.pos.x, g.pos.y, 11.0, 2.0, BLACK);
    // The tornado: a grey spinning funnel (a dashed circle where it will touch down during the warning).
    if let Some(t) = &g.tornado {
        if t.warn > 0.0 {
            let x = t.pos.x.clamp(T, (MW as f32 - 1.0) * T);
            for k in 0..12 {
                let a = k as f32 / 12.0 * std::f32::consts::TAU + get_time() as f32;
                draw_circle(x + a.cos() * 1.6 * T, t.pos.y + a.sin() * 1.6 * T, 3.0, Color::new(0.8, 0.8, 0.85, 0.6));
            }
        } else {
            for k in 0..6 {
                let r = (1.6 - k as f32 * 0.22) * T;
                let a = get_time() as f32 * (4.0 + k as f32) + k as f32;
                draw_circle_lines(t.pos.x + a.cos() * 4.0, t.pos.y + a.sin() * 4.0, r, 3.0, Color::new(0.55, 0.55, 0.6, 0.75));
            }
        }
    }

    // Darkness: the room is dark except a soft circle of light around you (smaller when you're scared).
    let dark = if g.on(0) { m.dark } else { 0.1 };
    let radius = (7.5 - g.fear * 3.0) * T;
    let a = (dark + g.fear * 0.25).min(0.97);
    let (sw, sh) = (MW as f32 * T, MH as f32 * T);
    let (lx, ly) = (g.pos.x - radius, g.pos.y - radius);
    let dim = Color::new(0.0, 0.0, 0.02, a);
    draw_texture_ex(light, lx, ly, Color::new(1.0, 1.0, 1.0, a), DrawTextureParams { dest_size: Some(vec2(radius * 2.0, radius * 2.0)), ..Default::default() });
    // Fill everything outside the light's square, without overlapping (overlaps showed up as darker bands).
    let (top, bottom) = (ly.max(0.0), (ly + radius * 2.0).min(sh));
    draw_rectangle(0.0, 0.0, sw, top, dim);
    draw_rectangle(0.0, bottom, sw, sh - bottom, dim);
    draw_rectangle(0.0, top, lx.max(0.0), bottom - top, dim);
    draw_rectangle(lx + radius * 2.0, top, (sw - (lx + radius * 2.0)).max(0.0), bottom - top, dim);
    // Infection mode: your noise as a ring (they hear you inside it), and the world turning green as you turn.
    if g.infection {
        if g.noise > 0.05 && !g.lost_control {
            draw_circle_lines(g.pos.x, g.pos.y, g.noise * 6.0 * T, 2.0, Color::new(1.0, 1.0, 1.0, 0.25 * g.noise));
        }
        if g.sneaking && !g.lost_control { draw_text("sneaking", g.pos.x - 26.0, g.pos.y + 28.0, 14.0, Color::new(0.7, 0.8, 1.0, 0.9)); }
        if g.inf > 0.0 { draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.1, 0.8, 0.1, g.inf * 0.18)); }
        if g.lost_control {
            draw_rectangle(0.0, 0.0, sw, 44.0, Color::new(0.0, 0.0, 0.0, 0.7));
            draw_text("YOU ARE INFECTED: you can see, but you can't move. It's walking toward the Claudes.", 16.0, 28.0, 20.0, Color::new(0.4, 1.0, 0.35, 1.0));
        }
        for c in g.chars.iter().filter(|c| c.map == g.map && c.claude && !c.infected && c.infect > 0.0) {
            draw_rectangle(c.pos.x - 14.0, c.pos.y - 22.0, 28.0 * c.infect, 4.0, Color::new(0.35, 0.95, 0.3, 1.0));
        }
    }
    // Liminality: everything goes a little grey and too quiet.
    if g.liminality > 0.3 { draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.6, 0.65, 0.7, (g.liminality - 0.3) * 0.25)); }
    // Discomfort: a faint wrong-coloured tint.
    if g.discomfort > 0.2 { draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.6, 0.0, 0.3, g.discomfort * 0.12)); }

    // Speech bubbles (drawn over the dark so you can read them).
    for c in g.chars.iter().filter(|c| c.map == g.map && c.bubble_t > 0.0) {
        bubble(&c.bubble, &c.via, c.pos.x, c.pos.y);
    }
    if let Some(i) = nearest(g, 2.2) {
        if g.screen == Screen::Play {
            draw_text(&format!("E: talk to {}", g.chars[i].name), g.pos.x - 40.0, g.pos.y - 22.0, 16.0, YELLOW);
        }
    }

    // Jumpscare: a face fills the screen for a moment (no flashing lights: one quick fade).
    if g.scare_t > 0.0 {
        let k = (g.scare_t / 0.7).min(1.0);
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.85 * k));
        draw_circle(sw / 2.0, sh / 2.0, 220.0 * (1.2 - k * 0.2), Color::new(g.scare_face.r, g.scare_face.g, g.scare_face.b, k));
        draw_circle(sw / 2.0 - 70.0, sh / 2.0 - 30.0, 26.0, Color::new(1.0, 0.0, 0.0, k));
        draw_circle(sw / 2.0 + 70.0, sh / 2.0 - 30.0, 26.0, Color::new(1.0, 0.0, 0.0, k));
    }

    // HUD.
    let y0 = sh;
    draw_rectangle(0.0, y0, sw, HUD, Color::new(0.05, 0.05, 0.07, 1.0));
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
        if g.tornado.as_ref().is_some_and(|t| t.warn > 0.0) && (get_time() * 2.0) as i32 % 2 == 0 {
            draw_text("TORNADO WARNING", sw / 2.0 - 120.0, 40.0, 32.0, Color::new(1.0, 0.85, 0.2, 1.0));
        }
    }
    let courage = match g.courage { Courage::Brave => "Brave", Courage::Normal => "Normal", Courage::Phobias => "Phobias" };
    draw_text(&format!("{}  ·  {}  ·  {}", m.name, g.time_text(), courage), 320.0, y0 + 24.0, 18.0, Color::new(0.85, 0.85, 0.9, 1.0));
    for (i, l) in g.log.iter().rev().take(2).enumerate() {
        draw_text(l, 320.0, y0 + 48.0 + i as f32 * 20.0, 16.0, Color::new(0.65, 0.68, 0.75, 1.0));
    }
    draw_text(if g.infection { "WASD move · Shift sneak · H hide · E talk · P PANIC · Esc" } else { "WASD move · E talk · P PANIC (everything off) · Esc menu" },
        sw - 380.0, y0 + 72.0, 14.0, Color::new(0.5, 0.52, 0.6, 1.0));
    if g.vest.enabled { draw_text("vest ON", sw - 70.0, y0 + 24.0, 16.0, Color::new(0.4, 0.9, 0.5, 1.0)); }
}

fn draw_title(g: &Game) {
    draw_text("HOLLOW HOURS", 60.0, 110.0, 64.0, Color::new(0.85, 0.3, 0.35, 1.0));
    let lines = [
        "A small horror game where the characters are real AIs.".to_string(),
        format!("Claude app: {}", if ai::claude_cli().is_some() { "found, so the Claude characters talk through it (real Claude)" } else { "not found" }),
        format!("Brains: {}", if g.brains.ollama_models.is_empty() { "no local AI found (start Ollama), scripted lines only".into() } else { format!("local models {}", g.brains.ollama_models.join(", ")) }),
        "Cloud AIs join if you set their keys: ANTHROPIC_API_KEY, OPENAI_API_KEY, GEMINI_API_KEY.".into(),
        String::new(),
        "How brave are you?".into(),
        "  1  Brave: no fear, no chasing, friendly AIs".into(),
        "  2  Normal".into(),
        "  3  Many phobias: pick what scares you (and switch off what you can't handle)".into(),
        "  4  INFECTION: the AIs are infected, you're the last human. Hide, eat, drink, dodge tornadoes.".into(),
        String::new(),
        format!("Haptic vest: {}", if !g.vest.found { "none found".to_string() } else if g.vest.enabled { format!("ON at {}% (V off, [ ] strength)", (g.vest.strength * 100.0) as i32) } else { "found, OFF (V to turn on)".into() }),
        "Vest effects are mild and capped. Don't use them if you have a heart condition.".into(),
        "No flashing lights. P = panic at any time: everything stops.".into(),
        format!("Outside the window: {}   {}", match g.meta.allowed { Some(true) => "ALLOWED (files, Terminal, window switching)", Some(false) => "off", None => "not asked" },
            if g.meta.created.is_empty() { "M to change".to_string() } else { format!("M to change, C removes its {} files", g.meta.created.len()) }),
    ];
    for (i, l) in lines.iter().enumerate() {
        draw_text(l, 60.0, 170.0 + i as f32 * 28.0, 22.0, Color::new(0.82, 0.84, 0.9, 1.0));
    }
    // Who's who: our own characters, each only inspired by a kind of AI.
    let y = 170.0 + lines.len() as f32 * 28.0 + 10.0;
    draw_text("Who lives here:", 60.0, y, 20.0, WHITE);
    for (i, c) in g.chars.iter().enumerate() {
        let (col, row) = (i % 2, i / 2);
        draw_circle(70.0 + col as f32 * 440.0, y + 18.0 + row as f32 * 22.0, 7.0, c.color);
        draw_text(&format!("{}: {}", c.name, c.inspired), 84.0 + col as f32 * 440.0, y + 24.0 + row as f32 * 22.0, 17.0, Color::new(0.7, 0.72, 0.8, 1.0));
    }
}

fn draw_phobias(g: &Game) {
    draw_text("Your phobias", 60.0, 100.0, 48.0, WHITE);
    draw_text("ON = the world uses it to scare you. OFF = it never happens.", 60.0, 140.0, 22.0, Color::new(0.7, 0.72, 0.8, 1.0));
    for (i, (name, what)) in PHOBIAS.iter().enumerate() {
        let on = g.phobia[i];
        let col = if on { Color::new(0.95, 0.45, 0.45, 1.0) } else { Color::new(0.5, 0.75, 0.55, 1.0) };
        draw_text(&format!("{}  {:<14} {}   ({})", i + 1, name, if on { "ON " } else { "OFF" }, what), 60.0, 200.0 + i as f32 * 34.0, 24.0, col);
    }
    draw_text("1-6 toggle · Enter start · Esc back", 60.0, 440.0, 22.0, Color::new(0.7, 0.72, 0.8, 1.0));
}
