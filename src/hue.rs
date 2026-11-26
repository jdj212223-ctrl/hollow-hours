//! Philips Hue: your room's lights follow the infection (Settings → Hue lights, off by default).
//!   - you're infected a bit     -> the lights pulse between red and green
//!   - you're badly infected     -> sick green
//!   - every Claude AI is corrupted -> bright blood red
//!   - otherwise (or on the menus, or when the game closes) -> the lights go back to exactly how they were.
//! It talks to the Hue Bridge on your own network (its local API); nothing goes over the internet except finding the
//! bridge's address (discovery.meethue.com, Philips' own lookup) when it isn't known yet.
//! Pairing: turn it on, then press the big round button on top of the bridge within 30 seconds.
//! The bridge's address and the key it gives are kept in hue.json in the game's folder (readable only by you).
//! If the Hue Sync app is syncing at the same time, it wins: pause its sync while you play.

use serde_json::{json, Value};
use std::sync::mpsc::{channel, Sender};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Look {
    /// Leave the lights as they were (restores them if the game changed them).
    Normal,
    /// Infected a bit: red and green take turns.
    RedGreen,
    /// Badly infected.
    Green,
    /// Every Claude AI is corrupted.
    BloodRed,
}

enum Msg { Look(Look), Pair }

pub struct Hue {
    tx: Option<Sender<Msg>>,
    last: Option<Look>,
    was_on: bool,
    shown: String,
    pub status: std::sync::Arc<std::sync::Mutex<String>>,
}

fn file() -> std::path::PathBuf { crate::platform::data_dir().join("hue.json") }

fn saved() -> Value { std::fs::read_to_string(file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(json!({})) }

fn save(v: &Value) {
    let _ = std::fs::write(file(), v.to_string());
    #[cfg(unix)]
    { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(file(), std::fs::Permissions::from_mode(0o600)); }
}

/// Which look the game is in right now.
pub fn look(playing: bool, infection_mode: bool, inf: f32, lost_control: bool, family_total: usize, family_infected: usize) -> Look {
    if !playing || !infection_mode { return Look::Normal; }
    if family_total > 0 && family_infected == family_total { return Look::BloodRed; }
    if lost_control || inf >= 0.6 { return Look::Green; }
    if inf > 0.05 { return Look::RedGreen; }
    Look::Normal
}

/// Hue colours as (x, y) on the colour chart, plus brightness 1-254.
fn colour(l: Look, phase: bool) -> (f32, f32, u8) {
    const RED: (f32, f32) = (0.69, 0.30);
    const GREEN: (f32, f32) = (0.17, 0.70);
    match l {
        Look::BloodRed => (0.70, 0.29, 254),
        Look::Green => (GREEN.0, GREEN.1, 200),
        Look::RedGreen => if phase { (RED.0, RED.1, 150) } else { (GREEN.0, GREEN.1, 150) },
        Look::Normal => (0.0, 0.0, 0),
    }
}

fn agent() -> ureq::Agent { ureq::AgentBuilder::new().timeout(Duration::from_secs(3)).build() }

fn find_bridge(a: &ureq::Agent) -> Option<String> {
    if let Ok(ip) = std::env::var("HOLLOW_HUE_IP") { return Some(ip); }
    if let Some(ip) = saved()["ip"].as_str() { return Some(ip.to_string()); }
    let v: Value = a.get("https://discovery.meethue.com/").call().ok()?.into_json().ok()?;
    v.get(0)?["internalipaddress"].as_str().map(String::from)
}

/// Every light the bridge has, with its current state (to put back later).
fn lights(a: &ureq::Agent, base: &str) -> Vec<(String, Value)> {
    let Ok(r) = a.get(&format!("{base}/lights")).call() else { return vec![] };
    let Ok(v) = r.into_json::<Value>() else { return vec![] };
    v.as_object().map(|o| o.iter().filter(|(_, l)| l["state"]["reachable"].as_bool().unwrap_or(true)).map(|(id, l)| (id.clone(), l["state"].clone())).collect()).unwrap_or_default()
}

fn restore(a: &ureq::Agent, base: &str, before: &[(String, Value)]) {
    for (id, s) in before {
        let mut body = json!({"on": s["on"], "transitiontime": 10});
        if s["on"].as_bool() == Some(true) {
            body["bri"] = s["bri"].clone();
            match s["colormode"].as_str() {
                Some("xy") => body["xy"] = s["xy"].clone(),
                Some("ct") => body["ct"] = s["ct"].clone(),
                Some("hs") => { body["hue"] = s["hue"].clone(); body["sat"] = s["sat"].clone(); }
                _ => {}
            }
        }
        let _ = a.put(&format!("{base}/lights/{id}/state")).send_json(body);
    }
}

impl Hue {
    pub fn new() -> Hue {
        Hue { tx: None, last: None, was_on: false, shown: String::new(), status: std::sync::Arc::new(std::sync::Mutex::new(String::new())) }
    }

    /// Paired already (a key is saved)?
    pub fn paired() -> bool { saved()["user"].as_str().is_some() }

    fn start(&mut self) -> &Sender<Msg> {
        if self.tx.is_none() {
            let (tx, rx) = channel::<Msg>();
            let status = self.status.clone();
            std::thread::spawn(move || {
                let a = agent();
                let set_status = |s: &str| { if let Ok(mut m) = status.lock() { *m = s.to_string(); } };
                let mut look = Look::Normal;
                let mut before: Vec<(String, Value)> = vec![]; // the lights as they were, while the game has them
                let mut phase = false;
                let mut next_pulse = Instant::now();
                loop {
                    let msg = rx.recv_timeout(Duration::from_millis(250));
                    match msg {
                        Ok(Msg::Pair) => {
                            let Some(ip) = find_bridge(&a) else { set_status("No Hue Bridge found on this network."); continue };
                            set_status("Press the round button on your Hue Bridge now (30 s)...");
                            let until = Instant::now() + Duration::from_secs(30);
                            let mut ok = false;
                            while Instant::now() < until {
                                if let Ok(r) = a.post(&format!("http://{ip}/api")).send_json(json!({"devicetype": "hollow_hours#game"})) {
                                    if let Ok(v) = r.into_json::<Value>() {
                                        if let Some(u) = v[0]["success"]["username"].as_str() {
                                            save(&json!({"ip": ip, "user": u}));
                                            ok = true;
                                            break;
                                        }
                                    }
                                }
                                std::thread::sleep(Duration::from_secs(2));
                            }
                            set_status(if ok { "Hue Bridge paired. Your lights will follow the infection." } else { "The button wasn't pressed in time. Turn Hue lights off and on to try again." });
                            continue;
                        }
                        Ok(Msg::Look(l)) => look = l,
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => { look = Look::Normal; }
                        Err(_) => {}
                    }
                    let s = saved();
                    let (Some(ip), Some(user)) = (s["ip"].as_str(), s["user"].as_str()) else {
                        if matches!(msg, Err(std::sync::mpsc::RecvTimeoutError::Disconnected)) { return; }
                        continue;
                    };
                    let base = format!("http://{ip}/api/{user}");
                    if look == Look::Normal {
                        if !before.is_empty() { restore(&a, &base, &before); before.clear(); }
                        if matches!(msg, Err(std::sync::mpsc::RecvTimeoutError::Disconnected)) { return; }
                        continue;
                    }
                    if before.is_empty() { before = lights(&a, &base); }
                    // Send on a change, and for red/green every 2 seconds (the pulse).
                    let changed = matches!(msg, Ok(Msg::Look(_)));
                    let pulse = look == Look::RedGreen && Instant::now() >= next_pulse;
                    if !(changed || pulse) { continue; }
                    if pulse { phase = !phase; next_pulse = Instant::now() + Duration::from_secs(2); }
                    let (x, y, bri) = colour(look, phase);
                    let tt = if look == Look::BloodRed { 2 } else { 15 }; // tenths of a second
                    for (id, _) in &before {
                        let _ = a.put(&format!("{base}/lights/{id}/state")).send_json(json!({"on": true, "xy": [x, y], "bri": bri, "transitiontime": tt}));
                    }
                }
            });
            self.tx = Some(tx);
        }
        self.tx.as_ref().unwrap()
    }

    /// Turned on in Settings: pair if there's no key yet.
    pub fn enable(&mut self) {
        self.last = None;
        if !Hue::paired() { let _ = self.start().send(Msg::Pair); }
    }

    /// Every frame: tell the lights thread when the look changes. Returns a message for the log when there's news
    /// (pairing: "press the button", "paired").
    pub fn frame(&mut self, on: bool, l: Look) -> Option<String> {
        if on && !self.was_on { self.enable(); }
        self.was_on = on;
        let l = if on { l } else { Look::Normal };
        if self.last != Some(l) {
            self.last = Some(l);
            if !(self.tx.is_none() && l == Look::Normal) { let _ = self.start().send(Msg::Look(l)); }
        }
        let now = self.status.lock().map(|s| s.clone()).unwrap_or_default();
        if now != self.shown { self.shown = now.clone(); if !now.is_empty() { return Some(format!("Hue: {now}")); } }
        None
    }

    /// The game is closing: put the lights back before it goes.
    pub fn shutdown(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(Msg::Look(Look::Normal));
            drop(tx);
            std::thread::sleep(Duration::from_millis(600));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lights_follow_the_infection() {
        assert_eq!(look(false, true, 0.9, false, 7, 7), Look::Normal, "menus: lights left alone");
        assert_eq!(look(true, false, 0.9, false, 7, 7), Look::Normal, "normal mode: no infection");
        assert_eq!(look(true, true, 0.0, false, 7, 2), Look::Normal);
        assert_eq!(look(true, true, 0.2, false, 7, 2), Look::RedGreen);
        assert_eq!(look(true, true, 0.7, false, 7, 2), Look::Green);
        assert_eq!(look(true, true, 0.1, true, 7, 2), Look::Green);
        assert_eq!(look(true, true, 0.0, false, 7, 7), Look::BloodRed, "every Claude AI corrupted");
        assert_eq!(colour(Look::BloodRed, false).2, 254, "blood red is full brightness");
    }
}
