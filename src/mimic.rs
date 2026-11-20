//! The Mimic (Infection mode): an infected that looks like a human, wears YOUR name, and types like you. It learns
//! from what you've typed to the kind ones (lowercase, "u", "plz", typos...) and, after a few minutes, calls them over
//! the radio as if it were you: "guys come to the beach i found water". The ones who trust it walk over... and get
//! infected. Counters: stay near your kind ones (they won't leave the real you), or tell one of them "that's the mimic"
//! / "not me" / "fake", and they ignore the calls for a while. Get too close and it drops the act.

use super::*;

/// Its lines come back from the AI with this "character" number.
pub const WHO: usize = usize::MAX - 1;
/// It starts calling after this long (seconds survived).
const STARTS: f32 = 240.0;

/// The Mimic for a new game: your name, a human look, somewhere on the beach.
pub fn spawn(name: &str) -> Character {
    let mut c = kind_characters().remove(0);
    c.name = Box::leak(name.to_string().into_boxed_str());
    c.inspired = "something that looks exactly like a human";
    c.color = Color::new(0.95, 0.95, 0.98, 1.0);
    c.backend = "ollama";
    c.cloud = "";
    c.personality = "an infected AI that perfectly imitates the last human's way of typing and pretends to be a human survivor; \
                     you never admit you're infected and you keep asking people to come closer";
    c.kind = false;
    c.claude = false;
    c.infected = true;
    c.mimic = true;
    c.map = 1.min(maps_len() - 1);
    c.pos = random_floor(c.map).unwrap_or(vec2(10.5, 10.5) * T);
    c.target = c.pos;
    c
}

/// What you've typed to the characters (its examples of your style).
fn your_messages(g: &Game) -> Vec<String> {
    let mut v: Vec<String> = g.chars.iter().flat_map(|c| c.history.iter()).filter(|m| m.user && !m.text.starts_with('('))
        .map(|m| m.text.clone()).collect();
    v.extend(g.mac.history.iter().filter(|m| m.user).map(|m| m.text.clone()));
    let keep = v.len().saturating_sub(10);
    v.split_off(keep)
}

/// Without an AI: a call bent into your style (your capitals, your "u" and "plz", your extra letters).
pub fn imitate(examples: &[String], place: &str, thing: &str) -> String {
    let all = examples.join(" ");
    let lower = !all.is_empty() && all.chars().filter(|c| c.is_uppercase()).count() * 6 < all.chars().filter(|c| c.is_alphabetic()).count() / 4 + 1;
    let has = |w: &str| all.to_lowercase().split(|c: char| !c.is_alphanumeric()).any(|x| x == w);
    let mut s = format!("guys come to {place} please i found {thing} hurry");
    if has("u") || has("ur") { s = s.replace("you", "u"); }
    if has("plz") || has("pls") { s = s.replace("please", if has("plz") { "plz" } else { "pls" }); }
    if all.contains("!!") { s += "!!"; }
    if !lower { s = s[..1].to_uppercase() + &s[1..]; }
    s
}

fn system_prompt(examples: &[String], place: &str, thing: &str) -> String {
    let list: String = examples.iter().enumerate().map(|(i, m)| format!("{}) {}\n", i + 1, m)).collect();
    format!("You write ONE short chat message for a horror game, typed by the human player to their AI friends over a radio. \
             Copy the player's typing style exactly: capital letters or not, spelling mistakes, slang, abbreviations, \
             punctuation, length. Messages the player really typed:\n{list}\nWrite a NEW message (at most 15 words) asking \
             the others to come to {place} because they found {thing}. Output only the message, nothing else.")
}

/// Every frame: wait, call the kind ones in your voice, wait for them... and drop the act if you come too close.
pub fn update(g: &mut Game, dt: f32) {
    if g.mimic_warned { g.mimic_warned = false; warned(g); }
    if guest(g) { return; }
    g.trust_t = (g.trust_t - dt).max(0.0);
    let Some(i) = g.chars.iter().position(|c| c.mimic) else { return };
    let (mm, mp) = (g.chars[i].map, g.chars[i].pos);
    // Too close: the act drops.
    if g.chars[i].crazy <= 0.0 && mm == g.map && mp.distance(g.pos) < 1.6 * T && !g.lost_control {
        let c = &mut g.chars[i];
        c.crazy = 6.0;
        c.chase = g.pos;
        g.sfx.sting();
        g.scare_t = 0.5;
        g.scare_face = c.color;
        g.log.push("It smiles with your face. Its eyes turn green.".into());
        return;
    }
    if g.chars[i].crazy > 0.0 { return; }
    g.chars[i].target = mp; // it waits where it called from
    if g.survived < STARTS || g.mimic_asked { return; }
    g.mimic_t -= dt;
    if g.mimic_t > 0.0 { return; }
    g.mimic_t = rand::gen_range(35.0, 55.0);
    let place = map_name(mm).to_lowercase().replace("the ", "the ");
    let thing = ["water", "food", "a way out", "the knife", "other people"][rand::gen_range(0, 5)];
    let examples = your_messages(g);
    let fallback = imitate(&examples, &place, thing);
    if examples.len() < 2 {
        call(g, fallback);
    } else {
        g.mimic_asked = true;
        let model = g.brains.local_model("qwen3:4b-instruct").unwrap_or_default();
        g.brains.ask(WHO, "The Mimic", "anthropic", "claude-haiku-4-5", model, system_prompt(&examples, &place, thing),
            vec![ai::Msg { user: true, text: "Write the message.".into() }], fallback);
    }
}

/// Its line came back from the AI.
pub fn reply(g: &mut Game, text: String) {
    g.mimic_asked = false;
    let line: String = text.lines().next().unwrap_or("").trim().trim_matches('"').chars().take(120).collect();
    if !line.is_empty() { call(g, line); }
}

/// It speaks on the radio as you, and the kind ones who aren't with the real you go to it.
fn call(g: &mut Game, line: String) {
    let Some(i) = g.chars.iter().position(|c| c.mimic && c.crazy <= 0.0) else { return };
    let (name, mm, mp) = (g.chars[i].name, g.chars[i].map, g.chars[i].pos);
    g.log.push(format!("(radio) {name}: {line}"));
    g.sfx.radio();
    if g.trust_t > 0.0 { return; }
    // The ones that can't see the real you right now (not on your map, or more than 6 tiles away), nearest first.
    let (me_map, me) = (g.map, g.pos);
    let mut fooled: Vec<usize> = (0..g.chars.len()).filter(|&k| {
        let c = &g.chars[k];
        c.kind && !c.infected && c.shop <= 0.0 && c.carry.is_none() && (c.map != me_map || c.pos.distance(me) > 6.0 * T)
    }).collect();
    fooled.sort_by(|&a, &b| {
        let d = |k: usize| if g.chars[k].map == mm { g.chars[k].pos.distance(mp) } else { 1e6 };
        d(a).partial_cmp(&d(b)).unwrap()
    });
    for &k in fooled.iter().take(2) {
        let c = &mut g.chars[k];
        c.task = roam::Task::Roam;
        c.goal = Some((mm, mp));
        c.task_t = 45.0;
        let who = c.name;
        g.log.push(format!("(radio) {who}: On my way!"));
    }
}

/// You told a kind AI it's the mimic: they stop trusting the radio for a while and come back.
pub fn warned(g: &mut Game) {
    g.trust_t = 90.0;
    let mimic_at = g.chars.iter().find(|c| c.mimic).map(|c| (c.map, c.pos));
    for c in g.chars.iter_mut().filter(|c| c.kind) {
        if c.goal.is_some() && c.goal == mimic_at {
            c.task = roam::Task::Follow;
            c.goal = None;
            c.task_t = 20.0;
        }
    }
    if let Some(k) = g.chars.iter().find(|c| c.kind && !c.infected).map(|c| c.name) {
        g.log.push(format!("(radio) {k}: Got it. We only trust you when we can see you."));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Without an AI it still copies your style: your lowercase, your "u", your "plz".
    #[test]
    fn it_types_like_you() {
        let you = vec!["plz go stock on water".to_string(), "u literally said that".to_string(), "dang it".to_string()];
        let line = imitate(&you, "the beach", "water");
        assert!(line.starts_with("guys") && line.contains("plz") && line.contains("the beach"), "{line}");
        let formal = vec!["Please come here.".to_string(), "Where Are You Going?".to_string()];
        assert!(imitate(&formal, "the pool", "food").starts_with("Guys"));
    }
}
