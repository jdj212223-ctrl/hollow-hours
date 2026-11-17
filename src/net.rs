//! Multiplayer through the Cloudflare Worker (server/): one WebSocket per player, one room per code.
//! A background thread does all the network work; the game just sends and reads JSON messages through channels.

use serde_json::Value;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::time::Duration;

pub const SERVER: &str = "wss://hollow-hours-mp.jdj212223.workers.dev";

pub struct Net {
    pub room: String,
    pub me: String,
    pub host: String,
    pub players: usize,
    pub connected: bool,
    tx: Sender<String>,
    rx: Receiver<Value>,
}

impl Net {
    /// Joins room `code` (4-8 letters/digits) with your login token (the server checks it and uses your account name).
    /// Connecting happens in the background.
    pub fn join(code: &str, token: &str) -> Self {
        let base = std::env::var("HOLLOW_SERVER").unwrap_or_else(|_| SERVER.into());
        Self::join_at(&base, code, &format!("Bearer {token}"))
    }

    /// Joins a room on any Hollow Hours server: `base` is its address (wss://...), `auth` the Authorization header:
    /// "Bearer <token>" for the official server, "Ticket <ticket>" for one hosted somewhere else.
    pub fn join_at(base: &str, code: &str, auth: &str) -> Self {
        let auth = auth.to_string();
        let room: String = code.chars().filter(|c| c.is_ascii_alphanumeric()).take(8).collect::<String>().to_uppercase();
        let (to_server, outbox) = channel::<String>();
        let (inbox, from_server) = channel::<Value>();
        let url = base.trim_end_matches('/').to_string() + "/room/" + &room;
        std::thread::spawn(move || {
            // The token goes in a header, not the address (addresses end up in logs).
            use tungstenite::client::IntoClientRequest;
            let Ok(mut req) = url.as_str().into_client_request() else { return };
            if let Ok(v) = auth.parse() { req.headers_mut().insert("Authorization", v); }
            let (mut ws, _) = match tungstenite::connect(req) {
                Ok(c) => c,
                Err(tungstenite::Error::Http(r)) if r.status() == 401 => {
                    let _ = inbox.send(serde_json::json!({"t": "error", "msg": "your login expired: log in again (L)"}));
                    return;
                }
                Err(tungstenite::Error::Http(r)) if r.status() == 429 => {
                    let _ = inbox.send(serde_json::json!({"t": "error", "msg": "that server is full"}));
                    return;
                }
                Err(_) => {
                    let _ = inbox.send(serde_json::json!({"t": "error", "msg": "couldn't reach the multiplayer server"}));
                    return;
                }
            };
            // Short read timeouts so the same thread can also send.
            if let tungstenite::stream::MaybeTlsStream::Rustls(s) = ws.get_mut() {
                let _ = s.get_mut().set_read_timeout(Some(Duration::from_millis(20)));
            } else if let tungstenite::stream::MaybeTlsStream::Plain(s) = ws.get_mut() {
                let _ = s.set_read_timeout(Some(Duration::from_millis(20)));
            }
            loop {
                loop {
                    match outbox.try_recv() {
                        Ok(m) => if ws.send(tungstenite::Message::Text(m)).is_err() { return; },
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => { let _ = ws.close(None); return; }
                    }
                }
                match ws.read() {
                    Ok(tungstenite::Message::Text(t)) => {
                        if let Ok(v) = serde_json::from_str::<Value>(&t) { let _ = inbox.send(v); }
                    }
                    Ok(tungstenite::Message::Close(f)) => {
                        let why = f.map(|f| f.reason.to_string()).filter(|r| !r.is_empty()).unwrap_or_else(|| "disconnected".into());
                        let _ = inbox.send(serde_json::json!({"t": "error", "msg": why}));
                        return;
                    }
                    Ok(_) => {}
                    Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                    Err(_) => { let _ = inbox.send(serde_json::json!({"t": "error", "msg": "disconnected"})); return; }
                }
            }
        });
        Net { room, me: String::new(), host: String::new(), players: 1, connected: false, tx: to_server, rx: from_server }
    }

    pub fn is_host(&self) -> bool {
        self.connected && self.me == self.host
    }

    pub fn send(&self, v: Value) {
        let _ = self.tx.send(v.to_string());
    }

    /// Messages that arrived since last frame (hello/host/left are handled here and also passed on).
    pub fn poll(&mut self) -> Vec<Value> {
        let mut out = vec![];
        while let Ok(v) = self.rx.try_recv() {
            match v["t"].as_str() {
                Some("hello") => {
                    self.connected = true;
                    self.me = v["you"].as_str().unwrap_or("").into();
                    self.host = v["host"].as_str().unwrap_or("").into();
                    self.players = v["players"].as_u64().unwrap_or(1) as usize;
                }
                Some("host") | Some("left") => {
                    self.host = v["host"].as_str().unwrap_or("").into();
                    self.players = v["players"].as_u64().unwrap_or(1) as usize;
                }
                Some("error") => self.connected = false,
                _ => {}
            }
            out.push(v);
        }
        out
    }
}
