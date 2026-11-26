//! FLUGELHORN's voice (a from-scratch fighter, 2026-11-16): the shared brass block (`bf_sound::voice::pp::brass`,
//! the TRUMPET's) with a flugelhorn's tone: the widest conical bore of the trumpet family, so the darkest and
//! softest sound (fewer bright harmonics and a steeper roll-off than CORNET), a gentle scoop into the note, a slow
//! vibrato and a breath of air. Made at 44,100 Hz and resampled to 48,000 Hz. It draws no noise.

use bf_sound::voice::pp;
use bf_sound::voice::pp::brass::{Brass, brass};
use bf_sound::voice::real::{Correction, Fill, Rework, Tail};
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

/// The FLUGELHORN's added overtones: the harmonics its own note barely has, laid in at the level it is aimed at (dB
/// under the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (52, [-99.0, -99.0, 0.5, -2.6, -3.6, -1.5, -5.0, -8.8]),
    (53, [-99.0, -99.0, 0.4, -2.5, -3.7, -1.4, -5.0, -8.9]),
    (54, [-99.0, -99.0, 0.3, -2.4, -3.7, -1.3, -5.0, -9.0]),
    (55, [-99.0, -99.0, 0.3, -2.3, -3.7, -1.2, -5.0, -9.0]),
    (56, [-99.0, -99.0, 0.2, -2.3, -3.7, -1.1, -5.0, -9.0]),
    (57, [-99.0, -99.0, 0.2, -2.2, -3.7, -1.0, -5.0, -8.9]),
    (58, [-99.0, -99.0, 0.2, -2.2, -3.8, -1.0, -5.0, -8.8]),
    (59, [-99.0, -99.0, 0.2, -2.2, -3.9, -0.9, -5.0, -8.7]),
    (60, [-99.0, -99.0, 0.2, -2.2, -4.1, -1.0, -5.1, -8.7]),
    (61, [-99.0, -99.0, 0.1, -2.3, -4.3, -1.2, -5.5, -9.0]),
    (62, [-99.0, -99.0, 0.1, -2.4, -4.7, -1.5, -6.0, -9.5]),
    (63, [-99.0, -99.0, -0.0, -2.6, -5.0, -1.9, -6.5, -10.2]),
    (64, [-99.0, -99.0, -0.0, -2.7, -5.1, -2.0, -6.6, -10.2]),
    (65, [-99.0, -99.0, 0.0, -2.6, -5.0, -1.8, -6.3, -9.7]),
    (66, [-99.0, -99.0, 0.0, -2.6, -5.0, -1.7, -6.2, -9.6]),
    (67, [-99.0, -99.0, 0.4, -0.3, -2.8, -8.4, -11.7, -15.4]),
    (68, [-99.0, -99.0, 0.3, -0.3, -2.9, -8.5, -11.8, -15.6]),
    (69, [-99.0, -99.0, 0.3, -0.4, -2.9, -8.6, -12.0, -15.9]),
    (70, [-99.0, -99.0, 0.3, -0.4, -3.0, -8.7, -12.1, -16.1]),
    (71, [-99.0, -99.0, 0.3, -0.4, -3.0, -8.7, -12.1, -16.1]),
    (72, [-99.0, 5.6, 2.8, -4.4, -5.9, -7.4, -14.0, -21.0]),
    (73, [-99.0, 5.4, 2.1, -5.6, -6.8, -8.0, -14.8, -22.1]),
    (74, [-99.0, 5.2, 1.4, -6.8, -7.7, -8.7, -15.7, -23.5]),
    (75, [-99.0, 5.2, 1.4, -6.8, -8.4, -9.4, -16.5, -99.0]),
    (76, [-99.0, -99.0, -99.0, -6.8, -12.3, -99.0, -99.0, -99.0]),
    (77, [-99.0, -99.0, -4.4, -7.3, -99.0, -99.0, -99.0, -99.0]),
];

/// The FLUGELHORN's overtone correction toward a real trumpet made mellower (1 dB less per harmonic: FluidR3 has no flugelhorn): gains in dB for its first eight harmonics, note by note
/// (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "FLUGELHORN"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (52, [0.9, 7.9, 0.0, 0.3, 1.5, 2.1, 1.8, 2.4]),
    (53, [0.8, 7.7, 0.0, 0.3, 1.6, 2.1, 1.9, 2.6]),
    (54, [0.8, 7.6, 0.0, 0.4, 1.6, 2.2, 2.0, 2.8]),
    (55, [0.7, 7.5, 0.0, 0.4, 1.7, 2.3, 2.2, 3.0]),
    (56, [0.7, 7.5, 0.0, 0.4, 1.7, 2.4, 2.3, 3.2]),
    (57, [0.6, 7.4, 0.0, 0.5, 1.8, 2.5, 2.5, 3.5]),
    (58, [0.6, 7.4, 0.0, 0.5, 1.9, 2.7, 2.7, 3.8]),
    (59, [0.5, 7.4, 0.0, 0.6, 2.0, 2.8, 3.0, 4.2]),
    (60, [0.4, 7.4, 0.0, 0.6, 2.1, 3.0, 3.3, 4.6]),
    (61, [0.3, 7.4, 0.0, 0.6, 2.2, 3.3, 3.6, 5.1]),
    (62, [0.3, 7.4, 0.0, 0.7, 2.4, 3.5, 3.9, 5.1]),
    (63, [0.3, 7.4, -0.0, 0.7, 2.5, 3.8, 4.4, 4.8]),
    (64, [0.2, 7.3, 0.0, 0.8, 2.7, 4.1, 4.7, 4.5]),
    (65, [0.1, 7.3, 0.0, 0.9, 2.9, 4.5, 4.4, 4.3]),
    (66, [0.0, 7.2, 0.0, 1.0, 3.1, 5.0, 4.1, 4.1]),
    (67, [-3.2, 9.0, -3.2, -1.8, 0.4, 2.3, 0.5, 0.9]),
    (68, [-3.3, 8.9, -3.2, -1.6, 0.7, 1.9, 0.2, 0.8]),
    (69, [-3.4, 8.9, -3.2, -1.5, 1.1, 1.6, -0.0, 0.8]),
    (70, [-3.5, 8.9, -3.0, -1.1, 1.3, 1.4, -0.0, 1.0]),
    (71, [-3.5, 9.0, -2.9, -0.8, 1.1, 1.3, 0.0, 1.4]),
    (72, [0.1, 0.0, 1.6, 2.9, 4.4, 4.7, 3.6, 5.2]),
    (73, [0.0, 0.0, 1.7, 3.2, 4.1, 4.7, 3.7, 5.8]),
    (74, [-0.0, 0.0, 1.9, 3.1, 3.9, 4.8, 4.0, 5.3]),
    (75, [-1.0, 0.0, 2.2, 2.8, 3.7, 4.8, 4.1, 14.3]),
    (76, [-5.2, 9.0, 9.4, -3.1, -1.7, 6.4, 8.1, -0.0]),
    (77, [-5.3, 9.1, -3.6, -3.4, 9.2, 5.0, 6.3, -1.5]),
];

/// Its real instrument above the eighth harmonic (sound upgrade, stage 3, S1): the 9th to 12th harmonics beside the
/// 5th to 8th, which [`REWORK`] carries its notes on with (`references -- tail`).
#[rustfmt::skip]
pub const TAIL: &Tail = &[
    (52, [-14.1, -16.6, -20.2, -21.9]),
    (53, [-14.2, -16.8, -20.4, -22.1]),
    (54, [-14.3, -16.9, -20.5, -22.2]),
    (55, [-14.3, -16.9, -20.5, -22.2]),
    (56, [-14.3, -16.9, -20.5, -22.1]),
    (57, [-14.3, -16.8, -20.5, -22.0]),
    (58, [-14.3, -16.8, -20.4, -21.9]),
    (59, [-14.4, -16.7, -20.3, -21.8]),
    (60, [-14.5, -16.8, -20.5, -21.9]),
    (61, [-14.7, -16.9, -20.7, -22.1]),
    (62, [-15.0, -17.3, -21.1, -22.4]),
    (63, [-15.5, -18.1, -21.8, -22.9]),
    (64, [-15.6, -17.9, -22.2, -23.6]),
    (65, [-15.4, -17.5, -21.7, -23.1]),
    (66, [-15.5, -17.6, -21.8, -23.2]),
    (67, [-14.4, -17.9, -21.0, -24.7]),
    (68, [-14.5, -18.2, -21.3, -25.1]),
    (69, [-14.8, -18.6, -21.9, -25.8]),
    (70, [-15.0, -18.8, -22.2, -26.3]),
    (71, [-15.0, -18.9, -22.3, -26.5]),
    (72, [-23.9, -29.5, -28.9, -30.5]),
    (73, [-24.3, -29.8, -29.1, -30.9]),
    (74, [-25.1, -30.2, -29.6, -31.7]),
    (75, [-25.5, -30.7, -30.2, -32.5]),
    (76, [-19.3, -28.4, -25.9, -35.2]),
    (77, [-19.4, -28.6, -26.4, -33.6]),
];

/// The Composed arrangement's FLUGELHORN (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added; its own start and stop, already near the real one's.
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.0,
    stop: 0.0,
    keep_shape: false,
    tail: TAIL,
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

    fn rework(&self) -> Option<&'static Rework> {
        Some(&REWORK)
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
