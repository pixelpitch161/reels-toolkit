//! CONTRABASS CLARINET's voice: BASS CLARINET's reed synth ([`crate::fighters::bass_clarinet::voice::bass_reed`])
//! an octave lower, with a growl that grows toward the bottom of its range: the reed flaps so slowly down there that
//! each flap is heard (the "motorboat").

use bf_sound::voice::pp;
use bf_sound::voice::{PpNoise, Voice};

use crate::fighters::bass_clarinet::voice::bass_reed;

/// Its register: D1 to Bb3.
pub const REGISTER: (i32, i32) = (26, 58);

/// The growl at MIDI note `m`: 0.6 at D1, easing to 0.15 by D3 and none above.
pub fn growl(m: f64) -> f64 {
    if m >= 50.0 {
        0.0
    } else {
        0.15 + 0.45 * ((50.0 - m) / 24.0).clamp(0.0, 1.0)
    }
}

/// CONTRABASS CLARINET's voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct ContraVoice;

impl Voice for ContraVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&bass_reed(pp::midi_hz(midi), dur, vel, growl(midi)))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "CONTRABASS CLARINET"
    }

    /// Down to -0.16 s, as the reed synth's.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_low_note_sounds_and_growls() {
        let mut noise = PpNoise::start();
        let x = ContraVoice.note(&mut noise, 26.0, 0.9, 0.4);
        assert!(x.iter().all(|v| v.is_finite()) && x.iter().any(|v| v.abs() > 0.01));
        assert!(growl(26.0) > growl(40.0) && growl(60.0) == 0.0);
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
    }
}
