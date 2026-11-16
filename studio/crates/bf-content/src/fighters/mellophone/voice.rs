//! MELLOPHONE's voice (a from-scratch fighter, 2026-11-16): the shared brass block (`bf_sound::voice::pp::brass`, the
//! TRUMPET's) with a mellophone's tone: a horn's round, conical sound pushed brighter by the big forward bell (between
//! HORN and TRUMPET), a quick scoop into the note and a little drum-corps vibrato. Made at 44,100 Hz and resampled to
//! 48,000 Hz, like the other Pixel Pitch voices. It draws no noise.

use bf_sound::voice::pp;
use bf_sound::voice::pp::brass::{Brass, brass};
use bf_sound::voice::{PpNoise, Voice};

/// MELLOPHONE's register (the mellophone in F, written F#3 to C6, sounding a fifth lower: played here from F3 to C6).
pub const REGISTER: (i32, i32) = (53, 84);

/// A mellophone's tone: rounder than the TRUMPET's `bright=1.1, tilt=.75`, brighter than CORNET's.
const TONE: Brass = Brass {
    bright: 0.92,
    vib: 0.006,
    scoop: 0.03,
    rough: 0.0,
    tilt: 0.85,
};

/// MELLOPHONE's voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct MellophoneVoice;

impl Voice for MellophoneVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "MELLOPHONE"
    }

    /// Down to -0.12 s, as the brass block's TRUMPET.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.12)
    }
}

/// One mellophone note at 44,100 Hz, before it is resampled.
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    brass(pp::midi_hz(midi), dur, vel, &TONE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mellophone_is_its_own_brass_and_draws_no_noise() {
        let mut noise = PpNoise::start();
        let note = MellophoneVoice.note(&mut noise, 65.0, 0.9, 0.3);
        assert_eq!(note, pp::to_48k(&synth(65.0, 0.9, 0.3)));
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert_ne!(
            synth(65.0, 0.9, 0.3),
            crate::fighters::trumpet::voice::synth(65.0, 0.9, 0.3)
        );
        assert_ne!(
            synth(65.0, 0.9, 0.3),
            crate::fighters::cornet::voice::synth(65.0, 0.9, 0.3)
        );
        assert_eq!(
            (MellophoneVoice.register(), MellophoneVoice.name()),
            (REGISTER, "MELLOPHONE")
        );
    }
}
