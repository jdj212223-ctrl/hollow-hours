//! The scout drone (Q, D-pad up on a controller, DRONE on touch). While it flies, your keys fly it and your body
//! stands still where you left it, exposed. In 3D you see through its camera.
//!  - It flies over furniture and water (walls stop it) and can't go through doors.
//!  - Its buzz is loud: infected nearby go after the DRONE instead of you. A perfect distraction.
//!  - Fly over food or water to grab it; it's yours when the drone comes back (Q again).
//!  - About 45 seconds of battery, recharging while it's docked. An infected that reaches it knocks it down.

use super::*;

pub struct Drone {
    pub pos: Vec2,
    pub map: usize,
    pub battery: f32,
    pub carry: Option<bool>,
}

const SPEED: f32 = 5.0;
const FLIGHT_SECS: f32 = 45.0;
/// How far its buzz carries (tiles).
const BUZZ: f32 = 7.0;

/// Where you're looking from: the drone while it flies, otherwise you.
pub fn view_pos(g: &Game) -> Vec2 {
    g.drone.as_ref().map(|d| d.pos).unwrap_or(g.pos)
}

/// Q: launch, or call it back.
pub fn toggle(g: &mut Game) {
    if g.drone.is_some() {
        land(g);
    } else if g.drone_charge < 0.25 {
        g.log.push(format!("The drone is charging ({}%).", (g.drone_charge * 100.0) as i32));
    } else {
        g.drone = Some(Drone { pos: g.pos, map: g.map, battery: g.drone_charge, carry: None });
        g.log.push("Drone up. Your body stays here: keep an eye on it. Q brings it back.".into());
    }
}

/// The drone comes home (and hands over what it carries).
pub fn land(g: &mut Game) {
    let Some(d) = g.drone.take() else { return };
    g.drone_charge = d.battery;
    g.sfx.drone(false);
    if let Some(water) = d.carry {
        if water { g.water = (g.water + g.rules.supply_amount).min(1.0); } else { g.food = (g.food + g.rules.supply_amount).min(1.0); }
        g.sfx.pickup();
        g.log.push(format!("The drone brought you {}.", if water { "water" } else { "food" }));
    } else {
        g.log.push("Drone back.".into());
    }
}

/// Moves the drone with your input. True while it flies (then you don't walk).
pub fn fly(g: &mut Game, d: Vec2, dt: f32) -> bool {
    let Some(dr) = g.drone.as_mut() else { return false };
    if d != Vec2::ZERO {
        let step = d.normalize() * SPEED * T * dt;
        let wall = |m: usize, x: f32, y: f32| tile(m, (x / T).floor() as i32, (y / T).floor() as i32) == '#';
        let r = 8.0;
        let nx = dr.pos.x + step.x;
        if !wall(dr.map, nx + r * step.x.signum(), dr.pos.y) && nx > T * 0.5 && nx < (MW as f32 - 0.5) * T { dr.pos.x = nx; }
        let ny = dr.pos.y + step.y;
        if !wall(dr.map, dr.pos.x, ny + r * step.y.signum()) && ny > T * 0.5 && ny < (MH as f32 - 0.5) * T { dr.pos.y = ny; }
    }
    true
}

/// Battery, charging, grabbing supplies, pulling the infected toward its buzz, and getting knocked down.
pub fn update(g: &mut Game, dt: f32) {
    let Some(dr) = g.drone.as_mut() else {
        g.drone_charge = (g.drone_charge + dt / 60.0).min(1.0);
        g.sfx.drone(false);
        return;
    };
    dr.battery -= dt / FLIGHT_SECS;
    let (dpos, dmap) = (dr.pos, dr.map);
    if dr.battery <= 0.0 {
        dr.battery = 0.0;
        g.log.push("Drone battery empty: it limps back home.".into());
        land(g);
        return;
    }
    g.sfx.drone(true);
    // Grab food or water it flies over.
    if dr.carry.is_none() {
        if let Some(i) = g.items.iter().position(|(m, p, _)| *m == dmap && p.distance(dpos) < 0.6 * T) {
            let (im, ip, water) = g.items.remove(i);
            if let Some(n) = &g.net { n.send(json!({"t": "pick", "map": im, "x": ip.x / T, "y": ip.y / T})); }
            if let Some(d) = g.drone.as_mut() { d.carry = Some(water); }
            g.log.push(format!("The drone grabbed {}. Bring it home (Q).", if water { "water" } else { "food" }));
        }
    }
    if guest(g) { return; } // the host's game moves the infected
    // The buzz: infected nearby go after the drone if it's closer than you.
    let (me_map, me) = (g.map, g.pos);
    let mut knocked = false;
    for c in g.chars.iter_mut().filter(|c| c.infected && c.map == dmap && c.stun <= 0.0) {
        let to_drone = c.pos.distance(dpos);
        if to_drone < BUZZ * T && (c.map != me_map || to_drone < c.pos.distance(me)) {
            c.chase = dpos;
            c.crazy = c.crazy.max(2.0);
        }
        if to_drone < 0.6 * T { knocked = true; }
    }
    if knocked {
        let carry = g.drone.as_ref().and_then(|d| d.carry);
        g.drone = None;
        g.drone_charge = 0.0;
        g.sfx.drone(false);
        g.sfx.sting();
        if let Some(water) = carry { g.items.push((dmap, dpos, water)); }
        g.log.push("Something swatted the drone out of the air. It's recharging from zero.".into());
    }
}

/// The drone in the 2D view: a little X with four rotors.
pub fn draw_2d(g: &Game) {
    let Some(d) = &g.drone else { return };
    if d.map != g.map { return; }
    let (x, y) = (d.pos.x, d.pos.y);
    let spin = get_time() as f32 * 30.0;
    for (dx, dy) in [(-7.0, -7.0), (7.0, -7.0), (-7.0, 7.0), (7.0, 7.0)] {
        draw_line(x, y, x + dx, y + dy, 2.0, Color::new(0.85, 0.85, 0.9, 1.0));
        draw_circle_lines(x + dx, y + dy, 4.0 + (spin + dx).sin().abs(), 1.0, Color::new(0.8, 0.9, 1.0, 0.8));
    }
    draw_circle(x, y, 3.0, Color::new(0.3, 0.8, 1.0, 1.0));
}
