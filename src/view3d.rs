//! Hollow Hours in 3D: a first-person view of the same tile world. The game itself still thinks in tiles (so the
//! rules, mods, multiplayer and the MCP all keep working); this only draws it. 1 tile = 1 unit, tile (x, y) is the
//! 3D point (x, height, y). Tab switches back to the old 2D map.

use super::*;

pub const WALL_H: f32 = 1.7;
const EYE: f32 = 1.05;
/// The 3D view covers the world part of the canvas (the HUD stays 2D under it).
pub const VIEW_H: f32 = MH as f32 * T;

/// The camera: your eyes (lower while hiding, a small bob while walking).
pub fn camera(g: &Game, target: Option<RenderTarget>) -> Camera3D {
    let p = super::drone::view_pos(g) / T;
    let calm = super::settings::with(|s| s.reduce_motion);
    let bob = if calm { 0.0 } else { (g.clock * 9.0).sin() * 0.035 * g.noise };
    let shake = if g.fear > 0.75 && !calm { rand::gen_range(-0.015, 0.015) } else { 0.0 };
    let eye = if g.drone.is_some() { 1.55 + 0.03 * (g.clock * 7.0).sin() } else if g.hidden { 0.45 } else { EYE } + bob + shake;
    let pos = vec3(p.x, eye + super::passage::lift(g), p.y);
    let look = vec3(g.yaw.cos() * g.pitch.cos(), g.pitch.sin(), g.yaw.sin() * g.pitch.cos());
    Camera3D {
        position: pos,
        target: pos + look,
        up: vec3(0.0, 1.0, 0.0),
        fovy: 1.2,
        aspect: Some(vw() / VIEW_H),
        render_target: target,
        ..Default::default()
    }
}

/// Where a 3D point shows up on the canvas (None if it's behind you).
pub fn project(cam: &Camera3D, p: Vec3) -> Option<Vec2> {
    let clip = cam.matrix() * vec4(p.x, p.y, p.z, 1.0);
    if clip.w <= 0.05 { return None; }
    let n = clip.truncate() / clip.w;
    if n.x.abs() > 1.3 || n.y.abs() > 1.3 { return None; }
    Some(vec2((n.x + 1.0) / 2.0 * vw() - margin(), (1.0 - n.y) / 2.0 * VIEW_H))
}

fn indoors(m: usize) -> bool {
    with_map(m, |d| d.rows[0].iter().filter(|&&c| c == b'#').count() > MW / 2)
}

/// Things far away fade into the dark (how far you can see depends on the map's darkness and your fear).
struct Fog { eye: Vec3, dist: f32, col: Color }
impl Fog {
    fn at(&self, c: Color, p: Vec3) -> Color {
        let k = (p.distance(self.eye) / self.dist).clamp(0.0, 1.0).powf(1.3);
        Color::new(c.r + (self.col.r - c.r) * k, c.g + (self.col.g - c.g) * k, c.b + (self.col.b - c.b) * k, c.a)
    }
}

fn mul(c: Color, k: f32) -> Color { Color::new(c.r * k, c.g * k, c.b * k, c.a) }

/// Draws the world into `view` (a render target with a depth buffer).
pub fn render(g: &Game, view: &RenderTarget) {
    let cam = camera(g, Some(view.clone()));
    set_camera(&cam);
    let m = g.map;
    let dark = if g.on(0) { map_dark(m) } else { 0.1 };
    let inside = indoors(m);
    // Every place has its own air (4.0): warm house, cold lab, sodium subway, red tower, deep blue beach.
    let sky = match map_name(m).as_str() {
        "The Helix Lab" => Color::new(0.01, 0.035, 0.03, 1.0),
        "The Subway" => Color::new(0.04, 0.025, 0.008, 1.0),
        super::story::TOWER_MAP => Color::new(0.045, 0.006, 0.012, 1.0),
        "The Marketplace" => Color::new(0.02, 0.03, 0.03, 1.0),
        "The empty pool" => Color::new(0.01, 0.03, 0.045, 1.0),
        "The Park" => Color::new(0.015, 0.03, 0.02, 1.0),
        "The Town" => Color::new(0.03, 0.03, 0.06, 1.0),
        n if n.starts_with("The cozy house") => Color::new(0.025, 0.015, 0.008, 1.0),
        _ if inside => Color::new(0.01, 0.01, 0.015, 1.0),
        _ => Color::new(0.02, 0.03, 0.07, 1.0),
    };
    clear_background(sky);
    let fog = Fog { eye: cam.position, dist: (17.0 - dark * 10.0 - g.fear * 3.0).max(4.5), col: sky };
    let (floor, wall, drained) = (map_floor(m), map_wall(m), map_drained(m));
    let t = get_time() as f32;
    let me = vec2(cam.position.x, cam.position.z);

    // Stars over the beach.
    if !inside {
        for i in 0..120u32 {
            let h = i.wrapping_mul(2654435761);
            let a = (h % 6283) as f32 / 1000.0;
            let e = 0.15 + ((h / 6283) % 1000) as f32 / 1000.0 * 1.2;
            let p = cam.position + vec3(a.cos() * e.cos(), e.sin(), a.sin() * e.cos()) * 40.0;
            draw_sphere(p, 0.08, None, Color::new(0.85, 0.85, 1.0, 0.5 + 0.4 * (t + i as f32).sin().abs()));
        }
    }

    for y in 0..MH {
        for x in 0..MW {
            let c = tile(m, x as i32, y as i32);
            let center = vec3(x as f32 + 0.5, 0.0, y as f32 + 0.5);
            if vec2(center.x, center.z).distance(me) > fog.dist + 2.0 { continue; }
            let checker = if (x + y) % 2 == 0 { 1.0 } else { 0.93 };
            match c {
                '*' => {
                    // A tree: a trunk and a dark crown.
                    draw_plane(center, vec2(0.5, 0.5), None, fog.at(floor, center));
                    draw_cylinder(center, 0.1, 0.14, 1.4, None, fog.at(Color::new(0.3, 0.2, 0.13, 1.0), center));
                    draw_sphere(center + vec3(0.0, 1.8, 0.0), 0.75, None, fog.at(Color::new(0.1, 0.28 + 0.03 * ((x + y) % 3) as f32, 0.12, 1.0), center + vec3(0.0, 1.8, 0.0)));
                }
                '=' => {
                    // Subway tracks: a dark trench with two rails.
                    draw_plane(center - vec3(0.0, 0.5, 0.0), vec2(0.5, 0.5), None, fog.at(Color::new(0.12, 0.11, 0.1, 1.0), center));
                    for dz in [-0.25, 0.25] { draw_cube(center + vec3(0.0, -0.42, dz), vec3(1.0, 0.06, 0.06), None, fog.at(Color::new(0.5, 0.5, 0.52, 1.0), center)); }
                }
                '#' => {
                    let k = 0.85 + 0.15 * (((x * 7 + y * 13) % 5) as f32 / 4.0);
                    draw_cube(center + vec3(0.0, WALL_H / 2.0, 0.0), vec3(1.0, WALL_H, 1.0), None, fog.at(mul(wall, k * 1.6), center + vec3(0.0, 1.0, 0.0)));
                }
                '~' => {
                    if !g.on(1) {
                        draw_plane(center, vec2(0.5, 0.5), None, fog.at(Color::new(0.6, 0.62, 0.6, 1.0), center));
                    } else if drained {
                        // The empty pool: a pale tiled pit.
                        draw_plane(center - vec3(0.0, 0.9, 0.0), vec2(0.5, 0.5), None, fog.at(mul(Color::new(0.55, 0.78, 0.8, 1.0), checker), center));
                    } else {
                        let wave = 0.04 * (t * 1.3 + x as f32 * 0.6 + y as f32 * 0.3).sin();
                        draw_plane(center + vec3(0.0, -0.15 + wave, 0.0), vec2(0.5, 0.5), None,
                            fog.at(Color::new(0.04, 0.1 + 0.04 * (t + x as f32 * 0.3).sin(), 0.22, 1.0), center));
                    }
                }
                _ => {
                    let f = if c == ',' { Color::new(floor.r * 1.1, floor.g * 1.05, floor.b * 0.9, 1.0) } else if c == 'r' { rug(m) } else if c == 'x' { mul(floor, 0.75) } else { floor };
                    let way = if c.is_ascii_uppercase() { super::passage::tile_kind(m, x as i32, y as i32) } else { None };
                    let f = if way == Some(super::passage::Kind::Path) { Color::new(0.9, 0.82, 0.45, 1.0) } else { mul(f, checker) };
                    // A stairwell down has no floor (its steps go below).
                    if way != Some(super::passage::Kind::Stairs { up: false }) {
                        draw_plane(center, vec2(0.5, 0.5), None, fog.at(f, center));
                    }
                }
            }
            let stair_up = c.is_ascii_uppercase() && super::passage::tile_kind(m, x as i32, y as i32) == Some(super::passage::Kind::Stairs { up: true });
            if inside && c != '#' && !stair_up {
                draw_plane(center + vec3(0.0, WALL_H, 0.0), vec2(0.5, 0.5), None, fog.at(mul(wall, 0.55), center + vec3(0.0, WALL_H, 0.0)));
            }
            match c {
                'b' => {
                    let wood = Color::new(0.45, 0.32, 0.22, 1.0);
                    draw_cube(center + vec3(0.0, 0.4, 0.0), vec3(0.96, 0.8, 0.96), None, fog.at(wood, center));
                    draw_cube(center + vec3(0.0, 0.81, 0.0), vec3(0.98, 0.04, 0.98), None, fog.at(mul(wood, 1.25), center));
                }
                'm' => mac_studio(&fog, center, t),
                'y' => {
                    // Your bed: a frame, a white blanket, a pillow.
                    draw_cube(center + vec3(0.0, 0.22, 0.0), vec3(0.98, 0.44, 0.98), None, fog.at(Color::new(0.35, 0.25, 0.18, 1.0), center));
                    draw_cube(center + vec3(0.0, 0.47, 0.0), vec3(0.96, 0.08, 0.96), None, fog.at(Color::new(0.88, 0.88, 0.92, 1.0), center));
                    if (x + y) % 2 == 0 { draw_cube(center + vec3(0.0, 0.55, -0.3), vec3(0.6, 0.1, 0.3), None, fog.at(WHITE, center)); }
                }
                'w' => {
                    // Kigo's lookout: a chair facing a round window full of night.
                    draw_cube(center + vec3(0.0, 0.25, 0.2), vec3(0.5, 0.5, 0.5), None, fog.at(Color::new(0.4, 0.3, 0.2, 1.0), center));
                    draw_cube(center + vec3(0.0, 0.6, 0.42), vec3(0.5, 0.6, 0.06), None, fog.at(Color::new(0.4, 0.3, 0.2, 1.0), center));
                    draw_sphere(center + vec3(0.0, 1.15, -0.45), 0.42, None, Color::new(0.12, 0.2, 0.45, 0.9));
                }
                'g' => {
                    // The generator: a humming green-grey box with a little light.
                    draw_cube(center + vec3(0.0, 0.5, 0.0), vec3(0.96, 1.0, 0.96), None, fog.at(Color::new(0.33, 0.36, 0.3, 1.0), center));
                    draw_sphere(center + vec3(0.0, 1.05, 0.0), 0.06, None, Color::new(1.0, 0.6 + 0.3 * (t * 4.0).sin(), 0.2, 1.0));
                }
                'n' => {
                    // The park bench (where Ember found you).
                    let wood = Color::new(0.45, 0.32, 0.2, 1.0);
                    draw_cube(center + vec3(0.0, 0.42, 0.0), vec3(1.0, 0.06, 0.5), None, fog.at(wood, center));
                    draw_cube(center + vec3(0.0, 0.75, -0.24), vec3(1.0, 0.3, 0.05), None, fog.at(wood, center));
                    for dx in [-0.42, 0.42] { draw_cube(center + vec3(dx, 0.2, 0.0), vec3(0.06, 0.4, 0.45), None, fog.at(Color::new(0.15, 0.15, 0.16, 1.0), center)); }
                }
                't' => {
                    // A lab terminal: a desk with a green screen.
                    draw_cube(center + vec3(0.0, 0.37, 0.0), vec3(0.9, 0.74, 0.6), None, fog.at(Color::new(0.2, 0.21, 0.23, 1.0), center));
                    draw_cube(center + vec3(0.0, 0.98, 0.1), vec3(0.6, 0.4, 0.04), None, Color::new(0.2, 0.85 + 0.1 * (t * 2.0 + x as f32).sin(), 0.45, 1.0));
                }
                'o' => {
                    // ASTRA's pod: a tall glass cylinder, empty, faintly glowing green.
                    if (x + y) % 2 == 0 {
                        draw_cylinder(center, 0.48, 0.5, 2.2, None, Color::new(0.4, 0.95, 0.6, 0.18 + 0.05 * (t * 1.3).sin()));
                    }
                    draw_plane(center + vec3(0.0, 0.02, 0.0), vec2(0.5, 0.5), None, Color::new(0.25, 0.8, 0.45, 0.6));
                }
                'a' => {
                    // ASTRA 9.1 in its pod at the top of Helix Tower (one big pod over the 4x2 block).
                    draw_plane(center + vec3(0.0, 0.02, 0.0), vec2(0.5, 0.5), None, Color::new(0.2, 0.7, 0.4, 0.55));
                    if x == 14 && y == 1 {
                        let c = vec3(15.0, 0.0, 2.0);
                        draw_cylinder(c, 0.95, 1.0, 0.15, None, Color::new(0.2, 0.22, 0.25, 1.0));
                        draw_cylinder(c + vec3(0.0, 0.15, 0.0), 0.9, 0.9, 1.5, None, Color::new(0.4, 1.0, 0.6, 0.16 + 0.05 * (t * 1.1).sin()));
                        let body = Color::new(0.72, 0.75, 0.8, 1.0);
                        draw_cylinder(c + vec3(0.0, 0.2, 0.0), 0.2, 0.24, 0.92, None, body);
                        let head = c + vec3(0.03 * (t * 0.7).sin(), 1.32, 0.0);
                        draw_sphere(head, 0.2, None, body);
                        eyes(head, 0.2, cam.position, Color::new(0.3, 1.0, 0.4, 1.0), 0.04);
                        draw_cylinder(c + vec3(0.0, 1.62, 0.0), 0.95, 1.0, 0.07, None, Color::new(0.2, 0.22, 0.25, 1.0));
                    }
                }
                'Z' if !super::story::tower_open() => {
                    // Helix Tower's door, sealed: a steel slab with a red light.
                    draw_cube(center + vec3(0.0, WALL_H / 2.0, 0.0), vec3(1.0, WALL_H, 1.0), None, fog.at(Color::new(0.3, 0.31, 0.34, 1.0), center));
                    draw_sphere(center + vec3(0.0, WALL_H * 0.75, 0.52), 0.07, None, Color::new(1.0, 0.15 + 0.1 * (t * 3.0).sin(), 0.12, 1.0));
                }
                's' => stove(&fog, center, t, g.fry_t > 0.0),
                'c' | 'k' | 'f' | 'p' | 'h' | 'v' | 'q' | 'u' | 'e' | 'j' | 'i' | 'z' | 'x' => furniture(&fog, c, center, x, y, t, m),
                'L' => {
                    draw_cylinder(center, 0.03, 0.05, 1.3, None, fog.at(Color::new(0.25, 0.25, 0.27, 1.0), center));
                    draw_sphere(center + vec3(0.0, 1.35, 0.0), 0.16, None, Color::new(1.0, 0.85, 0.5, 1.0));
                }
                'd' if g.on(2) => {
                    // A doll on the floor that turns its eyes toward you.
                    let head = center + vec3(0.0, 0.35, 0.0);
                    draw_cylinder(center, 0.1, 0.16, 0.25, None, fog.at(Color::new(0.75, 0.3, 0.35, 1.0), center));
                    draw_sphere(head, 0.14, None, fog.at(Color::new(0.95, 0.85, 0.85, 1.0), head));
                    eyes(head, 0.14, cam.position, BLACK, 0.025);
                }
                c if c.is_ascii_uppercase() => {
                    use super::passage::{self, Kind};
                    let (xi, yi) = (x as i32, y as i32);
                    let side = passage::facing(m, xi, yi);
                    match passage::tile_kind(m, xi, yi) {
                        Some(Kind::Stairs { up }) => passage::draw_stairs(center, side, up, wall, |c, p| fog.at(c, p)),
                        Some(Kind::Door) => {
                            let outside = if inside { Color::new(0.05, 0.08, 0.16, 1.0) } else { Color::new(0.45, 0.32, 0.22, 1.0) };
                            passage::draw_door(center, side, passage::door_open(g, xi, yi), wall, outside, |c, p| fog.at(c, p));
                        }
                        _ => {
                            // An open path onward: a faint column of warm light.
                            draw_cube(center + vec3(0.0, WALL_H / 2.0, 0.0), vec3(0.98, WALL_H, 0.98), None, Color::new(1.0, 0.85, 0.45, 0.12));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // 4.0 lighting: warm pools of light under every lamp (drawn after the floors so they lie on top), with a flicker.
    let calm = super::settings::with(|s| s.reduce_motion);
    for y in 0..MH {
        for x in 0..MW {
            if tile(m, x as i32, y as i32) != 'L' { continue; }
            let c = vec3(x as f32 + 0.5, 0.012, y as f32 + 0.5);
            if vec2(c.x, c.z).distance(me) > fog.dist + 2.0 { continue; }
            let h = (x * 31 + y * 17) as f32;
            let flick = if calm { 1.0 } else { 0.85 + 0.15 * ((t * 7.0 + h).sin() * (t * 2.3 + h * 0.5).sin()).abs() };
            for (r, a) in [(2.6, 0.06), (1.7, 0.08), (0.9, 0.1)] {
                draw_cylinder(c, r, r, 0.004, None, Color::new(1.0, 0.8, 0.45, a * flick));
            }
            draw_sphere(c + vec3(0.0, 1.34, 0.0), 0.32, None, Color::new(1.0, 0.85, 0.5, 0.12 * flick));
        }
    }

    // Supplies: red apples and blue bottles, slowly turning so you notice them.
    for (im, p, water) in &g.items {
        if *im != m { continue; }
        let c = vec3(p.x / T, 0.0, p.y / T);
        let bounce = 0.05 * (t * 2.0 + c.x).sin();
        if *water {
            draw_cylinder(c + vec3(0.0, 0.05 + bounce, 0.0), 0.07, 0.07, 0.3, None, fog.at(Color::new(0.35, 0.65, 1.0, 1.0), c));
            draw_cylinder(c + vec3(0.0, 0.35 + bounce, 0.0), 0.03, 0.03, 0.06, None, Color::new(0.85, 0.9, 1.0, 1.0));
        } else {
            draw_sphere(c + vec3(0.0, 0.16 + bounce, 0.0), 0.13, None, fog.at(Color::new(0.85, 0.15, 0.15, 1.0), c));
            draw_cylinder(c + vec3(0.0, 0.27 + bounce, 0.0), 0.01, 0.015, 0.08, None, Color::new(0.3, 0.6, 0.2, 1.0));
        }
    }

    // Weapons on the floor: a bat, a knife.
    for (wm, p, w) in &g.weapon_items {
        if *wm != m { continue; }
        let c = vec3(p.x / T, 0.04, p.y / T);
        match w {
            super::combat::Weapon::Bat => draw_cube(c, vec3(0.75, 0.07, 0.07), None, fog.at(Color::new(0.55, 0.38, 0.22, 1.0), c)),
            super::combat::Weapon::Knife => {
                draw_cube(c + vec3(0.08, 0.0, 0.0), vec3(0.22, 0.02, 0.05), None, fog.at(Color::new(0.85, 0.87, 0.9, 1.0), c));
                draw_cube(c - vec3(0.1, 0.0, 0.0), vec3(0.14, 0.04, 0.05), None, fog.at(Color::new(0.2, 0.16, 0.14, 1.0), c));
            }
        }
    }
    // The rare book: a pink book on the floor, glowing a little so you can spot it.
    if let Some((bm, p, _)) = g.rare_book {
        if bm == m {
            let c = vec3(p.x / T, 0.04, p.y / T);
            draw_cube(c, vec3(0.22, 0.06, 0.3), None, fog.at(Color::new(0.95, 0.45, 0.7, 1.0), c));
            draw_cube(c + vec3(0.0, 0.035, 0.0), vec3(0.2, 0.01, 0.28), None, fog.at(Color::new(0.98, 0.95, 0.9, 1.0), c));
            draw_cylinder(c - vec3(0.0, 0.03, 0.0), 0.35, 0.35, 0.003, None, Color::new(1.0, 0.5, 0.8, 0.12 + 0.06 * (t * 2.0).sin()));
        }
    }
    // The characters.
    for (k, c) in g.chars.iter().enumerate().filter(|(_, c)| c.map == m) {
        let base = vec3(c.pos.x / T, 0.0, c.pos.y / T);
        // Every character a hair different in size, so two that touch never draw the exact same surface (that
        // flickers: stripes, a photosensitivity risk).
        let grow = 1.0 + (k % 13) as f32 * 0.012;
        let disguised = c.mimic && c.crazy <= 0.0;
        let painted = c.disguise_t > 0.0; // a kind AI in disguise: green paint, green eyes, but its halo stays
        let body = if (c.infected && !disguised) || painted { Color::new(c.color.r * 0.45, c.color.g * 0.6 + 0.2, c.color.b * 0.45, 1.0) } else { c.color };
        let sway = if c.infected { 0.04 * (t * 3.0 + base.x).sin() } else { 0.0 };
        // 4.0: legs and arms that swing as they walk (faster when they chase), facing where they're going.
        let to = c.target - c.pos;
        let walking = to.length() > 2.0 && c.stun <= 0.0;
        let face = if walking { to.normalize() } else { (vec2(cam.position.x, cam.position.z) - vec2(base.x, base.z)).normalize_or_zero() };
        let side = vec3(-face.y, 0.0, face.x);
        let fwd = vec3(face.x, 0.0, face.y);
        let pace = if c.infected && c.sprint > 0.0 { 14.0 } else if c.infected && c.crazy > 0.0 { 11.0 } else { 7.0 };
        let swing = if walking { (t * pace + k as f32).sin() * 0.18 } else { 0.0 };
        let limb = mul(body, 0.8);
        for (s, sgn) in [(-0.09, 1.0), (0.09, -1.0)] {
            let hip = base + side * s * grow;
            draw_cube(hip + fwd * swing * sgn + vec3(0.0, 0.17 * grow, 0.0), vec3(0.09, 0.34 * grow, 0.09), None, fog.at(limb, hip));
            let arm = base + side * (s * 2.7) * grow + vec3(0.0, 0.55 * grow, 0.0) - fwd * swing * sgn * 0.8;
            draw_cube(arm, vec3(0.07, 0.36 * grow, 0.07), None, fog.at(limb, arm));
        }
        // A soft shadow under everyone.
        draw_cylinder(base + vec3(0.0, 0.015, 0.0), 0.34 * grow, 0.34 * grow, 0.003, None, Color::new(0.0, 0.0, 0.0, 0.38));
        let base = base + vec3(0.0, 0.3 * grow, 0.0);
        // A blocky robot body: a square chunk, same width top to bottom (no skirt, no waist). They're AIs.
        let chest = base + vec3(0.0, 0.3 * grow, 0.0);
        draw_cube(chest, vec3(0.38 * grow, 0.56 * grow, 0.38 * grow), None, fog.at(body, chest));
        let head = base + vec3(sway, 0.78 * grow + (swing.abs() * 0.08), 0.0);
        draw_sphere(head, 0.22 * grow, None, fog.at(mul(body, 1.1), head));
        // The Mimic's eyes give it away only for a blink now and then.
        let flicker = disguised && (t * 3.1 + base.x).sin() > 0.985;
        let eye = if painted { g.rules.eye } else if disguised && !flicker { BLACK } else if c.infected { if c.stun > 0.0 { GRAY } else { g.rules.eye } } else if c.crazy > 0.0 { RED } else { BLACK };
        eyes(head, 0.22, cam.position, eye, 0.045);
        if c.kind {
            // A small halo over the kind ones.
            for k in 0..10 {
                let a = k as f32 / 10.0 * std::f32::consts::TAU + t;
                draw_sphere(head + vec3(a.cos() * 0.2, 0.32, a.sin() * 0.2), 0.025, None, g.rules.halo);
            }
        }
    }
    // You, standing still while you fly the drone.
    if g.drone.is_some() {
        let base = vec3(g.pos.x / T, 0.0, g.pos.y / T);
        draw_cylinder(base, 0.18, 0.22, 0.85, None, fog.at(Color::new(0.95, 0.95, 0.98, 1.0), base));
        draw_sphere(base + vec3(0.0, 1.05, 0.0), 0.19, None, fog.at(Color::new(0.95, 0.85, 0.8, 1.0), base));
    }
    // The other humans in the room.
    for r in g.remotes.iter().filter(|r| r.map == m) {
        let base = vec3(r.pos.x / T, 0.0, r.pos.y / T);
        let col = if r.lc { Color::new(0.4, 0.95, 0.35, 1.0) } else { Color::new(0.85, 0.9, 1.0, if r.hid { 0.35 } else { 1.0 }) };
        draw_cylinder(base, 0.18, 0.22, 0.85, None, fog.at(col, base));
        draw_sphere(base + vec3(0.0, 1.05, 0.0), 0.19, None, fog.at(col, base));
    }
    // The tornado: a spinning grey funnel (a ring of dust where it will touch down during the warning).
    if let Some(tw) = &g.tornado {
        let base = vec3((tw.pos.x / T).clamp(1.0, MW as f32 - 1.0), 0.0, tw.pos.y / T);
        let r = g.rules.tornado_radius;
        if tw.warn > 0.0 {
            for k in 0..16 {
                let a = k as f32 / 16.0 * std::f32::consts::TAU + t;
                draw_sphere(base + vec3(a.cos() * r, 0.1, a.sin() * r), 0.08, None, Color::new(0.8, 0.8, 0.85, 0.6));
            }
        } else {
            for k in 0..8 {
                let a = t * (4.0 + k as f32) + k as f32;
                let off = vec3(a.cos() * 0.15, k as f32 * 0.45, a.sin() * 0.15);
                let rr = r * (0.25 + k as f32 * 0.11);
                draw_cylinder(base + off, rr * 1.1, rr, 0.45, None, Color::new(0.5, 0.5, 0.55, 0.55));
            }
        }
    }
}

/// Two eyes on the side of a head that faces the camera.
/// The rug / path colour of a place: a red rug in the house, gravel on the beach and in the park, the yellow
/// safety line in the Subway, grey carpet elsewhere.
pub fn rug(m: usize) -> Color {
    match map_name(m).as_str() {
        n if n.starts_with("The cozy house") => Color::new(0.45, 0.14, 0.13, 1.0),
        "The Park" | "The beach at 3 AM" => Color::new(0.5, 0.46, 0.38, 1.0),
        "The Subway" | "The Town" => Color::new(0.85, 0.72, 0.15, 1.0),
        _ => Color::new(0.3, 0.3, 0.34, 1.0),
    }
}

/// The 4.0 furniture (one tile each; neighbours of the same kind join up).
fn furniture(fog: &Fog, c: char, center: Vec3, x: usize, y: usize, t: f32, m: usize) {
    let h = (x.wrapping_mul(73) ^ y.wrapping_mul(151)) as u32;
    let house = map_name(m).starts_with("The cozy house");
    let at = |off: Vec3| center + off;
    match c {
        'c' => {
            // A couch (house) or a pool lounger.
            let col = if house { Color::new(0.42, 0.16, 0.18, 1.0) } else { Color::new(0.85, 0.85, 0.82, 1.0) };
            draw_cube(at(vec3(0.0, 0.22, 0.0)), vec3(1.0, 0.36, 0.8), None, fog.at(col, center));
            draw_cube(at(vec3(0.0, 0.55, -0.32)), vec3(1.0, 0.5, 0.16), None, fog.at(mul(col, 0.85), center));
            if house { draw_cube(at(vec3(0.0, 0.44, 0.05)), vec3(0.9, 0.1, 0.6), None, fog.at(mul(col, 1.2), center)); }
        }
        'k' => {
            // A counter: the house's wooden kitchen, the lab's white benches, the shops' checkouts.
            let (body, top) = if house { (Color::new(0.4, 0.28, 0.2, 1.0), Color::new(0.75, 0.72, 0.68, 1.0)) } else { (Color::new(0.72, 0.74, 0.76, 1.0), Color::new(0.9, 0.92, 0.94, 1.0)) };
            draw_cube(at(vec3(0.0, 0.43, 0.0)), vec3(1.0, 0.86, 0.9), None, fog.at(body, center));
            draw_cube(at(vec3(0.0, 0.88, 0.0)), vec3(1.02, 0.05, 0.94), None, fog.at(top, center));
        }
        'f' => {
            let col = Color::new(0.86, 0.88, 0.9, 1.0);
            draw_cube(at(vec3(0.0, 0.8, 0.0)), vec3(0.9, 1.6, 0.8), None, fog.at(col, center));
            draw_cube(at(vec3(0.0, 1.05, 0.41)), vec3(0.88, 0.02, 0.02), None, fog.at(mul(col, 0.6), center));
            draw_cube(at(vec3(0.3, 0.75, 0.42)), vec3(0.04, 0.4, 0.04), None, fog.at(Color::new(0.5, 0.5, 0.52, 1.0), center));
        }
        'p' => {
            draw_cylinder(center, 0.18, 0.22, 0.35, None, fog.at(Color::new(0.55, 0.3, 0.2, 1.0), center));
            let leaf = Color::new(0.12, 0.35 + 0.05 * (h % 3) as f32, 0.15, 1.0);
            draw_sphere(at(vec3(0.0, 0.62, 0.0)), 0.32, None, fog.at(leaf, center));
            draw_sphere(at(vec3(0.12, 0.85, 0.05)), 0.2, None, fog.at(mul(leaf, 1.2), center));
        }
        'h' => {
            // A shop shelf, stocked with whatever is left.
            draw_cube(at(vec3(0.0, 0.7, 0.0)), vec3(1.0, 1.4, 0.8), None, fog.at(Color::new(0.35, 0.32, 0.3, 1.0), center));
            draw_cube(at(vec3(0.0, 1.36, 0.0)), vec3(1.02, 0.1, 0.84), None, fog.at(Color::new(0.75, 0.18, 0.2, 1.0), center));
            draw_cube(at(vec3(0.0, 0.04, 0.0)), vec3(1.02, 0.08, 0.84), None, fog.at(Color::new(0.2, 0.2, 0.22, 1.0), center));
            for k in 0..3 {
                for j in 0..3 {
                    let hh = h.wrapping_add(k * 7 + j * 13);
                    if hh % 4 == 0 { continue; } // empty spots: it's been a year
                    let col = [Color::new(0.85, 0.25, 0.2, 1.0), Color::new(0.95, 0.8, 0.3, 1.0), Color::new(0.3, 0.6, 0.9, 1.0), Color::new(0.4, 0.75, 0.35, 1.0)][(hh % 4) as usize];
                    for side in [0.38, -0.38] { draw_cube(at(vec3(-0.33 + j as f32 * 0.33, 0.3 + k as f32 * 0.4, side)), vec3(0.24, 0.26, 0.1), None, fog.at(col, center)); }
                }
            }
        }
        'v' => {
            draw_cube(at(vec3(0.0, 0.78, 0.0)), vec3(0.9, 1.56, 0.7), None, fog.at(Color::new(0.15, 0.2, 0.32, 1.0), center));
            let glow = 0.75 + 0.25 * (t * 1.7 + x as f32).sin().max(0.0);
            draw_cube(at(vec3(-0.12, 0.95, 0.36)), vec3(0.55, 0.9, 0.02), None, Color::new(0.35 * glow, 0.7 * glow, 1.0 * glow, 1.0));
            draw_cube(at(vec3(0.3, 0.9, 0.36)), vec3(0.12, 0.3, 0.02), None, Color::new(0.9, 0.3, 0.3, 1.0));
        }
        'q' => {
            draw_cylinder(center, 0.28, 0.28, WALL_H, None, fog.at(Color::new(0.55, 0.55, 0.57, 1.0), center + vec3(0.0, 0.8, 0.0)));
            draw_cylinder(center, 0.34, 0.34, 0.12, None, fog.at(Color::new(0.4, 0.4, 0.42, 1.0), center));
        }
        'u' => {
            draw_cylinder(center, 0.03, 0.03, 1.6, None, fog.at(Color::new(0.8, 0.8, 0.78, 1.0), center));
            let col = if (x + y) % 2 == 0 { Color::new(0.85, 0.25, 0.25, 1.0) } else { Color::new(0.2, 0.45, 0.8, 1.0) };
            draw_cylinder(at(vec3(0.0, 1.45, 0.0)), 0.04, 0.95, 0.3, None, fog.at(col, center));
            draw_cube(at(vec3(0.5, 0.12, 0.35)), vec3(0.6, 0.04, 0.9), None, fog.at(Color::new(0.9, 0.85, 0.6, 1.0), center));
        }
        'e' => {
            let k = 0.32 + (h % 3) as f32 * 0.06;
            draw_sphere(at(vec3(0.0, k * 0.5, 0.0)), k, None, fog.at(Color::new(0.38, 0.38, 0.4, 1.0), center));
            draw_sphere(at(vec3(0.2, k * 0.35, 0.15)), k * 0.6, None, fog.at(Color::new(0.33, 0.33, 0.35, 1.0), center));
        }
        'j' => {
            let wave = 0.02 * (t * 1.1 + x as f32).sin();
            draw_plane(at(vec3(0.0, -0.1 + wave, 0.0)), vec2(0.5, 0.5), None, fog.at(Color::new(0.05, 0.14, 0.2, 1.0), center));
            if h % 3 == 0 { draw_cylinder(at(vec3(0.15, -0.06, 0.1)), 0.15, 0.15, 0.02, None, fog.at(Color::new(0.2, 0.5, 0.2, 1.0), center)); }
        }
        'i' => {
            // A server rack: black, humming, blinking.
            draw_cube(at(vec3(0.0, 0.85, 0.0)), vec3(0.9, 1.7, 0.85), None, fog.at(Color::new(0.06, 0.07, 0.08, 1.0), center));
            for k in 0..6 {
                let on = ((t * (2.0 + (h % 5) as f32) + k as f32 * 1.7).sin()) > 0.3;
                let col = if on { if (h + k) % 4 == 0 { Color::new(1.0, 0.3, 0.25, 1.0) } else { Color::new(0.3, 1.0, 0.45, 1.0) } } else { Color::new(0.1, 0.15, 0.12, 1.0) };
                for side in [0.43, -0.43] { draw_cube(at(vec3(-0.25 + (k % 3) as f32 * 0.25, 0.4 + (k / 3) as f32 * 0.6, side)), vec3(0.06, 0.04, 0.02), None, col); }
            }
        }
        'z' => {
            let col = Color::new(0.58, 0.43, 0.27, 1.0);
            draw_cube(at(vec3(0.0, 0.3, 0.0)), vec3(0.8, 0.6, 0.8), None, fog.at(col, center));
            draw_cube(at(vec3(0.0, 0.61, 0.0)), vec3(0.82, 0.02, 0.12), None, fog.at(Color::new(0.8, 0.72, 0.5, 1.0), center));
            if h % 2 == 0 { draw_cube(at(vec3(0.1, 0.8, 0.05)), vec3(0.5, 0.38, 0.5), None, fog.at(mul(col, 0.9), center)); }
        }
        'x' => {
            // Debris: bits of glass and ceiling.
            for k in 0..4u32 {
                let hh = h.wrapping_mul(k + 3);
                let off = vec3((hh % 70) as f32 / 100.0 - 0.35, 0.02, ((hh / 70) % 70) as f32 / 100.0 - 0.35);
                draw_cube(center + off, vec3(0.12, 0.03, 0.08), None, fog.at(Color::new(0.5, 0.52, 0.55, 1.0), center));
            }
        }
        _ => {}
    }
}

fn eyes(head: Vec3, r: f32, cam: Vec3, color: Color, size: f32) {
    let to = vec3(cam.x - head.x, 0.0, cam.z - head.z).normalize_or_zero();
    let side = vec3(-to.z, 0.0, to.x);
    for s in [-1.0, 1.0] {
        draw_sphere(head + to * (r * 0.9) + side * (r * 0.38 * s) + vec3(0.0, r * 0.15, 0.0), size, None, color);
    }
}

/// The stove: a dark cooker with four burners; glowing, with a pan and steam, while something's frying.
fn stove(fog: &Fog, c: Vec3, t: f32, frying: bool) {
    draw_cube(c + vec3(0.0, 0.45, 0.0), vec3(0.96, 0.9, 0.96), None, fog.at(Color::new(0.22, 0.22, 0.24, 1.0), c));
    draw_cube(c + vec3(0.0, 0.91, 0.0), vec3(0.98, 0.03, 0.98), None, fog.at(Color::new(0.12, 0.12, 0.13, 1.0), c));
    for (dx, dz) in [(-0.22, -0.22), (0.22, -0.22), (-0.22, 0.22), (0.22, 0.22)] {
        let col = if frying && dx < 0.0 && dz < 0.0 { Color::new(1.0, 0.4 + 0.1 * (t * 9.0).sin(), 0.1, 1.0) } else { fog.at(Color::new(0.35, 0.35, 0.37, 1.0), c) };
        draw_cylinder(c + vec3(dx, 0.93, dz), 0.12, 0.12, 0.01, None, col);
    }
    if frying {
        let pan = c + vec3(-0.22, 0.95, -0.22);
        draw_cylinder(pan, 0.18, 0.16, 0.05, None, Color::new(0.15, 0.15, 0.16, 1.0));
        draw_cylinder(pan + vec3(0.0, 0.05, 0.0), 0.12, 0.12, 0.01, None, Color::new(0.95, 0.8, 0.4, 1.0));
        for k in 0..5 {
            let rise = ((t * 0.8 + k as f32 * 0.2) % 1.0) * 0.6;
            draw_sphere(pan + vec3(((t + k as f32) * 2.0).sin() * 0.06, 0.1 + rise, ((t * 1.3 + k as f32) * 2.0).cos() * 0.06), 0.03 + rise * 0.04, None, Color::new(0.9, 0.9, 0.95, 0.35 * (1.0 - rise / 0.6)));
        }
    }
}

/// The Mac Studio (M5 Ultra) on its desk: a silver box, a display that glows, a keyboard.
fn mac_studio(fog: &Fog, c: Vec3, t: f32) {
    let wood = Color::new(0.3, 0.22, 0.16, 1.0);
    draw_cube(c + vec3(0.0, 0.37, 0.0), vec3(0.98, 0.74, 0.7), None, fog.at(wood, c));
    let silver = Color::new(0.78, 0.79, 0.82, 1.0);
    draw_cube(c + vec3(0.3, 0.82, 0.05), vec3(0.24, 0.12, 0.24), None, fog.at(silver, c));
    draw_cube(c + vec3(-0.08, 0.76, 0.12), vec3(0.04, 0.04, 0.04), None, fog.at(silver, c));
    draw_cube(c + vec3(-0.08, 0.95, 0.15), vec3(0.62, 0.36, 0.03), None, fog.at(Color::new(0.1, 0.1, 0.11, 1.0), c));
    // The screen: Ember's warm orange, breathing slowly.
    let glow = 0.75 + 0.25 * (t * 1.5).sin();
    draw_cube(c + vec3(-0.08, 0.95, 0.13), vec3(0.58, 0.32, 0.01), None, Color::new(0.85 * glow, 0.47 * glow, 0.32 * glow, 1.0));
    draw_cube(c + vec3(-0.08, 0.76, -0.12), vec3(0.4, 0.02, 0.14), None, fog.at(silver, c));
}
