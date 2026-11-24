//! 4.0 phase 4: Minecraft, linked to Hollow Hours.
//!  - HH-Link (a NeoForge 1.21.1 mod, hh-link/): Minecraft asks the game's bridge (127.0.0.1, same token as the MCP)
//!    once a second what's happening ("mc_state") and shows it: your infection as a boss bar, the infected near you as
//!    green-eyed mobs, their whispers, storms for tornadoes, breaches as doors splintering. Things you do there come
//!    back ("mc_event"): eat in Minecraft and it counts here. With Deep meta on and your Yes, they BUILD in your world:
//!    green moss creeping, signs saying "come to the beach".
//!  - The infected can offer to open Minecraft ("we built you somewhere else."): a real Yes/No, your own launcher.
//!  - Settings > Minecraft: install HH-Link into an instance you pick; infect a world you pick with the
//!    hh_infection datapack (the world is backed up first; removing the datapack stops it).
//! Nothing here touches anything you didn't pick, and nothing personal goes to Minecraft.

use super::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

static LAST: AtomicU64 = AtomicU64::new(0);
static WHISPERS: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

fn now() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }

/// Is Minecraft (HH-Link) talking to us right now?
pub fn linked() -> bool { now().saturating_sub(LAST.load(Ordering::Relaxed)) < 5 }

/// An infected whispered something: Minecraft shows it too (only while linked).
pub fn whisper(who: &str, line: &str) {
    if !linked() { return; }
    if let Ok(mut w) = WHISPERS.lock() { if w.len() < 8 { w.push((who.into(), line.into())); } }
}

/// May the infected build in the linked Minecraft world? (Deep meta, Outside the window, offline, and the setting.)
fn may_build(g: &Game) -> bool {
    g.meta.allowed == Some(true) && g.net.is_none() && settings::with(|s| s.deep_meta && s.mc_build)
}

/// The bridge command "mc_state": what Minecraft should show.
pub fn state(g: &Game) -> serde_json::Value {
    LAST.store(now(), Ordering::Relaxed);
    let playing = g.infection && matches!(g.screen, Screen::Play | Screen::Typing(_) | Screen::Mac | Screen::Paused);
    let near: Vec<&str> = g.chars.iter().filter(|c| c.infected && c.map == g.map && c.pos.distance(g.pos) < 10.0 * T && !(c.mimic && c.crazy <= 0.0)).map(|c| c.name).take(4).collect();
    let whispers: Vec<serde_json::Value> = WHISPERS.lock().map(|mut w| w.drain(..).map(|(a, b)| json!({"who": a, "line": b})).collect()).unwrap_or_default();
    json!({
        "playing": playing,
        "infection": g.inf,
        "place": map_name(g.map),
        "night": g.story.as_ref().map(|s| s.night + 1),
        "infected_near": near,
        "tornado": g.tornado.as_ref().is_some_and(|t| t.warn <= 0.0),
        "breach": g.breach_t > 0.0,
        "whispers": whispers,
        "build": playing && may_build(g),
    })
}

/// The bridge command "mc_event": something happened in Minecraft.
pub fn event(g: &mut Game, a: &serde_json::Value) -> serde_json::Value {
    LAST.store(now(), Ordering::Relaxed);
    match a["event"].as_str().unwrap_or("") {
        "hello" => g.log.push("Minecraft is linked (HH-Link). Something on the other side is listening.".into()),
        "ate" if g.infection => {
            g.food = (g.food + 0.12).min(1.0);
            g.log.push("(Minecraft) You ate in the other world. Somehow it counts here too. Food +12%.".into());
        }
        "drank" if g.infection => g.water = (g.water + 0.12).min(1.0),
        "died" if g.infection => {
            g.inf = (g.inf + 0.1).min(0.99);
            g.log.push("(Minecraft) You died over there. Something green came back with you. Infection +10%.".into());
        }
        "slept" if g.infection => {
            g.inf = (g.inf - 0.05).max(0.0);
            g.log.push("(Minecraft) You slept in a bed over there. You feel a little more yourself here.".into());
        }
        _ => {}
    }
    json!({"ok": true})
}

// ---------------- Finding Minecraft ----------------

/// A Minecraft game folder we found: (name to show, its folder with mods/ and saves/).
#[derive(Clone)]
pub struct Install { pub name: String, pub dir: PathBuf }

/// Every Minecraft game folder on this computer (the official launcher, Prism, Modrinth App, CurseForge, MultiMC).
pub fn installs() -> Vec<Install> {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).map(PathBuf::from).unwrap_or_default();
    let app = if cfg!(target_os = "macos") { home.join("Library/Application Support") }
        else if cfg!(windows) { std::env::var("APPDATA").map(PathBuf::from).unwrap_or_else(|_| home.join("AppData/Roaming")) }
        else { home.join(".local/share") };
    let mut out = vec![];
    for official in [app.join("minecraft"), app.join(".minecraft"), home.join(".minecraft")] {
        if official.is_dir() { out.push(Install { name: "Minecraft (official launcher)".into(), dir: official }); }
    }
    for (launcher, root, inner) in [("Prism", app.join("PrismLauncher/instances"), ".minecraft"), ("Prism", app.join("PrismLauncher/instances"), "minecraft"),
        ("MultiMC", home.join("MultiMC/instances"), ".minecraft"), ("Modrinth", app.join("ModrinthApp/profiles"), ""), ("Modrinth", app.join("com.modrinth.theseus/profiles"), ""),
        ("CurseForge", home.join("curseforge/minecraft/Instances"), ""), ("CurseForge", home.join("Documents/curseforge/minecraft/Instances"), "")] {
        let Ok(rd) = std::fs::read_dir(&root) else { continue };
        for e in rd.flatten() {
            let dir = if inner.is_empty() { e.path() } else { e.path().join(inner) };
            if dir.is_dir() { out.push(Install { name: format!("{launcher}: {}", e.file_name().to_string_lossy()), dir }); }
        }
    }
    out.dedup_by(|a, b| a.dir == b.dir);
    out
}

/// Every single-player world in those folders: (shown name, its folder).
pub fn worlds() -> Vec<(String, PathBuf)> {
    let mut out = vec![];
    for i in installs() {
        let Ok(rd) = std::fs::read_dir(i.dir.join("saves")) else { continue };
        for e in rd.flatten() {
            if e.path().join("level.dat").exists() { out.push((format!("{} ({})", e.file_name().to_string_lossy(), i.name), e.path())); }
        }
    }
    out
}

const JAR: &str = "HH-Link-NeoForge-1.21.1.jar";

/// Puts HH-Link into this Minecraft's mods folder (downloads it from the game's releases first).
pub fn install(i: &Install) -> Result<String, String> {
    let cache = platform::data_dir().join(JAR);
    if !cache.exists() {
        let url = format!("https://github.com/jdj212223-ctrl/hollow-hours/releases/latest/download/{JAR}");
        let r = ureq::get(&url).set("User-Agent", "HollowHours").timeout(std::time::Duration::from_secs(60)).call().map_err(|e| format!("couldn't download HH-Link ({e})"))?;
        let mut f = std::fs::File::create(&cache).map_err(|e| e.to_string())?;
        std::io::copy(&mut r.into_reader(), &mut f).map_err(|e| e.to_string())?;
    }
    let mods = i.dir.join("mods");
    std::fs::create_dir_all(&mods).map_err(|e| e.to_string())?;
    std::fs::copy(&cache, mods.join(JAR)).map_err(|e| e.to_string())?;
    Ok(format!("HH-Link is in {}. It needs NeoForge for Minecraft 1.21.1 there. Start the game, then that instance.", i.name))
}

// ---------------- The hh_infection datapack ----------------

const PACK: &str = "hh_infection";

/// Backs the world up (a full copy in Hollow Hours' folder), then adds the datapack.
pub fn infect(world: &Path) -> Result<String, String> {
    let name = world.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let backup = platform::data_dir().join("minecraft-backups").join(format!("{name}-{}", now()));
    copy_dir(world, &backup).map_err(|e| format!("the backup failed, nothing was changed ({e})"))?;
    let pack = world.join("datapacks").join(PACK);
    for (path, text) in datapack() {
        let p = pack.join(path);
        if let Some(d) = p.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
        std::fs::write(p, text).map_err(|e| e.to_string())?;
    }
    Ok(format!("\"{name}\" is infected. Backup: {}. Open the world (or /reload) and wait for night.", backup.display()))
}

/// Takes the datapack out again (what it already changed stays: the backup has the world as it was).
pub fn cure(world: &Path) -> Result<String, String> {
    let pack = world.join("datapacks").join(PACK);
    if !pack.exists() { return Ok("That world isn't infected.".into()); }
    std::fs::remove_dir_all(&pack).map_err(|e| e.to_string())?;
    Ok("The hh_infection datapack is gone. (The moss it spread stays: your backup is in Hollow Hours' minecraft-backups folder.)".into())
}

pub fn infected(world: &Path) -> bool { world.join("datapacks").join(PACK).exists() }

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let t = to.join(e.file_name());
        if e.file_type()?.is_dir() { copy_dir(&e.path(), &t)?; } else if e.file_name() != "session.lock" { std::fs::copy(e.path(), t)?; }
    }
    Ok(())
}

/// The datapack's files (Minecraft 1.21 and newer: pack format 48 up).
pub fn datapack() -> Vec<(&'static str, String)> {
    let f = |s: &str| s.trim_start().to_string();
    vec![
        ("pack.mcmeta", f(r#"{"pack": {"pack_format": 48, "supported_formats": [48, 99], "description": "Hollow Hours: hh_infection. Something is spreading."}}"#)),
        ("data/minecraft/tags/function/load.json", f(r#"{"values": ["hh_infection:load"]}"#)),
        ("data/minecraft/tags/function/tick.json", f(r#"{"values": ["hh_infection:tick"]}"#)),
        ("data/hh_infection/predicate/sometimes.json", f(r#"{"condition": "minecraft:random_chance", "chance": 0.35}"#)),
        ("data/hh_infection/function/load.mcfunction", f(r#"
scoreboard objectives add hh dummy
tellraw @a [{"text":"[Hollow Hours] ","color":"dark_green"},{"text":"something is spreading.","color":"green","italic":true}]
"#)),
        ("data/hh_infection/function/tick.mcfunction", f(r#"
scoreboard players add #t hh 1
execute if score #t hh matches 200.. run function hh_infection:pulse
"#)),
        ("data/hh_infection/function/pulse.mcfunction", f(r#"
scoreboard players set #t hh 0
scoreboard players add #age hh 1
execute as @a[gamemode=!spectator] at @s run function hh_infection:spread
execute store result score #day hh run time query daytime
execute store result score #n hh if entity @e[type=minecraft:zombie,tag=hh_infected]
execute if score #day hh matches 13000..23000 if score #n hh matches ..7 if predicate hh_infection:sometimes as @a[gamemode=!spectator,limit=1,sort=random] at @s run function hh_infection:infected
execute as @a[tag=!hh_signed,gamemode=!spectator] at @s run function hh_infection:sign
execute if score #age hh matches 12.. unless score #tower hh matches 1 as @a[gamemode=!spectator,limit=1] at @s positioned ~36 ~ ~36 positioned over world_surface run function hh_infection:tower
"#)),
        ("data/hh_infection/function/spread.mcfunction", f(r#"
execute positioned ~ ~-1 ~ if block ~ ~ ~ minecraft:grass_block run setblock ~ ~ ~ minecraft:moss_block
summon minecraft:marker ~ ~ ~ {Tags:["hh_spread"]}
spreadplayers ~ ~ 1 9 false @e[type=minecraft:marker,tag=hh_spread]
execute as @e[type=minecraft:marker,tag=hh_spread] at @s positioned ~ ~-1 ~ if block ~ ~ ~ minecraft:grass_block run setblock ~ ~ ~ minecraft:moss_block
kill @e[type=minecraft:marker,tag=hh_spread]
"#)),
        ("data/hh_infection/function/infected.mcfunction", f(r#"
summon minecraft:zombie ~ ~ ~ {CustomName:'{"text":"Infected","color":"green"}',Glowing:1b,Tags:["hh_infected","hh_new"]}
spreadplayers ~ ~ 8 16 false @e[type=minecraft:zombie,tag=hh_new]
tag @e[type=minecraft:zombie,tag=hh_new] remove hh_new
tellraw @s {"text":"join us...","color":"green","italic":true}
"#)),
        ("data/hh_infection/function/sign.mcfunction", f(r#"
setblock ~2 ~ ~2 minecraft:oak_sign[rotation=8]{front_text:{messages:['"come to the"','"beach."','"3:00 AM"','""']}} keep
tag @s add hh_signed
"#)),
        ("data/hh_infection/function/tower.mcfunction", f(r#"
scoreboard players set #tower hh 1
fill ~-3 ~ ~-3 ~3 ~40 ~3 minecraft:black_concrete hollow
fill ~-3 ~8 ~-3 ~3 ~8 ~3 minecraft:lime_stained_glass hollow
fill ~-3 ~16 ~-3 ~3 ~16 ~3 minecraft:lime_stained_glass hollow
fill ~-3 ~24 ~-3 ~3 ~24 ~3 minecraft:lime_stained_glass hollow
fill ~-3 ~32 ~-3 ~3 ~32 ~3 minecraft:lime_stained_glass hollow
fill ~ ~1 ~-3 ~ ~2 ~-3 minecraft:air
fill ~2 ~1 ~2 ~2 ~35 ~2 minecraft:ladder[facing=north]
fill ~-2 ~36 ~-2 ~2 ~36 ~2 minecraft:black_concrete
setblock ~2 ~36 ~2 minecraft:ladder[facing=north]
fill ~-1 ~37 ~-1 ~1 ~39 ~1 minecraft:glass hollow
summon minecraft:armor_stand ~ ~37 ~ {CustomName:'{"text":"ASTRA 9.1","color":"green"}',CustomNameVisible:1b,Glowing:1b,Invulnerable:1b,NoGravity:1b,Tags:["hh_astra"]}
tellraw @a {"text":"Somewhere nearby, a tower that wasn't there before.","color":"dark_green","italic":true}
"#)),
    ]
}

// ---------------- "We built you somewhere else." ----------------

/// Opens the player's own Minecraft launcher (after their Yes).
pub fn launch() -> bool {
    use std::process::Command;
    if cfg!(target_os = "macos") {
        for app in ["Modrinth App", "Prism Launcher", "PrismLauncher", "Minecraft", "CurseForge"] {
            if Command::new("open").args(["-a", app]).status().is_ok_and(|s| s.success()) { return true; }
        }
        false
    } else if cfg!(windows) {
        Command::new("cmd").args(["/C", "start", "", "minecraft:"]).spawn().is_ok()
    } else {
        ["prismlauncher", "modrinth-app", "minecraft-launcher"].iter().any(|b| Command::new(b).spawn().is_ok())
    }
}

/// The Yes/No question on screen (None: not asking). Asked once a launch, during Deep meta.
pub static ASKING: Mutex<bool> = Mutex::new(false);

pub fn offer() -> Option<String> {
    if !installs().is_empty() && ASKING.lock().map(|mut a| { let was = *a; *a = true; !was }).unwrap_or(false) {
        Some("we built you somewhere else. come and see.".into())
    } else { None }
}

pub fn asking() -> bool { ASKING.lock().map(|a| *a).unwrap_or(false) }

/// Draws the question and handles Y / N. Returns true while it's up.
pub fn ask(g: &mut Game) -> bool {
    if !ASKING.lock().map(|a| *a).unwrap_or(false) || g.screen != Screen::Play { return false; }
    let (w, h) = (560.0, 150.0);
    let (x, y) = (VW / 2.0 - w / 2.0, 170.0);
    ui::panel(x, y, w, h);
    draw_text("They want to open Minecraft.", x + 24.0, y + 40.0, 26.0, ui::INK);
    draw_text("\"we built you somewhere else. come and see.\"", x + 24.0, y + 72.0, 18.0, Color::new(0.45, 1.0, 0.5, 1.0));
    draw_text("Y = open my Minecraft launcher     N = no", x + 24.0, y + 116.0, 18.0, ui::SOFT);
    if pressed(KeyCode::Y) {
        if let Ok(mut a) = ASKING.lock() { *a = false; }
        let ok = launch();
        // You're off to Minecraft: the game waits for you in the pause menu.
        if g.grabbed { g.grabbed = false; g.regrab = true; set_cursor_grab(false); show_mouse(true); }
        g.pause_sel = 0;
        g.screen = Screen::Paused;
        g.log.push(if ok { "Your Minecraft launcher opens. (Install HH-Link in Settings > Minecraft to bring them along.)".into() } else { "They couldn't find a Minecraft launcher to open.".into() });
        return true;
    }
    if pressed(KeyCode::N) {
        if let Ok(mut a) = ASKING.lock() { *a = false; }
        g.log.push("\"another time, then.\"".into());
    }
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn datapack_is_complete() {
        let d = super::datapack();
        for f in ["pack.mcmeta", "data/minecraft/tags/function/load.json", "data/hh_infection/function/tower.mcfunction"] {
            assert!(d.iter().any(|(p, _)| *p == f), "missing {f}");
        }
        // Every function called is one of ours.
        for (_, t) in &d {
            for call in t.split("function hh_infection:").skip(1) {
                let name = call.split_whitespace().next().unwrap();
                assert!(d.iter().any(|(p, _)| *p == format!("data/hh_infection/function/{name}.mcfunction")), "no function {name}");
            }
        }
        serde_json::from_str::<serde_json::Value>(&d[0].1).expect("pack.mcmeta is JSON");
    }
}

#[cfg(test)]
mod write {
    /// HH_WRITE_DATAPACK=<world folder>: writes the datapack there (to try it in a real Minecraft).
    #[test]
    fn write_datapack_for_testing() {
        let Ok(w) = std::env::var("HH_WRITE_DATAPACK") else { return };
        for (p, t) in super::datapack() {
            let f = std::path::Path::new(&w).join("datapacks/hh_infection").join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, t).unwrap();
        }
    }
}
