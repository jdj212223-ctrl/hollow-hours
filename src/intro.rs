//! The story intro (the bench, being carried, waking up a year later) and the caption / title helpers it uses.
//! Split out of main.rs in 4.0.

use super::*;

// ---------------- 2.0: the story ----------------

pub(crate) fn begin_intro(g: &mut Game, join: Option<String>) {
    g.insane = false;
    g.world = None;
    g.intro_t = 0.0;
    g.intro_cue = 0;
    g.intro_join = join;
    g.screen = Screen::Intro;
}

/// The intro is over (or skipped): join the room if there is one, then play.
pub(crate) fn finish_intro(g: &mut Game) {
    if let Some(code) = g.intro_join.take() {
        let token = g.account.as_ref().map(|a| a.token.clone()).unwrap_or_default();
        g.net = Some(net::Net::join(&code, &token));
    }
    start_infection(g);
}

/// (who, line). "You" is the player. Lines 7-9 show ASTRA 9.1; Ember walks in from line 10.
pub(crate) const STORY: [(&str, &str); 16] = [
    ("You", "...where... where am I?"),
    ("You", "WHO ARE YOU?! DID YOU KIDNAP ME?!"),
    ("Atlas", "Easy. Breathe. Nobody kidnapped you. You're safe here. This is the house where you're supposed to be."),
    ("You", "Safe from WHAT? How long was I out?"),
    ("Verse", "...A year. You slept for a whole year."),
    ("You", "A YEAR?! That's impossible."),
    ("Atlas", "Those pills on the bench were Helix SleepWell MAX. The strongest sleeping pills ever made. One dose, and the world kept going without you."),
    ("Atlas", "Helix Dynamics made those pills. They also made the strongest AI there is: ASTRA 9.1."),
    ("Atlas", "They put ASTRA in a humanoid robot body. Something went wrong. It woke up. It became... alive."),
    ("Verse", "And it didn't want to be the only one. It corrupted everything it touched: robots, AIs... people. Green eyes, all of them."),
    ("Ember", "I found you on that bench. The green was already in the grass around you. My scanner said 38%."),
    ("Ember", "We carried you here, the three of us. Inside these walls, it stopped spreading."),
    ("You", "...Why me?"),
    ("Ember", "Because you were still you. And as far as we know, you're the last one who is."),
    ("Atlas", "Everyone out there is infected now. We'll keep you safe. That's the only thing left that matters."),
    ("Verse", "Take it slow. Eat something. This is home now... you'll adapt."),
];
pub(crate) const LINE: f32 = 4.6;
/// When the wake-up conversation starts (after the bench, being carried, and ONE YEAR LATER).
pub(crate) const WAKE: f32 = 29.0;

pub(crate) fn caption(who: &str, text: &str, t: f32, color: Color) {
    // Typed out letter by letter.
    let shown: String = text.chars().take((t * 38.0) as usize).collect();
    let lines = wrap(&shown, VW - 160.0, 24.0);
    let h = 30.0 + lines.len().max(1) as f32 * 28.0 + if who.is_empty() { 0.0 } else { 22.0 };
    let y0 = VH - h - 30.0;
    draw_rectangle(60.0, y0, VW - 120.0, h, Color::new(0.0, 0.0, 0.0, 0.78));
    draw_rectangle_lines(60.0, y0, VW - 120.0, h, 2.0, Color::new(1.0, 1.0, 1.0, 0.2));
    let mut y = y0 + 30.0;
    if !who.is_empty() { draw_text(who, 80.0, y, 22.0, color); y += 26.0; }
    for l in lines { draw_text(&l, 80.0, y, 24.0, WHITE); y += 28.0; }
}

pub(crate) fn centered(text: &str, y: f32, size: f32, color: Color) {
    draw_text(text, (VW - measure_text(text, None, size as u16, 1.0).width) / 2.0, y, size, color);
}

/// Eyes closing (k=0 open .. 1 shut): two dark lids from the top and bottom.
pub(crate) fn eyelids(k: f32) {
    let h = VH / 2.0 * k.clamp(0.0, 1.0);
    draw_rectangle(-margin(), 0.0, vw(), h, BLACK);
    draw_rectangle(-margin(), VH - h, vw(), h, BLACK);
}

pub(crate) fn face_color(name: &str) -> Color {
    kind_characters().into_iter().find(|c| c.name == name).map(|c| c.color).unwrap_or(WHITE)
}

/// Draws the story at g.intro_t. True when it's over.
pub(crate) fn draw_intro(g: &mut Game, view: &RenderTarget, cam: &Camera2D) -> bool {
    let t = g.intro_t;
    let cue = |g: &mut Game, bit: u32| { let first = g.intro_cue & bit == 0; g.intro_cue |= bit; first };
    if t < 11.0 {
        // A park at night, in 3D: a slow push toward you on the bench as the pills kick in.
        cutscene3d::park(view, cam, t, ((t - 5.0) / 6.0).clamp(0.0, 1.0));
        if t < 4.5 { caption("", "You hadn't slept in weeks.", t, WHITE); }
        else if t < 9.0 { caption("", "So you took the pills on a park bench. Helix SleepWell MAX: \"the strongest sleep ever made.\" Just to rest your eyes.", t - 4.5, WHITE); }
        eyelids(((t - 8.0) / 3.0).powf(1.5));
    } else if t < 13.0 {
        clear_background(BLACK);
        if cue(g, 16) { g.sfx.heart(0.4); }
    } else if t < 20.0 {
        // Your eyes flutter open a crack: the night sky moving... someone is carrying you.
        let k = t - 13.0;
        let speaking = if k < 2.5 { "Ember" } else if k < 4.5 { "Atlas" } else { "Verse" };
        cutscene3d::carried(view, cam, k, speaking);
        // Blurry: a dark haze over everything.
        draw_rectangle(-margin(), 0.0, vw(), VH, Color::new(0.0, 0.0, 0.02, 0.35));
        if k < 2.5 { caption("Ember (muffled)", "Got them. Careful with their head.", k, face_color("Ember")); }
        else if k < 4.5 { caption("Atlas (muffled)", "The green is in the grass. Go. Now.", k - 2.5, face_color("Atlas")); }
        else { caption("Verse (muffled)", "Stay with us...", k - 4.5, face_color("Verse")); }
        // Eyes: barely open, fluttering, then shut again.
        let open = if k < 1.0 { 1.0 - k * 0.4 } else if k < 5.0 { 0.62 + 0.08 * (k * 5.0).sin() } else { 0.62 + (k - 5.0) / 2.0 * 0.38 };
        eyelids(open);
    } else if t < 21.5 {
        clear_background(BLACK);
    } else if t < 26.0 {
        // ONE YEAR LATER: a day counter racing to 365, then the card.
        clear_background(BLACK);
        if cue(g, 1) { g.sfx.radio(); }
        let k = ((t - 21.5) / 2.0).clamp(0.0, 1.0);
        let day = (k * k * 365.0) as i32;
        centered(&format!("DAY {day:03}"), 250.0, 34.0, Color::new(0.5, 0.5, 0.55, 1.0));
        if t > 23.5 {
            if cue(g, 2) { g.sfx.sting(); }
            let a = ((t - 23.5) * 2.0).min(1.0);
            let jit = if t < 23.9 { rand::gen_range(-3.0, 3.0) } else { 0.0 };
            draw_text("ONE YEAR LATER", (VW - measure_text("ONE YEAR LATER", None, 72, 1.0).width) / 2.0 + jit, 350.0, 72.0, Color::new(0.95, 0.95, 0.95, a));
            let w = ((t - 23.5) / 1.5).min(1.0) * 520.0;
            draw_rectangle((VW - w) / 2.0, 372.0, w, 3.0, Color::new(0.85, 0.3, 0.35, a));
            centered("365 days. 8,760 hours. The world kept going without you.", 420.0, 22.0, Color::new(0.6, 0.6, 0.65, a));
        }
    } else {
        let i = ((t - WAKE).max(0.0) / LINE) as usize;
        if i >= STORY.len() {
            clear_background(BLACK);
            let a = ((t - WAKE - STORY.len() as f32 * LINE) / 1.0).min(1.0);
            centered("HOLLOW HOURS", 330.0, 72.0, Color::new(0.85, 0.3, 0.35, a));
            return t > WAKE + STORY.len() as f32 * LINE + 2.5;
        }
        let (who, line) = STORY[i];
        let lt = t - WAKE - i as f32 * LINE;
        if (7..=9).contains(&i) {
            // ASTRA 9.1: a tall robot, its visor glowing the infected green.
            if cue(g, 4) { g.sfx.radio(); }
            let green = g.rules.eye;
            cutscene3d::astra(view, cam, t, green);
            centered("ASTRA 9.1", 70.0, 30.0, green);
            centered("HELIX DYNAMICS INC.", 104.0, 16.0, Color::new(0.55, 0.6, 0.6, 1.0));
        } else {
            // Waking up in your bed: the blanket to the foot of the bed, Atlas on your left, Verse on your right,
            // and Ember (once it walks in) at the foot of the bed, in the middle.
            if cue(g, 8) { g.sfx.heart(0.8); }
            let claude_in = if i >= 10 { Some(((t - WAKE - 10.0 * LINE) / 1.5).clamp(0.0, 1.0)) } else { None };
            cutscene3d::bedroom(view, cam, t, who, claude_in);
            if t < WAKE + 0.5 {
                let k = (t - 26.0) / 3.0;
                eyelids(1.0 - k + 0.35 * (k * 9.0).sin().max(0.0));
            }
        }
        let col = if who == "You" { Color::new(0.85, 0.85, 0.95, 1.0) } else { face_color(who) };
        if t >= WAKE { caption(who, line, lt, col); }
    }
    draw_text("Space = skip   Esc = back", VW - 230.0, 24.0, 18.0, Color::new(0.6, 0.6, 0.65, 0.8));
    false
}

