//! The HORN's voice: `pp_bridge.voice('HORN', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/horn60_lowpoly.py`'s `synth` on the scene its
//! roster selects for the HORN, the module's first: the modern double horn (key `modern`). That is two notes of
//! `brass` (the shared block of `trumpet_lowpoly`, `bf_sound::voice::pp::brass`) with the tone `TONE['modern']`
//! (`bright=.55, vib=.002, scoop=.02, tilt=1.0`, `rough` at its default): the note itself and one a touch sharp
//! (x 1.002), mixed `.65` and `.35` for a section's width. The note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! The voice draws no noise: the shared stream is left where it was.

use bf_sound::voice::pp;
use bf_sound::voice::pp::brass::{Brass, brass};
use bf_sound::voice::real::{Correction, Fill, Rework};
use bf_sound::voice::{PpNoise, Voice};

/// The HORN's register in `pp_bridge`'s roster (`('HORN', 'horn60_lowpoly', None, (48, 70), 0, 1.0)`). As a
/// fighter it plays from 53 to 77; `kit.music.inst_gain` measures the voice on the middle of this one.
pub const REGISTER: (i32, i32) = (48, 70);

/// `TONE['modern']`: `dict(bright=.55, vib=.002, scoop=.02, tilt=1.0)`.
const TONE: Brass = Brass {
    bright: 0.55,
    vib: 0.002,
    scoop: 0.02,
    rough: 0.0,
    tilt: 1.0,
};

/// The HORN's added overtones: the harmonics its own note barely has, laid in at the level it is aimed at (dB
/// under the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (48, [-99.0, 8.5, 12.3, 2.5, 10.5, 2.9, 2.2, -4.5]),
    (49, [-99.0, 7.9, 12.7, 3.3, 9.8, 2.5, 1.8, -5.6]),
    (50, [-99.0, 7.5, 13.1, 3.7, 8.9, 1.9, 1.1, -6.2]),
    (51, [-99.0, 7.3, 13.3, 3.3, 8.1, 1.4, 0.1, -6.4]),
    (52, [-99.0, 7.4, 13.4, 2.4, 7.1, 1.0, -0.9, -6.4]),
    (53, [-99.0, -99.0, 7.7, -7.3, -9.2, -99.0, -99.0, -99.0]),
    (54, [-99.0, -99.0, 8.1, -7.0, -9.2, -99.0, -99.0, -99.0]),
    (55, [-99.0, -99.0, 8.3, -7.0, -9.5, -99.0, -99.0, -99.0]),
    (56, [-99.0, -99.0, 8.3, -7.3, -10.2, -99.0, -99.0, -99.0]),
    (57, [-99.0, 5.9, 8.1, -8.1, -99.0, -99.0, -99.0, -99.0]),
    (58, [-99.0, 7.0, 7.8, -9.0, -99.0, -99.0, -99.0, -99.0]),
    (59, [-99.0, 8.5, -3.1, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (60, [-99.0, 8.8, -4.2, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (61, [-99.0, 8.8, -5.3, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (62, [-99.0, 8.7, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (63, [-99.0, 8.4, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (64, [-99.0, 8.3, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (65, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (66, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (67, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (68, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (69, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (70, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
];

/// The HORN's overtone correction toward a real french horn: gains in dB for its first eight harmonics, note by note
/// (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "HORN"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (48, [0.2, -0.6, 0.0, -0.3, 0.3, 0.3, 0.4, 0.1]),
    (49, [0.2, -0.7, 0.0, -0.2, 0.3, 0.2, 0.4, 0.0]),
    (50, [0.2, -0.7, 0.0, -0.2, 0.3, 0.2, 0.4, -0.0]),
    (51, [0.2, -0.8, 0.0, -0.2, 0.3, 0.1, 0.5, -0.1]),
    (52, [0.2, -0.8, 0.0, -0.2, 0.3, 0.1, 0.5, -0.1]),
    (53, [-0.1, 10.8, 0.0, -0.2, 1.2, 15.5, 10.1, 9.8]),
    (54, [0.0, 12.1, 0.0, -0.1, 1.3, 15.2, 9.9, 9.4]),
    (55, [0.1, 13.7, 0.0, 0.1, 1.6, 15.1, 9.8, 9.1]),
    (56, [0.1, 15.4, 0.0, 0.3, 1.9, 14.8, 9.8, 9.3]),
    (57, [-0.4, -0.8, 0.0, -0.3, 14.3, 13.1, 9.3, 9.2]),
    (58, [-0.5, -0.7, 0.0, -0.1, 12.9, 12.3, 9.7, 9.9]),
    (59, [0.5, 0.0, -0.3, 6.7, 0.8, 11.9, 4.6, 5.0]),
    (60, [0.3, 0.0, -0.4, 5.5, 0.4, 10.6, 4.5, 6.0]),
    (61, [0.3, 0.0, -0.5, 4.7, 0.1, 9.4, 4.9, 6.2]),
    (62, [0.6, 0.0, 14.8, 3.6, 0.6, 9.1, 6.0, 5.5]),
    (63, [0.7, 0.0, 13.5, 2.6, 1.4, 9.5, 6.7, 3.6]),
    (64, [0.7, 0.0, 12.4, 1.4, 2.4, 10.6, 6.6, 2.5]),
    (65, [0.0, 12.0, 7.7, 2.6, -3.3, -0.3, 5.6, -1.2]),
    (66, [0.0, 10.8, 5.5, 0.9, -3.8, 0.0, 3.5, -2.1]),
    (67, [0.0, 9.2, 3.4, -0.8, -3.6, -1.0, 0.8, -3.2]),
    (68, [0.0, 7.0, 1.6, -2.1, -3.7, -3.0, -1.9, -4.4]),
    (69, [0.0, 4.5, 0.1, -3.0, -4.4, -5.3, -4.5, -4.9]),
    (70, [0.0, 2.1, -1.4, -3.7, -6.0, -7.2, -5.9, -6.2]),
];

/// The Composed arrangement's HORN (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added, and a slower start (70 ms to full level), nearer the real one's.
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.07,
    stop: 0.0,
};

/// The HORN's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct HornVoice;

impl Voice for HornVoice {
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
        "HORN"
    }

    /// Down to -0.12 s: the tone is `int((dur + .12) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.12)
    }
}

/// `horn60_lowpoly.synth` for the one note `pp_bridge.voice` asks of the modern scene, at 44,100 Hz before it is
/// resampled: `x = brass(midi_hz(m), dur, v, **tone)`, `y = brass(midi_hz(m) * 1.002, dur, v, **tone)`,
/// `.65 * x + .35 * y[:len(x)]`. `midi` is a whole number.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or so far below -0.12 s that the note has a negative number of
/// samples.
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    let f = pp::midi_hz(midi);
    let x = brass(f, dur, vel, &TONE);
    let y = brass(f * 1.002, dur, vel, &TONE);
    x.iter().zip(&y).map(|(a, b)| 0.65 * a + 0.35 * b).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_voice_draws_no_noise_and_is_the_synth_at_48k() {
        let mut noise = PpNoise::start();
        let note = HornVoice.note(&mut noise, 62.0, 0.8, 0.25);
        let raw = synth(62.0, 0.8, 0.25);
        assert_eq!(note, pp::to_48k(&raw));
        // int((dur + .12) * 44100) samples
        assert_eq!(raw.len(), 16317);
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert_eq!((HornVoice.register(), HornVoice.name()), (REGISTER, "HORN"));
        assert!(HornVoice.can_hold(0.0) && HornVoice.can_hold(-0.12));
        assert!(!HornVoice.can_hold(-0.2) && !HornVoice.can_hold(f64::NAN));
        assert!(synth(62.0, 0.8, -0.12).is_empty());
    }

    #[test]
    fn a_note_is_two_horns_a_touch_apart() {
        let f = pp::midi_hz(62.0);
        let (x, y) = (brass(f, 0.3, 0.8, &TONE), brass(f * 1.002, 0.3, 0.8, &TONE));
        let own = synth(62.0, 0.8, 0.3);
        assert_eq!(own.len(), x.len());
        assert_eq!(own[1000], 0.65 * x[1000] + 0.35 * y[1000]);
        assert_ne!(own, x);
    }
}
