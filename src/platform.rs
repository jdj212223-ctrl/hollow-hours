//! Everything that's different on macOS, Windows and Linux (SteamOS too), in one place: where Hollow Hours keeps
//! its files, opening links and files, the terminal window for the meta scares, and good random numbers.

#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;

pub fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default())
}

/// Where the game keeps its settings, login, sessions, mods and the MCP bridge file.
///  - macOS: ~/Library/Application Support/Hollow Hours
///  - Windows: %APPDATA%\Hollow Hours
///  - Linux / SteamOS: ~/.local/share/hollow-hours (or $XDG_DATA_HOME/hollow-hours)
pub fn data_dir() -> PathBuf {
    let dir = if cfg!(target_os = "android") {
        // The app's own private storage on Android.
        PathBuf::from("/data/data/io.github.jdj212223_ctrl.hollowhours/files")
    } else if cfg!(any(target_os = "macos", target_os = "ios")) {
        home().join("Library/Application Support/Hollow Hours")
    } else if cfg!(windows) {
        std::env::var("APPDATA").map(PathBuf::from).unwrap_or_else(|_| home().join("AppData/Roaming")).join("Hollow Hours")
    } else {
        std::env::var("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|_| home().join(".local/share")).join("hollow-hours")
    };
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// The Desktop folder (for the meta scare files).
pub fn desktop() -> PathBuf {
    #[cfg(target_os = "linux")]
    if let Ok(o) = Command::new("xdg-user-dir").arg("DESKTOP").output() {
        let p = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !p.is_empty() { return PathBuf::from(p); }
    }
    home().join("Desktop")
}

/// Makes a file readable only by you (on Windows, files in your profile already are).
pub fn private(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    let _ = path;
}

/// Opens a web link in the browser.
/// Browsers found on this computer (macOS: every known one in Applications; elsewhere: none, the default is used).
pub fn browsers() -> Vec<String> {
    if !cfg!(target_os = "macos") { return vec![]; }
    const KNOWN: [&str; 30] = ["Safari", "Google Chrome", "Firefox", "Arc", "Microsoft Edge", "Brave Browser", "Opera", "Opera GX",
        "Vivaldi", "Orion", "Zen", "Zen Browser", "DuckDuckGo", "Chromium", "Tor Browser", "Firefox Developer Edition", "Firefox Nightly",
        "Google Chrome Canary", "Google Chrome Beta", "Floorp", "LibreWolf", "Waterfox", "Comet", "Dia", "SigmaOS", "Yandex", "Sidekick",
        "Min", "Ladybird", "Safari Technology Preview"];
    let home = std::env::var("HOME").unwrap_or_default();
    let dirs = ["/Applications".to_string(), format!("{home}/Applications"), "/System/Volumes/Preboot/Cryptexes/App/System/Applications".into()];
    KNOWN.iter().filter(|b| dirs.iter().any(|d| std::path::Path::new(&format!("{d}/{b}.app")).exists())).map(|b| b.to_string()).collect()
}

/// Opens a link in the browser the player picked in Settings (or their default browser).
pub fn open_url_in(url: &str, browser: &str) {
    if cfg!(target_os = "macos") && !browser.is_empty() && browsers().iter().any(|b| b == browser) {
        let _ = Command::new("open").args(["-a", browser, url]).spawn();
    } else {
        open_url(url);
    }
}

pub fn open_url(url: &str) {
    if cfg!(target_os = "macos") {
        let _ = Command::new("open").arg(url).spawn();
    } else if cfg!(windows) {
        // Not `cmd /c start`: cmd would treat the & in the link as a new command.
        let _ = Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn();
    } else {
        let _ = Command::new("xdg-open").arg(url).spawn();
    }
}

/// Opens a folder in Finder / Explorer / the file manager.
pub fn open_folder(path: &std::path::Path) {
    if cfg!(target_os = "macos") {
        let _ = Command::new("open").arg(path).spawn();
    } else if cfg!(windows) {
        let _ = Command::new("explorer").arg(path).spawn();
    } else {
        let _ = Command::new("xdg-open").arg(path).spawn();
    }
}

/// Opens a text file in TextEdit / Notepad / the default text editor.
pub fn open_text(path: &std::path::Path) {
    if cfg!(target_os = "macos") {
        let _ = Command::new("open").arg("-e").arg(path).spawn();
    } else if cfg!(windows) {
        let _ = Command::new("notepad").arg(path).spawn();
    } else {
        let _ = Command::new("xdg-open").arg(path).spawn();
    }
}

/// A terminal window that only prints a message. `lines` must already be safe (no quotes or shell characters).
pub fn terminal(lines: &[&str]) {
    let text: String = lines.join("\n").chars().filter(|c| c.is_alphanumeric() || " .,:!?-\n".contains(*c)).collect();
    if cfg!(target_os = "macos") {
        // \\\\n here = \\n for AppleScript = \n for printf (a real newline would press Enter in Terminal).
        let shell = format!("clear; printf '\\\\n{}\\\\n\\\\n'", text.replace('\n', "\\\\n"));
        let script = format!("tell application \"Terminal\"\nactivate\ndo script \"{shell}\"\nend tell");
        let _ = Command::new("osascript").arg("-e").arg(script).spawn();
    } else if cfg!(windows) {
        let echo: Vec<String> = text.lines().map(|l| if l.is_empty() { "echo.".to_string() } else { format!("echo {l}") }).collect();
        let _ = Command::new("cmd").args(["/c", "start", "Hollow Hours", "cmd", "/k", &format!("color 0C & {}", echo.join(" & "))]).spawn();
    } else {
        // The first terminal app that's installed (Konsole on SteamOS desktop mode).
        let script = format!("clear; printf '\\n{}\\n\\n'; read _", text.replace('\n', "\\n"));
        for (app, flag) in [("konsole", "-e"), ("gnome-terminal", "--"), ("xfce4-terminal", "-x"), ("xterm", "-e")] {
            if which(app) {
                let _ = Command::new(app).args([flag, "sh", "-c", &script]).spawn();
                break;
            }
        }
    }
}

pub fn which(app: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| std::env::split_paths(&paths).any(|p| p.join(app).is_file()))
}

/// Good random bytes from the system (for login secrets and tokens), as hex.
pub fn random_hex(n: usize) -> String {
    let mut b = vec![0u8; n];
    if getrandom::getrandom(&mut b).is_err() {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        for (i, x) in b.iter_mut().enumerate() { *x = (t >> ((i % 16) * 8)) as u8 ^ i as u8; }
    }
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    let _ = getrandom::getrandom(&mut b);
    b
}

/// The computer's processor, for the Mac Studio's About screen.
pub fn cpu_name() -> String {
    if cfg!(target_os = "macos") {
        if let Ok(o) = Command::new("sysctl").args(["-n", "machdep.cpu.brand_string"]).output() {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !s.is_empty() { return s; }
        }
    } else if cfg!(windows) {
        if let Ok(s) = std::env::var("PROCESSOR_IDENTIFIER") { return s; }
    } else if let Ok(info) = std::fs::read_to_string("/proc/cpuinfo") {
        if let Some(l) = info.lines().find(|l| l.starts_with("model name")) {
            return l.split(':').nth(1).unwrap_or("").trim().to_string();
        }
    }
    std::env::consts::OS.to_string()
}

/// The name of this kind of computer, for messages ("Mac", "PC", "Linux").
pub fn os_name() -> &'static str {
    if cfg!(target_os = "macos") { "Mac" } else if cfg!(windows) { "Windows PC" } else { "Linux" }
}
