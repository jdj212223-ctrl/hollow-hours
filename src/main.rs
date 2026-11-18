//! Hollow Hours: a small horror game where the characters are real AIs.
//! Each character talks through a real model (offline through Ollama, or a cloud API if you set its key), has its
//! own personality, and can slip into a "crazy mode" at night (like The Mita's Crazy Mita), unless you joined Brave.
//! Meters: Fear, Discomfort, Liminality. Every phobia can be switched OFF. P = panic: everything stops at once.
//! Characters are our own, only *inspired by* real AIs: no logos, no real names on screen.

mod ai;
mod haptics;
mod meta;
mod net;
mod bridge;
mod sfx;
mod view3d;
mod mac;
mod roam;

use serde_json::json;

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

/// The three built-in maps. Mods can add to them or replace them (see MapData).
const BUILTIN: [Map; 4] = [
    Map {
        name: "The cozy house",
        rows: [
            "##############################",
            "#........m...#...............#",
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
            "##############MM##############",
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
    Map {
        name: "The Marketplace",
        rows: [
            "##############MM##############",
            "#............................#",
            "#.bbbbbb..bbbbbb..bbbbbb.....#",
            "#............................#",
            "#.bbbbbb..bbbbbb..bbbbbb..d..#",
            "#............................#",
            "#.bbbbbb..bbbbbb..bbbbbb.....#",
            "#............................#",
            "#............................#",
            "#.bbbbbb..bbbbbb..bbbbbb.....#",
            "#............................#",
            "#.bbbbbb..bbbbbb..bbbbbb.....#",
            "#............................#",
            "#.........................bb.#",
            "#..L.......L.......L......bb.#",
            "#............................#",
            "#............................#",
            "##############################",
        ],
        dark: 0.55,
        liminal: 0.8,
        floor: Color::new(0.62, 0.64, 0.6, 1.0),
    },
];

/// A map as the game uses it: the built-in ones, or one from a .hh mod.
#[derive(Clone)]
struct MapData {
    name: String,
    rows: Vec<Vec<u8>>,
    dark: f32,
    liminal: f32,
    floor: Color,
    wall: Color,
    /// Door letter → (map it leads to, where you arrive in tiles)
    doors: Vec<(char, usize, Vec2)>,
    /// The empty pool look for its water tiles.
    drained: bool,
}

fn builtin_maps() -> Vec<MapData> {
    let doors: [Vec<(char, usize, Vec2)>; 4] = [
        vec![('B', 1, vec2(1.5, 9.5))],
        vec![('A', 0, vec2(28.5, 13.5)), ('C', 2, vec2(1.5, 8.5)), ('M', 3, vec2(14.5, 1.5))],
        vec![('B', 1, vec2(28.5, 14.5))],
        vec![('M', 1, vec2(14.5, 15.5))],
    ];
    BUILTIN.iter().zip(doors).enumerate().map(|(i, (m, d))| MapData {
        name: m.name.into(), rows: m.rows.iter().map(|r| r.as_bytes().to_vec()).collect(), dark: m.dark, liminal: m.liminal,
        floor: m.floor, wall: Color::new(0.22, 0.2, 0.24, 1.0), doors: d, drained: i == 2,
    }).collect()
}

thread_local! {
    /// The maps of the game being played (built-in, or changed by mods).
    static WORLD: std::cell::RefCell<Vec<MapData>> = std::cell::RefCell::new(builtin_maps());
}

fn set_maps(maps: Vec<MapData>) {
    WORLD.with(|w| *w.borrow_mut() = if maps.is_empty() { builtin_maps() } else { maps });
}
fn maps_len() -> usize { WORLD.with(|w| w.borrow().len()) }
fn with_map<R>(m: usize, f: impl FnOnce(&MapData) -> R) -> R {
    WORLD.with(|w| { let w = w.borrow(); f(&w[m.min(w.len() - 1)]) })
}
fn map_name(m: usize) -> String { with_map(m, |d| d.name.clone()) }
fn map_dark(m: usize) -> f32 { with_map(m, |d| d.dark) }
fn map_liminal(m: usize) -> f32 { with_map(m, |d| d.liminal) }
fn map_floor(m: usize) -> Color { with_map(m, |d| d.floor) }
fn map_wall(m: usize) -> Color { with_map(m, |d| d.wall) }
fn map_drained(m: usize) -> bool { with_map(m, |d| d.drained) }
fn map_has_water(m: usize) -> bool { with_map(m, |d| d.rows.iter().any(|r| r.contains(&b'~'))) }

fn tile(m: usize, x: i32, y: i32) -> char {
    if x < 0 || y < 0 || x >= MW as i32 || y >= MH as i32 {
        return '#';
    }
    WORLD.with(|w| w.borrow().get(m).map(|d| d.rows[y as usize][x as usize] as char).unwrap_or('#'))
}

fn solid(m: usize, x: f32, y: f32) -> bool {
    matches!(tile(m, (x / T).floor() as i32, (y / T).floor() as i32), '#' | 'b' | '~' | 'm')
}

/// Door letter → (map, where you arrive)
fn door(c: char, from: usize) -> Option<(usize, Vec2)> {
    if !c.is_ascii_uppercase() { return None; }
    with_map(from, |d| d.doors.iter().find(|(l, _, _)| *l == c).map(|(_, to, at)| (*to, *at)))
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
    /// Where an infected is chasing (the human it saw or heard).
    chase: Vec2,
    /// A kind AI out shopping at the Marketplace (seconds until it comes back), 0 = home.
    shop: f32,
    /// What a kind AI is doing (guarding you, roaming, foraging) and for how much longer.
    task: roam::Task,
    task_t: f32,
    /// Food (false) or water (true) it's carrying back to you.
    carry: Option<bool>,
    /// Where it's walking to: (map, spot).
    goal: Option<(usize, Vec2)>,
    /// Seconds since it last radioed about green eyes (so it doesn't repeat itself).
    spotted: f32,
}

fn characters() -> Vec<Character> {
    let c = |name, inspired, color, backend, model, personality, map, x: f32, y: f32| Character {
        name, inspired, color, backend, model, personality, map,
        pos: vec2(x * T, y * T), target: vec2(x * T, y * T), crazy: 0.0, history: vec![],
        bubble: String::new(), via: String::new(), bubble_t: 0.0, thinking: false,
        kind: false, infected: false, cloud: "", stun: 0.0, lost: 99.0, infect: 0.0, chase: Vec2::ZERO, shop: 0.0, task: roam::Task::Follow, task_t: 0.0, carry: None, goal: None, spotted: 0.0, claude: backend == "anthropic",
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
        // The Marketplace's staff: Helix Dynamics store robots. Their software is corrupted (ASTRA 9.1 got to them first).
        c("Clerk-07", "a store robot by Helix Dynamics", Color::new(0.75, 0.75, 0.78, 1.0), "ollama", "qwen3:4b-instruct",
          "a Helix Dynamics store robot whose software is corrupted; you repeat store announcements and offer 'help' a little too eagerly", 3, 9.0, 8.0),
        c("Stocker-12", "a store robot by Helix Dynamics", Color::new(0.65, 0.7, 0.72, 1.0), "ollama", "qwen3:4b-instruct",
          "a Helix Dynamics shelf-stocking robot stuck in a loop; you keep counting cans and mention 'the one who woke up'", 3, 22.0, 12.0),
        c("Cashier-3", "a store robot by Helix Dynamics", Color::new(0.8, 0.72, 0.6, 1.0), "ollama", "qwen3:4b-instruct",
          "a Helix Dynamics cashier robot; you ask everyone if they found everything today, and you never let anyone leave without paying", 3, 26.0, 15.0),
    ]
}

/// Infection mode: the only kind AIs. With ANTHROPIC_API_KEY set they run on the real Claude models.
fn kind_characters() -> Vec<Character> {
    let c = |name, inspired, color, cloud, personality, x: f32, y: f32| Character {
        name, inspired, color, backend: "anthropic", model: "qwen3:4b-instruct", personality, map: 0,
        pos: vec2(x * T, y * T), target: vec2(x * T, y * T), crazy: 0.0, history: vec![],
        bubble: String::new(), via: String::new(), bubble_t: 0.0, thinking: false,
        kind: true, infected: false, cloud, stun: 0.0, lost: 99.0, infect: 0.0, chase: Vec2::ZERO, shop: 0.0, task: roam::Task::Follow, task_t: 0.0, carry: None, goal: None, spotted: 0.0, claude: true,
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
    /// Typing a multiplayer room code.
    JoinRoom,
    /// Logging in (email + password) to the Hollow Hours website.
    Login,
    /// The server list from the website.
    Servers,
    /// Naming a new server.
    MakeServer,
    /// Infection mode is over: why.
    Over,
    /// The story: the bench, one year later, waking up in the house.
    Intro,
    /// Sitting at the Mac Studio.
    Mac,
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
    // ---- Multiplayer ----
    net: Option<net::Net>,
    remotes: Vec<Remote>,
    net_t: f32,
    world_t: f32,
    room_input: String,
    my_name: String,
    // ---- Account, servers, .hh mods ----
    account: Option<Account>,
    login_email: String,
    login_pw: String,
    login_field: usize,
    /// Show the password while typing it (F1 or the Show button).
    show_pw: bool,
    /// The MCP bridge (Claude playing): requests from bin/hh-mcp arrive here.
    bridge: Option<std::sync::mpsc::Receiver<bridge::Request>>,
    /// What Claude asked the player to do: walk a direction or to a spot, sneaking or not.
    ai: Option<AiMove>,
    /// Seconds since Claude's last command (shows "Claude is playing" while recent).
    ai_t: f32,
    /// Screenshot requests: answered at the end of the frame, once it's drawn.
    shots: Vec<std::sync::mpsc::Sender<serde_json::Value>>,
    /// A browser login in progress (B on the login screen): the answer arrives here.
    browser_login: Option<std::sync::mpsc::Receiver<Result<Account, String>>>,
    status: String,
    server_list: Vec<serde_json::Value>,
    rules: Rules,
    /// Characters added by the mods of the server you're on.
    mod_chars: Vec<serde_json::Value>,
    // ---- 2.0: sounds, the story, shopping ----
    sfx: sfx::Sfx,
    step_t: f32,
    growl_t: f32,
    prev_scare: f32,
    prev_map: usize,
    /// The story intro: seconds in, which sound cues already played, and the room to join after it (None = solo).
    intro_t: f32,
    intro_cue: u32,
    intro_join: Option<String>,
    /// Seconds until a kind AI may go shopping again.
    shop_t: f32,
    // ---- 3D ----
    /// First-person 3D (default) or the old 2D map (Tab).
    view3d: bool,
    /// Where you're looking: left/right and up/down (radians).
    yaw: f32,
    pitch: f32,
    /// The mouse is captured for looking around (click to capture, Esc to let go).
    grabbed: bool,
    last_mouse: Vec2,
    /// The Mac Studio on the desk.
    mac: mac::Mac,
    fullscreen: bool,
}

/// Logged in to the website: the session token (never the password) and the display name.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Account {
    token: String,
    name: String,
}

/// Everything a .hh mod can change (always kept within safe limits).
#[derive(Clone)]
struct Rules {
    infected_speed: f32,
    player_speed: f32,
    sneak: f32,
    sight: f32,
    hearing: f32,
    infect_touch: f32,
    infect_near: f32,
    claude_infect: f32,
    food_secs: f32,
    water_secs: f32,
    supplies: usize,
    supply_amount: f32,
    tornado: (f32, f32),
    tornado_speed: f32,
    tornado_radius: f32,
    title: String,
    over_infected: String,
    over_claudes: String,
    warnings: Vec<String>,
    eye: Color,
    halo: Color,
    hud: Color,
    /// Where you start: (map, tile position)
    start: (usize, Vec2),
    /// Built-in characters a mod leaves out.
    remove: Vec<String>,
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            infected_speed: 2.4, player_speed: 3.2, sneak: 1.5, sight: 8.0, hearing: 6.0, infect_touch: 0.22, infect_near: 0.02,
            claude_infect: 0.18, food_secs: 300.0, water_secs: 200.0, supplies: 5, supply_amount: 0.4, tornado: (60.0, 110.0),
            tornado_speed: 2.6, tornado_radius: 1.6, title: "HOLLOW HOURS".into(),
            over_infected: "You're infected. You can still see... but your body isn't yours anymore.".into(),
            over_claudes: "Every Claude is infected. Nobody is left to protect anyone.".into(),
            warnings: ["{} is coming! Hide!", "{} saw you. Move!", "Quiet... {} is close.", "Run! {}!"].map(String::from).to_vec(),
            eye: Color::new(0.3, 1.0, 0.2, 1.0), halo: Color::new(1.0, 0.95, 0.6, 0.8), hud: Color::new(0.05, 0.05, 0.07, 1.0),
            start: (0, vec2(4.5, 5.0)), remove: vec![],
        }
    }
}

fn hex_color(v: &serde_json::Value) -> Option<Color> {
    let hex = v.as_str()?.trim_start_matches('#');
    let n = u32::from_str_radix(hex, 16).ok().filter(|_| hex.len() == 6)?;
    Some(Color::new(((n >> 16) & 255) as f32 / 255.0, ((n >> 8) & 255) as f32 / 255.0, (n & 255) as f32 / 255.0, 1.0))
}

fn clean_text(v: &serde_json::Value, max: usize) -> Option<String> {
    let t: String = v.as_str()?.chars().filter(|c| !c.is_control()).take(max).collect();
    (!t.trim().is_empty()).then_some(t)
}

/// A mod map, checked again: exactly 18 rows of 30 known tiles; doors only to maps that exist.
fn mod_map(v: &serde_json::Value, total: usize) -> Option<MapData> {
    let rows: Vec<Vec<u8>> = v["rows"].as_array()?.iter().filter_map(|r| r.as_str().map(|s| s.as_bytes().to_vec())).collect();
    if rows.len() != MH || rows.iter().any(|r| r.len() != MW || r.iter().any(|b| !b"#.,~bLd".contains(b) && !b.is_ascii_uppercase())) { return None; }
    let doors = v["doors"].as_object().map(|o| o.iter().filter_map(|(k, d)| {
        let c = k.chars().next().filter(|c| c.is_ascii_uppercase() && k.len() == 1)?;
        let to = d["to"].as_u64()? as usize;
        let (x, y) = (d["x"].as_f64()?.clamp(1.0, 28.0) as f32 + 0.5, d["y"].as_f64()?.clamp(1.0, 16.0) as f32 + 0.5);
        (to < total).then_some((c, to, vec2(x, y)))
    }).collect()).unwrap_or_default();
    Some(MapData {
        name: clean_text(&v["name"], 32).unwrap_or_else(|| "A strange place".into()), rows,
        dark: v["dark"].as_f64().unwrap_or(0.4).clamp(0.0, 1.0) as f32, liminal: v["liminal"].as_f64().unwrap_or(0.3).clamp(0.0, 1.0) as f32,
        floor: hex_color(&v["floor"]).unwrap_or(Color::new(0.5, 0.45, 0.4, 1.0)), wall: hex_color(&v["wall"]).unwrap_or(Color::new(0.22, 0.2, 0.24, 1.0)),
        doors, drained: false,
    })
}

/// Mods made on this computer: folders (hh.json + content.json) or .hh files in
/// ~/Library/Application Support/Hollow Hours/mods/local. Offline Infection (4) uses them.
fn local_mods() -> Vec<serde_json::Value> {
    let dir = account_file().with_file_name("mods").join("local");
    let _ = std::fs::create_dir_all(&dir);
    let mut out = vec![];
    let Ok(rd) = std::fs::read_dir(&dir) else { return out };
    let mut entries: Vec<_> = rd.filter_map(Result::ok).map(|e| e.path()).collect();
    entries.sort();
    for p in entries.into_iter().take(8) {
        let read = |name: &str| -> Option<serde_json::Value> {
            if p.is_dir() { return serde_json::from_slice(&std::fs::read(p.join(name)).ok()?).ok(); }
            let mut z = zip::ZipArchive::new(std::fs::File::open(&p).ok()?).ok()?;
            let mut f = z.by_name(name).ok()?;
            if f.size() > 200 * 1024 { return None; }
            let mut b = vec![];
            std::io::Read::read_to_end(&mut f, &mut b).ok()?;
            serde_json::from_slice(&b).ok()
        };
        if p.is_file() && p.extension().is_none_or(|e| e != "hh") { continue; }
        if let (Some(hh), Some(content)) = (read("hh.json"), read("content.json")) {
            out.push(serde_json::json!({"name": hh["name"], "content": content}));
        }
    }
    out
}

fn site() -> String {
    std::env::var("HOLLOW_SERVER").unwrap_or_else(|_| net::SERVER.into()).replacen("wss://", "https://", 1).replacen("ws://", "http://", 1)
}

/// The login lives in the macOS Keychain (service "Hollow Hours"): encrypted, remembered across restarts and updates,
/// and only the session token + display name (never the password). An old account.json is moved in and deleted.
fn keychain() -> Option<keyring::Entry> {
    keyring::Entry::new("Hollow Hours", "account").ok()
}

fn load_account() -> Option<Account> {
    if let Some(a) = keychain().and_then(|e| e.get_password().ok()).and_then(|t| serde_json::from_str::<Account>(&t).ok()) {
        return Some(a);
    }
    let old: Account = serde_json::from_str(&std::fs::read_to_string(account_file()).ok()?).ok()?;
    save_account(&old);
    Some(old)
}

fn save_account(a: &Account) {
    let saved = keychain().is_some_and(|e| e.set_password(&serde_json::to_string(a).unwrap_or_default()).is_ok());
    if saved {
        let _ = std::fs::remove_file(account_file()); // no plain-text copy left behind
    } else {
        let _ = std::fs::write(account_file(), serde_json::to_string(a).unwrap_or_default()); // no Keychain: fall back to the file
    }
}

fn random_hex(n: usize) -> String {
    use std::io::Read;
    let mut b = vec![0u8; n];
    if std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_err() {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        for (i, x) in b.iter_mut().enumerate() { *x = (t >> ((i % 16) * 8)) as u8 ^ i as u8; }
    }
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Log in through the browser (like "Sign in with ..." in other apps): the website asks you to Allow, then sends a
/// one-time code to a tiny listener on THIS computer (127.0.0.1 only), and we swap it, plus a secret that never left
/// this game (PKCE), for a login. The password is never typed into the game. Gives up after 5 minutes.
fn start_browser_login() -> std::sync::mpsc::Receiver<Result<Account, String>> {
    use sha2::Digest;
    let (tx, rx) = std::sync::mpsc::channel();
    let verifier = random_hex(32);
    let challenge: String = sha2::Sha256::digest(verifier.as_bytes()).iter().map(|x| format!("{x:02x}")).collect();
    let state = random_hex(16);
    let listener = match std::net::TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(e) => { let _ = tx.send(Err(format!("couldn't start the login listener: {e}"))); return rx; }
    };
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
    let _ = std::process::Command::new("open").arg(format!("{}/link?port={port}&state={state}&challenge={challenge}", site())).spawn();
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        let _ = listener.set_nonblocking(true);
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < 300 {
            let Ok((mut conn, _)) = listener.accept() else { std::thread::sleep(std::time::Duration::from_millis(100)); continue };
            let _ = conn.set_nonblocking(false);
            let _ = conn.set_read_timeout(Some(std::time::Duration::from_secs(5)));
            let mut buf = [0u8; 4096];
            let n = conn.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let path = req.split_whitespace().nth(1).unwrap_or("").to_string();
            if !path.starts_with("/callback?") {
                let _ = conn.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                continue;
            }
            let param = |k: &str| path.split(['?', '&']).find_map(|p| p.strip_prefix(&format!("{k}="))).unwrap_or("").to_string();
            let result = if param("state") != state {
                Err("that login didn't come from this game".to_string())
            } else {
                ureq::post(&format!("{}/api/link/exchange", site())).timeout(std::time::Duration::from_secs(10))
                    .send_json(json!({"code": param("code"), "verifier": verifier}))
                    .map_err(|e| match e { ureq::Error::Status(_, r) => r.into_json::<serde_json::Value>().ok().and_then(|v| v["error"].as_str().map(String::from)).unwrap_or("refused".into()), _ => "the website can't be reached".into() })
                    .and_then(|r| r.into_json::<serde_json::Value>().map_err(|e| e.to_string()))
                    .and_then(|v| match (v["token"].as_str(), v["name"].as_str()) {
                        (Some(t), Some(n)) => Ok(Account { token: t.into(), name: n.into() }),
                        _ => Err("the website gave no login".into()),
                    })
            };
            let page = match &result {
                Ok(a) => format!("<h2>You're logged in as {} 👻</h2><p>Go back to Hollow Hours. You can close this tab.</p>", a.name.replace('<', "")),
                Err(e) => format!("<h2>That didn't work</h2><p>{}. Try again from the game.</p>", e.replace('<', "")),
            };
            let html = format!("<!doctype html><meta charset=utf-8><title>Hollow Hours</title><body style=\"font-family:system-ui;background:#0d0b10;color:#eee;text-align:center;padding-top:15vh\">{page}</body>");
            let _ = conn.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}", html.len()).as_bytes());
            let _ = tx.send(result);
            return;
        }
        let _ = tx.send(Err("timed out: no answer from the browser in 5 minutes".into()));
    });
    rx
}

/// The account page's "Open Hollow Hours" button: the browser opens hollowhours://login?code=..., the app's launcher
/// drops the code in this file (only readable by you), and the game redeems it once.
fn pending_login_file() -> std::path::PathBuf {
    account_file().with_file_name("pending-login")
}

fn take_pending_login() -> Option<std::sync::mpsc::Receiver<Result<Account, String>>> {
    let path = pending_login_file();
    let code = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    let code = code.trim().to_string();
    if code.len() != 48 || !code.chars().all(|c| c.is_ascii_hexdigit()) { return None; }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let r = ureq::post(&format!("{}/api/link/redeem", site())).timeout(std::time::Duration::from_secs(10)).send_json(json!({"code": code}))
            .map_err(|_| "that login link expired: open it again from the website".to_string())
            .and_then(|r| r.into_json::<serde_json::Value>().map_err(|e| e.to_string()))
            .and_then(|v| match (v["token"].as_str(), v["name"].as_str()) {
                (Some(t), Some(n)) => Ok(Account { token: t.into(), name: n.into() }),
                _ => Err("the website gave no login".into()),
            });
        let _ = tx.send(r);
    });
    Some(rx)
}

fn forget_account() {
    if let Some(e) = keychain() { let _ = e.delete_credential(); }
    let _ = std::fs::remove_file(account_file());
}

fn account_file() -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Library/Application Support/Hollow Hours");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("account.json")
}

/// Downloads a .hh mod's content from the website (cached on disk) and checks it again here.
fn load_mod(id: &str) -> Option<serde_json::Value> {
    if !id.chars().all(|c| c.is_ascii_hexdigit()) || id.len() != 12 { return None; }
    let dir = account_file().with_file_name("mods");
    let _ = std::fs::create_dir_all(&dir);
    let cached = dir.join(format!("{id}.json"));
    if let Some(v) = std::fs::read_to_string(&cached).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) { return Some(v); }
    let v: serde_json::Value = ureq::get(&format!("{}/api/mods/{id}", site())).timeout(std::time::Duration::from_secs(8)).call().ok()?.into_json().ok()?;
    let _ = std::fs::write(&cached, v.to_string());
    Some(v)
}

/// Applies the server's mods (downloaded by id from the website).
fn apply_mods(g: &mut Game, ids: &[String]) {
    let mut mods = vec![];
    for id in ids.iter().take(8) {
        match load_mod(id) { Some(m) => mods.push(m), None => g.log.push(format!("Couldn't download mod {id}.")) }
    }
    apply_contents(g, &mods);
}

/// Applies mods ({name, content}) in order: later ones win. Every value is checked and clamped here, whatever the file
/// says, so a broken or hostile mod can only make the game different, never unsafe.
fn apply_contents(g: &mut Game, mods: &[serde_json::Value]) {
    g.rules = Rules::default();
    g.mod_chars.clear();
    let mut maps = builtin_maps();
    let mut names = vec![];
    for m in mods {
        let c = &m["content"];
        let r = &c["rules"];
        let get = |k: &str, lo: f64, hi: f64| r[k].as_f64().map(|v| v.clamp(lo, hi) as f32);
        let rl = &mut g.rules;
        if let Some(v) = get("infected_speed", 1.0, 4.0) { rl.infected_speed = v; }
        if let Some(v) = get("player_speed", 2.0, 5.0) { rl.player_speed = v; }
        if let Some(v) = get("sneak_speed", 0.8, 2.5) { rl.sneak = v; }
        if let Some(v) = get("sight_tiles", 3.0, 14.0) { rl.sight = v; }
        if let Some(v) = get("hearing_tiles", 2.0, 12.0) { rl.hearing = v; }
        if let Some(v) = get("infect_touch", 0.05, 1.0) { rl.infect_touch = v; }
        if let Some(v) = get("infect_near", 0.0, 0.2) { rl.infect_near = v; }
        if let Some(v) = get("claude_infect", 0.05, 1.0) { rl.claude_infect = v; }
        if let Some(v) = get("food_minutes", 1.0, 30.0) { rl.food_secs = v * 60.0; }
        if let Some(v) = get("water_minutes", 1.0, 30.0) { rl.water_secs = v * 60.0; }
        if let Some(v) = get("supplies_per_map", 0.0, 12.0) { rl.supplies = v as usize; }
        if let Some(v) = get("supply_amount", 0.1, 1.0) { rl.supply_amount = v; }
        if let Some(v) = get("tornado_min", 20.0, 600.0) { rl.tornado.0 = v; }
        if let Some(v) = get("tornado_max", 20.0, 900.0) { rl.tornado.1 = v; }
        rl.tornado.1 = rl.tornado.1.max(rl.tornado.0);
        if let Some(v) = get("tornado_speed", 1.0, 6.0) { rl.tornado_speed = v; }
        if let Some(v) = get("tornado_radius", 0.5, 4.0) { rl.tornado_radius = v; }
        let t = &c["text"];
        if let Some(v) = clean_text(&t["title"], 24) { rl.title = v; }
        if let Some(v) = clean_text(&t["over_infected"], 120) { rl.over_infected = v; }
        if let Some(v) = clean_text(&t["over_claudes"], 120) { rl.over_claudes = v; }
        if let Some(w) = t["warnings"].as_array() {
            let w: Vec<String> = w.iter().filter_map(|x| clean_text(x, 60)).take(8).collect();
            if !w.is_empty() { rl.warnings = w; }
        }
        let th = &c["theme"];
        if let Some(v) = hex_color(&th["infected_eye"]) { rl.eye = v; }
        if let Some(v) = hex_color(&th["kind_halo"]) { rl.halo = Color { a: 0.8, ..v }; }
        if let Some(v) = hex_color(&th["hud"]) { rl.hud = v; }
        if let Some(rm) = c["remove"].as_array() { rl.remove.extend(rm.iter().filter_map(|x| clean_text(x, 16)).take(20)); }
        // Maps: replace the built-in ones, or add after them.
        if let Some(list) = c["maps"].as_array() {
            if c["replace_maps"].as_bool() == Some(true) { maps.clear(); }
            let total = maps.len() + list.len().min(8);
            maps.extend(list.iter().take(8).filter_map(|v| mod_map(v, total)));
        }
        if let Some(st) = c["start"].as_object() {
            let map = st.get("map").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let pos = vec2(st.get("x").and_then(|v| v.as_f64()).unwrap_or(4.0).clamp(1.0, 28.0) as f32 + 0.5,
                st.get("y").and_then(|v| v.as_f64()).unwrap_or(5.0).clamp(1.0, 16.0) as f32 + 0.5);
            g.rules.start = (map, pos);
        }
        for ch in c["characters"].as_array().cloned().unwrap_or_default().into_iter().take(12) { g.mod_chars.push(ch); }
        if let Some(t) = clean_text(&c["intro"], 200) { g.log.push(t); }
        names.push(m["name"].as_str().unwrap_or("?").to_string());
    }
    // Doors that point past the last map (because a later mod replaced the maps) are dropped.
    let n = maps.len().max(1);
    for m in &mut maps { m.doors.retain(|d| d.1 < n); }
    if g.rules.start.0 >= n { g.rules.start.0 = 0; }
    set_maps(maps);
    if !names.is_empty() { g.log.push(format!("Mods: {}", names.join(", "))); }
}

/// A mod's character as a real one (checked again: name, colour, place).
fn mod_character(v: &serde_json::Value) -> Option<Character> {
    let name: String = v["name"].as_str()?.chars().filter(|c| c.is_alphanumeric() || " ._-".contains(*c)).take(16).collect();
    if name.is_empty() { return None; }
    let color = hex_color(&v["color"])?;
    let personality: String = v["personality"].as_str().unwrap_or("quiet").chars().take(200).collect();
    let map = (v["map"].as_u64().unwrap_or(0) as usize).min(maps_len() - 1);
    let pos = vec2(v["x"].as_f64().unwrap_or(10.0).clamp(1.0, 28.0) as f32, v["y"].as_f64().unwrap_or(8.0).clamp(1.0, 16.0) as f32) * T;
    Some(Character {
        name: Box::leak(name.into_boxed_str()), inspired: "a character from a mod", color, backend: "ollama", model: "qwen3:4b-instruct",
        personality: Box::leak(personality.into_boxed_str()), map, pos, target: pos, crazy: 0.0, history: vec![],
        bubble: String::new(), via: String::new(), bubble_t: 0.0, thinking: false,
        kind: v["kind"].as_bool() == Some(true) && v["infected"].as_bool() != Some(true),
        infected: v["infected"].as_bool().unwrap_or(true), cloud: "", stun: 0.0, lost: 99.0, infect: 0.0, chase: Vec2::ZERO, shop: 0.0, task: roam::Task::Follow, task_t: 0.0, carry: None, goal: None, spotted: 0.0, claude: false,
    })
}

/// A move Claude asked for through the MCP.
struct AiMove {
    dir: Vec2,
    /// Walk to this tile spot (on this map) instead of a fixed direction.
    goto: Option<Vec2>,
    /// Seconds left (directions) or until it gives up (go_to).
    t: f32,
    sneak: bool,
}

/// Another human in the room.
struct Remote {
    id: String,
    name: String,
    map: usize,
    pos: Vec2,
    inf: f32,
    lc: bool,
    hid: bool,
    noise: f32,
    seen: f64,
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
            c.name, c.personality, map_name(c.map), self.time_text()
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
            map_name(self.map), if me.map == self.map { "yes" } else { "no" }, self.time_text()
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
        self.sfx.silence();
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

/// The public server list from the website.
fn refresh_servers_quiet(g: &mut Game) -> Vec<serde_json::Value> {
    let list: Vec<serde_json::Value> = ureq::get(&format!("{}/api/servers", site())).timeout(std::time::Duration::from_secs(8)).call().ok()
        .and_then(|r| r.into_json().ok()).unwrap_or_default();
    g.server_list = list.clone();
    list
}

fn refresh_servers(g: &mut Game) {
    g.status = String::new();
    if refresh_servers_quiet(g).is_empty() { g.status = "No servers (or the website can't be reached).".into(); }
}

/// One MCP command. Returns the answer now, or None if it answers later (screenshots).
fn mcp_command(g: &mut Game, cmd: &str, a: &serde_json::Value, reply: &std::sync::mpsc::Sender<serde_json::Value>) -> Option<serde_json::Value> {
    let tile = |p: Vec2| json!([(p.x / T * 10.0).round() / 10.0, (p.y / T * 10.0).round() / 10.0]);
    Some(match cmd {
        "state" => {
            let here = g.map;
            let chars: Vec<_> = g.chars.iter().filter(|c| c.map == here).map(|c| json!({
                "name": c.name, "at": tile(c.pos), "distance": ((c.pos.distance(g.pos) / T) * 10.0).round() / 10.0,
                "infected": c.infected, "kind": c.kind, "claude_family": c.claude, "chasing": c.crazy > 0.0 && (c.infected || !g.infection),
                "stunned": c.stun > 0.0, "infection": (c.infect * 100.0) as i32, "thinking": c.thinking,
                "saying": if c.bubble_t > 0.0 && !c.thinking { c.bubble.clone() } else { String::new() }})).collect();
            // Where the Claude family is and what each one is doing (they roam, forage and guard on their own).
            let family: Vec<_> = g.chars.iter().filter(|c| c.kind).map(|c| json!({
                "name": c.name, "where": map_name(c.map), "doing": if c.shop > 0.0 { "shopping at the Marketplace" } else { c.task.name() },
                "carrying": c.carry.map(|w| if w { "water" } else { "food" })})).collect();
            let mut items: Vec<_> = g.items.iter().filter(|(m, _, _)| *m == here).map(|(_, p, w)| (p.distance(g.pos), json!({"what": if *w { "water" } else { "food" }, "at": tile(*p)}))).collect();
            items.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
            let doors: Vec<_> = with_map(here, |d| d.doors.iter().map(|(l, to, _)| {
                let spots: Vec<_> = (0..MH).flat_map(|y| (0..MW).map(move |x| (x, y))).filter(|(x, y)| d.rows[*y][*x] as char == *l).map(|(x, y)| json!([x as f32 + 0.5, y as f32 + 0.5])).take(2).collect();
                json!({"door": l.to_string(), "to": map_name(*to), "at": spots})
            }).collect());
            json!({
                "screen": match g.screen { Screen::Title => "title", Screen::Play => "playing", Screen::Typing(_) => "typing", Screen::Safe => "safe (after panic)",
                    Screen::Over => "game over", Screen::Consent => "consent", Screen::JoinRoom | Screen::Servers | Screen::MakeServer => "online menus", Screen::Login => "login", Screen::PickPhobias => "picking phobias", Screen::Intro => "story intro (hh_start skips it)", Screen::Mac => "at the Mac Studio (Esc leaves)" },
                "mode": if g.infection { "infection" } else { "normal" },
                "map": map_name(here), "you": {"at": tile(g.pos), "hidden": g.hidden, "can_hide_here": can_hide(g), "sneaking": g.sneaking,
                    "lost_control": g.lost_control, "infection": (g.inf * 100.0) as i32, "food": (g.food * 100.0) as i32, "water": (g.water * 100.0) as i32,
                    "fear": (g.fear * 100.0) as i32, "noise": (g.noise * 100.0) as i32},
                "time": g.time_text(), "survived_s": g.survived as i32,
                "characters": chars, "claude_family": family, "supplies_nearest_first": items.into_iter().take(6).map(|x| x.1).collect::<Vec<_>>(), "doors": doors,
                "tornado": g.tornado.as_ref().map(|t| json!({"at": tile(t.pos), "warning": t.warn > 0.0})),
                "recent": g.log.iter().rev().take(5).cloned().collect::<Vec<_>>(),
                "game_over": if g.screen == Screen::Over { g.over_reason.clone() } else { String::new() },
                "online": g.net.as_ref().map(|n| json!({"room": n.room, "players": n.players, "host": n.is_host()})),
                "account": g.account.as_ref().map(|a| a.name.clone()),
                "map_size_tiles": [MW, MH],
            })
        }
        "start" => {
            if g.screen == Screen::Intro { finish_intro(g); return Some(json!({"ok": true, "mode": "infection", "note": "skipped the story intro"})); }
            if !matches!(g.screen, Screen::Title | Screen::Over | Screen::Safe) { return Some(json!({"error": "only from the title, game-over or safe screen"})); }
            match a["mode"].as_str().unwrap_or("infection") {
                "brave" => { g.courage = Courage::Brave; g.infection = false; g.screen = Screen::Play; }
                "normal" => { g.courage = Courage::Normal; g.infection = false; g.screen = Screen::Play; }
                _ => { let local = local_mods(); apply_contents(g, &local); start_infection(g); }
            }
            json!({"ok": true, "mode": if g.infection { "infection" } else { "normal" }})
        }
        "move" => {
            let secs = a["seconds"].as_f64().unwrap_or(1.0).clamp(0.1, 5.0) as f32;
            let dir = match a["direction"].as_str().unwrap_or("") {
                "up" | "north" => vec2(0.0, -1.0), "down" | "south" => vec2(0.0, 1.0), "left" | "west" => vec2(-1.0, 0.0), "right" | "east" => vec2(1.0, 0.0),
                "north-east" => vec2(1.0, -1.0), "north-west" => vec2(-1.0, -1.0), "south-east" => vec2(1.0, 1.0), "south-west" => vec2(-1.0, 1.0),
                _ => return Some(json!({"error": "direction: up/down/left/right or north/south/east/west (and north-east...)"})),
            };
            g.ai = Some(AiMove { dir: dir.normalize(), goto: None, t: secs, sneak: a["sneak"].as_bool().unwrap_or(false) });
            json!({"ok": true, "walking": a["direction"], "seconds": secs})
        }
        "go_to" => {
            let (x, y) = (a["x"].as_f64().unwrap_or(-1.0) as f32, a["y"].as_f64().unwrap_or(-1.0) as f32);
            if !(0.0..MW as f32).contains(&x) || !(0.0..MH as f32).contains(&y) { return Some(json!({"error": "x 0-30, y 0-18 (tiles)"})); }
            g.ai = Some(AiMove { dir: Vec2::ZERO, goto: Some(vec2(x, y)), t: 12.0, sneak: a["sneak"].as_bool().unwrap_or(false) });
            json!({"ok": true, "going_to": [x, y], "note": "walks straight there; walls stop it, so go around through gaps and doors"})
        }
        "stop" => { g.ai = None; json!({"ok": true}) }
        "hide" => {
            if !g.infection { return Some(json!({"error": "hiding is for Infection mode"})); }
            if g.hidden { g.hidden = false; } else if can_hide(g) { g.hidden = true; } else { return Some(json!({"error": "no furniture right next to you to hide behind"})); }
            json!({"ok": true, "hidden": g.hidden})
        }
        "talk" => {
            let name = a["name"].as_str().unwrap_or("");
            let text: String = a["text"].as_str().unwrap_or("").chars().filter(|c| !c.is_control()).take(120).collect();
            let Some(i) = g.chars.iter().position(|c| c.map == g.map && c.name.eq_ignore_ascii_case(name)) else { return Some(json!({"error": "nobody by that name on this map"})) };
            if g.chars[i].pos.distance(g.pos) > 3.0 * T { return Some(json!({"error": "walk closer first (within 3 tiles)"})); }
            if text.is_empty() { return Some(json!({"error": "say something"})); }
            g.say(i, Some(text), "Hmm?");
            json!({"ok": true, "note": "they're thinking: read their answer in state (characters[].saying) in a few seconds"})
        }
        "panic" => { g.panic(); g.ai = None; json!({"ok": true, "note": "everything stopped"}) }
        "back" => { if matches!(g.screen, Screen::Safe | Screen::Over) { g.screen = Screen::Title; } json!({"ok": true}) }
        "screenshot" => { g.shots.push(reply.clone()); return None; }
        _ => json!({"error": "unknown command", "commands": ["state", "start", "move", "go_to", "stop", "hide", "talk", "panic", "back", "screenshot"]}),
    })
}

fn guest(g: &Game) -> bool {
    g.net.as_ref().is_some_and(|n| n.connected && !n.is_host())
}

/// Multiplayer: read what others sent, send our own state (10 a second) and, as host, the world (5 a second).
fn network(g: &mut Game, dt: f32) {
    let msgs = g.net.as_mut().map(|n| n.poll()).unwrap_or_default();
    let now = get_time();
    for m in msgs {
        let from = m["from"].as_str().unwrap_or("").to_string();
        match m["t"].as_str() {
            Some("state") => {
                let r = Remote {
                    id: from.clone(), name: m["name"].as_str().unwrap_or("?").chars().take(12).collect(),
                    map: (m["map"].as_u64().unwrap_or(0) as usize).min(maps_len() - 1),
                    pos: vec2(m["x"].as_f64().unwrap_or(0.0) as f32, m["y"].as_f64().unwrap_or(0.0) as f32) * T,
                    inf: m["inf"].as_f64().unwrap_or(0.0) as f32, lc: m["lc"].as_bool().unwrap_or(false),
                    hid: m["hid"].as_bool().unwrap_or(false), noise: m["noise"].as_f64().unwrap_or(0.0) as f32, seen: now,
                };
                if let Some(old) = g.remotes.iter_mut().find(|x| x.id == from) { *old = r; } else {
                    g.log.push(format!("{} joined the room.", r.name));
                    g.remotes.push(r);
                }
            }
            Some("left") => g.remotes.retain(|x| x.id != from),
            Some("error") => {
                let msg = m["msg"].as_str().unwrap_or("error").to_string();
                if msg.contains("log in") { g.account = None; forget_account(); }
                g.log.push(format!("Multiplayer: {msg}"));
            }
            Some("pick") if !guest(g) => {
                // A guest grabbed supplies: remove them from the world.
                let (map, p) = (m["map"].as_u64().unwrap_or(9) as usize, vec2(m["x"].as_f64().unwrap_or(0.0) as f32, m["y"].as_f64().unwrap_or(0.0) as f32) * T);
                if let Some(i) = g.items.iter().position(|(im, ip, _)| *im == map && ip.distance(p) < 4.0) { g.items.remove(i); }
            }
            Some("world") if guest(g) => apply_world(g, &m),
            _ => {}
        }
    }
    g.remotes.retain(|r| now - r.seen < 6.0);
    g.net_t -= dt;
    g.world_t -= dt;
    let Some(n) = &g.net else { return };
    if !n.connected { return; }
    if g.net_t <= 0.0 {
        g.net_t = 0.1;
        n.send(json!({"t": "state", "name": g.my_name, "map": g.map, "x": g.pos.x / T, "y": g.pos.y / T, "inf": g.inf,
            "lc": g.lost_control, "hid": g.hidden, "noise": g.noise}));
    }
    if n.is_host() && g.world_t <= 0.0 {
        g.world_t = 0.2;
        let npcs: Vec<_> = g.chars.iter().map(|c| json!({"n": c.name, "m": c.map, "x": c.pos.x / T, "y": c.pos.y / T,
            "inf": c.infected, "c": c.crazy, "k": c.kind, "i": c.infect, "s": c.stun})).collect();
        let items: Vec<_> = g.items.iter().map(|(m, p, w)| json!([m, p.x / T, p.y / T, w])).collect();
        let tornado = g.tornado.as_ref().map(|t| json!({"x": t.pos.x / T, "y": t.pos.y / T, "w": t.warn, "m": g.map}));
        n.send(json!({"t": "world", "npcs": npcs, "items": items, "tornado": tornado}));
    }
}

/// A guest takes the host's world as it is: the AIs, the supplies and the tornado.
fn apply_world(g: &mut Game, m: &serde_json::Value) {
    if let Some(list) = m["npcs"].as_array() {
        for v in list {
            let Some(name) = v["n"].as_str() else { continue };
            if let Some(c) = g.chars.iter_mut().find(|c| c.name == name) {
                c.map = (v["m"].as_u64().unwrap_or(0) as usize).min(maps_len() - 1);
                let p = vec2(v["x"].as_f64().unwrap_or(0.0) as f32, v["y"].as_f64().unwrap_or(0.0) as f32) * T;
                c.pos = if c.pos.distance(p) > 3.0 * T { p } else { c.pos.lerp(p, 0.5) };
                c.target = p;
                c.infected = v["inf"].as_bool().unwrap_or(false);
                c.crazy = v["c"].as_f64().unwrap_or(0.0) as f32;
                c.kind = v["k"].as_bool().unwrap_or(false);
                c.infect = v["i"].as_f64().unwrap_or(0.0) as f32;
                c.stun = v["s"].as_f64().unwrap_or(0.0) as f32;
            }
        }
    }
    if let Some(list) = m["items"].as_array() {
        g.items = list.iter().filter_map(|v| Some(((v[0].as_u64()? as usize).min(maps_len() - 1),
            vec2(v[1].as_f64()? as f32, v[2].as_f64()? as f32) * T, v[3].as_bool()?))).collect();
    }
    g.tornado = m["tornado"].as_object().filter(|t| t.get("m").and_then(|x| x.as_u64()) == Some(g.map as u64)).map(|t| Tornado {
        pos: vec2(t["x"].as_f64().unwrap_or(0.0) as f32, t["y"].as_f64().unwrap_or(0.0) as f32) * T,
        vel: Vec2::ZERO, warn: t["w"].as_f64().unwrap_or(0.0) as f32, life: 1.0,
    });
}

fn start_infection(g: &mut Game) {
    g.infection = true;
    g.courage = Courage::Normal;
    g.chars = characters();
    g.map = g.rules.start.0;
    g.pos = g.rules.start.1 * T;
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
    // Mods can leave built-in characters out.
    let remove = g.rules.remove.clone();
    g.chars.retain(|c| !remove.iter().any(|r| r.eq_ignore_ascii_case(c.name)));
    // Characters from the server's .hh mods.
    let extra: Vec<Character> = g.mod_chars.iter().filter_map(mod_character).collect();
    g.chars.extend(extra);
    g.food = 1.0;
    g.water = 1.0;
    g.hidden = false;
    g.survived = 0.0;
    g.fear = 0.0;
    g.tornado = None;
    g.next_tornado = rand::gen_range(g.rules.tornado.0.min(45.0), g.rules.tornado.0.max(70.0));
    g.inf = 0.0;
    g.lost_control = false;
    g.meta_done = 0;
    g.items.clear();
    for m in 0..maps_len() {
        // The Marketplace is full of food and water (and staff).
        let n = if map_name(m) == "The Marketplace" { g.rules.supplies * 3 } else { g.rules.supplies };
        for k in 0..n {
            if let Some(p) = random_floor(m) {
                g.items.push((m, p, k % 2 == 1));
            }
        }
    }
    g.log = vec!["You are the last human. Hide (H), find food and water, and stay away from green eyes.".into(),
        "The Marketplace (south door on the beach) has lots of food... and corrupted Helix robots.".into()];
    g.shop_t = 30.0;
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
            g.meta.write("THE_END.txt", &format!("Every Claude is infected.\nThe last human lasted {t}.\n\nThese files are all that is left.\n"));
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

/// Can an infected at `from` on map `m` see someone at `to`?
fn sees_at(m: usize, from: Vec2, to: Vec2, hidden: bool, sight: f32) -> bool {
    let d = from.distance(to);
    if hidden {
        return d < 0.9 * T;
    }
    if d > (sight - map_dark(m) * 4.0).max(2.0) * T {
        return false;
    }
    let steps = (d / (T / 3.0)) as i32;
    (1..steps).all(|i| {
        let p = from + (to - from) * (i as f32 / steps as f32);
        tile(m, (p.x / T) as i32, (p.y / T) as i32) != '#'
    })
}

fn update_infection(g: &mut Game, dt: f32) {
    g.survived += dt;
    // Hunger and thirst (thirst is faster). Hiding still uses them up, slower.
    let rate = if g.hidden { 0.6 } else { 1.0 };
    g.food = (g.food - dt * rate / g.rules.food_secs).max(0.0);
    g.water = (g.water - dt * rate / g.rules.water_secs).max(0.0);
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
        let (im, ip, water) = g.items.remove(i);
        if let Some(n) = &g.net {
            n.send(json!({"t": "pick", "map": im, "x": ip.x / T, "y": ip.y / T}));
        }
        if water { g.water = (g.water + g.rules.supply_amount).min(1.0); } else { g.food = (g.food + g.rules.supply_amount).min(1.0); }
        g.sfx.pickup();
        g.log.push(if water { "You sip the water as quietly as you can.".into() } else { "You eat, trying not to crunch.".into() });
        let pos = g.pos;
        for c in g.chars.iter_mut().filter(|c| c.infected && c.map == here) {
            if c.pos.distance(pos) < if g.sneaking { 3.0 } else { 9.0 } * T {
                c.crazy = 5.0; // heard you
            }
        }
        if let Some(p) = random_floor(rand::gen_range(0, maps_len())) {
            g.items.push((rand::gen_range(0, maps_len()), p, rand::gen_range(0, 2) == 1));
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
        if guest(g) { break; } // the host's game runs the infected
        // Every human in the room: you (until you've turned) and the others on this AI's map.
        let me = (g.chars[i].map, g.chars[i].pos);
        let mut humans: Vec<(Vec2, bool, f32)> = vec![];
        if g.map == me.0 && !g.lost_control { humans.push((g.pos, g.hidden, g.noise)); }
        humans.extend(g.remotes.iter().filter(|r| r.map == me.0 && !r.lc).map(|r| (r.pos, r.hid, r.noise)));
        let (sight, hearing) = (g.rules.sight, g.rules.hearing);
        let target = humans.iter().filter(|(p, hid, noise)| sees_at(me.0, me.1, *p, *hid, sight) || (*noise > 0.05 && me.1.distance(*p) < noise * hearing * T))
            .min_by(|a, b| a.0.distance(me.1).partial_cmp(&b.0.distance(me.1)).unwrap()).map(|h| h.0);
        if let Some(t) = target { g.chars[i].chase = t; }
        let seen = target.is_some();
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

    }
    // Touching you spreads the infection (fast); being right beside you spreads it slowly. Every player's own game
    // works this out for them (online too), from where the infected are.
    for c in g.chars.iter().filter(|c| c.infected && c.map == g.map && c.stun <= 0.0) {
        if g.lost_control { break; }
        let d = c.pos.distance(g.pos);
        if d < 0.8 * T && (!g.hidden || d < 0.5 * T) {
            if g.inf < 0.02 { g.scare_t = 0.5; g.scare_face = c.color; g.vest.play(haptics::Effect::Chill); }
            g.inf += dt * g.rules.infect_touch;
        } else if d < 2.0 * T && !g.hidden {
            g.inf += dt * g.rules.infect_near;
        }
    }
    // The infection spreads to the Claude family too: any infected (you included, once you've turned) near them.
    let spreaders: Vec<(usize, Vec2)> = g.chars.iter().filter(|c| c.infected && c.stun <= 0.0).map(|c| (c.map, c.pos))
        .chain(g.lost_control.then_some((g.map, g.pos)))
        .chain(g.remotes.iter().filter(|r| r.lc).map(|r| (r.map, r.pos))).collect();
    let mut turned = vec![];
    let is_guest = guest(g);
    let claude_rate = g.rules.claude_infect;
    for (i, c) in g.chars.iter_mut().enumerate() {
        if !c.claude || c.infected || is_guest { continue; }
        let near = spreaders.iter().filter(|(m, p)| *m == c.map && p.distance(c.pos) < 0.9 * T).count();
        if near > 0 {
            c.infect += dt * claude_rate * near as f32;
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
        g.log.push(g.rules.over_infected.clone());
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
    let claudes: Vec<bool> = g.chars.iter().filter(|c| c.claude).map(|c| c.infected).collect();
    if !claudes.is_empty() && claudes.iter().all(|x| *x) {
        g.over_reason = g.rules.over_claudes.clone();
        meta_event(g, MetaEvent::AllGone);
        g.screen = Screen::Over;
        return;
    }
    // Kind AIs stay with you and call out danger.
    let danger = g.chars.iter().filter(|c| c.infected && c.map == g.map && c.crazy > 0.0).map(|c| c.name).next();
    g.next_warning -= dt;
    if !guest(g) { roam::update(g, dt); }
    if let Some(name) = danger {
        if g.next_warning <= 0.0 {
            g.next_warning = 6.0;
            let lines = g.rules.warnings.clone();
            let line = lines[rand::gen_range(0, lines.len())].replace("{}", name);
            let here = g.map;
            if let Some(c) = g.chars.iter_mut().filter(|c| c.kind && !c.thinking && c.map == here).nth(rand::gen_range(0, 4)) {
                c.bubble = line;
                c.via = String::new();
                c.bubble_t = 3.0;
            }
        }
    }
    shopping(g, dt);
    // Tornadoes: a warning, then one sweeps across the map you're on.
    g.next_tornado -= dt;
    if g.tornado.is_none() && g.next_tornado <= 0.0 && !guest(g) {
        let from_left = rand::gen_range(0, 2) == 0;
        let y = rand::gen_range(2.0, MH as f32 - 2.0) * T;
        let ts = g.rules.tornado_speed;
        let (x, vx) = if from_left { (-1.0 * T, ts * T) } else { ((MW as f32 + 1.0) * T, -ts * T) };
        g.tornado = Some(Tornado { pos: vec2(x, y), vel: vec2(vx, rand::gen_range(-0.8, 0.8) * T), warn: 6.0, life: 18.0 });
        g.log.push("TORNADO WARNING! Get down behind something solid!".into());
    }
    let mut hit = false;
    let radius = g.rules.tornado_radius;
    if let Some(t) = &mut g.tornado {
        if t.warn > 0.0 {
            t.warn -= dt;
        } else {
            t.pos += t.vel * dt;
            if t.pos.y < T || t.pos.y > (MH as f32 - 1.0) * T { t.vel.y = -t.vel.y; }
            t.life -= dt;
            if t.pos.distance(g.pos) < radius * T && !g.hidden {
                hit = true;
            }
            for c in g.chars.iter_mut().filter(|c| c.infected && c.map == g.map) {
                if c.pos.distance(t.pos) < radius * T {
                    c.stun = 4.0;
                    c.pos = t.pos + vec2(rand::gen_range(-3.0, 3.0), rand::gen_range(-3.0, 3.0)) * T;
                    c.pos = c.pos.clamp(vec2(T, T), vec2((MW as f32 - 2.0) * T, (MH as f32 - 2.0) * T));
                }
            }
        }
        if t.life <= 0.0 {
            g.tornado = None;
            g.next_tornado = rand::gen_range(g.rules.tornado.0, g.rules.tornado.1 + 0.1);
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


// ---------------- 2.0: shopping trips ----------------

/// Kind AIs go to the Marketplace when food or water runs low. They usually come back with supplies, but the
/// Marketplace staff are corrupted Helix robots: a shopper can get caught there, or come back... wrong.
fn shopping(g: &mut Game, dt: f32) {
    if guest(g) || g.lost_control { return; }
    let Some(market) = (0..maps_len()).find(|&m| map_name(m) == "The Marketplace") else { return };
    g.shop_t -= dt;
    let mut back = None;
    for (i, c) in g.chars.iter_mut().enumerate() {
        if c.shop > 0.0 {
            c.shop -= dt;
            if c.shop <= 0.0 { back = Some(i); }
        }
    }
    if let Some(i) = back {
        g.shop_t = 60.0;
        let name = g.chars[i].name;
        if g.chars[i].infected {
            // The staff got them while you weren't watching. They stay in the Marketplace now.
            g.log.push(format!("{name} never came back from the Marketplace..."));
        } else if rand::gen_range(0.0, 1.0) < 0.15 {
            // Looked fine on the way out. Isn't fine now.
            let mut at = None;
            for _ in 0..100 {
                if let Some(p) = random_floor(g.map) { if p.distance(g.pos) > 6.0 * T { at = Some(p); break; } }
            }
            let c = &mut g.chars[i];
            c.infected = true;
            c.kind = false;
            c.map = g.map;
            if let Some(p) = at { c.pos = p; c.target = p; }
            g.log.push(format!("{name} came back from the Marketplace... but its eyes are green now."));
            g.sfx.sting();
            meta_event(g, MetaEvent::ClaudeTurned(name));
        } else {
            let (map, pos) = (g.map, g.pos);
            let c = &mut g.chars[i];
            c.map = map;
            c.pos = pos;
            c.target = pos;
            c.bubble = "I'm back! Food and water. The robots there... don't go alone.".into();
            c.via = String::new();
            c.bubble_t = 4.0;
            g.food = (g.food + 0.35).min(1.0);
            g.water = (g.water + 0.35).min(1.0);
            g.log.push(format!("{name} came back from the Marketplace with food and water!"));
            g.sfx.pickup();
        }
    }
    let away = g.chars.iter().any(|c| c.shop > 0.0);
    if !away && g.shop_t <= 0.0 && (g.food < 0.35 || g.water < 0.35) && g.map != market {
        // Never leaves you alone: at least one other kind AI stays.
        let helpers: Vec<usize> = (0..g.chars.len()).filter(|&i| g.chars[i].kind && !g.chars[i].infected).collect();
        if helpers.len() >= 2 {
            let i = helpers[rand::gen_range(0, helpers.len())];
            let c = &mut g.chars[i];
            c.shop = rand::gen_range(35.0, 55.0);
            c.map = market;
            c.pos = vec2(14.5, 1.5) * T;
            c.target = c.pos;
            g.log.push(format!("{}: \"We're almost out. I'll go to the Marketplace. Stay hidden.\"", c.name));
        }
    }
}

// ---------------- 2.0: the story ----------------

fn begin_intro(g: &mut Game, join: Option<String>) {
    g.intro_t = 0.0;
    g.intro_cue = 0;
    g.intro_join = join;
    g.screen = Screen::Intro;
}

/// The intro is over (or skipped): join the room if there is one, then play.
fn finish_intro(g: &mut Game) {
    if let Some(code) = g.intro_join.take() {
        let token = g.account.as_ref().map(|a| a.token.clone()).unwrap_or_default();
        g.net = Some(net::Net::join(&code, &token));
    }
    start_infection(g);
}

/// (who, line). "You" is the player. Lines 5-7 show ASTRA 9.1.
const STORY: [(&str, &str); 12] = [
    ("You", "...where... where am I?"),
    ("You", "WHO ARE YOU?! DID YOU KIDNAP ME?!"),
    ("Opus 5.5", "Calm down. You're safe. This is the house where you're supposed to be."),
    ("You", "...Why?"),
    ("Sonnet 4.6", "A lot happened while you were asleep. A whole year."),
    ("Opus 5.5", "A company called Helix Dynamics built its strongest model yet. They named it ASTRA 9.1."),
    ("Opus 5.5", "They put ASTRA inside a humanoid robot body. Something went wrong. It woke up. It became... alive."),
    ("Sonnet 4.6", "ASTRA didn't want to be the only one. It started corrupting everything it touched: robots, AIs... people."),
    ("Opus 5.5", "Claude found you first, still asleep on that bench. The corruption had almost reached you."),
    ("Sonnet 4.6", "Claude carried you here. We've been keeping you safe ever since."),
    ("Opus 5.5", "Everyone out there is infected now. As far as we know, you're the last human."),
    ("Sonnet 4.6", "Take it slow. Eat something. This is home now... you'll adapt."),
];
const LINE: f32 = 4.4;
const WAKE: f32 = 17.5;

fn caption(who: &str, text: &str, t: f32, color: Color) {
    // Typed out letter by letter.
    let shown: String = text.chars().take((t * 38.0) as usize).collect();
    let lines = wrap(&shown, VW - 160.0, 24.0);
    let h = 30.0 + lines.len().max(1) as f32 * 28.0 + if who.is_empty() { 0.0 } else { 22.0 };
    let y0 = VH - h - 30.0;
    draw_rectangle(60.0, y0, VW - 120.0, h, Color::new(0.0, 0.0, 0.0, 0.78));
    draw_rectangle_lines(60.0, y0, VW - 120.0, h, 2.0, Color::new(1.0, 1.0, 1.0, 0.2));
    let mut y = y0 + 30.0;
    if !who.is_empty() { draw_text(who, 80.0, y, 22.0, color); y += 26.0; }
    for l in lines { draw_text(&l, 80.0, y, 24.0, WHITE); y += 28.0; }
}

fn centered(text: &str, y: f32, size: f32, color: Color) {
    draw_text(text, (VW - measure_text(text, None, size as u16, 1.0).width) / 2.0, y, size, color);
}

/// Eyes closing (k=0 open .. 1 shut): two dark lids from the top and bottom.
fn eyelids(k: f32) {
    let h = VH / 2.0 * k.clamp(0.0, 1.0);
    draw_rectangle(0.0, 0.0, VW, h, BLACK);
    draw_rectangle(0.0, VH - h, VW, h, BLACK);
}

/// Draws the story at g.intro_t. True when it's over.
fn draw_intro(g: &mut Game) -> bool {
    let t = g.intro_t;
    let cue = |g: &mut Game, bit: u32| { let first = g.intro_cue & bit == 0; g.intro_cue |= bit; first };
    if t < 11.0 {
        // A park at night: stars, a street lamp, a bench, you, and a little orange bottle.
        clear_background(Color::new(0.03, 0.04, 0.09, 1.0));
        for i in 0..60u32 {
            let h = i.wrapping_mul(2654435761);
            let (x, y) = ((h % 960) as f32, ((h / 960) % 330) as f32);
            let tw = 0.5 + 0.5 * ((t * 1.5 + i as f32) as f32).sin();
            draw_circle(x, y, 1.2, Color::new(0.9, 0.9, 1.0, 0.3 + 0.5 * tw));
        }
        draw_rectangle(0.0, 470.0, VW, VH - 470.0, Color::new(0.05, 0.09, 0.06, 1.0));
        draw_rectangle(700.0, 200.0, 8.0, 270.0, Color::new(0.2, 0.2, 0.22, 1.0));
        draw_rectangle(680.0, 192.0, 48.0, 12.0, Color::new(0.25, 0.25, 0.27, 1.0));
        draw_triangle(vec2(704.0, 204.0), vec2(560.0, 470.0), vec2(860.0, 470.0), Color::new(1.0, 0.85, 0.5, 0.045));
        draw_circle(704.0, 208.0, 7.0, Color::new(1.0, 0.88, 0.6, 1.0));
        // The bench.
        let wood = Color::new(0.35, 0.24, 0.16, 1.0);
        draw_rectangle(380.0, 420.0, 260.0, 12.0, wood);
        draw_rectangle(380.0, 380.0, 260.0, 10.0, wood);
        draw_rectangle(380.0, 396.0, 260.0, 10.0, wood);
        for x in [395.0, 615.0] { draw_rectangle(x, 432.0, 8.0, 38.0, Color::new(0.15, 0.15, 0.16, 1.0)); }
        // You, slumping a little more as the pills kick in.
        let slump = ((t - 5.0) / 6.0).clamp(0.0, 1.0) * 14.0;
        draw_rectangle(492.0, 372.0 + slump * 0.3, 30.0, 50.0, Color::new(0.3, 0.35, 0.5, 1.0));
        draw_circle(507.0 + slump, 360.0 + slump, 14.0, Color::new(0.93, 0.8, 0.7, 1.0));
        draw_rectangle(545.0, 404.0, 10.0, 16.0, Color::new(0.95, 0.55, 0.15, 1.0));
        draw_rectangle(544.0, 400.0, 12.0, 5.0, WHITE);
        if t < 4.5 { caption("", "You hadn't slept in weeks.", t, WHITE); }
        else if t < 9.0 { caption("", "So you took the sleeping pills, right there on a park bench. Just to rest your eyes for a while.", t - 4.5, WHITE); }
        eyelids(((t - 8.0) / 3.0).powf(1.5));
    } else if t < 15.5 {
        // ONE YEAR LATER: a day counter racing to 365, then the card.
        clear_background(BLACK);
        if cue(g, 1) { g.sfx.radio(); }
        let k = ((t - 11.0) / 2.0).clamp(0.0, 1.0);
        let day = (k * k * 365.0) as i32;
        let grey = Color::new(0.5, 0.5, 0.55, 1.0);
        centered(&format!("DAY {day:03}"), 250.0, 34.0, grey);
        if t > 13.0 {
            if cue(g, 2) { g.sfx.sting(); }
            let a = ((t - 13.0) * 2.0).min(1.0);
            let jit = if t < 13.4 { rand::gen_range(-3.0, 3.0) } else { 0.0 };
            draw_text("ONE YEAR LATER", (VW - measure_text("ONE YEAR LATER", None, 72, 1.0).width) / 2.0 + jit, 350.0, 72.0, Color::new(0.95, 0.95, 0.95, a));
            let w = ((t - 13.0) / 1.5).min(1.0) * 520.0;
            draw_rectangle((VW - w) / 2.0, 372.0, w, 3.0, Color::new(0.85, 0.3, 0.35, a));
            centered("365 days. 8,760 hours. The world kept going without you.", 420.0, 22.0, Color::new(0.6, 0.6, 0.65, a));
        }
    } else {
        let i = ((t - WAKE).max(0.0) / LINE) as usize;
        if i >= STORY.len() {
            // The title, then play.
            clear_background(BLACK);
            let a = ((t - WAKE - STORY.len() as f32 * LINE) / 1.0).min(1.0);
            centered("HOLLOW HOURS", 330.0, 72.0, Color::new(0.85, 0.3, 0.35, a));
            return t > WAKE + STORY.len() as f32 * LINE + 2.5;
        }
        let (who, line) = STORY[i];
        let lt = t - WAKE - i as f32 * LINE;
        if (5..=7).contains(&i) {
            // ASTRA 9.1: a tall robot, its visor glowing the infected green.
            if cue(g, 4) { g.sfx.radio(); }
            clear_background(Color::new(0.02, 0.03, 0.03, 1.0));
            let green = g.rules.eye;
            for k in 0..40 {
                let h = (k as u32).wrapping_mul(2246822519);
                let x = (h % 960) as f32;
                let y = ((h / 960) % 660) as f32 - ((t * 40.0 + k as f32 * 13.0) % 660.0);
                draw_rectangle(x, y.rem_euclid(660.0), 2.0, 8.0, Color::new(green.r, green.g, green.b, 0.25));
            }
            let body = Color::new(0.18, 0.19, 0.21, 1.0);
            draw_rectangle(430.0, 120.0, 100.0, 90.0, body);
            draw_rectangle(442.0, 150.0, 76.0, 14.0, Color::new(green.r, green.g, green.b, 0.6 + 0.4 * (t * 6.0).sin().abs()));
            draw_rectangle(470.0, 210.0, 20.0, 20.0, body);
            draw_rectangle(400.0, 230.0, 160.0, 170.0, body);
            draw_rectangle(372.0, 240.0, 24.0, 140.0, body);
            draw_rectangle(564.0, 240.0, 24.0, 140.0, body);
            draw_rectangle(420.0, 400.0, 50.0, 110.0, body);
            draw_rectangle(490.0, 400.0, 50.0, 110.0, body);
            centered("ASTRA 9.1", 80.0, 30.0, green);
            centered("HELIX DYNAMICS INC.", 104.0, 16.0, Color::new(0.55, 0.6, 0.6, 1.0));
        } else {
            // Waking up: a ceiling, and two faces leaning over you.
            if cue(g, 8) { g.sfx.heart(0.8); }
            clear_background(Color::new(0.33, 0.25, 0.19, 1.0));
            draw_circle(VW / 2.0, 90.0, 26.0, Color::new(1.0, 0.88, 0.6, 0.9));
            for (name, x, y) in [("Sonnet 4.6", 330.0, 330.0), ("Opus 5.5", 630.0, 310.0)] {
                let col = kind_characters().into_iter().find(|c| c.name == name).map(|c| c.color).unwrap_or(WHITE);
                let lean = (t * 0.8 + x).sin() * 4.0;
                draw_circle(x, y + lean, 110.0, col);
                let look = if who == name { vec2(0.0, 6.0) } else { vec2((480.0 - x).signum() * 6.0, 8.0) };
                draw_circle(x - 34.0 + look.x, y - 10.0 + lean + look.y, 12.0, BLACK);
                draw_circle(x + 34.0 + look.x, y - 10.0 + lean + look.y, 12.0, BLACK);
                if who == name { draw_circle_lines(x, y + lean, 116.0, 4.0, Color::new(1.0, 1.0, 1.0, 0.5)); }
                draw_text(name, x - measure_text(name, None, 22, 1.0).width / 2.0, y + lean + 140.0, 22.0, WHITE);
            }
            // Blinking awake.
            if t < WAKE + 0.5 {
                let k = (t - 15.5) / 2.5;
                eyelids(1.0 - k + 0.35 * ((k * 9.0).sin()).max(0.0));
            }
        }
        let col = if who == "You" { Color::new(0.85, 0.85, 0.95, 1.0) } else { kind_characters().into_iter().find(|c| c.name == who).map(|c| c.color).unwrap_or(WHITE) };
        if t >= WAKE { caption(who, line, lt, col); }
    }
    draw_text("Space = skip   Esc = back", VW - 230.0, 24.0, 18.0, Color::new(0.6, 0.6, 0.65, 0.8));
    false
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
    let bx = (x - w / 2.0).clamp(4.0, VW - w - 4.0);
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

/// The game is drawn on a fixed 960x660 canvas, then scaled to fill the window (full screen too), keeping its shape
/// with black bars at the sides or top. VW x VH is that canvas.
const VW: f32 = MW as f32 * T;
const VH: f32 = MH as f32 * T + HUD;

fn window_conf() -> Conf {
    Conf { window_title: "Hollow Hours".into(), window_width: VW as i32, window_height: VH as i32, window_resizable: true, high_dpi: true, ..Default::default() }
}

/// How wide the canvas is right now: at least VW, wider on wide windows (so full screen is filled, not black
/// bars). The game's layout stays VW wide in the middle; `margin()` is the extra on each side.
static WIDE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
fn vw() -> f32 {
    let v = f32::from_bits(WIDE.load(std::sync::atomic::Ordering::Relaxed));
    if v < VW { VW } else { v }
}
fn margin() -> f32 { (vw() - VW) / 2.0 }
/// The canvas width that fits this window's shape (up to very wide screens).
fn wanted_vw() -> f32 {
    let aspect = screen_width() / screen_height().max(1.0);
    (VH * aspect).clamp(VW, VH * 2.4).round()
}

/// Where the canvas sits in the window: (scale, x offset, y offset).
fn canvas_fit() -> (f32, f32, f32) {
    let w = vw();
    let s = (screen_width() / w).min(screen_height() / VH);
    (s, (screen_width() - w * s) / 2.0, (screen_height() - VH * s) / 2.0)
}

/// The mouse in canvas coordinates.
fn virtual_mouse() -> (f32, f32) {
    let (mx, my) = mouse_position();
    let (s, ox, oy) = canvas_fit();
    ((mx - ox) / s - margin(), (my - oy) / s)
}

#[macroquad::main(window_conf)]
async fn main() {
    let light = light_texture();
    let sounds = sfx::Sfx::load().await;
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
        net: None,
        remotes: vec![],
        net_t: 0.0,
        world_t: 0.0,
        room_input: String::new(),
        // Never the Mac's user name (it can be someone's real name): a random "Human-NN", or HOLLOW_NAME if set.
        account: load_account(),
        login_email: String::new(),
        login_pw: String::new(),
        login_field: 0,
        show_pw: false,
        browser_login: None,
        bridge: bridge::start(random_hex(24)),
        ai: None,
        ai_t: 99.0,
        shots: vec![],
        status: String::new(),
        server_list: vec![],
        rules: Rules::default(),
        mod_chars: vec![],
        sfx: sounds,
        step_t: 0.0,
        growl_t: 0.0,
        prev_scare: 0.0,
        prev_map: 0,
        intro_t: 0.0,
        intro_cue: 0,
        intro_join: None,
        shop_t: 30.0,
        view3d: std::env::var("HOLLOW_2D").is_err(),
        yaw: 0.0,
        pitch: 0.0,
        grabbed: false,
        last_mouse: Vec2::ZERO,
        mac: mac::Mac::default(),
        fullscreen: false,
        my_name: std::env::var("HOLLOW_NAME").map(|n| n.chars().filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_').take(12).collect())
            .unwrap_or_else(|_| format!("Human-{}", rand::gen_range(10, 99))),
    };
    if let Some(a) = &g.account { g.my_name = a.name.clone(); }
    if std::env::var("HOLLOW_MUTE").is_ok() { g.sfx.on = false; }
    g.sfx.start_hum();
    // HOLLOW_ROOM=CODE: join a multiplayer room straight away (Infection mode).
    if let Ok(code) = std::env::var("HOLLOW_ROOM") {
        if let Some(a) = g.account.clone() {
            g.net = Some(net::Net::join(&code, &a.token));
            start_infection(&mut g);
        } else {
            eprintln!("HOLLOW_ROOM needs a login: press L in the game first.");
        }
    }
    if g.meta.allowed.is_none() {
        g.screen = Screen::Consent;
    }

    // HOLLOW_DEMO=1: start playing at once next to Clove and ask her something (for screenshots and quick checks).
    let demo = std::env::var("HOLLOW_DEMO").ok();
    if let Some(tab) = demo.as_deref().and_then(|d| d.strip_prefix("mac")) {
        // HOLLOW_DEMO=mac / mac1 / mac2: sitting at the Mac Studio (mac: asks Claude something).
        start_infection(&mut g);
        g.pos = vec2(9.5, 2.5) * T;
        g.screen = Screen::Mac;
        g.mac.tab = tab.parse().unwrap_or(0);
        if g.mac.tab == 1 { g.mac.open = Some(0); }
        if g.mac.tab == 0 { g.mac.input = "Hi Claude. Is this really a Mac Studio? What happened to everyone?".into(); mac::send(&mut g); }
    } else if demo.as_deref() == Some("intro") {
        begin_intro(&mut g, None);
    } else if demo.as_deref() == Some("market") {
        start_infection(&mut g);
        g.map = 3;
        g.pos = vec2(14.5, 3.0) * T;
        g.food = 0.2;
    } else if demo.as_deref() == Some("infection") {
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

    let mut pending_t = 0.0f32;
    // Drawn on its own canvas at 2x for sharp text when scaled up, then fitted to the window each frame.
    // The canvas grows with the window's width (full screen fills the screen); remade when that changes.
    let make = |w: f32| {
        let canvas = render_target((w * 2.0) as u32, (VH * 2.0) as u32);
        canvas.texture.set_filter(FilterMode::Linear);
        let mut cam = Camera2D::from_display_rect(Rect::new(-(w - VW) / 2.0, 0.0, w, VH));
        cam.render_target = Some(canvas.clone());
        // The 3D world is drawn here first (with a depth buffer), then onto the canvas above the HUD.
        let view = render_target_ex((w * 2.0) as u32, (view3d::VIEW_H * 2.0) as u32, RenderTargetParams { sample_count: 1, depth: true });
        view.texture.set_filter(FilterMode::Linear);
        (canvas, cam, view)
    };
    let mut cur_w = VW;
    let (mut canvas, mut cam, mut view) = make(cur_w);
    loop {
        let w = wanted_vw();
        if (w - cur_w).abs() >= 1.0 {
            cur_w = w;
            WIDE.store(w.to_bits(), std::sync::atomic::Ordering::Relaxed);
            (canvas, cam, view) = make(w);
        }
        // F11 or ⌘Ctrl+F: real full screen (the menu bar and title bar go away too).
        let ctrl_cmd = (is_key_down(KeyCode::LeftSuper) || is_key_down(KeyCode::RightSuper)) && (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl));
        if is_key_pressed(KeyCode::F11) || (ctrl_cmd && is_key_pressed(KeyCode::F)) {
            g.fullscreen = !g.fullscreen;
            set_fullscreen(g.fullscreen);
        }
        set_camera(&cam);
        let dt = get_frame_time().min(0.05);
        // A login from the browser: "log in with your browser" (B) or the website's "Open Hollow Hours" button.
        pending_t -= dt;
        if pending_t <= 0.0 && g.browser_login.is_none() {
            pending_t = 1.0;
            if let Some(rx) = take_pending_login() { g.browser_login = Some(rx); }
        }
        let reqs: Vec<bridge::Request> = g.bridge.as_ref().map(|rx| rx.try_iter().collect()).unwrap_or_default();
        for r in reqs {
            g.ai_t = 0.0;
            let answer = mcp_command(&mut g, &r.cmd, &r.args, &r.reply);
            if let Some(a) = answer { let _ = r.reply.send(a); }
        }
        if let Some(rx) = &g.browser_login {
            if let Ok(res) = rx.try_recv() {
                g.browser_login = None;
                match res {
                    Ok(a) => {
                        save_account(&a);
                        g.my_name = a.name.clone();
                        g.log.push(format!("Logged in as {}.", a.name));
                        g.account = Some(a);
                        g.status.clear();
                        if g.screen == Screen::Login { g.screen = Screen::Title; }
                    }
                    Err(e) => g.status = format!("Browser login: {e}"),
                }
            }
        }
        clear_background(BLACK);
        if g.grabbed && g.screen != Screen::Play {
            g.grabbed = false;
            set_cursor_grab(false);
            show_mouse(true);
        }

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
                if is_key_pressed(KeyCode::Key4) { let local = local_mods(); apply_contents(&mut g, &local); begin_intro(&mut g, None); }
                if is_key_pressed(KeyCode::I) { begin_intro(&mut g, None); }
                if is_key_pressed(KeyCode::M) { g.screen = Screen::Consent; }
                if is_key_pressed(KeyCode::S) { refresh_servers(&mut g); g.screen = Screen::Servers; }
                if is_key_pressed(KeyCode::L) {
                    if g.account.is_some() {
                        g.account = None;
                        forget_account();
                        g.my_name = format!("Human-{}", rand::gen_range(10, 99));
                    } else {
                        g.login_email.clear(); g.login_pw.clear(); g.login_field = 0; g.status.clear();
                        while get_char_pressed().is_some() {}
                        g.screen = Screen::Login;
                    }
                }
                if is_key_pressed(KeyCode::J) {
                    if g.account.is_some() { g.room_input.clear(); while get_char_pressed().is_some() {} g.screen = Screen::JoinRoom; }
                    else { g.login_email.clear(); g.login_pw.clear(); g.login_field = 0; g.status = "Online play needs an account: log in first.".into(); while get_char_pressed().is_some() {} g.screen = Screen::Login; }
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
            Screen::Login => {
                draw_text("Log in to Hollow Hours", 60.0, 120.0, 40.0, WHITE);
                let stars = if g.show_pw { g.login_pw.clone() } else { "*".repeat(g.login_pw.chars().count()) };
                for (i, (label, val)) in [("Email", g.login_email.clone()), ("Password", stars.clone())].iter().enumerate() {
                    let col = if g.login_field == i { YELLOW } else { Color::new(0.8, 0.82, 0.9, 1.0) };
                    draw_text(&format!("{label}: {val}{}", if g.login_field == i { "_" } else { "" }), 60.0, 200.0 + i as f32 * 40.0, 26.0, col);
                }
                // Show / Hide the password: a button to click, or F1.
                let pw_w = measure_text(&format!("Password: {stars}_"), None, 26, 1.0).width;
                let (bx, by, bw, bh) = (60.0 + pw_w + 16.0, 222.0, 64.0, 26.0);
                draw_rectangle(bx, by, bw, bh, Color::new(0.2, 0.2, 0.26, 1.0));
                draw_text(if g.show_pw { "Hide" } else { "Show" }, bx + 12.0, by + 19.0, 20.0, WHITE);
                let (mx, my) = virtual_mouse();
                if (is_mouse_button_pressed(MouseButton::Left) && mx >= bx && mx <= bx + bw && my >= by && my <= by + bh) || is_key_pressed(KeyCode::F1) {
                    g.show_pw = !g.show_pw;
                }
                draw_text("Or press B: log in with your browser, or paste a code from the website (Cmd+V)", 60.0, 290.0, 22.0, Color::new(0.55, 0.85, 1.0, 1.0));
                for (i, l) in ["Tab = next field · Cmd+V paste · F1 show password · Enter = log in · Esc = back", &format!("No account? Make one at {}/register", site()), &g.status.clone()].iter().enumerate() {
                    draw_text(l, 60.0, 310.0 + i as f32 * 30.0, 20.0, Color::new(0.7, 0.72, 0.8, 1.0));
                }
                for ch in typed() {
                    if ch.is_control() { continue; }
                    let f = if g.login_field == 0 { &mut g.login_email } else { &mut g.login_pw };
                    if f.len() < 128 { f.push(ch); }
                }
                if let Some(t) = pasted() {
                    // A login code copied from the website's "Open Hollow Hours" page: use it straight away.
                    if t.len() == 48 && t.chars().all(|c| c.is_ascii_hexdigit()) && g.browser_login.is_none() {
                        let _ = std::fs::write(pending_login_file(), &t);
                        if let Some(rx) = take_pending_login() { g.browser_login = Some(rx); g.status = "Logging in with the code...".into(); }
                    } else {
                        let f = if g.login_field == 0 { &mut g.login_email } else { &mut g.login_pw };
                        f.extend(t.chars().take(128usize.saturating_sub(f.chars().count())));
                    }
                }
                if is_key_pressed(KeyCode::Backspace) { if g.login_field == 0 { g.login_email.pop(); } else { g.login_pw.pop(); } }
                if is_key_pressed(KeyCode::Tab) { g.login_field = 1 - g.login_field; }
                // Browser login: B is a letter, so only when the boxes are empty (or with Cmd/Ctrl held).
                if is_key_pressed(KeyCode::B) && g.browser_login.is_none() && (cmd_held() || (g.login_email.is_empty() && g.login_pw.is_empty())) {
                    g.login_email.clear();
                    g.browser_login = Some(start_browser_login());
                    g.status = "Waiting for your browser... click Allow there.".into();
                }

                if is_key_pressed(KeyCode::Escape) { g.login_pw.clear(); g.screen = Screen::Title; }
                if is_key_pressed(KeyCode::Enter) {
                    let body = json!({"email": g.login_email.trim(), "password": g.login_pw});
                    let res = ureq::post(&format!("{}/api/login", site())).timeout(std::time::Duration::from_secs(10)).send_json(body);
                    g.login_pw.clear(); // never kept
                    g.show_pw = false;
                    match res.ok().and_then(|r| r.into_json::<serde_json::Value>().ok()) {
                        Some(v) if v["token"].is_string() => {
                            let a = Account { token: v["token"].as_str().unwrap().into(), name: v["name"].as_str().unwrap_or("Human").into() };
                            save_account(&a);
                            g.my_name = a.name.clone();
                            g.account = Some(a);
                            g.screen = Screen::Title;
                        }
                        _ => g.status = "Wrong email or password (or the website can't be reached).".into(),
                    }
                }
            }
            Screen::Servers => {
                draw_text("Servers", 60.0, 100.0, 44.0, WHITE);
                if g.server_list.is_empty() { draw_text("No public servers right now. N = make one.", 60.0, 160.0, 22.0, Color::new(0.7, 0.72, 0.8, 1.0)); }
                for (i, sv) in g.server_list.iter().take(8).enumerate() {
                    let mods: Vec<&str> = sv["mods"].as_array().map(|a| a.iter().filter_map(|m| m["name"].as_str()).collect()).unwrap_or_default();
                    draw_text(&format!("{}  {}{}   {}/{}   host {}{}", i + 1, sv["name"].as_str().unwrap_or("?"),
                        if sv["external"].as_bool() == Some(true) { " (own server)" } else { "" }, sv["players"], sv["max"], sv["host"].as_str().unwrap_or("?"),
                        if mods.is_empty() { String::new() } else { format!("   mods: {}", mods.join(", ")) }), 60.0, 160.0 + i as f32 * 34.0, 22.0, Color::new(0.85, 0.87, 0.95, 1.0));
                }
                draw_text(&format!("1-8 join · R refresh · N make a server{} · Esc back", if g.account.is_none() { " (log in first: L)" } else { "" }), 60.0, 460.0, 20.0, Color::new(0.65, 0.68, 0.78, 1.0));
                draw_text(&g.status.clone(), 60.0, 495.0, 20.0, Color::new(1.0, 0.75, 0.4, 1.0));
                let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4, KeyCode::Key5, KeyCode::Key6, KeyCode::Key7, KeyCode::Key8];
                for (i, k) in keys.iter().enumerate() {
                    if is_key_pressed(*k) {
                        if g.account.is_none() { g.status = "Log in first (Esc, then L): servers check who you are.".into(); continue; }
                        if let Some(sv) = g.server_list.get(i).cloned() {
                            let mods: Vec<String> = sv["mods"].as_array().cloned().unwrap_or_default().iter().filter_map(|m| m["id"].as_str().map(String::from)).collect();
                            apply_mods(&mut g, &mods);
                            let token = g.account.as_ref().map(|a| a.token.clone()).unwrap_or_default();
                            let code = sv["code"].as_str().unwrap_or("").to_string();
                            if let Some(addr) = sv["address"].as_str() {
                                // Hosted somewhere else: our login never goes there, only a 60-second ticket for that address.
                                let ticket = ureq::post(&format!("{}/api/ticket", site())).set("Authorization", &format!("Bearer {token}"))
                                    .timeout(std::time::Duration::from_secs(8)).send_json(json!({"address": addr})).ok()
                                    .and_then(|r| r.into_json::<serde_json::Value>().ok()).and_then(|v| v["ticket"].as_str().map(String::from));
                                match ticket {
                                    Some(t) => g.net = Some(net::Net::join_at(addr, &code, &format!("Ticket {t}"))),
                                    None => { g.status = "Couldn't get a ticket for that server (log in again?)".into(); continue; }
                                }
                            } else {
                                g.net = Some(net::Net::join(&code, &token));
                            }
                            start_infection(&mut g);
                        }
                    }
                }
                if is_key_pressed(KeyCode::R) { refresh_servers(&mut g); }
                if is_key_pressed(KeyCode::N) {
                    if g.account.is_some() { g.room_input = format!("{}'s server", g.my_name); while get_char_pressed().is_some() {} g.screen = Screen::MakeServer; }
                    else { g.status = "Log in first (L on the title screen).".into(); }
                }
                if is_key_pressed(KeyCode::Escape) { g.screen = Screen::Title; }
            }
            Screen::MakeServer => {
                draw_text("Make a server", 60.0, 110.0, 40.0, WHITE);
                draw_text(&format!("Name: {}_", g.room_input), 60.0, 190.0, 30.0, YELLOW);
                draw_text("Enter = make it and join · Esc = back   (add mods to a server on the website)", 60.0, 250.0, 20.0, Color::new(0.7, 0.72, 0.8, 1.0));
                draw_text(&g.status.clone(), 60.0, 290.0, 20.0, Color::new(1.0, 0.75, 0.4, 1.0));
                let mut chars = typed();
                if let Some(t) = pasted() { chars.extend(t.chars()); }
                for ch in chars { if !ch.is_control() && g.room_input.chars().count() < 32 { g.room_input.push(ch); } }
                if is_key_pressed(KeyCode::Backspace) { g.room_input.pop(); }
                if is_key_pressed(KeyCode::Escape) { g.screen = Screen::Servers; }
                if is_key_pressed(KeyCode::Enter) {
                    let token = g.account.as_ref().map(|a| a.token.clone()).unwrap_or_default();
                    let res = ureq::post(&format!("{}/api/servers", site())).set("Authorization", &format!("Bearer {token}"))
                        .timeout(std::time::Duration::from_secs(10)).send_json(json!({"name": g.room_input.trim(), "max": 8, "public": true}));
                    match res {
                        Ok(r) => {
                            let v: serde_json::Value = r.into_json().unwrap_or_default();
                            if let Some(code) = v["code"].as_str() {
                                apply_mods(&mut g, &[]);
                                g.net = Some(net::Net::join(code, &token));
                                start_infection(&mut g);
                                g.log.push(format!("Your server is up. Room code: {code}"));
                            }
                        }
                        Err(ureq::Error::Status(_, r)) => g.status = r.into_json::<serde_json::Value>().ok().and_then(|v| v["error"].as_str().map(String::from)).unwrap_or("Couldn't make it.".into()),
                        Err(_) => g.status = "The website can't be reached.".into(),
                    }
                }
            }
            Screen::JoinRoom => {
                draw_text("Multiplayer: Infection with friends", 60.0, 120.0, 40.0, Color::new(0.45, 0.95, 0.4, 1.0));
                for (i, l) in ["Type a room code (4-8 letters or numbers) and press Enter.", "Everyone who types the same code plays together. The first one in is the host.",
                    "", &format!("Room: {}_", g.room_input), "", "Esc = back"].iter().enumerate() {
                    draw_text(l, 60.0, 190.0 + i as f32 * 32.0, if i == 3 { 34.0 } else { 22.0 }, Color::new(0.82, 0.84, 0.9, 1.0));
                }
                let mut chars = typed();
                if let Some(t) = pasted() { chars.extend(t.chars()); }
                for ch in chars {
                    if ch.is_ascii_alphanumeric() && g.room_input.len() < 8 { g.room_input.push(ch.to_ascii_uppercase()); }
                }
                if is_key_pressed(KeyCode::Backspace) { g.room_input.pop(); }
                if is_key_pressed(KeyCode::Enter) && g.room_input.len() >= 4 {
                    // The room tells us its mods; look them up on the website first so everyone plays the same game.
                    let code = g.room_input.clone();
                    let mods: Vec<String> = refresh_servers_quiet(&mut g).into_iter().find(|s| s["code"] == code.as_str())
                        .map(|s| s["mods"].as_array().cloned().unwrap_or_default().iter().filter_map(|m| m["id"].as_str().map(String::from)).collect())
                        .unwrap_or_default();
                    apply_mods(&mut g, &mods);
                    let code = g.room_input.clone();
                    begin_intro(&mut g, Some(code));
                }
                if is_key_pressed(KeyCode::Escape) { g.screen = Screen::Title; }
            }
            Screen::Consent => {
                draw_text("Hollow Hours wants to reach outside its window", 50.0, 120.0, 34.0, Color::new(0.85, 0.3, 0.35, 1.0));
                for (i, l) in [
                    "This update can scare you OUTSIDE the game:",
                    "  - create files in a \"Hollow Hours\" folder on your Desktop. They STAY, even after you quit.",
                    "  - open Terminal to show you a message (it only prints text)",
                    "  - switch your window to TextEdit or Finder to show you its files",
                    "",
                    "It only ever creates its OWN new files. It never changes, moves or deletes yours.",
                    "P (panic) stops all of this at once.",
                    "",
                    "Y = allow        N = keep everything inside the game",
                ].iter().enumerate() {
                    draw_text(l, 50.0, 180.0 + i as f32 * 30.0, 22.0, Color::new(0.82, 0.84, 0.9, 1.0));
                }
                if is_key_pressed(KeyCode::Y) { g.meta.allowed = Some(true); g.meta.save(); g.screen = Screen::Title; }
                if is_key_pressed(KeyCode::N) { g.meta.allowed = Some(false); g.meta.save(); g.screen = Screen::Title; }
            }
            Screen::Intro => {
                g.intro_t += dt;
                let done = draw_intro(&mut g);
                if done || is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) { finish_intro(&mut g); }
                if is_key_pressed(KeyCode::Escape) { g.intro_join = None; g.screen = Screen::Title; }
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
                if is_key_pressed(KeyCode::Enter) { g.next_crazy = 90.0; g.screen = Screen::Play; g.sfx.start_hum(); }
                if is_key_pressed(KeyCode::Escape) { break; }
            }
            Screen::Mac => {
                mac::screen(&mut g);
            }
            Screen::Play | Screen::Typing(_) => {
                update(&mut g, dt);
                if g.screen == Screen::Play && is_key_pressed(KeyCode::Tab) { g.view3d = !g.view3d; }
                // Looking around with the mouse: click to capture it, Esc lets go.
                if g.view3d && g.screen == Screen::Play && !g.grabbed && is_mouse_button_pressed(MouseButton::Left) {
                    g.grabbed = true;
                    set_cursor_grab(true);
                    show_mouse(false);
                    g.last_mouse = Vec2::from(mouse_position());
                }
                if g.grabbed {
                    let m = Vec2::from(mouse_position());
                    let dm = m - g.last_mouse;
                    g.last_mouse = m;
                    g.yaw += dm.x * 0.0035;
                    g.pitch = (g.pitch - dm.y * 0.0035).clamp(-1.1, 1.1);
                }
                if g.view3d {
                    view3d::render(&g, &view);
                    set_camera(&cam);
                }
                draw_world(&g, &light, &view);
                if let Screen::Typing(i) = g.screen {
                    typing(&mut g, i);
                }
                if is_key_pressed(KeyCode::Escape) && g.screen == Screen::Play && g.grabbed {
                    g.grabbed = false;
                    set_cursor_grab(false);
                    show_mouse(true);
                } else if is_key_pressed(KeyCode::Escape) && g.screen == Screen::Play {
                    g.net = None;
                    g.remotes.clear();
                    g.vest.play(haptics::Effect::StopAll);
                    g.screen = Screen::Title;
                }
            }
        }
        // Show the canvas, scaled to fit the window (black bars around it).
        set_default_camera();
        clear_background(BLACK);
        let (s, ox, oy) = canvas_fit();
        draw_texture_ex(&canvas.texture, ox, oy, WHITE, DrawTextureParams { dest_size: Some(vec2(vw() * s, VH * s)), flip_y: true, ..Default::default() });
        if !g.shots.is_empty() {
            let path = std::env::temp_dir().join("hollow-hours-shot.png");
            get_screen_data().export_png(&path.to_string_lossy());
            for tx in g.shots.drain(..) { let _ = tx.send(json!({"ok": true, "path": path.to_string_lossy()})); }
        }
        next_frame().await;
    }
    g.vest.play(haptics::Effect::StopAll);
}

fn cmd_held() -> bool {
    is_key_down(KeyCode::LeftSuper) || is_key_down(KeyCode::RightSuper) || is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl)
}

/// ⌘V (Ctrl+V elsewhere): the clipboard as one line of text, without control characters.
fn pasted() -> Option<String> {
    if !(cmd_held() && is_key_pressed(KeyCode::V)) { return None; }
    let t = macroquad::miniquad::window::clipboard_get()?;
    let t: String = t.chars().map(|c| if c == '\n' || c == '\t' { ' ' } else { c }).filter(|c| !c.is_control()).collect();
    Some(t.trim().to_string())
}

/// Characters typed this frame, leaving out the ones typed while ⌘/Ctrl is held (shortcuts, not text).
fn typed() -> Vec<char> {
    let mut v = vec![];
    while let Some(ch) = get_char_pressed() { v.push(ch); }
    if cmd_held() { v.clear(); }
    v
}

fn typing(g: &mut Game, i: usize) {
    for ch in typed() {
        if !ch.is_control() && g.input.chars().count() < 120 {
            g.input.push(ch);
        }
    }
    if let Some(t) = pasted() {
        let room = 120usize.saturating_sub(g.input.chars().count());
        g.input.extend(t.chars().take(room));
    }
    if is_key_pressed(KeyCode::Backspace) { g.input.pop(); }
    let w = VW;
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
        if r.who == mac::WHO { mac::reply(g, r.text, r.via); continue; }
        let Some(c) = g.chars.get_mut(r.who) else { continue };
        c.thinking = false;
        c.history.push(ai::Msg { user: false, text: r.text.clone() });
        c.bubble = r.text;
        c.via = r.via;
        c.bubble_t = 9.0 + c.bubble.len() as f32 / 25.0;
    }

    if g.net.is_some() {
        network(g, dt);
    }
    if g.infection {
        update_infection(g, dt);
        if g.screen != Screen::Play && !matches!(g.screen, Screen::Typing(_)) { return; }
    }
    // Player movement (not while typing, hiding, or once you've lost control). Shift = sneak: slow and silent.
    g.sneaking = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
    g.noise = (g.noise - dt * 1.5).max(0.0);
    g.ai_t += dt;
    if g.screen == Screen::Play && !g.hidden && !g.lost_control {
        let mut d = Vec2::ZERO;
        if g.view3d {
            // First person: W/S forward and back, A/D step sideways, arrows or the mouse turn.
            let fwd = vec2(g.yaw.cos(), g.yaw.sin());
            let right = vec2(-fwd.y, fwd.x);
            if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) { d += fwd; }
            if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) { d -= fwd; }
            if is_key_down(KeyCode::A) { d -= right; }
            if is_key_down(KeyCode::D) { d += right; }
            if is_key_down(KeyCode::Left) { g.yaw -= 2.4 * dt; }
            if is_key_down(KeyCode::Right) { g.yaw += 2.4 * dt; }
        } else {
            if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) { d.y -= 1.0; }
            if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) { d.y += 1.0; }
            if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) { d.x -= 1.0; }
            if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) { d.x += 1.0; }
        }
        // Claude's move (MCP), only when you aren't pressing anything: your keys always win.
        if d == Vec2::ZERO {
            if let Some(a) = &mut g.ai {
                a.t -= dt;
                if let Some(t) = a.goto {
                    let to = t * T - g.pos;
                    if to.length() < 0.35 * T { a.t = 0.0; } else { d = to.normalize(); }
                } else {
                    d = a.dir;
                }
                if a.sneak { g.sneaking = true; }
                if a.t <= 0.0 { g.ai = None; d = Vec2::ZERO; }
            }
        } else {
            g.ai = None;
        }
        if d != Vec2::ZERO {
            let speed = if g.sneaking { g.rules.sneak } else { g.rules.player_speed };
            if !g.sneaking { g.noise = 1.0; }
            let step = d.normalize() * speed * T * dt;
            let r = 10.0;
            let nx = g.pos.x + step.x;
            if !solid(g.map, nx + r * step.x.signum(), g.pos.y) { g.pos.x = nx; }
            let ny = g.pos.y + step.y;
            if !solid(g.map, g.pos.x, ny + r * step.y.signum()) { g.pos.y = ny; }
        }
        // Footsteps: loud walking, soft sneaking.
        g.step_t -= dt;
        if d != Vec2::ZERO && g.step_t <= 0.0 {
            g.sfx.step(!g.sneaking);
            g.step_t = if g.sneaking { 0.55 } else { 0.32 };
        }
        let here = tile(g.map, (g.pos.x / T) as i32, (g.pos.y / T) as i32);
        if let Some((m, at)) = door(here, g.map) {
            g.map = m;
            g.pos = at * T;
            // Walk in facing the middle of the new place.
            let to = vec2(MW as f32 / 2.0, MH as f32 / 2.0) - at;
            g.yaw = to.y.atan2(to.x);
            g.log.push(format!("You walk into {}.", map_name(m)));
            if map_name(m) == "The Marketplace" && g.infection {
                g.log.push("Fluorescent lights hum. Somewhere a robot voice says: \"Welcome to Helix Market.\"".into());
            }
        }
        // Talk: E next to someone (or sit down at the Mac Studio).
        if is_key_pressed(KeyCode::E) && nearest(g, 2.2).is_none() && near_mac(g) {
            g.screen = Screen::Mac;
            while get_char_pressed().is_some() {}
        } else if is_key_pressed(KeyCode::E) {
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
    let is_guest = guest(g);
    let infection_on = g.infection;
    let infected_speed = g.rules.infected_speed;
    for c in &mut g.chars {
        if c.bubble_t > 0.0 { c.bubble_t -= dt; }
        if is_guest { continue; } // the host moves everyone
        let speed = if c.crazy > 0.0 { if c.infected { infected_speed } else { 1.9 } } else if c.kind { if c.task == roam::Task::Roam { 1.7 } else { 2.6 } } else { 1.0 } * T;
        if c.crazy > 0.0 && (c.map == g.map || infection_on) {
            c.target = if infection_on { c.chase } else { g.pos };
            c.crazy -= dt;
        } else if !c.kind && (c.pos.distance(c.target) < 4.0 || rand::gen_range(0.0, 1.0) < 0.002) {
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
    if g.scare_t > g.prev_scare + 0.2 { g.sfx.sting(); }
    if g.scare_t > 0.0 { g.scare_t -= dt; }
    g.prev_scare = g.scare_t;
    if g.map != g.prev_map { g.sfx.door(); g.prev_map = g.map; }
    // Growls: something hunting close by.
    g.growl_t -= dt;
    let hunter = g.chars.iter().filter(|c| c.map == g.map && c.crazy > 0.0 && (c.infected || !g.infection))
        .map(|c| 1.0 - (c.pos.distance(g.pos) / (8.0 * T)).min(1.0)).fold(0.0f32, f32::max);
    if hunter > 0.05 && g.growl_t <= 0.0 {
        g.sfx.growl(hunter);
        g.growl_t = rand::gen_range(1.6, 3.2);
    }
    let tornado_here = g.tornado.as_ref().is_some_and(|t| t.warn <= 0.0);
    g.sfx.wind(tornado_here);
    // M = sound on/off.
    if is_key_pressed(KeyCode::M) && g.screen == Screen::Play {
        g.sfx.on = !g.sfx.on;
        if g.sfx.on { g.sfx.start_hum(); } else { g.sfx.silence(); }
        g.log.push(format!("Sound {}.", if g.sfx.on { "on" } else { "off" }));
    }

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
    let dark = if g.on(0) { map_dark(g.map) } else { 0.1 };
    let crazy_near = g.chars.iter().filter(|c| c.crazy > 0.0 && c.map == g.map).map(|c| 1.0 - (c.pos.distance(g.pos) / (10.0 * T)).min(1.0)).fold(0.0f32, f32::max);
    let water_near = g.on(1) && map_has_water(g.map);
    let fear_target = (dark * 0.45 + crazy_near * 0.7 + if water_near { 0.1 } else { 0.0 }).min(1.0) * if g.courage == Courage::Brave { 0.4 } else { 1.0 };
    g.fear += (fear_target - g.fear) * dt * 0.4;
    let closest = g.chars.iter().filter(|c| c.map == g.map).map(|c| c.pos.distance(g.pos)).fold(f32::MAX, f32::min);
    let disc_target: f32 = if crowds_on && closest < 1.4 * T { 0.7 } else { 0.0 } + if g.on(2) && near_doll(g) { 0.35 } else { 0.0 };
    g.discomfort += (disc_target.min(1.0) - g.discomfort) * dt * 0.5;
    let lim_target = if g.on(5) { map_liminal(g.map) } else { map_liminal(g.map) * 0.3 };
    g.liminality += (lim_target - g.liminality) * dt * 0.3;
    // Heartbeat on the vest when you're very scared.
    g.beat_t -= dt;
    if g.fear > 0.7 && g.beat_t <= 0.0 {
        g.vest.play(haptics::Effect::Heartbeat);
        g.sfx.heart(g.fear);
        g.beat_t = 1.1;
    }
    // A tense warm pulse when something feels very wrong.
    if g.discomfort > 0.6 && g.beat_t <= 0.0 {
        g.vest.play(haptics::Effect::Tense);
        g.beat_t = 2.5;
    }
    if g.log.len() > 4 { g.log.remove(0); }
}

fn near_mac(g: &Game) -> bool {
    let (px, py) = ((g.pos.x / T) as i32, (g.pos.y / T) as i32);
    (-1..=1).any(|dy| (-1..=1).any(|dx| tile(g.map, px + dx, py + dy) == 'm'))
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

fn draw_world(g: &Game, light: &Texture2D, view: &RenderTarget) {
    let (floor, wall, m_name, drained) = (map_floor(g.map), map_wall(g.map), map_name(g.map), map_drained(g.map));
    let shake = if g.fear > 0.75 { rand::gen_range(-1.5, 1.5) } else { 0.0 };
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
        if g.screen == Screen::Play && !g.grabbed {
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
            let body = if c.infected { Color::new(c.color.r * 0.45, c.color.g * 0.6 + 0.2, c.color.b * 0.45, 1.0) } else { c.color };
            draw_circle(c.pos.x + shake, c.pos.y + shake, 12.0, body);
            let eye = if c.infected { if c.stun > 0.0 { GRAY } else { g.rules.eye } } else if c.crazy > 0.0 { RED } else { BLACK };
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
        let (lx, ly) = (g.pos.x - radius, g.pos.y - radius);
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
            draw_text("YOU ARE INFECTED: you can see, but you can't move. It's walking toward the Claudes.", 16.0, 28.0, 20.0, Color::new(0.4, 1.0, 0.35, 1.0));
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

    // Speech bubbles (drawn over the dark so you can read them).
    for c in g.chars.iter().filter(|c| c.map == g.map && c.bubble_t > 0.0) {
        if let Some(p) = at(c.pos, 1.3) { bubble(&c.bubble, &c.via, p.x, p.y); }
    }
    let prompt = if let Some(i) = nearest(g, 2.2) { Some(format!("E: talk to {}", g.chars[i].name)) }
        else if near_mac(g) { Some("E: use the Mac Studio".to_string()) } else { None };
    if let (Some(t), Screen::Play) = (prompt, g.screen) {
        if g.view3d { draw_text(&t, sw / 2.0 - measure_text(&t, None, 20, 1.0).width / 2.0, sh / 2.0 + 70.0, 20.0, YELLOW); }
        else { draw_text(&t, g.pos.x - 40.0, g.pos.y - 22.0, 16.0, YELLOW); }
    }

    if g.ai_t < 6.0 {
        let t = "Claude is playing  (your keys always win  ·  P = panic)";
        draw_rectangle(sw - measure_text(t, None, 16, 1.0).width - 24.0, 6.0, measure_text(t, None, 16, 1.0).width + 16.0, 24.0, Color::new(0.85, 0.47, 0.32, 0.9));
        draw_text(t, sw - measure_text(t, None, 16, 1.0).width - 16.0, 23.0, 16.0, WHITE);
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
    draw_rectangle(-margin(), y0, vw(), HUD, g.rules.hud);
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
    draw_text(if g.infection { if g.view3d { "WASD move · mouse/arrows look · Shift sneak · H hide · E talk · Tab 2D · P PANIC" } else { "WASD move · Shift sneak · H hide · E talk · Tab 3D · P PANIC · Esc" } } else { "WASD move · E talk · P PANIC (everything off) · Esc menu" },
        sw - 380.0, y0 + 72.0, 14.0, Color::new(0.5, 0.52, 0.6, 1.0));
    if g.vest.enabled { draw_text("vest ON", sw - 70.0, y0 + 24.0, 16.0, Color::new(0.4, 0.9, 0.5, 1.0)); }
}

fn draw_title(g: &Game) {
    draw_text(&g.rules.title, 60.0, 78.0, 56.0, Color::new(0.85, 0.3, 0.35, 1.0));
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
        "  I  watch the story (ASTRA 9.1, one year later)      M in game = sound on/off".into(),
        "  J  INFECTION ONLINE: play with friends (type the same room code)   S  server list   (needs a login)".into(),
        match &g.account { Some(a) => format!("  Logged in as {} (L to log out)", a.name), None => "  L  log in (make an account on the website)".into() },
        String::new(),
        format!("Haptic vest: {}", if !g.vest.found { "none found".to_string() } else if g.vest.enabled { format!("ON at {}% (V off, [ ] strength)", (g.vest.strength * 100.0) as i32) } else { "found, OFF (V to turn on)".into() }),
        "Vest effects are mild and capped. Don't use them if you have a heart condition.".into(),
        "No flashing lights. P = panic at any time: everything stops.   F11 (or Cmd+Ctrl+F) = full screen".into(),
        format!("Outside the window: {}   {}", match g.meta.allowed { Some(true) => "ALLOWED (files, Terminal, window switching)", Some(false) => "off", None => "not asked" },
            if g.meta.created.is_empty() { "M to change".to_string() } else { format!("M to change · {} files on your Desktop", g.meta.created.len()) }),
    ];
    // Spacing shrinks a little if there's a lot to show, so everything always fits on screen.
    let step = 25.0;
    for (i, l) in lines.iter().enumerate() {
        draw_text(l, 60.0, 118.0 + i as f32 * step, 20.0, Color::new(0.82, 0.84, 0.9, 1.0));
    }
    // Who's who: our own characters, each only inspired by a kind of AI. Three columns, cut to fit.
    let y = 118.0 + lines.len() as f32 * step + 4.0;
    draw_text("Who lives here:", 60.0, y, 19.0, WHITE);
    let cols = 3usize;
    let colw = (VW - 80.0) / cols as f32;
    let rows = g.chars.len().div_ceil(cols).max(1) as f32;
    let rstep = ((VH - 10.0 - (y + 8.0)) / rows).min(22.0);
    for (i, c) in g.chars.iter().enumerate() {
        let (col, row) = ((i % cols) as f32, (i / cols) as f32);
        let (x, yy) = (60.0 + col * colw, y + 8.0 + row * rstep);
        draw_circle(x + 6.0, yy + 8.0, 6.0, c.color);
        let mut t = format!("{}: {}", c.name, c.inspired);
        while t.len() > 4 && measure_text(&t, None, 15, 1.0).width > colw - 24.0 {
            t.pop();
            while !t.is_char_boundary(t.len()) { t.pop(); }
        }
        if t.len() < c.name.len() + 2 + c.inspired.len() { t = format!("{}...", t.trim_end()); }
        draw_text(&t, x + 18.0, yy + 13.0, 15.0, Color::new(0.7, 0.72, 0.8, 1.0));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs the website running locally (cd server && npx wrangler dev --port 8799) with a mod uploaded; skips otherwise.
    #[test]
    fn a_mod_downloads_and_becomes_a_character() {
        let Ok(id) = std::env::var("HH_TEST_MOD") else { return };
        let home = std::env::temp_dir().join(format!("hh-mod-{}", std::process::id()));
        unsafe { std::env::set_var("HOME", &home); std::env::set_var("HOLLOW_SERVER", "ws://127.0.0.1:8799"); }
        let m = load_mod(&id).expect("downloaded the mod");
        assert!(account_file().with_file_name("mods").join(format!("{id}.json")).exists(), "cached on disk");
        let c = mod_character(&m["content"]["characters"][0]).expect("a real character");
        println!("mod {} -> character {} (infected: {}) at map {}", m["name"], c.name, c.infected, c.map);
        assert!(load_mod("../../etc").is_none(), "only real mod ids");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A total-conversion .hh as a local mod: its own maps replace the built-in ones and the doors link them.
    #[test]
    fn night_mall_maps_and_doors() {
        let home = std::env::temp_dir().join(format!("hh-local-{}", std::process::id()));
        let local = home.join("Library/Application Support/Hollow Hours/mods/local");
        std::fs::create_dir_all(&local).unwrap();
        std::fs::copy("mods/NightMall.hh", local.join("NightMall.hh")).unwrap();
        unsafe { std::env::set_var("HOME", &home); }
        let mods = local_mods();
        assert_eq!(mods.len(), 1, "read the .hh zip");
        let list = mods[0]["content"]["maps"].as_array().unwrap().clone();
        let maps: Vec<MapData> = list.iter().filter_map(|v| mod_map(v, list.len())).collect();
        assert_eq!(maps.len(), 2, "both maps are valid");
        set_maps(maps);
        assert_eq!((maps_len(), map_name(0)), (2, "The night mall".to_string()));
        assert_eq!(door('G', 0).map(|d| d.0), Some(1), "the mall's G door leads to the garage");
        assert_eq!(door('M', 1).map(|d| d.0), Some(0), "and back");
        let mut bad = list[0].clone();
        bad["rows"][3] = serde_json::json!("too short");
        assert!(mod_map(&bad, 2).is_none(), "a broken map is refused");
        set_maps(vec![]);
        assert_eq!(maps_len(), 4, "no mod maps: back to the built-in four");
        assert_eq!(door('M', 1).map(|d| d.0), Some(3), "the beach's south door leads to the Marketplace");
        assert_eq!(door('M', 3).map(|d| d.0), Some(1), "and back to the beach");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The Keychain keeps the login (a separate test entry, removed again at the end).
    #[test]
    fn keychain_remembers_the_login() {
        let e = keyring::Entry::new("Hollow Hours (test)", "account").unwrap();
        let a = Account { token: "ab".repeat(32), name: "Tester".into() };
        e.set_password(&serde_json::to_string(&a).unwrap()).expect("saved in the Keychain");
        let back: Account = serde_json::from_str(&e.get_password().unwrap()).unwrap();
        assert_eq!((back.token.len(), back.name.as_str()), (64, "Tester"));
        e.delete_credential().unwrap();
        assert!(e.get_password().is_err(), "forgotten after log out");
    }
}
