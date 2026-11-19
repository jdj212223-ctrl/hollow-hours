//! Voice chat for online games.
//!  - Hold V: talk out loud. Only players on the same map hear you, quieter the further away they are (12 tiles).
//!  - Hold R: talk on the radio. Everyone in the room hears you wherever they are, through crackle and static.
//! The microphone only opens the first time you press V or R (macOS asks for permission then), and only while you
//! hold the key is anything sent. Sound goes as 8 kHz mu-law (telephone quality, ~8 KB/s) through the room's
//! WebSocket, like the other game messages: the server only passes it on, nothing is recorded or stored.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub const RATE: u32 = 8000;
/// How far (in tiles) your voice carries without the radio.
pub const RANGE: f32 = 12.0;

struct Speaker {
    q: VecDeque<f32>,
    /// Samples per second of this speaker's sound (voice chat 8000, AI voices whatever the speech engine made).
    rate: f32,
    gain: f32,
    radio: bool,
    phase: f32,
    cur: f32,
    noise: u32,
}

pub struct Voice {
    mic: Option<cpal::Stream>,
    out: Option<cpal::Stream>,
    captured: Arc<Mutex<Vec<f32>>>,
    recording: Arc<AtomicBool>,
    in_rate: u32,
    speakers: Arc<Mutex<HashMap<String, Speaker>>>,
    /// What you're doing right now: None, Some(false) = talking nearby, Some(true) = on the radio.
    pub talking: Option<bool>,
    pub error: Option<String>,
    /// When each player was last heard (for "talking" over their head).
    pub heard: HashMap<String, (f64, bool)>,
}

impl Default for Voice {
    fn default() -> Self {
        Voice { mic: None, out: None, captured: Default::default(), recording: Default::default(), in_rate: 48000,
            speakers: Default::default(), talking: None, error: None, heard: HashMap::new() }
    }
}

// ---- mu-law (G.711): 8 bits per sample ----
fn mulaw_encode(x: f32) -> u8 {
    let s = (x.clamp(-1.0, 1.0) * 32635.0) as i32;
    let sign = if s < 0 { 0x80 } else { 0 };
    let mut m = s.unsigned_abs() as i32 + 0x84;
    let mut exp = 7;
    let mut mask = 0x4000;
    while exp > 0 && m & mask == 0 { exp -= 1; mask >>= 1; }
    m = (m >> (exp + 3)) & 0x0f;
    !(sign | (exp << 4) as u8 | m as u8)
}
fn mulaw_decode(b: u8) -> f32 {
    let b = !b;
    let sign = b & 0x80;
    let exp = ((b >> 4) & 7) as i32;
    let m = (b & 0x0f) as i32;
    let v = (((m << 3) + 0x84) << exp) - 0x84;
    (if sign != 0 { -v } else { v }) as f32 / 32768.0
}

// ---- base64 (the room speaks JSON) ----
const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
fn b64_encode(d: &[u8]) -> String {
    let mut o = String::with_capacity(d.len().div_ceil(3) * 4);
    for c in d.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 { o.push(if i <= c.len() { B64[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' }); }
    }
    o
}
fn b64_decode(s: &str) -> Vec<u8> {
    let val = |c: u8| B64.iter().position(|&x| x == c).map(|v| v as u32);
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    for chunk in s.as_bytes().chunks(4) {
        let mut n = 0u32;
        let mut k: u32 = 0;
        for &c in chunk {
            if let Some(v) = val(c) { n = n << 6 | v; k += 1; }
        }
        n <<= 6 * (4 - k);
        for i in 0..k.saturating_sub(1) { out.push((n >> (16 - 8 * i)) as u8); }
    }
    out
}

/// Turns microphone sound (at the device's rate) into 8 kHz mu-law bytes.
pub fn encode(samples: &[f32], in_rate: u32) -> Vec<u8> {
    let step = in_rate as f32 / RATE as f32;
    let n = (samples.len() as f32 / step) as usize;
    (0..n).map(|i| {
        let a = (i as f32 * step) as usize;
        let b = (((i + 1) as f32 * step) as usize).min(samples.len()).max(a + 1);
        let avg = samples[a..b].iter().sum::<f32>() / (b - a) as f32;
        mulaw_encode(avg)
    }).collect()
}

pub fn decode(bytes: &[u8]) -> Vec<f32> { bytes.iter().map(|&b| mulaw_decode(b)).collect() }

impl Voice {
    /// Opens the microphone (the first time only; macOS asks for permission).
    fn open_mic(&mut self) -> Result<(), String> {
        if self.mic.is_some() { return Ok(()); }
        let dev = cpal::default_host().default_input_device().ok_or("no microphone found")?;
        let cfg = dev.default_input_config().map_err(|e| e.to_string())?;
        let fmt = cfg.sample_format();
        let config: cpal::StreamConfig = cfg.into();
        self.in_rate = config.sample_rate.0;
        let ch = config.channels as usize;
        let (buf, rec) = (self.captured.clone(), self.recording.clone());
        let push = move |mono: &mut dyn Iterator<Item = f32>| {
            if !rec.load(Ordering::Relaxed) { return; }
            if let Ok(mut b) = buf.lock() {
                b.extend(mono);
                let cap = 48000 * 2;
                if b.len() > cap { let extra = b.len() - cap; b.drain(..extra); }
            }
        };
        let err = |e: cpal::StreamError| eprintln!("microphone: {e}");
        let stream = match fmt {
            cpal::SampleFormat::F32 => dev.build_input_stream(&config, move |d: &[f32], _: &_| push(&mut d.chunks(ch).map(|f| f[0])), err, None),
            cpal::SampleFormat::I16 => dev.build_input_stream(&config, move |d: &[i16], _: &_| push(&mut d.chunks(ch).map(|f| f[0] as f32 / 32768.0)), err, None),
            cpal::SampleFormat::U16 => dev.build_input_stream(&config, move |d: &[u16], _: &_| push(&mut d.chunks(ch).map(|f| (f[0] as f32 - 32768.0) / 32768.0)), err, None),
            other => return Err(format!("microphone format {other:?} isn't supported")),
        }.map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        self.mic = Some(stream);
        Ok(())
    }

    /// Opens the speakers for other players' voices (the first time someone talks).
    fn open_out(&mut self) {
        if self.out.is_some() { return; }
        let Some(dev) = cpal::default_host().default_output_device() else { return };
        let Ok(cfg) = dev.default_output_config() else { return };
        if cfg.sample_format() != cpal::SampleFormat::F32 { return; }
        let config: cpal::StreamConfig = cfg.into();
        let (rate, ch) = (config.sample_rate.0 as f32, config.channels as usize);
        let speakers = self.speakers.clone();
        let stream = dev.build_output_stream(&config, move |out: &mut [f32], _: &_| {
            let Ok(mut sp) = speakers.lock() else { return };
            for frame in out.chunks_mut(ch) {
                let mut mix = 0.0;
                for s in sp.values_mut() {
                    s.phase += s.rate / rate;
                    while s.phase >= 1.0 {
                        s.phase -= 1.0;
                        s.cur = s.q.pop_front().unwrap_or(0.0);
                    }
                    let mut v = s.cur;
                    if s.radio {
                        // A walkie-talkie: squashed, a bit distorted, with static under it.
                        s.noise ^= s.noise << 13; s.noise ^= s.noise >> 17; s.noise ^= s.noise << 5;
                        let n = (s.noise as f32 / u32::MAX as f32 - 0.5) * 0.06;
                        v = (v * 2.5).tanh() * 0.7 + if s.q.is_empty() { 0.0 } else { n };
                    }
                    mix += v * s.gain;
                }
                let mix = mix.clamp(-1.0, 1.0);
                for x in frame.iter_mut() { *x = mix; }
            }
        }, |e| eprintln!("speakers: {e}"), None);
        if let Ok(s) = stream { if s.play().is_ok() { self.out = Some(s); } }
    }

    /// Start or stop talking (None = stop, Some(radio)).
    pub fn set_talking(&mut self, t: Option<bool>) {
        if t.is_some() && self.mic.is_none() {
            if let Err(e) = self.open_mic() { self.error = Some(e); return; }
        }
        self.talking = t;
        self.recording.store(t.is_some(), Ordering::Relaxed);
        if t.is_none() { if let Ok(mut b) = self.captured.lock() { b.clear(); } }
    }

    /// What's been recorded since last time, ready to send ({"t": "vc", "r": radio, "d": base64}), every ~100 ms.
    pub fn take_packet(&mut self) -> Option<serde_json::Value> {
        let radio = self.talking?;
        let mut b = self.captured.lock().ok()?;
        if b.len() < self.in_rate as usize / 10 { return None; }
        let chunk: Vec<f32> = std::mem::take(&mut *b);
        drop(b);
        Some(serde_json::json!({"t": "vc", "r": radio, "d": b64_encode(&encode(&chunk, self.in_rate))}))
    }

    /// Someone's voice arrived: queue it at this loudness (0 = can't hear them).
    pub fn hear(&mut self, from: &str, data: &str, radio: bool, gain: f32, now: f64) {
        self.heard.insert(from.to_string(), (now, radio));
        if gain <= 0.0 { return; }
        self.open_out();
        let samples = decode(&b64_decode(data));
        if let Ok(mut sp) = self.speakers.lock() {
            let s = sp.entry(from.to_string()).or_insert(Speaker { q: VecDeque::new(), rate: RATE as f32, gain, radio, phase: 0.0, cur: 0.0, noise: 0x1234_5678 });
            s.gain = gain;
            s.radio = radio;
            s.q.extend(samples);
            // Never fall more than half a second behind.
            let max = RATE as usize / 2;
            if s.q.len() > max { let extra = s.q.len() - max; s.q.drain(..extra); }
        }
    }

    /// Cuts off every AI voice that's playing right now.
    pub fn stop_ai(&mut self) {
        if let Ok(mut sp) = self.speakers.lock() {
            for (k, s) in sp.iter_mut() { if k.starts_with("ai:") { s.q.clear(); } }
        }
    }

    /// An AI character's spoken line (see tts.rs): played on its own channel, at its own sample rate.
    pub fn play(&mut self, who: &str, samples: Vec<f32>, rate: u32, gain: f32) {
        self.open_out();
        if let Ok(mut sp) = self.speakers.lock() {
            let s = sp.entry(format!("ai:{who}")).or_insert(Speaker { q: VecDeque::new(), rate: rate as f32, gain, radio: false, phase: 0.0, cur: 0.0, noise: 7 });
            s.rate = rate as f32;
            s.gain = gain;
            s.q.extend(samples);
            let max = rate as usize * 30;
            if s.q.len() > max { let extra = s.q.len() - max; s.q.drain(..extra); }
        }
    }

    /// Leaving the room: microphone and speakers off.
    pub fn stop(&mut self) {
        self.set_talking(None);
        self.mic = None;
        self.out = None;
        if let Ok(mut sp) = self.speakers.lock() { sp.clear(); }
        self.heard.clear();
    }

    /// Is this player talking right now? (Some(radio))
    pub fn is_talking(&self, id: &str, now: f64) -> Option<bool> {
        self.heard.get(id).filter(|(t, _)| now - t < 0.4).map(|(_, r)| *r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A voice survives the trip: 48 kHz mic sound -> 8 kHz mu-law -> base64 -> back, still the same tone.
    #[test]
    fn voice_round_trip() {
        let tone: Vec<f32> = (0..4800).map(|i| 0.5 * (i as f32 / 48000.0 * 440.0 * std::f32::consts::TAU).sin()).collect();
        let bytes = encode(&tone, 48000);
        assert_eq!(bytes.len(), 800, "100 ms at 8 kHz");
        let back = decode(&b64_decode(&b64_encode(&bytes)));
        assert_eq!(back.len(), 800);
        // Compare with the tone sampled at 8 kHz (allowing for the averaging and mu-law's small steps).
        for (i, v) in back.iter().enumerate().skip(2) {
            let want = 0.5 * ((i as f32 * 6.0 + 2.5) / 48000.0 * 440.0 * std::f32::consts::TAU).sin();
            assert!((v - want).abs() < 0.06, "sample {i}: {v} vs {want}");
        }
        for x in [-1.0f32, -0.3, 0.0, 0.01, 0.7, 1.0] { assert!((mulaw_decode(mulaw_encode(x)) - x).abs() < 0.04, "{x}"); }
        assert_eq!(b64_decode(&b64_encode(b"hi!?")), b"hi!?");
    }
}
