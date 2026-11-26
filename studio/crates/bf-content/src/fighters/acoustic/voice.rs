//! ACOUSTIC's voice: `pp_bridge.voice('ACOUSTIC', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/acoustic_lowpoly.py`'s `synth` on the scene its
//! roster selects for the ACOUSTIC (key `steel`, the modern steel-string): two Karplus-Strong strings
//! (`piano_lowpoly.ks_pluck`, the shared `bf_sound::voice::pp::pluck`), the note's pitch at 0.7 and a string tuned
//! 1.0025 sharp at 0.3, each ringing 1.8 s whatever the held length, bright 0.9, decay 0.9988, seeded 0 and 50 (the
//! note's index and 50 more), times the velocity. The note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! The voice draws no noise: its strings come from their own seeded generators, so the shared stream is left where
//! it was. It is also the stadium rock bed's unplugged strums (`M.note('ACOUSTIC', ...)`).

use bf_sound::voice::pp;
use bf_sound::voice::pp::pluck::ks_pluck;
use bf_sound::voice::real::{Correction, Fill, Rework};
use bf_sound::voice::{PpNoise, Voice};

/// The ACOUSTIC's register in `pp_bridge`'s roster (`('ACOUSTIC', 'acoustic_lowpoly', 'steel', (40, 76), 0, 1.0)`);
/// also its roster `register`. `kit.music.inst_gain` measures the voice on the middle of it.
pub const REGISTER: (i32, i32) = (40, 76);

/// The ACOUSTIC's added overtones: the harmonics its own note barely has, laid in at a real steel-string guitar's level (dB
/// under the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (40, [-99.0, 25.6, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (41, [-99.0, 25.5, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (42, [-99.0, 25.5, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (43, [-99.0, 25.6, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (44, [-99.0, 14.2, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (45, [-99.0, 15.1, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (46, [-99.0, 15.8, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (47, [-99.0, 16.6, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (48, [-99.0, 17.2, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (49, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (50, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (51, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (52, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (53, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (54, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (55, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (56, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (57, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (58, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (59, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (60, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (61, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (62, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (63, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (64, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (65, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (66, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (67, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (68, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -19.5]),
    (69, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -19.9]),
    (70, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -20.3]),
    (71, [-99.0, -99.0, -99.0, -99.0, -99.0, -9.7, -99.0, -24.9]),
    (72, [-99.0, -99.0, -99.0, -99.0, -99.0, -9.5, -99.0, -24.8]),
    (73, [-99.0, -99.0, -99.0, -99.0, -99.0, -9.2, -30.4, -24.6]),
    (74, [-99.0, -99.0, -99.0, -99.0, -99.0, -9.0, -30.3, -24.6]),
    (75, [-99.0, -99.0, -99.0, -99.0, -19.9, -8.7, -30.4, -24.6]),
    (76, [-99.0, -99.0, -99.0, -10.5, -19.6, -29.2, -39.2, -28.8]),
];

/// The ACOUSTIC's overtone correction toward a real steel-string guitar: gains in dB for its first eight harmonics, note by
/// note (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "ACOUSTIC"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (40, [0.7, 0.0, 13.0, 2.9, 2.4, -8.9, -12.6, 1.0]),
    (41, [-0.2, 0.0, 12.9, 7.6, 6.8, -12.8, -9.5, 10.0]),
    (42, [0.5, 0.0, 10.7, 3.3, 14.0, -17.2, -10.3, 2.1]),
    (43, [0.0, 0.0, 7.8, 8.2, 6.8, -16.3, -11.1, 4.6]),
    (44, [2.9, 0.0, 3.5, 3.8, -9.0, -5.4, -30.0, -19.5]),
    (45, [1.2, 0.0, 9.4, 7.0, -7.0, -2.8, -30.0, -14.9]),
    (46, [-0.7, 0.0, 12.3, 10.4, -4.7, -1.1, -30.0, -3.2]),
    (47, [-0.4, 0.0, 10.9, 6.2, -5.7, -3.1, -30.0, -6.1]),
    (48, [-0.7, 0.0, 11.0, 5.4, -4.6, -8.4, -30.0, -4.8]),
    (49, [6.5, 5.4, 5.6, -3.0, 2.3, -12.9, -13.5, -9.2]),
    (50, [6.4, 4.1, 3.2, -3.5, 2.1, -12.9, -7.5, -9.9]),
    (51, [10.7, 3.6, -0.5, -1.4, -0.3, -11.7, -12.9, -15.8]),
    (52, [8.0, 6.4, -3.2, -1.0, -6.9, -9.3, -10.3, -14.7]),
    (53, [6.3, 11.6, -3.5, 5.1, -6.7, -6.2, -13.9, -10.4]),
    (54, [2.5, 15.6, -2.7, -0.9, -11.7, -0.6, -30.0, -4.3]),
    (55, [2.0, 9.0, -2.1, -2.7, -10.8, 1.6, -30.0, 4.0]),
    (56, [1.2, 3.9, -2.2, -7.0, -8.4, -6.1, -30.0, 0.4]),
    (57, [1.2, 0.0, -1.0, -9.4, -4.8, -11.7, -24.4, -7.4]),
    (58, [4.8, 0.0, 1.2, -7.8, 2.3, -7.4, -23.3, -8.1]),
    (59, [11.7, -1.4, 5.4, -1.7, -2.4, 2.3, -4.1, -9.5]),
    (60, [3.9, -1.1, 3.3, 0.7, -5.7, 9.1, -16.5, -8.0]),
    (61, [-0.0, -0.7, 0.5, 8.1, -5.6, 8.0, -22.8, 4.4]),
    (62, [-0.1, -0.8, 1.5, 8.1, 0.2, 4.1, -20.6, -1.4]),
    (63, [5.8, -2.8, 1.2, 2.4, 4.4, -4.7, -3.0, -11.2]),
    (64, [4.5, -2.5, 4.0, 2.5, 15.7, -5.9, -6.8, -2.0]),
    (65, [4.0, -2.8, 5.5, 5.1, -1.5, -3.4, -4.2, -6.4]),
    (66, [2.4, -2.5, 5.3, 5.6, -7.7, 2.6, -1.7, -5.3]),
    (67, [0.9, -1.7, 7.5, 8.4, -7.3, 2.3, 1.8, 4.1]),
    (68, [0.4, -12.2, -15.5, 0.3, -24.6, -1.1, -13.7, 0.1]),
    (69, [0.0, -12.0, -13.7, -3.9, -19.3, 1.1, -13.7, 2.6]),
    (70, [1.6, -13.8, -12.3, -5.4, -18.5, 2.4, -6.0, 1.1]),
    (71, [2.9, -7.8, -20.5, -9.3, 0.8, 2.8, 14.5, 3.3]),
    (72, [3.2, -7.8, -18.1, -9.3, 4.1, 2.2, 15.9, 2.1]),
    (73, [2.6, -7.1, -19.9, -5.6, 7.4, 2.7, 2.5, 2.5]),
    (74, [0.0, -3.0, -21.2, -1.5, 7.9, 2.3, 2.4, 2.9]),
    (75, [0.0, -1.0, -24.6, 5.7, -0.6, 0.1, 0.3, -0.0]),
    (76, [0.0, -4.9, -3.2, 0.7, -0.4, -0.2, 0.1, -0.1]),
];

/// The Composed arrangement's ACOUSTIC (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added; it keeps its own pluck's snap and ring.
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.0,
    stop: 0.0,
    keep_shape: true,
};

/// The ACOUSTIC's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct AcousticVoice;

impl Voice for AcousticVoice {
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
        "ACOUSTIC"
    }
}

/// `acoustic_lowpoly.synth` for the one note `pp_bridge.voice` asks of the `steel` scene (its index 0, a chord size
/// of 1: no strum offset), at 44,100 Hz before it is resampled:
///
/// ```python
/// f = midi_hz(m); ring = 1.8
/// x = P.ks_pluck(f, ring, .9, .9988, idx)*.7 + P.ks_pluck(f*1.0025, ring, .9, .9988, idx + 50)*.3
/// return x*v
/// ```
///
/// The held length `dur` is not read: the strings ring their 1.8 s.
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    let _ = dur;
    let f = pp::midi_hz(midi);
    let ring = 1.8;
    let a = ks_pluck(f, ring, 0.9, 0.9988, 0);
    let b = ks_pluck(f * 1.0025, ring, 0.9, 0.9988, 50);
    a.iter().zip(&b).map(|(&a, &b)| (a * 0.7 + b * 0.3) * vel).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_voice_draws_no_noise_and_is_the_synth_at_48k() {
        let mut noise = PpNoise::start();
        let note = AcousticVoice.note(&mut noise, 52.0, 0.9, 0.25);
        let raw = synth(52.0, 0.9, 0.25);
        assert_eq!(note, pp::to_48k(&raw));
        // int(1.8 * 44100) samples, whatever the held length
        assert_eq!(raw.len(), 79380);
        assert_eq!(synth(52.0, 0.9, 3.0), raw);
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert_eq!((AcousticVoice.register(), AcousticVoice.name()), (REGISTER, "ACOUSTIC"));
    }
}
