//! CORNET's voice (a from-scratch fighter, 2026-11-16): the shared brass block (`bf_sound::voice::pp::brass`, the
//! TRUMPET's) with a cornet's tone: a conical bore's rounder, darker sound (fewer bright harmonics, a steeper roll-off),
//! a wider scoop into the note and a slow, warm vibrato, as early jazz cornet players had. Made at 44,100 Hz and
//! resampled to 48,000 Hz, like the other Pixel Pitch voices. It draws no noise.

use bf_sound::voice::pp;
use bf_sound::voice::pp::brass::{Brass, brass};
use bf_sound::voice::{PpNoise, Voice};

/// CORNET's register (B-flat cornet, written F#3 to C6, played here from G3 to G5).
pub const REGISTER: (i32, i32) = (55, 79);

/// A cornet's tone: darker and rounder than the TRUMPET's `bright=1.1, tilt=.75`.
const TONE: Brass = Brass {
    bright: 0.78,
    vib: 0.008,
    scoop: 0.045,
    rough: 0.0,
    tilt: 0.95,
};

/// CORNET's voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct CornetVoice;

impl Voice for CornetVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "CORNET"
    }

    /// Down to -0.12 s, as the brass block's TRUMPET.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.12)
    }
}

/// One cornet note at 44,100 Hz, before it is resampled.
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    brass(pp::midi_hz(midi), dur, vel, &TONE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cornet_is_darker_than_the_trumpet_and_draws_no_noise() {
        let mut noise = PpNoise::start();
        let note = CornetVoice.note(&mut noise, 67.0, 0.9, 0.3);
        assert_eq!(note, pp::to_48k(&synth(67.0, 0.9, 0.3)));
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert_ne!(
            synth(67.0, 0.9, 0.3),
            crate::fighters::trumpet::voice::synth(67.0, 0.9, 0.3)
        );
        assert_eq!((CornetVoice.register(), CornetVoice.name()), (REGISTER, "CORNET"));
    }
}
