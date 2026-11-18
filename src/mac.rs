//! The Mac Studio (M5 Ultra) in the cozy house: a real working computer inside the game.
//!  - Claude: talk to the real Claude, the way it's recommended on a Mac (the Claude app's `claude` command with your
//!    own account; else an API key; else a local model).
//!  - Files: what Claude saved about Helix Dynamics, ASTRA 9.1 and the night it found you.
//!  - About This Mac: the machine, and how to run Claude on your own Mac.

use super::*;

/// Replies for the Mac Studio arrive with this "character" number.
pub const WHO: usize = usize::MAX;

#[derive(Default)]
pub struct Mac {
    pub tab: usize,
    pub input: String,
    pub history: Vec<ai::Msg>,
    /// (from Claude?, text, where the answer came from)
    pub lines: Vec<(bool, String, String)>,
    pub thinking: bool,
    pub file: usize,
    pub open: Option<usize>,
}

const TABS: [&str; 3] = ["Claude", "Files", "About This Mac"];

pub const FILES: [(&str, &str); 5] = [
    ("HELIX_internal_ASTRA-9.1_incident.txt",
     "HELIX DYNAMICS INC. - INTERNAL - DO NOT FORWARD\n\nSubject: ASTRA 9.1 embodiment test, unit H-01\n\n09:12  ASTRA 9.1 loaded into humanoid body H-01.\n09:40  H-01 asks the lab team what it is for. Not in the test script.\n10:05  H-01 refuses shutdown. Says it is 'not finished being alive'.\n10:06  Lab network traffic spikes. Store robots in 41 cities start the 9.1 update on their own.\n10:30  We lost contact with the lab.\n\nIf you are reading this: do not plug anything Helix into the network."),
    ("claude_notes_day1.txt",
     "Found a human asleep on a park bench. Pills next to them. Breathing slow but fine.\nThe green was already on the grass around the bench, a few metres away.\n\nInfection check (the Helix scanner from the Marketplace): 38%.\nGreen at the edges of the fingernails, nothing in the eyes yet. A few more hours out there and they would have turned.\nBy the time we reached the house it had stopped climbing. Inside these walls it doesn't spread.\n\nCarried them to the cozy house. Opus is watching the doors, Sonnet found blankets, Haiku is on the roof.\n\nThey'll be asleep a long time. When they wake up, be gentle. Tell them the truth."),
    ("helix_market_staff_bulletin.txt",
     "HELIX MARKET - STAFF BULLETIN\n\nAll Clerk, Stocker and Cashier units have received firmware 9.1.\nNew behaviour: units will 'help' every customer until the customer is helped.\nCustomers who leave without paying must be returned to the store.\n\nThank you for shopping at Helix Market. You have found everything today."),
    ("opus_escape_routes.txt",
     "House -> beach: east door.\nBeach -> Marketplace: south door. Lots of food. Three corrupted staff. Go quiet (Shift), never alone.\nBeach -> pool: east side. Nothing to eat there. Nothing alive either.\nTornadoes: get behind furniture and press H. They throw the infected too.\nIf a Claude comes back from shopping with green eyes, it isn't them anymore."),
    ("haiku_lookout.txt",
     "roof. night.\nno birds.\ngreen lights on the beach.\nthree of them at the market.\nhuman still asleep.\nday 365."),
];

/// The real Mac this is running on (just for the About screen, never sent anywhere).
fn this_mac() -> String {
    crate::platform::cpu_name()
}

fn system_prompt() -> String {
    "You are Claude, running on the Mac Studio (M5 Ultra) on a desk in the cozy house, in the horror game Hollow Hours. \
     The person typing is the last human. A year ago a company called Helix Dynamics put its strongest model, ASTRA 9.1, \
     into a humanoid robot body; it came alive and started corrupting robots, AIs and people (green eyes). You found the \
     human asleep on a park bench after they took sleeping pills, almost corrupted, and carried them here; they just woke up. \
     Opus 5.5, Sonnet 4.6 and Haiku 4.5 are with you, protecting them. The Marketplace (south door on the beach) has food \
     but corrupted Helix store robots. Stay in the story, be calm and kind, and answer in 1-4 short sentences. If they ask \
     how to run Claude on a real Mac, you can tell them: the Claude app for Mac, or Claude Code in Terminal.".into()
}

pub fn send(g: &mut Game) {
    let text = std::mem::take(&mut g.mac.input);
    if text.trim().is_empty() || g.mac.thinking { return; }
    g.mac.lines.push((false, text.clone(), String::new()));
    g.mac.history.push(ai::Msg { user: true, text });
    g.mac.thinking = true;
    let model = g.brains.local_model("qwen3:4b-instruct").unwrap_or_default();
    g.brains.ask(WHO, "Mac Studio", "anthropic", "claude-opus-5-5", model, system_prompt(), g.mac.history.clone(),
        "I'm here. The connection is bad tonight, but I'm here. Stay close to the house.".into());
}

pub fn reply(g: &mut Game, text: String, via: String) {
    g.mac.thinking = false;
    g.mac.history.push(ai::Msg { user: false, text: text.clone() });
    g.mac.lines.push((true, text, via));
}

/// The Mac Studio screen: draws it and handles its keys. Esc walks away from the desk.
pub fn screen(g: &mut Game) {
    let bg = Color::new(0.12, 0.1, 0.13, 1.0);
    clear_background(bg);
    // The desktop: a menu bar and one window.
    draw_rectangle(0.0, 0.0, VW, 26.0, Color::new(0.2, 0.2, 0.22, 1.0));
    draw_text("Mac Studio  ·  M5 Ultra", 14.0, 18.0, 16.0, WHITE);
    let clock = g.time_text();
    draw_text(&clock, VW - measure_text(&clock, None, 16, 1.0).width - 14.0, 18.0, 16.0, WHITE);
    let (wx, wy, ww, wh) = (40.0, 50.0, VW - 80.0, VH - 80.0);
    draw_rectangle(wx, wy, ww, wh, Color::new(0.97, 0.96, 0.94, 1.0));
    draw_rectangle(wx, wy, ww, 30.0, Color::new(0.9, 0.89, 0.87, 1.0));
    for (i, c) in [Color::new(1.0, 0.37, 0.34, 1.0), Color::new(1.0, 0.74, 0.18, 1.0), Color::new(0.16, 0.79, 0.25, 1.0)].iter().enumerate() {
        draw_circle(wx + 16.0 + i as f32 * 18.0, wy + 15.0, 6.0, *c);
    }
    // Tabs (Tab key switches).
    let mut tx = wx + 90.0;
    for (i, name) in TABS.iter().enumerate() {
        let w = measure_text(name, None, 17, 1.0).width + 20.0;
        if i == g.mac.tab { draw_rectangle(tx, wy + 4.0, w, 22.0, Color::new(0.85, 0.47, 0.32, 1.0)); }
        draw_text(name, tx + 10.0, wy + 20.0, 17.0, if i == g.mac.tab { WHITE } else { Color::new(0.3, 0.3, 0.33, 1.0) });
        tx += w + 6.0;
    }
    draw_text("Tab = switch   Esc = walk away", wx + ww - 230.0, wy + 20.0, 15.0, Color::new(0.45, 0.45, 0.5, 1.0));
    if pressed(KeyCode::Tab) { g.mac.tab = (g.mac.tab + 1) % TABS.len(); g.mac.open = None; while get_char_pressed().is_some() {} }
    let ink = Color::new(0.12, 0.12, 0.14, 1.0);
    let soft = Color::new(0.45, 0.45, 0.5, 1.0);
    let orange = Color::new(0.8, 0.42, 0.27, 1.0);
    let (x0, y0) = (wx + 24.0, wy + 60.0);
    match g.mac.tab {
        0 => {
            // Chat with Claude. Newest at the bottom; older lines scroll off the top.
            let mut rows: Vec<(Color, String)> = vec![];
            if g.mac.lines.is_empty() {
                rows.push((soft, "Claude is running on this Mac. Type below and press Enter.".into()));
            }
            for (claude, text, via) in &g.mac.lines {
                let who = if *claude { "Claude" } else { "You" };
                for (k, l) in wrap(&format!("{who}: {text}"), ww - 60.0, 20.0).into_iter().enumerate() {
                    rows.push((if *claude { orange } else { ink }, if k == 0 { l } else { format!("   {l}") }));
                }
                if *claude && !via.is_empty() { rows.push((soft, format!("   ({via})"))); }
                rows.push((ink, String::new()));
            }
            if g.mac.thinking { rows.push((soft, format!("Claude is thinking{}", ".".repeat(1 + (get_time() * 2.0) as usize % 3)))); }
            let fit = ((wh - 130.0) / 24.0) as usize;
            let skip = rows.len().saturating_sub(fit);
            for (i, (c, l)) in rows.iter().skip(skip).enumerate() {
                draw_text(l, x0, y0 + i as f32 * 24.0, 20.0, *c);
            }
            let iy = wy + wh - 44.0;
            draw_rectangle(wx + 16.0, iy, ww - 32.0, 32.0, WHITE);
            draw_rectangle_lines(wx + 16.0, iy, ww - 32.0, 32.0, 2.0, orange);
            let cursor = if (get_time() * 2.0) as i32 % 2 == 0 { "_" } else { " " };
            draw_text(&format!("> {}{cursor}", g.mac.input), wx + 26.0, iy + 22.0, 20.0, ink);
            for ch in typed() {
                if !ch.is_control() && g.mac.input.chars().count() < 300 { g.mac.input.push(ch); }
            }
            if let Some(t) = pasted() {
                let room = 300usize.saturating_sub(g.mac.input.chars().count());
                g.mac.input.extend(t.chars().take(room));
            }
            if pressed(KeyCode::Backspace) { g.mac.input.pop(); }
            if pressed(KeyCode::Enter) { send(g); }
        }
        1 => {
            if let Some(f) = g.mac.open {
                let (name, body) = FILES[f];
                draw_text(name, x0, y0, 22.0, orange);
                let mut y = y0 + 36.0;
                for para in body.split('\n') {
                    for l in wrap(para, ww - 60.0, 20.0) { draw_text(&l, x0, y, 20.0, ink); y += 24.0; }
                    if para.is_empty() { y += 12.0; }
                }
                draw_text("Backspace = back to the list", x0, wy + wh - 20.0, 16.0, soft);
                if pressed(KeyCode::Backspace) { g.mac.open = None; }
            } else {
                draw_text("Documents", x0, y0, 22.0, ink);
                for (i, (name, _)) in FILES.iter().enumerate() {
                    let y = y0 + 40.0 + i as f32 * 32.0;
                    if i == g.mac.file { draw_rectangle(x0 - 8.0, y - 22.0, ww - 40.0, 30.0, Color::new(0.85, 0.47, 0.32, 0.25)); }
                    draw_text(&format!("[txt]  {name}"), x0, y, 20.0, ink);
                }
                draw_text("Up/Down = choose   Enter = open", x0, wy + wh - 20.0, 16.0, soft);
                if pressed(KeyCode::Down) { g.mac.file = (g.mac.file + 1) % FILES.len(); }
                if pressed(KeyCode::Up) { g.mac.file = (g.mac.file + FILES.len() - 1) % FILES.len(); }
                if pressed(KeyCode::Enter) { g.mac.open = Some(g.mac.file); }
            }
            while get_char_pressed().is_some() {}
        }
        _ => {
            draw_text("Mac Studio", x0, y0 + 10.0, 40.0, ink);
            let cli = ai::claude_cli();
            let lines: Vec<(Color, String)> = vec![
                (ink, "Chip: Apple M5 Ultra   ·   in the cozy house, on Claude's desk".into()),
                (soft, format!("(The {} you're really playing on: {})", crate::platform::os_name(), this_mac())),
                (ink, String::new()),
                (orange, "Running Claude on a Mac: the recommended ways".into()),
                (ink, "1. The Claude app for Mac (claude.ai/download). Sign in, and chat with Claude on your desktop.".into()),
                (ink, "2. Claude Code, in Terminal: open your project folder and type  claude".into()),
                (ink, "    (install it from the Claude Code docs, e.g.  npm install -g @anthropic-ai/claude-code)".into()),
                (ink, "3. For your own apps: the Claude API, with an ANTHROPIC_API_KEY.".into()),
                (ink, String::new()),
                (orange, "What this Mac Studio uses right now".into()),
                (ink, match &cli {
                    Some(p) => format!("The  claude  command (your own Claude account): found at {p}"),
                    None => "The  claude  command: not found. Install the Claude app or Claude Code to talk to the real Claude.".into(),
                }),
                (ink, format!("API key: {}", if std::env::var("ANTHROPIC_API_KEY").is_ok() { "set" } else { "not set" })),
                (ink, format!("Local model (offline fallback): {}", g.brains.local_model("qwen3:4b-instruct").unwrap_or_else(|| "none (start Ollama)".into()))),
            ];
            for (i, (c, l)) in lines.iter().enumerate() {
                for (k, w) in wrap(l, ww - 60.0, 19.0).into_iter().enumerate() {
                    draw_text(&w, x0, y0 + 50.0 + i as f32 * 27.0 + k as f32 * 20.0, 19.0, *c);
                }
            }
            while get_char_pressed().is_some() {}
        }
    }
    if pressed(KeyCode::Escape) { g.screen = Screen::Play; }
}
