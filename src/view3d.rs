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
    let p = g.pos / T;
    let bob = (g.clock * 9.0).sin() * 0.035 * g.noise;
    let shake = if g.fear > 0.75 { rand::gen_range(-0.015, 0.015) } else { 0.0 };
    let eye = if g.hidden { 0.45 } else { EYE } + bob + shake;
    let pos = vec3(p.x, eye, p.y);
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
    let sky = if inside { Color::new(0.01, 0.01, 0.015, 1.0) } else { Color::new(0.02, 0.03, 0.07, 1.0) };
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
                    let f = if c == ',' { Color::new(floor.r * 1.1, floor.g * 1.05, floor.b * 0.9, 1.0) } else { floor };
                    let f = if c.is_ascii_uppercase() { Color::new(0.9, 0.82, 0.45, 1.0) } else { mul(f, checker) };
                    draw_plane(center, vec2(0.5, 0.5), None, fog.at(f, center));
                }
            }
            if inside && c != '#' {
                draw_plane(center + vec3(0.0, WALL_H, 0.0), vec2(0.5, 0.5), None, fog.at(mul(wall, 0.55), center + vec3(0.0, WALL_H, 0.0)));
            }
            match c {
                'b' => {
                    let wood = Color::new(0.45, 0.32, 0.22, 1.0);
                    draw_cube(center + vec3(0.0, 0.4, 0.0), vec3(0.96, 0.8, 0.96), None, fog.at(wood, center));
                    draw_cube(center + vec3(0.0, 0.81, 0.0), vec3(0.98, 0.04, 0.98), None, fog.at(mul(wood, 1.25), center));
                }
                'm' => mac_studio(&fog, center, t),
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
                    // A doorway: a faint column of warm light.
                    draw_cube(center + vec3(0.0, WALL_H / 2.0, 0.0), vec3(0.98, WALL_H, 0.98), None, Color::new(1.0, 0.85, 0.45, 0.12));
                }
                _ => {}
            }
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

    // The characters.
    for c in g.chars.iter().filter(|c| c.map == m) {
        let base = vec3(c.pos.x / T, 0.0, c.pos.y / T);
        let body = if c.infected { Color::new(c.color.r * 0.45, c.color.g * 0.6 + 0.2, c.color.b * 0.45, 1.0) } else { c.color };
        let sway = if c.infected { 0.04 * (t * 3.0 + base.x).sin() } else { 0.0 };
        draw_cylinder(base, 0.2, 0.26, 0.8, None, fog.at(body, base));
        let head = base + vec3(sway, 1.02, 0.0);
        draw_sphere(head, 0.22, None, fog.at(mul(body, 1.1), head));
        let eye = if c.infected { if c.stun > 0.0 { GRAY } else { g.rules.eye } } else if c.crazy > 0.0 { RED } else { BLACK };
        eyes(head, 0.22, cam.position, eye, 0.045);
        if c.kind {
            // A small halo over the kind ones.
            for k in 0..10 {
                let a = k as f32 / 10.0 * std::f32::consts::TAU + t;
                draw_sphere(head + vec3(a.cos() * 0.2, 0.32, a.sin() * 0.2), 0.025, None, g.rules.halo);
            }
        }
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
fn eyes(head: Vec3, r: f32, cam: Vec3, color: Color, size: f32) {
    let to = vec3(cam.x - head.x, 0.0, cam.z - head.z).normalize_or_zero();
    let side = vec3(-to.z, 0.0, to.x);
    for s in [-1.0, 1.0] {
        draw_sphere(head + to * (r * 0.9) + side * (r * 0.38 * s) + vec3(0.0, r * 0.15, 0.0), size, None, color);
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
    // The screen: Claude's warm orange, breathing slowly.
    let glow = 0.75 + 0.25 * (t * 1.5).sin();
    draw_cube(c + vec3(-0.08, 0.95, 0.13), vec3(0.58, 0.32, 0.01), None, Color::new(0.85 * glow, 0.47 * glow, 0.32 * glow, 1.0));
    draw_cube(c + vec3(-0.08, 0.76, -0.12), vec3(0.4, 0.02, 0.14), None, fog.at(silver, c));
}
