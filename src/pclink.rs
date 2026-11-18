//! "Lend Claude to my phone" (computers only). With it on (K on the title screen, needs a login and the `claude`
//! command), this computer keeps a link open to the Hollow Hours website. When your phone, logged in to the SAME
//! account, needs a Claude character's answer, the website passes the question here; this computer asks `claude`
//! (the same per-character sessions as the game on this computer) and sends the answer back.
//! Nobody else can reach it: the website only connects your phone to your own computer.

use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct PcLink {
    pub status: Arc<Mutex<String>>,
    stop: Arc<AtomicBool>,
}

impl Drop for PcLink {
    fn drop(&mut self) { self.stop.store(true, Ordering::Relaxed); }
}

fn settings_file() -> std::path::PathBuf { crate::platform::data_dir().join("pclink.json") }

/// Whether lending is switched on (remembered between games).
pub fn enabled() -> bool {
    std::fs::read_to_string(settings_file()).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v["on"].as_bool()).unwrap_or(false)
}

pub fn set_enabled(on: bool) {
    let _ = std::fs::write(settings_file(), json!({"on": on}).to_string());
}

/// Starts lending (in the background, reconnecting if the connection drops).
pub fn start(base_wss: &str, token: &str) -> PcLink {
    let status = Arc::new(Mutex::new("connecting...".to_string()));
    let stop = Arc::new(AtomicBool::new(false));
    let (st, sp) = (status.clone(), stop.clone());
    let url = base_wss.trim_end_matches('/').to_string() + "/api/pc/connect";
    let auth = format!("Bearer {token}");
    std::thread::spawn(move || {
        let set = |s: &str| { if let Ok(mut x) = st.lock() { *x = s.to_string(); } };
        let mut wait = 2;
        while !sp.load(Ordering::Relaxed) {
            use tungstenite::client::IntoClientRequest;
            let Ok(mut req) = url.as_str().into_client_request() else { return };
            if let Ok(v) = auth.parse() { req.headers_mut().insert("Authorization", v); }
            match tungstenite::connect(req) {
                Ok((mut ws, _)) => {
                    set("on: your phone can use this computer's Claude");
                    wait = 2;
                    if let tungstenite::stream::MaybeTlsStream::Rustls(s) = ws.get_mut() {
                        let _ = s.get_mut().set_read_timeout(Some(Duration::from_millis(200)));
                    } else if let tungstenite::stream::MaybeTlsStream::Plain(s) = ws.get_mut() {
                        let _ = s.set_read_timeout(Some(Duration::from_millis(200)));
                    }
                    let mut ping = Instant::now();
                    loop {
                        if sp.load(Ordering::Relaxed) { let _ = ws.close(None); return; }
                        if ping.elapsed() > Duration::from_secs(25) {
                            ping = Instant::now();
                            if ws.send(tungstenite::Message::Text(r#"{"t":"ping"}"#.into())).is_err() { break; }
                        }
                        match ws.read() {
                            Ok(tungstenite::Message::Text(t)) => {
                                let Ok(m) = serde_json::from_str::<Value>(&t) else { continue };
                                if m["t"] != "ask" { continue; }
                                set("on: answering your phone...");
                                let history: Vec<crate::ai::Msg> = m["history"].as_array().cloned().unwrap_or_default().iter()
                                    .map(|h| crate::ai::Msg { user: h["user"].as_bool().unwrap_or(true), text: h["text"].as_str().unwrap_or("").to_string() })
                                    .collect();
                                let answer = crate::ai::ask_claude_here(m["name"].as_str().unwrap_or("Claude"), m["cloud"].as_str().unwrap_or(""),
                                    m["system"].as_str().unwrap_or(""), &history);
                                let reply = match answer {
                                    Some((text, via)) => json!({"t": "answer", "id": m["id"], "text": text, "via": format!("{via}, on your computer")}),
                                    None => json!({"t": "answer", "id": m["id"], "error": "Claude on your computer didn't answer"}),
                                };
                                if ws.send(tungstenite::Message::Text(reply.to_string())).is_err() { break; }
                                set("on: your phone can use this computer's Claude");
                            }
                            Ok(tungstenite::Message::Close(_)) => break,
                            Ok(_) => {}
                            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                            Err(_) => break,
                        }
                    }
                    set("reconnecting...");
                }
                Err(tungstenite::Error::Http(r)) if r.status() == 401 => { set("off: your login expired (log in again)"); return; }
                Err(_) => set("can't reach the website, trying again..."),
            }
            for _ in 0..wait * 10 {
                if sp.load(Ordering::Relaxed) { return; }
                std::thread::sleep(Duration::from_millis(100));
            }
            wait = (wait * 2).min(60);
        }
    });
    PcLink { status, stop }
}
