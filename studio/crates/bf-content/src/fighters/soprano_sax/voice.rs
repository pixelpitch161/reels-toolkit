//! SOPRANO SAX's voice (a from-scratch fighter, 2026-11-16): the shared alto-sax reed block (`bf_sound::voice::pp::alto`,
//! ALTO SAX's) with a soprano's tone: brighter and more nasal (a higher formant with more boost, stronger odd
//! harmonics), a quicker attack and a faster, slightly wider vibrato, so it buzzes. Made at 44,100 Hz and resampled to
//! 48,000 Hz, like the other Pixel Pitch voices. It draws only the alto generator, as ALTO SAX does.

use bf_sound::voice::pp::alto::{AltoTone, alto_tone};
use bf_sound::voice::pp::{self};
use bf_sound::voice::{PpNoise, Voice};

/// SOPRANO SAX's register (B-flat soprano, played here from B-flat 3 to D6).
pub const REGISTER: (i32, i32) = (58, 86);

/// A soprano's tone: brighter, more nasal and buzzier than ALTO SAX's.
pub const TONE: AltoTone = AltoTone {
    bright: 0.9,
    vib: 0.006,
    vrate: 5.8,
    odd: 0.22,
    scoop: 0.025,
    att: 0.02,
    formant: 1650.0,
    fboost: 1.25,
    breath: 0.035,
    growl: 0.0,
    slap: 0.0,
    gl: None,
};

/// Seconds of tail after the held length.
pub const TAIL: f64 = 0.14;

/// SOPRANO SAX's voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct SopranoSaxVoice;

impl Voice for SopranoSaxVoice {
    fn note(&self, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&alto_tone(&mut noise.1.alto, pp::midi_hz(midi), dur, vel, &TONE))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "SOPRANO SAX"
    }

    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + TAIL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::assertions_on_constants)] // the two TONE tables checked against each other
    fn a_note_is_finite_and_brighter_than_the_altos() {
        let mut noise = PpNoise::start();
        let x = SopranoSaxVoice.note(&mut noise, 74.0, 0.8, 0.2);
        assert!(!x.is_empty() && x.iter().all(|v| v.is_finite()));
        assert!(TONE.bright > crate::fighters::alto_sax::voice::TONE.bright);
        assert_eq!(
            (SopranoSaxVoice.register(), SopranoSaxVoice.name()),
            (REGISTER, "SOPRANO SAX")
        );
    }
}
