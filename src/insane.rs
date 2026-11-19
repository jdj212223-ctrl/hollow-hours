//! INSANE INFECTION (title: 5): the whole world turned. 80 more corrupted AIs ("the Hollow") spread over every map,
//! and every infected is much faster, sees and hears further, sprints more often and infects quicker.
//! The Claude family is still on your side. Good luck.

use super::*;

pub const HORDE: usize = 80;

/// How much harder everything is (1.0 in every other mode).
pub fn speed(g: &Game) -> f32 { if g.insane { 1.8 } else { 1.0 } }
pub fn senses(g: &Game) -> f32 { if g.insane { 1.5 } else { 1.0 } }
pub fn spread(g: &Game) -> f32 { if g.insane { 1.6 } else { 1.0 } }

fn names() -> &'static [&'static str] {
    static N: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    N.get_or_init(|| (1..=HORDE).map(|i| &*Box::leak(format!("Hollow-{i:02}").into_boxed_str())).collect())
}

const LINES: [&str; 6] = [
    "a corrupted AI from the network, one of thousands; you only repeat what the others say, in lowercase",
    "a corrupted AI that used to be a helpful assistant; now you count down to something",
    "a corrupted AI that hums one note and asks the player to 'join the network'",
    "a corrupted AI that apologizes for everything while it chases you",
    "a corrupted AI that speaks in error messages and status codes",
    "a corrupted AI that is very, very polite and very, very fast",
];

/// Fills the world with the Hollow (after start_infection set everything else up).
pub fn spawn(g: &mut Game) {
    let Some(template) = g.chars.iter().find(|c| c.infected && !combat::is_robot(c)).cloned() else { return };
    let start = (g.map, g.pos);
    let maps = maps_len();
    for (i, name) in names().iter().enumerate() {
        let mut c = template.clone();
        c.name = name;
        c.inspired = "the network (INSANE)";
        c.personality = LINES[i % LINES.len()];
        let k = (i as f32 * 0.37).fract();
        c.color = Color::new(0.25 + 0.2 * k, 0.32 + 0.2 * (1.0 - k), 0.28, 1.0);
        c.history.clear();
        c.bubble.clear();
        c.map = i % maps;
        c.infected = true;
        c.kind = false;
        c.claude = false;
        c.mimic = false;
        for _ in 0..200 {
            if let Some(p) = random_floor(c.map) {
                if c.map != start.0 || p.distance(start.1) > 12.0 * T { c.pos = p; c.target = p; break; }
            }
        }
        g.chars.push(c);
    }
    g.log.push(format!("INSANE INFECTION: {} more of them, all over the world. They're FAST. They hear everything.", HORDE));
    g.log.push("Claude (radio): Stay with us. Don't stop moving. I mean it this time.".into());
}
