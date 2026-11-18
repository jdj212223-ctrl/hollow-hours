//! The characters' brains. Each one talks through a real AI:
//!   - "ollama": a model running on this computer (offline, free), e.g. qwen3:4b-instruct or deepseek-r1:1.5b
//!   - "anthropic" / "openai" / "gemini": a cloud API, used only if its key is set (ANTHROPIC_API_KEY, OPENAI_API_KEY,
//!     GEMINI_API_KEY); otherwise that character falls back to Ollama
//!   - if nothing answers, a scripted line, so the game always works
//! Requests run on a background thread; the answer comes back through a channel.

use serde_json::{json, Value};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

pub const OLLAMA: &str = "http://127.0.0.1:11434";

#[derive(Clone)]
pub struct Msg {
    pub user: bool,
    pub text: String,
}

pub struct Reply {
    pub who: usize,
    pub text: String,
    /// Where it came from, shown small under the bubble: "qwen3 (local)", "Claude API", "scripted"...
    pub via: String,
}

pub struct Brains {
    tx: Sender<Reply>,
    pub rx: Receiver<Reply>,
    pub ollama_models: Vec<String>,
}

impl Brains {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let ollama_models = ureq::get(&format!("{OLLAMA}/api/tags"))
            .timeout(Duration::from_millis(800))
            .call()
            .ok()
            .and_then(|r| r.into_json::<Value>().ok())
            .map(|v| v["models"].as_array().cloned().unwrap_or_default().iter().filter_map(|m| m["name"].as_str().map(String::from)).collect())
            .unwrap_or_default();
        Brains { tx, rx, ollama_models }
    }

    /// The local model to use for a character that wants `preferred` (falls back to any chat model installed).
    pub fn local_model(&self, preferred: &str) -> Option<String> {
        let has = |m: &str| self.ollama_models.iter().any(|x| x.starts_with(m));
        if has(preferred) {
            return self.ollama_models.iter().find(|x| x.starts_with(preferred)).cloned();
        }
        ["qwen3:4b-instruct", "qwen", "llama", "gemma", "mistral", "phi", "deepseek"]
            .iter()
            .find_map(|p| self.ollama_models.iter().find(|x| x.starts_with(p)).cloned())
    }

    /// Asks character `who` for a reply in the background.
    /// `cloud`: the exact cloud model for this character (e.g. "claude-opus-5-5"), or "" for the default.
    pub fn ask(&self, who: usize, name: &str, backend: &str, cloud: &str, model: String, system: String, history: Vec<Msg>, fallback: String) {
        let tx = self.tx.clone();
        let name = name.to_string();
        let backend = backend.to_string();
        let cloud = cloud.to_string();
        std::thread::spawn(move || {
            // A cloud brain that fails (no key, offline, unknown model) falls back to the local one.
            let (text, via) = match talk(&name, &backend, &cloud, &model, &system, &history).or_else(|| talk(&name, "ollama", "", &model, &system, &history)) {
                Some(r) => r,
                None => (fallback, "scripted (no AI answered)".into()),
            };
            let _ = tx.send(Reply { who, text: tidy(&text), via });
        });
    }
}

/// Short and clean for a speech bubble: no reasoning blocks, no markdown, at most ~3 sentences.
fn tidy(t: &str) -> String {
    let mut s = t.to_string();
    while let (Some(a), Some(b)) = (s.find("<think>"), s.find("</think>")) {
        if b < a { break; }
        s.replace_range(a..b + 8, "");
    }
    let s = s.replace(['*', '#', '`'], "").replace("\n\n", " ").replace('\n', " ");
    // The game's font has no emoji: keep letters and normal punctuation only.
    let s: String = s.replace(['’', '‘'], "'").replace(['“', '”'], "\"").replace('…', "...").replace(['—', '–'], "-")
        .chars().filter(|c| (*c as u32) < 0x2000).collect();
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let s = s.trim().trim_matches('"').to_string();
    if s.chars().count() <= 300 { return s; }
    // Too long for a bubble: end at the last whole sentence that fits, or at least at a whole word
    // (cutting mid-word once turned "a piece of furniture" into "a piece of fu...").
    let head: String = s.chars().take(300).collect();
    if let Some(i) = head.rfind(['.', '!', '?']).filter(|&i| i > 80) {
        return head[..=i].to_string();
    }
    match head.rfind(' ') {
        Some(i) => format!("{}...", head[..i].trim_end_matches([',', ';', ':'])),
        None => head,
    }
}

/// The Claude desktop app is installed and its `claude` command works: Claude characters can talk through it (your
/// own Claude account, no API key needed).
pub fn claude_cli() -> Option<String> {
    if !std::path::Path::new("/Applications/Claude.app").exists() && std::env::var("HOLLOW_CLAUDE_CLI").is_err() {
        return None;
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let mut paths = vec![std::env::var("HOLLOW_CLAUDE_CLI").unwrap_or_default(), format!("{home}/.local/bin/claude"),
        "/opt/homebrew/bin/claude".into(), "/usr/local/bin/claude".into(), format!("{home}/.claude/local/claude")];
    paths.retain(|p| !p.is_empty());
    paths.into_iter().find(|p| std::path::Path::new(p).is_file())
}

/// Where Hollow Hours keeps its own things (and where the Claude sessions run, so they can always be resumed).
fn home_dir() -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Library/Application Support/Hollow Hours");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn sessions_file() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("HOLLOW_SESSIONS_FILE") { return p.into(); } // tests
    home_dir().join("claude_sessions.json")
}

/// One Claude session per character that carries on from game to game: the first time you talk to Sonnet 4.6 a
/// session named "Hollow Hours: Sonnet 4.6" starts, and every later game continues that same session (so it
/// remembers you). Saved in ~/Library/Application Support/Hollow Hours/claude_sessions.json; `claude --resume` lists them.
fn sessions() -> &'static std::sync::Mutex<std::collections::HashMap<String, String>> {
    static S: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, String>>> = std::sync::OnceLock::new();
    S.get_or_init(|| {
        let saved = std::fs::read_to_string(sessions_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        std::sync::Mutex::new(saved)
    })
}

fn save_sessions(map: &std::collections::HashMap<String, String>) {
    let file = sessions_file();
    if std::fs::write(&file, serde_json::to_string_pretty(map).unwrap_or_default()).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600));
        }
    }
}

fn remember_session(name: &str, id: Option<String>) {
    if let Ok(mut m) = sessions().lock() {
        match id { Some(id) => { m.insert(name.to_string(), id); } None => { m.remove(name); } }
        save_sessions(&m);
    }
}

fn new_uuid() -> String {
    use std::io::Read;
    let mut b = [0u8; 16];
    if std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_err() {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        b = t.to_le_bytes();
    }
    b[6] = (b[6] & 0x0f) | 0x40; // version 4
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

/// Asks real Claude through the Claude app's command line (`claude -p`), with the right model, continuing this
/// character's session.
fn via_claude_cli(name: &str, cloud: &str, system: &str, history: &[Msg]) -> Option<(String, String)> {
    let r = ask_claude_cli(name, cloud, system, history);
    if r.is_some() { return r; }
    // The saved session is gone (deleted, or made on another Mac): forget it and start a fresh one.
    if sessions().lock().ok()?.contains_key(name) {
        remember_session(name, None);
        return ask_claude_cli(name, cloud, system, history);
    }
    None
}

fn ask_claude_cli(name: &str, cloud: &str, system: &str, history: &[Msg]) -> Option<(String, String)> {
    let cli = claude_cli()?;
    let alias = if cloud.contains("haiku") { "haiku" } else if cloud.contains("sonnet") { "sonnet" } else { "opus" };
    let existing = sessions().lock().ok()?.get(name).cloned();
    let last = history.iter().rev().find(|m| m.user).map(|m| m.text.clone()).unwrap_or_default();
    // A new session gets the character and the story so far; a continued one remembers, so it only needs the fresh
    // situation and the player's new line.
    let prompt = match &existing {
        None => {
            let mut convo = String::new();
            for m in history.iter().rev().take(8).rev() {
                convo += &format!("{}: {}\n", if m.user { "Player" } else { "You" }, m.text);
            }
            format!("{system}\n\nThe conversation so far:\n{convo}\nWrite only your next line (1-3 short sentences), nothing else.")
        }
        Some(_) => format!("{system}\n\nThe player says: {last}\nWrite only your next line (1-3 short sentences), nothing else."),
    };
    let mut args: Vec<String> = vec!["-p".into(), "--model".into(), alias.into(), "--output-format".into(), "text".into()];
    match &existing {
        Some(id) => args.extend(["--resume".into(), id.clone()]),
        None => {
            let id = new_uuid();
            args.extend(["--session-id".into(), id.clone(), "--name".into(), format!("Hollow Hours: {name}")]);
            remember_session(name, Some(id));
        }
    }
    args.extend(["--tools".into(), String::new()]);
    // The prompt goes in through stdin (--tools takes a list and would swallow it as an argument).
    use std::io::Write;
    let mut child = std::process::Command::new(cli)
        .args(&args)
        .current_dir(home_dir()) // always the same folder, so every session can be resumed next time
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(prompt.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !text.is_empty()).then(|| (text, format!("real Claude ({alias}) via the Claude app")))
}

fn key(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|k| !k.trim().is_empty())
}

fn talk(name: &str, backend: &str, cloud: &str, model: &str, system: &str, history: &[Msg]) -> Option<(String, String)> {
    let msgs: Vec<Value> = history.iter().map(|m| json!({"role": if m.user { "user" } else { "assistant" }, "content": m.text})).collect();
    let t = Duration::from_secs(40);
    match backend {
        "anthropic" if key("ANTHROPIC_API_KEY").is_some() => {
            let model = if cloud.is_empty() { std::env::var("HOLLOW_CLAUDE_MODEL").unwrap_or_else(|_| "claude-haiku-5-5".into()) } else { cloud.to_string() };
            let v: Value = ureq::post("https://api.anthropic.com/v1/messages")
                .timeout(t)
                .set("x-api-key", &key("ANTHROPIC_API_KEY")?)
                .set("anthropic-version", "2023-06-01")
                .send_json(json!({"model": model, "max_tokens": 160, "system": system, "messages": msgs}))
                .ok()?
                .into_json()
                .ok()?;
            Some((v["content"][0]["text"].as_str()?.to_string(), format!("Claude API ({model})")))
        }
        "anthropic" => via_claude_cli(name, cloud, system, history),
        "openai" if key("OPENAI_API_KEY").is_some() => {
            let model = std::env::var("HOLLOW_OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into());
            let mut all = vec![json!({"role": "system", "content": system})];
            all.extend(msgs);
            let v: Value = ureq::post("https://api.openai.com/v1/chat/completions")
                .timeout(t)
                .set("Authorization", &format!("Bearer {}", key("OPENAI_API_KEY")?))
                .send_json(json!({"model": model, "max_tokens": 160, "messages": all}))
                .ok()?
                .into_json()
                .ok()?;
            Some((v["choices"][0]["message"]["content"].as_str()?.to_string(), "OpenAI API".into()))
        }
        "gemini" if key("GEMINI_API_KEY").is_some() => {
            let model = std::env::var("HOLLOW_GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.0-flash".into());
            let contents: Vec<Value> = history.iter().map(|m| json!({"role": if m.user { "user" } else { "model" }, "parts": [{"text": m.text}]})).collect();
            let v: Value = ureq::post(&format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"))
                .timeout(t)
                .set("x-goog-api-key", &key("GEMINI_API_KEY")?)
                .send_json(json!({"systemInstruction": {"parts": [{"text": system}]}, "contents": contents}))
                .ok()?
                .into_json()
                .ok()?;
            Some((v["candidates"][0]["content"]["parts"][0]["text"].as_str()?.to_string(), "Gemini API".into()))
        }
        _ if model.is_empty() => None,
        _ => {
            // Local: Ollama.
            let mut all = vec![json!({"role": "system", "content": system})];
            all.extend(msgs);
            let v: Value = ureq::post(&format!("{OLLAMA}/api/chat"))
                .timeout(Duration::from_secs(90))
                .send_json(json!({"model": model, "messages": all, "stream": false, "options": {"num_predict": 140, "temperature": 0.8}}))
                .ok()?
                .into_json()
                .ok()?;
            let short = model.split(':').next().unwrap_or(model).to_string();
            Some((v["message"]["content"].as_str()?.to_string(), format!("{short} (local, offline)")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests keep their sessions in their own file, never the player's.
    fn test_sessions_file() {
        let f = std::env::temp_dir().join("hh-test-claude-sessions.json");
        unsafe { std::env::set_var("HOLLOW_SESSIONS_FILE", &f); }
    }

    /// A character's session is saved, so the next game continues it instead of starting a new one.
    #[test]
    fn sessions_carry_on_between_games() {
        test_sessions_file();
        remember_session("Sonnet 4.6 (test)", Some("11111111-2222-4333-8444-555555555555".into()));
        let saved: std::collections::HashMap<String, String> = serde_json::from_str(&std::fs::read_to_string(sessions_file()).unwrap()).unwrap();
        assert_eq!(saved.get("Sonnet 4.6 (test)").map(String::as_str), Some("11111111-2222-4333-8444-555555555555"), "written to disk");
        remember_session("Sonnet 4.6 (test)", None);
        let saved: std::collections::HashMap<String, String> = serde_json::from_str(&std::fs::read_to_string(sessions_file()).unwrap()).unwrap();
        assert!(!saved.contains_key("Sonnet 4.6 (test)"), "and forgotten again");
    }

    /// Long answers end at a whole sentence, never mid-word ("a piece of fu...").
    #[test]
    fn long_bubbles_end_cleanly() {
        let sonnet = "Honestly, probably not on purpose, but I wouldn't be surprised if they ignored warnings. Right now the tornado is 15 tiles south-east, so stay low and keep the house between you and it, with Opus 5.5 and Haiku 4.5 right next to you. Press H at a piece of furniture to get down behind something solid.";
        let t = tidy(sonnet);
        assert!(t.ends_with('.') && !t.contains("piece of fu…") && !t.ends_with("fu..."), "{t}");
        let words = "word ".repeat(100);
        let t = tidy(&words);
        assert!(t.ends_with("word...") && t.chars().count() <= 303, "{t}");
        assert_eq!(tidy("Short and fine."), "Short and fine.");
    }

    /// Needs Ollama running with a chat model; skips quietly otherwise.
    #[test]
    fn a_local_character_answers() {
        let b = Brains::new();
        let Some(model) = b.local_model("qwen3:4b-instruct") else { return };
        let r = talk("Clove", "ollama", "", &model, "You are Clove, a warm character in a spooky game. Reply in one short sentence.",
            &[Msg { user: true, text: "Hi Clove, is this house haunted?".into() }]);
        let (text, via) = r.expect("ollama answered");
        let text = tidy(&text);
        println!("Clove ({via}): {text}");
        assert!(!text.is_empty() && !text.contains("<think>"));
    }

    /// Needs the Claude app and its `claude` command; skips quietly otherwise.
    #[test]
    fn claude_answers_through_the_claude_app() {
        test_sessions_file();
        if claude_cli().is_none() { return; }
        let r = via_claude_cli("Haiku 4.5 (test)", "claude-haiku-4-5", "You are Haiku 4.5, the lookout in a survival game. Very short sentences.",
            &[Msg { user: true, text: "Haiku, is it safe to go outside?".into() }]);
        let (text, via) = r.expect("the Claude app answered");
        println!("Haiku ({via}): {}", tidy(&text));
        // The second message continues the same session, so Haiku remembers the first one.
        let id = sessions().lock().unwrap().get("Haiku 4.5 (test)").cloned().unwrap();
        let (again, _) = via_claude_cli("Haiku 4.5 (test)", "claude-haiku-4-5", "You are Haiku 4.5, the lookout. Very short sentences.",
            &[Msg { user: true, text: "What did I just ask you?".into() }]).expect("second answer");
        println!("Haiku, same session {id}: {}", tidy(&again));
        assert_eq!(sessions().lock().unwrap().get("Haiku 4.5 (test)"), Some(&id));
    }
}
