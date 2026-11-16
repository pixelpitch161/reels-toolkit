//! CONTRABASSOON's voice: the double reed of [`crate::fighters::bassoon::voice::bassoon_reed`] with a rattle (its
//! 31 Hz growl, the reed flapping so slowly you hear it), for the register of the contrabassoon (B flat 0 up).

use bf_sound::voice::pp;
use bf_sound::voice::{PpNoise, Voice};

use crate::fighters::bassoon::voice::bassoon_reed;

/// Its register: B flat 0 to about G4.
pub const REGISTER: (i32, i32) = (22, 67);

/// The rattle's depth (the growl of `bassoon_reed`).
const RATTLE: f64 = 0.22;

/// The seconds a note rings past its held length (`bassoon_reed`'s).
const TAIL: f64 = 0.14;

/// The CONTRABASSOON's voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct ContrabassoonVoice;

impl Voice for ContrabassoonVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&bassoon_reed(pp::midi_hz(midi), dur, vel, RATTLE))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "CONTRABASSOON"
    }

    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + TAIL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_low_note_sounds() {
        let mut noise = PpNoise::start();
        let x = ContrabassoonVoice.note(&mut noise, 34.0, 0.9, 0.4);
        let peak = x.iter().fold(0.0_f64, |a, v| a.max(v.abs()));
        assert!(x.len() > 20_000 && peak > 0.05, "{} samples, peak {peak}", x.len());
    }
}
