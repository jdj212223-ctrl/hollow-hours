//! Optional haptic vest (bHaptics TactSuit through the bHaptics Player app on this computer).
//! Safety rules from the design:
//!   - OFF until the player turns it on in Settings, and only if a vest app is actually found
//!   - an intensity slider with a hard maximum (never above 60%)
//!   - the panic key (and leaving the game) stops every effect at once
//!   - a warning screen: no strong effects for people with heart conditions
//! Patterns: a "chill" sweeping down the back (jumpscare), a heartbeat when fear is high, a "tense" warm pulse.

use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;

const PLAYER: &str = "127.0.0.1:15881";
pub const MAX_INTENSITY: f32 = 0.6;

pub enum Effect {
    Chill,
    Heartbeat,
    Tense,
    StopAll,
}

pub struct Vest {
    pub found: bool,
    pub enabled: bool,
    /// 0.0..=1.0 of MAX_INTENSITY
    pub strength: f32,
    tx: Option<Sender<(Effect, f32)>>,
}

impl Vest {
    /// Looks for the bHaptics Player app (it listens on localhost). Never connects anywhere else.
    pub fn detect() -> Self {
        let found = PLAYER.parse::<SocketAddr>().ok().and_then(|a| TcpStream::connect_timeout(&a, Duration::from_millis(250)).ok()).is_some();
        let tx = found.then(|| {
            let (tx, rx) = channel::<(Effect, f32)>();
            std::thread::spawn(move || {
                let Ok((mut ws, _)) = tungstenite::connect(format!("ws://{PLAYER}/v2/feedbacks?app_id=hollow-hours&app_name=Hollow%20Hours")) else { return };
                for (effect, level) in rx {
                    let msg = frame(effect, level);
                    if ws.send(tungstenite::Message::Text(msg)).is_err() {
                        return;
                    }
                }
            });
            tx
        });
        Vest { found, enabled: false, strength: 0.5, tx }
    }

    pub fn play(&self, e: Effect) {
        let stop = matches!(e, Effect::StopAll);
        if let Some(tx) = &self.tx {
            if self.enabled || stop {
                let _ = tx.send((e, (self.strength * MAX_INTENSITY).min(MAX_INTENSITY)));
            }
        }
    }
}

/// A bHaptics Player "Submit" frame: dot points on the vest (0-19 front, 0-19 back), intensity 0-100.
fn frame(e: Effect, level: f32) -> String {
    let i = (level * 100.0).round() as u32;
    let (pos, dots, ms): (&str, Vec<u32>, u32) = match e {
        Effect::Chill => ("VestBack", vec![0, 1, 2, 3, 4, 5, 6, 7], 400),
        Effect::Heartbeat => ("VestFront", vec![5, 6, 9, 10], 120),
        Effect::Tense => ("VestFront", vec![8, 9, 10, 11, 12, 13, 14, 15], 600),
        Effect::StopAll => return r#"{"Submit":[{"Type":"turnOffAll","Key":""}]}"#.into(),
    };
    let points: Vec<String> = dots.iter().map(|d| format!(r#"{{"Index":{d},"Intensity":{i}}}"#)).collect();
    format!(
        r#"{{"Submit":[{{"Type":"frame","Key":"hollow","Frame":{{"Position":"{pos}","DotPoints":[{}],"DurationMillis":{ms}}}}}]}}"#,
        points.join(",")
    )
}
