//! FLUGELHORN's voice (a from-scratch fighter, 2026-11-16): the shared brass block (`bf_sound::voice::pp::brass`,
//! the TRUMPET's) with a flugelhorn's tone: the widest conical bore of the trumpet family, so the darkest and
//! softest sound (fewer bright harmonics and a steeper roll-off than CORNET), a gentle scoop into the note, a slow
//! vibrato and a breath of air. Made at 44,100 Hz and resampled to 48,000 Hz. It draws no noise.

use bf_sound::voice::pp;
use bf_sound::voice::pp::brass::{Brass, brass};
use bf_sound::voice::{PpNoise, Voice};

/// FLUGELHORN's register (B-flat flugelhorn, played here from E3 to F5).
pub const REGISTER: (i32, i32) = (52, 77);

/// A flugelhorn's tone: darker and rounder than the CORNET's `bright=.78, tilt=.95`.
const TONE: Brass = Brass {
    bright: 0.55,
    vib: 0.006,
    scoop: 0.03,
    rough: 0.0,
    tilt: 1.2,
};

/// FLUGELHORN's voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct FlugelVoice;

impl Voice for FlugelVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "FLUGELHORN"
    }

    /// Down to -0.12 s, as the brass block's TRUMPET.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.12)
    }
}

/// One flugelhorn note at 44,100 Hz, before it is resampled: the brass block, softened (a lower velocity into it
/// for the same loudness, since a flugelhorn is played softly).
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    brass(pp::midi_hz(midi), dur, vel * 0.85, &TONE)
        .into_iter()
        .map(|v| v * 1.15)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_sounds() {
        let mut noise = PpNoise::start();
        let x = FlugelVoice.note(&mut noise, 64.0, 0.8, 0.4);
        assert!(x.iter().all(|v| v.is_finite()) && x.iter().any(|v| v.abs() > 0.01));
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
    }
}
