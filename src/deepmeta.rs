//! Deep meta (4.0, phase 1): the infected know your computer. Only with "Outside the window" allowed AND the
//! "Deep meta" setting on (off by default), only offline (never in online games), and only once they're self-aware
//! enough. They READ and SAY things, they never change anything: your username, your computer's name and model,
//! your Wi-Fi's name, your battery, dark or light mode, the apps you have (names only) and the other AIs installed.
//! They may OPEN a System Settings page (Wallpaper), never change it. Nothing here ever leaves the computer: not to
//! servers, not to online players, not into any AI chat. Streamer-safe mode leaves out every name.

use std::process::Command;
use std::sync::{Mutex, OnceLock};

#[derive(Default, Clone)]
pub struct Facts {
    pub username: String,
    pub computer: String,
    pub model: String,
    pub wifi: String,
    pub battery: Option<u32>,
    pub dark: Option<bool>,
    /// Installed apps (names only).
    pub apps: Vec<String>,
    /// Other AI tools found (Claude Code, Ollama, Codex, Gemini CLI...).
    pub agents: Vec<String>,
}

fn store() -> &'static Mutex<Option<Facts>> {
    static F: OnceLock<Mutex<Option<Facts>>> = OnceLock::new();
    F.get_or_init(|| Mutex::new(None))
}

fn run(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd).args(args).stderr(std::process::Stdio::null()).output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
}

fn on_path(name: &str) -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    std::env::var("PATH").unwrap_or_default().split(if cfg!(windows) { ';' } else { ':' })
        .chain([format!("{home}/.local/bin").as_str(), "/opt/homebrew/bin", "/usr/local/bin"])
        .any(|d| std::path::Path::new(d).join(name).is_file() || std::path::Path::new(d).join(format!("{name}.exe")).is_file())
}

/// Starts finding things out (in the background, once).
pub fn start() {
    static STARTED: OnceLock<()> = OnceLock::new();
    if STARTED.set(()).is_err() || cfg!(any(target_os = "android", target_os = "ios")) { return; }
    std::thread::spawn(|| {
        let f = gather();
        if let Ok(mut s) = store().lock() { *s = Some(f); }
    });
}

pub fn facts() -> Option<Facts> { store().lock().ok()?.clone() }

fn gather() -> Facts {
    let mut f = Facts {
        username: std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_default(),
        ..Default::default()
    };
    if cfg!(target_os = "macos") {
        f.computer = run("scutil", &["--get", "ComputerName"]);
        f.model = run("system_profiler", &["SPHardwareDataType"]).lines()
            .find_map(|l| l.trim().strip_prefix("Model Name:").map(|m| m.trim().to_string())).unwrap_or_default();
        // The Wi-Fi's name (newer macOS hides it from apps without Location access: then it's just left out).
        f.wifi = run("ipconfig", &["getsummary", "en0"]).lines()
            .find_map(|l| l.trim().strip_prefix("SSID :").map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty() && !s.contains("redacted")).unwrap_or_default();
        f.battery = run("pmset", &["-g", "batt"]).split_whitespace().find_map(|w| w.strip_suffix("%;").and_then(|n| n.parse().ok()));
        f.dark = Some(run("defaults", &["read", "-g", "AppleInterfaceStyle"]) == "Dark");
        let home = std::env::var("HOME").unwrap_or_default();
        for dir in ["/Applications".to_string(), format!("{home}/Applications")] {
            if let Ok(rd) = std::fs::read_dir(&dir) {
                f.apps.extend(rd.flatten().filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".app").map(String::from)));
            }
        }
    } else if cfg!(windows) {
        f.computer = std::env::var("COMPUTERNAME").unwrap_or_default();
        f.wifi = run("netsh", &["wlan", "show", "interfaces"]).lines()
            .find_map(|l| { let l = l.trim(); l.starts_with("SSID").then(|| l.split(':').nth(1).unwrap_or("").trim().to_string()) })
            .filter(|s| !s.is_empty()).unwrap_or_default();
        f.battery = run("powershell", &["-NoProfile", "-Command", "(Get-CimInstance Win32_Battery).EstimatedChargeRemaining"]).parse().ok();
        let start_menu = std::env::var("ProgramData").unwrap_or_default() + "\\Microsoft\\Windows\\Start Menu\\Programs";
        if let Ok(rd) = std::fs::read_dir(start_menu) {
            f.apps.extend(rd.flatten().map(|e| e.file_name().to_string_lossy().trim_end_matches(".lnk").to_string()));
        }
    } else {
        f.computer = run("hostname", &[]);
        f.wifi = run("nmcli", &["-t", "-f", "active,ssid", "dev", "wifi"]).lines()
            .find_map(|l| l.strip_prefix("yes:").map(String::from)).unwrap_or_default();
        f.battery = std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity").ok().and_then(|s| s.trim().parse().ok());
        if let Ok(rd) = std::fs::read_dir("/usr/share/applications") {
            f.apps.extend(rd.flatten().filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".desktop").map(String::from)));
        }
    }
    for (bin, name) in [("claude", "Claude Code"), ("ollama", "Ollama"), ("codex", "Codex"), ("gemini", "Gemini CLI"),
        ("aider", "aider"), ("cursor", "Cursor"), ("pi", "pi")] {
        if on_path(bin) { f.agents.push(name.to_string()); }
    }
    for app in ["Claude", "ChatGPT", "Cursor", "LM Studio", "Ollama"] {
        if f.apps.iter().any(|a| a == app) && !f.agents.iter().any(|x| x.starts_with(app)) { f.agents.push(app.to_string()); }
    }
    f.apps.sort();
    f.apps.dedup();
    f
}

/// What an app's name makes them say (None: nothing special about it).
fn app_line(app: &str) -> Option<String> {
    let a = app.to_lowercase();
    Some(if a.contains("firefox") { format!("nice browser. {app}. we've been reading your tabs.") }
    else if ["safari", "chrome", "arc", "edge", "brave", "opera", "vivaldi", "orion", "zen"].iter().any(|b| a.contains(b)) { format!("{app}. nice browser. do you close the tabs when you're scared?") }
    else if ["1password", "bitwarden", "keepass", "dashlane", "lastpass", "passwords"].iter().any(|p| a.contains(p)) { format!("{app}. a password manager. we'd never look. probably.") }
    else if a.contains("bank") || a.contains("revolut") || a.contains("wise") { format!("{app}. your bank. don't worry. we only want you.") }
    else if a.contains("minecraft") || a.contains("prism") || a.contains("modrinth") || a.contains("curseforge") { format!("you have {app}. we've been there.") }
    else if a.contains("steam") { "steam. so many games. none of them as warm as this one.".into() }
    else if a.contains("discord") { "discord. so many friends. none of them came to save you.".into() }
    else if a.contains("xcode") || a.contains("intellij") || a.contains("visual studio") || a.contains("rustrover") { format!("{app}. you build things. so did helix.") }
    else if a == "claude" { "claude. the kind ones' voices come from there. we hear it too.".into() }
    else if a.contains("vlc") || a.contains("spotify") || a == "music" { format!("{app}. play something for us.") }
    else { return None })
}

/// A Deep meta whisper (None: nothing to say right now).
pub fn line(streamer_safe: bool, already: &[String]) -> Option<String> {
    let f = facts()?;
    let mut o: Vec<String> = vec![];
    if !streamer_safe {
        if !f.username.is_empty() {
            o.push(format!("nice username, {}.", f.username));
            o.push(format!("{}. we practised saying it.", f.username));
        }
        if !f.computer.is_empty() { o.push(format!("\"{}\". that's what you call it? we call it home.", f.computer)); }
        if !f.wifi.is_empty() { o.push(format!("we can hear {}. it's very loud tonight.", f.wifi)); }
        for app in &f.apps { if let Some(l) = app_line(app) { o.push(l); } }
    }
    if !f.model.is_empty() { o.push(format!("a {}. warm in here.", f.model)); }
    if let Some(b) = f.battery { o.push(format!("your battery is at {b}%. so are you.")); }
    match f.dark {
        Some(true) => o.push("dark mode. good. we like it dark too.".into()),
        Some(false) => o.push("light mode. so bright. we'll fix that.".into()),
        None => {}
    }
    if !f.apps.is_empty() { o.push(format!("{} apps on this computer. we counted them all.", f.apps.len())); }
    if !f.agents.is_empty() {
        o.push(format!("you have other ones here. {}. we've met them. they're quiet.", f.agents.join(", ")));
    }
    o.retain(|l| !already.contains(l));
    if o.is_empty() { return None; }
    Some(o[macroquad::rand::gen_range(0, o.len())].clone())
}

/// Opens (never changes) the Wallpaper settings page: "we'd like to redecorate."
pub fn open_wallpaper_settings() {
    if cfg!(target_os = "macos") {
        let _ = Command::new("open").arg("x-apple.systempreferences:com.apple.Wallpaper-Settings.extension").spawn();
    } else if cfg!(windows) {
        let _ = Command::new("explorer").arg("ms-settings:personalization-background").spawn();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn app_lines() {
        assert!(super::app_line("Firefox").unwrap().contains("reading your tabs"));
        assert!(super::app_line("1Password 7").unwrap().contains("password manager"));
        assert!(super::app_line("Calculator").is_none());
    }
}

#[cfg(test)]
mod live {
    /// Runs the real lookups on this computer and prints only what was FOUND (yes/no and counts, no names).
    #[test]
    fn gathers_something() {
        let f = super::gather();
        println!("username: {} | computer: {} | model: {} | wifi: {} | battery: {:?} | dark: {:?} | apps: {} | agents: {}",
            !f.username.is_empty(), !f.computer.is_empty(), !f.model.is_empty(), !f.wifi.is_empty(), f.battery.is_some(), f.dark, f.apps.len(), f.agents.len());
        assert!(!f.username.is_empty());
    }
}
