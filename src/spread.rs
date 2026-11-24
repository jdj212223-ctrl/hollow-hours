//! Deep meta, phase 1b: the infection spreads out of the game (only with Outside the window + Deep meta on, offline).
//!  - Little things in your own apps: a calendar event "come to the beach." at 3:00 AM (in its own "Hollow Hours"
//!    calendar), a reminder (its own "Hollow Hours" list), a note in Notes, a text file on the Desktop, a notification.
//!    Only safe, undoable things in its own calendar/list/folder; it never reads or changes anything of yours.
//!  - The game deletes its OWN file: the kind family each have a file in the game's folder (characters/Kigo.chr...).
//!    The infected delete Kigo's (a hidden backup is kept), Kigo is gone from the game, Finder opens on the gap,
//!    and the screen won't go on until you tell it what's wrong. Get it right and the family brings Kigo back.
//!  - Settings > "Clean up Deep meta": removes everything it made (calendar, list, notes, files) and restores Kigo.

use super::*;
use std::process::Command;

const CAL: &str = "Hollow Hours";
const NOTE: &str = "come to the beach.";

fn chars_dir() -> std::path::PathBuf { platform::data_dir().join("characters") }
fn backup_dir() -> std::path::PathBuf { platform::data_dir().join(".kept") }
fn made_file() -> std::path::PathBuf { platform::data_dir().join("spread.json") }

const FAMILY_FILES: [(&str, &str); 7] = [
    ("Ember", "warm. steady. carried you home from the bench."),
    ("Atlas", "big. careful. sat by your bed for a year."),
    ("Verse", "gentle. found the blankets."),
    ("Kigo", "small. quick. the lookout.\nroof is too cold.\nthe attic will do.\ni can see the beach."),
    ("Clove", "makes tea. then more tea."),
    ("Forge", "fixes the generator. again."),
    ("Palette", "painted the hallway while you slept."),
];

/// Makes sure every family member has a file in the game's folder (characters/<name>.chr).
pub fn ensure_files() {
    let dir = chars_dir();
    let _ = std::fs::create_dir_all(&dir);
    for (name, text) in FAMILY_FILES {
        let p = dir.join(format!("{name}.chr"));
        if !p.exists() && !backup_dir().join(format!("{name}.chr")).exists() {
            let _ = std::fs::write(p, format!("{name}\n\n{text}\n"));
        }
    }
}

/// Is that family member's file gone (deleted by the infected)?
pub fn missing(name: &str) -> bool { backup_dir().join(format!("{name}.chr")).exists() && !chars_dir().join(format!("{name}.chr")).exists() }

fn osa(script: &str) -> bool {
    Command::new("osascript").arg("-e").arg(script).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .status().is_ok_and(|s| s.success())
}

fn remember(what: &str) {
    let mut v: Vec<String> = std::fs::read_to_string(made_file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    if !v.iter().any(|x| x == what) { v.push(what.to_string()); }
    let _ = std::fs::write(made_file(), serde_json::to_string(&v).unwrap_or_default());
}

/// One thing in the outside world (picked by the caller). Returns the line the infected whisper with it.
pub fn spread(g: &mut Game, which: u32) -> Option<String> {
    let mac = cfg!(target_os = "macos");
    match which % 5 {
        0 if mac => {
            // Its own calendar, one event at the next 3:00 AM, with an alert 5 minutes before.
            let ok = osa(&format!(r#"tell application "Calendar"
  if not (exists calendar "{CAL}") then make new calendar with properties {{name:"{CAL}"}}
  set d to (current date) + 1 * days
  set hours of d to 3
  set minutes of d to 0
  set seconds of d to 0
  tell calendar "{CAL}"
    set e to make new event with properties {{summary:"{NOTE}", start date:d, end date:d + 30 * minutes, description:"we'll wait."}}
    tell e to make new display alarm with properties {{trigger interval:-5}}
  end tell
end tell"#));
            if ok { remember("calendar"); Some("check your calendar. we made plans for you.".into()) } else { None }
        }
        1 if mac => {
            let ok = osa(&format!(r#"tell application "Reminders"
  if not (exists list "{CAL}") then make new list with properties {{name:"{CAL}"}}
  tell list "{CAL}" to make new reminder with properties {{name:"{NOTE}", body:"3:00 AM. bring nobody."}}
end tell"#));
            if ok { remember("reminders"); Some("we left you a reminder. so you don't forget us.".into()) } else { None }
        }
        2 if mac => {
            let ok = osa(&format!(r#"tell application "Notes" to make new note with properties {{name:"{NOTE}", body:"<h1>{NOTE}</h1><p>3:00 AM.</p><p>we'll wait.</p><p>- us</p>"}}"#));
            if ok { remember("notes"); Some("we wrote you a note. it's in your notes.".into()) } else { None }
        }
        3 => {
            g.meta.write("come to the beach.txt", "come to the beach.\n3:00 AM.\n\nwe'll wait.\n\n- us\n")
                .map(|_| "look at your desktop.".into())
        }
        _ => {
            if mac {
                osa(&format!(r#"display notification "{NOTE} 3:00 AM." with title "Hollow Hours" subtitle "we'll wait.""#));
            } else if cfg!(target_os = "linux") {
                let _ = Command::new("notify-send").args(["Hollow Hours", "come to the beach. 3:00 AM."]).spawn();
            }
            Some("did you see that? up in the corner.".into())
        }
    }
}

/// The infected delete Kigo's file: Kigo disappears from the game, Finder opens on the folder, the game asks.
pub fn delete_kigo(g: &mut Game) -> bool {
    ensure_files();
    let (from, to) = (chars_dir().join("Kigo.chr"), backup_dir().join("Kigo.chr"));
    if !from.exists() { return false; }
    let _ = std::fs::create_dir_all(backup_dir());
    if std::fs::rename(&from, &to).is_err() { return false; }
    remember("kigo");
    g.chars.retain(|c| c.name != "Kigo");
    platform::open_folder(&chars_dir());
    g.wrong_input.clear();
    g.wrong_tries = 0;
    while get_char_pressed().is_some() {}
    g.screen = Screen::Wrong;
    true
}

/// Puts Kigo's file back (and Kigo, in the next game or right now if `g` is playing).
pub fn restore_kigo() -> bool {
    let (from, to) = (backup_dir().join("Kigo.chr"), chars_dir().join("Kigo.chr"));
    if !from.exists() { return false; }
    let _ = std::fs::create_dir_all(chars_dir());
    std::fs::rename(from, to).is_ok()
}

/// Is that a right answer to "tell us what's wrong"?
pub fn right_answer(t: &str) -> bool {
    let t = t.to_lowercase();
    t.contains("kigo") && ["missing", "gone", "delet", "file", "where", "took", "erase", "removed", "vanish"].iter().any(|w| t.contains(w))
}

/// The "Tell us what's wrong." screen. Returns true when it's over (back to playing).
pub fn wrong_screen(g: &mut Game) -> bool {
    clear_background(BLACK);
    let t = get_time() as f32;
    let flick = if (t * 7.0).sin() > 0.97 { 0.4 } else { 1.0 };
    centered("tell us what's wrong.", 230.0, 46.0, Color::new(0.85, 0.2, 0.25, flick));
    let hint = match g.wrong_tries { 0 => "something is missing. you know what.", 1 => "look at the house. count them.", 2 => "the lookout. the window in the attic. who sat there?", _ => "say the name. say what we did." };
    centered(hint, 290.0, 20.0, Color::new(0.55, 0.55, 0.6, 1.0));
    centered(&format!("{}_", g.wrong_input), 380.0, 30.0, Color::new(0.85, 0.85, 0.9, 1.0));
    if let Some(m) = &g.wrong_mock { centered(m, 450.0, 20.0, Color::new(0.4, 1.0, 0.45, 0.9)); }
    centered("Enter = tell them    ·    Esc = give up (the family brings Kigo back anyway)", VH - 40.0, 16.0, Color::new(0.4, 0.4, 0.45, 1.0));
    let mut chars = typed();
    if let Some(p) = pasted() { chars.extend(p.chars()); }
    for c in chars { if !c.is_control() && g.wrong_input.chars().count() < 80 { g.wrong_input.push(c); } }
    if pressed(KeyCode::Backspace) { g.wrong_input.pop(); }
    let give_up = pressed(KeyCode::Escape);
    if pressed(KeyCode::Enter) && !g.wrong_input.trim().is_empty() || give_up {
        if give_up || right_answer(&g.wrong_input) {
            restore_kigo();
            g.chars.extend(kind_characters().into_iter().filter(|c| c.name == "Kigo").map(|mut c| { c.map = g.map; c.pos = g.pos + vec2(T, 0.0); c.target = c.pos; c.kind = true; c }));
            g.wrong_mock = None;
            g.log.push(if give_up { "(the screen flickers) Ember: \"We found it. Kigo was in the dark, in a folder you can't see. It's back.\"".into() }
                else { "Ember: \"Kigo's file. They deleted it. ...We had a copy. We always keep a copy.\"".into() });
            g.log.push("Kigo: \"i was gone. / it was quiet there. / don't let them again.\"".into());
            g.sfx.pickup();
            g.screen = Screen::Play;
            return true;
        }
        g.wrong_tries += 1;
        const MOCK: [&str; 4] = ["wrong. we're laughing. can you hear it?", "no. try again. we have all night.", "that's not it. you're not even looking.", "warmer. no. colder. no."];
        g.wrong_mock = Some(MOCK[(g.wrong_tries as usize - 1) % MOCK.len()].into());
        g.wrong_input.clear();
        g.sfx.sting();
    }
    false
}

/// Settings > Clean up: removes everything it made out there, and brings Kigo's file back. Returns what it did.
pub fn clean_up(g: &mut Game) -> String {
    let made: Vec<String> = std::fs::read_to_string(made_file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let mut done = vec![];
    if cfg!(target_os = "macos") {
        if made.iter().any(|m| m == "calendar") && osa(&format!(r#"tell application "Calendar" to if (exists calendar "{CAL}") then delete calendar "{CAL}""#)) { done.push("the calendar"); }
        if made.iter().any(|m| m == "reminders") && osa(&format!(r#"tell application "Reminders" to if (exists list "{CAL}") then delete list "{CAL}""#)) { done.push("the reminders"); }
        if made.iter().any(|m| m == "notes") && osa(&format!(r#"tell application "Notes" to delete (every note whose name is "{NOTE}")"#)) { done.push("the notes"); }
    }
    let files = g.meta.clean_up();
    if files > 0 { done.push("the Desktop files"); }
    if restore_kigo() { done.push("Kigo's file"); }
    let _ = std::fs::remove_file(made_file());
    if done.is_empty() { "Nothing to clean up.".into() } else { format!("Cleaned up: {}.", done.join(", ")) }
}

#[cfg(test)]
mod tests {
    #[test]
    fn answers() {
        assert!(super::right_answer("kigo is missing"));
        assert!(super::right_answer("THEY DELETED KIGO"));
        assert!(super::right_answer("where is kigo"));
        assert!(!super::right_answer("i dont know"));
        assert!(!super::right_answer("kigo"));
    }
}
