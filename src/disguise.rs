//! Disguises and the cure (Infection mode).
//!  - Disguise kits (Helix Store, 3 uses): Z puts one on you for 90 seconds. The infected don't see or hear you unless
//!    you walk right into them. Sprinting, fighting or frying blows your cover. Tell a Claude "disguise yourself" (or
//!    "disguise everyone") and they get one for 2 minutes: the infected ignore them and can't infect them.
//!  - Antidotes (Helix Store, or brewed on the stove from a Helix chip + some water): C uses one. On yourself: -60%
//!    infection (even at 100%: one last chance). Next to an infected Claude: stay close 3 seconds and they come back.
//!  - Hiding (H) inside the cozy house slowly heals you (below 75%): inside these walls it doesn't spread.

use super::*;

pub const YOU_SECS: f32 = 90.0;
pub const CLAUDE_SECS: f32 = 120.0;
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
    if !done.is_empty() { g.log.push(format!("Disguised: {} (2 minutes). Kits left: {}.", done.join(", "), g.kits)); }
}

pub fn update(g: &mut Game, dt: f32) {
    if let Some((i, everyone)) = g.disguise_ask.take() { give_claudes(g, i, everyone); }
    if g.disguise_t > 0.0 {
        g.disguise_t -= dt;
        if g.disguise_t <= 0.0 { g.disguise_t = 0.0; g.log.push("Your disguise wore off.".into()); }
        if g.sprinting { blow(g, "you ran"); }
    }
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
        if g.disguise_t > 0.0 { g.log.push(format!("You're already disguised ({:.0} s left).", g.disguise_t)); }
        else if g.kits == 0 { g.log.push("No disguise kits (Helix Store on the Mac Studio, H$ 20).".into()); }
        else {
            g.kits -= 1;
            g.disguise_t = YOU_SECS;
            g.log.push(format!("Helix staff clothes, green eye paint. You look like one of them for 90 s. Don't run. (Kits left: {})", g.kits));
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
        let t = format!("DISGUISED {:.0} s   (don't run, don't fight)", g.disguise_t);
        draw_text(&t, sw / 2.0 - measure_text(&t, None, 18, 1.0).width / 2.0, 52.0, 18.0, Color::new(0.5, 1.0, 0.55, 0.95));
    }
    if let Some((_, t)) = g.cure {
        draw_rectangle(sw / 2.0 - 80.0, sh / 2.0 + 100.0, 160.0, 8.0, Color::new(0.0, 0.0, 0.0, 0.5));
        draw_rectangle(sw / 2.0 - 80.0, sh / 2.0 + 100.0, 160.0 * (1.0 - t / CURE_SECS), 8.0, Color::new(0.4, 0.8, 1.0, 0.95));
    }
    let items: Vec<String> = [(g.kits, "disguise"), (g.antidotes, "antidote"), (g.chips, "Helix chip")].iter()
        .filter(|(n, _)| *n > 0).map(|(n, w)| format!("{n} {w}{}", if *n > 1 { "s" } else { "" })).collect();
    if !items.is_empty() {
        let t = items.join(" · ");
        draw_text(&t, sw - measure_text(&t, None, 15, 1.0).width - 14.0, sh - 46.0, 15.0, Color::new(0.7, 0.9, 1.0, 0.9));
    }
}
