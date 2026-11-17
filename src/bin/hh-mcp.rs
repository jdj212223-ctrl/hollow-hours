//! hh-mcp: the Hollow Hours MCP server (a fork of the idea behind the Minecraft MCP, for Hollow Hours).
//! Lets an AI agent like Claude Code play Hollow Hours through the game's bridge (127.0.0.1, token in
//! ~/Library/Application Support/Hollow Hours/bridge.json, readable only by you).
//!   claude mcp add --scope user hollow-hours -- /path/to/hh-mcp
//! The game shows "Claude is playing" while it's being used, your keyboard always wins, and P (panic) stops it.

use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};

fn bridge() -> Result<(u16, String), String> {
    let path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Library/Application Support/Hollow Hours/bridge.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path).map_err(|_| "Hollow Hours isn't running (start the game first)".to_string())?)
        .map_err(|e| e.to_string())?;
    Ok((v["port"].as_u64().unwrap_or(25610) as u16, v["token"].as_str().unwrap_or("").to_string()))
}

fn game(cmd: &str, args: Value) -> Result<Value, String> {
    let (port, token) = bridge()?;
    let r = ureq::post(&format!("http://127.0.0.1:{port}/")).set("Authorization", &format!("Bearer {token}"))
        .timeout(std::time::Duration::from_secs(25)).send_json(json!({"cmd": cmd, "args": args}));
    match r {
        Ok(r) => r.into_json().map_err(|e| e.to_string()),
        Err(_) => Err("Hollow Hours isn't answering (is the game open?)".into()),
    }
}

fn b64(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            out.push(if i <= c.len() { T[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
        }
    }
    out
}

fn tools() -> Value {
    let o = |props: Value, req: &[&str]| json!({"type": "object", "properties": props, "required": req});
    json!([
        {"name": "hh_state", "description": "Everything about the game right now: screen, mode, map, you (tile position, infection/food/water/fear %, hidden, can hide here, sneaking, lost control), every character on this map (position, distance, infected, kind, chasing you, what they're saying), the nearest food and water, doors (where they lead and their tiles), tornado, recent events, online room. The map is 30 x 18 tiles; x grows to the right (east), y downward (south). Call this often.",
         "inputSchema": o(json!({}), &[])},
        {"name": "hh_start", "description": "Start a game from the title, game-over or safe screen: mode infection (default), normal or brave.",
         "inputSchema": o(json!({"mode": {"type": "string", "enum": ["infection", "normal", "brave"]}}), &[])},
        {"name": "hh_move", "description": "Walk in a direction for up to 5 seconds. sneak: true = slow and silent (infected hear normal footsteps).",
         "inputSchema": o(json!({"direction": {"type": "string", "enum": ["north", "south", "east", "west", "north-east", "north-west", "south-east", "south-west"]}, "seconds": {"type": "number"}, "sneak": {"type": "boolean"}}), &["direction"])},
        {"name": "hh_go_to", "description": "Walk straight to a tile spot (x 0-30, y 0-18) on this map. Walls stop you: go through gaps and doors (doors are in hh_state). Walking onto a door tile takes you to the next map.",
         "inputSchema": o(json!({"x": {"type": "number"}, "y": {"type": "number"}, "sneak": {"type": "boolean"}}), &["x", "y"])},
        {"name": "hh_stop", "description": "Stop walking.", "inputSchema": o(json!({}), &[])},
        {"name": "hh_hide", "description": "Infection mode: hide next to furniture (can_hide_here in hh_state), or come out if you're hidden. Hidden = you can't move; infected only find you right next to you.",
         "inputSchema": o(json!({}), &[])},
        {"name": "hh_talk", "description": "Say something to a character within 3 tiles. Their answer (a real AI) appears in hh_state under characters[].saying a few seconds later.",
         "inputSchema": o(json!({"name": {"type": "string"}, "text": {"type": "string"}}), &["name", "text"])},
        {"name": "hh_panic", "description": "The panic button: every scare stops at once and the game goes to a calm screen. Use it if things get too much.",
         "inputSchema": o(json!({}), &[])},
        {"name": "hh_back", "description": "From the game-over or safe screen, back to the title.", "inputSchema": o(json!({}), &[])},
        {"name": "hh_screenshot", "description": "A picture of the game window.", "inputSchema": o(json!({}), &[])},
    ])
}

fn call(name: &str, a: &Value) -> Result<Vec<Value>, String> {
    let text = |v: Value| Ok(vec![json!({"type": "text", "text": serde_json::to_string_pretty(&v).unwrap_or_default()})]);
    match name {
        "hh_state" => text(game("state", json!({}))?),
        "hh_start" => text(game("start", json!({"mode": a["mode"].as_str().unwrap_or("infection")}))?),
        "hh_move" => text(game("move", a.clone())?),
        "hh_go_to" => text(game("go_to", a.clone())?),
        "hh_stop" => text(game("stop", json!({}))?),
        "hh_hide" => text(game("hide", json!({}))?),
        "hh_talk" => text(game("talk", a.clone())?),
        "hh_panic" => text(game("panic", json!({}))?),
        "hh_back" => text(game("back", json!({}))?),
        "hh_screenshot" => {
            let v = game("screenshot", json!({}))?;
            let path = v["path"].as_str().ok_or("no screenshot")?;
            let mut bytes = vec![];
            std::fs::File::open(path).and_then(|mut f| f.read_to_end(&mut bytes)).map_err(|e| e.to_string())?;
            Ok(vec![json!({"type": "image", "data": b64(&bytes), "mimeType": "image/png"})])
        }
        _ => Err(format!("unknown tool {name}")),
    }
}

fn main() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines().map_while(Result::ok) {
        let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
        let Some(id) = msg.get("id").cloned() else { continue }; // notifications need no answer
        let result = match msg["method"].as_str().unwrap_or("") {
            "initialize" => json!({"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "serverInfo": {"name": "hollow-hours", "version": "1.0"}}),
            "tools/list" => json!({"tools": tools()}),
            "tools/call" => match call(msg["params"]["name"].as_str().unwrap_or(""), &msg["params"]["arguments"]) {
                Ok(content) => json!({"content": content}),
                Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
            },
            "ping" => json!({}),
            _ => {
                let _ = writeln!(out, "{}", json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "method not found"}}));
                let _ = out.flush();
                continue;
            }
        };
        let _ = writeln!(out, "{}", json!({"jsonrpc": "2.0", "id": id, "result": result}));
        let _ = out.flush();
    }
}
