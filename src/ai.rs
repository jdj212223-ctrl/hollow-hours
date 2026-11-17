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
    pub fn ask(&self, who: usize, backend: &str, model: String, system: String, history: Vec<Msg>, fallback: String) {
        let tx = self.tx.clone();
        let backend = backend.to_string();
        std::thread::spawn(move || {
            // A cloud brain that fails (no key, offline, unknown model) falls back to the local one.
            let (text, via) = match talk(&backend, &model, &system, &history).or_else(|| talk("ollama", &model, &system, &history)) {
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
    if s.chars().count() > 260 { format!("{}…", s.chars().take(257).collect::<String>()) } else { s }
}

fn key(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|k| !k.trim().is_empty())
}

fn talk(backend: &str, model: &str, system: &str, history: &[Msg]) -> Option<(String, String)> {
    let msgs: Vec<Value> = history.iter().map(|m| json!({"role": if m.user { "user" } else { "assistant" }, "content": m.text})).collect();
    let t = Duration::from_secs(40);
    match backend {
        "anthropic" if key("ANTHROPIC_API_KEY").is_some() => {
            let model = std::env::var("HOLLOW_CLAUDE_MODEL").unwrap_or_else(|_| "claude-haiku-5-5".into());
            let v: Value = ureq::post("https://api.anthropic.com/v1/messages")
                .timeout(t)
                .set("x-api-key", &key("ANTHROPIC_API_KEY")?)
                .set("anthropic-version", "2023-06-01")
                .send_json(json!({"model": model, "max_tokens": 160, "system": system, "messages": msgs}))
                .ok()?
                .into_json()
                .ok()?;
            Some((v["content"][0]["text"].as_str()?.to_string(), "Claude API".into()))
        }
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

    /// Needs Ollama running with a chat model; skips quietly otherwise.
    #[test]
    fn a_local_character_answers() {
        let b = Brains::new();
        let Some(model) = b.local_model("qwen3:4b-instruct") else { return };
        let r = talk("ollama", &model, "You are Clove, a warm character in a spooky game. Reply in one short sentence.",
            &[Msg { user: true, text: "Hi Clove, is this house haunted?".into() }]);
        let (text, via) = r.expect("ollama answered");
        let text = tidy(&text);
        println!("Clove ({via}): {text}");
        assert!(!text.is_empty() && !text.contains("<think>"));
    }
}
