//! Hollow Hours' sounds, all made from math at startup (no audio files, nothing borrowed): a low ambient hum,
//! a heartbeat, footsteps, a jumpscare sting, tornado wind, an infected growl, a pickup blip, a door creak and a
//! radio crackle. Turned into WAV in memory and handed to macroquad's audio.

use macroquad::audio::{load_sound_from_bytes, play_sound, play_sound_once, set_sound_volume, stop_sound, PlaySoundParams, Sound};

const RATE: u32 = 22050;

/// A tiny pseudo-random generator for noise (same sounds every run).
struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn wav(samples: &[f32]) -> Vec<u8> {
    let data: Vec<u8> = samples.iter().flat_map(|s| ((s.clamp(-1.0, 1.0) * 30000.0) as i16).to_le_bytes()).collect();
    let mut out = Vec::with_capacity(44 + data.len());
    out.extend(b"RIFF");
    out.extend((36 + data.len() as u32).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes()); // PCM
    out.extend(1u16.to_le_bytes()); // mono
    out.extend(RATE.to_le_bytes());
    out.extend((RATE * 2).to_le_bytes());
    out.extend(2u16.to_le_bytes());
    out.extend(16u16.to_le_bytes());
    out.extend(b"data");
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
    out
}

fn gen(secs: f32, mut f: impl FnMut(f32, &mut Rng) -> f32) -> Vec<f32> {
    let mut r = Rng(0x9E37_79B9);
    (0..(secs * RATE as f32) as usize).map(|i| f(i as f32 / RATE as f32, &mut r)).collect()
}

const TAU: f32 = std::f32::consts::TAU;

pub struct Sfx {
    pub hum: Option<Sound>,
    pub wind: Option<Sound>,
    heart: Option<Sound>,
    step: Option<Sound>,
    sting: Option<Sound>,
    growl: Option<Sound>,
    pickup: Option<Sound>,
    door: Option<Sound>,
    radio: Option<Sound>,
    sizzle: Option<Sound>,
    sizzle_on: bool,
    buzz: Option<Sound>,
    buzz_on: bool,
    hit: Option<Sound>,
    /// The breach theme: epic, looping (see breach_theme).
    breach: Option<Sound>,
    breach_on: bool,
    captions: std::cell::RefCell<Vec<(String, f64)>>,
    pub on: bool,
    wind_on: bool,
}

impl Sfx {
    pub async fn load() -> Self {
        async fn s(samples: Vec<f32>) -> Option<Sound> {
            load_sound_from_bytes(&wav(&samples)).await.ok()
        }
        // A low, slowly beating hum (two close notes) with a faint high whine: loops seamlessly every 8 s.
        let hum = gen(8.0, |t, _| 0.18 * (TAU * 55.0 * t).sin() + 0.12 * (TAU * 55.5 * t).sin() + 0.02 * (TAU * 1760.0 * t).sin() * (TAU * 0.25 * t).sin());
        // Wind: noise, smoothed, swelling.
        let wind = {
            let mut last = 0.0;
            gen(6.0, move |t, r| { last = last * 0.97 + r.next() * 0.03; last * 6.0 * (0.6 + 0.4 * (TAU * t / 6.0).sin().abs()) })
        };
        // Heartbeat: lub-dub.
        let heart = gen(0.6, |t, _| {
            let beat = |t0: f32, amp: f32| if t >= t0 { amp * (-(t - t0) * 30.0).exp() * (TAU * 50.0 * (t - t0)).sin() } else { 0.0 };
            beat(0.0, 0.9) + beat(0.18, 0.6)
        });
        let step = gen(0.12, |t, r| r.next() * 0.5 * (-t * 45.0).exp());
        // Sting: a dissonant cluster that falls in pitch, with noise.
        let sting = gen(1.4, |t, r| {
            let env = (-t * 2.2).exp();
            let fall = 1.0 - t * 0.25;
            env * (0.3 * (TAU * 220.0 * fall * t).sin() + 0.25 * (TAU * 233.0 * fall * t).sin() + 0.2 * (TAU * 311.0 * fall * t).sin() + 0.25 * r.next() * (-t * 6.0).exp())
        });
        // Growl: a low rough buzz that wobbles.
        let growl = gen(0.9, |t, r| {
            let f = 70.0 + 12.0 * (TAU * 6.0 * t).sin();
            let saw = 2.0 * ((f * t) % 1.0) - 1.0;
            (saw * 0.35 + r.next() * 0.1) * (t * 8.0).min(1.0) * (-(t - 0.5).max(0.0) * 5.0).exp()
        });
        let pickup = gen(0.18, |t, _| 0.4 * (TAU * (880.0 + 1200.0 * t) * t).sin() * (-t * 18.0).exp());
        let door = gen(0.8, |t, r| 0.25 * (TAU * (180.0 + 90.0 * (TAU * 3.0 * t).sin()) * t).sin() * (1.0 - t / 0.8) + 0.05 * r.next());
        // Frying: hissing noise with little fat pops.
        let sizzle = {
            let mut last = 0.0;
            gen(3.0, move |t, r| {
                let n = r.next();
                last = last * 0.4 + n * 0.6; // brighter noise
                let pop = if (t * 37.0).fract() < 0.01 && r.next() > 0.3 { r.next() * 0.8 } else { 0.0 };
                (last * 0.22 + pop) * (0.8 + 0.2 * (TAU * 0.7 * t).sin())
            })
        };
        // The drone: two close, buzzing saw tones with a fast flutter (the propellers).
        let buzz = gen(2.0, |t, _| {
            let saw = |f: f32| 2.0 * ((f * t) % 1.0) - 1.0;
            (saw(210.0) * 0.5 + saw(213.0) * 0.5) * 0.14 * (0.75 + 0.25 * (TAU * 30.0 * t).sin())
        });
        // A hit: a low thud with a bit of crunch.
        let hit = gen(0.25, |t, r| ((TAU * 70.0 * t).sin() * 0.7 + r.next() * 0.35) * (-t * 22.0).exp());
        let radio = gen(0.35, |t, r| r.next() * 0.35 * (1.0 - t / 0.35) * if ((t * 60.0) as i32) % 3 == 0 { 1.0 } else { 0.3 });
        Sfx {
            hum: s(hum).await, wind: s(wind).await, heart: s(heart).await, step: s(step).await, sting: s(sting).await,
            growl: s(growl).await, pickup: s(pickup).await, door: s(door).await, radio: s(radio).await, sizzle: s(sizzle).await, sizzle_on: false, buzz: s(buzz).await, buzz_on: false, hit: s(hit).await, breach: s(breach_theme()).await, breach_on: false, captions: Default::default(), on: true, wind_on: false,
        }
    }

    fn amb() -> f32 { crate::settings::with(|s| s.master * s.ambience) }
    fn fx() -> f32 { crate::settings::with(|s| s.master * s.effects) }

    /// A caption for people who can't hear the sounds (Settings > Accessibility > Captions).
    pub fn caption(&self, text: &str) {
        if !crate::settings::with(|s| s.captions) { return; }
        let mut c = self.captions.borrow_mut();
        let now = macroquad::time::get_time();
        if let Some(last) = c.iter_mut().find(|(t, _)| t == text) { last.1 = now; return; }
        c.push((text.to_string(), now));
        if c.len() > 4 { c.remove(0); }
    }
    /// Captions from the last 2.5 seconds.
    pub fn recent_captions(&self) -> Vec<String> {
        let now = macroquad::time::get_time();
        let mut c = self.captions.borrow_mut();
        c.retain(|(_, t)| now - t < 2.5);
        c.iter().map(|(x, _)| x.clone()).collect()
    }

    fn once(&self, s: &Option<Sound>, volume: f32) {
        if let (true, Some(s)) = (self.on, s) {
            play_sound(s, PlaySoundParams { looped: false, volume: volume * Self::fx() });
        }
    }

    /// The looping sounds follow the volume sliders right away.
    pub fn apply_volumes(&self) {
        let a = Self::amb();
        if let Some(h) = &self.hum { set_sound_volume(h, 0.35 * a); }
        if let (true, Some(w)) = (self.wind_on, &self.wind) { set_sound_volume(w, 0.6 * a); }
        if let (true, Some(z)) = (self.sizzle_on, &self.sizzle) { set_sound_volume(z, 0.55 * a); }
        if let (true, Some(b)) = (self.buzz_on, &self.buzz) { set_sound_volume(b, 0.45 * a); }
        if let (true, Some(m)) = (self.breach_on, &self.breach) { set_sound_volume(m, 0.8 * a); }
    }

    pub fn start_hum(&self) {
        if let (true, Some(s)) = (self.on, &self.hum) {
            play_sound(s, PlaySoundParams { looped: true, volume: 0.35 * Self::amb() });
        }
    }
    pub fn heart(&self, fear: f32) { self.once(&self.heart, 0.25 + fear * 0.6); self.caption("[heartbeat]"); }
    pub fn step(&self, loud: bool) { self.once(&self.step, if loud { 0.35 } else { 0.08 }); }
    pub fn sting(&self) { self.once(&self.sting, 0.8); self.caption("[sudden screech]"); }
    /// A growl; `side` says where it comes from ("left", "behind"...).
    pub fn growl(&self, near: f32, side: &str) {
        self.once(&self.growl, 0.2 + near * 0.6);
        self.caption(&format!("[growl{}, {side}]", if near > 0.6 { " (close!)" } else { "" }));
    }
    pub fn pickup(&self) { self.once(&self.pickup, 0.5); }
    pub fn door(&self) { self.once(&self.door, 0.5); self.caption("[door creaks]"); }
    pub fn radio(&self) { self.once(&self.radio, 0.4); self.caption("[radio crackle]"); }
    pub fn hit(&self) { self.once(&self.hit, 0.8); self.caption("[thud]"); }
    /// The drone flying (loops until it lands).
    pub fn drone(&mut self, on: bool) {
        let Some(s) = &self.buzz else { return };
        if on && !self.buzz_on && self.on { play_sound(s, PlaySoundParams { looped: true, volume: 0.45 * Self::amb() }); self.buzz_on = true; }
        if !on && self.buzz_on { stop_sound(s); self.buzz_on = false; }
        if on { self.caption("[drone buzzing]"); }
    }
    /// Frying on the stove (loops until turned off).
    pub fn sizzle(&mut self, on: bool) {
        let Some(s) = &self.sizzle else { return };
        if on && !self.sizzle_on && self.on { play_sound(s, PlaySoundParams { looped: true, volume: 0.55 * Self::amb() }); self.sizzle_on = true; }
        if !on && self.sizzle_on { stop_sound(s); self.sizzle_on = false; }
        if on { self.caption("[sizzling, loud]"); }
    }
    /// The breach theme (loops while the house is being broken into).
    pub fn breach_music(&mut self, on: bool) {
        let Some(m) = &self.breach else { return };
        if on && !self.breach_on && self.on { play_sound(m, PlaySoundParams { looped: true, volume: 0.8 * Self::amb() }); self.breach_on = true; }
        if !on && self.breach_on { stop_sound(m); self.breach_on = false; }
        if on { self.caption("[epic music: drums, brass, choir]"); }
    }
    /// Tornado wind while one is on your map.
    pub fn wind(&mut self, on: bool) {
        let Some(w) = &self.wind else { return };
        if on && !self.wind_on && self.on { play_sound(w, PlaySoundParams { looped: true, volume: 0.6 * Self::amb() }); self.wind_on = true; }
        if !on && self.wind_on { stop_sound(w); self.wind_on = false; }
        if on { self.caption("[howling wind]"); }
    }
    /// Everything off (the panic key, or M to mute).
    pub fn silence(&mut self) {
        for s in [&self.hum, &self.wind, &self.sizzle, &self.buzz, &self.breach].into_iter().flatten() { stop_sound(s); }
        self.breach_on = false;
        self.buzz_on = false;
        self.wind_on = false;
        self.sizzle_on = false;
    }
    #[allow(dead_code)]
    pub fn play_once(s: &Sound) { play_sound_once(s); }
}

/// THE BREACH: an epic orchestral loop made from math. D minor, 132 BPM, 16 bars of 4/4... in 8 bars (14.5 s):
/// a low brass BRAAM on bar 1, taiko war drums, a driving low-brass ostinato, staccato strings in 16ths and a choir
/// pad over i - VI - III - VII (Dm, Bb, F, C), the chords every trailer uses.
fn breach_theme() -> Vec<f32> {
    let bpm = 132.0f32;
    let beat = 60.0 / bpm;
    let bar = beat * 4.0;
    let bars = 8;
    let len = bar * bars as f32;
    let midi = |n: f32| 440.0 * 2f32.powf((n - 69.0) / 12.0);
    // Chord roots (MIDI) per bar, two bars each: D, Bb, F, C.
    let roots = [38.0, 38.0, 34.0, 34.0, 41.0, 41.0, 36.0, 36.0];
    // Minor/major triads on those roots: Dm, Bb, F, C.
    let thirds = [3.0, 3.0, 4.0, 4.0, 4.0, 4.0, 4.0, 4.0];
    let saw = |f: f32, t: f32| 2.0 * ((f * t).fract()) - 1.0;
    let mut lp_brass = 0.0f32;
    let mut lp_str = 0.0f32;
    let mut lp_noise = 0.0f32;
    let mut out = gen(len, |t, r| {
        let b = ((t / bar) as usize).min(bars - 1);
        let tb = t % beat; // time in this beat
        let beat_i = ((t / beat) as usize) % 4;
        let eighth = t % (beat / 2.0);
        let sixteenth = t % (beat / 4.0);
        let root = roots[b];
        let third = thirds[b];
        // --- Taiko: big low hits on 1 and 3, a double on the "and" of 4, plus a quieter one on 2.
        let taiko = |t0: f32, amp: f32| if t0 >= 0.0 && t0 < 0.6 {
            let f = 95.0 * (-t0 * 9.0).exp() + 42.0;
            amp * (TAU * f * t0).sin() * (-t0 * 7.0).exp()
        } else { 0.0 };
        let mut drums = taiko(tb, if beat_i == 0 || beat_i == 2 { 1.0 } else { 0.45 });
        if beat_i == 3 { drums += taiko(tb - beat / 2.0, 0.8); }
        // Snare/clap with a bit of room on 2 and 4.
        let n = r.next();
        lp_noise = lp_noise * 0.55 + n * 0.45;
        if beat_i == 1 || beat_i == 3 { drums += lp_noise * 0.35 * (-tb * 18.0).exp(); }
        // Last bar: a rolling build of 16th taikos into the loop.
        if b == bars - 1 && beat_i >= 2 { drums += taiko(sixteenth, 0.5); }
        // --- Low brass ostinato: root, root, octave, fifth... in eighths, filtered saws, accents on the beat.
        let pattern = [0.0, 0.0, 12.0, 0.0, 7.0, 0.0, 10.0, 7.0];
        let step = ((t / (beat / 2.0)) as usize) % 8;
        let f = midi(root + pattern[step]);
        let env = (eighth * 60.0).min(1.0) * (-eighth * 5.0).exp();
        let raw = (saw(f, t) + saw(f * 1.006, t)) * 0.5;
        lp_brass = lp_brass * 0.86 + raw * 0.14;
        let brass = lp_brass * env * if step % 2 == 0 { 0.55 } else { 0.38 };
        // --- Staccato strings: chord tones in 16ths, up an octave and a bit, short and bright.
        let arp = [0.0, third, 7.0, 12.0, 7.0, third, 0.0, 12.0];
        let s16 = ((t / (beat / 4.0)) as usize) % 8;
        let fs = midi(root + 24.0 + arp[s16]);
        lp_str = lp_str * 0.6 + saw(fs, t) * 0.4;
        let strings = lp_str * (sixteenth * 200.0).min(1.0) * (-sixteenth * 28.0).exp() * 0.16;
        // --- Choir: soft "aah" on the chord, slow vibrato, swelling every two bars.
        let tb2 = t % (bar * 2.0);
        let swell = (tb2 / 0.8).min(1.0) * (0.75 + 0.25 * (TAU * tb2 / (bar * 2.0)).sin().abs());
        let vib = 1.0 + 0.004 * (TAU * 5.2 * t).sin();
        let mut choir = 0.0;
        for (k, iv) in [12.0, 12.0 + third, 19.0, 24.0].iter().enumerate() {
            let fc = midi(root + iv) * vib * (1.0 + 0.002 * k as f32);
            choir += (TAU * fc * t).sin() + 0.3 * (TAU * 2.0 * fc * t).sin();
        }
        let choir = choir * 0.045 * swell;
        // --- BRAAM: a huge low brass blast at the top of the loop, and a smaller one halfway.
        let braam = |t0: f32, amp: f32| if t0 >= 0.0 && t0 < 2.4 {
            let e = (t0 * 20.0).min(1.0) * (-t0 * 1.4).exp();
            let fb = midi(26.0);
            amp * e * ((saw(fb, t0) + saw(fb * 1.5, t0) * 0.6 + saw(fb * 2.0, t0) * 0.5) * 0.3 + (TAU * fb * 0.5 * t0).sin() * 0.4)
        } else { 0.0 };
        let hit = braam(t, 0.9) + braam(t - bar * 4.0, 0.5);
        drums * 0.8 + brass + strings + choir + hit
    });
    // Normalise, and a tiny fade at the loop point so it never clicks.
    let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs())).max(0.01);
    let n = out.len();
    for (i, x) in out.iter_mut().enumerate() {
        let edge = ((i.min(n - 1 - i)) as f32 / 400.0).min(1.0);
        *x = *x / peak * 0.85 * edge;
    }
    out
}

#[cfg(test)]
mod tests {
    /// HH_MUSIC_DEMO=path.wav: writes the breach theme (twice through) to listen to.
    #[test]
    fn breach_theme_is_loud_enough_and_loops() {
        let m = super::breach_theme();
        assert!(m.len() > 22050 * 10, "at least 10 seconds");
        assert!(m.iter().any(|x| x.abs() > 0.5), "not silent");
        assert!(m[0].abs() < 0.01 && m[m.len() - 1].abs() < 0.01, "no click at the loop point");
        if let Ok(p) = std::env::var("HH_MUSIC_DEMO") {
            let twice: Vec<f32> = m.iter().chain(m.iter()).copied().collect();
            std::fs::write(p, super::wav(&twice)).unwrap();
        }
    }
}
