//! Disguises and the cure (Infection mode).
//!  - Disguise kits (Helix Store, 20 uses): Z puts one on you, and it stays on until you take it off (Z again).
//!    The infected don't see or hear you and can't infect you. Fighting or cooking blows your cover (running doesn't).
//!    Tell a Claude "disguise yourself" (or "disguise everyone") and they wear one too, for good.
//!  - Intel: disguised, stand close to an infected for a few seconds and you pick something up (an access-code
//!    fragment, something overheard). With 3, a terminal in the Helix Lab synthesizes the ASTRA Cure: it cures you,
//!    gives 3 antidotes, and goes out over the Helix network: every infected Claude comes back.
//!  - Antidotes (Helix Store, or brewed on the stove from a Helix chip + some water): C uses one. On yourself: -60%
//!    infection (even at 100%: one last chance). Next to an infected Claude: stay close 3 seconds and they come back.
//!  - Hiding (H) inside the cozy house slowly heals you (below 75%): inside these walls it doesn't spread.

use super::*;

/// Disguises stay on until taken off (or blown).
pub const YOU_SECS: f32 = f32::INFINITY;
pub const CLAUDE_SECS: f32 = f32::INFINITY;
pub const INTEL_NEEDED: u32 = 3;
const INTEL_SECS: f32 = 4.0;
pub const SYNTH_SECS: f32 = 15.0;
const CURE_SECS: f32 = 3.0;

/// Your disguise drops (something gave you away).
pub fn blow(g: &mut Game, why: &str) {
    if g.disguise_t > 0.0 {
        g.disguise_t = 0.0;
        g.log.push(format!("They see through your disguise! ({why})"));
        g.sfx.sting();
    }
}

/// You asked a Claude to disguise itself (or everyone). Returns what happened, for the log.
pub fn give_claudes(g: &mut Game, first: usize, everyone: bool) {
    let mut who: Vec<usize> = if everyone {
        (0..g.chars.len()).filter(|&i| g.chars[i].kind && !g.chars[i].infected).collect()
    } else { vec![first] };
    who.retain(|&i| g.chars[i].disguise_t <= 0.0);
    if g.kits == 0 {
        g.log.push("No disguise kits left (Helix Store on the Mac Studio).".into());
        return;
    }
    let mut done = vec![];
    for i in who {
        if g.kits == 0 { break; }
        g.kits -= 1;
        g.chars[i].disguise_t = CLAUDE_SECS;
        done.push(g.chars[i].name);
    }
    if !done.is_empty() { g.log.push(format!("Disguised: {} (for good, while they keep it on). Kits left: {}.", done.join(", "), g.kits)); }
}

pub fn update(g: &mut Game, dt: f32) {
    if let Some((i, everyone)) = g.disguise_ask.take() { give_claudes(g, i, everyone); }
    if let Some((i, everyone)) = g.undisguise_ask.take() {
        let mut off = vec![];
        for (j, c) in g.chars.iter_mut().enumerate() {
            if c.disguise_t > 0.0 && ((everyone && c.kind) || j == i) { c.disguise_t = 0.0; off.push(c.name); }
        }
        if !off.is_empty() { g.log.push(format!("{} wiped off the green paint.", off.join(", "))); }
    }
    if g.disguise_t > 0.0 {
        g.disguise_t -= dt;
        if g.disguise_t <= 0.0 { g.disguise_t = 0.0; g.log.push("Your disguise wore off.".into()); }
        gather_intel(g, dt);
    }
    synthesis(g, dt);
    for c in g.chars.iter_mut() { if c.disguise_t > 0.0 { c.disguise_t = (c.disguise_t - dt).max(0.0); } }
    // Resting: hiding inside the house slowly lowers your infection.
    if g.hidden && map_name(g.map).starts_with("The cozy house") && g.inf > 0.0 && g.inf < 0.75 && !g.lost_control {
        g.inf = (g.inf - dt / 300.0).max(0.0);
    }
    // Curing a Claude: stay close until it's done.
    if let Some((i, t)) = g.cure.as_mut() {
        let i = *i;
        let close = g.chars.get(i).is_some_and(|c| c.infected && c.map == g.map && c.pos.distance(g.pos) < 1.8 * T);
        if !close {
            g.cure = None;
            g.log.push("Too far: the antidote spilled. (Stay close, or stun them first.)".into());
            return;
        }
        *t -= dt;
        if *t <= 0.0 {
            g.cure = None;
            let c = &mut g.chars[i];
            c.infected = false;
            c.kind = true;
            c.infect = 0.0;
            c.crazy = 0.0;
            c.stun = 2.0;
            c.task = roam::Task::Follow;
            c.bubble = "...where... where am I?".into();
            c.via = String::new();
            c.bubble_t = 5.0;
            let name = c.name;
            g.log.push(format!("{name} is back! The green fades from its eyes."));
            g.sfx.pickup();
        }
    }
    if g.screen != Screen::Play || g.hidden && !g.lost_control { return; }
    // Z: disguise yourself.
    if pressed(KeyCode::Z) && !g.lost_control {
        if g.disguise_t > 0.0 { g.disguise_t = 0.0; g.log.push("You wipe off the green paint. You're yourself again (and they can see you).".into()); }
        else if g.kits == 0 { g.log.push("No disguise kits (Helix Store on the Mac Studio, H$ 20).".into()); }
        else {
            g.kits -= 1;
            g.disguise_t = YOU_SECS;
            g.log.push(format!("Helix staff clothes, green eye paint. You look like one of them until you take it off (Z). Don't fight, don't cook. (Kits left: {})", g.kits));
        }
    }
    // C: an antidote (an infected Claude next to you, otherwise yourself).
    if pressed(KeyCode::C) && g.cure.is_none() {
        if g.antidotes == 0 { g.log.push("No antidotes. Buy one (H$ 60) or brew one on the stove from a Helix chip.".into()); return; }
        let near = (0..g.chars.len()).filter(|&i| {
            let c = &g.chars[i];
            c.infected && c.claude && c.map == g.map && c.pos.distance(g.pos) < 1.4 * T
        }).min_by(|&a, &b| g.chars[a].pos.distance(g.pos).partial_cmp(&g.chars[b].pos.distance(g.pos)).unwrap());
        if let (Some(i), false) = (near, g.lost_control) {
            g.antidotes -= 1;
            g.cure = Some((i, CURE_SECS));
            let name = g.chars[i].name;
            g.log.push(format!("You press the antidote into {name}'s hand. Hold on... (stay close 3 s)"));
        } else if g.inf > 0.0 {
            g.antidotes -= 1;
            let was = g.inf;
            g.inf = (g.inf - 0.6).max(0.0);
            if g.lost_control {
                g.lost_control = false;
                g.log.push("Your hands are yours again. The antidote burns. Infection down to 40%.".into());
            } else {
                g.log.push(format!("The antidote burns. Infection {}% -> {}%.", (was * 100.0) as i32, (g.inf * 100.0) as i32));
            }
            g.sfx.pickup();
        } else {
            g.log.push("You're not infected. Save it for a Claude.".into());
        }
    }
}

/// HUD: disguise time, kits, antidotes, chips, and the cure in progress.
pub fn draw_hud(g: &Game, sw: f32, sh: f32) {
    if !g.infection { return; }
    if g.disguise_t > 0.0 {
        draw_rectangle_lines(-margin(), 0.0, vw(), sh, 10.0, Color::new(0.3, 0.9, 0.35, 0.35));
        let t = format!("DISGUISED   intel {}/{}   (Z takes it off · don't fight, don't cook)", g.intel.min(INTEL_NEEDED), INTEL_NEEDED);
        draw_text(&t, sw / 2.0 - measure_text(&t, None, 18, 1.0).width / 2.0, 52.0, 18.0, Color::new(0.5, 1.0, 0.55, 0.95));
    }
    if let Some(t) = g.synth_t {
        let w = 260.0;
        draw_rectangle(sw / 2.0 - w / 2.0, sh / 2.0 + 120.0, w, 10.0, Color::new(0.0, 0.0, 0.0, 0.6));
        draw_rectangle(sw / 2.0 - w / 2.0, sh / 2.0 + 120.0, w * (1.0 - t / SYNTH_SECS), 10.0, Color::new(0.5, 1.0, 0.6, 0.95));
        let l = format!("SYNTHESIZING THE ASTRA CURE... {:.0} s (stay at the terminal)", t.ceil());
        draw_text(&l, sw / 2.0 - measure_text(&l, None, 18, 1.0).width / 2.0, sh / 2.0 + 112.0, 18.0, Color::new(0.6, 1.0, 0.65, 1.0));
    }
    if let Some((_, t)) = g.cure {
        draw_rectangle(sw / 2.0 - 80.0, sh / 2.0 + 100.0, 160.0, 8.0, Color::new(0.0, 0.0, 0.0, 0.5));
        draw_rectangle(sw / 2.0 - 80.0, sh / 2.0 + 100.0, 160.0 * (1.0 - t / CURE_SECS), 8.0, Color::new(0.4, 0.8, 1.0, 0.95));
    }
    let items: Vec<String> = [(g.kits, "disguise"), (g.antidotes, "antidote"), (g.chips, "Helix chip"), (g.intel, "intel")].iter()
        .filter(|(n, _)| *n > 0).map(|(n, w)| format!("{n} {w}{}", if *n > 1 { "s" } else { "" })).collect();
    if !items.is_empty() {
        let t = items.join(" · ");
        draw_text(&t, sw - measure_text(&t, None, 15, 1.0).width - 14.0, sh - 46.0, 15.0, Color::new(0.7, 0.9, 1.0, 0.9));
    }
}

/// Disguised and close to an infected for a few seconds: you pick up a piece of intel (once per infected).
fn gather_intel(g: &mut Game, dt: f32) {
    if g.screen != Screen::Play || !g.infection { return; }
    let near = g.chars.iter().position(|c| c.infected && c.map == g.map && c.pos.distance(g.pos) < 2.5 * T && !g.intel_from.contains(&c.name));
    match near {
        Some(i) if g.intel_t.0 == Some(i) => {
            g.intel_t.1 += dt;
            if g.intel_t.1 >= INTEL_SECS {
                let c = &g.chars[i];
                let (name, robot) = (c.name, combat::is_robot(c));
                g.intel_from.push(name);
                g.intel += 1;
                g.intel_t = (None, 0.0);
                const OVERHEARD: [&str; 4] = ["\"...the lab door answers to staff with the code on their badge...\"",
                    "\"...ASTRA keeps the cure recipe on the lab terminals. it doesn't want it used...\"",
                    "\"...the network listens to the lab. what the lab sends, every one of us hears...\"",
                    "\"...the pod was empty before it started. it was never inside...\""];
                let what = if robot { format!("{name}'s badge flickers: you memorise part of a Helix access code") }
                    else { format!("{name} mutters, not noticing you: {}", OVERHEARD[(g.intel as usize + i) % OVERHEARD.len()]) };
                g.log.push(format!("INTEL {}/{}: {what}.", g.intel.min(INTEL_NEEDED), INTEL_NEEDED));
                if g.intel == INTEL_NEEDED { g.log.push("That's enough to get into the Helix Lab's cure files. Go to a terminal in the Lab (E).".into()); }
                g.sfx.radio();
            }
        }
        Some(i) => g.intel_t = (Some(i), 0.0),
        None => g.intel_t = (None, 0.0),
    }
}

/// At a Helix Lab terminal with enough intel: E starts the ASTRA Cure.
pub fn lab_ready(g: &Game) -> bool {
    g.infection && g.synth_t.is_none() && g.intel >= INTEL_NEEDED && map_name(g.map) == "The Helix Lab" && near_tile(g, 't')
}

pub fn start_synthesis(g: &mut Game) {
    g.synth_t = Some(SYNTH_SECS);
    g.log.push("You type the access code. The terminal opens ASTRA_CURE.bin and starts synthesizing. Stay at the terminal (15 s)...".into());
    g.sfx.radio();
}

fn synthesis(g: &mut Game, dt: f32) {
    let Some(mut t) = g.synth_t else { return };
    if !(map_name(g.map) == "The Helix Lab" && near_tile(g, 't')) {
        g.synth_t = None;
        g.log.push("You left the terminal: the synthesis stopped. (The intel is still yours: try again.)".into());
        return;
    }
    t -= dt;
    g.synth_t = Some(t);
    if t > 0.0 { return; }
    g.synth_t = None;
    g.intel -= INTEL_NEEDED;
    g.inf = 0.0;
    g.lost_control = false;
    g.antidotes += 3;
    let mut back = vec![];
    for c in g.chars.iter_mut().filter(|c| c.claude && c.infected) {
        c.infected = false;
        c.kind = true;
        c.infect = 0.0;
        c.crazy = 0.0;
        c.stun = 2.0;
        c.task = roam::Task::Follow;
        c.bubble = "...the green... it's gone. I can think again.".into();
        c.via = String::new();
        c.bubble_t = 6.0;
        back.push(c.name);
    }
    g.log.push("THE ASTRA CURE. Your infection: 0%. Three vials of it in your pocket.".into());
    if back.is_empty() {
        g.log.push("The terminal sends the cure out over the Helix network. Somewhere, green eyes flicker.".into());
    } else {
        g.log.push(format!("The terminal sends the cure out over the Helix network... and {} come back!", back.join(", ")));
    }
    g.sfx.pickup();
    g.sfx.sting();
}

/// "you have not paid." You can, now: a Helix robot next to you (E, H$ 5).
pub const PAY: u32 = 5;

pub fn robot_to_pay(g: &Game) -> Option<usize> {
    (0..g.chars.len()).filter(|&i| {
        let c = &g.chars[i];
        c.infected && combat::is_robot(c) && c.map == g.map && c.pos.distance(g.pos) < 1.8 * T
    }).min_by(|&a, &b| g.chars[a].pos.distance(g.pos).total_cmp(&g.chars[b].pos.distance(g.pos)))
}

/// Paying a robot: it stops chasing you for a while ("processing payment"), and if you're disguised it prints a
/// receipt with part of a Helix access code on it (intel, once per robot).
pub fn pay_robot(g: &mut Game) {
    let Some(i) = robot_to_pay(g) else { return };
    let name = g.chars[i].name;
    if g.credits < PAY {
        g.log.push(format!("{name}: \"insufficient funds.\" (you need H$ {PAY}, you have H$ {})", g.credits));
        return;
    }
    g.credits -= PAY;
    let c = &mut g.chars[i];
    c.crazy = 0.0;
    c.stun = c.stun.max(20.0);
    c.bubble = "thank you for shopping at helix.".into();
    c.via = String::new();
    c.bubble_t = 4.0;
    g.sfx.pickup();
    let receipt = g.disguise_t > 0.0 && !g.intel_from.contains(&name);
    if receipt {
        g.intel_from.push(name);
        g.intel += 1;
        g.log.push(format!("You pay {name} H$ {PAY}. It prints a receipt... with part of a Helix access code on it. INTEL {}/{}.",
            g.intel.min(INTEL_NEEDED), INTEL_NEEDED));
        if g.intel == INTEL_NEEDED { g.log.push("That's enough to get into the Helix Lab's cure files. Go to a terminal in the Lab (E).".into()); }
    } else {
        g.log.push(format!("You pay {name} H$ {PAY}. \"payment accepted.\" It stands still, processing. (20 s)"));
    }
}
