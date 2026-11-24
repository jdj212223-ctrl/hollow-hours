//! Getting between places without teleporting:
//! - the house's floors are joined by real stairs (you walk onto them and climb up or down),
//! - the house's outside doors (the beach, the park) are real doors that swing open as you step through,
//! - open-air paths (beach ↔ pool, the market, the lab, the subway) are a short walk with a quick fade.

use super::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Stairs { up: bool },
    Door,
    Path,
}

#[derive(Clone, Copy)]
pub struct Pass {
    pub kind: Kind,
    pub t: f32,
    pub to: usize,
    pub at: Vec2,
    pub from: Vec2,
    /// Into the doorway / up the stairs (a unit step on the grid).
    pub dir: Vec2,
    /// The tile you're going through.
    pub tile: (i32, i32),
}

/// After you arrive: how it happened and for how long you've been there.
#[derive(Clone, Copy)]
pub struct Arrive {
    pub kind: Kind,
    pub t: f32,
    pub tile: Option<(i32, i32)>,
}

/// Which floor of the house a map is (None if it isn't the house).
fn level(m: usize) -> Option<i32> {
    let n = map_name(m);
    if !n.starts_with("The cozy house") { return None; }
    Some(match n.as_str() {
        "The cozy house: upstairs" => 1,
        "The cozy house: basement" => -1,
        "The cozy house: attic" => 2,
        _ => 0,
    })
}

pub fn kind(from: usize, to: usize) -> Kind {
    match (level(from), level(to)) {
        (Some(a), Some(b)) if a != b => Kind::Stairs { up: b > a },
        (Some(_), None) | (None, Some(_)) => Kind::Door,
        _ => Kind::Path,
    }
}

/// What a tile is, if it's a way somewhere.
pub fn tile_kind(m: usize, x: i32, y: i32) -> Option<Kind> {
    door(tile(m, x, y), m).map(|(to, _)| kind(m, to))
}

/// The side of a doorway / stairs you walk in from (a unit step toward the open floor next to it).
pub fn facing(m: usize, x: i32, y: i32) -> Vec2 {
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let c = tile(m, x + dx, y + dy);
        if !c.is_ascii_uppercase() && !solid(m, (x + dx) as f32 * T + T / 2.0, (y + dy) as f32 * T + T / 2.0) {
            return vec2(dx as f32, dy as f32);
        }
    }
    vec2(1.0, 0.0)
}

fn duration(k: Kind) -> f32 {
    match k { Kind::Stairs { .. } => 1.2, Kind::Door => 0.95, Kind::Path => 0.45 }
}

/// You stepped onto a door/stairs tile: start going through instead of popping out on the other side.
pub fn start(g: &mut Game, x: i32, y: i32, to: usize, at: Vec2) {
    let k = kind(g.map, to);
    let dir = -facing(g.map, x, y);
    g.pass = Some(Pass { kind: k, t: 0.0, to, at, from: g.pos, dir, tile: (x, y) });
    match k {
        Kind::Door => g.sfx.door(),
        Kind::Stairs { .. } => g.sfx.step(true),
        Kind::Path => {}
    }
}

/// Each frame: climb, step through, then arrive.
pub fn update(g: &mut Game, dt: f32) {
    if let Some(a) = &mut g.arrive {
        a.t += dt;
        if a.t > 1.0 { g.arrive = None; }
    }
    let Some(mut p) = g.pass else { return };
    let before = p.t;
    p.t += dt;
    let d = duration(p.kind);
    let k = (p.t / d).min(1.0);
    // Glide forward into the doorway / up the steps, turning to face it.
    let center = vec2(p.tile.0 as f32 + 0.5, p.tile.1 as f32 + 0.5) * T;
    let start = center - p.dir * T * 0.5;
    let reach = match p.kind { Kind::Door => 0.35, _ => 0.8 };
    let k_move = if p.kind == Kind::Door { ((k - 0.25) / 0.75).clamp(0.0, 1.0) } else { k };
    g.pos = p.from.lerp(start, (k * 3.0).min(1.0)) + p.dir * T * reach * k_move;
    let want = p.dir.y.atan2(p.dir.x);
    let diff = (want - g.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    g.yaw += diff * (dt * 8.0).min(1.0);
    g.pitch *= 1.0 - (dt * 4.0).min(1.0);
    if let Kind::Stairs { .. } = p.kind {
        // A footstep on each step.
        if (before / 0.24) as i32 != (p.t / 0.24) as i32 { g.sfx.step(true); }
    }
    g.pass = Some(p);
    if p.t >= d {
        g.pass = None;
        arrive(g, p);
    }
}

fn arrive(g: &mut Game, p: Pass) {
    let (from_map, from_pos) = (g.map, p.from);
    let m = p.to;
    g.map = m;
    g.pos = p.at * T;
    learn::followed_through_door(g, from_map, from_pos, m, p.at);
    // The door/stairs you came out of on this side, right behind you: face away from it.
    let (ax, ay) = (p.at.x.floor() as i32, p.at.y.floor() as i32);
    let mut back = None;
    'find: for dy in -1..=1 {
        for dx in -1..=1 {
            if (dx != 0 || dy != 0) && door(tile(m, ax + dx, ay + dy), m).map(|(to, _)| to) == Some(from_map) {
                back = Some((ax + dx, ay + dy));
                break 'find;
            }
        }
    }
    let to = match back {
        Some((x, y)) => facing(m, x, y),
        None => vec2(MW as f32 / 2.0, MH as f32 / 2.0) - p.at,
    };
    g.yaw = to.y.atan2(to.x);
    g.arrive = Some(Arrive { kind: p.kind, t: 0.0, tile: back });
    if p.kind == Kind::Door { g.sfx.door(); }
    g.log.push(match p.kind {
        Kind::Stairs { up: true } => format!("You climb the stairs to {}.", map_name(m)),
        Kind::Stairs { up: false } => format!("You go down the stairs to {}.", map_name(m)),
        Kind::Door => format!("You step through the door: {}.", map_name(m)),
        Kind::Path => format!("You walk into {}.", map_name(m)),
    });
    if map_name(m) == "The Helix Lab" && g.infection {
        g.log.push("The air hums. In the middle of the room: a tall glass pod. Empty.".into());
        meta_event(g, MetaEvent::Lab);
    }
    if map_name(m) == story::TOWER_MAP && g.infection {
        g.log.push("Floors of servers hum in the dark. Far above, something is waiting. It already knows you're here.".into());
    }
    if map_name(m) == "The Marketplace" && g.infection {
        g.log.push("Fluorescent lights hum. Somewhere a robot voice says: \"Welcome to Helix Market.\"".into());
    }
}

/// How high your eyes are above normal while on the stairs (and just after, coming off them).
pub fn lift(g: &Game) -> f32 {
    if settings::with(|s| s.reduce_motion) { return 0.0; }
    if let Some(p) = g.pass {
        if let Kind::Stairs { up } = p.kind {
            let k = (p.t / duration(p.kind)).min(1.0);
            return if up { k * 1.3 } else { -k * 1.3 };
        }
    }
    if let Some(a) = g.arrive {
        if let Kind::Stairs { up } = a.kind {
            let k = (1.0 - a.t / 0.6).max(0.0);
            return if up { -k * 0.9 } else { k * 0.9 };
        }
    }
    0.0
}

/// How dark the screen is going through (a short blink between places).
pub fn fade(g: &Game) -> f32 {
    if let Some(p) = g.pass {
        let k = p.t / duration(p.kind);
        let from = match p.kind { Kind::Path => 0.3, Kind::Door => 0.7, Kind::Stairs { .. } => 0.65 };
        return ((k - from) / (1.0 - from)).clamp(0.0, 1.0);
    }
    if let Some(a) = g.arrive {
        return (1.0 - a.t / 0.35).clamp(0.0, 1.0);
    }
    0.0
}

/// How far open a door tile is (0 shut … 1 wide open): yours as you go through, and the one you just came out of.
pub fn door_open(g: &Game, x: i32, y: i32) -> f32 {
    if let Some(p) = g.pass {
        if p.tile == (x, y) { return (p.t / 0.35).min(1.0); }
    }
    if let Some(a) = g.arrive {
        if a.tile == Some((x, y)) { return (1.0 - (a.t - 0.2) / 0.6).clamp(0.0, 1.0); }
    }
    0.0
}

/// 3D: stairs going up (steps rising to an opening in the ceiling) or down (steps into a dark stairwell).
pub fn draw_stairs(center: Vec3, open_side: Vec2, up: bool, wall: Color, fogc: impl Fn(Color, Vec3) -> Color) {
    let a = vec3(-open_side.x, 0.0, -open_side.y); // away from where you walk in
    let along_x = a.x.abs() > 0.5;
    let wood = Color::new(0.42, 0.29, 0.19, 1.0);
    let n = 6;
    let depth = 1.0 / n as f32;
    for i in 0..n {
        let off = -0.5 + (i as f32 + 0.5) * depth;
        let shade = 0.8 + 0.2 * ((i % 2) as f32);
        let col = Color::new(wood.r * shade, wood.g * shade, wood.b * shade, 1.0);
        if up {
            let h = (i + 1) as f32 * super::view3d::WALL_H / n as f32;
            let size = if along_x { vec3(depth, h, 0.9) } else { vec3(0.9, h, depth) };
            let p = center + a * off + vec3(0.0, h / 2.0, 0.0);
            draw_cube(p, size, None, fogc(col, p));
        } else {
            let top = -(i as f32 + 1.0) * 0.28;
            let dark = 1.0 - i as f32 * 0.13;
            let size = if along_x { vec3(depth, 0.08, 0.9) } else { vec3(0.9, 0.08, depth) };
            let p = center + a * off + vec3(0.0, top, 0.0);
            draw_cube(p, size, None, fogc(Color::new(col.r * dark, col.g * dark, col.b * dark, 1.0), p));
        }
    }
    // A handrail along one side.
    let side = vec3(-a.z, 0.0, a.x) * 0.45;
    let rail = Color::new(0.25, 0.18, 0.12, 1.0);
    let (h0, h1) = if up { (0.9, super::view3d::WALL_H + 0.6) } else { (0.9, -0.9) };
    for k in 0..8 {
        let f = k as f32 / 7.0;
        let p = center + side + a * (-0.5 + f) + vec3(0.0, h0 + (h1 - h0) * f, 0.0);
        draw_cube(p, vec3(0.05, 0.05, 0.05) + if along_x { vec3(0.13, 0.0, 0.0) } else { vec3(0.0, 0.0, 0.13) }, None, fogc(rail, p));
    }
    if !up {
        // The bottom of the stairwell is dark.
        draw_plane(center - vec3(0.0, 1.9, 0.0), vec2(0.5, 0.5), None, Color::new(0.0, 0.0, 0.0, 1.0));
        // The edge of the opening.
        let lip = center - a * 0.5 + vec3(0.0, -0.02, 0.0);
        let size = if along_x { vec3(0.04, 0.06, 1.0) } else { vec3(1.0, 0.06, 0.04) };
        draw_cube(lip, size, None, fogc(Color::new(wall.r * 1.4, wall.g * 1.4, wall.b * 1.4, 1.0), lip));
    }
}

/// 3D: a door in its frame, swung open by `open` (0..1), the outside showing through when it's open.
pub fn draw_door(center: Vec3, open_side: Vec2, open: f32, wall: Color, outside: Color, fogc: impl Fn(Color, Vec3) -> Color) {
    let wh = super::view3d::WALL_H;
    let a = vec3(-open_side.x, 0.0, -open_side.y);
    let along_x = a.x.abs() > 0.5;
    let across = vec3(-a.z, 0.0, a.x); // along the wall
    let frame = Color::new(wall.r * 1.5, wall.g * 1.5, wall.b * 1.5, 1.0);
    // Wall above the door, and the two posts.
    let lintel = center + vec3(0.0, wh - 0.2, 0.0);
    draw_cube(lintel, if along_x { vec3(1.0, 0.4, 1.0) } else { vec3(1.0, 0.4, 1.0) }, None, fogc(frame, lintel));
    for s in [-1.0, 1.0] {
        let post = center + across * (0.46 * s) + vec3(0.0, (wh - 0.4) / 2.0, 0.0);
        draw_cube(post, if along_x { vec3(1.0, wh - 0.4, 0.08) } else { vec3(0.08, wh - 0.4, 1.0) }, None, fogc(frame, post));
    }
    // What's outside, through the gap (seen when it's open).
    let far = center + a * 0.5 + vec3(0.0, (wh - 0.4) / 2.0, 0.0);
    draw_cube(far, if along_x { vec3(0.02, wh - 0.4, 0.84) } else { vec3(0.84, wh - 0.4, 0.02) }, None, outside);
    // The door itself, hinged on one post, swinging away from you.
    let hinge = center + across * 0.42;
    // Turn so the free edge (toward -across) swings toward `a`, away from you.
    let v = -across;
    let sign = (v.z * a.x - v.x * a.z).signum();
    let m = Mat4::from_translation(hinge) * Mat4::from_rotation_y(sign * open * 1.45);
    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.push_model_matrix(m);
    let panel = Color::new(0.38, 0.25, 0.16, 1.0);
    let half = -across * 0.42;
    let body = half + vec3(0.0, (wh - 0.4) / 2.0, 0.0);
    draw_cube(body, if along_x { vec3(0.06, wh - 0.42, 0.84) } else { vec3(0.84, wh - 0.42, 0.06) }, None, fogc(panel, center));
    // Two panels' lines and the knob.
    for h in [0.45, 1.05] {
        let line = half + vec3(0.0, h, 0.0) - a * 0.035;
        draw_cube(line, if along_x { vec3(0.01, 0.45, 0.6) } else { vec3(0.6, 0.45, 0.01) }, None, fogc(Color::new(0.32, 0.21, 0.13, 1.0), center));
    }
    let knob = -across * 0.74 + vec3(0.0, 0.7, 0.0) - a * 0.06;
    draw_sphere(knob, 0.035, None, Color::new(0.85, 0.72, 0.35, 1.0));
    gl.quad_gl.pop_model_matrix();
}
