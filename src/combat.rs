//! Fighting back (Infection mode). F attacks with what you hold (fists, a bat or a knife), G kicks, and G while
//! sprinting is a dropkick. Weapons lie around: a bat in the house and on the beach, knives in the kitchen and the
//! Marketplace. Hits stun the infected and knock them back (never more: they're corrupted friends, not monsters),
//! except the Helix store robots, which a knife shuts down for good. No gore: thuds and sparks.

use super::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Weapon {
    Bat,
    Knife,
}

impl Weapon {
    pub fn name(self) -> &'static str { match self { Weapon::Bat => "bat", Weapon::Knife => "knife" } }
    fn uses(self) -> u32 { match self { Weapon::Bat => 12, Weapon::Knife => 8 } }
}

/// One kind of hit: how far it reaches, how long it stuns, how far it shoves, how long until the next one, how loud.
struct Hit { range: f32, stun: f32, knock: f32, cooldown: f32, noise: f32 }

/// A stun this long means a robot was shut down for good.
pub const SHUT_DOWN: f32 = 1e9;

pub fn is_robot(c: &Character) -> bool { c.inspired.contains("robot") }

/// The weapons lying around at the start of a game: (map, where, what).
pub fn spawn() -> Vec<(usize, Vec2, Weapon)> {
    // Weapons aren't free anymore: buy them in the Helix Store on the Mac Studio.
    vec![]
}

/// A weapon from the delivery drone, at the house's front door.
pub fn delivered(w: Weapon) -> (usize, Vec2, Weapon) {
    (0, vec2(27.5, 13.5) * T, w)
}

/// The infected you'd hit: the nearest one in reach (in front of you, in 3D).
fn target(g: &Game, range: f32) -> Option<usize> {
    let fwd = vec2(g.yaw.cos(), g.yaw.sin());
    (0..g.chars.len()).filter(|&i| {
        let c = &g.chars[i];
        let to = c.pos - g.pos;
        c.infected && c.map == g.map && c.stun < SHUT_DOWN / 2.0 && to.length() < range * T
            && (!g.view3d || to.length() < 0.5 * T || to.normalize_or_zero().dot(fwd) > 0.45)
    }).min_by(|&a, &b| g.chars[a].pos.distance(g.pos).partial_cmp(&g.chars[b].pos.distance(g.pos)).unwrap())
}

/// Pushes a character away, stopping at walls and furniture.
fn shove(c: &mut Character, dir: Vec2, dist: f32) {
    let steps = (dist * 4.0).ceil() as i32;
    for _ in 0..steps {
        let next = c.pos + dir * (dist * T / steps as f32);
        let front = next + dir * 0.35 * T; // keep the whole body out of the wall, not just its middle
        if solid(c.map, next.x, next.y) || solid(c.map, front.x, front.y) { break; }
        c.pos = next;
    }
    c.target = c.pos;
}

/// Lands a hit on character i (here on the host, or sent to the host when you're a guest).
pub fn apply_hit(g: &mut Game, i: usize, stun: f32, dir: Vec2, knock: f32, shut_down: bool) {
    let Some(c) = g.chars.get_mut(i) else { return };
    if !c.infected { return; }
    if shut_down {
        c.stun = SHUT_DOWN;
        c.crazy = 0.0;
        c.target = c.pos;
    } else {
        c.stun = c.stun.max(stun);
        shove(c, dir, knock);
    }
}

fn attack(g: &mut Game, hit: Hit, what: &str) {
    if g.attack_cd > 0.0 || g.down_t > 0.0 { return; }
    g.attack_cd = hit.cooldown;
    g.swing_t = 0.25;
    crate::disguise::blow(g, "you attacked");
    g.noise = g.noise.max(hit.noise);
    let Some(i) = target(g, hit.range) else {
        g.sfx.step(true);
        return;
    };
    let dir = (g.chars[i].pos - g.pos).normalize_or_zero();
    let knife = g.weapon.map(|w| w.0) == Some(Weapon::Knife) && what == "stab";
    let shut_down = knife && is_robot(&g.chars[i]);
    let name = g.chars[i].name;
    if guest(g) {
        if let Some(n) = &g.net { n.send(json!({"t": "hit", "i": i, "stun": hit.stun, "dx": dir.x, "dy": dir.y, "knock": hit.knock, "off": shut_down})); }
    }
    apply_hit(g, i, hit.stun, dir, hit.knock, shut_down);
    g.sfx.hit();
    if shut_down {
        g.sfx.sting();
        g.credits += 10;
        g.chips += 1;
        g.log.push(format!("Sparks fly. {name} powers down for good. It dropped a credit chip (+H$ 10) and a Helix chip (brew it into an antidote)."));
    } else {
        g.log.push(format!("You {what} {name}. It staggers back."));
    }
    // Weapons wear out.
    if matches!(what, "hit" | "stab") {
        if let Some((w, left)) = g.weapon.as_mut() {
            *left = left.saturating_sub(1);
            if *left == 0 {
                g.log.push(format!("Your {} broke.", w.name()));
                g.weapon = None;
            }
        }
    }
}

/// F: attack with what you're holding. G: kick (a dropkick if you're sprinting). Walking over a weapon picks it up.
pub fn update(g: &mut Game, dt: f32) {
    g.attack_cd = (g.attack_cd - dt).max(0.0);
    g.swing_t = (g.swing_t - dt).max(0.0);
    if g.down_t > 0.0 {
        g.down_t -= dt;
        if g.down_t <= 0.0 { g.log.push("You get back up.".into()); }
    }
    // Pick up a weapon you walk over (the one you held stays on the floor).
    if let Some(k) = g.weapon_items.iter().position(|(m, p, _)| *m == g.map && p.distance(g.pos) < 0.7 * T) {
        let (_, _, w) = g.weapon_items.remove(k);
        if let Some((old, _)) = g.weapon.take() { g.weapon_items.push((g.map, g.pos + vec2(0.6, 0.0) * T, old)); }
        g.weapon = Some((w, w.uses()));
        g.sfx.pickup();
        g.log.push(match w {
            Weapon::Bat => "You picked up a bat. F to swing it.".into(),
            Weapon::Knife => "You picked up a knife. F to stab. The Helix robots go down in one.".into(),
        });
    }
    if g.screen != Screen::Play || g.hidden || g.lost_control || g.drone.is_some() { return; }
    if pressed(KeyCode::F) {
        match g.weapon.map(|w| w.0) {
            Some(Weapon::Bat) => attack(g, Hit { range: 1.6, stun: 3.0, knock: 2.0, cooldown: 0.9, noise: 1.0 }, "hit"),
            Some(Weapon::Knife) => attack(g, Hit { range: 1.1, stun: 2.0, knock: 0.4, cooldown: 0.45, noise: 0.5 }, "stab"),
            None => attack(g, Hit { range: 1.1, stun: 1.2, knock: 0.6, cooldown: 0.5, noise: 0.6 }, "punch"),
        }
    }
    if pressed(KeyCode::G) {
        if g.sprinting {
            // Dropkick: flies them across the room, and you land on the floor.
            attack(g, Hit { range: 1.7, stun: 3.5, knock: 3.5, cooldown: 1.4, noise: 1.3 }, "dropkick");
            g.down_t = 1.2;
            g.stamina = (g.stamina - 0.3).max(0.0);
        } else {
            attack(g, Hit { range: 1.2, stun: 1.5, knock: 1.5, cooldown: 0.7, noise: 0.8 }, "kick");
        }
    }
}

/// Weapons on the floor (2D: a little bat or knife).
pub fn draw_items_2d(g: &Game) {
    // The rare book: a small pink book.
    if let Some((m, p)) = g.rare_book { if m == g.map { draw_rectangle(p.x - 6.0, p.y - 8.0, 12.0, 16.0, Color::new(0.95, 0.45, 0.7, 1.0)); draw_line(p.x - 4.0, p.y - 8.0, p.x - 4.0, p.y + 8.0, 2.0, Color::new(0.6, 0.2, 0.4, 1.0)); } }
    for (m, p, w) in &g.weapon_items {
        if *m != g.map { continue; }
        match w {
            Weapon::Bat => draw_line(p.x - 9.0, p.y + 6.0, p.x + 9.0, p.y - 6.0, 4.0, Color::new(0.6, 0.42, 0.25, 1.0)),
            Weapon::Knife => {
                draw_line(p.x - 7.0, p.y + 4.0, p.x + 2.0, p.y - 2.0, 3.0, Color::new(0.85, 0.87, 0.9, 1.0));
                draw_line(p.x + 2.0, p.y - 2.0, p.x + 7.0, p.y - 5.0, 3.0, Color::new(0.25, 0.2, 0.18, 1.0));
            }
        }
    }
}

/// What you hold, drawn in the corner of your view in 3D (it swings when you attack), and how you're doing.
pub fn draw_hud(g: &Game, sw: f32, sh: f32) {
    if !g.infection { return; }
    let swing = (g.swing_t / 0.25).clamp(0.0, 1.0);
    if g.view3d && g.drone.is_none() {
        let (x, y) = (sw - 170.0 - swing * 120.0, sh - 40.0 - swing * 60.0);
        match g.weapon.map(|w| w.0) {
            Some(Weapon::Bat) => draw_line(x, y, x + 110.0 - swing * 40.0, y - 160.0 + swing * 60.0, 16.0, Color::new(0.55, 0.38, 0.22, 1.0)),
            Some(Weapon::Knife) => {
                draw_line(x + 20.0, y, x + 50.0, y - 40.0, 12.0, Color::new(0.2, 0.16, 0.14, 1.0));
                draw_line(x + 50.0, y - 40.0, x + 80.0 - swing * 30.0, y - 110.0, 8.0, Color::new(0.85, 0.87, 0.9, 1.0));
            }
            None if swing > 0.0 => draw_circle(x + 60.0, y - 40.0, 34.0, Color::new(0.95, 0.85, 0.75, 1.0)),
            None => {}
        }
    }
    if let Some((w, left)) = g.weapon {
        draw_text(&format!("{} ({} hits left)  F", w.name(), left), sw - 230.0, 22.0 + if g.ai_t < 6.0 { 30.0 } else { 0.0 }, 16.0, Color::new(1.0, 0.85, 0.6, 0.9));
    }
    if g.down_t > 0.0 {
        draw_text("on the floor...", sw / 2.0 - 50.0, sh / 2.0 + 90.0, 18.0, Color::new(1.0, 0.6, 0.5, 0.9));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weapons_are_bought_and_shoves_stop_at_walls() {
        set_maps(vec![]);
        assert!(spawn().is_empty(), "no free weapons: they're bought");
        assert!(!solid(0, delivered(Weapon::Bat).1.x, delivered(Weapon::Bat).1.y), "deliveries land on the floor");
        let mut robot = characters().into_iter().find(|c| c.name == "Clerk-07").unwrap();
        assert!(is_robot(&robot));
        // Shoved toward the house's west wall from 2 tiles away: it stops in front of the wall, never inside it.
        robot.map = 0;
        robot.pos = vec2(2.5, 9.5) * T;
        shove(&mut robot, vec2(-1.0, 0.0), 5.0);
        assert!(!solid(0, robot.pos.x, robot.pos.y) && robot.pos.x > T, "stopped at the wall: {}", robot.pos / T);
    }
}
