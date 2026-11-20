//! Auto-updating: on launch the game checks GitHub for a newer release, downloads it in the background and
//! installs it when you press U on the title screen (or when you quit). Only official builds update themselves
//! (CI bakes in HH_BUILD = the release number); a build you made yourself never gets replaced.
//! macOS replaces the .app, Windows and Linux replace the program file; Flatpak, Android and iPhone just get told.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

const API: &str = "https://api.github.com/repos/jdj212223-ctrl/hollow-hours/releases/latest";
const DL: &str = "https://github.com/jdj212223-ctrl/hollow-hours/releases/latest/download";

#[derive(Clone, PartialEq, Debug)]
pub enum State {
    Off,
    Checking,
    UpToDate,
    Downloading(String),
    /// Downloaded and unpacked: (version, where the new program is).
    Ready(String, PathBuf),
    /// Newer, but this kind of install can't replace itself (Flatpak, phones): (version).
    Manual(String),
    Failed(String),
}

#[derive(Clone)]
pub struct Updater { pub state: Arc<Mutex<State>> }

impl Default for Updater { fn default() -> Self { Updater { state: Arc::new(Mutex::new(State::Off)) } } }

/// This build's release number (None for a build made by hand).
pub fn build() -> Option<u32> { option_env!("HH_BUILD")?.parse().ok() }

/// "3.0.45" for a release, "3.0.0-dev" for a build made by hand.
pub fn version() -> String {
    let (ma, mi) = major_minor(env!("CARGO_PKG_VERSION"));
    match build() { Some(b) => format!("{ma}.{mi}.{b}"), None => format!("{}-dev", env!("CARGO_PKG_VERSION")) }
}

fn major_minor(v: &str) -> (u32, u32) {
    let mut p = v.trim_start_matches('v').split('.').map(|x| x.parse().unwrap_or(0));
    (p.next().unwrap_or(0), p.next().unwrap_or(0))
}

/// "v3.0.47" → (3, 0, 47)
pub fn parse(tag: &str) -> Option<(u32, u32, u32)> {
    let p: Vec<u32> = tag.trim().trim_start_matches('v').split('.').map(|x| x.parse().ok()).collect::<Option<_>>()?;
    (p.len() == 3).then(|| (p[0], p[1], p[2]))
}

/// Is `tag` newer than (major, minor, build)?
pub fn newer(tag: &str, cur: (u32, u32, u32)) -> bool { parse(tag).is_some_and(|t| t > cur) }

/// The download for this computer (None: can't update itself here).
fn asset() -> Option<&'static str> {
    if std::env::var("FLATPAK_ID").is_ok() { return None; }
    if cfg!(target_os = "macos") { Some("Hollow-Hours-macOS.zip") }
    else if cfg!(all(windows, target_arch = "aarch64")) { Some("Hollow-Hours-Windows-ARM64.zip") }
    else if cfg!(windows) { Some("Hollow-Hours-Windows.zip") }
    else if cfg!(all(target_os = "linux", target_arch = "x86_64")) { Some("Hollow-Hours-Linux-x86_64.tar.gz") }
    else if cfg!(all(target_os = "linux", target_arch = "aarch64")) { Some("Hollow-Hours-Linux-aarch64.tar.gz") }
    else { None }
}

fn dir() -> PathBuf { crate::platform::data_dir().join("update") }

impl Updater {
    pub fn get(&self) -> State { self.state.lock().map(|s| s.clone()).unwrap_or(State::Off) }
    fn set(&self, s: State) { if let Ok(mut m) = self.state.lock() { *m = s; } }

    /// Checks (and downloads) in the background. Does nothing for hand-made builds or with auto-update off.
    pub fn start(&self, enabled: bool) {
        let Some(b) = build() else { return };
        if !enabled || cfg!(any(target_os = "android", target_os = "ios")) { return; }
        let me = self.clone();
        me.set(State::Checking);
        std::thread::spawn(move || {
            let (ma, mi) = major_minor(env!("CARGO_PKG_VERSION"));
            let r = (|| -> Result<State, String> {
                let v: serde_json::Value = ureq::get(API).set("User-Agent", "HollowHours-updater").set("Accept", "application/vnd.github+json")
                    .timeout(std::time::Duration::from_secs(10)).call().map_err(|e| e.to_string())?.into_json().map_err(|e| e.to_string())?;
                let tag = v["tag_name"].as_str().unwrap_or("").to_string();
                if !newer(&tag, (ma, mi, b)) { return Ok(State::UpToDate); }
                let ver = tag.trim_start_matches('v').to_string();
                let Some(file) = asset() else { return Ok(State::Manual(ver)) };
                me.set(State::Downloading(ver.clone()));
                let _ = std::fs::remove_dir_all(dir());
                std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
                let zip = dir().join(file);
                let resp = ureq::get(&format!("{DL}/{file}")).set("User-Agent", "HollowHours-updater")
                    .timeout(std::time::Duration::from_secs(600)).call().map_err(|e| e.to_string())?;
                let mut out = std::fs::File::create(&zip).map_err(|e| e.to_string())?;
                std::io::copy(&mut resp.into_reader(), &mut out).map_err(|e| e.to_string())?;
                drop(out);
                let new = dir().join("new");
                std::fs::create_dir_all(&new).map_err(|e| e.to_string())?;
                unpack(&zip, &new)?;
                let program = find_program(&new).ok_or("the download didn't have the game in it")?;
                Ok(State::Ready(ver, program))
            })();
            me.set(r.unwrap_or_else(State::Failed));
        });
    }

    /// Installs the downloaded update and starts it. Returns false if there's nothing to install
    /// (on success the game should quit right away: the new one is already starting).
    pub fn install(&self) -> bool {
        let State::Ready(_, program) = self.get() else { return false };
        match install(&program) {
            Ok(()) => true,
            Err(e) => { self.set(State::Failed(e)); false }
        }
    }
}

fn unpack(archive: &Path, to: &Path) -> Result<(), String> {
    let ok = if cfg!(target_os = "macos") {
        Command::new("ditto").args(["-x", "-k"]).arg(archive).arg(to).status()
    } else if cfg!(windows) {
        Command::new("powershell").args(["-NoProfile", "-Command", "Expand-Archive", "-Force", "-LiteralPath"]).arg(archive)
            .arg("-DestinationPath").arg(to).status()
    } else {
        Command::new("tar").arg("xzf").arg(archive).arg("-C").arg(to).status()
    };
    match ok { Ok(s) if s.success() => Ok(()), _ => Err("couldn't unpack the update".into()) }
}

/// The new program inside the unpacked download.
fn find_program(new: &Path) -> Option<PathBuf> {
    let p = if cfg!(target_os = "macos") { new.join("Hollow Hours.app") }
        else if cfg!(windows) { new.join("Hollow Hours").join("Hollow Hours.exe") }
        else { new.join("hollow-hours").join("hollow-hours") };
    p.exists().then_some(p)
}

fn install(program: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if cfg!(target_os = "macos") {
        // Replace the whole .app this game runs from (…/Hollow Hours.app/Contents/MacOS/hollow-hours-game).
        let app = exe.ancestors().nth(3).filter(|p| p.extension().is_some_and(|e| e == "app")).ok_or("not running from an .app")?.to_path_buf();
        let old = dir().join("old.app");
        let _ = std::fs::remove_dir_all(&old);
        run(Command::new("mv").arg(&app).arg(&old))?;
        if let Err(e) = run(Command::new("mv").arg(program).arg(&app)) {
            let _ = run(Command::new("mv").arg(&old).arg(&app)); // put the old one back
            return Err(e);
        }
        Command::new("open").arg("-n").arg(&app).spawn().map_err(|e| e.to_string())?;
    } else {
        // Windows and Linux let a running program be renamed: move it aside, put the new one in its place.
        let old = exe.with_extension("old");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(&exe, &old).map_err(|e| e.to_string())?;
        if let Err(e) = std::fs::copy(program, &exe) {
            let _ = std::fs::rename(&old, &exe);
            return Err(e.to_string());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755));
        }
        // The helper next to it (hh-mcp) too, if it came along.
        if let (Some(src), Some(dst)) = (program.parent(), exe.parent()) {
            for name in ["hh-mcp", "hh-mcp.exe"] {
                if src.join(name).exists() && dst.join(name).exists() {
                    let o = dst.join(format!("{name}.old"));
                    let _ = std::fs::remove_file(&o);
                    if std::fs::rename(dst.join(name), &o).is_ok() { let _ = std::fs::copy(src.join(name), dst.join(name)); }
                }
            }
        }
        Command::new(&exe).spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn run(c: &mut Command) -> Result<(), String> {
    match c.status() { Ok(s) if s.success() => Ok(()), Ok(s) => Err(format!("{s}")), Err(e) => Err(e.to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions() {
        assert_eq!(parse("v3.0.47"), Some((3, 0, 47)));
        assert!(newer("v3.0.47", (3, 0, 45)));
        assert!(!newer("v3.0.45", (3, 0, 45)));
        assert!(newer("v3.1.2", (3, 0, 99)));
        assert!(!newer("v2.4.38", (3, 0, 1)));
        assert!(!newer("garbage", (3, 0, 1)));
    }
}
