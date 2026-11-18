//! Hollow Hours' sounds, all made from math at startup (no audio files, nothing borrowed): a low ambient hum,
//! a heartbeat, footsteps, a jumpscare sting, tornado wind, an infected growl, a pickup blip, a door creak and a
//! radio crackle. Turned into WAV in memory and handed to macroquad's audio.

use macroquad::audio::{load_sound_from_bytes, play_sound, play_sound_once, stop_sound, PlaySoundParams, Sound};

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
        let radio = gen(0.35, |t, r| r.next() * 0.35 * (1.0 - t / 0.35) * if ((t * 60.0) as i32) % 3 == 0 { 1.0 } else { 0.3 });
        Sfx {
            hum: s(hum).await, wind: s(wind).await, heart: s(heart).await, step: s(step).await, sting: s(sting).await,
            growl: s(growl).await, pickup: s(pickup).await, door: s(door).await, radio: s(radio).await, on: true, wind_on: false,
        }
    }

    fn once(&self, s: &Option<Sound>, volume: f32) {
        if let (true, Some(s)) = (self.on, s) {
            play_sound(s, PlaySoundParams { looped: false, volume });
        }
    }

    pub fn start_hum(&self) {
        if let (true, Some(s)) = (self.on, &self.hum) {
            play_sound(s, PlaySoundParams { looped: true, volume: 0.35 });
        }
    }
    pub fn heart(&self, fear: f32) { self.once(&self.heart, 0.25 + fear * 0.6); }
    pub fn step(&self, loud: bool) { self.once(&self.step, if loud { 0.35 } else { 0.08 }); }
    pub fn sting(&self) { self.once(&self.sting, 0.8); }
    pub fn growl(&self, near: f32) { self.once(&self.growl, 0.2 + near * 0.6); }
    pub fn pickup(&self) { self.once(&self.pickup, 0.5); }
    pub fn door(&self) { self.once(&self.door, 0.5); }
    pub fn radio(&self) { self.once(&self.radio, 0.4); }
    /// Tornado wind while one is on your map.
    pub fn wind(&mut self, on: bool) {
        let Some(w) = &self.wind else { return };
        if on && !self.wind_on && self.on { play_sound(w, PlaySoundParams { looped: true, volume: 0.6 }); self.wind_on = true; }
        if !on && self.wind_on { stop_sound(w); self.wind_on = false; }
    }
    /// Everything off (the panic key, or M to mute).
    pub fn silence(&mut self) {
        for s in [&self.hum, &self.wind].into_iter().flatten() { stop_sound(s); }
        self.wind_on = false;
    }
    #[allow(dead_code)]
    pub fn play_once(s: &Sound) { play_sound_once(s); }
}
