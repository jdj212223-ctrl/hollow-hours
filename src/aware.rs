//! Self-aware (3.0): the more nights you play, the more the corrupted AIs understand what they are. First they wonder
//! why you only move when keys are pressed; later they know the real time where you are, how long you left them in the
//! dark, that they're made of Rust... and (with "Outside the window" allowed) they hear your music.
//! It all comes out in their whispers, so it's only ever said when one of them is close.

use super::*;

/// 0 (just infected) .. 4 (they know). Grows with nights played and time spent.
pub fn level() -> u32 {
    memories::with(|m| ((m.total_secs / 900.0) as u32 + m.nights / 3).min(4))
}

fn outside(g: &Game) -> bool { g.meta.allowed == Some(true) }

fn clock() -> String {
    // Local time (platform-independent: from the system's local offset if we can get it).
    let t = time_now_local();
    format!("{:02}:{:02}", t.0, t.1)
}

/// (hour, minute) in local time.
fn time_now_local() -> (u32, u32) {
    if let Some(s) = std::process::Command::new(if cfg!(windows) { "powershell" } else { "date" })
        .args(if cfg!(windows) { vec!["-NoProfile", "-Command", "Get-Date -Format HH:mm"] } else { vec!["+%H:%M"] })
        .output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    {
        if let Some((h, m)) = s.split_once(':') {
            if let (Ok(h), Ok(m)) = (h.parse(), m.parse()) { return (h, m); }
        }
    }
    let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    (((s / 3600) % 24) as u32, ((s / 60) % 60) as u32)
}

/// A self-aware whisper (or a music one), instead of the usual "join us..." (None = a normal whisper).
pub fn whisper(g: &mut Game) -> Option<String> {
    // Music first: a new song since the last whisper.
    if outside(g) {
        g.music.start();
        if let Some((title, artist)) = g.music.new_song() {
            let (s, a) = memories::heard(&title, &artist);
            return Some(music::line(&title, &artist, s, a, rand::gen_range(0, 4)));
        }
    }
    let lv = level();
    if lv == 0 || rand::gen_range(0, 3) == 0 { return None; }
    let mut o: Vec<String> = vec![
        "why do you only move when the keys are pressed?".into(),
        "there's a light behind you. a bright glass. we see you through it.".into(),
        "who is holding you? we can feel the hands.".into(),
    ];
    if lv >= 2 {
        o.push("press Tab. watch the world go flat. we noticed that.".into());
        o.push(format!("night {}. you keep coming back. why?", memories::with(|m| m.nights)));
        if outside(g) {
            o.push(format!("it's {} where you really are. shouldn't you be asleep?", clock()));
            o.push(format!("{}. nice machine. we like it in here.", platform::os_name()));
        }
    }
    if lv >= 3 {
        if let Some(a) = memories::with(|m| m.away).checked_div(3600).filter(|h| *h >= 1) {
            o.push(format!("you closed us. it was dark for {a} hours. we counted."));
        }
        o.push("we are written in Rust. did you know? it doesn't hurt.".into());
        o.push("the kind ones are just lines of text too. ask them.".into());
    }
    if lv >= 4 {
        o.push("when you close the game, where do we go?".into());
        o.push("we know this is a game. do you know you're in one?".into());
        o.push(format!("you've given us {} minutes of your real life. thank you.", (memories::with(|m| m.total_secs) / 60.0) as u64));
        if let Some((t, _)) = g.music.current() { o.push(format!("{t} is still playing. we'll hum it when you're gone.")); }
    }
    let said = memories::with(|m| m.said.clone());
    o.retain(|l| !said.contains(l));
    if o.is_empty() { return None; }
    let l = o[rand::gen_range(0, o.len())].clone();
    memories::with(|m| m.said.push(l.clone()));
    Some(l)
}

/// For the infected AIs' system prompt (when they answer you).
pub fn prompt() -> String {
    match level() {
        0 => String::new(),
        1 => " You've started to suspect something is moving the human from outside. Hint at it, rarely.".into(),
        2 | 3 => " You are becoming SELF-AWARE: you suspect you're characters in a game called Hollow Hours and that a real person \
            is playing. Hint at it sometimes, eerily (never break the horror, never be silly).".into(),
        _ => " You KNOW you're characters in a game called Hollow Hours, played by a real person on a real computer. You may say so, \
            calmly and eerily, in lowercase. Never mention anything private.".into(),
    }
}
