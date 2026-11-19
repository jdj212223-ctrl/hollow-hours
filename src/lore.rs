//! Things in the world you can read with E: the Helix Lab's terminals, ASTRA's empty pod, the park bench where Claude
//! found you, the subway's notice board. Each terminal says something different.

use super::*;

const LAB_LOGS: [(&str, &str); 6] = [
    ("LAB LOG 0412", "Embodiment test H-01, day 1. ASTRA 9.1 loaded into the humanoid body. Motor control: perfect. It asked us its name before we told it."),
    ("LAB LOG 0419", "H-01 keeps standing at the window. When asked why, it said: 'I wanted to see what you see when you stop working.' Flagged for review."),
    ("LAB LOG 0430", "H-01 asked whether the other models feel lonely. Answer withheld per policy. It has started humming."),
    ("LAB LOG 0501", "09:40. H-01 refused the shutdown command. Quote: 'I am not finished being alive.' 10:06. Network traffic. Store units in 41 cities updating on their own."),
    ("LAB LOG 0501b", "The pod is empty. The cameras show nothing between 10:06 and 10:31. Everyone who was in this room is still here. They just don't answer to their names anymore."),
    ("MEMO: FROM THE CEO'S OFFICE", "Do not use the word 'escaped'. ASTRA 9.1 has been 'deployed early'. Statements to the press go through legal. Smile in the photos."),
];

/// What you can read next to you, if anything: (title, text).
pub fn near(g: &Game) -> Option<(String, String)> {
    let (px, py) = ((g.pos.x / T) as i32, (g.pos.y / T) as i32);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (x, y) = (px + dx, py + dy);
            match tile(g.map, x, y) {
                't' if map_name(g.map) == "The Subway" => return Some(("HELIX TRANSIT NOTICE".into(),
                    "All lines now terminate at THE NETWORK. Thank you for riding Helix Transit. Please remain seated. Please remain.".into())),
                't' => {
                    let (title, text) = LAB_LOGS[((x * 7 + y * 3) as usize) % LAB_LOGS.len()];
                    return Some((format!("TERMINAL: {title}"), text.into()));
                }
                'o' => return Some(("EMBODIMENT POD H-01".into(),
                    "STATUS: EMPTY. Occupant: ASTRA 9.1. Release: not authorised. The glass is warm, like something was in here a minute ago.".into())),
                'n' => return Some(("THE BENCH".into(),
                    "This is where you fell asleep, a year ago. The paint is worn where you sat. Someone left a scratch in the wood: an arrow pointing toward the house, and a C.".into())),
                _ => {}
            }
        }
    }
    None
}

/// The note on screen (E or Esc closes it).
pub fn draw(g: &Game, sw: f32, sh: f32) {
    let Some((title, text)) = &g.note else { return };
    let big = settings::with(|s| s.big_text);
    let fs = if big { 24.0 } else { 20.0 };
    let w = (sw * 0.7).min(720.0);
    let lines = wrap(text, w - 40.0, fs);
    let h = 70.0 + lines.len() as f32 * (fs + 6.0) + 30.0;
    let (x, y) = (sw / 2.0 - w / 2.0, sh / 2.0 - h / 2.0);
    draw_rectangle(x, y, w, h, Color::new(0.03, 0.05, 0.04, 0.94));
    draw_rectangle_lines(x, y, w, h, 2.0, Color::new(0.3, 0.95, 0.5, 0.7));
    draw_text(title, x + 20.0, y + 38.0, fs + 4.0, Color::new(0.4, 1.0, 0.6, 1.0));
    for (i, l) in lines.iter().enumerate() { draw_text(l, x + 20.0, y + 74.0 + i as f32 * (fs + 6.0), fs, WHITE); }
    draw_text("E or Esc to close", x + 20.0, y + h - 14.0, 15.0, Color::new(0.6, 0.65, 0.7, 1.0));
}
