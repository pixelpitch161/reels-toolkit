//! BASS GUITAR's voice (a from-scratch fighter, 2026-11-21): a picked electric bass. Two Karplus-Strong strings (the
//! shared `bf_sound::voice::pp::pluck`, the ACOUSTIC's), a hair apart, darker and longer-ringing than a guitar's,
//! through a low-pass for the pickup's warmth, with the pick's click on top (a short bright pluck two octaves up) and
//! a quick palm-mute fade when the note is let go. Made at 44,100 Hz and resampled to 48,000 Hz, like the other Pixel
//! Pitch voices. It draws no noise: its strings come from their own seeded generators.

use bf_sound::voice::pp;
use bf_sound::voice::pp::pluck::ks_pluck;
use bf_sound::voice::real::{Correction, Fill, Rework};
use bf_sound::voice::{PpNoise, Voice};

/// BASS GUITAR's register: a four-string bass, E1 to G3 (and the upper frets to C4).
pub const REGISTER: (i32, i32) = (28, 60);

/// The BASS GUITAR's added overtones: the harmonics its own note barely has, laid in at a real finger bass's level (dB
/// under the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (28, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (29, [-99.0, 2.9, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (30, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (31, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (32, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (33, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (34, [-99.0, 1.1, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (35, [-99.0, 3.7, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (36, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (37, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (38, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (39, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (40, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (41, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (42, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (43, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (44, [-99.0, -99.0, 13.2, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (45, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (46, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (47, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (48, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (49, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (50, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (51, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (52, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (53, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (54, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (55, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (56, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (57, [-99.0, 0.1, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (58, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (59, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (60, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
];

/// The BASS GUITAR's overtone correction toward a real finger bass: gains in dB for its first eight harmonics, note by
/// note (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "BASS GUITAR"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (28, [-2.6, -0.6, 9.9, 0.4, -9.5, -15.0, -5.7, 4.9]),
    (29, [1.2, 0.0, 11.3, -7.5, -4.3, -11.7, -5.6, -4.4]),
    (30, [-1.0, 9.7, 4.8, -14.8, -6.4, -16.8, -16.0, -22.2]),
    (31, [-0.2, 9.7, -2.9, -17.9, -26.6, -10.3, -8.2, -22.1]),
    (32, [-2.6, 9.7, -9.9, -15.8, -30.0, -13.0, -20.3, -23.7]),
    (33, [0.7, 13.5, 0.0, 11.7, -5.4, -1.8, -13.6, -15.9]),
    (34, [0.2, -2.2, 0.0, 12.0, -6.7, -10.8, -8.4, -17.2]),
    (35, [0.9, 0.0, -1.4, 3.1, -5.8, -15.6, -14.6, -11.6]),
    (36, [-0.3, 14.6, -3.5, -2.7, -1.9, -16.2, -16.0, -8.3]),
    (37, [0.5, 9.1, -3.5, -6.0, -9.0, -19.2, -15.5, -19.1]),
    (38, [-0.8, 3.9, 2.2, 0.2, -10.6, -18.4, -24.2, -18.0]),
    (39, [-2.7, -2.3, 3.1, 8.2, -12.0, -22.1, -24.0, -15.2]),
    (40, [-2.1, 1.0, -0.2, 3.4, -21.1, -11.4, -18.7, -10.0]),
    (41, [-2.1, 0.0, -0.4, 0.8, -17.3, -13.5, -15.5, -9.3]),
    (42, [-4.7, 0.5, -2.5, -8.1, -10.0, -19.2, -15.8, -3.0]),
    (43, [-4.0, 0.0, 7.1, -11.7, -7.2, -17.0, -19.3, -16.7]),
    (44, [-1.6, 0.5, -1.0, -9.1, -6.9, -15.6, -18.8, -14.2]),
    (45, [24.0, 0.0, 13.5, -3.8, -6.7, -6.5, -17.3, -8.4]),
    (46, [1.2, 0.0, 11.9, -4.8, -5.9, -14.6, -21.4, -16.4]),
    (47, [-1.3, 0.0, 5.6, -3.1, -4.1, -13.4, -16.0, -16.9]),
    (48, [7.8, 0.0, -0.2, -18.9, -30.0, -5.5, -7.6, -24.8]),
    (49, [1.6, 1.0, -0.4, -23.0, -27.5, -6.8, -11.0, -27.8]),
    (50, [0.4, 4.0, 2.7, -22.7, -28.6, -2.3, -10.7, -27.1]),
    (51, [3.5, 1.8, 12.3, -23.9, -23.9, -2.1, -11.0, -24.4]),
    (52, [7.4, 0.0, 0.7, -24.1, -7.7, -8.9, -9.5, -27.2]),
    (53, [2.9, 1.2, -3.1, -24.7, -25.3, -8.8, -12.0, -28.8]),
    (54, [3.7, 0.0, -1.2, -17.2, -23.4, -9.0, -8.3, -27.5]),
    (55, [5.5, 0.0, -2.0, -13.8, -23.1, -6.6, -6.0, -29.5]),
    (56, [4.1, 2.9, -3.2, -19.3, -25.3, -9.8, -6.7, -30.0]),
    (57, [4.1, 0.0, -0.5, -16.0, -22.8, -8.7, -7.2, -25.2]),
    (58, [4.3, 3.4, -1.5, -28.5, -23.7, -7.4, -12.6, -30.0]),
    (59, [1.8, 2.6, 2.8, -22.9, -26.3, -7.6, -1.7, -27.0]),
    (60, [0.0, 2.9, -1.1, -21.1, -25.2, -9.7, -6.6, -18.8]),
];

/// The Composed arrangement's BASS GUITAR (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, aimed 1 dB per harmonic brighter (FluidR3's finger bass is nearly silent past its fourth, so on a
/// phone its notes would vanish), the ones it lacks added, and a quicker stop (a 20 ms fade after its length, the
/// real one's damped string); it keeps its own pluck's snap.
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.0,
    stop: 0.02,
    keep_shape: true,
};

/// BASS GUITAR's voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct BassGuitarVoice;

impl Voice for BassGuitarVoice {
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
        "BASS GUITAR"
    }
}

/// One bass note at 44,100 Hz, before it is resampled: it rings for its held length (at least 0.12 s, at most 2 s)
/// and fades out over 0.08 s after it.
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    let f = pp::midi_hz(midi);
    let held = dur.clamp(0.12, 2.0);
    let ring = held + 0.08;
    let a = ks_pluck(f, ring, 0.55, 0.9994, 3);
    let b = ks_pluck(f * 1.0018, ring, 0.55, 0.9994, 61);
    let body: Vec<f64> = a.iter().zip(&b).map(|(&a, &b)| a * 0.7 + b * 0.3).collect();
    let mut x = pp::lp(&body, 1700.0);
    let click = ks_pluck(f * 4.0, 0.03, 1.0, 0.99, 11);
    for (v, c) in x.iter_mut().zip(&click) {
        *v += c * 0.18;
    }
    let n_held = pp::len(held);
    let n_fade = pp::len(0.08).max(1);
    for (i, v) in x.iter_mut().enumerate() {
        let g = if i < n_held {
            1.0
        } else {
            (1.0 - (i - n_held) as f64 / n_fade as f64).max(0.0)
        };
        *v *= g * vel * 1.6;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bass_is_its_own_pluck_and_draws_no_noise() {
        let mut noise = PpNoise::start();
        let note = BassGuitarVoice.note(&mut noise, 40.0, 0.9, 0.3);
        assert_eq!(note, pp::to_48k(&synth(40.0, 0.9, 0.3)));
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert!(note.iter().all(|v| v.is_finite()));
        assert!(note.iter().any(|v| v.abs() > 0.01), "it sounds");
        assert_ne!(
            synth(52.0, 0.9, 0.3),
            crate::fighters::acoustic::voice::synth(52.0, 0.9, 0.3)
        );
        assert_eq!(
            (BassGuitarVoice.register(), BassGuitarVoice.name()),
            (REGISTER, "BASS GUITAR")
        );
    }
}
