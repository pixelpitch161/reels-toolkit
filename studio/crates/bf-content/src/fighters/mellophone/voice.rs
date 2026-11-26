//! MELLOPHONE's voice (a from-scratch fighter, 2026-11-16): the shared brass block (`bf_sound::voice::pp::brass`, the
//! TRUMPET's) with a mellophone's tone: a horn's round, conical sound pushed brighter by the big forward bell (between
//! HORN and TRUMPET), a quick scoop into the note and a little drum-corps vibrato. Made at 44,100 Hz and resampled to
//! 48,000 Hz, like the other Pixel Pitch voices. It draws no noise.

use bf_sound::voice::pp;
use bf_sound::voice::pp::brass::{Brass, brass};
use bf_sound::voice::real::{Correction, Fill, Rework};
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

/// The MELLOPHONE's added overtones: the harmonics its own note barely has, laid in at the level it is aimed at (dB
/// under the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (53, [-99.0, -99.0, 8.7, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (54, [-99.0, -99.0, 9.1, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (55, [-99.0, -99.0, 9.3, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (56, [-99.0, -99.0, 9.3, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (57, [-99.0, -99.0, 9.1, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (58, [-99.0, -99.0, 8.8, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (59, [-99.0, 9.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (60, [-99.0, 9.3, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (61, [-99.0, 9.3, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (62, [-99.0, 9.2, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (63, [-99.0, 8.9, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (64, [-99.0, 8.8, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (65, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (66, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (67, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (68, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (69, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (70, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (71, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (72, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (73, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (74, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (75, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (76, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (77, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (78, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (79, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (80, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (81, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -44.4]),
    (82, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -43.5]),
    (83, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -43.1]),
    (84, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -39.0, -43.4]),
];

/// The MELLOPHONE's overtone correction toward a real french horn made a little brighter (0.5 dB more per harmonic: FluidR3 has no mellophone): gains in dB for its first eight harmonics, note by note
/// (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "MELLOPHONE"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (53, [0.2, 9.2, 0.0, 10.7, 12.2, 10.3, 8.4, 10.1]),
    (54, [0.2, 10.4, 0.0, 11.0, 12.3, 11.1, 9.0, 10.3]),
    (55, [0.2, 11.8, 0.0, 11.1, 12.1, 12.0, 9.7, 10.4]),
    (56, [0.2, 13.2, 0.0, 10.8, 11.4, 12.8, 10.4, 10.8]),
    (57, [0.1, 14.6, 0.0, 10.1, 10.7, 13.1, 11.1, 11.5]),
    (58, [0.1, 15.7, 0.0, 9.4, 10.5, 13.1, 11.9, 12.0]),
    (59, [0.7, 0.0, 11.2, 1.1, -0.4, 13.0, 6.5, 6.1]),
    (60, [0.6, 0.0, 10.0, 1.0, 0.1, 12.2, 6.3, 6.4]),
    (61, [0.6, 0.0, 8.9, 1.3, 0.7, 11.3, 6.3, 6.3]),
    (62, [0.6, 0.0, 8.0, 1.7, 1.7, 11.0, 6.5, 5.4]),
    (63, [0.7, 0.0, 7.6, 1.8, 3.1, 11.3, 6.7, 4.0]),
    (64, [0.7, 0.0, 7.4, 1.6, 4.5, 12.1, 6.7, 3.8]),
    (65, [0.0, 6.2, 4.2, 3.7, -1.0, 0.4, 6.0, 0.9]),
    (66, [0.0, 4.8, 3.4, 2.8, -1.5, 0.5, 4.7, 0.5]),
    (67, [0.0, 3.1, 2.5, 1.8, -1.6, -0.4, 3.0, -0.5]),
    (68, [0.0, 1.3, 1.9, 0.9, -2.2, -1.8, 1.0, -2.2]),
    (69, [0.0, -0.5, 1.5, 0.3, -3.3, -3.1, -1.3, -3.7]),
    (70, [0.0, -2.0, 1.1, -0.3, -4.6, -4.0, -2.9, -4.6]),
    (71, [0.0, -2.1, 1.0, 7.7, -0.6, -5.0, -8.7, -0.7]),
    (72, [0.0, -3.1, 0.6, 7.3, -1.2, -4.7, -8.1, -2.4]),
    (73, [0.0, -3.8, 0.0, 6.7, -1.9, -5.5, -7.8, -4.3]),
    (74, [0.0, -4.7, -0.8, 5.2, -2.9, -6.1, -8.0, -7.6]),
    (75, [0.0, -5.2, -1.9, 3.4, -3.9, -6.8, -8.3, -10.5]),
    (76, [0.0, -5.5, -3.0, 1.8, -4.7, -7.6, -9.5, -12.4]),
    (77, [0.0, -5.4, -4.1, 0.8, -5.3, -8.1, -11.0, -13.6]),
    (78, [0.0, -5.1, -4.6, 0.2, -5.4, -8.2, -12.3, -14.0]),
    (79, [0.0, -4.5, -5.0, 0.1, -4.8, -8.4, -13.6, -13.9]),
    (80, [0.0, -3.7, -5.0, 0.4, -3.3, -8.2, -14.6, -13.6]),
    (81, [0.0, -1.5, -2.1, 2.2, 0.3, -6.5, -11.8, 6.9]),
    (82, [0.0, -0.6, -0.8, 2.5, 0.9, -6.2, -9.9, 6.4]),
    (83, [0.0, 0.7, 0.7, 3.1, 0.9, -5.9, -9.6, 6.2]),
    (84, [0.0, 2.0, 1.6, 3.7, 0.2, -6.4, 6.0, 6.0]),
];

/// The Composed arrangement's MELLOPHONE (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added, and a slower start (60 ms to full level), nearer the real one's.
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.06,
    stop: 0.0,
    keep_shape: false,
    tail: &[],
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

    fn rework(&self) -> Option<&'static Rework> {
        Some(&REWORK)
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
