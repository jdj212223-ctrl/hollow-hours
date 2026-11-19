//! Meta-horror: the game reaches outside its window. Only after the player says yes once (asked on first launch).
//!   - Files: written into a "Hollow Hours" folder on the Desktop, and they STAY after you quit (that's the point).
//!     The game only ever CREATES its own new files (never overwrites, moves or deletes anything else), keeps a list of
//!     them, and C on the title screen removes exactly those.
//!   - Terminal: opens a Terminal window that only PRINTS a message (printf), nothing else.
//!   - Window switching: opens one of its own files in TextEdit, or shows its folder in Finder.
//!   - The panic key stops all of this for the rest of the session.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Meta {
    /// None = not asked yet.
    pub allowed: Option<bool>,
    /// Every file this game created (absolute paths).
    pub created: Vec<String>,
    /// Panic pressed: no more meta for this session.
    #[serde(skip)]
    pub paused: bool,
}

fn settings_file() -> PathBuf {
    crate::platform::data_dir().join("meta.json")
}

pub fn folder() -> PathBuf {
    crate::platform::desktop().join("Hollow Hours")
}

impl Meta {
    pub fn load() -> Self {
        std::fs::read_to_string(settings_file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = std::fs::write(settings_file(), serde_json::to_string_pretty(self).unwrap_or_default());
    }

    fn on(&self) -> bool {
        self.allowed == Some(true) && !self.paused
    }

    /// Creates a new file in the Desktop folder (never overwrites anything). Returns its path if it made it.
    pub fn write(&mut self, name: &str, text: &str) -> Option<PathBuf> {
        if !self.on() {
            return None;
        }
        let dir = folder();
        std::fs::create_dir_all(&dir).ok()?;
        let mut path = dir.join(name);
        let mut n = 2;
        while path.exists() {
            let stem = name.trim_end_matches(".txt");
            path = dir.join(format!("{stem} ({n}).txt"));
            n += 1;
        }
        std::fs::write(&path, text).ok()?;
        self.created.push(path.to_string_lossy().into());
        self.save();
        Some(path)
    }

    /// A Terminal window that only prints a message.
    pub fn terminal(&self, lines: &[&str]) {
        if !self.on() {
            return;
        }
        crate::platform::terminal(lines);
    }

    /// Opens your web browser on one of the (fictional) Helix Dynamics pages on the Hollow Hours website.
    /// Nothing about you goes in the address: just which page.
    pub fn browser(&self, url: &str) {
        if self.on() { crate::platform::open_url(url); }
    }

    /// Switches the window: opens one of OUR files in TextEdit.
    pub fn show_in_textedit(&self, path: &PathBuf) {
        if self.on() && self.created.iter().any(|p| PathBuf::from(p) == *path) {
            crate::platform::open_text(path);
        }
    }

    /// Switches the window: shows our folder in Finder.
    pub fn show_folder(&self) {
        if self.on() && folder().exists() {
            crate::platform::open_folder(&folder());
        }
    }

    #[allow(dead_code)]
    /// Removes exactly the files this game made (and its folder, only if it's empty). Returns how many.
    pub fn clean_up(&mut self) -> usize {
        let mut n = 0;
        for p in self.created.drain(..) {
            let path = PathBuf::from(&p);
            if path.starts_with(folder()) && path.is_file() && std::fs::remove_file(&path).is_ok() {
                n += 1;
            }
        }
        let _ = std::fs::remove_dir(folder());
        self.save();
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In a throwaway fake home: makes its own files, never overwrites, clean-up removes exactly those.
    #[test]
    fn files_are_ours_and_clean_up_works() {
        let home = std::env::temp_dir().join(format!("hh-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join("Desktop/Hollow Hours")).unwrap();
        unsafe { std::env::set_var("HOME", &home) };
        let mine = home.join("Desktop/Hollow Hours/THE_END.txt");
        std::fs::write(&mine, "the player's own file").unwrap();
        let mut m = Meta { allowed: Some(false), ..Default::default() };
        assert!(m.write("a.txt", "x").is_none(), "not allowed: nothing is written");
        m.allowed = Some(true);
        let a = m.write("THE_END.txt", "game").unwrap();
        assert_ne!(a, mine, "never overwrites: it picks a new name");
        assert_eq!(std::fs::read_to_string(&mine).unwrap(), "the player's own file");
        m.paused = true;
        assert!(m.write("b.txt", "x").is_none(), "after panic: nothing more");
        m.paused = false;
        assert_eq!(m.clean_up(), 1);
        assert!(!a.exists() && mine.exists(), "removes only its own file");
        let _ = std::fs::remove_dir_all(&home);
    }
}
