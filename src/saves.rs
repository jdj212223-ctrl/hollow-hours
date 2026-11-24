//! Worlds (title: W): Infection games you can leave and come back to, like Minecraft worlds. Each world is a file in
//! the game's folder (worlds/<name>.json): where you are, your supplies, the kind ones (who's infected), the broken
//! walls, your kits, antidotes, intel, credits and pantry. It saves every 30 s while you play and when you leave.

use super::*;
use serde_json::{json, Value};

fn dir() -> std::path::PathBuf { platform::data_dir().join("worlds") }

fn slug(name: &str) -> String {
    let s: String = name.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "world".into() } else { s.chars().take(40).collect() }
}

fn now() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }

/// A world on disk: (file slug, name, minutes played, last played, insane, food%, infection%).
pub struct Entry { pub slug: String, pub name: String, pub minutes: u32, pub last: u64, pub insane: bool, pub alive: bool }

pub fn list() -> Vec<Entry> {
    let mut v: Vec<Entry> = std::fs::read_dir(dir()).into_iter().flatten().flatten().filter_map(|e| {
        let p = e.path();
        if p.extension()? != "json" { return None; }
        let w: Value = serde_json::from_str(&std::fs::read_to_string(&p).ok()?).ok()?;
        Some(Entry {
            slug: p.file_stem()?.to_string_lossy().to_string(),
            name: w["name"].as_str().unwrap_or("World").to_string(),
            minutes: (w["survived"].as_f64().unwrap_or(0.0) / 60.0) as u32,
            last: w["last"].as_u64().unwrap_or(0),
            insane: w["insane"].as_bool().unwrap_or(false),
            alive: !w["over"].as_bool().unwrap_or(false),
        })
    }).collect();
    v.sort_by(|a, b| b.last.cmp(&a.last));
    v
}

/// A free name: "World 1", "World 2"...
pub fn next_name() -> String {
    let taken: Vec<String> = list().into_iter().map(|e| e.slug).collect();
    (1..).map(|n| format!("World {n}")).find(|n| !taken.contains(&slug(n))).unwrap()
}

/// Saves the world being played (if any).
pub fn save(g: &Game) {
    let Some(name) = &g.world else { return };
    if !g.infection || g.net.is_some() { return; }
    let chars: Vec<Value> = g.chars.iter().map(|c| json!({
        "name": c.name, "map": c.map, "x": c.pos.x / T, "y": c.pos.y / T, "infected": c.infected, "kind": c.kind,
        "disguised": c.disguise_t > 0.0,
    })).collect();
    // Broken walls: every tile that differs from the untouched maps.
    let mut walls = vec![];
    PRISTINE.with(|p| WORLD.with(|w| {
        for (m, (a, b)) in p.borrow().iter().zip(w.borrow().iter()).enumerate() {
            for (y, (ra, rb)) in a.rows.iter().zip(b.rows.iter()).enumerate() {
                for (x, (ca, cb)) in ra.iter().zip(rb.iter()).enumerate() {
                    if ca != cb { walls.push(json!([m, x, y, *cb as char as u32])); }
                }
            }
        }
    }));
    let w = json!({
        "name": name, "last": now(), "insane": g.insane, "over": g.screen == Screen::Over,
        "map": g.map, "x": g.pos.x / T, "y": g.pos.y / T, "yaw": g.yaw,
        "food": g.food, "water": g.water, "inf": g.inf, "survived": g.survived, "clock": g.clock, "learn": g.learn_level,
        "credits": g.credits, "kits": g.kits, "antidotes": g.antidotes, "chips": g.chips, "intel": g.intel,
        "intel_from": g.intel_from, "disguised": g.disguise_t > 0.0, "pantry": g.pantry.to_vec(),
        "weapon": g.weapon.map(|(w, n)| json!([format!("{w:?}"), n])),
        "items": g.items.iter().map(|(m, p, wat)| json!([m, p.x / T, p.y / T, wat])).collect::<Vec<_>>(),
        "chars": chars, "walls": walls, "tower": story::tower_open(),
    });
    let _ = std::fs::create_dir_all(dir());
    let _ = std::fs::write(dir().join(format!("{}.json", slug(name))), serde_json::to_string(&w).unwrap_or_default());
}

/// Starts a brand-new world with this name.
pub fn create(g: &mut Game, name: &str) {
    let local = local_mods();
    apply_contents(g, &local);
    g.insane = false;
    start_infection(g);
    g.world = Some(name.to_string());
    g.log.push(format!("New world: {name}. It saves by itself (and when you leave)."));
    save(g);
}

/// Loads a world and drops you back where you were.
pub fn load(g: &mut Game, slug_: &str) -> bool {
    let Some(w) = std::fs::read_to_string(dir().join(format!("{slug_}.json"))).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()) else { return false };
    if w["over"].as_bool().unwrap_or(false) {
        // That world's last game ended: a fresh start, same name.
        let name = w["name"].as_str().unwrap_or("World").to_string();
        create(g, &name);
        g.log.push("Last time it ended badly here. A new night begins.".into());
        return true;
    }
    let local = local_mods();
    apply_contents(g, &local);
    g.insane = w["insane"].as_bool().unwrap_or(false);
    start_infection(g); // a fresh world first (characters, supplies), then everything saved on top
    story::set_tower(w["tower"].as_bool().unwrap_or(false));
    let f = |k: &str, d: f64| w[k].as_f64().unwrap_or(d);
    let u = |k: &str| w[k].as_u64().unwrap_or(0) as u32;
    g.world = w["name"].as_str().map(String::from);
    g.map = (f("map", 0.0) as usize).min(maps_len() - 1);
    g.pos = vec2(f("x", 14.0) as f32, f("y", 9.0) as f32) * T;
    g.yaw = f("yaw", 0.0) as f32;
    g.food = f("food", 1.0) as f32;
    g.water = f("water", 1.0) as f32;
    g.inf = f("inf", 0.0) as f32;
    g.survived = f("survived", 0.0) as f32;
    g.clock = f("clock", 0.0) as f32;
    g.learn_level = f("learn", 0.0) as u8;
    g.credits = u("credits");
    g.kits = u("kits");
    g.antidotes = u("antidotes");
    g.chips = u("chips");
    g.intel = u("intel");
    g.intel_from = w["intel_from"].as_array().map(|a| a.iter().filter_map(|n| n.as_str()).filter_map(|n| g.chars.iter().find(|c| c.name == n).map(|c| c.name)).collect()).unwrap_or_default();
    g.disguise_t = if w["disguised"].as_bool().unwrap_or(false) { disguise::YOU_SECS } else { 0.0 };
    if let Some(p) = w["pantry"].as_array() { for (i, n) in p.iter().enumerate().take(g.pantry.len()) { g.pantry[i] = n.as_u64().unwrap_or(0) as u32; } }
    g.weapon = w["weapon"].as_array().and_then(|a| {
        let kind = match a.first()?.as_str()? { "Bat" => combat::Weapon::Bat, "Knife" => combat::Weapon::Knife, _ => return None };
        Some((kind, a.get(1)?.as_u64()? as u32))
    });
    if let Some(items) = w["items"].as_array() {
        g.items = items.iter().filter_map(|i| Some((i[0].as_u64()? as usize, vec2(i[1].as_f64()? as f32, i[2].as_f64()? as f32) * T, i[3].as_bool()?))).collect();
    }
    if let Some(cs) = w["chars"].as_array() {
        for s in cs {
            let Some(c) = g.chars.iter_mut().find(|c| Some(c.name) == s["name"].as_str()) else { continue };
            c.map = (s["map"].as_u64().unwrap_or(0) as usize).min(maps_len() - 1);
            c.pos = vec2(s["x"].as_f64().unwrap_or(1.0) as f32, s["y"].as_f64().unwrap_or(1.0) as f32) * T;
            c.target = c.pos;
            c.infected = s["infected"].as_bool().unwrap_or(c.infected);
            c.kind = s["kind"].as_bool().unwrap_or(c.kind);
            c.disguise_t = if s["disguised"].as_bool().unwrap_or(false) { disguise::CLAUDE_SECS } else { 0.0 };
        }
    }
    if let Some(ws) = w["walls"].as_array() {
        for t in ws {
            if let (Some(m), Some(x), Some(y), Some(c)) = (t[0].as_u64(), t[1].as_i64(), t[2].as_i64(), t[3].as_u64()) {
                set_tile(m as usize, x as i32, y as i32, c as u8);
            }
        }
    }
    let mins = (g.survived / 60.0) as u32;
    g.log = vec![format!("Back in {}. ({mins} min survived so far.)", g.world.clone().unwrap_or_default()),
        "Ember (radio): You're back. We kept your place.".into()];
    g.screen = Screen::Play;
    true
}

pub fn delete(slug_: &str) { let _ = std::fs::remove_file(dir().join(format!("{slug_}.json"))); }

/// The Worlds screen: ↑↓ choose, Enter play, N new, D (twice) delete, Esc back.
pub fn screen(g: &mut Game) {
    let worlds = list();
    let n = worlds.len() + 1; // + "New world"
    if g.worlds_sel >= n { g.worlds_sel = n - 1; }
    if pressed(KeyCode::Down) { g.worlds_sel = (g.worlds_sel + 1) % n; g.worlds_del = false; }
    if pressed(KeyCode::Up) { g.worlds_sel = (g.worlds_sel + n - 1) % n; g.worlds_del = false; }
    let new = pressed(KeyCode::N) || (pressed(KeyCode::Enter) && g.worlds_sel == 0);
    if new {
        let name = next_name();
        create(g, &name);
        return;
    }
    if pressed(KeyCode::Enter) && g.worlds_sel > 0 {
        let s = worlds[g.worlds_sel - 1].slug.clone();
        if !load(g, &s) { g.status = "That world couldn't be read.".into(); }
        return;
    }
    if pressed(KeyCode::D) && g.worlds_sel > 0 {
        if g.worlds_del { delete(&worlds[g.worlds_sel - 1].slug); g.worlds_del = false; } else { g.worlds_del = true; }
    }
    if pressed(KeyCode::Escape) { g.screen = Screen::Title; g.worlds_del = false; return; }

    // Drawing.
    ui::backdrop();
    let (ink, soft) = (ui::INK, ui::SOFT);
    ui::header("Worlds", "Infection games you can leave and come back to. They save by themselves.");
    let rows: Vec<(String, String)> = std::iter::once(("+  New world".to_string(), format!("starts \"{}\" (Infection)", next_name())))
        .chain(worlds.iter().map(|w| {
            let ago = now().saturating_sub(w.last);
            let when = if ago < 3600 { format!("{} min ago", ago / 60) } else if ago < 86400 { format!("{} h ago", ago / 3600) } else { format!("{} days ago", ago / 86400) };
            (w.name.clone(), format!("{} min survived · {}{}{}", w.minutes, when, if w.insane { " · INSANE" } else { "" }, if w.alive { "" } else { " · game over" }))
        })).collect();
    let first = g.worlds_sel.saturating_sub(9);
    for (i, (name, what)) in rows.iter().enumerate().skip(first).take(10) {
        let y = 190.0 + (i - first) as f32 * 48.0;
        ui::row(ui::LEFT, y - 30.0, VW - 2.0 * ui::LEFT, 42.0, i == g.worlds_sel);
        if ui::hit(ui::LEFT, y - 30.0, VW - 2.0 * ui::LEFT, 42.0).1 { if g.worlds_sel == i { inject_key(KeyCode::Enter); } g.worlds_sel = i; }
        draw_text(name, ui::LEFT + 22.0, y, 24.0, if i == 0 { ui::GOOD } else { ink });
        draw_text(what, 430.0, y, 18.0, soft);
    }
    if g.worlds_del {
        ui::status("Press D again to DELETE this world for good (any other key cancels).");
        ui::hints(&[("D", "delete for good"), ("Esc", "cancel")]);
    } else {
        ui::hints(&[("Enter", "play"), ("↑↓", "choose"), ("N", "new world"), ("D", "delete"), ("Esc", "back")]);
    }
}
