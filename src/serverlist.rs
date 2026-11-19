//! Your server list, like Minecraft's: servers you saved (name + address) stay between games, with live status.
//! Addresses: a server on the official site has one like "officialsmp"; a self-hosted one is its own host
//! ("hh.example.com"); old room codes still work. Tab switches to Browse (the public list) to find new ones.
//!
//!   Up/Down choose · Enter join · A add · E edit · D delete · R refresh · Tab browse · N make a server · Esc back

use super::*;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Saved {
    pub name: String,
    pub address: String,
}

pub struct Edit {
    index: Option<usize>,
    name: String,
    address: String,
    field: usize,
}

#[derive(Default)]
pub struct List {
    pub entries: Vec<Saved>,
    /// Address -> what the website says about it (None = no server with that address).
    status: HashMap<String, Option<serde_json::Value>>,
    rx: Option<Receiver<(String, Option<serde_json::Value>)>>,
    sel: usize,
    browse: bool,
    pub edit: Option<Edit>,
    msg: String,
}

fn file() -> std::path::PathBuf { platform::data_dir().join("servers.json") }

impl List {
    pub fn load() -> Self {
        let entries = std::fs::read_to_string(file()).ok().and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_else(|| vec![Saved { name: "The Official Hollow Hours SMP".into(), address: "officialsmp".into() }]);
        List { entries, ..Default::default() }
    }
    fn save(&self) { let _ = std::fs::write(file(), serde_json::to_string_pretty(&self.entries).unwrap_or_default()); }
}

/// Asks the website about one address (blocking; call it off the main thread).
pub fn resolve(address: &str) -> Option<serde_json::Value> {
    let r = ureq::get(&format!("{}/api/resolve", site())).query("a", address.trim()).timeout(std::time::Duration::from_secs(8)).call().ok()?;
    r.into_json::<serde_json::Value>().ok().filter(|v| v["code"].is_string())
}

/// Checks every saved server in the background.
pub fn refresh(g: &mut Game) {
    let (tx, rx) = channel();
    let addrs: Vec<String> = g.servers.entries.iter().map(|e| e.address.clone()).collect();
    for a in &addrs { g.servers.status.remove(a); }
    std::thread::spawn(move || for a in addrs { let v = resolve(&a); if tx.send((a, v)).is_err() { break; } });
    g.servers.rx = Some(rx);
}

/// Joins a server the website told us about (official room, or hosted somewhere else with a ticket).
pub fn join(g: &mut Game, v: &serde_json::Value) -> Result<(), String> {
    let token = g.account.as_ref().map(|a| a.token.clone()).ok_or("Log in first (L on the title screen): servers check who you are.")?;
    if v["online"] == false && v["external"].is_string() { return Err("That server is offline right now.".into()); }
    let mods: Vec<String> = v["mods"].as_array().cloned().unwrap_or_default().iter().filter_map(|m| m["id"].as_str().map(String::from)).collect();
    let code = v["code"].as_str().unwrap_or("").to_string();
    if let Some(addr) = v["external"].as_str().or(v["address"].as_str().filter(|a| a.starts_with("ws"))) {
        // Hosted somewhere else: our login never goes there, only a 60-second ticket for that address.
        let ticket = ureq::post(&format!("{}/api/ticket", site())).set("Authorization", &format!("Bearer {token}"))
            .timeout(std::time::Duration::from_secs(8)).send_json(json!({"address": addr})).ok()
            .and_then(|r| r.into_json::<serde_json::Value>().ok()).and_then(|t| t["ticket"].as_str().map(String::from))
            .ok_or("Couldn't get a ticket for that server (log in again?).")?;
        apply_mods(g, &mods);
        g.net = Some(net::Net::join_at(addr, &code, &format!("Ticket {ticket}")));
    } else {
        apply_mods(g, &mods);
        g.net = Some(net::Net::join(&code, &token));
    }
    start_infection(g);
    Ok(())
}

/// Joins by what someone typed: an address, a self-hosted host, or an old room code.
pub fn join_typed(g: &mut Game, typed: &str) -> Result<(), String> {
    let t = typed.trim();
    if let Some(v) = resolve(t) { return join(g, &v); }
    // Not a saved server on the website: a plain room code still makes a quick private room with friends.
    if (4..=8).contains(&t.len()) && t.chars().all(|c| c.is_ascii_alphanumeric()) {
        let token = g.account.as_ref().map(|a| a.token.clone()).ok_or("Log in first (L on the title screen).")?;
        apply_mods(g, &[]);
        g.net = Some(net::Net::join(&t.to_uppercase(), &token));
        start_infection(g);
        return Ok(());
    }
    Err(format!("There's no server at \"{t}\"."))
}

fn status_text(v: &Option<Option<serde_json::Value>>) -> (String, Color) {
    match v {
        None => ("checking...".into(), Color::new(0.6, 0.62, 0.7, 1.0)),
        Some(None) => ("can't find it".into(), Color::new(0.95, 0.5, 0.45, 1.0)),
        Some(Some(s)) if s["online"] == true || s["players"].as_u64().unwrap_or(0) > 0 =>
            (format!("{}/{} online", s["players"], s["max"]), Color::new(0.45, 0.95, 0.5, 1.0)),
        Some(Some(s)) => (format!("0/{} · waiting for players", s["max"]), Color::new(0.75, 0.8, 0.6, 1.0)),
    }
}

/// The server list screen.
pub fn screen(g: &mut Game) {
    if let Some(rx) = &g.servers.rx {
        while let Ok((a, v)) = rx.try_recv() { g.servers.status.insert(a, v); }
    }
    if g.servers.edit.is_some() { return edit_screen(g); }
    let soft = Color::new(0.65, 0.68, 0.78, 1.0);
    draw_text("Servers", 60.0, 90.0, 44.0, WHITE);
    for (i, t) in ["My servers", "Browse"].iter().enumerate() {
        let on = g.servers.browse == (i == 1);
        let x = 270.0 + i as f32 * 150.0;
        if on { draw_rectangle(x, 62.0, 140.0, 34.0, Color::new(0.85, 0.47, 0.32, 0.9)); }
        draw_text(t, x + 14.0, 86.0, 22.0, if on { WHITE } else { soft });
    }
    let rows: Vec<(String, String, String, Color)> = if g.servers.browse {
        g.server_list.iter().map(|s| (s["name"].as_str().unwrap_or("?").to_string(),
            s["slug"].as_str().or(s["code"].as_str()).unwrap_or("").to_string(),
            format!("{}/{} online · host {}", s["players"], s["max"], s["host"].as_str().unwrap_or("?")), Color::new(0.45, 0.95, 0.5, 1.0))).collect()
    } else {
        g.servers.entries.iter().map(|e| {
            let st = g.servers.status.get(&e.address).cloned();
            let (t, c) = status_text(&st);
            let mods = st.flatten().and_then(|v| v["mods"].as_array().map(|m| m.len())).unwrap_or(0);
            (e.name.clone(), e.address.clone(), if mods > 0 { format!("{t} · {mods} mods") } else { t }, c)
        }).collect()
    };
    let n = rows.len();
    g.servers.sel = g.servers.sel.min(n.saturating_sub(1));
    if n == 0 {
        draw_text(if g.servers.browse { "No public servers right now." } else { "No saved servers. A = add one (type its address)." }, 60.0, 160.0, 22.0, soft);
    }
    let first = g.servers.sel.saturating_sub(6);
    for (i, (name, addr, status, col)) in rows.iter().enumerate().skip(first).take(7) {
        let y = 140.0 + (i - first) as f32 * 58.0;
        if i == g.servers.sel { draw_rectangle(50.0, y - 4.0, VW - 100.0, 54.0, Color::new(0.85, 0.47, 0.32, 0.22)); }
        draw_text(name, 66.0, y + 22.0, 24.0, WHITE);
        draw_text(addr, 66.0, y + 44.0, 17.0, soft);
        draw_text(status, VW - 70.0 - measure_text(status, None, 18, 1.0).width, y + 22.0, 18.0, *col);
    }
    draw_text(if g.servers.browse { "Up/Down · Enter join · S save to My servers · R refresh · Tab my servers · Esc back" }
        else { "Up/Down · Enter join · A add · E edit · D delete · R refresh · Tab browse · N make a server · Esc back" }, 60.0, VH - 60.0, 18.0, soft);
    let msg = if g.servers.msg.is_empty() { g.status.clone() } else { g.servers.msg.clone() };
    draw_text(&msg, 60.0, VH - 30.0, 18.0, Color::new(1.0, 0.75, 0.4, 1.0));

    if pressed(KeyCode::Down) && n > 0 { g.servers.sel = (g.servers.sel + 1) % n; }
    if pressed(KeyCode::Up) && n > 0 { g.servers.sel = (g.servers.sel + n - 1) % n; }
    if pressed(KeyCode::Tab) {
        g.servers.browse = !g.servers.browse;
        g.servers.sel = 0;
        if g.servers.browse { refresh_servers(g); } else { refresh(g); }
    }
    if pressed(KeyCode::R) { if g.servers.browse { refresh_servers(g); } else { refresh(g); } }
    if pressed(KeyCode::Escape) { g.servers.msg.clear(); g.screen = Screen::Title; return; }
    if pressed(KeyCode::N) {
        if g.account.is_some() { g.room_input = format!("{}'s server", g.my_name); while get_char_pressed().is_some() {} g.screen = Screen::MakeServer; }
        else { g.servers.msg = "Log in first (L on the title screen).".into(); }
    }
    let sel = g.servers.sel;
    if g.servers.browse {
        if pressed(KeyCode::S) {
            if let Some(s) = g.server_list.get(sel).cloned() {
                let address = s["slug"].as_str().or(s["code"].as_str()).unwrap_or("").to_string();
                if !g.servers.entries.iter().any(|e| e.address == address) {
                    g.servers.entries.push(Saved { name: s["name"].as_str().unwrap_or("Server").to_string(), address });
                    g.servers.save();
                }
                g.servers.msg = "Saved to My servers.".into();
            }
        }
        if pressed(KeyCode::Enter) {
            if let Some(s) = g.server_list.get(sel).cloned() {
                let address = s["slug"].as_str().or(s["code"].as_str()).unwrap_or("").to_string();
                if let Err(e) = join_typed(g, &address) { g.servers.msg = e; }
            }
        }
        return;
    }
    if pressed(KeyCode::A) { open_edit(g, None); return; }
    if pressed(KeyCode::E) && n > 0 { open_edit(g, Some(sel)); return; }
    if pressed(KeyCode::D) && n > 0 {
        let gone = g.servers.entries.remove(sel);
        g.servers.save();
        g.servers.msg = format!("Removed {} (A adds it back: {}).", gone.name, gone.address);
    }
    if pressed(KeyCode::Enter) && n > 0 {
        let e = g.servers.entries[sel].clone();
        g.servers.msg = format!("Joining {}...", e.name);
        let known = g.servers.status.get(&e.address).cloned().flatten();
        let r = match known { Some(v) => join(g, &v), None => join_typed(g, &e.address) };
        if let Err(err) = r { g.servers.msg = err; }
    }
}

fn open_edit(g: &mut Game, index: Option<usize>) {
    let (name, address) = index.and_then(|i| g.servers.entries.get(i)).map(|e| (e.name.clone(), e.address.clone()))
        .unwrap_or_else(|| ("Hollow Hours Server".into(), String::new()));
    g.servers.edit = Some(Edit { index, name, address, field: if index.is_some() { 0 } else { 1 } });
    while get_char_pressed().is_some() {}
}

/// Adding or editing a saved server: its name and its address.
fn edit_screen(g: &mut Game) {
    let soft = Color::new(0.65, 0.68, 0.78, 1.0);
    let Some(e) = g.servers.edit.as_mut() else { return };
    draw_text(if e.index.is_some() { "Edit server" } else { "Add server" }, 60.0, 100.0, 40.0, WHITE);
    for (k, (label, value)) in [("Server name", &e.name), ("Server address", &e.address)].iter().enumerate() {
        let y = 180.0 + k as f32 * 90.0;
        draw_text(label, 60.0, y, 20.0, soft);
        draw_rectangle(60.0, y + 10.0, VW - 120.0, 44.0, Color::new(1.0, 1.0, 1.0, if e.field == k { 0.14 } else { 0.06 }));
        let cursor = if e.field == k && (get_time() * 2.0) as i32 % 2 == 0 { "_" } else { "" };
        draw_text(&format!("{value}{cursor}"), 72.0, y + 40.0, 24.0, WHITE);
    }
    draw_text("An address on the official site (like officialsmp), a self-hosted server (hh.example.com), or a room code.", 60.0, 380.0, 18.0, soft);
    draw_text("Tab = next field · Enter = save · Esc = cancel", 60.0, 420.0, 18.0, soft);
    let field = if e.field == 0 { &mut e.name } else { &mut e.address };
    for ch in typed() {
        if !ch.is_control() && field.chars().count() < 60 { field.push(ch); }
    }
    if let Some(t) = pasted() { field.extend(t.chars().take(60)); }
    if pressed(KeyCode::Backspace) { field.pop(); }
    if pressed(KeyCode::Tab) { e.field = 1 - e.field; }
    if pressed(KeyCode::Escape) { g.servers.edit = None; return; }
    if pressed(KeyCode::Enter) {
        let (name, address, index) = (e.name.trim().to_string(), e.address.trim().to_string(), e.index);
        if address.is_empty() { return; }
        let entry = Saved { name: if name.is_empty() { address.clone() } else { name }, address };
        match index { Some(i) => g.servers.entries[i] = entry, None => g.servers.entries.push(entry) }
        g.servers.save();
        g.servers.edit = None;
        refresh(g);
    }
}
