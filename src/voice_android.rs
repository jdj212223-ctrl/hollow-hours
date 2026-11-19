//! Voice chat on phones (Android, iOS): not yet (the microphone needs its own Android permission flow). Same API as voice.rs,
//! so the game is the same everywhere; you still hear the radio clicks and see who's talking.

use std::collections::HashMap;

pub const RANGE: f32 = 12.0;

#[derive(Default)]
pub struct Voice {
    pub talking: Option<bool>,
    pub error: Option<String>,
    pub heard: HashMap<String, (f64, bool)>,
}

impl Voice {
    pub fn set_talking(&mut self, t: Option<bool>) {
        if t.is_some() { self.error = Some("voice chat isn't on phones yet".into()); }
        self.talking = None;
    }
    pub fn take_packet(&mut self) -> Option<serde_json::Value> { None }
    pub fn hear(&mut self, from: &str, _data: &str, radio: bool, _gain: f32, now: f64) {
        self.heard.insert(from.to_string(), (now, radio));
    }
    pub fn stop(&mut self) { self.heard.clear(); }
    pub fn play(&mut self, _who: &str, _samples: Vec<f32>, _rate: u32, _gain: f32) {}
    pub fn stop_ai(&mut self) {}
    pub fn is_talking(&self, id: &str, now: f64) -> Option<bool> {
        self.heard.get(id).filter(|(t, _)| now - t < 0.4).map(|(_, r)| *r)
    }
}
