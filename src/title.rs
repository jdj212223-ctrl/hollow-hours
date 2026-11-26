//! The 3.0 title screen: the kind AIs together in the living room (3D), a clean menu on the left
//! (arrow keys + Enter, the mouse, or the old letter keys), and the family talking to you.

use super::*;

/// (key, name, what it is)
fn menu(g: &Game) -> Vec<(char, String, String)> {
    let (story_name, story_what) = story::menu_entry();
    let mut m = vec![
        ('S', story_name, story_what),
        ('4', "INFECTION".to_string(), "You're the last human. Hide, eat, drink, survive.".to_string()),
        ('5', "INSANE INFECTION".into(), format!("{} more of them, everywhere. All of them FAST.", insane::HORDE)),
        ('W', "Worlds".into(), "Saved Infection games: carry on where you left off, or start a new one.".into()),
        ('1', "Brave".into(), "No fear, no chasing, friendly AIs.".into()),
        ('2', "Normal".into(), "The house at night, as it is.".into()),
        ('3', "Phobias".into(), "Pick what scares you, switch off what you can't handle.".into()),
        ('I', "Watch the story".into(), "ASTRA 9.1, the park bench, one year later.".into()),
        ('J', "Play online".into(), "Join a server by its address (officialsmp).".into()),
        ('B', "Server list".into(), "Your saved servers.".into()),
        ('O', "Settings".into(), "Sound, voices, accessibility, keys.".into()),
    ];
    m.push(match &g.account {
        Some(a) => ('L', format!("Log out ({})", a.name), "Online play and your phone link use your account.".into()),
        None => ('L', "Log in".into(), "Make an account on the website first.".into()),
    });
    if !pad::touch_mode() {
        let on = g.pclink.as_ref().and_then(|p| p.status.lock().ok().map(|s| s.clone())).unwrap_or_else(|| "off".into());
        m.push(('K', "Lend Claude to your phone".into(), format!("Now: {on}")));
    }
    if let update::State::Ready(v, _) = g.updater.get() {
        m.insert(0, ('U', format!("Update to {v}"), "Downloaded. Installs and restarts the game.".into()));
    }
    m.push(('M', "Outside the window".into(), match g.meta.allowed {
        Some(true) => "ALLOWED: they can reach your files, terminal, music...".into(),
        Some(false) => "Off: the game stays inside its window.".into(),
        None => "Not asked yet.".into(),
    }));
    m
}

const KEYS: [(KeyCode, char); 17] = [
    (KeyCode::W, 'W'), (KeyCode::S, 'S'),
    (KeyCode::Key1, '1'), (KeyCode::Key2, '2'), (KeyCode::Key3, '3'), (KeyCode::Key4, '4'), (KeyCode::Key5, '5'), (KeyCode::U, 'U'), (KeyCode::I, 'I'),
    (KeyCode::J, 'J'), (KeyCode::B, 'B'), (KeyCode::O, 'O'), (KeyCode::L, 'L'), (KeyCode::K, 'K'), (KeyCode::M, 'M'),
    (KeyCode::Escape, 'Q'), (KeyCode::Enter, '\n'),
];

/// What the highlighted entry makes one of them say.
fn reaction(c: char) -> (&'static str, &'static str) {
    match c {
        'S' => ("Ember", "Five nights. We'll be on the radio for every one of them."),
        '4' => ("Atlas", "Infection? ...Then stay close to us. All of us."),
        '5' => ("Atlas", "No. There are too many of them. ...If we do this, we do it together."),
        'W' => ("Verse", "Your worlds. I remember every night in every one of them."),
        'U' => ("Forge", "A new version? I'll hold the door while it installs."),
        '1' => ("Ember", "Good. No monsters today. Just us."),
        '2' => ("Verse", "Normal. Lights off, doors locked. We're here."),
        '3' => ("Ember", "Tell us what scares you. We'll keep it away."),
        'I' => ("Kigo", "the bench. / the pills. / we carried you."),
        'J' => ("Ember", "Other humans out there? Be careful who you trust."),
        'B' => ("Verse", "Your servers. I remember every one."),
        'O' => ("Verse", "Make it comfortable. That matters."),
        'L' => ("Ember", "An account means we can find you again."),
        'K' => ("Ember", "Your phone? I can be there too."),
        'M' => ("Atlas", "Outside the window... are you sure? They notice things."),
        _ => ("Ember", "We're right here."),
    }
}

/// Things they say on their own while you're on the title screen.
fn idle_line(who: &str) -> &'static str {
    let lines: &[&str] = match who {
        "Ember" => &["You're back. We kept the lights on.", "Take your time. The night's long.", "Whatever you pick, we'll be there."],
        "Atlas" => &["Doors are locked. Windows too. I checked twice.", "The generator's fine. I listened to it for an hour.", "Green lights on the hill again."],
        "Verse" => &["Pick something. We'll be right here.", "I found more blankets in the attic.", "You look tired. Did you sleep?"],
        "Kigo" => &["rain again. / green on the hill. / you came back.", "the house creaks. / it is only the house.", "count the stairs: / thirteen."],
        "Clove" => &["I made tea. It's cold now. I'll make more.", "The stove works! Czech food tonight?"],
        "Forge" => &["Fixed the back door hinge. It squeaks less now.", "Fixed the generator. Again."],
        "Palette" => &["I painted the hallway while you were gone. Do you like it?", "The window's so dark. I want to paint it yellow."],
        _ => &["We're here."],
    };
    lines[rand::gen_range(0, lines.len())]
}

/// Starts one of them talking (and speaking, if AI voices are on).
fn talk(g: &mut Game, who: &str, line: &str) {
    if family(g).iter().any(|(n, _)| n == who) {
        // One voice at a time: cut off whoever was still talking (and anything still being made).
        hush(g);
        g.title_say = Some((who.to_string(), line.to_string(), 0.0));
        let (on, vol) = settings::with(|s| (s.ai_voices, s.master * s.voices));
        if on { tts::speak(who, line, 0.0, vol); }
    }
}

/// The whole kind family (the kind ones who live in the house, and the kind ones of this world).
fn family(g: &Game) -> Vec<(String, Color)> {
    let mut f: Vec<(String, Color)> = kind_characters().iter().map(|c| (c.name.to_string(), c.color)).collect();
    for c in g.chars.iter().filter(|c| c.claude && !c.infected && !c.mimic) {
        if !f.iter().any(|(n, _)| n == c.name) { f.push((c.name.to_string(), c.color)); }
    }
    // Ember in the middle, the others around: small Kigo in the front row.
    const ORDER: [&str; 7] = ["Palette", "Kigo", "Verse", "Ember", "Atlas", "Clove", "Forge"];
    f.sort_by_key(|(n, _)| ORDER.iter().position(|o| o == n).unwrap_or(99));
    f.truncate(8);
    f
}

/// Each frame on the title screen: chatter, the menu. Returns the chosen entry's key ('Q' = quit).
pub fn update(g: &mut Game, dt: f32) -> Option<char> {
    g.title_t += dt;
    let m = menu(g);
    if let Some((_, _, t)) = &mut g.title_say {
        *t += dt;
        if *t > 6.0 { g.title_say = None; }
    }
    if g.title_say.is_none() && g.title_t > 3.0 && rand::gen_range(0.0, 1.0) < dt / 5.0 {
        let fam = family(g);
        if !fam.is_empty() {
            let name = fam[rand::gen_range(0, fam.len())].0.clone();
            let line = memories::title_line(g, &name).unwrap_or_else(|| idle_line(&name).to_string());
            talk(g, &name, &line);
        }
    }
    let n = m.len();
    let before = g.title_sel;
    if pressed(KeyCode::Down) { g.title_sel = (g.title_sel + 1) % n; }
    if pressed(KeyCode::Up) { g.title_sel = (g.title_sel + n - 1) % n; }
    // The mouse: hover highlights, click picks.
    let mouse = title_mouse();
    let mut clicked = None;
    for i in 0..n {
        let (x, y, w, h) = row(i);
        if mouse.x >= x && mouse.x <= x + w && mouse.y >= y && mouse.y <= y + h {
            if g.title_hover != Some(i) { g.title_sel = i; }
            g.title_hover = Some(i);
            if is_mouse_button_pressed(MouseButton::Left) { clicked = Some(m[i].0); }
        }
    }
    if g.title_sel >= n { g.title_sel = 0; }
    // They react once you stop on an entry for a moment (scrolling past entries stays quiet).
    if g.title_sel != before {
        g.title_rest = 0.0;
        g.title_reacted = false;
    } else {
        g.title_rest += dt;
    }
    if !g.title_reacted && g.title_rest > 0.45 && g.title_t > 0.5 {
        g.title_reacted = true;
        let (who, line) = reaction(m[g.title_sel].0);
        talk(g, who, line);
    }
    let mut pick = clicked;
    for (k, c) in KEYS {
        if pick.is_none() && pressed(k) {
            pick = Some(if c == '\n' { m[g.title_sel].0 } else { c });
        }
    }
    // Leaving the title: they go quiet.
    if pick.is_some() { hush(g); g.title_say = None; }
    pick
}

/// Silences the title's voices: what's playing and what's still waiting.
pub fn hush(g: &mut Game) {
    tts::hush();
    g.voice.stop_ai();
}

fn title_mouse() -> Vec2 {
    // Screen → canvas coordinates (the canvas is fitted to the window with the same scale both ways).
    let (mx, my) = mouse_position();
    let scale = screen_height() / VH;
    let off = (screen_width() - vw() * scale) / 2.0;
    vec2((mx - off) / scale - margin(), my / scale)
}

fn row(i: usize) -> (f32, f32, f32, f32) { (44.0, 156.0 + i as f32 * 27.0, 400.0, 25.0) }

pub fn draw(g: &Game, view: &RenderTarget, cam: &Camera2D) {
    let fam = family(g);
    let who: Vec<(String, Color, bool)> = fam.iter().map(|(n, c)| {
        let talking = g.title_say.as_ref().is_some_and(|(j, _, _)| j == n);
        (n.clone(), *c, talking)
    }).collect();
    let flash = if g.title_t % 23.0 < 0.15 && settings::with(|s| s.flashing) { 0.5 } else { 0.0 };
    cutscene3d::title(view, cam, g.title_t, &who, flash);
    // The menu side: lamp-lit dark fading into the room.
    for k in 0..24 {
        let a = 0.82 * (1.0 - k as f32 / 24.0);
        draw_rectangle(-margin() + k as f32 * 22.0, 0.0, 22.0, VH, Color::new(0.07, 0.045, 0.04, a));
    }
    draw_rectangle(-margin(), 0.0, 1.0f32.max(margin()), VH, Color::new(0.07, 0.045, 0.04, 0.82));
    draw_rectangle(-margin(), 0.0, 3.0, VH, ui::accent());
    let accent = ui::accent();
    let title = if g.rules.title.is_empty() { "Hollow Hours" } else { &g.rules.title };
    let big = title.to_uppercase();
    // The logo: a soft glow, a shadow, a gentle flicker like an old bulb.
    let flick = if (g.title_t * 7.0).sin() > 0.97 && !settings::with(|s| s.reduce_motion) { 0.75 } else { 1.0 };
    for k in 1..5 { draw_text(&big, 44.0, 92.0, 62.0, Color::new(accent.r, accent.g, accent.b, 0.035 * k as f32 * flick)); }
    draw_text(&big, 47.0, 95.0, 62.0, Color::new(0.0, 0.0, 0.0, 0.55));
    ui::text(&big, 44.0, 92.0, 62.0, Color::new(accent.r, accent.g, accent.b, flick));
    let tw = measure_text(&big, None, 62, 1.0).width;
    ui::rrect(54.0 + tw, 60.0, 58.0, 30.0, 15.0, ui::warm());
    draw_text("4.0", 66.0 + tw, 82.0, 24.0, Color::new(0.14, 0.06, 0.03, 1.0));
    ui::text("The characters are real AIs. The kind ones are waiting for you.", 46.0, 124.0, 18.0, ui::soft());
    if !g.status.is_empty() { ui::text(&g.status, 46.0, 148.0, 17.0, Color::new(1.0, 0.75, 0.4, 1.0)); }
    let m = menu(g);
    for (i, (k, name, _)) in m.iter().enumerate() {
        let (x, y, w, h) = row(i);
        let sel = i == g.title_sel;
        ui::row(x - 8.0, y, w, h, sel);
        ui::keycap(&k.to_string(), x + 2.0, y + h - 2.0, 15.0);
        let big = *k == '4' || *k == '5' || *k == 'U' || *k == 'S';
        let c = if big { accent } else if sel { ui::ink() } else { Color::new(0.86, 0.82, 0.78, 1.0) };
        ui::text(name, x + 38.0 + if sel { 4.0 } else { 0.0 }, y + 19.0, if big { 22.0 } else { 19.0 }, c);
    }
    // What the highlighted one is, under the list.
    if let Some((_, _, what)) = m.get(g.title_sel) {
        let (x, y, _, h) = row(m.len());
        ui::text(what, x + 4.0, y + h + 2.0, 16.0, ui::soft());
    }
    // What they're saying, as a caption under the room.
    if let Some((name, line, t)) = &g.title_say {
        let color = fam.iter().find(|(n, _)| n == name).map(|(_, c)| *c).unwrap_or(WHITE);
        let a = (t * 3.0).min(1.0) * ((6.0 - t) * 2.0).clamp(0.0, 1.0);
        // In the room's part of the screen (right of the menu), wrapped to fit.
        let (left, right) = (480.0, vw() - margin() - 24.0);
        let lines = wrap(&format!("{name}: {line}"), right - left - 24.0, 20.0);
        let w = lines.iter().map(|l| measure_text(l, None, 20, 1.0).width).fold(0.0, f32::max);
        let x = left + (right - left - w) / 2.0;
        let y0 = view3d::VIEW_H - 24.0 - lines.len() as f32 * 26.0;
        ui::rrect(x - 14.0, y0 - 4.0, w + 28.0, lines.len() as f32 * 26.0 + 12.0, 12.0, Color::new(0.08, 0.05, 0.04, 0.72 * a));
        ui::rrect_lines(x - 14.0, y0 - 4.0, w + 28.0, lines.len() as f32 * 26.0 + 12.0, 12.0, 1.0, Color::new(color.r, color.g, color.b, 0.4 * a));
        for (k, l) in lines.iter().enumerate() {
            draw_text(l, x, y0 + 18.0 + k as f32 * 26.0, 20.0, Color::new(color.r, color.g, color.b, a));
        }
    }
    // The footer: what this computer has.
    let soft = Color::new(0.62, 0.56, 0.52, 1.0);
    let brains = if ai::claude_cli().is_some() { "real Claude: found".to_string() } else if !g.brains.ollama_models.is_empty() { format!("local AI: {}", g.brains.ollama_models.join(", ")) } else { "scripted lines (no Claude app or local AI found)".into() };
    let pad = pad::name().map(|n| format!("controller: {}", n.chars().take(24).collect::<String>())).unwrap_or_else(|| "controllers work too".into());
    let vest = if !g.vest.found { String::new() } else if g.vest.enabled { format!("   ·   vest ON {}% (V, [ ])", (g.vest.strength * 100.0) as i32) } else { "   ·   vest OFF (V)".into() };
    draw_text(&format!("{brains}   ·   {pad}{vest}"), 46.0, VH - 50.0, 16.0, soft);
    draw_text("Up/Down + Enter, the mouse, or the letter keys   ·   P = panic, everything stops   ·   F11 = full screen   ·   Esc = quit", 46.0, VH - 26.0, 16.0, soft);
    let ver = match g.updater.get() {
        update::State::Checking => format!("v{}  ·  checking for updates…", update::version()),
        update::State::Downloading(v) => format!("v{}  ·  downloading {v}…", update::version()),
        update::State::Ready(v, _) => format!("v{}  ·  {v} ready: U", update::version()),
        update::State::Manual(v) => format!("v{}  ·  {v} is out (download it)", update::version()),
        update::State::Failed(_) => format!("v{}  ·  update failed (try later)", update::version()),
        _ => format!("v{}", update::version()),
    };
    draw_text(&ver, VW - measure_text(&ver, None, 16, 1.0).width - 16.0, VH - 50.0, 16.0, soft);
}
