//! What's playing on this computer (3.0, only with "Outside the window" allowed): the corrupted AIs notice your
//! music. macOS asks Music / Spotify (only if they're already running, it never opens them), Linux asks playerctl,
//! Windows asks the system's media controls. Read locally, never sent anywhere (not to servers, not to chats).

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
    }
}
