//! CORNET's voice (a from-scratch fighter, 2026-11-16): the shared brass block (`bf_sound::voice::pp::brass`, the
//! TRUMPET's) with a cornet's tone: a conical bore's rounder, darker sound (fewer bright harmonics, a steeper roll-off),
//! a wider scoop into the note and a slow, warm vibrato, as early jazz cornet players had. Made at 44,100 Hz and
//! resampled to 48,000 Hz, like the other Pixel Pitch voices. It draws no noise.

use bf_sound::voice::pp;
use bf_sound::voice::pp::brass::{Brass, brass};
use bf_sound::voice::real::{Correction, Fill, Rework};
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

/// The CORNET's added overtones: the harmonics its own note barely has, laid in at the level it is aimed at (dB
/// under the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (55, [-99.0, -99.0, -99.0, -0.8, -1.7, 1.3, -2.0, -5.5]),
    (56, [-99.0, -99.0, -99.0, -0.8, -1.7, 1.4, -2.0, -5.5]),
    (57, [-99.0, -99.0, -99.0, -0.7, -1.7, 1.5, -2.0, -5.4]),
    (58, [-99.0, -99.0, -99.0, -0.7, -1.8, 1.5, -2.0, -5.3]),
    (59, [-99.0, -99.0, -99.0, -0.7, -1.9, 1.6, -2.0, -5.2]),
    (60, [-99.0, -99.0, -99.0, -0.7, -2.1, 1.5, -2.1, -5.2]),
    (61, [-99.0, -99.0, -99.0, -0.8, -2.3, 1.3, -2.5, -5.5]),
    (62, [-99.0, -99.0, -99.0, -0.9, -2.6, 1.0, -3.0, -6.0]),
    (63, [-99.0, -99.0, -99.0, -1.1, -3.0, 0.6, -3.5, -6.7]),
    (64, [-99.0, -99.0, 1.0, -1.2, -3.1, 0.5, -3.6, -6.7]),
    (65, [-99.0, -99.0, 1.0, -1.1, -3.0, 0.8, -3.3, -6.2]),
    (66, [-99.0, -99.0, 1.0, -1.1, -3.0, 0.8, -3.2, -6.1]),
    (67, [-99.0, -99.0, 1.4, 1.2, -0.8, -5.9, -8.7, -11.9]),
    (68, [-99.0, -99.0, 1.3, 1.2, -0.9, -6.0, -8.8, -12.1]),
    (69, [-99.0, -99.0, 1.3, 1.1, -0.9, -6.1, -9.0, -12.4]),
    (70, [-99.0, -99.0, 1.3, 1.1, -1.0, -6.2, -9.1, -12.6]),
    (71, [-99.0, -99.0, 1.3, 1.1, -1.0, -6.2, -9.1, -12.6]),
    (72, [-99.0, 6.1, 3.8, -2.9, -3.9, -4.9, -11.0, -99.0]),
    (73, [-99.0, -99.0, 3.1, -4.1, -4.8, -5.5, -11.8, -99.0]),
    (74, [-99.0, -99.0, 2.4, -99.0, -5.7, -6.2, -12.7, -99.0]),
    (75, [-99.0, 5.8, 2.4, -99.0, -6.4, -6.9, -13.5, -99.0]),
    (76, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (77, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (78, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (79, [-99.0, -99.0, -99.0, -5.5, -99.0, -99.0, -99.0, -99.0]),
];

/// The CORNET's overtone correction toward a real trumpet made a little mellower (0.5 dB less per harmonic: FluidR3 has no cornet): gains in dB for its first eight harmonics, note by note
/// (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "CORNET"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (55, [-1.6, 3.7, 13.0, -2.4, -0.2, 1.0, 1.0, 2.4]),
    (56, [-1.8, 3.6, 12.9, -2.4, -0.1, 1.2, 1.3, 2.9]),
    (57, [-1.9, 3.5, 12.8, -2.3, 0.0, 1.4, 1.6, 3.0]),
    (58, [-1.9, 3.4, 12.8, -2.3, 0.1, 1.6, 2.1, 2.6]),
    (59, [-2.0, 3.4, 12.8, -2.2, 0.3, 1.9, 2.4, 2.3]),
    (60, [-2.0, 3.4, 12.8, -2.1, 0.6, 2.4, 2.1, 2.1]),
    (61, [-1.9, 3.6, 13.0, -1.8, 1.0, 3.0, 2.0, 2.1]),
    (62, [-1.9, 3.7, 13.1, -1.6, 1.5, 3.3, 1.9, 2.2]),
    (63, [-2.0, 3.6, 13.0, -1.5, 1.7, 2.9, 1.5, 2.0]),
    (64, [-0.2, 5.8, 0.0, 1.3, 4.3, 4.6, 3.4, 4.1]),
    (65, [-0.3, 5.7, 0.0, 1.4, 4.4, 4.2, 3.2, 4.1]),
    (66, [-0.6, 5.6, 0.0, 1.7, 4.0, 3.9, 3.1, 4.3]),
    (67, [-3.0, 8.2, -2.3, 0.0, 1.4, 2.0, 0.5, 2.3]),
    (68, [-3.3, 8.0, -2.3, 0.3, 0.9, 1.8, 0.5, 2.8]),
    (69, [-3.6, 7.8, -2.3, 0.1, 0.5, 1.6, 0.7, 2.1]),
    (70, [-3.6, 7.9, -2.1, -0.2, 0.3, 1.7, 1.2, 1.7]),
    (71, [-3.7, 7.9, -1.7, -0.5, 0.3, 2.0, 1.2, 1.4]),
    (72, [-1.1, 0.0, 2.8, 2.2, 3.7, 5.7, 3.4, 14.2]),
    (73, [-8.5, 7.7, -4.8, -5.7, -3.5, -0.9, -4.2, 6.0]),
    (74, [-8.2, 7.8, -4.7, 7.3, -3.6, -1.2, -4.3, 5.2]),
    (75, [-1.4, 0.0, 2.3, 14.1, 3.6, 5.2, 2.3, 11.6]),
    (76, [-5.5, 8.3, 7.0, 8.7, 8.8, 2.6, 4.2, -2.7]),
    (77, [-5.4, 8.5, 6.5, 8.3, 6.9, 1.2, 3.6, -4.3]),
    (78, [-5.4, 8.8, 6.3, 8.6, 7.2, 1.5, 3.1, -4.9]),
    (79, [-5.6, 9.1, 6.4, -3.9, 7.7, 2.1, 2.6, -4.7]),
];

/// The Composed arrangement's CORNET (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added; its own start and stop, already near the real one's.
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.0,
    stop: 0.0,
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

    fn rework(&self) -> Option<&'static Rework> {
        Some(&REWORK)
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
