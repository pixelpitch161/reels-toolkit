//! The TRUMPET's voice: `pp_bridge.voice('TRUMPET', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/trumpet_lowpoly.py`'s `synth` on the scene its
//! roster selects for the TRUMPET, the module's first: the modern trumpet (key `modern`). That is `brass` (the
//! shared block, `bf_sound::voice::pp::brass`) with the tone `TONE['modern']` (`bright=1.1, vib=.006, scoop=.03,
//! rough=0`, `tilt` at its default): up to 29 harmonics summed with a dark and a bright set of weights, blended by
//! the note's own level; a scoop up into the pitch, a vibrato from 0.18 s, a 35 ms attack and a 30 ms release
//! after the held length. The note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! The voice draws no noise: the shared stream is left where it was. The `0.7` level of a note inside a chord is
//! not taken: the lone note `pp_bridge` plays has a chord size of 1.

use bf_sound::voice::pp;
use bf_sound::voice::pp::brass::{Brass, brass};
use bf_sound::voice::real::{Correction, Fill, Rework, Tail};
use bf_sound::voice::{PpNoise, Voice};

/// The TRUMPET's register in `pp_bridge`'s roster (`('TRUMPET', 'trumpet_lowpoly', None, (60, 82), 0, 1.0)`). As a
/// fighter it plays from 60 to 84; `kit.music.inst_gain` measures the voice on the middle of this one.
pub const REGISTER: (i32, i32) = (60, 82);

/// `TONE['modern']`: `dict(bright=1.1, vib=.006, scoop=.03, rough=0)`.
const TONE: Brass = Brass {
    bright: 1.1,
    vib: 0.006,
    scoop: 0.03,
    rough: 0.0,
    tilt: 0.75,
};

/// The TRUMPET's added overtones: the harmonics its own note barely has, laid in at the level it is aimed at (dB
/// under the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (60, [-99.0, -99.0, -99.0, 0.8, -0.1, 4.0, 0.9, -1.7]),
    (61, [-99.0, -99.0, -99.0, 0.7, -0.3, 3.8, 0.5, -2.0]),
    (62, [-99.0, -99.0, -99.0, 0.6, -0.6, 3.5, 0.0, -2.5]),
    (63, [-99.0, -99.0, -99.0, 0.4, -1.0, 3.1, -0.5, -3.2]),
    (64, [-99.0, -99.0, -99.0, 0.3, -1.1, 3.0, -0.6, -3.2]),
    (65, [-99.0, -99.0, -99.0, 0.4, -1.0, 3.2, -0.3, -2.7]),
    (66, [-99.0, -99.0, -99.0, 0.4, -1.1, 3.3, -0.2, -2.6]),
    (67, [-99.0, -99.0, -99.0, 2.7, 1.2, -3.4, -5.7, -8.4]),
    (68, [-99.0, -99.0, -99.0, 2.7, 1.1, -3.5, -5.8, -8.6]),
    (69, [-99.0, -99.0, -99.0, 2.6, 1.1, -3.6, -6.0, -8.9]),
    (70, [-99.0, -99.0, -99.0, 2.6, 1.0, -3.7, -6.1, -9.1]),
    (71, [-99.0, -99.0, -99.0, 2.6, 1.0, -3.7, -6.1, -9.1]),
    (72, [-99.0, -99.0, 4.8, -1.4, -1.9, -2.4, -8.0, -99.0]),
    (73, [-99.0, -99.0, 4.1, -2.6, -2.8, -3.0, -8.8, -99.0]),
    (74, [-99.0, -99.0, 3.4, -99.0, -3.7, -3.7, -9.7, -99.0]),
    (75, [-99.0, -99.0, 3.4, -99.0, -4.4, -4.4, -10.5, -99.0]),
    (76, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (77, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (78, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (79, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (80, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (81, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -29.8]),
    (82, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -30.7]),
];

/// The TRUMPET's overtone correction toward a real trumpet: gains in dB for its first eight harmonics, note by note
/// (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "TRUMPET"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (60, [-2.7, 2.0, 10.9, -3.5, -1.2, 0.0, -0.1, 1.4]),
    (61, [-2.9, 1.9, 10.8, -3.6, -1.2, 0.1, 0.1, 1.8]),
    (62, [-2.8, 2.0, 10.8, -3.4, -0.9, 0.5, 0.6, 2.0]),
    (63, [-2.8, 2.0, 10.9, -3.3, -0.7, 0.9, 1.1, 1.8]),
    (64, [-2.9, 1.9, 10.8, -3.3, -0.6, 1.1, 1.4, 1.4]),
    (65, [-3.3, 1.6, 10.5, -3.5, -0.7, 1.3, 0.8, 0.9]),
    (66, [-3.5, 1.3, 10.4, -3.5, -0.6, 1.5, 0.3, 0.6]),
    (67, [-1.3, 8.6, 12.9, -0.8, 2.1, 4.4, 1.9, 2.7]),
    (68, [-1.6, 8.3, 12.7, -0.8, 2.3, 4.0, 1.5, 2.5]),
    (69, [-2.0, 8.0, 12.4, -0.9, 2.4, 3.3, 1.0, 2.2]),
    (70, [-2.4, 7.6, 12.2, -1.0, 2.3, 2.8, 0.6, 2.0]),
    (71, [-2.8, 7.3, 11.9, -1.0, 1.6, 2.3, 0.3, 2.0]),
    (72, [-5.3, 9.8, -3.7, -2.2, -0.0, 0.6, -1.6, 9.9]),
    (73, [-6.3, 8.6, -4.6, -3.0, -1.3, -0.3, -2.4, 8.4]),
    (74, [-6.6, 8.0, -5.1, 8.8, -2.5, -0.8, -2.7, 6.0]),
    (75, [-6.8, 7.7, -5.1, 8.2, -2.9, -0.8, -2.5, 4.4]),
    (76, [-5.5, 6.4, 5.5, 7.9, 6.0, 1.4, 3.5, -4.5]),
    (77, [-5.4, 6.5, 5.8, 7.2, 4.4, 0.6, 2.0, -5.7]),
    (78, [-5.4, 6.6, 6.4, 7.1, 5.3, 1.5, 1.1, -6.0]),
    (79, [-5.5, 6.8, 6.3, 7.0, 6.0, 1.1, 1.0, -5.7]),
    (80, [-5.2, 7.0, 6.0, 7.2, 6.6, 0.8, 1.0, -5.2]),
    (81, [-5.0, 7.1, 6.2, 7.7, 7.3, 0.8, 0.8, 1.1]),
    (82, [-3.7, 7.4, 6.9, 8.6, 7.6, 1.5, 1.4, 2.0]),
];

/// Its real instrument above the eighth harmonic (sound upgrade, stage 3, S1): the 9th to 12th harmonics beside the
/// 5th to 8th, which [`REWORK`] carries its notes on with (`references -- tail`).
#[rustfmt::skip]
pub const TAIL: &Tail = &[
    (60, [-11.7, -13.0, -15.7, -16.1]),
    (61, [-11.9, -13.1, -15.9, -16.3]),
    (62, [-12.2, -13.5, -16.3, -16.6]),
    (63, [-12.7, -14.3, -17.0, -17.1]),
    (64, [-12.8, -14.1, -17.3, -17.8]),
    (65, [-12.6, -13.7, -16.9, -17.3]),
    (66, [-12.7, -13.7, -17.0, -17.4]),
    (67, [-11.0, -13.5, -15.6, -18.2]),
    (68, [-11.1, -13.7, -15.9, -18.6]),
    (69, [-11.4, -14.1, -16.4, -19.4]),
    (70, [-11.5, -14.4, -16.8, -19.9]),
    (71, [-11.6, -14.4, -16.9, -20.0]),
    (72, [-20.5, -25.1, -23.5, -24.1]),
    (73, [-21.0, -25.4, -23.8, -24.6]),
    (74, [-21.7, -25.8, -24.2, -25.3]),
    (75, [-22.2, -26.4, -24.9, -26.1]),
    (76, [-15.5, -23.6, -20.2, -30.4]),
    (77, [-15.7, -23.9, -20.7, -31.0]),
    (78, [-17.2, -25.3, -22.0, -33.3]),
    (79, [-17.6, -26.2, -22.9, -34.4]),
    (80, [-18.0, -26.9, -23.7, -35.3]),
    (81, [-18.4, -27.4, -24.3, -36.1]),
    (82, [-19.1, -28.1, -25.3, -36.8]),
];

/// The Composed arrangement's TRUMPET (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added; its own start and stop, already near the real one's.
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.0,
    stop: 0.0,
    keep_shape: false,
    tail: TAIL,
};

/// The TRUMPET's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct TrumpetVoice;

impl Voice for TrumpetVoice {
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
        "TRUMPET"
    }

    /// Down to -0.12 s: the tone is `int((dur + .12) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.12)
    }
}

/// `trumpet_lowpoly.synth` for the one note `pp_bridge.voice` asks of the modern scene: `brass(midi_hz(m), dur, v,
/// **TONE['modern'])` at 44,100 Hz, before it is resampled (times 1.0: the chord size is 1). `midi` is a whole
/// number.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or so far below -0.12 s that the note has a negative number of
/// samples.
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    brass(pp::midi_hz(midi), dur, vel, &TONE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_voice_draws_no_noise_and_is_the_synth_at_48k() {
        let mut noise = PpNoise::start();
        let note = TrumpetVoice.note(&mut noise, 72.0, 0.9, 0.25);
        let raw = synth(72.0, 0.9, 0.25);
        assert_eq!(note, pp::to_48k(&raw));
        // int((dur + .12) * 44100) samples
        assert_eq!(raw.len(), 16317);
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert_eq!((TrumpetVoice.register(), TrumpetVoice.name()), (REGISTER, "TRUMPET"));
        // what it can hold is what Python does not raise for
        assert!(TrumpetVoice.can_hold(0.0) && TrumpetVoice.can_hold(-0.12));
        assert!(!TrumpetVoice.can_hold(-0.2) && !TrumpetVoice.can_hold(f64::NAN));
        assert!(synth(72.0, 0.8, -0.12).is_empty());
    }

    #[test]
    fn the_tone_is_the_modern_trumpets() {
        // brighter and with more vibrato than the block's default tone
        let default = brass(pp::midi_hz(72.0), 0.5, 0.9, &Brass::default());
        let own = synth(72.0, 0.9, 0.5);
        assert_eq!(default.len(), own.len());
        assert_ne!(default, own);
    }
}
