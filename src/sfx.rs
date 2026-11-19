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
            growl: s(growl).await, pickup: s(pickup).await, door: s(door).await, radio: s(radio).await, sizzle: s(sizzle).await, sizzle_on: false, buzz: s(buzz).await, buzz_on: false, hit: s(hit).await, captions: Default::default(), on: true, wind_on: false,
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
    /// Tornado wind while one is on your map.
    pub fn wind(&mut self, on: bool) {
        let Some(w) = &self.wind else { return };
        if on && !self.wind_on && self.on { play_sound(w, PlaySoundParams { looped: true, volume: 0.6 * Self::amb() }); self.wind_on = true; }
        if !on && self.wind_on { stop_sound(w); self.wind_on = false; }
        if on { self.caption("[howling wind]"); }
    }
    /// Everything off (the panic key, or M to mute).
    pub fn silence(&mut self) {
        for s in [&self.hum, &self.wind, &self.sizzle, &self.buzz].into_iter().flatten() { stop_sound(s); }
        self.buzz_on = false;
        self.wind_on = false;
        self.sizzle_on = false;
    }
    #[allow(dead_code)]
    pub fn play_once(s: &Sound) { play_sound_once(s); }
}
