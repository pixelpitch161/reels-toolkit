//! OCTOBASS's voice: CONTRABASS's bowed and plucked string synth ([`crate::fighters::contrabass::voice::synth`]) at
//! the octobass's pitch, with the same note an octave up mixed in under it so its lowest notes still read on a
//! phone's small speakers (the real instrument's fundamental is felt more than heard).

use bf_sound::voice::engine2::bowed::BowedParts;
use bf_sound::voice::engine2::space::{Home, Space};
use bf_sound::voice::engine2::{Instrument, Parts};
use bf_sound::voice::pp;
use bf_sound::voice::{PpNoise, Voice};

use crate::fighters::contrabass::voice::synth;

/// Its register: Bb0 to Bb2.
pub const REGISTER: (i32, i32) = (22, 46);

/// The Composed arrangement's OCTOBASS (Voice Engine 2): built from how it works, on the bowed-string engine (Aedan's
/// pick Oc2, the growl: three strings over two metres long bowed nearer the bridge and pressed harder, through a body
/// twice the contrabass's with more of its low middle), with every technique it has, in a light room of its own
/// (`docs/studio/voice-engine-2-instruments.md`, "The bowed strings"). Classic keeps the note below.
pub static ENGINE2: Instrument = Instrument::built(
    "OCTOBASS",
    Parts::Bowed(BowedParts::OC2),
    Space {
        width: 0.0,
        tilt: 0.0,
        home: Some(Home {
            length: 3.3,
            under: 14.0,
            hp: 120.0,
        }),
        amp: None,
    },
);

/// How much of the octave above is mixed in.
const UPPER: f64 = 0.55;

/// OCTOBASS's voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct OctobassVoice;

impl Voice for OctobassVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        let low = synth(midi.round(), vel, dur);
        let high = synth(midi.round() + 12.0, vel, dur);
        let n = low.len().max(high.len());
        let x: Vec<f64> = (0..n)
            .map(|i| low.get(i).copied().unwrap_or(0.0) + UPPER * high.get(i).copied().unwrap_or(0.0))
            .collect();
        pp::to_48k(&x)
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn engine2(&self) -> Option<&'static Instrument> {
        Some(&ENGINE2)
    }

    fn name(&self) -> &str {
        "OCTOBASS"
    }

    /// Any finite length, as the contrabass synth's.
    fn can_hold(&self, dur: f64) -> bool {
        dur.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_low_note_sounds() {
        let mut noise = PpNoise::start();
        let x = OctobassVoice.note(&mut noise, 22.0, 0.9, 0.8);
        assert!(x.iter().all(|v| v.is_finite()) && x.iter().any(|v| v.abs() > 0.01));
    }
}
