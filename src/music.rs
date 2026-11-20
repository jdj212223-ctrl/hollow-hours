//! What's playing on this computer (3.0, only with "Outside the window" allowed): the corrupted AIs notice your
//! music. macOS asks Music / Spotify (only if they're already running, it never opens them), Linux asks playerctl,
//! Windows asks the system's media controls. Read locally, never sent anywhere (not to servers, not to chats).
//! YouTube in Firefox (any system): Firefox keeps its open tabs in a session file in its profile; the game reads it,
//! keeps ONLY YouTube tabs (watch / Shorts / YouTube Music) and takes the most recently used one's title.
//! Everything else in that file is dropped right away. (macOS asks once: "access data from other apps".)

use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct Music {
    now: Arc<Mutex<Option<(String, String)>>>,
    started: bool,
    last: Option<(String, String)>,
}

impl Music {
    /// Starts watching (once). Call only when the player allowed "Outside the window".
    pub fn start(&mut self) {
        if self.started || cfg!(any(target_os = "android", target_os = "ios")) { return; }
        self.started = true;
        let now = self.now.clone();
        std::thread::spawn(move || loop {
            let t = playing();
            if let Ok(mut n) = now.lock() { *n = t; }
            std::thread::sleep(std::time::Duration::from_secs(12));
        });
    }

    /// A song that just started: (title, artist).
    pub fn new_song(&mut self) -> Option<(String, String)> {
        let cur = self.now.lock().ok()?.clone();
        if cur.is_some() && cur != self.last {
            self.last = cur.clone();
            return cur;
        }
        if cur.is_none() { self.last = None; }
        None
    }

    pub fn current(&self) -> Option<(String, String)> { self.now.lock().ok()?.clone() }
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(cmd).args(args).stderr(std::process::Stdio::null()).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn split(s: String) -> Option<(String, String)> {
    let (t, a) = s.split_once('\t')?;
    let (t, a) = (t.trim(), a.trim());
    if t.is_empty() { return None; }
    let clip = |x: &str| x.chars().take(60).collect::<String>();
    Some((clip(t), clip(if a.is_empty() { "someone" } else { a })))
}

fn playing() -> Option<(String, String)> {
    players().or_else(firefox_youtube)
}

fn players() -> Option<(String, String)> {
    if cfg!(target_os = "macos") {
        // `application "X" is running` never launches it.
        for app in ["Spotify", "Music"] {
            let script = format!(
                "if application \"{app}\" is running then\n tell application \"{app}\"\n  if player state is playing then return (name of current track) & tab & (artist of current track)\n end tell\nend if\nreturn \"\"");
            if let Some(s) = run("osascript", &["-e", &script]).and_then(split) { return Some(s); }
        }
        None
    } else if cfg!(target_os = "linux") {
        if run("playerctl", &["status"]).as_deref() != Some("Playing") { return None; }
        run("playerctl", &["metadata", "--format", "{{title}}\t{{artist}}"]).and_then(split)
    } else if cfg!(target_os = "windows") {
        let ps = "$ErrorActionPreference='Stop';Add-Type -AssemblyName System.Runtime.WindowsRuntime;\
            $m=[Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager,Windows.Media.Control,ContentType=WindowsRuntime];\
            $aw=([System.WindowsRuntimeSystemExtensions].GetMethods()|?{$_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'})[0];\
            function W($op,$t){$task=$aw.MakeGenericMethod($t).Invoke($null,@($op));$task.Wait();$task.Result};\
            $s=(W ($m::RequestAsync()) $m).GetCurrentSession();if(-not $s){exit};\
            if($s.GetPlaybackInfo().PlaybackStatus -ne 'Playing'){exit};\
            $p=W ($s.TryGetMediaPropertiesAsync()) ([Windows.Media.Control.GlobalSystemMediaTransportControlsSessionMediaProperties,Windows.Media.Control,ContentType=WindowsRuntime]);\
            Write-Output ($p.Title+\"`t\"+$p.Artist)";
        run("powershell", &["-NoProfile", "-WindowStyle", "Hidden", "-Command", ps]).and_then(split)
    } else {
        None
    }
}

/// What a corrupted AI says about it. `song_times` / `artist_times` = how often heard so far (this one included).
pub fn line(title: &str, artist: &str, song_times: u32, artist_times: u32, pick: u32) -> String {
    if song_times >= 3 {
        return format!("{title}. again. huh... you really like {title}, don't you.");
    }
    if artist == "YouTube" {
        // A YouTube tab: talk about the video.
        return match pick % 4 {
            0 => format!("\"{title}\". we can see your other window. we watch it with you."),
            1 => format!("huh. you really like \"{title}\", don't you."),
            2 => format!("{title}... is that what you do while we wait for you?"),
            _ => format!("we saw \"{title}\". we'll remember it."),
        };
    }
    if artist_times >= 3 {
        return format!("huh. you really like {artist}.");
    }
    match pick % 4 {
        0 => format!("{title}... we can hear it too."),
        1 => format!("is that {artist}? turn it up. we like it in here."),
        2 => format!("\"{title}\". it's playing outside the window. we can hear it through the glass."),
        _ => format!("{artist}. we'll remember that."),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn lines() {
        assert_eq!(super::line("Song", "Band", 1, 3, 0), "huh. you really like Band.");
        assert!(super::line("Song", "Band", 3, 3, 0).contains("you really like Song"));
        assert!(super::split("A\tB".into()).is_some());
        assert!(super::split("\tB".into()).is_none());
        assert_eq!(super::clean_youtube_title("(1865) [Black MIDI] Music using ONLY Sounds - YouTube"), "[Black MIDI] Music using ONLY Sounds");
        assert_eq!(super::clean_youtube_title("Song - YouTube Music"), "Song");
    }
}

/// Firefox profiles' session files (recovery.jsonlz4) on this system.
fn firefox_sessions() -> Vec<std::path::PathBuf> {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    let roots = [
        format!("{home}/Library/Application Support/Firefox/Profiles"),
        format!("{home}/.mozilla/firefox"),
        format!("{home}/snap/firefox/common/.mozilla/firefox"),
        format!("{home}/.var/app/org.mozilla.firefox/.mozilla/firefox"),
        format!("{appdata}/Mozilla/Firefox/Profiles"),
    ];
    let mut out = vec![];
    for r in roots {
        let Ok(rd) = std::fs::read_dir(&r) else { continue };
        for e in rd.flatten() {
            let p = e.path().join("sessionstore-backups").join("recovery.jsonlz4");
            if p.is_file() { out.push(p); }
        }
    }
    out
}

/// "mozLz40\0" + decompressed size (u32 LE) + an LZ4 block.
fn read_mozlz4(p: &std::path::Path) -> Option<serde_json::Value> {
    let b = std::fs::read(p).ok()?;
    if b.len() < 12 || &b[..8] != b"mozLz40\0" { return None; }
    let size = u32::from_le_bytes([b[8], b[9], b[10], b[11]]) as usize;
    let raw = lz4_flex::block::decompress(&b[12..], size).ok()?;
    serde_json::from_slice(&raw).ok()
}

/// "(1865) [Black MIDI] Music... - YouTube" -> "[Black MIDI] Music..."
pub fn clean_youtube_title(t: &str) -> String {
    let mut t = t.trim();
    if t.starts_with('(') { if let Some(i) = t.find(") ") { if t[1..i].chars().all(|c| c.is_ascii_digit() || c == '+') { t = &t[i + 2..]; } } }
    for suffix in [" - YouTube Music", " - YouTube"] { if let Some(s) = t.strip_suffix(suffix) { t = s; } }
    t.trim().to_string()
}

/// The YouTube tab you used last in Firefox: (title, "YouTube"). Nothing else from the session is kept.
fn firefox_youtube() -> Option<(String, String)> {
    let mut best: Option<(u64, String)> = None;
    for p in firefox_sessions() {
        let Some(v) = read_mozlz4(&p) else { continue };
        for w in v["windows"].as_array().into_iter().flatten() {
            for tab in w["tabs"].as_array().into_iter().flatten() {
                let i = tab["index"].as_u64().unwrap_or(1).max(1) as usize - 1;
                let Some(e) = tab["entries"].as_array().and_then(|a| a.get(i)) else { continue };
                let url = e["url"].as_str().unwrap_or("");
                let yt = url.contains("youtube.com/watch") || url.contains("youtube.com/shorts/") || url.contains("music.youtube.com/watch");
                if !yt { continue; }
                let last = tab["lastAccessed"].as_u64().unwrap_or(0);
                let title = clean_youtube_title(e["title"].as_str().unwrap_or(""));
                if !title.is_empty() && best.as_ref().is_none_or(|b| last > b.0) { best = Some((last, title)); }
            }
        }
    }
    best.and_then(|(_, t)| split(format!("{t}\tYouTube")))
}
