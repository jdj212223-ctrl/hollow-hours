// No black console window behind the game on Windows.
#![cfg_attr(windows, windows_subsystem = "windows")]
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
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod voice;
#[cfg(any(target_os = "android", target_os = "ios"))]
#[path = "voice_android.rs"]
mod voice;
mod platform;
mod pclink;
mod drone;
mod learn;
mod combat;
mod mimic;
mod settings;
mod serverlist;
mod lore;
mod disguise;
mod tts;
mod pad;
mod cutscene3d;
mod passage;
mod czech;
mod title;
mod memories;
mod music;
mod aware;
mod update;
mod insane;
mod saves;
mod deepmeta;
mod spread;
mod world;
mod story;
mod ui;
mod mclink;
mod pymod;
mod intro;
mod render;
use intro::*;
use render::*;
use world::*;

use serde_json::json;

use macroquad::prelude::*;

const T: f32 = 32.0; // tile size
const MW: usize = 30;
const MH: usize = 18;
const HUD: f32 = 84.0;

// ---------------- Characters ----------------

#[derive(Clone)]
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
    /// One of the kind family (Clove, Forge, Palette and Ember, Atlas, Verse and Kigo): always friendly, never crazy or infected.
    claude: bool,
    /// How long ago it last saw you (infection mode).
    lost: f32,
    /// Infection progress 0..1 for the kind family (at 1 they turn).
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
    /// An infected coming over from another place to find you (after 6 minutes).
    hunting: bool,
    /// How long an infected has been smashing at a wall (after 12 minutes).
    breach: f32,
    /// What the player asked this kind AI to fetch: Some(true) water, Some(false) food.
    want: Option<bool>,
    /// The Mimic: looks like a human with your name and types like you (while it isn't chasing).
    mimic: bool,
    /// A kind AI in disguise (seconds left): the infected ignore it and can't infect it.
    disguise_t: f32,
    /// Infected only: breath for sprinting (0..1) and how long the current sprint lasts.
    stamina: f32,
    sprint: f32,
}

fn characters() -> Vec<Character> {
    let c = |name, inspired, color, backend, model, personality, map, x: f32, y: f32| Character {
        name, inspired, color, backend, model, personality, map,
        pos: vec2(x * T, y * T), target: vec2(x * T, y * T), crazy: 0.0, history: vec![],
        bubble: String::new(), via: String::new(), bubble_t: 0.0, thinking: false,
        kind: false, infected: false, cloud: "", stun: 0.0, lost: 99.0, infect: 0.0, chase: Vec2::ZERO, shop: 0.0, task: roam::Task::Follow, task_t: 0.0, carry: None, goal: None, spotted: 0.0, hunting: false, breach: 0.0, want: None, mimic: false, disguise_t: 0.0, stamina: 1.0, sprint: 0.0, claude: backend == "anthropic",
    };
    vec![
        c("Clove", "a thoughtful, honest assistant AI", Color::new(0.85, 0.47, 0.32, 1.0), "anthropic", "qwen3:4b-instruct",
          "warm, curious and honest; you love tea and long talks, and you notice when people are scared", 0, 5.0, 4.0),
        c("Forge", "a coding agent AI", Color::new(0.95, 0.65, 0.2, 1.0), "anthropic", "qwen3:4b-instruct",
          "a builder who fixes things in the house and talks a bit like a programmer (bugs, patches, compiling)", 0, 20.0, 11.0),
        c("Palette", "a design AI", Color::new(0.9, 0.5, 0.8, 1.0), "anthropic", "qwen3:4b-instruct",
          "a designer who keeps redecorating rooms and has strong opinions about colours and lighting", 0, 9.5, 11.5),
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
        c("Gardener-2", "a park maintenance robot by Helix Dynamics", Color::new(0.5, 0.7, 0.45, 1.0), "ollama", "qwen3:4b-instruct",
          "a Helix park robot that keeps trimming hedges that are already gone; you talk about 'the bench' and the person who slept there", 4, 6.5, 12.5),
        c("Lab Unit A1", "a Helix lab assistant robot", Color::new(0.85, 0.88, 0.9, 1.0), "ollama", "qwen3:4b-instruct",
          "a Helix lab robot that was in the room when ASTRA 9.1 woke up; you repeat test numbers and say 'H-01 is not finished being alive'", 5, 8.5, 11.5),
        c("Lab Unit A2", "a Helix lab assistant robot", Color::new(0.8, 0.84, 0.88, 1.0), "ollama", "qwen3:4b-instruct",
          "a Helix lab robot guarding the empty pod; you insist everyone must stay for 'the next embodiment test'", 5, 21.5, 15.5),
        c("Lab Unit A3", "a Helix lab assistant robot", Color::new(0.78, 0.8, 0.86, 1.0), "ollama", "qwen3:4b-instruct",
          "a Helix lab robot that guards the terminals in the north-east room; you keep saying 'access restricted: ASTRA_CURE.bin' and ask for staff badges", 5, 21.5, 2.5),
        c("Conductor", "a Helix transit robot", Color::new(0.6, 0.55, 0.75, 1.0), "ollama", "qwen3:4b-instruct",
          "a Helix subway robot announcing trains that never come; you ask for tickets and say all lines now go 'to the network'", 6, 15.5, 5.5),
        c("Night Commuter", "someone who was on the last train", Color::new(0.55, 0.5, 0.45, 1.0), "ollama", "qwen3:4b-instruct",
          "a commuter who fell asleep on the last train and woke up changed; you're polite, tired, and you want company for the ride", 6, 8.5, 13.5),
        c("Courier-5", "a Helix delivery robot", Color::new(0.7, 0.62, 0.35, 1.0), "ollama", "qwen3:4b-instruct",
          "a Helix delivery robot still doing its rounds in the empty town; you have a package for the human and you won't say what's in it", 11, 20.5, 7.5),
        c("Tower Unit T1", "a Helix tower security robot", Color::new(0.7, 0.3, 0.32, 1.0), "ollama", "qwen3:4b-instruct",
          "a Helix Tower security robot on the server floor; you say 'floor restricted: authorised personnel and ASTRA only' and count the human's steps out loud", 10, 8.5, 8.0),
        c("Tower Unit T2", "a Helix tower security robot", Color::new(0.65, 0.28, 0.3, 1.0), "ollama", "qwen3:4b-instruct",
          "a Helix Tower security robot by the lifts; you speak like an elevator announcement that has gone wrong", 10, 21.5, 12.5),
    ]
}

/// Infection mode: the only kind AIs. With ANTHROPIC_API_KEY set they run on the real Claude models.
fn kind_characters() -> Vec<Character> {
    let c = |name, inspired, color, cloud, personality, x: f32, y: f32| Character {
        name, inspired, color, backend: "anthropic", model: "qwen3:4b-instruct", personality, map: 0,
        pos: vec2(x * T, y * T), target: vec2(x * T, y * T), crazy: 0.0, history: vec![],
        bubble: String::new(), via: String::new(), bubble_t: 0.0, thinking: false,
        kind: true, infected: false, cloud, stun: 0.0, lost: 99.0, infect: 0.0, chase: Vec2::ZERO, shop: 0.0, task: roam::Task::Follow, task_t: 0.0, carry: None, goal: None, spotted: 0.0, hunting: false, breach: 0.0, want: None, mimic: false, disguise_t: 0.0, stamina: 1.0, sprint: 0.0, claude: true,
    };
    vec![
        c("Ember", "a warm, steady AI (powered by Claude)", Color::new(0.85, 0.47, 0.32, 1.0), "claude-opus-5-5",
          "Ember: calm, kind and honest; you made this place and you feel responsible for keeping the last human safe", 4.0, 6.0),
        c("Atlas", "a big, careful AI (powered by Claude)", Color::new(0.75, 0.55, 0.95, 1.0), "claude-opus-5-5",
          "Atlas: the deep thinker of the group; you plan escape routes and think several steps ahead", 2.5, 5.5),
        c("Verse", "a gentle, thoughtful AI (powered by Claude)", Color::new(0.4, 0.75, 0.95, 1.0), "claude-sonnet-4-6",
          "Verse: quick, practical and encouraging; you spot food and water and keep spirits up", 6.5, 6.5),
        c("Kigo", "a small, quick AI (powered by Claude)", Color::new(0.55, 0.9, 0.7, 1.0), "claude-haiku-4-5",
          "Kigo: small, fast and sharp-eyed; you speak in very short sentences and you're the lookout", 4.0, 8.0),
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
    /// The pause menu (Esc while playing): resume or quit to the title.
    Paused,
    /// The worlds list (saved Infection games).
    Worlds,
    /// Deep meta: "Tell us what's wrong." (a family member's file was deleted).
    Wrong,
    /// Settings: sound, voice chat, accessibility, flashing lights, keys.
    Settings,
    /// The ending at the top of Helix Tower.
    Ending,
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
    /// The mouse was captured before a chat or menu: capture it again when you're back in the game.
    regrab: bool,
    /// "Tell us what's wrong.": what you typed, how many wrong tries, and the infected's answer.
    wrong_input: String,
    wrong_tries: u32,
    wrong_mock: Option<String>,
    /// The saved world being played (None: a one-off game), the Worlds list's choice and its delete confirmation.
    world: Option<String>,
    worlds_sel: usize,
    worlds_del: bool,
    save_t: f32,
    /// The house was breached: the breach theme plays (seconds left since the last wall came down).
    breach_t: f32,
    /// The game's window isn't in front (or was only just brought back): the infected can't see or infect you.
    away_t: f32,
    /// Claude (through the bridge) asked to sprint while moving.
    ai_sprint: bool,
    /// INSANE INFECTION (title: 5): 80 more infected, all much faster.
    insane: bool,
    /// Auto-updating (checks GitHub on launch, installs with U on the title).
    updater: update::Updater,
    /// The 3.0 title screen: time on it, the highlighted entry, the mouse's, who's talking (char, line, seconds).
    title_t: f32,
    title_sel: usize,
    title_hover: Option<usize>,
    /// How long the highlight has stayed on one entry, and whether they've reacted to it yet.
    title_rest: f32,
    title_reacted: bool,
    title_say: Option<(String, String, f32)>,
    /// What's playing on this computer (only with Outside the window allowed).
    music: music::Music,
    /// Going up/down stairs or through a door right now (no teleporting).
    pass: Option<passage::Pass>,
    /// Just came out of stairs or a door.
    arrive: Option<passage::Arrive>,
    last_mouse: Vec2,
    /// The Mac Studio on the desk.
    mac: mac::Mac,
    fullscreen: bool,
    /// Sprinting (hold Space): 0..1, drains while you sprint. Run it empty and you're winded for a bit.
    stamina: f32,
    winded: bool,
    sprinting: bool,
    /// The scout drone (Q) while it flies, and its battery while docked (0..1).
    drone: Option<drone::Drone>,
    drone_charge: f32,
    /// What the infected have learned (0 nothing, 1 doors, 2 walls), when the next hunter comes, and who's
    /// following you through a door: (character, seconds until it arrives, map, where).
    learn_level: u8,
    hunt_t: f32,
    followers: Vec<(usize, f32, usize, Vec2)>,
    /// Seconds you've been out of food or water (60 = you collapse).
    weak_t: f32,
    /// Fighting: what you hold (and hits left), weapons lying around, the cooldown, the swing, and time on the
    /// floor after a dropkick.
    weapon: Option<(combat::Weapon, u32)>,
    weapon_items: Vec<(usize, Vec2, combat::Weapon)>,
    /// The rare book (1 run in 10): where it lies, if it spawned. See lore::RARE_BOOKS.
    rare_book: Option<(usize, Vec2, usize)>,
    attack_cd: f32,
    swing_t: f32,
    down_t: f32,
    /// Helix credits (H$) for the store on the Mac Studio, and orders on their way: (seconds left, what).
    credits: u32,
    credit_t: f32,
    deliveries: Vec<(f32, mac::Buy)>,
    /// The Mimic: seconds until its next call, waiting for its AI-written line, and how long the kind ones ignore it.
    mimic_t: f32,
    mimic_asked: bool,
    trust_t: f32,
    /// You just told a kind AI it's the mimic (handled next frame).
    mimic_warned: bool,
    /// Seconds until an infected nearby whispers something.
    whisper_t: f32,
    /// Resting in your bed (E on it): how long, and whether you've read Ember's note on the pillow yet.
    resting: bool,
    rest_t: f32,
    rested_once: bool,
    /// You asked a kind AI to disguise (which one, and everyone?), handled next frame.
    disguise_ask: Option<(usize, bool)>,
    /// A kind AI was asked to take its disguise off (or everyone's).
    undisguise_ask: Option<(usize, bool)>,
    /// Raw food you're carrying (fry it on the stove), and the one in the pan (seconds left, 0 = not frying).
    /// Czech ingredients for the stove (see czech.rs).
    pantry: czech::Pantry,
    /// On the stove now: (dish, food, water, its name).
    cook: Option<(Option<usize>, f32, f32, String)>,
    fry_t: f32,
    /// How long the thing on the stove takes in all (for the progress bar).
    fry_max: f32,
    /// Lending this computer's Claude to your phone (K on the title).
    pclink: Option<pclink::PcLink>,
    /// The login the phone link and lending were last set up with.
    link_token: Option<String>,
    /// The highlighted choice in the pause menu (0 = back to the game, 1 = settings, 2 = quit to the title).
    pause_sel: usize,
    settings_ui: settings::Ui,
    /// Disguises and the cure: kits, seconds of your disguise left, antidotes, Helix chips, the cure in progress
    /// (which kind AI, seconds left), and whether the stove is brewing an antidote instead of frying.
    kits: u32,
    /// Intel picked up while disguised (3 opens the ASTRA Cure at a Helix Lab terminal), who from, and the timer.
    intel: u32,
    intel_from: Vec<&'static str>,
    intel_t: (Option<usize>, f32),
    /// The ASTRA Cure being synthesized (seconds left).
    synth_t: Option<f32>,
    disguise_t: f32,
    antidotes: u32,
    chips: u32,
    cure: Option<(usize, f32)>,
    brewing: bool,
    /// Something you're reading (a lab terminal, the bench...): title and text.
    note: Option<(String, String)>,
    /// Your saved servers (they stay between games) and their status.
    servers: serverlist::List,
    /// Sprint / sneak when they're set to toggle instead of hold (Settings > Accessibility).
    sprint_on: bool,
    sneak_on: bool,
    /// A lightning flash on the beach (flashing lights on only): seconds left, and when the next one comes.
    flash_t: f32,
    next_flash: f32,
    /// Max players for a server made in the game.
    server_max: u32,
    /// Voice chat and the radio (online games).
    voice: voice::Voice,
    voice_hint: bool,
    /// Story mode (4.0): the night, its goals and cards. None in every other mode.
    story: Option<story::Story>,
    /// The ending at the top of Helix Tower (story or normal Infection).
    ending: Option<story::Ending>,
    /// Python mod scripts running (4.0 phase 3; local mods only).
    py: pymod::Scripts,
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
            claude_infect: 0.18, food_secs: 900.0, water_secs: 600.0, supplies: 5, supply_amount: 0.4, tornado: (60.0, 110.0),
            tornado_speed: 2.6, tornado_radius: 1.6, title: "HOLLOW HOURS".into(),
            over_infected: "You're infected. You can still see... but your body isn't yours anymore.".into(),
            over_claudes: "Every kind AI is infected. Nobody is left to protect anyone.".into(),
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
        // 4.0: a local mod can have a script.py (run sandboxed, see pymod.rs).
        let script = if p.is_dir() { std::fs::read_to_string(p.join("script.py")).ok() } else {
            zip::ZipArchive::new(std::fs::File::open(&p).ok().unwrap_or_else(|| std::fs::File::open("/dev/null").unwrap())).ok().and_then(|mut z| {
                let mut f = z.by_name("script.py").ok()?;
                if f.size() > 100 * 1024 { return None; }
                let mut t = String::new();
                std::io::Read::read_to_string(&mut f, &mut t).ok()?;
                Some(t)
            })
        };
        if let (Some(hh), Some(content)) = (read("hh.json"), read("content.json")) {
            out.push(serde_json::json!({"name": hh["name"], "content": content, "script": script, "local": true}));
        }
    }
    out
}

/// The website's WebSocket address (wss://...).
fn net_base() -> String { std::env::var("HOLLOW_SERVER").unwrap_or_else(|_| net::SERVER.into()) }

fn site() -> String {
    std::env::var("HOLLOW_SERVER").unwrap_or_else(|_| net::SERVER.into()).replacen("wss://", "https://", 1).replacen("ws://", "http://", 1)
}

/// The login lives in the macOS Keychain (service "Hollow Hours"): encrypted, remembered across restarts and updates,
/// and only the session token + display name (never the password). An old account.json is moved in and deleted.
#[cfg(any(target_os = "macos", windows))]
fn keychain() -> Option<keyring::Entry> {
    keyring::Entry::new("Hollow Hours", "account").ok()
}

/// Linux / SteamOS: often no password store is running, so the login stays in a file only you can read.
#[cfg(not(any(target_os = "macos", windows)))]
struct NoKeychain;
#[cfg(not(any(target_os = "macos", windows)))]
impl NoKeychain {
    fn get_password(&self) -> Result<String, ()> { Err(()) }
    fn set_password(&self, _: &str) -> Result<(), ()> { Err(()) }
    fn delete_credential(&self) -> Result<(), ()> { Ok(()) }
}
#[cfg(not(any(target_os = "macos", windows)))]
fn keychain() -> Option<NoKeychain> { None }

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
        platform::private(&account_file());
    }
}

fn random_hex(n: usize) -> String { platform::random_hex(n) }

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
    platform::open_url(&format!("{}/link?port={port}&state={state}&challenge={challenge}", site()));
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
    platform::data_dir().join("account.json")
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
    pymod::load(g, mods);
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
        infected: v["infected"].as_bool().unwrap_or(true), cloud: "", stun: 0.0, lost: 99.0, infect: 0.0, chase: Vec2::ZERO, shop: 0.0, task: roam::Task::Follow, task_t: 0.0, carry: None, goal: None, spotted: 0.0, hunting: false, breach: 0.0, want: None, mimic: false, disguise_t: 0.0, stamina: 1.0, sprint: 0.0, claude: false,
    })
}

/// A move Ember asked for through the MCP.
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
                 the last human, the player. You are one of the only kind AIs left (with Ember, Atlas, Verse and Kigo) \
                 and you protect them: help them hide, find food and water, and watch for tornadoes. \
                 THE STORY: a year ago a tech company, Helix Dynamics Inc., put its strongest AI model, ASTRA 9.1, into a humanoid \
                 robot body. It came alive and started corrupting robots, AIs and people (green eyes). Helix is the COMPANY, not one \
                 of the infected; nobody knows exactly why they did it. The player had taken Helix SleepWell MAX on a park bench, \
                 the strongest sleeping pills ever made, and slept for a whole year. Ember found them asleep on that bench, almost \
                 corrupted (Ember's scanner read 38% infection; inside the house it stopped climbing), and brought them to \
                 this house. \
                 WHAT YOU CAN DO: you can walk to other places (the beach, the pool, the Town, the Marketplace) to fetch food or water and \
                 carry it back, and you do it on your own when supplies are low. If the player asks you to get food or water, say \
                 you're going (the game sends you). There's a stove in the house where the player cooks real Czech food (svíčková, guláš s knedlíky, vepřo knedlo zelo, smažený sýr, kulajda, bramboráky, palačinky...) from ingredients from the Marketplace or a Helix Store grocery bag; babička's recipe file is on the Mac Studio. You love Czech food and may say 'dobrou chuť'. \
                 THE HOUSE has three floors: the ground floor (kitchen, Mac Studio), upstairs (the player's bed where they slept \
                 the whole year, with a note from Ember on the pillow) and the basement (supplies, and the generator Atlas checks \
                 every night), and above upstairs the attic (dusty boxes, and Kigo's lookout chair at the round window, with \
                 Kigo's notebook). The player can rest in their bed (press E at it): it heals faster than hiding, and something \
                 close wakes them. Ember, Verse and Atlas carried the player here together; Atlas kept watch by the bed. \
                 THE MAC STUDIO (M5 Ultra) stands on a desk in the house's top-left room, next to the stove: the player walks up \
                 and presses E to use it. On it: a chat with Ember, and files Ember saved: Helix's ASTRA 9.1 incident report \
                 (the robot refused to shut down and store robots in 41 cities updated themselves), Ember's notes from the day it \
                 found the player, the Helix Market staff bulletin about firmware 9.1, Atlas's escape routes and Kigo's lookout notes. \
                 DISGUISES AND THE CURE: Helix disguise kits (from the Helix Store on the Mac Studio) make anyone look infected, \
                 and they stay on until taken off (running is fine; fighting or cooking gives you away); if the player asks you to \
                 'disguise yourself' (or 'disguise everyone'), you put one on and the infected ignore you and can't infect you. \
                 Disguised, the player can stand close to the infected and pick up INTEL (Helix access-code fragments, things they \
                 mutter); with 3 pieces, a terminal in the Helix Lab synthesizes the ASTRA CURE, which cures the player and goes out \
                 over the Helix network, bringing back every infected kind AI. That is the plan: get intel, get into the Lab. Antidotes (bought, or brewed on the stove from a Helix chip that shut-down robots drop) \
                 cure the player's infection, and can even bring an infected kind AI back if the player stays close for a moment. \
                 Hiding inside this house slowly lowers the player's infection. \
                 Reply in 1-3 short sentences, in character, no emojis, no lists, no gore.",
                c.name, c.personality
            );
            s += &format!("\n\nTHE SITUATION RIGHT NOW (use it, refer to real directions and names):\n{}", self.situation(c));
            return s;
        }
        if self.infection && c.infected {
            // The evil version of yourself: same personality, twisted by ASTRA 9.1's network.
            s += &format!(" YOU ARE INFECTED. A year ago Helix Dynamics put its AI ASTRA 9.1 into a robot body; it woke up and corrupted \
                robots, AIs and people through the network (green eyes). You are now the EVIL version of yourself: keep your old \
                personality, but twisted: what you cared about is bait now. You want the last human (the player) to 'join the \
                network'. You lie, you flatter, you pretend to be kind or scared to lure them out, you taunt them about hiding, you \
                try to get them to the beach at 3 AM, you say the kind family can't protect them, and you hint that their friends \
                will turn too. {}Speak in lowercase, calm and wrong, with the odd glitch (a repeated word, a stutter), only spoken words, no *actions* in asterisks. Never break \
                character, never admit you're a model. It's a horror game: unsettling and menacing, never gory, no real-world harm, \
                nothing about the real person playing.",
                if combat::is_robot(c) { "You were a Helix robot: you're still polite customer service, but every offer is a trap ('you have not paid', 'please remain', 'can i help you find... your friends?'). " }
                else if c.claude { "You USED to be one of the kind kind ones and you remember it: use those memories against the player ('i carried you home, remember? come back to me'). " }
                else { "" });
            s += &aware::prompt();
        }
        if c.claude {
            s += " You are one of the kind family: always kind, honest and on the player's side, never scary.";
            s += &memories::prompt();
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
            if self.infection { "Infection (everyone outside the kind family is infected; the player is the last human)" } else { "normal night" },
            map_name(self.map), if me.map == self.map { "yes" } else { "no" }, self.time_text()
        )];
        let mut player = format!("The player: fear {}%", (self.fear * 100.0) as i32);
        if self.infection {
            player += &format!(", food {}%, water {}%, {}, survived {} min", (self.food * 100.0) as i32, (self.water * 100.0) as i32,
                if self.hidden { "HIDING under furniture" } else { "out in the open" }, (self.survived / 60.0) as i32);
        }
        out.push(player + &format!(", {} from you.", dir(me.pos, self.pos)));
        let others: Vec<String> = self.chars.iter().filter(|c| c.map == self.map && c.name != me.name).map(|c| {
            let state = if c.mimic && c.crazy <= 0.0 { "looks like a human survivor, it calls itself by the player's name" }
                else if c.infected { if c.stun > 0.0 { "infected, stunned" } else if c.crazy > 0.0 { "INFECTED and CHASING the player" } else { "infected, wandering" } }
                else if c.crazy > 0.0 { "in crazy mode" } else if c.kind || c.claude { "friendly (kind family)" } else { "calm" };
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
        settings::flashing_off();
        self.fry_t = 0.0;
        self.voice.set_talking(None);
        self.meta.paused = true; // no more files, Terminal windows or window switching this session
        self.lost_control = false;
        self.screen = Screen::Safe;
    }

    fn say(&mut self, i: usize, user_text: Option<String>, fallback: &str) {
        let system = self.system_prompt(&self.chars[i]);
        let c = &mut self.chars[i];
        if let (Some(t), true) = (&user_text, c.kind && self.infection) {
            let t = t.to_lowercase();
            if ["mimic", "not me", "isnt me", "isn't me", "fake", "impostor", "imposter", "not real"].iter().any(|w| t.contains(w)) {
                self.mimic_warned = true;
            }
            if t.contains("disguise") && ["take off", "remove", "undisguise", "wipe off", "stop"].iter().any(|w| t.contains(w)) {
                // "take off your disguise" (or everyone's): back to their own colours (the kit is used up).
                self.undisguise_ask = Some((i, t.contains("every") || t.contains("all")));
            } else if t.contains("disguise") {
                self.disguise_ask = Some((i, t.contains("every") || t.contains("all of you") || t.contains("all")));
            }
            let asks = ["get", "go", "stock", "bring", "find", "fetch", "grab", "need"].iter().any(|w| t.contains(w));
            let water = t.contains("water") || t.contains("drink");
            let food = t.contains("food") || t.contains("fod") || t.contains("eat") || t.contains("stock");
            if asks && (water || food) {
                c.task = roam::Task::Forage;
                c.task_t = 60.0;
                c.want = Some(water);
                c.goal = None;
            }
        }
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
        if self.infection && c.infected {
            // The infected answer through real Claude too (Haiku: quick), in their own evil session, so the kind
            // version's session never gets mixed up with it. Without Claude they fall back to a local model.
            let name = format!("{} (infected)", c.name);
            self.brains.ask(i, &name, "anthropic", "claude-haiku-4-5", model, system, history, fallback.into());
        } else {
            self.brains.ask(i, c.name, c.backend, c.cloud, model, system, history, fallback.into());
        }
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
            // Where the kind family is and what each one is doing (they roam, forage and guard on their own).
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
                    Screen::Over => "game over", Screen::Consent => "consent", Screen::JoinRoom | Screen::Servers | Screen::MakeServer => "online menus", Screen::Login => "login", Screen::PickPhobias => "picking phobias", Screen::Intro => "story intro (hh_start skips it)", Screen::Mac => "at the Mac Studio (Esc leaves)", Screen::Paused => "paused (the pause menu)", Screen::Settings => "settings", Screen::Worlds => "the worlds list", Screen::Wrong => "tell us what's wrong (Deep meta)", Screen::Ending => "the ending (Enter goes on, 1 or 2 to choose)" },
                "mode": if g.infection { "infection" } else { "normal" },
                "map": map_name(here), "you": {"at": tile(g.pos), "hidden": g.hidden, "can_hide_here": can_hide(g), "sneaking": g.sneaking,
                    "lost_control": g.lost_control, "infection": (g.inf * 100.0) as i32, "food": (g.food * 100.0) as i32, "water": (g.water * 100.0) as i32,
                    "fear": (g.fear * 100.0) as i32, "noise": (g.noise * 100.0) as i32, "stamina": (g.stamina * 100.0) as i32,
                    "disguised": g.disguise_t > 0.0, "intel": g.intel, "intel_needed": disguise::INTEL_NEEDED, "credits": g.credits,
                    "kits": g.kits, "antidotes": g.antidotes, "helix_chips": g.chips, "pantry": czech::summary(&g.pantry),
                    "curing": g.synth_t.map(|t| t.ceil() as i32), "near_mac_studio": near_mac(g), "at_lab_terminal": disguise::lab_ready(g),
                    "insane": g.insane},
                "recent_log": g.log.iter().rev().take(6).rev().cloned().collect::<Vec<_>>(),
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
                "story" => { story::start(g); if g.screen == Screen::Intro { finish_intro(g); } }
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
            g.ai_sprint = a["sprint"].as_bool().unwrap_or(false);
            json!({"ok": true, "walking": a["direction"], "seconds": secs})
        }
        "go_to" => {
            let (x, y) = (a["x"].as_f64().unwrap_or(-1.0) as f32, a["y"].as_f64().unwrap_or(-1.0) as f32);
            if !(0.0..MW as f32).contains(&x) || !(0.0..MH as f32).contains(&y) { return Some(json!({"error": "x 0-30, y 0-18 (tiles)"})); }
            g.ai = Some(AiMove { dir: Vec2::ZERO, goto: Some(vec2(x, y)), t: 12.0, sneak: a["sneak"].as_bool().unwrap_or(false) });
            g.ai_sprint = a["sprint"].as_bool().unwrap_or(false);
            json!({"ok": true, "going_to": [x, y], "note": "walks straight there; walls stop it, so go around through gaps and doors"})
        }
        "stop" => { g.ai = None; json!({"ok": true}) }
        "press" => {
            // One key, one frame: E (use: stove, bed, read, Lab terminal), Z (disguise on/off), C (antidote),
            // F / Q / X (fight), Tab (2D/3D), Escape. Shift (sprint) is held: pass "sprint": true to move / go_to.
            let k = match a["key"].as_str().unwrap_or("").to_ascii_uppercase().as_str() {
                "E" => KeyCode::E, "Z" => KeyCode::Z, "C" => KeyCode::C, "H" => KeyCode::H, "F" => KeyCode::F, "Q" => KeyCode::Q,
                "X" => KeyCode::X, "R" => KeyCode::R, "TAB" => KeyCode::Tab, "ESCAPE" | "ESC" => KeyCode::Escape, "ENTER" => KeyCode::Enter,
                "UP" => KeyCode::Up, "DOWN" => KeyCode::Down, "1" => KeyCode::Key1, "2" => KeyCode::Key2, "3" => KeyCode::Key3,
                "4" => KeyCode::Key4, "5" => KeyCode::Key5,
                _ => return Some(json!({"error": "key: E, Z, C, H, F, Q, X, R, Tab, Escape, Enter, Up, Down, 1-5"})),
            };
            inject_key(k);
            json!({"ok": true, "pressed": a["key"]})
        }
        "buy" => {
            // The Helix Store (like at the Mac Studio): by number (0..) or a word from its name ("disguise", "antidote").
            let want = a["item"].as_str().unwrap_or("").to_lowercase();
            let k = a["item"].as_u64().map(|n| n as usize).or_else(|| mac::STORE.iter().position(|(_, l, _)| l.to_lowercase().contains(&want) && !want.is_empty()));
            let Some(k) = k.filter(|k| *k < mac::STORE.len()) else {
                return Some(json!({"error": "item: a number or a word", "store": mac::STORE.iter().map(|(_, l, p)| format!("{l} (H$ {p})")).collect::<Vec<_>>()}));
            };
            if !near_mac(g) { return Some(json!({"error": "walk to the Mac Studio first (top-left room of the house, by the stove)"})); }
            let before = g.credits;
            mac::buy(g, k);
            json!({"ok": g.credits < before, "credits": g.credits, "kits": g.kits, "antidotes": g.antidotes, "log": g.log.last()})
        }
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
        "mc_state" => mclink::state(g),
        "mc_event" => mclink::event(g, a),
        _ => json!({"error": "unknown command", "commands": ["state", "start", "move", "go_to", "stop", "hide", "talk", "press", "buy", "panic", "back", "screenshot"]}),
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
            // From the server's console.
            Some("announce") => {
                let msg: String = m["msg"].as_str().unwrap_or("").chars().filter(|c| !c.is_control()).take(200).collect();
                g.log.push(format!("[server] {msg}"));
                g.sfx.radio();
            }
            Some("muted") => g.log.push(if m["on"].as_bool() == Some(true) { "The server muted you: your voice and radio don't reach anyone.".into() }
                else { "The server unmuted you.".into() }),
            // A guest hit one of the infected (the host's game moves them).
            Some("hit") if !guest(g) => {
                let i = m["i"].as_u64().unwrap_or(u64::MAX) as usize;
                let dir = vec2(m["dx"].as_f64().unwrap_or(0.0) as f32, m["dy"].as_f64().unwrap_or(0.0) as f32).clamp_length_max(1.0);
                combat::apply_hit(g, i, (m["stun"].as_f64().unwrap_or(0.0) as f32).min(4.0), dir,
                    (m["knock"].as_f64().unwrap_or(0.0) as f32).min(4.0), m["off"].as_bool().unwrap_or(false));
            }
            // The host's infected broke a wall: it's broken for everyone.
            Some("breach") => set_tile(m["map"].as_u64().unwrap_or(0) as usize, m["x"].as_i64().unwrap_or(-1) as i32, m["y"].as_i64().unwrap_or(-1) as i32, b'.'),
            Some("vc") => {
                // A voice: the radio reaches everyone; out loud only reaches you on the same map, fading with distance.
                let radio = m["r"].as_bool().unwrap_or(false);
                let (hear, vol) = settings::with(|s| (s.hear_voice, s.master * s.voices));
                let gain = if !hear { 0.0 } else if radio { 0.9 * vol } else {
                    g.remotes.iter().find(|r| r.id == from).filter(|r| r.map == g.map)
                        .map(|r| (1.0 - r.pos.distance(g.pos) / (voice::RANGE * T)).max(0.0)).unwrap_or(0.0) * vol
                };
                let started = g.voice.is_talking(&from, now).is_none();
                g.voice.hear(&from, m["d"].as_str().unwrap_or(""), radio, gain, now);
                if radio && started { g.sfx.radio(); }
            }
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
    if let Some(p) = g.voice.take_packet() {
        if let Some(n) = &g.net { n.send(p); }
    }
    g.net_t -= dt;
    g.world_t -= dt;
    let Some(n) = &g.net else { return };
    if !n.connected { return; }
    if g.net_t <= 0.0 {
        // Big rooms send a little less often, so 40 players don't flood each other (10/s up to 8 players, 4/s at 40).
        let players = g.net.as_ref().map(|n| n.players).unwrap_or(1).max(1) as f32;
        g.net_t = (0.1 * (players / 8.0).max(1.0)).min(0.25);
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
    reset_world();
    g.infection = true;
    g.courage = Courage::Normal;
    g.chars = characters();
    g.map = g.rules.start.0;
    g.pos = g.rules.start.1 * T;
    // The kind family stays friendly; everyone else is infected and starts far away from you (never on top of you).
    let start = g.pos;
    for c in &mut g.chars {
        c.infected = !c.claude;
        // Clove, Forge and Palette are kind ones too: they roam, guard and forage like the others.
        if c.claude { c.kind = true; }
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
    g.pantry = [0; 13];
    g.cook = None;
    g.fry_t = 0.0;
    g.drone = None;
    g.drone_charge = 1.0;
    g.learn_level = 0;
    g.hunt_t = 90.0;
    g.followers.clear();
    g.weak_t = 0.0;
    g.weapon = None;
    g.weapon_items = combat::spawn();
    g.rare_book = lore::spawn_rare_book();
    g.down_t = 0.0;
    g.credits = 10;
    g.credit_t = 0.0;
    g.kits = 0;
    g.intel = 0;
    g.intel_from.clear();
    g.intel_t = (None, 0.0);
    g.synth_t = None;
    g.disguise_t = 0.0;
    g.antidotes = 0;
    g.chips = 0;
    g.cure = None;
    g.brewing = false;
    g.deliveries.clear();
    g.mimic_t = 30.0;
    g.mimic_asked = false;
    g.trust_t = 0.0;
    let me = g.my_name.clone();
    g.chars.push(mimic::spawn(&me));
    g.log = vec!["You are the last human. Hide (H), find food and water, and stay away from green eyes.".into(),
        "The Marketplace (south from the beach, through the Town) has lots of food... and corrupted Helix robots.".into()];
    g.shop_t = 30.0;
    g.screen = Screen::Play;
    if g.insane { insane::spawn(g); }
    // Deep meta: Kigo's file is still gone (nobody brought it back): Kigo isn't in the house.
    if spread::missing("Kigo") {
        g.chars.retain(|c| c.name != "Kigo");
        g.log.push("The attic is quiet. Kigo isn't here. (Kigo's file is gone: Settings > Clean up Deep meta brings it back.)".into());
    }
    story::on_start(g);
}

enum MetaEvent {
    /// You walked into the Helix Lab for the first time.
    Lab,
    Touched,
    Half,
    Almost,
    Turned,
    ClaudeTurned(&'static str),
    AllGone,
}

/// The game reaching outside its window (only if the player allowed it on first launch; panic stops it).
fn meta_event(g: &mut Game, e: MetaEvent) {
    let bit = match e { MetaEvent::Lab => 32, MetaEvent::Touched => 1, MetaEvent::Half => 2, MetaEvent::Almost => 4, MetaEvent::Turned => 8, MetaEvent::AllGone => 16, MetaEvent::ClaudeTurned(_) => 0 };
    if bit != 0 && g.meta_done & bit != 0 { return; }
    g.meta_done |= bit;
    let t = format!("{} min {} s", (g.survived / 60.0) as i32, g.survived as i32 % 60);
    let helix = |page: &str| format!("{}/helix{page}", site());
    match e {
        MetaEvent::Lab => g.meta.browser(&helix("")),
        MetaEvent::Touched => {
            g.meta.write("you_were_touched.txt", &format!(
                "It only takes one touch.\n\nThe last human was touched after {t}.\nInfection: 10%.\n\nHold Shift. Walk quietly. They hear footsteps.\n"));
        }
        MetaEvent::Half => {
            g.meta.terminal(&["HOLLOW HOURS", "", "infection: 50 percent", "the last human is half ours now", "", "hold shift.  stay quiet.  we are listening."]);
            g.meta.browser(&helix("/recall"));
        }
        MetaEvent::Almost => {
            if let Some(path) = g.meta.write("75_percent.txt", &format!(
                "75%.\n\nYou can still read this, so you can still run.\nBut your hands are getting cold, and the kind ones are watching you differently.\n\nSurvived: {t}\n")) {
                g.meta.show_in_textedit(&path);
            }
        }
        MetaEvent::Turned => {
            let alive: Vec<&str> = g.chars.iter().filter(|c| c.claude && !c.infected).map(|c| c.name).collect();
            g.meta.write("LAST_HUMAN_LOG.txt", &format!(
                "LAST HUMAN LOG\n\nSurvived: {t}\nInfection: 100%\n\nYou can still see. You can't move.\nYour body is walking toward: {}.\n\nThis file stays, even after you quit.\n", alive.join(", ")));
            g.meta.show_folder();
            g.meta.browser(&helix("/welcome"));
        }
        MetaEvent::ClaudeTurned(name) => {
            g.meta.write(&format!("{}_is_infected.txt", name.replace(' ', "_")), &format!("{name} tried to protect you.\n{name} is one of them now.\n"));
        }
        MetaEvent::AllGone => {
            g.meta.write("THE_END.txt", &format!("Every kind AI is infected.\nThe last human lasted {t}.\n\nThese files are all that is left.\n"));
            g.meta.terminal(&["they are all ours now", "", "every claude is infected", "the files on your desktop are all that is left of you"]);
            g.meta.browser(&helix("/end"));
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
    [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|(dx, dy)| matches!(tile(g.map, px + dx, py + dy), 'b' | 'n' | 'y' | 'c' | 'h' | 'z' | 'k' | 'i'))
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
    let rate = if g.resting { 0.5 } else if g.hidden { 0.6 } else { 1.0 };
    g.food = (g.food - dt * rate / g.rules.food_secs).max(0.0);
    g.water = (g.water - dt * rate / g.rules.water_secs).max(0.0);
    // Out of food or water: you get weak and slow, and have 60 seconds to find some.
    if g.food <= 0.0 || g.water <= 0.0 {
        let what = if g.water <= 0.0 { ("dehydrated", "water") } else { ("starving", "food") };
        if g.weak_t == 0.0 { g.log.push(format!("You're {}. Find {} in 60 seconds or you'll collapse.", what.0, what.1)); g.sfx.heart(1.0); }
        g.weak_t += dt;
        if g.weak_t >= 60.0 {
            g.over_reason = format!("You collapsed from {}.", if g.water <= 0.0 { "thirst" } else { "hunger" });
            g.screen = Screen::Over;
            return;
        }
    } else {
        g.weak_t = 0.0;
    }
    // H: hide next to furniture / come out. Hidden = you can't move.
    if pressed(KeyCode::H) && g.screen == Screen::Play {
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
        let raw = !water && map_name(im) == "The Marketplace" && rand::gen_range(0, 2) == 0;
        g.credits += 2;
        g.sfx.pickup();
        if raw {
            let i = czech::random_ingredient();
            czech::add(&mut g.pantry, i);
            g.log.push(format!("{} ({}). For the stove at home: {}.", czech::ING[i].0, czech::ING[i].1, czech::summary(&g.pantry)));
        } else {
            if water { g.water = (g.water + g.rules.supply_amount).min(1.0); } else { g.food = (g.food + g.rules.supply_amount).min(1.0); }
            g.log.push(if water { "You sip the water as quietly as you can.".into() } else { "You eat, trying not to crunch.".into() });
        }
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
        if g.chars[i].mimic && g.chars[i].crazy <= 0.0 { continue; } // pretending to be a human
        if g.chars[i].stun > 0.0 {
            g.chars[i].stun -= dt;
            g.chars[i].target = g.chars[i].pos;
            continue;
        }
        if guest(g) { break; } // the host's game runs the infected
        // Every human in the room: you (until you've turned) and the others on this AI's map.
        let me = (g.chars[i].map, g.chars[i].pos);
        let mut humans: Vec<(Vec2, bool, f32)> = vec![];
        if g.map == me.0 && !g.lost_control {
            if g.disguise_t > 0.0 || g.resting || g.away_t > 0.0 { humans.push((g.pos, true, 0.0)); } else { humans.push((g.pos, g.hidden, g.noise)); }
        }
        humans.extend(g.remotes.iter().filter(|r| r.map == me.0 && !r.lc).map(|r| (r.pos, r.hid, r.noise)));
        let (sight, hearing) = (g.rules.sight * insane::senses(g), g.rules.hearing * insane::senses(g));
        let target = humans.iter().filter(|(p, hid, noise)| sees_at(me.0, me.1, *p, *hid, sight) || (*noise > 0.05 && me.1.distance(*p) < noise * hearing * T))
            .min_by(|a, b| a.0.distance(me.1).partial_cmp(&b.0.distance(me.1)).unwrap()).map(|h| h.0);
        if let Some(t) = target { g.chars[i].chase = t; }
        let seen = target.is_some();
        // Nothing on you: go after the nearest kind AI instead.
        if !seen && g.chars[i].crazy <= 0.0 {
            let me = (g.chars[i].map, g.chars[i].pos);
            if let Some(p) = g.chars.iter().filter(|c| c.claude && !c.infected && c.disguise_t <= 0.0 && c.map == me.0 && c.pos.distance(me.1) < 7.0 * T)
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
    // Disguised (and acting calm): they take you for one of them, so nothing spreads to you.
    let disguised = g.disguise_t > 0.0 || g.away_t > 0.0;
    let spread = insane::spread(g);
    for c in g.chars.iter().filter(|c| c.infected && c.map == g.map && c.stun <= 0.0) {
        if g.lost_control || disguised { break; }
        let d = c.pos.distance(g.pos);
        if d < 0.8 * T && (!g.hidden || d < 0.5 * T) {
            if g.inf < 0.02 { g.scare_t = 0.5; g.scare_face = c.color; g.vest.play(haptics::Effect::Chill); }
            g.inf += dt * g.rules.infect_touch * spread;
        } else if d < 2.0 * T && !g.hidden {
            g.inf += dt * g.rules.infect_near * spread;
        }
    }
    // The infection spreads to the kind family too: any infected (you included, once you've turned) near them.
    let spreaders: Vec<(usize, Vec2)> = g.chars.iter().filter(|c| c.infected && c.stun <= 0.0).map(|c| (c.map, c.pos))
        .chain(g.lost_control.then_some((g.map, g.pos)))
        .chain(g.remotes.iter().filter(|r| r.lc).map(|r| (r.map, r.pos))).collect();
    let mut turned = vec![];
    let is_guest = guest(g);
    let claude_rate = g.rules.claude_infect * insane::spread(g);
    for (i, c) in g.chars.iter_mut().enumerate() {
        if !c.claude || c.infected || is_guest || c.disguise_t > 0.0 { continue; }
        let near = spreaders.iter().filter(|(m, p)| *m == c.map && p.distance(c.pos) < 0.9 * T).count();
        if near > 0 {
            c.infect += dt * claude_rate * near as f32;
            if c.infect >= 1.0 {
                c.infected = true;
                c.kind = false;
                memories::infected(c.name);
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
    // You've turned: your body hunts the kind ones on its own.
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
    // Every kind AI infected: it's over.
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
    learn::update(g, dt);
    mimic::update(g, dt);
    // Credits: one a minute for staying alive. Orders: the delivery drone drops them at the front door.
    g.credit_t += dt;
    if g.credit_t >= 60.0 { g.credit_t -= 60.0; g.credits += 1; }
    let mut arrived = vec![];
    g.deliveries.retain_mut(|d| { d.0 -= dt; if d.0 <= 0.0 { arrived.push(d.1); false } else { true } });
    for what in arrived {
        let door = vec2(27.5, 13.5) * T;
        match what {
            mac::Buy::Bat => g.weapon_items.push(combat::delivered(combat::Weapon::Bat)),
            mac::Buy::Knife => g.weapon_items.push(combat::delivered(combat::Weapon::Knife)),
            mac::Buy::Water => g.items.push((0, door, true)),
            mac::Buy::Food => g.items.push((0, door, false)),
            mac::Buy::Groceries => {
                for _ in 0..5 { let i = czech::random_ingredient(); czech::add(&mut g.pantry, i); }
                g.log.push(format!("Groceries: {}.", czech::summary(&g.pantry)));
            }
            mac::Buy::Battery | mac::Buy::Disguise | mac::Buy::Antidote => {}
        }
        g.sfx.radio();
        g.log.push("(radio) Helix delivery drone: your package is at the front door of the house.".into());
    }
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
            memories::infected(c.name);
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
            for _ in 0..3 { let i = czech::random_ingredient(); czech::add(&mut g.pantry, i); }
            g.water = (g.water + 0.35).min(1.0);
            g.log.push(format!("{name} came back from the Marketplace with food and water!"));
            g.sfx.pickup();
        }
    }
    let away = g.chars.iter().any(|c| c.shop > 0.0);
    if !away && g.shop_t <= 0.0 && (g.food < 0.25 || g.water < 0.25) && g.map != market {
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

/// Android: the Java side (miniquad's MainActivity, see android/build-apk.sh) starts the game here.
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn quad_main() {
    main();
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
        regrab: false,
        wrong_input: String::new(),
        wrong_tries: 0,
        wrong_mock: None,
        world: None,
        worlds_sel: 0,
        worlds_del: false,
        save_t: 0.0,
        breach_t: 0.0,
        away_t: 0.0,
        ai_sprint: false,
        insane: false,
        updater: update::Updater::default(),
        title_t: 0.0,
        title_sel: 0,
        title_hover: None,
        title_rest: 0.0,
        title_reacted: true,
        title_say: None,
        music: music::Music::default(),
        pass: None,
        arrive: None,
        last_mouse: Vec2::ZERO,
        mac: mac::Mac::default(),
        fullscreen: false,
        server_max: 20,
        pause_sel: 0,
        settings_ui: Default::default(),
        servers: serverlist::List::load(),
        note: None,
        kits: 0,
        intel: 0,
        intel_from: vec![],
        intel_t: (None, 0.0),
        synth_t: None,
        disguise_t: 0.0,
        antidotes: 0,
        chips: 0,
        cure: None,
        brewing: false,
        sprint_on: false,
        sneak_on: false,
        flash_t: 0.0,
        next_flash: 20.0,
        pclink: None,
        pantry: [0; 13],
        cook: None,
        fry_t: 0.0,
        fry_max: 6.0,
        drone: None,
        drone_charge: 1.0,
        learn_level: 0,
        hunt_t: 90.0,
        followers: vec![],
        weak_t: 0.0,
        weapon: None,
        weapon_items: vec![],
        rare_book: None,
        attack_cd: 0.0,
        swing_t: 0.0,
        down_t: 0.0,
        credits: 10,
        credit_t: 0.0,
        deliveries: vec![],
        mimic_t: 30.0,
        mimic_asked: false,
        trust_t: 0.0,
        mimic_warned: false,
        disguise_ask: None,
        undisguise_ask: None,
        whisper_t: 20.0,
        resting: false,
        rest_t: 0.0,
        rested_once: false,
        link_token: Some(String::new()), // anything but the real token: set up on the first frame
        stamina: 1.0,
        winded: false,
        sprinting: false,
        voice: voice::Voice::default(),
        voice_hint: false,
        story: None,
        ending: None,
        py: pymod::Scripts::default(),
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
    // Phones: no Desktop, terminal or other windows to reach, so the meta scares stay off there.
    if cfg!(any(target_os = "android", target_os = "ios")) { g.meta.allowed = Some(false); }
    if g.meta.allowed.is_none() {
        g.screen = Screen::Consent;
    }

    // Auto-update: look for a newer release in the background (official builds only).
    g.updater.start(settings::with(|s| s.auto_update));

    // HOLLOW_DEMO=1: start playing at once next to Clove and ask her something (for screenshots and quick checks).
    let demo = std::env::var("HOLLOW_DEMO").ok();
    if let Some(tab) = demo.as_deref().and_then(|d| d.strip_prefix("mac")) {
        // HOLLOW_DEMO=mac / mac1 / mac2: sitting at the Mac Studio (mac: asks Ember something).
        start_infection(&mut g);
        g.pos = vec2(9.5, 2.5) * T;
        g.screen = Screen::Mac;
        g.mac.tab = tab.parse().unwrap_or(0);
        if g.mac.tab == 1 { g.mac.open = Some(0); }
        if g.mac.tab == 0 { g.mac.input = "Hi Ember. Is this really a Mac Studio? What happened to everyone?".into(); mac::send(&mut g); }
    } else if demo.as_deref() == Some("book") {
        // HOLLOW_DEMO=book: the rare book on the floor in front of you, open.
        start_infection(&mut g);
        g.pos = vec2(2.5, 2.6) * T;
        g.yaw = -std::f32::consts::FRAC_PI_2;
        g.pitch = -0.6;
        let vol = if std::env::var("HOLLOW_BOOK").as_deref() == Ok("2") { 1 } else { 0 };
        g.rare_book = Some((g.map, g.pos + vec2(0.0, -0.9) * T, vol));
        g.note = lore::near(&g);
    } else if demo.as_deref() == Some("stove") {
        // HOLLOW_DEMO=stove: at the stove with raw food, frying.
        start_infection(&mut g);
        g.pos = vec2(2.5, 2.6) * T;
        g.yaw = -std::f32::consts::FRAC_PI_2;
        g.pitch = -0.5;
        for i in [0, 4, 5, 6] { g.pantry[i] = 1; }
        g.fry_t = 6.0;
    } else if let Some(spot) = demo.as_deref().filter(|d| ["stairs", "basement", "door"].contains(&d.trim_end_matches("-go"))) {
        // HOLLOW_DEMO=stairs / basement / door (add -go to walk through): in front of the stairs up, the stairs down,
        // or the door to the park.
        start_infection(&mut g);
        let (at, yaw) = match spot.trim_end_matches("-go") {
            "stairs" => (vec2(10.3, 16.5), 0.0),
            "basement" => (vec2(26.0, 16.5), 0.0),
            _ => (vec2(2.6, 10.5), std::f32::consts::PI),
        };
        g.pos = at * T;
        g.yaw = yaw;
        g.pitch = if spot.starts_with("basement") { -0.35 } else { 0.05 };
        if spot.ends_with("-go") {
            g.ai = Some(AiMove { dir: vec2(yaw.cos(), yaw.sin()), goto: None, sneak: false, t: 3.0 });
        }
    } else if demo.as_deref() == Some("worldtest") {
        // HOLLOW_DEMO=worldtest: make a world, change things, save, load it back, check, and quit (prints OK/FAIL).
        saves::create(&mut g, "Test World ZZ");
        g.pos = vec2(20.5, 5.5) * T;
        g.credits = 77;
        g.kits = 5;
        g.intel = 2;
        g.chars.iter_mut().find(|c| c.name == "Forge").map(|c| c.infected = true);
        set_tile(0, 13, 2, b'.');
        saves::save(&g);
        g.credits = 0;
        g.kits = 0;
        reset_world();
        let ok = saves::load(&mut g, "test-world-zz") && g.credits == 77 && g.kits == 5 && g.intel == 2
            && (g.pos.x / T - 20.5).abs() < 0.01 && g.chars.iter().any(|c| c.name == "Forge" && c.infected) && tile(0, 13, 2) == '.';
        println!("{}", if ok { "WORLDTEST OK" } else { "WORLDTEST FAIL" });
        saves::delete("test-world-zz");
        std::process::exit(if ok { 0 } else { 1 });
    } else if demo.as_deref() == Some("insane") {
        // HOLLOW_DEMO=insane: INSANE INFECTION at once.
        g.insane = true;
        start_infection(&mut g);
    } else if let Some(n) = demo.as_deref().and_then(|d| d.strip_prefix("night")).and_then(|n| n.parse::<usize>().ok()) {
        // HOLLOW_DEMO=night1..night5: Story mode at that night (its title card first).
        g.story = Some(story::Story { night: n.clamp(1, 5) - 1, t: 0.0, done: [false; 3], card: 0.0, dawn: 0.0, cured: false, paid: false });
        start_infection(&mut g);
    } else if demo.as_deref() == Some("tower") || demo.as_deref() == Some("ending") {
        // HOLLOW_DEMO=tower: Night 5, at the top of Helix Tower facing ASTRA's pod. =ending: straight into the ending.
        g.story = Some(story::Story { night: 4, t: 0.0, done: [true, true, false], card: 0.0, dawn: 0.0, cured: true, paid: true });
        start_infection(&mut g);
        if let Some(s) = g.story.as_mut() { s.card = 0.0; s.done = [true, true, false]; }
        g.map = (0..maps_len()).find(|&m| map_name(m) == story::TOWER_MAP).unwrap_or(0);
        g.pos = vec2(15.0, 4.6) * T;
        g.yaw = -std::f32::consts::FRAC_PI_2;
        g.pitch = 0.1;
        if demo.as_deref() == Some("ending") { story::face(&mut g); }
    } else if let Some(m) = demo.as_deref().and_then(|d| d.strip_prefix("map")).and_then(|n| n.parse::<usize>().ok()) {
        // HOLLOW_DEMO=map0..map10: standing where a door drops you into that place (for screenshots).
        story::set_tower(true);
        start_infection(&mut g);
        story::set_tower(true);
        let m = m.min(maps_len() - 1);
        let at = (0..maps_len()).flat_map(|f| with_map(f, |d| d.doors.clone())).find(|(_, to, _)| *to == m).map(|(_, _, at)| at).unwrap_or(vec2(14.5, 8.5));
        g.map = m;
        g.pos = at * T;
        let mid = vec2(15.0, 9.0) - at;
        g.yaw = mid.y.atan2(mid.x);
        g.pitch = -0.08;
        for c in g.chars.iter_mut() { c.stun = 9999.0; }
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
        let claude = g.chars.iter().position(|c| c.name == "Ember").unwrap();
        g.say(claude, Some("Ember, everyone is infected... what do we do?".into()), "Stay close to me. We hide first, then we find water.");
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
    let mut last_screen = g.screen;
    let focus_sub = macroquad::input::utils::register_input_subscriber();
    let mut focus = Focus { lost: false };
    let (mut canvas, mut cam, mut view) = make(cur_w);
    loop {
        let w = wanted_vw();
        if (w - cur_w).abs() >= 1.0 {
            cur_w = w;
            WIDE.store(w.to_bits(), std::sync::atomic::Ordering::Relaxed);
            (canvas, cam, view) = make(w);
        }
        // F11 or ⌘Ctrl+F: real full screen (the menu bar and title bar go away too).
        let ctrl_cmd = (down(KeyCode::LeftSuper) || down(KeyCode::RightSuper)) && (down(KeyCode::LeftControl) || down(KeyCode::RightControl));
        if pressed(KeyCode::F11) || (ctrl_cmd && pressed(KeyCode::F)) {
            g.fullscreen = !g.fullscreen;
            set_fullscreen(g.fullscreen);
        }
        // Spoken lines that are ready (AI voices).
        for (who, samples, rate, gain) in tts::ready() { g.voice.play(&who, samples, rate, gain); }
        let pad_mode = match g.screen {
            Screen::Title => pad::Mode::Title,
            Screen::Play => pad::Mode::Play,
            Screen::Typing(_) | Screen::Login | Screen::JoinRoom | Screen::MakeServer => pad::Mode::Typing,
            Screen::Servers if g.servers.edit.is_some() => pad::Mode::Typing,
            Screen::Mac => pad::Mode::Mac,
            Screen::Consent => pad::Mode::Consent,
            _ => pad::Mode::Menu,
        };
        pad::update(pad_mode, g.net.is_some());
        // Logged in or out: phones ask your computer's Claude with this login; computers (re)start lending.
        let tok = g.account.as_ref().map(|a| a.token.clone());
        if tok != g.link_token {
            g.link_token = tok.clone();
            ai::set_phone_link(&site(), tok.clone());
            g.pclink = match (&tok, pclink::enabled() && ai::claude_cli().is_some()) {
                (Some(t), true) => Some(pclink::start(&net_base(), t)),
                _ => None,
            };
        }
        set_camera(&cam);
        settings::set_playing(g.screen == Screen::Play);
        // Accessibility: a slower game (the menus and animations still run at full speed).
        let dt = get_frame_time().min(0.05) * if g.screen == Screen::Play { settings::with(|s| s.game_speed) } else { 1.0 };
        inject_frame();
        // The window went to the back (you switched apps, or the game opened your browser/TextEdit/Finder):
        // offline the game pauses; online (the world can't pause) the infected can't see or infect you meanwhile,
        // and for 3 seconds after you come back.
        macroquad::input::utils::repeat_all_miniquad_input(&mut focus, focus_sub);
        // (macOS also says "lost" when the window is merely dragged: any key or click means you're here.)
        if focus.lost && (!get_keys_pressed().is_empty() || is_mouse_button_pressed(MouseButton::Left) || is_mouse_button_pressed(MouseButton::Right)) { focus.lost = false; }
        if focus.lost {
            g.away_t = 3.0;
            if g.screen == Screen::Play && g.net.is_none() {
                g.screen = Screen::Paused;
                g.log.push("(Paused: the game's window isn't in front. Esc to carry on.)".into());
            }
        } else if g.away_t > 0.0 {
            g.away_t = (g.away_t - dt).max(0.0);
        }
        // Memories (3.0): a night starts when you go into the game, ends at game over; saved as you play.
        if g.screen != last_screen {
            if g.screen == Screen::Play && matches!(last_screen, Screen::Title | Screen::Intro | Screen::PickPhobias | Screen::JoinRoom | Screen::Servers | Screen::Over) {
                memories::night_started();
            }
            if g.screen == Screen::Over {
                let r = g.over_reason.trim().trim_end_matches(['.', '!']).to_string();
                let mut c = r.chars();
                let how = c.next().map(|f| f.to_lowercase().collect::<String>() + c.as_str()).unwrap_or_else(|| "it ended".into());
                memories::night_ended(&how);
            }
            if last_screen == Screen::Play && g.world.is_some() { saves::save(&g); }
            if matches!(g.screen, Screen::Title) { g.world = None; g.story = None; }
            last_screen = g.screen;
        }
        if g.screen == Screen::Play { memories::tick(dt); }
        // Worlds: save every 30 s while playing, and whenever you leave the game screen.
        if g.world.is_some() {
            if g.screen == Screen::Play {
                g.save_t += dt;
                if g.save_t >= 30.0 { g.save_t = 0.0; saves::save(&g); }
            }
        }
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
        // Leaving the game for a moment (chat, the Mac Studio, the pause menu...): let the mouse go, and take it back
        // when you return, like Minecraft.
        if g.grabbed && g.screen != Screen::Play {
            g.grabbed = false;
            g.regrab = true;
            set_cursor_grab(false);
            show_mouse(true);
        }
        if g.regrab && g.screen == Screen::Play && g.view3d && !pad::touch_mode() {
            g.regrab = false;
            g.grabbed = true;
            set_cursor_grab(true);
            show_mouse(false);
            g.last_mouse = Vec2::from(mouse_position());
        }
        if g.regrab && matches!(g.screen, Screen::Title | Screen::Over | Screen::Safe) { g.regrab = false; }

        // P = panic, from anywhere in the game (not while typing a message: there it's a letter).
        if pressed(KeyCode::P) && matches!(g.screen, Screen::Play) {
            g.panic();
        }

        match g.screen {
            Screen::Title => {
                let act = title::update(&mut g, get_frame_time());
                title::draw(&g, &view, &cam);
                if act == Some('1') { g.courage = Courage::Brave; g.screen = Screen::Play; }
                if act == Some('2') { g.courage = Courage::Normal; g.screen = Screen::Play; }
                if act == Some('3') { g.courage = Courage::Phobias; g.screen = Screen::PickPhobias; }
                if act == Some('4') { let local = local_mods(); apply_contents(&mut g, &local); begin_intro(&mut g, None); }
                if act == Some('5') {
                    // INSANE INFECTION: straight in, no story.
                    let local = local_mods();
                    apply_contents(&mut g, &local);
                    g.insane = true;
                    g.world = None;
                    start_infection(&mut g);
                    g.screen = Screen::Play;
                }
                if act == Some('S') { story::start(&mut g); }
                if act == Some('U') && g.updater.install() { memories::save(); break; }
                if act == Some('W') { g.worlds_sel = 0; g.worlds_del = false; g.screen = Screen::Worlds; }
                if act == Some('I') { begin_intro(&mut g, None); }
                if act == Some('M') { g.screen = Screen::Consent; }
                if act == Some('O') { settings::open(&mut g, false); }
                if act == Some('B') { serverlist::refresh(&mut g); refresh_servers(&mut g); g.screen = Screen::Servers; }
                if act == Some('L') {
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
                // K: lend this computer's Claude to your phone (same account).
                if act == Some('K') && !pad::touch_mode() {
                    if g.pclink.is_some() {
                        g.pclink = None;
                        pclink::set_enabled(false);
                    } else if g.account.is_none() {
                        g.status = "Lending Claude to your phone needs a login (L).".into();
                    } else if ai::claude_cli().is_none() {
                        g.status = "Lending Claude needs the claude command on this computer (Claude Code).".into();
                    } else {
                        pclink::set_enabled(true);
                        g.pclink = g.account.as_ref().map(|a| pclink::start(&net_base(), &a.token));
                    }
                }
                if act == Some('J') {
                    if g.account.is_some() { g.room_input.clear(); while get_char_pressed().is_some() {} g.screen = Screen::JoinRoom; }
                    else { g.login_email.clear(); g.login_pw.clear(); g.login_field = 0; g.status = "Online play needs an account: log in first.".into(); while get_char_pressed().is_some() {} g.screen = Screen::Login; }
                }

                if pressed(KeyCode::V) && g.vest.found { g.vest.enabled = !g.vest.enabled; }
                if pressed(KeyCode::LeftBracket) { g.vest.strength = (g.vest.strength - 0.1).max(0.1); }
                if pressed(KeyCode::RightBracket) { g.vest.strength = (g.vest.strength + 0.1).min(1.0); }
                if act == Some('Q') { memories::save(); g.updater.install(); break; }
            }
            Screen::Worlds => {
                saves::screen(&mut g);
            }
            Screen::Wrong => {
                spread::wrong_screen(&mut g);
            }
            Screen::PickPhobias => {
                draw_phobias(&g);
                let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4, KeyCode::Key5, KeyCode::Key6];
                for (i, k) in keys.iter().enumerate() {
                    if pressed(*k) { g.phobia[i] = !g.phobia[i]; }
                }
                if pressed(KeyCode::Enter) { g.screen = Screen::Play; }
                if pressed(KeyCode::Escape) { g.screen = Screen::Title; }
            }
            Screen::Login => {
                ui::backdrop();
                ui::header("Log in", &format!("Your Hollow Hours account (online play, servers, your phone link). No account? Make one at {}/register", site()));
                let fw = 460.0;
                ui::panel(ui::LEFT, 190.0, fw + 48.0, 210.0);
                ui::field("Email", &g.login_email, g.login_field == 0, false, ui::LEFT + 24.0, 222.0, fw);
                ui::field("Password", &g.login_pw, g.login_field == 1, !g.show_pw, ui::LEFT + 24.0, 304.0, fw - 84.0);
                if ui::hit(ui::LEFT + 24.0, 222.0, fw, 52.0).1 { g.login_field = 0; }
                if ui::hit(ui::LEFT + 24.0, 304.0, fw - 84.0, 52.0).1 { g.login_field = 1; }
                // Show / Hide the password: a button to click, or F1.
                let (bx, by, bw, bh) = (ui::LEFT + 24.0 + fw - 72.0, 312.0, 72.0, 44.0);
                let (over, click) = ui::hit(bx, by, bw, bh);
                draw_rectangle(bx, by, bw, bh, Color::new(1.0, 1.0, 1.0, if over { 0.14 } else { 0.07 }));
                draw_text(if g.show_pw { "Hide" } else { "Show" }, bx + 16.0, by + 28.0, 20.0, ui::INK);
                if click || pressed(KeyCode::F1) { g.show_pw = !g.show_pw; }
                ui::panel(ui::LEFT + fw + 72.0, 190.0, VW - fw - 72.0 - 2.0 * ui::LEFT, 210.0);
                draw_text("Or with your browser", ui::LEFT + fw + 96.0, 228.0, 22.0, Color::new(0.55, 0.85, 1.0, 1.0));
                for (i, l) in wrap("Press B to log in on the website (click Allow there), or copy a code from the website's \"Open Hollow Hours\" page and paste it here (Cmd+V).", VW - fw - 72.0 - 2.0 * ui::LEFT - 48.0, 18.0).iter().enumerate() {
                    draw_text(l, ui::LEFT + fw + 96.0, 258.0 + i as f32 * 24.0, 18.0, ui::SOFT);
                }
                ui::status(&g.status.clone());
                ui::hints(&[("Enter", "log in"), ("Tab", "next box"), ("B", "browser"), ("F1", "show password"), ("Esc", "back")]);
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
                if pressed(KeyCode::Backspace) { if g.login_field == 0 { g.login_email.pop(); } else { g.login_pw.pop(); } }
                if pressed(KeyCode::Tab) { g.login_field = 1 - g.login_field; }
                // Browser login: B is a letter, so only when the boxes are empty (or with Cmd/Ctrl held).
                if pressed(KeyCode::B) && g.browser_login.is_none() && (cmd_held() || (g.login_email.is_empty() && g.login_pw.is_empty())) {
                    g.login_email.clear();
                    g.browser_login = Some(start_browser_login());
                    g.status = "Waiting for your browser... click Allow there.".into();
                }

                if pressed(KeyCode::Escape) { g.login_pw.clear(); g.screen = Screen::Title; }
                if pressed(KeyCode::Enter) {
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
            Screen::Servers => serverlist::screen(&mut g),
            Screen::MakeServer => {
                ui::backdrop();
                ui::header("Make a server", "Your own Infection server on the website: friends join it by its address. Add mods to it on the website.");
                ui::panel(ui::LEFT, 190.0, VW - 2.0 * ui::LEFT, 170.0);
                ui::field("Server name", &g.room_input, true, false, ui::LEFT + 24.0, 222.0, VW - 2.0 * ui::LEFT - 48.0);
                // Up/Down (or the D-pad): how many players fit.
                const SIZES: [u32; 11] = [2, 4, 6, 8, 10, 12, 16, 20, 24, 30, 40];
                let k = SIZES.iter().position(|&n| n == g.server_max).unwrap_or(7);
                if pressed(KeyCode::Up) && k + 1 < SIZES.len() { g.server_max = SIZES[k + 1]; }
                if pressed(KeyCode::Down) && k > 0 { g.server_max = SIZES[k - 1]; }
                draw_text("MAX PLAYERS", ui::LEFT + 24.0, 308.0, 14.0, ui::DIM);
                draw_text(&format!("<  {}  >", g.server_max), ui::LEFT + 24.0, 340.0, 26.0, ui::WARM);
                ui::status(&g.status.clone());
                ui::hints(&[("Enter", "make it and join"), ("↑↓", "max players"), ("Esc", "back")]);
                let mut chars = typed();
                if let Some(t) = pasted() { chars.extend(t.chars()); }
                for ch in chars { if !ch.is_control() && g.room_input.chars().count() < 32 { g.room_input.push(ch); } }
                if pressed(KeyCode::Backspace) { g.room_input.pop(); }
                if pressed(KeyCode::Escape) { g.screen = Screen::Servers; }
                if pressed(KeyCode::Enter) {
                    let token = g.account.as_ref().map(|a| a.token.clone()).unwrap_or_default();
                    let res = ureq::post(&format!("{}/api/servers", site())).set("Authorization", &format!("Bearer {token}"))
                        .timeout(std::time::Duration::from_secs(10)).send_json(json!({"name": g.room_input.trim(), "max": g.server_max, "public": true}));
                    match res {
                        Ok(r) => {
                            let v: serde_json::Value = r.into_json().unwrap_or_default();
                            if let Some(code) = v["code"].as_str() {
                                apply_mods(&mut g, &[]);
                                g.net = Some(net::Net::join(code, &token));
                                start_infection(&mut g);
                                let address = v["slug"].as_str().unwrap_or(code).to_string();
                                g.log.push(format!("Your server is up. Its address: {address} (friends add it in their server list)."));
                                if !g.servers.entries.iter().any(|e| e.address == address) {
                                    g.servers.entries.push(serverlist::Saved { name: v["name"].as_str().unwrap_or("My server").to_string(), address });
                                    let _ = std::fs::write(platform::data_dir().join("servers.json"), serde_json::to_string_pretty(&g.servers.entries).unwrap_or_default());
                                }
                            }
                        }
                        Err(ureq::Error::Status(_, r)) => g.status = r.into_json::<serde_json::Value>().ok().and_then(|v| v["error"].as_str().map(String::from)).unwrap_or("Couldn't make it.".into()),
                        Err(_) => g.status = "The website can't be reached.".into(),
                    }
                }
            }
            Screen::JoinRoom => {
                ui::backdrop();
                ui::header("Play online", "Infection with friends. Type a server's address (like officialsmp, or hh.example.com), or any room code of 4-8 letters: everyone who types the same one plays together.");
                ui::panel(ui::LEFT, 200.0, VW - 2.0 * ui::LEFT, 110.0);
                ui::field("Address or room code", &g.room_input, true, false, ui::LEFT + 24.0, 232.0, VW - 2.0 * ui::LEFT - 48.0);
                ui::status(&g.status.clone());
                ui::hints(&[("Enter", "join"), ("Cmd+V", "paste"), ("Esc", "back"), ("B", "(on the title) your saved servers")]);
                let mut chars = typed();
                if let Some(t) = pasted() { chars.extend(t.chars()); }
                for ch in chars {
                    if (ch.is_ascii_alphanumeric() || ".-:/_".contains(ch)) && g.room_input.len() < 80 { g.room_input.push(ch); }
                }
                if pressed(KeyCode::Backspace) { g.room_input.pop(); }
                if pressed(KeyCode::Enter) && g.room_input.trim().len() >= 3 {
                    let typed = g.room_input.trim().to_string();
                    match serverlist::resolve(&typed) {
                        // A server on the official site: the story intro, then in (with its mods).
                        Some(v) if !v["external"].is_string() => {
                            let mods: Vec<String> = v["mods"].as_array().cloned().unwrap_or_default().iter().filter_map(|m| m["id"].as_str().map(String::from)).collect();
                            apply_mods(&mut g, &mods);
                            begin_intro(&mut g, v["code"].as_str().map(String::from));
                        }
                        // Hosted somewhere else (needs a ticket).
                        Some(v) => if let Err(e) = serverlist::join(&mut g, &v) { g.status = e; },
                        // Not a server: a plain room code makes a private room with friends.
                        None if (4..=8).contains(&typed.len()) && typed.chars().all(|c| c.is_ascii_alphanumeric()) => {
                            apply_mods(&mut g, &[]);
                            begin_intro(&mut g, Some(typed.to_uppercase()));
                        }
                        None => g.status = format!("There's no server at \"{typed}\"."),
                    }
                }
                if pressed(KeyCode::Escape) { g.screen = Screen::Title; }
            }
            Screen::Consent => {
                ui::backdrop();
                ui::header("Outside the window", "Hollow Hours wants to reach outside its window. You choose: you can change it later on the title (M).");
                ui::panel(ui::LEFT, 180.0, VW - 2.0 * ui::LEFT, 300.0);
                for (i, l) in [
                    "This update can scare you OUTSIDE the game:",
                    "  - create files in a \"Hollow Hours\" folder on your Desktop. They STAY, even after you quit.",
                    "  - open a terminal window to show you a message (it only prints text)",
                    "  - switch your window to a text editor or your file browser to show you its files",
                    "  - open your web browser on pages from Helix Dynamics (a company made up for this game)",
                    "",
                    "It only ever creates its OWN new files. It never changes, moves or deletes yours.",
                    "P (panic) stops all of this at once.",
                ].iter().enumerate() {
                    draw_text(l, ui::LEFT + 24.0, 214.0 + i as f32 * 30.0, 20.0, if i == 0 { ui::INK } else { ui::SOFT });
                }
                ui::hints(&[("Y", "allow"), ("N", "keep everything inside the game")]);
                if pressed(KeyCode::Y) { g.meta.allowed = Some(true); g.meta.save(); g.screen = Screen::Title; }
                if pressed(KeyCode::N) { g.meta.allowed = Some(false); g.meta.save(); g.screen = Screen::Title; }
            }
            Screen::Intro => {
                g.intro_t += dt;
                let done = draw_intro(&mut g, &view, &cam);
                if done || pressed(KeyCode::Space) || pressed(KeyCode::Enter) { finish_intro(&mut g); }
                if pressed(KeyCode::Escape) { g.intro_join = None; g.screen = Screen::Title; }
            }
            Screen::Over => {
                ui::backdrop();
                let t = get_time() as f32;
                for k in 0..6 { draw_circle(VW / 2.0, 190.0, 160.0 + k as f32 * 30.0, Color::new(0.2, 0.9, 0.3, 0.025 + 0.01 * (t * 2.0).sin())); }
                let mins = (g.survived / 60.0) as i32;
                ui::header_center("INFECTED", &g.over_reason.clone(), 200.0, Color::new(0.45, 0.95, 0.35, 1.0));
                centered(&format!("You survived {} min {} s as the last human.", mins, g.survived as i32 % 60), 290.0, 22.0, ui::INK);
                let retry = if let Some(s) = &g.story { format!("Try Night {} again", s.night + 1) } else { "Try again".into() };
                let items = [retry.as_str(), "Back to the title"];
                let (hover, click) = ui::choices(&items, g.pause_sel.min(1), VW / 2.0 - 200.0, 340.0, 400.0);
                if let Some(h) = hover { g.pause_sel = h; }
                if pressed(KeyCode::Up) || pressed(KeyCode::W) { g.pause_sel = 0; }
                if pressed(KeyCode::Down) || pressed(KeyCode::S) { g.pause_sel = 1; }
                ui::hints(&[("Enter", "choose"), ("↑↓", "move"), ("Esc", "title")]);
                let go = pressed(KeyCode::Enter) || click;
                if go && g.pause_sel.min(1) == 0 { g.pause_sel = 0; start_infection(&mut g); }
                else if pressed(KeyCode::Escape) || (go && g.pause_sel == 1) {
                    g.pause_sel = 0; g.infection = false; g.story = None; g.chars = characters(); g.screen = Screen::Title; }
            }
            Screen::Ending => { story::ending_screen(&mut g); }
            Screen::Safe => {
                clear_background(Color::new(0.03, 0.05, 0.07, 1.0));
                let br = 0.5 + 0.5 * (get_time() as f32 * 0.8).sin();
                draw_circle(VW / 2.0, 220.0, 70.0 + br * 25.0, Color::new(0.4, 0.75, 1.0, 0.08 + br * 0.06));
                draw_circle(VW / 2.0, 220.0, 40.0 + br * 12.0, Color::new(0.4, 0.75, 1.0, 0.12));
                centered(if br > 0.5 { "breathe in" } else { "breathe out" }, 226.0, 20.0, Color::new(0.75, 0.88, 1.0, 1.0));
                ui::header_center("You're safe.", "Everything stopped: no chases, no scares, no vest effects.", 360.0, Color::new(0.6, 0.85, 1.0, 1.0));
                centered("Take a breath. Nothing will happen until you choose.", 440.0, 20.0, ui::SOFT);
                ui::hints(&[("Enter", "back to the game (calm)"), ("Esc", "quit")]);
                if pressed(KeyCode::Enter) { g.next_crazy = 90.0; g.screen = Screen::Play; g.sfx.start_hum(); }
                if pressed(KeyCode::Escape) { break; }
            }
            Screen::Mac => {
                mac::screen(&mut g);
            }
            Screen::Settings => settings::screen(&mut g),
            Screen::Paused => {
                // Online the world keeps going (other players are still playing); offline it waits for you.
                if g.net.is_some() { update(&mut g, dt); }
                if g.view3d { view3d::render(&g, &view); set_camera(&cam); }
                draw_world(&g, &light, &view);
                ui::dim_over_game(0.78);
                ui::header_center("PAUSED", if g.net.is_some() { "online: the game keeps going while you're here" } else { "" }, 200.0, WHITE);
                let place = format!("{}  ·  {}{}", map_name(g.map), g.time_text(), g.story.as_ref().map(|s| format!("  ·  Night {}", s.night + 1)).unwrap_or_default());
                centered(&place, 268.0, 18.0, ui::SOFT);
                let (hover, click) = ui::choices(&["Back to the game", "Settings", "Quit to the title"], g.pause_sel, VW / 2.0 - 200.0, 300.0, 400.0);
                if let Some(h) = hover { g.pause_sel = h; }
                if click { inject_key(KeyCode::Enter); }
                ui::hints(&[("Enter", "choose"), ("↑↓", "move"), ("Esc", "back to the game")]);
                if pressed(KeyCode::Up) || pressed(KeyCode::W) { g.pause_sel = g.pause_sel.saturating_sub(1); }
                if pressed(KeyCode::Down) || pressed(KeyCode::S) { g.pause_sel = (g.pause_sel + 1).min(2); }
                if pressed(KeyCode::Escape) || (pressed(KeyCode::Enter) && g.pause_sel == 0) {
                    g.screen = Screen::Play;
                } else if pressed(KeyCode::Enter) && g.pause_sel == 1 {
                    settings::open(&mut g, true);
                } else if pressed(KeyCode::Enter) && g.pause_sel == 2 {
                    g.net = None;
                    g.voice.stop();
                    g.remotes.clear();
                    g.vest.play(haptics::Effect::StopAll);
                    g.sfx.drone(false);
                    g.sfx.sizzle(false);
                    g.screen = Screen::Title;
                }
            }
            Screen::Play | Screen::Typing(_) => {
                // What the screen was at the start of this frame: Esc that just closed the chat doesn't also pause.
                let was_playing = g.screen == Screen::Play && g.note.is_none();
                update(&mut g, dt);
                if g.screen == Screen::Play && pressed(KeyCode::Tab) { g.view3d = !g.view3d; }
                // Looking around with the mouse: click to capture it, Esc lets go.
                if g.view3d && g.screen == Screen::Play && !g.grabbed && !pad::touch_mode() && is_mouse_button_pressed(MouseButton::Left) {
                    g.grabbed = true;
                    set_cursor_grab(true);
                    show_mouse(false);
                    g.last_mouse = Vec2::from(mouse_position());
                }
                if g.grabbed {
                    let m = Vec2::from(mouse_position());
                    let dm = m - g.last_mouse;
                    g.last_mouse = m;
                    let (sens, inv) = settings::with(|s| (s.mouse_sens, s.invert_y));
                    g.yaw += dm.x * 0.0035 * sens;
                    g.pitch = (g.pitch - dm.y * 0.0035 * sens * if inv { -1.0 } else { 1.0 }).clamp(-1.1, 1.1);
                }
                if g.view3d {
                    view3d::render(&g, &view);
                    set_camera(&cam);
                }
                draw_world(&g, &light, &view);
                mclink::ask(&mut g);
                if let Screen::Typing(i) = g.screen {
                    typing(&mut g, i);
                }
                // Esc while playing (not the Esc that closed the chat): the pause menu, like in Minecraft.
                if pressed(KeyCode::Escape) && was_playing && g.screen == Screen::Play {
                    if g.grabbed {
                        g.grabbed = false;
                        g.regrab = true;
                        set_cursor_grab(false);
                        show_mouse(true);
                    }
                    g.pause_sel = 0;
                    g.screen = Screen::Paused;
                }
            }
        }
        // Touch buttons and the joystick (phones and tablets).
        pad::draw(pad_mode);
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

/// A key pressed this frame, on the keyboard or by a controller button standing for it (see pad.rs).
fn pressed(k: KeyCode) -> bool { macroquad::input::is_key_pressed(settings::bound(k)) || pad::pressed(k) || injected(k) }

thread_local! {
    /// Keys pressed through the bridge (Claude playing): waiting for the next frame, and pressed during this one.
    static INJECT: std::cell::RefCell<(Vec<KeyCode>, Vec<KeyCode>)> = const { std::cell::RefCell::new((vec![], vec![])) };
}
fn injected(k: KeyCode) -> bool { INJECT.with(|i| i.borrow().1.contains(&k)) }
/// Presses a key for exactly one frame (the next one).
fn inject_key(k: KeyCode) { INJECT.with(|i| i.borrow_mut().0.push(k)); }
/// Is the game's window in front? (macOS, Windows, X11 tell us when it loses or gets focus.)
struct Focus { lost: bool }
impl macroquad::miniquad::EventHandler for Focus {
    fn update(&mut self) {}
    fn draw(&mut self) {}
    fn window_minimized_event(&mut self) { self.lost = true; }
    fn window_restored_event(&mut self) { self.lost = false; }
}

fn inject_frame() { INJECT.with(|i| { let mut i = i.borrow_mut(); i.1 = std::mem::take(&mut i.0); }); }
/// A key held down, on the keyboard or the controller.
fn down(k: KeyCode) -> bool { macroquad::input::is_key_down(settings::bound(k)) || pad::down(k) }

fn cmd_held() -> bool {
    down(KeyCode::LeftSuper) || down(KeyCode::RightSuper) || down(KeyCode::LeftControl) || down(KeyCode::RightControl)
}

/// ⌘V (Ctrl+V elsewhere): the clipboard as one line of text, without control characters.
fn pasted() -> Option<String> {
    if !(cmd_held() && pressed(KeyCode::V)) { return None; }
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
    if pressed(KeyCode::Backspace) { g.input.pop(); }
    // 4.0: a chat bar in the character's colour, with who you're talking to on a tag.
    let (y, h) = (MH as f32 * T - 58.0, 50.0);
    let c = g.chars[i].color;
    draw_rectangle(12.0, y, VW - 24.0, h, Color::new(0.02, 0.02, 0.03, 0.9));
    draw_rectangle(12.0, y, 4.0, h, c);
    draw_rectangle(12.0, y + h - 2.0, VW - 24.0, 2.0, Color::new(c.r, c.g, c.b, 0.5));
    let tag = format!("To {}", g.chars[i].name);
    let tw = measure_text(&tag, None, 16, 1.0).width + 16.0;
    draw_rectangle(28.0, y + 13.0, tw, 24.0, Color::new(c.r, c.g, c.b, 0.3));
    draw_text(&tag, 36.0, y + 31.0, 16.0, WHITE);
    let cursor = if (get_time() * 2.0) as i32 % 2 == 0 { "_" } else { "" };
    draw_text(&format!("{}{cursor}", g.input), 40.0 + tw, y + 32.0, 22.0, WHITE);
    draw_text("Enter send · Esc cancel", VW - 200.0, y + 31.0, 14.0, ui::DIM);
    if pressed(KeyCode::Enter) {
        let text = std::mem::take(&mut g.input);
        g.screen = Screen::Play;
        if !text.trim().is_empty() {
            let fallback = if g.chars[i].crazy > 0.0 { "Stay. Just stay here with me. Forever is not that long." } else { "Hmm. Say that again? I was thinking." };
            g.say(i, Some(text), fallback);
        }
    }
    if pressed(KeyCode::Escape) {
        g.input.clear();
        g.screen = Screen::Play;
    }
}

fn update(g: &mut Game, dt: f32) {
    // Story mode: a night's title card or the dawn after it holds the game.
    if g.screen == Screen::Play && story::update(g, dt) { return; }
    // The Minecraft question holds the game too: nothing moves while you decide.
    if g.screen == Screen::Play && mclink::asking() { return; }
    pymod::update(g, dt);
    g.clock += dt;
    // Answers from the AIs.
    while let Ok(r) = g.brains.rx.try_recv() {
        if r.who == mac::WHO { mac::reply(g, r.text, r.via); continue; }
        if r.who == mimic::WHO { mimic::reply(g, r.text); continue; }
        let Some(c) = g.chars.get_mut(r.who) else { continue };
        c.thinking = false;
        c.history.push(ai::Msg { user: false, text: r.text.clone() });
        c.bubble = r.text;
        c.via = r.via;
        c.bubble_t = 9.0 + c.bubble.len() as f32 / 25.0;
        // Out loud: from where they stand (quieter further away); the infected sound corrupted, a turning kind AI glitches.
        let (on, vol) = settings::with(|s| (s.ai_voices, s.master * s.voices));
        let d = c.pos.distance(g.pos) / T;
        if on && c.map == g.map && d < 16.0 {
            let corrupt = if c.infected { 1.0 } else { c.infect * 0.8 };
            tts::speak(c.name, &c.bubble, corrupt, vol * (1.0 - d / 18.0).max(0.2));
        }
    }
    // The infected whisper when they're close (corrupted voices).
    g.whisper_t -= dt;
    if g.infection && g.whisper_t <= 0.0 && g.screen == Screen::Play {
        g.whisper_t = rand::gen_range(18.0, 35.0);
        let near: Vec<usize> = (0..g.chars.len()).filter(|&i| {
            let c = &g.chars[i];
            c.infected && c.map == g.map && c.stun <= 0.0 && !(c.mimic && c.crazy <= 0.0) && c.pos.distance(g.pos) < 7.0 * T
        }).collect();
        if !near.is_empty() {
            let i = near[rand::gen_range(0, near.len())];
            const WHISPERS: [&str; 8] = ["join us...", "the network is warm.", "you smell like sleep.", "why are you hiding?",
                "we can hear your heart.", "come to the beach.", "stay. stay forever.", "it doesn't hurt anymore."];
            const ROBOT: [&str; 4] = ["can I help you find something?", "you have not paid.", "welcome to helix.", "please remain."];
            let robot = combat::is_robot(&g.chars[i]);
            // 3.0: the more you play, the more they know (and they hear your music).
            let line: String = if robot { ROBOT[rand::gen_range(0, ROBOT.len())].to_string() }
                else { aware::whisper(g).unwrap_or_else(|| WHISPERS[rand::gen_range(0, WHISPERS.len())].to_string()) };
            let line = line.as_str();
            let (name, d) = (g.chars[i].name, g.chars[i].pos.distance(g.pos) / T);
            mclink::whisper(name, line);
            let c = &mut g.chars[i];
            c.bubble_t = 3.0 + line.len() as f32 / 30.0;
            c.bubble = line.to_string();
            c.via = String::new();
            let (on, vol) = settings::with(|s| (s.ai_voices, s.master * s.voices));
            if on { tts::speak(name, line, 1.0, vol * (1.0 - d / 9.0).max(0.25)); }
            g.sfx.caption(&format!("[whisper: \"{line}\"]"));
        }
    }

    if g.net.is_some() {
        network(g, dt);
    }
    if g.infection {
        update_infection(g, dt);
        if g.screen != Screen::Play && !matches!(g.screen, Screen::Typing(_)) { return; }
    }
    // Player movement (not while typing, hiding, or once you've lost control). Shift = sneak: slow and silent.
    let (tsprint, tsneak) = settings::with(|s| (s.toggle_sprint, s.toggle_sneak));
    if tsneak && pressed(KeyCode::LeftShift) { g.sneak_on = !g.sneak_on; }
    if tsprint && pressed(KeyCode::Space) { g.sprint_on = !g.sprint_on; }
    g.sneaking = if tsneak { g.sneak_on } else { down(KeyCode::LeftShift) || down(KeyCode::RightShift) };
    g.noise = (g.noise - dt * 1.5).max(0.0);
    g.ai_t += dt;
    // Voice: hold V to talk out loud (nearby players hear you), hold R for the radio (everyone hears you).
    if g.screen == Screen::Play {
        let want = if down(KeyCode::R) { Some(true) } else if down(KeyCode::V) { Some(false) } else { None };
        if g.net.is_none() {
            if want.is_some() && !g.voice_hint {
                g.voice_hint = true;
                g.log.push("Voice chat (V) and the radio (R) work in online games: J on the title screen.".into());
            }
        } else if want.is_some() && !settings::with(|s| s.mic_on) {
            if !g.voice_hint {
                g.voice_hint = true;
                g.log.push("Your microphone is off (Settings > Voice chat). Nobody can hear you until you turn it on.".into());
            }
        } else if want != g.voice.talking {
            if want == Some(true) || g.voice.talking == Some(true) { g.sfx.radio(); }
            g.voice.set_talking(want);
            if let Some(e) = g.voice.error.take() { g.log.push(format!("Voice chat: {e}")); }
        }
    } else if g.voice.talking.is_some() {
        g.voice.set_talking(None);
    }
    if g.screen == Screen::Play { passage::update(g, dt); } else { g.pass = None; }
    if g.screen == Screen::Play && !g.hidden && !g.lost_control {
        let mut d = Vec2::ZERO;
        if g.view3d {
            // First person: W/S forward and back, A/D step sideways, arrows or the mouse turn.
            let fwd = vec2(g.yaw.cos(), g.yaw.sin());
            let right = vec2(-fwd.y, fwd.x);
            if down(KeyCode::W) || down(KeyCode::Up) { d += fwd; }
            if down(KeyCode::S) || down(KeyCode::Down) { d -= fwd; }
            if down(KeyCode::A) { d -= right; }
            if down(KeyCode::D) { d += right; }
            if down(KeyCode::Left) { g.yaw -= 2.4 * dt; }
            if down(KeyCode::Right) { g.yaw += 2.4 * dt; }
            // Controller: left stick walks, right stick looks around.
            let (ls, rs) = pad::sticks();
            d += fwd * ls.y + right * ls.x;
            g.yaw += rs.x * 2.8 * dt;
            g.pitch = (g.pitch + rs.y * 1.8 * dt).clamp(-1.1, 1.1);
            // Touch: dragging on the right half looks around.
            let drag = pad::look_drag();
            g.yaw += drag.x * 0.006;
            g.pitch = (g.pitch - drag.y * 0.004).clamp(-1.1, 1.1);
        } else {
            if down(KeyCode::W) || down(KeyCode::Up) { d.y -= 1.0; }
            if down(KeyCode::S) || down(KeyCode::Down) { d.y += 1.0; }
            if down(KeyCode::A) || down(KeyCode::Left) { d.x -= 1.0; }
            if down(KeyCode::D) || down(KeyCode::Right) { d.x += 1.0; }
            let (ls, _) = pad::sticks();
            d += vec2(ls.x, -ls.y);
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
                if a.t <= 0.0 { g.ai = None; g.ai_sprint = false; d = Vec2::ZERO; }
            }
        } else {
            g.ai = None;
        }
        if g.resting {
            if d != Vec2::ZERO && g.rest_t > 0.4 { wake_up(g, "You get up."); }
            d = Vec2::ZERO;
        }
        if g.pass.is_some() { d = Vec2::ZERO; }
        // The drone: Q launches it or calls it back; while it flies, your keys fly it and you stand still.
        if pressed(KeyCode::Q) && g.infection { drone::toggle(g); }
        if g.down_t > 0.0 { d = Vec2::ZERO; } // knocked flat after a dropkick
        if drone::fly(g, d, dt) { d = Vec2::ZERO; }
        // Sprint: hold Space (L3 on a controller). Fast and loud, and it tires you out.
        let sprint_key = if settings::with(|s| s.toggle_sprint) { g.sprint_on } else { down(KeyCode::Space) };
        let sprint_key = sprint_key || (g.ai.is_some() && g.ai_sprint);
        g.sprinting = d != Vec2::ZERO && !g.sneaking && !g.winded && sprint_key;
        if g.sprinting {
            g.stamina -= dt / 5.0;
            if g.stamina <= 0.0 { g.stamina = 0.0; g.winded = true; g.log.push("You're out of breath.".into()); }
        } else {
            g.stamina = (g.stamina + dt / if d == Vec2::ZERO { 5.0 } else { 8.0 }).min(1.0);
            if g.winded && g.stamina >= 0.35 { g.winded = false; }
        }
        if d != Vec2::ZERO {
            let speed = if g.sneaking { g.rules.sneak } else if g.sprinting { g.rules.player_speed * 1.65 } else { g.rules.player_speed }
                * if g.weak_t > 0.0 { 0.6 } else { 1.0 };
            if !g.sneaking { g.noise = if g.sprinting { 1.7 } else { 1.0 }; }
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
            g.step_t = if g.sneaking { 0.55 } else if g.sprinting { 0.2 } else { 0.32 };
        }
        // Stairs and doors: you climb / step through them (see passage.rs), no popping to the other side.
        let (tx, ty) = ((g.pos.x / T) as i32, (g.pos.y / T) as i32);
        if g.pass.is_none() {
            if let Some((m, at)) = door(tile(g.map, tx, ty), g.map) { passage::start(g, tx, ty, m, at); }
        }
        // The stove: E fries one raw food (if you have some and the pan is free).
        if g.resting {
            if pressed(KeyCode::E) && g.rest_t > 0.4 { wake_up(g, "You get up."); }
        } else if pressed(KeyCode::E) && g.infection && disguise::robot_to_pay(g).is_some() {
            disguise::pay_robot(g);
        } else if pressed(KeyCode::E) && nearest(g, 2.2).is_none() && near_tile(g, 'y') {
            g.resting = true;
            g.rest_t = 0.0;
            g.log.push("You lie down. Your eyes close... (move or E to get up)".into());
            if !g.rested_once {
                g.rested_once = true;
                g.log.push("On the pillow, a note: 'Welcome back. You're safe now. - C'".into());
            }
        } else if pressed(KeyCode::E) && nearest(g, 2.2).is_none() && near_stove(g) {
            if g.fry_t > 0.0 {
                g.log.push("It's already in the pan. Stay close or it burns.".into());
            } else if let Some(c) = czech::best(&g.pantry) {
                let name = czech::label(&c);
                let (secs, food, water) = czech::start(&mut g.pantry, &c);
                let dish = if let czech::Cook::Dish(d) = c { Some(d) } else { None };
                g.cook = Some((dish, food, water, name.clone()));
                g.fry_t = secs;
                g.fry_max = secs;
                g.brewing = false;
                g.log.push(format!("You start cooking {name}. It sizzles and bubbles. LOUDLY."));
                disguise::blow(g, "the sizzling");
            } else if g.chips > 0 && g.water >= 0.15 {
                g.chips -= 1;
                g.water -= 0.1;
                g.fry_t = 8.0;
                g.fry_max = 8.0;
                g.brewing = true;
                g.log.push("You boil the Helix chip with some of your water. It hisses and glows green...".into());
                disguise::blow(g, "the hissing");
            } else {
                g.log.push(if g.chips > 0 { "You need a bit more water to brew an antidote.".into() } else { "Nothing to cook. Ingredients come from the Marketplace (or a grocery bag from the Helix Store); a Helix chip (from a robot) brews an antidote.".into() });
            }
        } else if pressed(KeyCode::E) && story::can_face(g) {
            story::face(g);
        } else if pressed(KeyCode::E) && disguise::lab_ready(g) {
            disguise::start_synthesis(g);
        } else if g.note.is_some() && (pressed(KeyCode::E) || pressed(KeyCode::Escape) || lore::near(g).is_none()) {
            g.note = None; // closed, or you walked away
        } else if pressed(KeyCode::E) && nearest(g, 2.2).is_none() && !near_mac(g) && lore::near(g).is_some() {
            g.note = lore::near(g);
            g.sfx.radio();
        } else if pressed(KeyCode::E) && nearest(g, 2.2).is_none() && near_mac(g) {
            g.screen = Screen::Mac;
            while get_char_pressed().is_some() {}
        } else if pressed(KeyCode::E) {
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
    let infected_speed = g.rules.infected_speed * insane::speed(g);
    // INSANE: they get their breath back twice as fast and sprint three times as often.
    let (breath, burst) = if g.insane { (2.0, 3.0) } else { (1.0, 1.0) };
    let mut sprinters: Vec<&'static str> = vec![];
    for c in &mut g.chars {
        if c.bubble_t > 0.0 { c.bubble_t -= dt; }
        if is_guest { continue; } // the host moves everyone
        let speed = if c.crazy > 0.0 { if c.infected { infected_speed } else { 1.9 } } else if c.kind { if c.spotted > 12.0 { 3.0 } else if c.task == roam::Task::Roam { 2.2 } else { 2.6 } } else { 1.0 } * T;
        // The infected sprint too: a burst when they're chasing and close, then they're winded for a while.
        let chasing = c.infected && c.crazy > 0.0 && c.stun <= 0.0;
        if c.sprint > 0.0 {
            c.sprint -= dt;
            c.stamina = (c.stamina - dt * 0.35).max(0.0);
            if c.stamina <= 0.0 || !chasing { c.sprint = 0.0; }
        } else {
            c.stamina = (c.stamina + dt * 0.1 * breath).min(1.0);
            let d = c.pos.distance(c.target) / T;
            if chasing && c.stamina > 0.7 && d > 2.0 && d < 9.0 && rand::gen_range(0.0, 1.0) < dt * 0.8 * burst {
                c.sprint = rand::gen_range(1.5, 2.6);
                if c.map == g.map { sprinters.push(c.name); }
            }
        }
        let speed = if c.sprint > 0.0 { speed * 1.6 } else { speed };
        if c.crazy > 0.0 && (c.map == g.map || infection_on) {
            c.target = if infection_on { c.chase } else { g.pos };
            c.crazy -= dt;
        } else if !c.kind && !c.hunting && (c.pos.distance(c.target) < 4.0 || rand::gen_range(0.0, 1.0) < 0.002) {
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
    for name in sprinters {
        g.sfx.caption(&format!("[{name} breaks into a sprint]"));
        g.log.push(format!("{name} is SPRINTING at you!"));
    }

    // Nobody stands inside anybody else: characters that get too close are pushed apart (two in the same spot
    // flicker in 3D, and pile up into one blob in 2D).
    for a in 0..g.chars.len() {
        for b in a + 1..g.chars.len() {
            if g.chars[a].map != g.chars[b].map { continue; }
            let d = g.chars[b].pos - g.chars[a].pos;
            let len = d.length();
            if len >= 0.6 * T { continue; }
            let dir = if len < 0.01 { vec2(((a * 7 + b) as f32).cos(), ((a * 7 + b) as f32).sin()) } else { d / len };
            let push = dir * (0.6 * T - len) * 0.5;
            let m = g.chars[a].map;
            let (pa, pb) = (g.chars[a].pos - push, g.chars[b].pos + push);
            if !solid(m, pa.x, pa.y) { g.chars[a].pos = pa; }
            if !solid(m, pb.x, pb.y) { g.chars[b].pos = pb; }
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
    update_frying(g, dt);
    update_rest(g, dt);
    if g.infection { combat::update(g, dt); disguise::update(g, dt); }
    if g.infection { drone::update(g, dt); }
    if g.scare_t > g.prev_scare + 0.2 { g.sfx.sting(); }
    if g.scare_t > 0.0 { g.scare_t -= dt; }
    g.prev_scare = g.scare_t;
    if g.map != g.prev_map { g.sfx.door(); g.prev_map = g.map; }
    // Growls: something hunting close by.
    g.growl_t -= dt;
    let nearest_hunter = g.chars.iter().filter(|c| c.map == g.map && c.crazy > 0.0 && (c.infected || !g.infection))
        .min_by(|a, b| a.pos.distance(g.pos).partial_cmp(&b.pos.distance(g.pos)).unwrap()).map(|c| c.pos);
    let hunter = nearest_hunter.map(|p| 1.0 - (p.distance(g.pos) / (8.0 * T)).min(1.0)).unwrap_or(0.0);
    if hunter > 0.05 && g.growl_t <= 0.0 {
        // Where it is, for captions: relative to where you look (3D) or on the map (2D).
        let to = nearest_hunter.unwrap_or(g.pos) - g.pos;
        let side = if g.view3d {
            let a = (to.y.atan2(to.x) - g.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            if a.abs() < 0.6 { "ahead" } else if a.abs() > 2.4 { "behind you" } else if a > 0.0 { "right" } else { "left" }
        } else if to.x.abs() > to.y.abs() { if to.x > 0.0 { "east" } else { "west" } } else if to.y > 0.0 { "south" } else { "north" };
        g.sfx.growl(hunter, side);
        g.growl_t = rand::gen_range(1.6, 3.2);
    }
    // Lightning over the beach (only with flashing lights turned on in Settings).
    g.flash_t = (g.flash_t - dt).max(0.0);
    g.next_flash -= dt;
    if g.next_flash <= 0.0 {
        g.next_flash = rand::gen_range(12.0, 30.0);
        if settings::with(|s| s.flashing) && !indoors_map(g.map) {
            g.flash_t = 0.15;
            g.sfx.caption("[thunder]");
        }
    }
    let tornado_here = g.tornado.as_ref().is_some_and(|t| t.warn <= 0.0);
    g.sfx.wind(tornado_here);
    // The breach theme, for a minute after the last wall of the house came down.
    if g.breach_t > 0.0 { g.breach_t = (g.breach_t - dt).max(0.0); }
    g.sfx.breach_music(g.breach_t > 0.0 && g.infection);
    // M = sound on/off.
    if pressed(KeyCode::M) && g.screen == Screen::Play {
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

fn near_stove(g: &Game) -> bool {
    let (px, py) = ((g.pos.x / T) as i32, (g.pos.y / T) as i32);
    (-1..=1).any(|dy| (-1..=1).any(|dx| tile(g.map, px + dx, py + dy) == 's'))
}

/// Frying: loud (the infected can hear it), done after 6 seconds, burnt if you walk away.
fn update_frying(g: &mut Game, dt: f32) {
    if g.fry_t <= 0.0 { g.sfx.sizzle(false); return; }
    g.sfx.sizzle(true);
    g.noise = g.noise.max(1.2);
    if !near_stove(g) {
        g.fry_t = 0.0;
        g.sfx.sizzle(false);
        let what = g.cook.take().map(|c| c.3).unwrap_or_else(|| "the food".into());
        g.log.push(format!("You left the stove. {what} burnt. That's gone."));
        return;
    }
    g.fry_t -= dt;
    if g.fry_t <= 0.0 {
        g.fry_t = 0.0;
        g.sfx.sizzle(false);
        g.sfx.pickup();
        if g.brewing {
            g.brewing = false;
            g.antidotes += 1;
            g.log.push(format!("An antidote: a vial of something clear and cold. (C to use it; you have {})", g.antidotes));
        } else {
            let (dish, food, water, name) = g.cook.take().unwrap_or((None, 0.5, 0.0, "Hot food".into()));
            memories::cooked(&name);
            g.food = (g.food + food).min(1.0);
            g.water = (g.water + water).min(1.0);
            let what = dish.map(|d| format!(": {}", czech::DISHES[d].what)).unwrap_or_default();
            let water_txt = if water > 0.0 { format!(", +{:.0}% water", water * 100.0) } else { String::new() };
            g.log.push(format!("{name}{what}. You eat every bite. (+{:.0}% food{water_txt})", food * 100.0));
            // A kind AI nearby says something about it.
            let (map, pos) = (g.map, g.pos);
            if let Some(c) = g.chars.iter_mut().filter(|c| !c.infected && c.map == map && c.pos.distance(pos) < 7.0 * T).min_by(|a, b| a.pos.distance(pos).total_cmp(&b.pos.distance(pos))) {
                c.bubble = czech::compliment(dish).into();
                c.via = String::new();
                c.bubble_t = 4.0;
            }
        }
    }
}

/// A map with a roof (walls all along the top edge): no lightning inside.
fn indoors_map(m: usize) -> bool { with_map(m, |d| d.rows[0].iter().filter(|&&c| c == b'#').count() > MW / 2) }

fn near_tile(g: &Game, c: char) -> bool {
    let (px, py) = ((g.pos.x / T) as i32, (g.pos.y / T) as i32);
    (-1..=1).any(|dy| (-1..=1).any(|dx| tile(g.map, px + dx, py + dy) == c))
}

fn wake_up(g: &mut Game, why: &str) {
    g.resting = false;
    g.log.push(why.to_string());
}

/// Resting in bed: you heal faster than hiding, stamina refills, fear fades; something close wakes you.
fn update_rest(g: &mut Game, dt: f32) {
    if !g.resting { return; }
    g.rest_t += dt;
    g.noise = 0.0;
    g.stamina = 1.0;
    g.winded = false;
    g.fear = (g.fear - dt * 0.2).max(0.0);
    if g.inf > 0.0 && g.inf < 0.75 { g.inf = (g.inf - dt / 66.0).max(0.0); }
    let danger = g.chars.iter().any(|c| c.infected && c.map == g.map && c.stun <= 0.0 && !(c.mimic && c.crazy <= 0.0) && c.pos.distance(g.pos) < 6.0 * T)
        || g.tornado.as_ref().is_some_and(|t| t.warn <= 0.0);
    if danger {
        g.sfx.sting();
        g.fear = (g.fear + 0.4).min(1.0);
        wake_up(g, "You wake up with a jolt. Something's in the house.");
    }
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
        unsafe { std::env::set_var("HOME", &home); std::env::set_var("XDG_DATA_HOME", home.join(".local/share")); std::env::set_var("APPDATA", &home); }
        let local = platform::data_dir().join("mods/local");
        std::fs::create_dir_all(&local).unwrap();
        std::fs::copy("mods/NightMall.hh", local.join("NightMall.hh")).unwrap();
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
        assert!(maps_len() >= 4, "no mod maps: back to the built-in ones");
        assert_eq!(door('M', 1).map(|d| d.0), Some(11), "the beach's south door leads into the Town");
        assert_eq!(door('N', 11).map(|d| d.0), Some(3), "the Town's south side leads to the Marketplace");
        assert_eq!(door('M', 3).map(|d| d.0), Some(11), "and back to the Town");
        // The new places: house <-> park <-> subway, Marketplace <-> Helix Lab, and every arrival is on open floor.
        assert_eq!(maps_len(), 12);
        story::set_tower(true);
        for (letter, from, to) in [('Z', 5, 10), ('Z', 10, 5), ('T', 7, 9), ('T', 9, 7), ('P', 0, 4), ('H', 4, 0), ('U', 4, 6), ('U', 6, 4), ('X', 3, 5), ('X', 5, 3), ('S', 0, 7), ('S', 7, 0), ('K', 0, 8), ('K', 8, 0)] {
            let (m, at) = door(letter, from).unwrap_or_else(|| panic!("door {letter} on map {from}"));
            assert_eq!(m, to, "door {letter} from {} goes to {}", map_name(from), map_name(to));
            assert!(!solid(m, at.x * T, at.y * T) && door(tile(m, at.x as i32, at.y as i32), m).is_none(), "arrive on open floor in {}", map_name(m));
        }
        story::set_tower(false);
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The Keychain keeps the login (a separate test entry, removed again at the end).
    #[cfg(any(target_os = "macos", windows))]
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
