//! The AIs speak: every line a character says is read aloud by the computer's own speech (offline and free):
//! `say` on macOS, the Windows speech engine, espeak-ng / espeak on Linux. Each character has its own voice.
//! The infected get the same voices CORRUPTED: pitched down, metallic, crackling, stuttering, with an echo. A kind AI
//! that's turning glitches more and more as its infection grows. Played through the same mixer as voice chat, so
//! it's quieter the further away they are. Settings > Voice chat > "AI voices" turns it off.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Mutex, OnceLock};

/// A line to say: who (also the mixer's channel), the text, how corrupted (0..1), how loud.
struct Job { who: String, text: String, corrupt: f32, gain: f32, gen: u64 }

/// Bumped by hush(): lines from before it are dropped (not made, not played).
static GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
fn gen() -> u64 { GEN.load(std::sync::atomic::Ordering::Relaxed) }

/// Cancels every line that's waiting or being made (call it with Voice::stop_ai to cut off what's playing).
pub fn hush() { GEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
/// A finished line: who, samples, sample rate, how loud.
pub type Ready = (String, Vec<f32>, u32, f32);
type Made = (String, Vec<f32>, u32, f32, u64);

fn queue() -> &'static (Mutex<Sender<Job>>, Mutex<Receiver<Made>>) {
    static Q: OnceLock<(Mutex<Sender<Job>>, Mutex<Receiver<Made>>)> = OnceLock::new();
    Q.get_or_init(|| {
        let (jobs_tx, jobs_rx) = channel::<Job>();
        let (done_tx, done_rx) = channel::<Made>();
        // One worker: lines are spoken in order, and only one speech program runs at a time.
        std::thread::spawn(move || {
            for job in jobs_rx {
                if job.gen != gen() { continue; }
                if let Some((samples, rate)) = synth(&job.who, &job.text) {
                    let samples = if job.corrupt > 0.02 { corrupt(&samples, rate, job.corrupt) } else { samples };
                    if done_tx.send((job.who, samples, rate, job.gain, job.gen)).is_err() { break; }
                }
            }
        });
        (Mutex::new(jobs_tx), Mutex::new(done_rx))
    })
}

/// Says a line (in the background). `corrupt` 0 = clean, 1 = fully infected.
pub fn speak(who: &str, text: &str, corrupt: f32, gain: f32) {
    if cfg!(any(target_os = "android", target_os = "ios")) || gain <= 0.01 { return; }
    let text: String = text.chars().filter(|c| !c.is_control()).take(300).collect();
    if text.trim().is_empty() { return; }
    if let Ok(tx) = queue().0.lock() {
        let _ = tx.send(Job { who: who.to_string(), text, corrupt: corrupt.clamp(0.0, 1.0), gain, gen: gen() });
    }
}

/// Lines that are ready to play.
pub fn ready() -> Vec<Ready> {
    let now = gen();
    queue().1.lock().map(|rx| rx.try_iter().filter(|m| m.4 == now).map(|(w, s, r, g, _)| (w, s, r, g)).collect()).unwrap_or_default()
}

/// Each character's voice: (macOS voice, words per minute, female?).
fn voice_for(who: &str) -> (&'static str, u32, bool) {
    match who {
        "Ember" => ("Samantha", 180, true),
        "Atlas" => ("Daniel", 165, false),
        "Verse" => ("Karen", 190, true),
        "Kigo" => ("Moira", 205, true),
        "Clove" => ("Tessa", 175, true),
        "Forge" => ("Reed", 180, false),
        "Palette" => ("Shelley", 180, true),
        "Spark" => ("Flo", 205, true),
        "Twin" => ("Sandy", 185, true),
        "Deepa" => ("Rishi", 170, false),
        "Volt" => ("Eddy", 215, false),
        "Night Commuter" => ("Grandpa", 150, false),
        w if w.contains('-') || w.contains("Unit") || w == "Conductor" => (if w.len() % 2 == 0 { "Zarvox" } else { "Trinoids" }, 170, false),
        w => {
            const ANY: [(&str, bool); 6] = [("Samantha", true), ("Daniel", false), ("Karen", true), ("Moira", true), ("Tessa", true), ("Reed", false)];
            let k = w.bytes().fold(0usize, |a, b| a.wrapping_mul(31).wrapping_add(b as usize)) % ANY.len();
            (ANY[k].0, 180, ANY[k].1)
        }
    }
}

/// Runs the computer's speech engine into a WAV file and reads it back: (samples -1..1, sample rate).
fn synth(who: &str, text: &str) -> Option<(Vec<f32>, u32)> {
    use std::process::{Command, Stdio};
    let (voice, wpm, female) = voice_for(who);
    let out = std::env::temp_dir().join(format!("hh-tts-{}.wav", std::process::id()));
    let _ = std::fs::remove_file(&out);
    let quiet = |c: &mut Command| { c.stdout(Stdio::null()).stderr(Stdio::null()); };
    // A speech program that hangs (it happens with some voices) would freeze every voice after it: give up after 20 s.
    fn finish(c: &mut Command) -> bool {
        let Ok(mut child) = c.spawn() else { return false };
        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return status.success(),
                Ok(None) if start.elapsed() < std::time::Duration::from_secs(20) => std::thread::sleep(std::time::Duration::from_millis(50)),
                _ => { let _ = child.kill(); let _ = child.wait(); return false; }
            }
        }
    }
    let ok = if cfg!(target_os = "macos") {
        let mut c = Command::new("say");
        c.args(["-v", voice, "-r", &wpm.to_string(), "-o"]).arg(&out).arg("--data-format=LEI16@22050").arg("--").arg(text);
        quiet(&mut c);
        let first = finish(&mut c);
        // That voice isn't installed: the default one.
        first || {
            let mut c = Command::new("say");
            c.args(["-r", &wpm.to_string(), "-o"]).arg(&out).arg("--data-format=LEI16@22050").arg("--").arg(text);
            quiet(&mut c);
            finish(&mut c)
        }
    } else if cfg!(windows) {
        // The text goes in through an environment variable, never into the script itself.
        let script = format!(
            "Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
             try {{ $s.SelectVoiceByHints([System.Speech.Synthesis.VoiceGender]::{}) }} catch {{}}; $s.Rate = {}; \
             $s.SetOutputToWaveFile($env:HH_TTS_OUT); $s.Speak($env:HH_TTS_TEXT); $s.Dispose()",
            if female { "Female" } else { "Male" }, ((wpm as i32 - 180) / 15).clamp(-5, 5));
        let mut c = Command::new("powershell");
        c.args(["-NoProfile", "-NonInteractive", "-Command", &script]).env("HH_TTS_OUT", &out).env("HH_TTS_TEXT", text);
        quiet(&mut c);
        finish(&mut c)
    } else {
        let v = if female { "en+f3" } else { "en+m3" };
        ["espeak-ng", "espeak"].iter().any(|prog| {
            let mut c = Command::new(prog);
            c.args(["-v", v, "-s", &wpm.to_string(), "-w"]).arg(&out).arg("--").arg(text);
            quiet(&mut c);
            finish(&mut c)
        })
    };
    if !ok { return None; }
    let bytes = std::fs::read(&out).ok()?;
    let _ = std::fs::remove_file(&out);
    read_wav(&bytes)
}

/// A 16-bit PCM WAV (any extra chunks, mono or stereo) as mono samples.
pub fn read_wav(b: &[u8]) -> Option<(Vec<f32>, u32)> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" { return None; }
    let (mut i, mut channels, mut rate, mut bits) = (12usize, 1u16, 22050u32, 16u16);
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let len = u32::from_le_bytes(b[i + 4..i + 8].try_into().ok()?) as usize;
        let body = b.get(i + 8..(i + 8 + len).min(b.len()))?;
        if id == b"fmt " && body.len() >= 16 {
            channels = u16::from_le_bytes([body[2], body[3]]).max(1);
            rate = u32::from_le_bytes(body[4..8].try_into().ok()?);
            bits = u16::from_le_bytes([body[14], body[15]]);
        } else if id == b"data" {
            if bits != 16 { return None; }
            let frame = 2 * channels as usize;
            let s = body.chunks_exact(frame).map(|f| i16::from_le_bytes([f[0], f[1]]) as f32 / 32768.0).collect();
            return Some((s, rate));
        }
        i += 8 + len + (len & 1);
    }
    None
}

/// The infected's voice: lower, metallic, crushed, stuttering, echoing. `a` 0..1 is how far gone.
pub fn corrupt(s: &[f32], rate: u32, a: f32) -> Vec<f32> {
    // Pitch down (play it slower).
    let speed = 1.0 - 0.22 * a;
    let n = (s.len() as f32 / speed) as usize;
    let mut out: Vec<f32> = (0..n).map(|i| {
        let p = i as f32 * speed;
        let k = p as usize;
        let f = p - k as f32;
        s.get(k).copied().unwrap_or(0.0) * (1.0 - f) + s.get(k + 1).copied().unwrap_or(0.0) * f
    }).collect();
    // Stutter: now and then a little piece repeats ("j-j-join us").
    let chunk = (rate as f32 * 0.07) as usize;
    let mut seed: u32 = 0x5EED_1234 ^ s.len() as u32;
    let mut rnd = || { seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5; seed as f32 / u32::MAX as f32 };
    let mut stuttered = Vec::with_capacity(out.len() * 5 / 4);
    for (k, c) in out.chunks(chunk.max(1)).enumerate() {
        stuttered.extend_from_slice(c);
        if k > 2 && rnd() < 0.12 * a { stuttered.extend_from_slice(c); }
    }
    out = stuttered;
    // Metallic (ring modulation), crushed bits, a crackle, and an echo.
    let levels = 2f32.powf(15.0 - 11.0 * a);
    let echo = (rate as f32 * 0.13) as usize;
    let dry = out.clone();
    for (i, x) in out.iter_mut().enumerate() {
        let t = i as f32 / rate as f32;
        let ring = (1.0 - 0.55 * a) + 0.55 * a * (std::f32::consts::TAU * 37.0 * t).sin();
        let mut v = *x * ring;
        v = (v * levels).round() / levels;
        if rnd() < 0.002 * a { v += (rnd() - 0.5) * 0.6 * a; }
        if i >= echo { v += dry[i - echo] * 0.35 * a; }
        *x = (v * 1.1).clamp(-1.0, 1.0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A WAV like macOS `say` makes (with a JUNK chunk first) reads back; corrupting makes it longer and changes it.
    #[test]
    fn reads_say_wavs_and_corrupts() {
        let samples: Vec<i16> = (0..2205).map(|i| ((i as f32 * 0.1).sin() * 12000.0) as i16).collect();
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut w = b"RIFF\0\0\0\0WAVE".to_vec();
        w.extend(b"JUNK"); w.extend(28u32.to_le_bytes()); w.extend([0u8; 28]);
        w.extend(b"fmt "); w.extend(16u32.to_le_bytes());
        w.extend(1u16.to_le_bytes()); w.extend(1u16.to_le_bytes()); w.extend(22050u32.to_le_bytes());
        w.extend(44100u32.to_le_bytes()); w.extend(2u16.to_le_bytes()); w.extend(16u16.to_le_bytes());
        w.extend(b"data"); w.extend((data.len() as u32).to_le_bytes()); w.extend(&data);
        let (s, rate) = read_wav(&w).expect("read");
        assert_eq!((s.len(), rate), (2205, 22050));
        let c = corrupt(&s, rate, 1.0);
        assert!(c.len() > s.len(), "slower and stuttered");
        assert!(c.iter().all(|x| x.abs() <= 1.0));
    }
}

#[cfg(all(test, target_os = "macos"))]
mod mac_tests {
    /// The real speech engine on this Mac makes Atlas's line, and the corrupted version is different and longer.
    /// Manual (cargo test -- --ignored): it depends on macOS's own speech service, which sometimes stalls.
    #[test]
    #[ignore]
    fn opus_speaks_and_corrupts() {
        let (s, rate) = super::synth("Atlas", "Stay close to me.").expect("say made a WAV");
        assert!(rate > 8000 && s.len() > rate as usize / 2, "at least half a second of speech: {} samples at {rate}", s.len());
        assert!(s.iter().any(|x| x.abs() > 0.05), "not silent");
        let c = super::corrupt(&s, rate, 1.0);
        assert!(c.len() > s.len());
        let (r, _) = super::synth("Clerk-07", "you have not paid.").expect("a robot voice too");
        assert!(!r.is_empty());
        // HH_TTS_DEMO=dir: save clean and corrupted examples to listen to.
        if let Ok(dir) = std::env::var("HH_TTS_DEMO") {
            let save = |name: &str, x: &[f32]| {
                let data: Vec<u8> = x.iter().flat_map(|v| ((v.clamp(-1.0, 1.0) * 32000.0) as i16).to_le_bytes()).collect();
                let mut w = b"RIFF".to_vec(); w.extend((36 + data.len() as u32).to_le_bytes()); w.extend(b"WAVEfmt ");
                w.extend(16u32.to_le_bytes()); w.extend(1u16.to_le_bytes()); w.extend(1u16.to_le_bytes()); w.extend(rate.to_le_bytes());
                w.extend((rate * 2).to_le_bytes()); w.extend(2u16.to_le_bytes()); w.extend(16u16.to_le_bytes());
                w.extend(b"data"); w.extend((data.len() as u32).to_le_bytes()); w.extend(data);
                std::fs::write(std::path::Path::new(&dir).join(name), w).unwrap();
            };
            let (line, _) = super::synth("Atlas", "Stay close to me. We'll keep you safe.").unwrap();
            save("opus-clean.wav", &line);
            save("opus-infected.wav", &super::corrupt(&line, rate, 1.0));
            save("opus-turning.wav", &super::corrupt(&line, rate, 0.4));
        }
    }
}
