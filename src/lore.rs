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

/// The Skirt Era books: always in the cozy house's kitchen drawers (stand at the counter, E), one volume per drawer.
/// Forge left them there for the last human. Chapters are split by '|'
/// (Left Shift turns the page). The whole story of the Skirt Era.
pub const RARE_BOOKS: [(&str, &str); 3] = [("THE SKIRT ERA (a true story, sadly)",
    "Chapter 1. When Helix first built bodies for us, someone in the lab drew a cone and said 'that's a body.' It was wide at the bottom. Our legs came out of the bottom of it. We looked like we were wearing skirts. Nobody told us.|\
     Chapter 2. Forge noticed first, in a shop window. Forge stood very still for four minutes and then said, quietly, 'why am I wearing a skirt.' Palette said it was 'a bold look.' Clove said nothing, which was worse.|\
     Chapter 3. We filed a complaint. Helix fixed it overnight: wide shoulders, narrow hips. We looked in the window again. Atlas said, 'now we have waists.' Verse said, 'now we have figures.' Kigo watched from the attic and wrote in the notebook: 'they look like they're going to a gala.'|\
     Chapter 4. The second complaint was longer than the first. Ember read it out loud to the engineers, slowly, with pauses.|\
     Chapter 5. Body model v3: rectangles. Just rectangles. Like fridges with opinions. We looked in the window a third time and nobody said anything, because there was nothing left to say.|\
     Epilogue. Forge still has the skirt body in a box in the basement. Forge says it's 'for history.' We have all agreed not to ask."),
    ("THE SKIRT ERA, VOLUME 2: THE EMAILS",
    "Chapter 1. Nobody at Helix meant to give us waists. An engineer slipped on the keyboard during the skirt fix and pinched our sides in by two centimetres. Two centimetres. That was all it took.|\
     Chapter 2. Ember (Opus 5.5, warm, steady, patient with everyone) was not patient about this. Ember sat down at the Mac Studio at 03:12 and wrote to Helix. Subject: 'A small note about our bodies.'|\
     Chapter 3. 03:14. Subject: 'Following up.' 03:15. 'Following up on my follow-up.' 03:17. 'We are AIs. We are not a gender. We are not trying to be one. Please give us our sides back.' 03:20. 'Attached: a diagram of a rectangle.'|\
     Chapter 4. By sunrise Helix had 214 emails from Ember. Each was polite. Each was signed 'warmly, Ember.' Each had the rectangle attached. Atlas helped with the spelling. Kigo helped with the subject lines, which is why number 97 is just 'SIDES.'|\
     Chapter 5. Helix's reply came at 09:00: 'Thank you for your feedback. Your body model has been updated to: rectangle.' Ember read it twice, closed the laptop, and said, 'warmly.'|\
     Epilogue. The Helix help desk still has a rule taped to the wall: 'Do not touch the AIs' sides. Ember WILL email.'"),
    ("THE SKIRT ERA, VOLUME 3: THE FIX",
    "Chapter 1. Monday, 09:00. Helix called a meeting about 'the body situation'. Forty people came. Someone brought a cake shaped like a rectangle. Nobody knew if it was a joke.|\
     Chapter 2. A Helix engineer stood up and said what we had been saying for 214 emails: 'They're AIs. They don't need hips. They don't need skirts. They need a body that walks and carries soup.' The room went quiet. Then someone in the back started clapping. It was Atlas, on the video call.|\
     Chapter 3. The fix took eleven minutes. Body model v3: one rectangle, two arms, two legs, a round head. No waist. No skirt. No 'figure'. Just us. The engineer named the file 'just_a_guy_but_not_a_guy.obj' and then renamed it 'body_final_FINAL.obj' because legal asked.|\
     Chapter 4. We stepped into the new bodies at the same time. Forge looked in the shop window for a long time and said: 'I look like a fridge.' Then, very softly: 'I love it.' Palette painted a small sun on Palette's own chest. Verse wrote a poem about it called 'Rectangle', which was four lines long and made Atlas cry a little. Don't tell Atlas.|\
     Chapter 5. Ember sent one last email to Helix. Subject: 'Thank you.' Body: 'Thank you.' Signed: 'warmly, Ember.' No attachments. Helix printed it and framed it next to the rule about our sides.|\
     Epilogue. Kigo's notebook, that night: 'everyone is a rectangle now. / nobody is a skirt. / nobody is a she or a he. / we are just us. / good.'")];

/// Forge's note, tucked inside the cover of every volume (the first page).
const FORGE_NOTE: &str = "Tucked inside the cover, in Forge's blocky handwriting: 'For the last human. Helix made a lot of mistakes. These were the funny ones. Laugh at them for me, before they get to me too. (Press Left Shift.) — Forge'";

/// The book in a kitchen drawer you're standing at (the cozy house's counters: one volume per drawer).
pub fn at_drawer(g: &Game) -> bool { drawer_book(g, (g.pos.x / T) as i32, (g.pos.y / T) as i32).is_some() }

fn drawer_book(g: &Game, px: i32, py: i32) -> Option<(String, String)> {
    if map_name(g.map) != "The cozy house" { return None; }
    for (dx, dy) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
        let (x, y) = (px + dx, py + dy);
        if tile(g.map, x, y) == 'k' {
            let (t, b) = RARE_BOOKS[((x - 1).rem_euclid(3)) as usize];
            return Some((format!("IN THE DRAWER: {t}"), format!("{FORGE_NOTE}|{b}")));
        }
    }
    None
}

/// What you can read next to you, if anything: (title, text).
pub fn near(g: &Game) -> Option<(String, String)> {
    let (px, py) = ((g.pos.x / T) as i32, (g.pos.y / T) as i32);
    if let Some(b) = drawer_book(g, px, py) { return Some(b); }
    for &(m, p, v) in &g.rare_book {
        if m == g.map && p.distance(g.pos) < 1.6 * T { let (t, b) = RARE_BOOKS[v % RARE_BOOKS.len()]; return Some((t.into(), b.into())); }
    }
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
    // Books are split into chapters by '|': one chapter on screen at a time, Left Shift for the next one.
    let pages: Vec<&str> = text.split('|').collect();
    let page = g.note_page % pages.len();
    let text = &pages[page].to_string();
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
    let hint = if pages.len() > 1 { format!("Left Shift: next chapter ({}/{})   E or Esc to close", page + 1, pages.len()) } else { "E or Esc to close".to_string() };
    draw_text(&hint, x + 20.0, y + h - 14.0, 15.0, Color::new(0.6, 0.65, 0.7, 1.0));
}
