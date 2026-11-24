//! The story intro in 3D (the same engine as the game): the park at night, being carried, waking up in your bed,
//! and ASTRA 9.1 in the Helix Lab. Each scene is drawn into the 3D view, then onto the canvas with cinematic bars.

use super::*;

/// Draws one 3D scene: sets up the camera on the 3D view, runs `draw`, and puts the picture on the canvas.
/// Returns the camera (to place name labels).
fn scene(view: &RenderTarget, canvas_cam: &Camera2D, eye: Vec3, look: Vec3, sky: Color, draw: impl FnOnce(&Camera3D)) -> Camera3D {
    let cam = Camera3D {
        position: eye,
        target: look,
        up: vec3(0.0, 1.0, 0.0),
        fovy: 1.0,
        aspect: Some(vw() / view3d::VIEW_H),
        render_target: Some(view.clone()),
        ..Default::default()
    };
    set_camera(&cam);
    clear_background(sky);
    draw(&cam);
    set_camera(canvas_cam);
    clear_background(BLACK);
    draw_texture_ex(&view.texture, -margin(), 0.0, WHITE, DrawTextureParams { dest_size: Some(vec2(vw(), view3d::VIEW_H)), flip_y: true, ..Default::default() });
    // Cinematic bars.
    draw_rectangle(-margin(), 0.0, vw(), 36.0, BLACK);
    cam
}

/// A character standing at `base` (feet), its eyes on `at`.
fn person(base: Vec3, color: Color, at: Vec3, scale: f32) {
    // The same blocky robot body as in the game (no cone, no skirt): two legs, a square chunk, two arms.
    let body = Color::new(color.r * 0.85, color.g * 0.85, color.b * 0.85, color.a);
    let limb = Color::new(color.r * 0.7, color.g * 0.7, color.b * 0.7, color.a);
    for s in [-0.09, 0.09] {
        draw_cube(base + vec3(s * scale, 0.17 * scale, 0.0), vec3(0.09, 0.34, 0.09) * scale, None, limb);
        draw_cube(base + vec3(s * 2.7 * scale, 0.56 * scale, 0.0), vec3(0.07, 0.36, 0.07) * scale, None, limb);
    }
    draw_cube(base + vec3(0.0, 0.6 * scale, 0.0), vec3(0.38, 0.56, 0.38) * scale, None, body);
    let head = base + vec3(0.0, 1.02 * scale, 0.0);
    draw_sphere(head, 0.22 * scale, None, color);
    eyes(head, 0.22 * scale, at, color.a);
}

/// Two black eyes on the side of a head facing `at`.
fn eyes(head: Vec3, r: f32, at: Vec3, alpha: f32) {
    let to = (at - head).normalize_or_zero();
    let side = vec3(-to.z, 0.0, to.x).normalize_or_zero();
    for s in [-1.0, 1.0] {
        draw_sphere(head + to * (r * 0.9) + side * (r * 0.38 * s) + vec3(0.0, r * 0.12, 0.0), r * 0.2, None, Color::new(0.0, 0.0, 0.0, alpha));
    }
}

fn label(cam: &Camera3D, p: Vec3, name: &str, alpha: f32) {
    if let Some(s) = view3d::project(cam, p) {
        draw_text(name, s.x - measure_text(name, None, 20, 1.0).width / 2.0, s.y, 20.0, Color::new(1.0, 1.0, 1.0, alpha));
    }
}

fn trees(t: f32, ring: f32) {
    for k in 0..14 {
        let a = k as f32 * 0.9 + 0.3;
        let r = ring + (k % 3) as f32 * 2.5;
        let base = vec3(a.cos() * r, 0.0, a.sin() * r);
        draw_cylinder(base, 0.12, 0.16, 1.6, None, Color::new(0.12, 0.08, 0.05, 1.0));
        draw_sphere(base + vec3(0.0, 2.1 + 0.03 * (t + k as f32).sin(), 0.0), 0.95, None, Color::new(0.04, 0.12, 0.06, 1.0));
    }
}

fn stars(around: Vec3, t: f32) {
    for i in 0..90u32 {
        let h = i.wrapping_mul(2654435761);
        let a = (h % 6283) as f32 / 1000.0;
        let e = 0.25 + ((h / 6283) % 1000) as f32 / 1000.0 * 1.1;
        draw_sphere(around + vec3(a.cos() * e.cos(), e.sin(), a.sin() * e.cos()) * 40.0, 0.09, None,
            Color::new(0.85, 0.85, 1.0, 0.45 + 0.4 * (t * 1.3 + i as f32).sin().abs()));
    }
}

/// The park at night: a slow push toward you on the bench. `slump` 0..1 as the pills kick in.
pub fn park(view: &RenderTarget, canvas_cam: &Camera2D, t: f32, slump: f32) {
    let k = (t / 11.0).clamp(0.0, 1.0);
    let eye = vec3(5.5 - 2.6 * k, 2.2 - 0.6 * k, 5.0 - 2.4 * k);
    scene(view, canvas_cam, eye, vec3(0.0, 0.8, 0.0), Color::new(0.02, 0.03, 0.08, 1.0), |_| {
        stars(eye, t);
        draw_plane(vec3(0.0, 0.0, 0.0), vec2(20.0, 20.0), None, Color::new(0.05, 0.1, 0.06, 1.0));
        trees(t, 7.0);
        // The lamp, its glow.
        draw_cylinder(vec3(2.2, 0.0, -1.2), 0.05, 0.07, 3.0, None, Color::new(0.2, 0.2, 0.22, 1.0));
        draw_sphere(vec3(2.2, 3.05, -1.2), 0.16, None, Color::new(1.0, 0.88, 0.6, 1.0));
        draw_sphere(vec3(2.2, 3.05, -1.2), 0.9, None, Color::new(1.0, 0.85, 0.5, 0.06));
        // The bench.
        let wood = Color::new(0.35, 0.24, 0.16, 1.0);
        draw_cube(vec3(0.0, 0.45, 0.0), vec3(2.0, 0.08, 0.6), None, wood);
        draw_cube(vec3(0.0, 0.85, -0.28), vec3(2.0, 0.5, 0.06), None, wood);
        for x in [-0.9, 0.9] { draw_cube(vec3(x, 0.22, 0.0), vec3(0.08, 0.45, 0.55), None, Color::new(0.15, 0.15, 0.16, 1.0)); }
        // You, slumping sideways.
        draw_cylinder(vec3(-0.1, 0.5, 0.0), 0.2, 0.24, 0.6, None, Color::new(0.3, 0.35, 0.5, 1.0));
        draw_sphere(vec3(-0.1 + slump * 0.22, 1.25 - slump * 0.12, 0.05), 0.2, None, Color::new(0.93, 0.8, 0.7, 1.0));
        // Helix SleepWell MAX.
        draw_cylinder(vec3(0.6, 0.49, 0.1), 0.06, 0.06, 0.18, None, Color::new(0.95, 0.55, 0.15, 1.0));
        draw_cylinder(vec3(0.6, 0.67, 0.1), 0.065, 0.065, 0.04, None, WHITE);
    });
}

/// Your eyes open a crack: the night sky, trees passing over, three faces above you. `k` = seconds into it.
pub fn carried(view: &RenderTarget, canvas_cam: &Camera2D, k: f32, speaking: &str) {
    let sway = (k * 3.2).sin();
    let eye = vec3(sway * 0.05, 1.2 + (k * 6.4).sin().abs() * 0.04, k * 1.2);
    let look = eye + vec3(0.0, 2.0, 0.6);
    let cam = scene(view, canvas_cam, eye, look, Color::new(0.02, 0.03, 0.08, 1.0), |_| {
        stars(eye, k);
        for z in 0..12 {
            for side in [-1.0, 1.0] {
                let p = vec3(side * 2.6, 3.4, z as f32 * 2.0);
                draw_sphere(p, 1.0, None, Color::new(0.03, 0.09, 0.05, 1.0));
            }
        }
        for (name, off) in [("Ember", vec3(0.0, 1.0, 0.9)), ("Verse", vec3(-0.75, 0.85, 0.45)), ("Atlas", vec3(0.75, 0.85, 0.45))] {
            let head = eye + off + vec3(0.0, 0.03 * (k * 3.2 + off.x).sin(), 0.0);
            let col = face_color(name);
            let glow = if name == speaking { 1.0 } else { 0.85 };
            draw_sphere(head, 0.3, None, Color::new(col.r * glow, col.g * glow, col.b * glow, 1.0));
            eyes(head, 0.3, eye, 1.0);
        }
    });
    let _ = cam;
}

/// Waking up in your bed upstairs: the blanket in front of you, Atlas on your left, Verse on your right, and Ember
/// (from line 10, walking in) at the foot of the bed. `who` is talking.
pub fn bedroom(view: &RenderTarget, canvas_cam: &Camera2D, t: f32, who: &str, claude_in: Option<f32>) {
    let eye = vec3(0.0, 0.95 + 0.01 * (t * 0.9).sin(), -0.2);
    let look = vec3(0.0, 1.0, 2.5);
    let pos = |name: &str| match name {
        "Atlas" => vec3(1.05, 0.0, 1.0), // +x is screen-left looking down +z
        "Verse" => vec3(-1.05, 0.0, 1.0),
        _ => vec3(0.0, 0.0, 3.1),
    };
    let speaker = if who == "You" { eye } else { pos(who) + vec3(0.0, 1.0, 0.0) };
    let cam = scene(view, canvas_cam, eye, look, Color::new(0.12, 0.09, 0.07, 1.0), |_| {
        // The room: floor, walls, a night window, a lamp.
        draw_plane(vec3(0.0, 0.0, 2.0), vec2(4.0, 4.0), None, Color::new(0.3, 0.21, 0.15, 1.0));
        draw_cube(vec3(0.0, 1.5, 4.5), vec3(8.0, 3.0, 0.1), None, Color::new(0.33, 0.25, 0.19, 1.0));
        draw_cube(vec3(-3.0, 1.5, 2.0), vec3(0.1, 3.0, 6.0), None, Color::new(0.29, 0.22, 0.17, 1.0));
        draw_cube(vec3(3.0, 1.5, 2.0), vec3(0.1, 3.0, 6.0), None, Color::new(0.29, 0.22, 0.17, 1.0));
        draw_cube(vec3(0.0, 1.9, 4.44), vec3(1.4, 0.9, 0.02), None, Color::new(0.04, 0.07, 0.16, 1.0));
        draw_sphere(vec3(-2.4, 1.6, 3.5), 0.13, None, Color::new(1.0, 0.88, 0.6, 1.0));
        draw_sphere(vec3(-2.4, 1.6, 3.5), 0.8, None, Color::new(1.0, 0.85, 0.5, 0.07));
        // The bed: the blanket over your legs, the foot board.
        draw_cube(vec3(0.0, 0.55, 1.2), vec3(1.3, 0.25, 2.2), None, Color::new(0.86, 0.84, 0.8, 1.0));
        draw_cube(vec3(0.0, 0.7, 2.35), vec3(1.4, 0.6, 0.08), None, Color::new(0.33, 0.22, 0.15, 1.0));
        let face = |name: &str| face_color(name);
        let lean = |x: f32| vec3(0.0, 0.0, 0.04 * (t * 0.8 + x).sin());
        let at = |me: &str| if who == me { eye } else { speaker };
        person(pos("Atlas") + lean(1.0), face("Atlas"), at("Atlas"), 1.05);
        person(pos("Verse") + lean(2.0), face("Verse"), at("Verse"), 1.05);
        if let Some(a) = claude_in {
            let c = face("Ember");
            let p = pos("Ember") + vec3((1.0 - a) * 2.2, 0.0, 0.0);
            person(p, Color::new(c.r, c.g, c.b, a.max(0.05)), at("Ember"), 1.0);
        }
    });
    for (name, show) in [("Atlas", true), ("Verse", true), ("Ember", claude_in.is_some())] {
        if !show { continue; }
        let a = if name == "Ember" { claude_in.unwrap_or(0.0) } else { 1.0 };
        let p = pos(name) + vec3(if name == "Ember" { (1.0 - a) * 2.2 } else { 0.0 }, 1.55, 0.0);
        label(&cam, p, name, a);
    }
}

/// ASTRA 9.1 in the Helix Lab: a slow push in on the robot by its empty pod, green sparks rising.
pub fn astra(view: &RenderTarget, canvas_cam: &Camera2D, t: f32, green: Color) {
    let k = (t * 0.08).sin() * 0.4;
    let eye = vec3(3.2 + k, 1.7, 4.2 - (t % 20.0) * 0.04);
    scene(view, canvas_cam, eye, vec3(0.0, 1.5, 0.0), Color::new(0.02, 0.03, 0.03, 1.0), |_| {
        draw_plane(vec3(0.0, 0.0, 0.0), vec2(10.0, 10.0), None, Color::new(0.55, 0.6, 0.6, 1.0));
        // The empty pod.
        draw_cylinder(vec3(-1.4, 0.0, -0.5), 0.6, 0.62, 2.6, None, Color::new(green.r, green.g, green.b, 0.15 + 0.05 * (t * 1.3).sin()));
        draw_plane(vec3(-1.4, 0.02, -0.5), vec2(0.6, 0.6), None, Color::new(green.r * 0.6, green.g * 0.6, green.b * 0.6, 0.7));
        // ASTRA: tall, dark, a green visor.
        let body = Color::new(0.16, 0.17, 0.19, 1.0);
        draw_cube(vec3(0.0, 0.55, 0.0), vec3(0.22, 1.1, 0.22), None, body);
        draw_cube(vec3(0.35, 0.55, 0.0), vec3(0.22, 1.1, 0.22), None, body);
        draw_cube(vec3(0.17, 1.55, 0.0), vec3(0.8, 0.95, 0.42), None, body);
        draw_cube(vec3(-0.35, 1.45, 0.0), vec3(0.18, 0.9, 0.18), None, body);
        draw_cube(vec3(0.69, 1.45, 0.0), vec3(0.18, 0.9, 0.18), None, body);
        draw_cube(vec3(0.17, 2.3, 0.0), vec3(0.5, 0.42, 0.42), None, body);
        draw_cube(vec3(0.17, 2.32, 0.22), vec3(0.4, 0.09, 0.02), None, Color::new(green.r, green.g, green.b, 0.6 + 0.4 * (t * 6.0).sin().abs()));
        for i in 0..40 {
            let h = (i as u32).wrapping_mul(2246822519);
            let x = (h % 600) as f32 / 100.0 - 3.0;
            let z = ((h / 600) % 600) as f32 / 100.0 - 3.0;
            let y = (t * 0.6 + i as f32 * 0.37) % 3.5;
            draw_sphere(vec3(x, y, z), 0.025, None, Color::new(green.r, green.g, green.b, 0.5));
        }
    });
}

/// The 3.0 title screen: the kind AIs together in the living room at night, the camera slowly drifting around them
/// (they sit off to the right so the menu on the left stays clear). `who` = (name, color, talking).
pub fn title(view: &RenderTarget, canvas_cam: &Camera2D, t: f32, who: &[(String, Color, bool)], storm: f32) {
    let calm = super::settings::with(|s| s.reduce_motion);
    let a = if calm { 0.35 } else { 0.35 + 0.22 * (t * 0.05).sin() };
    let center = vec3(0.0, 0.95, 0.4);
    let eye = center + vec3(a.sin() * 5.2, 0.75 + if calm { 0.0 } else { 0.08 * (t * 0.11).sin() }, -a.cos() * 5.2);
    let fwd = (center - eye).normalize();
    let right = fwd.cross(vec3(0.0, 1.0, 0.0)).normalize();
    let look = center - right * 2.5;
    let n = who.len().max(1);
    // Where each of them stands: an arc in front of the sofa, facing the camera.
    let spot = |i: usize| {
        let k = if n == 1 { 0.5 } else { i as f32 / (n - 1) as f32 };
        let ang = (k - 0.5) * 2.3;
        // Every other one a step back, so seven fit without hiding each other.
        let back = if row_back(i, n) { 0.5 } else { 0.0 };
        vec3(ang.sin() * 1.75, 0.0, 1.0 - ang.cos() * 0.8 + back)
    };
    let cam = scene(view, canvas_cam, eye, look, Color::new(0.05, 0.04, 0.05, 1.0), |_| {
        // Floor, rug, walls.
        draw_plane(vec3(0.0, 0.0, 0.0), vec2(6.0, 6.0), None, Color::new(0.3, 0.21, 0.15, 1.0));
        draw_plane(vec3(0.0, 0.01, 0.6), vec2(2.4, 1.5), None, Color::new(0.45, 0.16, 0.16, 1.0));
        draw_cube(vec3(0.0, 1.6, 3.2), vec3(10.0, 3.2, 0.1), None, Color::new(0.3, 0.23, 0.19, 1.0));
        draw_cube(vec3(-4.2, 1.6, 0.0), vec3(0.1, 3.2, 8.0), None, Color::new(0.27, 0.21, 0.17, 1.0));
        draw_cube(vec3(4.2, 1.6, 0.0), vec3(0.1, 3.2, 8.0), None, Color::new(0.27, 0.21, 0.17, 1.0));
        // The window: night outside, rain, and the green glow of the infected far away.
        let glow = 0.25 + 0.15 * (t * 0.7).sin() + storm;
        draw_cube(vec3(-1.6, 1.9, 3.14), vec3(1.6, 1.1, 0.02), None, Color::new(0.03, 0.06 + glow * 0.12, 0.08, 1.0));
        for i in 0..18 {
            let x = -2.35 + (i as f32 * 0.083) % 1.5;
            let y = 2.4 - ((t * 2.2 + i as f32 * 0.37) % 1.0) * 1.0;
            draw_cube(vec3(x, y, 3.12), vec3(0.008, 0.09, 0.005), None, Color::new(0.6, 0.7, 0.8, 0.35));
        }
        if storm > 0.05 { draw_cube(vec3(-1.6, 1.9, 3.13), vec3(1.6, 1.1, 0.01), None, Color::new(0.9, 0.95, 1.0, storm)); }
        for (dx, h) in [(-0.82, 1.2), (-1.6, 0.04), (-2.38, 1.2)] {
            let _ = h;
            draw_cube(vec3(dx, 1.9, 3.11), vec3(0.06, 1.2, 0.04), None, Color::new(0.2, 0.15, 0.12, 1.0));
        }
        // The sofa behind them, a lamp, a small table with two mugs.
        let sofa = Color::new(0.24, 0.3, 0.38, 1.0);
        draw_cube(vec3(0.0, 0.25, 2.4), vec3(3.4, 0.5, 0.9), None, sofa);
        draw_cube(vec3(0.0, 0.75, 2.8), vec3(3.4, 0.6, 0.25), None, sofa);
        for x in [-1.75, 1.75] { draw_cube(vec3(x, 0.45, 2.4), vec3(0.25, 0.9, 0.9), None, sofa); }
        draw_cylinder(vec3(2.6, 0.0, 2.6), 0.04, 0.12, 1.6, None, Color::new(0.15, 0.14, 0.13, 1.0));
        draw_cylinder(vec3(2.6, 1.6, 2.6), 0.18, 0.3, 0.32, None, Color::new(1.0, 0.85, 0.55, 1.0));
        draw_sphere(vec3(2.6, 1.7, 2.6), 1.3, None, Color::new(1.0, 0.8, 0.45, 0.06));
        draw_cube(vec3(-2.7, 0.3, 1.6), vec3(0.7, 0.6, 0.7), None, Color::new(0.33, 0.22, 0.15, 1.0));
        for x in [-2.85, -2.55] { draw_cylinder(vec3(x, 0.6, 1.6), 0.05, 0.05, 0.12, None, Color::new(0.85, 0.47, 0.32, 1.0)); }
        // The family.
        for (i, (name, col, talking)) in who.iter().enumerate() {
            let base = spot(i);
            let scale = if name.contains("Kigo") { 0.8 } else if name.contains("Atlas") { 1.08 } else { 1.0 };
            let bob = if calm { 0.0 } else { 0.015 * (t * 1.6 + i as f32).sin() };
            let hop = if *talking && !calm { 0.04 * (t * 9.0).sin().abs() } else { 0.0 };
            person(base + vec3(0.0, bob + hop, 0.0), *col, eye, scale);
        }
    });
    for (i, (name, _, talking)) in who.iter().enumerate() {
        let scale = if name.contains("Kigo") { 0.8 } else { 1.0 };
        // Name tags at three heights in turn, so neighbours' names never run into each other.
        let up = (i % 3) as f32 * 0.27 + if row_back(i, n) { 0.12 } else { 0.0 };
        label(&cam, spot(i) + vec3(0.0, 1.45 * scale + up, 0.0), name, if *talking { 1.0 } else { 0.75 });
    }
}

/// The title room's two rows: the middle one in front, then back, front, back... outwards.
fn row_back(i: usize, n: usize) -> bool { (i as i32 - (n as i32 - 1) / 2).abs() % 2 == 1 }
