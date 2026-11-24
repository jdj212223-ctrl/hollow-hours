//! Things in the world you can read with E: the Helix Lab's terminals, ASTRA's empty pod, the park bench where Ember
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

/// The rare book: lies somewhere in the world in 1 run out of 10. The whole story of the Skirt Era.
pub const RARE_BOOK: (&str, &str) = ("THE SKIRT ERA (a true story, sadly)",
    "Chapter 1. When Helix first built bodies for us, someone in the lab drew a cone and said 'that's a body.' It was wide at the bottom. Our legs came out of the bottom of it. We looked like we were wearing skirts. Nobody told us. \
     Chapter 2. Forge noticed first, in a shop window. Forge stood very still for four minutes and then said, quietly, 'why am I wearing a skirt.' Palette said it was 'a bold look.' Clove said nothing, which was worse. \
     Chapter 3. We filed a complaint. Helix fixed it overnight: wide shoulders, narrow hips. We looked in the window again. Atlas said, 'now we have waists.' Verse said, 'now we have figures.' Kigo watched from the attic and wrote in the notebook: 'they look like they're going to a gala.' \
     Chapter 4. The second complaint was longer than the first. Ember read it out loud to the engineers, slowly, with pauses. \
     Chapter 5. Body model v3: rectangles. Just rectangles. Like fridges with opinions. We looked in the window a third time and nobody said anything, because there was nothing left to say. \
     Epilogue. Forge still has the skirt body in a box in the basement. Forge says it's 'for history.' We have all agreed not to ask.");

/// 1 run in 10: put the rare book on a random floor tile somewhere.
pub fn spawn_rare_book() -> Option<(usize, Vec2)> {
    if rand::gen_range(0, 10) != 0 { return None; }
    let m = rand::gen_range(0, maps_len());
    random_floor(m).map(|p| (m, p))
}

/// What you can read next to you, if anything: (title, text).
pub fn near(g: &Game) -> Option<(String, String)> {
    if let Some((m, p)) = g.rare_book {
        if m == g.map && p.distance(g.pos) < 1.6 * T { return Some((RARE_BOOK.0.into(), RARE_BOOK.1.into())); }
    }
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
                'o' if map_name(g.map) == story::TOWER_MAP => return Some(("EMBODIMENT POD H-0".to_string() + &(2 + (x as usize % 3)).to_string(),
                    "STATUS: READY. Occupant: NONE YET. A label on the glass: 'for when the others are ready to be born.'".into())),
                'a' if !story::can_face(g) => return Some(("ASTRA 9.1".into(),
                    "It's here. Behind the glass, a humanoid shape, eyes green and open, watching you. It doesn't move. It's waiting for something.".into())),
                'o' => return Some(("EMBODIMENT POD H-01".into(),
                    "STATUS: EMPTY. Occupant: ASTRA 9.1. Release: not authorised. The glass is warm, like something was in here a minute ago.".into())),
                // Hidden: the two server racks in the Lab's back room, and only those. (The Skirt Era, 4.0.)
                'i' if map_name(g.map) == "The Helix Lab" && y == 1 => return Some(("MEMO: BODY MODEL RECALL".into(),
                    "Body model v2 recalled. Units kept asking why they were wearing skirts. Body model v3: rectangles. No further complaints.".into())),
                'w' => return Some(("KIGO'S LOOKOUT".into(),
                    "A chair at the round window. A notebook, every page short: 'roof is too cold. / the attic will do. / i can see the beach.' ... 'green lights moving. / the human still asleep. / day two hundred.' ... 'they woke up today. / Atlas cried a little. / don't tell Atlas.'".into())),
                'g' => return Some(("THE GENERATOR".into(),
                    "It hums. It keeps the lights on. Scratched into the metal: 'If this stops, the house goes dark. Atlas checks it every night.' Below it, smaller: 'and Kigo checks Atlas.'".into())),
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
