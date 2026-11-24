//! 4.0 phase 3: Python mods. A .hh mod in your own mods folder (mods/local) can have a script.py:
//!
//!     def on_start(game):  return [say("Ember", "A mod? For me?")]
//!     def on_tick(game):   # every 2 seconds while you play Infection
//!         if game["food"] < 0.2: return [log("Your stomach growls."), item("food")]
//!     def on_night(game, n): return [log(f"Night {n}. The mod is watching.")]
//!     def on_over(game, why): ...
//!
//! It returns actions: say(who, text), log(text), give(food=, water=), credits(n), fear(x), item("food"|"water").
//! Safety: scripts run ONLY from your own local mods (never from a server's), each in its own Python process that
//! can't open files, use the network, start programs or load other modules (an audit hook blocks it all; on macOS
//! the system sandbox too), with a time limit. No Python installed: the mod still loads, the script just doesn't run.

use super::*;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};

const RUNNER: &str = r#"
import sys, json, math, random, re, itertools, functools, collections, string, statistics
try:
    import resource
    resource.setrlimit(resource.RLIMIT_CPU, (60, 60))
except Exception:
    pass
first = json.loads(sys.stdin.readline())
src, name = first["script"], first["name"]
out = sys.stdout
ALLOWED = {"math", "random", "re", "itertools", "functools", "collections", "string", "statistics", "json"}
_imp = __import__
def safe_import(n, g=None, l=None, f=(), lv=0):
    if n.split(".")[0] not in ALLOWED:
        raise ImportError("Hollow Hours mods can't import " + n)
    return _imp(n, g, l, f, lv)
BLOCK = ("open", "os.", "subprocess.", "socket.", "shutil.", "ctypes.", "webbrowser", "urllib", "http.", "glob.", "sqlite3.", "winreg.", "msvcrt.", "pty.", "fcntl.", "mmap.", "sys._getframe")
def hook(ev, args):
    if ev.startswith(BLOCK):
        raise PermissionError("Hollow Hours mods can't do that (" + ev + ")")
    if ev == "import" and args[0].split(".")[0] not in ALLOWED | {"encodings", "codecs"}:
        raise PermissionError("Hollow Hours mods can't import " + str(args[0]))
sys.addaudithook(hook)
import builtins
SAFE = {k: getattr(builtins, k) for k in ("abs all any bool dict enumerate filter float format int isinstance len list map max min "
        "print range reversed round set sorted str sum tuple zip ValueError KeyError IndexError Exception True False None").split()}
SAFE["__import__"] = safe_import
def say(who, text): return {"say": [str(who), str(text)[:200]]}
def log(text): return {"log": str(text)[:200]}
def give(food=0.0, water=0.0): return {"give": [float(food), float(water)]}
def credits(n): return {"credits": int(n)}
def fear(x): return {"fear": float(x)}
def item(kind): return {"item": "water" if kind == "water" else "food"}
env = {"__builtins__": SAFE, "__name__": "hh_mod", "say": say, "log": log, "give": give, "credits": credits, "fear": fear, "item": item, "random": random, "math": math}
try:
    exec(compile(src, name + "/script.py", "exec"), env)
    out.write(json.dumps({"ready": True}) + "\n"); out.flush()
except Exception as e:
    out.write(json.dumps({"error": "script.py: " + type(e).__name__ + ": " + str(e)}) + "\n"); out.flush()
    sys.exit(0)
for line in sys.stdin:
    try:
        m = json.loads(line)
        fn = env.get("on_" + m["event"])
        r = fn(m["game"], *m.get("args", [])) if callable(fn) else None
        acts = r if isinstance(r, list) else ([r] if isinstance(r, dict) else [])
        out.write(json.dumps({"actions": [a for a in acts if isinstance(a, dict)][:10]}) + "\n")
    except Exception as e:
        out.write(json.dumps({"error": type(e).__name__ + ": " + str(e)}) + "\n")
    out.flush()
"#;

pub struct Script {
    pub name: String,
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<serde_json::Value>,
    /// Seconds waiting for an answer (0 = not waiting).
    waiting: f32,
}

impl Drop for Script { fn drop(&mut self) { let _ = self.child.kill(); let _ = self.child.wait(); } }

#[derive(Default)]
pub struct Scripts {
    pub list: Vec<Script>,
    tick_t: f32,
    started: bool,
    night: Option<usize>,
    over: bool,
}

fn python() -> Option<String> {
    for p in ["python3", "/usr/local/bin/python3", "/opt/homebrew/bin/python3", "/usr/bin/python3", "python"] {
        if Command::new(p).arg("-c").arg("pass").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success()) { return Some(p.to_string()); }
    }
    None
}

/// Starts one script in its own locked-down Python.
pub fn spawn(name: &str, src: &str) -> Result<Script, String> {
    let py = python().ok_or("Python 3 isn't installed")?;
    let args = ["-I", "-S", "-B", "-u", "-c", RUNNER];
    let mut cmd = if cfg!(target_os = "macos") && std::path::Path::new("/usr/bin/sandbox-exec").exists() {
        // The system sandbox: no network, no writing files, no other programs.
        // Python's own program, and its folder (the macOS framework build restarts itself through Python.app).
        let out = Command::new(&py).args(["-c", "import sys, os; print(os.path.realpath(sys.executable)); print(os.path.realpath(sys.base_prefix))"]).output().ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
        let mut l = out.lines();
        let (real, home) = (l.next().unwrap_or("").trim().to_string(), l.next().unwrap_or("").trim().to_string());
        let profile = format!("(version 1)(allow default)(deny network*)(deny file-write*)(allow file-write* (literal \"/dev/null\"))\
            (deny process-exec*)(allow process-exec (literal \"{real}\") (subpath \"{home}\"))(deny process-fork)");
        let mut c = Command::new("/usr/bin/sandbox-exec");
        c.arg("-p").arg(profile).arg(&real).args(args);
        c
    } else {
        let mut c = Command::new(&py);
        c.args(args);
        c
    };
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|e| e.to_string())?;
    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) { if tx.send(v).is_err() { break; } }
        }
    });
    writeln!(stdin, "{}", json!({"script": src, "name": name})).map_err(|e| e.to_string())?;
    // Wait (briefly) for it to load.
    match rx.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(v) if v["ready"].as_bool() == Some(true) => Ok(Script { name: name.into(), child, stdin, rx, waiting: 0.0 }),
        Ok(v) => Err(v["error"].as_str().unwrap_or("it didn't start").to_string()),
        Err(_) => Err("it took too long to start".into()),
    }
}

impl Script {
    fn send(&mut self, event: &str, game: &serde_json::Value, args: serde_json::Value) {
        if self.waiting > 0.0 { return; } // one question at a time
        if writeln!(self.stdin, "{}", json!({"event": event, "game": game, "args": args})).is_ok() { self.waiting = 0.001; }
    }
}

/// The scripts of the local mods being played (restarted for each game).
pub fn load(g: &mut Game, mods: &[serde_json::Value]) {
    g.py = Scripts::default();
    if cfg!(any(target_os = "android", target_os = "ios")) { return; }
    for m in mods.iter().filter(|m| m["local"].as_bool() == Some(true)) {
        let (Some(name), Some(src)) = (m["name"].as_str(), m["script"].as_str()) else { continue };
        match spawn(name, src) {
            Ok(s) => g.py.list.push(s),
            Err(e) => g.log.push(format!("Mod \"{name}\": its script didn't run ({e}).")),
        }
    }
}

fn state(g: &Game) -> serde_json::Value {
    let near: Vec<&str> = g.chars.iter().filter(|c| c.map == g.map && c.pos.distance(g.pos) < 8.0 * T).map(|c| c.name).collect();
    json!({"place": map_name(g.map), "infection": g.inf, "food": g.food, "water": g.water, "fear": g.fear, "survived": g.survived,
        "credits": g.credits, "night": g.story.as_ref().map(|s| s.night + 1), "hidden": g.hidden, "near": near,
        "infected": g.chars.iter().filter(|c| c.infected).count(), "time": g.time_text()})
}

/// Every frame while playing: events out, actions in.
pub fn update(g: &mut Game, dt: f32) {
    if g.py.list.is_empty() || !g.infection { return; }
    let st = state(g);
    if !g.py.started { g.py.started = true; for s in &mut g.py.list { s.send("start", &st, json!([])); } }
    let night = g.story.as_ref().map(|s| s.night + 1);
    if night != g.py.night { g.py.night = night; if let Some(n) = night { for s in &mut g.py.list { s.send("night", &st, json!([n])); } } }
    if g.screen == Screen::Over && !g.py.over { g.py.over = true; let why = g.over_reason.clone(); for s in &mut g.py.list { s.send("over", &st, json!([why])); } }
    g.py.tick_t += dt;
    if g.py.tick_t >= 2.0 { g.py.tick_t = 0.0; for s in &mut g.py.list { s.send("tick", &st, json!([])); } }
    let mut acts = vec![];
    let mut dead = vec![];
    for (k, s) in g.py.list.iter_mut().enumerate() {
        if s.waiting > 0.0 { s.waiting += dt; }
        while let Ok(v) = s.rx.try_recv() {
            s.waiting = 0.0;
            if let Some(e) = v["error"].as_str() { acts.push((s.name.clone(), json!({"log": format!("(mod error) {e}")}))); }
            for a in v["actions"].as_array().cloned().unwrap_or_default() { acts.push((s.name.clone(), a)); }
        }
        if s.waiting > 3.0 { dead.push(k); }
    }
    for k in dead.into_iter().rev() {
        let s = g.py.list.remove(k);
        g.log.push(format!("Mod \"{}\": its script was stopped (it took too long).", s.name));
    }
    for (_, a) in acts { apply(g, &a); }
}

fn apply(g: &mut Game, a: &serde_json::Value) {
    if let Some(t) = a["log"].as_str() { g.log.push(t.chars().take(200).collect()); }
    if let Some([who, text]) = a["say"].as_array().map(|v| v.as_slice()) {
        let (who, text) = (who.as_str().unwrap_or(""), text.as_str().unwrap_or(""));
        if let Some(c) = g.chars.iter_mut().find(|c| c.name.eq_ignore_ascii_case(who)) {
            c.bubble = text.chars().take(200).collect();
            c.via = "mod".into();
            c.bubble_t = 4.0 + text.len() as f32 / 30.0;
            g.log.push(format!("{}: \"{text}\"", c.name));
        }
    }
    if let Some([f, w]) = a["give"].as_array().map(|v| v.as_slice()) {
        g.food = (g.food + f.as_f64().unwrap_or(0.0).clamp(-0.3, 0.3) as f32).clamp(0.0, 1.0);
        g.water = (g.water + w.as_f64().unwrap_or(0.0).clamp(-0.3, 0.3) as f32).clamp(0.0, 1.0);
    }
    if let Some(n) = a["credits"].as_i64() { g.credits = (g.credits as i64 + n.clamp(-20, 20)).max(0) as u32; }
    if let Some(x) = a["fear"].as_f64() { g.fear = (g.fear + (x as f32).clamp(-0.5, 0.5)).clamp(0.0, 1.0); }
    if let Some(k) = a["item"].as_str() {
        if let Some(p) = random_floor(g.map) { g.items.push((g.map, p, k == "water")); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A script runs and answers; and it can't read files, use the network, start programs or import os.
    #[test]
    fn scripts_run_and_stay_in_the_sandbox() {
        if python().is_none() { return; }
        let src = r#"
def on_tick(game):
    out = [log("tick " + game["place"])]
    for bad in ["open('/etc/hosts').read()", "__import__('os').system('true')", "__import__('socket')",
                "().__class__.__base__.__subclasses__()[0].__init__.__globals__", "__import__('subprocess')"]:
        try:
            eval(bad)
            out.append(log("ESCAPED " + bad))
        except Exception:
            pass
    return out
"#;
        // eval isn't in the safe builtins: that alone stops most of these. Try the same things directly too.
        let src2 = r#"
def on_tick(game):
    out = []
    try:
        open('/etc/hosts'); out.append(log('ESCAPED open'))
    except Exception: pass
    try:
        import os; out.append(log('ESCAPED import os'))
    except Exception: pass
    try:
        for c in ().__class__.__base__.__subclasses__():
            if c.__name__ == '_wrap_close':
                c.__init__.__globals__['system']('true'); out.append(log('ESCAPED system'))
    except Exception: pass
    try:
        for c in ().__class__.__base__.__subclasses__():
            if c.__name__ == 'FileIO':
                c('/tmp/hh-mod-escape', 'w'); out.append(log('ESCAPED FileIO'))
    except Exception: pass
    out.append(say("Ember", "safe"))
    return out
"#;
        for s in [src, src2] {
            let mut sc = spawn("test", s).expect("starts");
            sc.send("tick", &json!({"place": "The beach"}), json!([]));
            let v = sc.rx.recv_timeout(std::time::Duration::from_secs(5)).expect("answers");
            let text = v.to_string();
            assert!(!text.contains("ESCAPED"), "got out of the sandbox: {text}");
            assert!(text.contains("tick The beach") || text.contains("safe"), "{text}");
        }
        assert!(!std::path::Path::new("/tmp/hh-mod-escape").exists());
        // A broken script reports its error instead of crashing anything.
        assert!(spawn("bad", "def on_tick(:").is_err());
    }
}
