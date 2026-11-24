//! The game's bridge for the Hollow Hours MCP (bin/hh-mcp.rs): a tiny HTTP server on 127.0.0.1 only, locked with a
//! random token kept in ~/Library/Application Support/Hollow Hours/bridge.json (readable only by you). Each request
//! ({"cmd": ..., "args": {...}}) is handed to the game's main loop, which answers with JSON.

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

pub const PORT: u16 = 25610;
/// The port (HOLLOW_BRIDGE_PORT changes it, for running a second copy of the game to test with).
pub fn port() -> u16 { std::env::var("HOLLOW_BRIDGE_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(PORT) }

pub struct Request {
    pub cmd: String,
    pub args: Value,
    pub reply: Sender<Value>,
}

pub fn bridge_file() -> std::path::PathBuf {
    crate::platform::data_dir().join("bridge.json")
}

/// Starts the bridge (if the port is free). Returns where requests arrive.
pub fn start(token: String) -> Option<Receiver<Request>> {
    let listener = TcpListener::bind(("127.0.0.1", port())).ok()?;
    let file = bridge_file();
    std::fs::write(&file, json!({"port": port(), "token": token}).to_string()).ok()?;
    crate::platform::private(&file);
    let (tx, rx) = channel::<Request>();
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(mut conn) = conn else { continue };
            let tx = tx.clone();
            let token = token.clone();
            std::thread::spawn(move || {
                let _ = conn.set_read_timeout(Some(Duration::from_secs(5)));
                let mut buf = vec![0u8; 65536];
                let mut n = 0;
                // Read the headers, then the body (Content-Length).
                loop {
                    let Ok(k) = conn.read(&mut buf[n..]) else { return };
                    if k == 0 { break; }
                    n += k;
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    if let Some(h) = text.find("\r\n\r\n") {
                        let len = text[..h].lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0))).unwrap_or(0);
                        if n >= h + 4 + len || n == buf.len() { break; }
                    }
                }
                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
                let authed = head.lines().any(|l| l.to_ascii_lowercase().starts_with("authorization:") && l.trim().ends_with(&format!("Bearer {token}")));
                let respond = |conn: &mut std::net::TcpStream, code: &str, v: Value| {
                    let b = v.to_string();
                    let _ = conn.write_all(format!("HTTP/1.1 {code}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{b}", b.len()).as_bytes());
                };
                if !authed {
                    return respond(&mut conn, "401 Unauthorized", json!({"error": "wrong token"}));
                }
                let Ok(v) = serde_json::from_str::<Value>(body) else { return respond(&mut conn, "400 Bad Request", json!({"error": "send JSON {cmd, args}"})) };
                let (rtx, rrx) = channel();
                let _ = tx.send(Request { cmd: v["cmd"].as_str().unwrap_or("").to_string(), args: v["args"].clone(), reply: rtx });
                let answer = rrx.recv_timeout(Duration::from_secs(20)).unwrap_or(json!({"error": "the game didn't answer (is it frozen or closed?)"}));
                respond(&mut conn, "200 OK", answer);
            });
        }
    });
    Some(rx)
}
