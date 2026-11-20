//! The longer you survive, the more the infected learn (Infection mode):
//!  - after 6 minutes they follow you through doors, and hunters come over from other places to find you
//!  - after 12 minutes they smash through walls to reach you (the holes stay until the next game)

use super::*;

const DOORS_AT: f32 = 360.0;
const WALLS_AT: f32 = 720.0;

/// A wall (not the map's edge) between an infected and you, that it can break.
fn wall_toward(m: usize, from: Vec2, to: Vec2) -> Option<(i32, i32)> {
    let (fx, fy) = ((from.x / T) as i32, (from.y / T) as i32);
    let d = to - from;
    let step = if d.x.abs() > d.y.abs() { (d.x.signum() as i32, 0) } else { (0, d.y.signum() as i32) };
    let (x, y) = (fx + step.0, fy + step.1);
    let inside = x >= 1 && y >= 1 && x <= MW as i32 - 2 && y <= MH as i32 - 2;
    (inside && tile(m, x, y) == '#').then_some((x, y))
}

pub fn update(g: &mut Game, dt: f32) {
    // Getting smarter.
    let level = if g.survived >= WALLS_AT { 2 } else if g.survived >= DOORS_AT { 1 } else { 0 };
    if level > g.learn_level {
        g.learn_level = level;
        g.sfx.sting();
        g.log.push(if level == 1 { "They're learning. They follow you through doors now.".into() }
            else { "They're learning. Walls won't stop them anymore.".into() });
    }
    if guest(g) { return; } // the host's game moves the infected
    // The ones that followed you through a door arrive a moment later, right behind you.
    let mut arrived = vec![];
    g.followers.retain_mut(|f| { f.1 -= dt; if f.1 <= 0.0 { arrived.push(*f); false } else { true } });
    for (i, _, m, at) in arrived {
        if let Some(c) = g.chars.get_mut(i).filter(|c| c.infected) {
            c.map = m;
            c.pos = at * T;
            c.target = c.pos;
            c.chase = g.pos;
            c.crazy = 4.0;
            let name = c.name;
            g.log.push(format!("{name} followed you through the door."));
            g.sfx.door();
        }
    }
    if level == 0 { return; }
    // Hunters: now and then one comes over from another place to look for you.
    g.hunt_t -= dt;
    if g.hunt_t <= 0.0 {
        g.hunt_t = rand::gen_range(70.0, 120.0);
        let far: Vec<usize> = (0..g.chars.len()).filter(|&i| g.chars[i].infected && g.chars[i].map != g.map && !g.chars[i].hunting).collect();
        if !far.is_empty() {
            let i = far[rand::gen_range(0, far.len())];
            g.chars[i].hunting = true;
            let (name, from) = (g.chars[i].name, map_name(g.chars[i].map));
            match g.chars.iter().find(|c| c.kind && !c.infected).map(|c| c.name) {
                Some(k) => g.log.push(format!("(radio) {k}: {name} left {from}. It's coming this way.")),
                None => g.log.push(format!("You hear something moving, far away. {name} is coming.")),
            }
            g.sfx.radio();
        }
    }
    let (me_map, me) = (g.map, g.pos);
    let mut broke: Vec<(usize, i32, i32, &'static str)> = vec![];
    for c in g.chars.iter_mut().filter(|c| c.infected && c.stun <= 0.0) {
        if c.hunting {
            if c.map == me_map { c.hunting = false; c.chase = me; c.crazy = c.crazy.max(3.0); }
            else if !roam::walk_toward(c, me_map, me) { c.hunting = false; }
        }
        // Breaking walls: chasing you, and a wall is in the way.
        if level >= 2 && c.map == me_map && c.crazy > 0.0 && !g.lost_control {
            match wall_toward(c.map, c.pos, me) {
                Some((x, y)) if c.pos.distance(me) < 9.0 * T => {
                    c.breach += dt;
                    if c.breach >= 3.0 { c.breach = 0.0; broke.push((c.map, x, y, c.name)); }
                }
                _ => c.breach = 0.0,
            }
        }
    }
    for (m, x, y, name) in broke {
        set_tile(m, x, y, b'.');
        if let Some(n) = &g.net { n.send(json!({"t": "breach", "map": m, "x": x, "y": y})); }
        g.sfx.sting();
        g.fear = (g.fear + 0.3).min(1.0);
        g.vest.play(haptics::Effect::Tense);
        g.log.push(format!("CRACK! {name} smashed through the wall."));
        // Into the house: Kigo sounds the alarm from the attic, and the music kicks in.
        if map_name(m).starts_with("The cozy house") {
            if g.breach_t <= 0.0 {
                let name = map_name(m);
                let floor = name.strip_prefix("The cozy house: ").unwrap_or("ground floor");
                g.log.push(format!("(radio) Kigo: BREACH. / {floor}. / basement, now."));
                g.log.push("(radio) Ember: We'll lead them around the house. Get to the basement and stay quiet. You never do the circling.".into());
            }
            g.breach_t = 60.0;
        }
    }
}

/// You went through a door: infected chasing you close behind will follow (once they've learned doors).
pub fn followed_through_door(g: &mut Game, from_map: usize, from_pos: Vec2, to_map: usize, at: Vec2) {
    if g.learn_level < 1 || guest(g) { return; }
    for (i, c) in g.chars.iter().enumerate() {
        if c.infected && c.map == from_map && c.crazy > 0.0 && c.pos.distance(from_pos) < 8.0 * T {
            g.followers.push((i, rand::gen_range(2.0, 4.0), to_map, at));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An infected next to the house's inner wall, with you on the other side, would break that wall; the edge of
    /// the map is never broken; and a new game puts every broken wall back.
    #[test]
    fn walls_break_and_come_back() {
        set_maps(vec![]);
        // The cozy house: the wall at x=13 between the two top rooms; infected at (12,3), you at (16,3).
        let hole = wall_toward(0, vec2(12.5, 3.5) * T, vec2(16.5, 3.5) * T);
        assert_eq!(hole, Some((13, 3)));
        assert_eq!(wall_toward(0, vec2(1.5, 3.5) * T, vec2(-3.0, 3.5) * T), None, "never the outside wall");
        set_tile(0, 13, 3, b'.');
        assert_eq!(tile(0, 13, 3), '.');
        reset_world();
        assert_eq!(tile(0, 13, 3), '#', "fixed in the next game");
    }
}
