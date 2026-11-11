//! The fighters' instrument voices: the trait every voice implements, the noise stream the Pixel Pitch voices
//! share, and what those voices have in common.
//!
//! A fighter plays in the voice of its instrument from the "Pixel Pitch" instrument-history series
//! (`series/instrument-history/<instrument>_lowpoly.py`), reached in the kit through
//! `projects/which_instrument_wins/pp_bridge.py`: `pp_bridge.voice(name, midi, vel, dur)` is one note, made by the
//! instrument's own `synth` at 44,100 Hz and resampled to 48,000 Hz. The instruments' synthesisers are ported in
//! `bf-content` (one `voice.rs` per fighter); this module holds what they share:
//!
//! - [`Voice`]: one note at 48 kHz, as `pp_bridge.voice`.
//! - [`PpNoise`]: the series' ONE noise generator. Every voice with breath or attack noise draws from it, so a
//!   note's samples depend on every noisy note made before it. A soundtrack build starts it once
//!   ([`PpNoise::start`]) and synthesises its notes one at a time, in the order the score first asks for them.
//! - [`pp`]: the series' sample rate and time base, its pitch formula, its own low-pass, and the way to 48 kHz.

use bf_core::rng::Generator;

/// The Pixel Pitch voices' shared noise stream: `drums60_lowpoly._rng = np.random.default_rng(77)`, advanced by
/// every `nz(n)` call of every voice.
///
/// The generator is public so a test can resume it from a state recorded with a reference note
/// (`Generator::from_state`); nothing else should draw from it, or every later note changes.
#[derive(Clone, Debug)]
pub struct PpNoise(pub Generator);

impl PpNoise {
    /// The seed of the series' generator.
    pub const SEED: u64 = 77;
    /// The generator's `state` where a soundtrack build starts (`studio/reference/sound/pp_noise_start.json`):
    /// after every import of the kit and before the first note. It is the state of a fresh `default_rng(77)`:
    /// importing the kit and the voices draws nothing.
    pub const START_STATE: u128 = 289533129354323354318955456298181112347;
    /// The generator's `inc`, which never changes.
    pub const START_INC: u128 = 336983293413220778415499640756163231851;

    /// The stream where a soundtrack build starts. Call it once per build.
    pub fn start() -> PpNoise {
        PpNoise::from_state(Self::START_STATE, Self::START_INC)
    }

    /// The stream resumed from a recorded `state` and `inc` (a reference note stores the state before its call).
    pub fn from_state(state: u128, inc: u128) -> PpNoise {
        PpNoise(Generator::from_state(state, inc))
    }

    /// `drums60_lowpoly.nz(n)`: `_rng.normal(0, 1, n)`, the next `n` normals of the stream.
    pub fn nz(&mut self, n: usize) -> Vec<f64> {
        let mut x = vec![0.0; n];
        self.0.fill_normal(0.0, 1.0, &mut x);
        x
    }
}

/// An instrument voice: `pp_bridge.voice(name, midi, vel, dur)` for one instrument.
pub trait Voice: Send + Sync {
    /// One note at 48,000 Hz: MIDI note `midi` (a whole number; the kit passes `int(midi)`), velocity `vel`, held
    /// for `dur` seconds. The note rings on past `dur` as the synth makes it (a release, echoes), so it is longer
    /// than `dur`. Noise, if the voice has any, is drawn from `noise`, which is left where the note left it.
    ///
    /// A voice gives samples for every finite `midi` and `vel` and every `dur` it [can hold](Voice::can_hold),
    /// however far outside the instrument (Python does too; far outside they may be NaN). A `dur` a little below
    /// zero is such a `dur`: the note is then its release alone, as in Python. Where Python raises (a `dur` so far
    /// below zero that the note has a negative number of samples, a `dur` that is not a number) a voice panics;
    /// [`crate::music::MusicCtx::note`] asks `can_hold` first and returns an error instead.
    fn note(&self, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64>;

    /// The voice's name in `pp_bridge`'s roster (`entrant(name)[0]`), which is what a fighter's roster entry
    /// names as its `pp`: `FLUTE`, `TROMBONE`. [`crate::music::MusicCtx::new`] checks it against each fighter's.
    fn name(&self) -> &str;

    /// Whether the voice has a note for a held length of `dur` seconds: false where Python raises for it (the
    /// note would have a negative number of samples; a voice rings on for its own release past `dur`, so each
    /// has its own limit a little below zero). The default says yes to every `dur`: right for a voice whose
    /// `note` never panics.
    fn can_hold(&self, dur: f64) -> bool {
        let _ = dur;
        true
    }

    /// The instrument's register in `pp_bridge`'s roster, lowest and highest MIDI note (`entrant(name)[3]`).
    /// `kit.music.inst_gain` measures the instrument's level on the note in its middle, `(lo + hi) // 2`. This
    /// is the Pixel Pitch entrant's register, which need not be the fighter's own (`FighterCfg::register`):
    /// TROMBONE is 45 to 67 here and 40 to 65 as a fighter.
    fn register(&self) -> (i32, i32);
}

/// What the Pixel Pitch voices share: `pp_framework`'s sample rate and pitch, `drums60_lowpoly`'s low-pass, and
/// `pp_bridge`'s resampling.
pub mod pp {
    use bf_core::dmath;

    use crate::dsp::{self, FilterKind};

    /// The series' sample rate (`pp_framework.SR`, `pp_bridge.PP_SR`), in Hz.
    pub const SR: usize = 44_100;

    /// The sample rate as a float.
    const SRF: f64 = SR as f64;

    /// `int(seconds * SR)`: a length in the series' samples, cut toward zero.
    ///
    /// # Panics
    /// Where Python raises: a length that is not a number, or below zero (numpy refuses a negative length).
    pub fn len(seconds: f64) -> usize {
        let n = seconds * SRF;
        assert!(n.is_finite(), "cannot convert float {n} to integer");
        assert!(n > -1.0, "negative dimensions are not allowed ({seconds} s)");
        n as usize
    }

    /// Whether [`len`] takes `seconds`: a number, and not a negative count of samples (a fraction of a sample
    /// below zero is cut to none, as Python's `int` cuts it).
    pub fn has_len(seconds: f64) -> bool {
        let n = seconds * SRF;
        n.is_finite() && n > -1.0
    }

    /// `_t(n)`: the time of each of `n` samples, in seconds (`np.arange(n) / SR`).
    pub fn tt(n: usize) -> Vec<f64> {
        (0..n).map(|i| i as f64 / SRF).collect()
    }

    /// `midi_hz(m)`: the frequency of MIDI note `m`, `440 * 2 ** ((m - 69) / 12)`.
    pub fn midi_hz(m: f64) -> f64 {
        440.0 * dmath::pow(2.0, (m - 69.0) / 12.0)
    }

    /// `drums60_lowpoly.lp(x, f)`: `x` through `butter(2, f / (SR / 2), "low")` with `lfilter` (the series
    /// designs its filters as `b`, `a`, not as second-order sections). An empty `x` gives an empty result.
    ///
    /// # Panics
    /// Where scipy raises: `f` is not between 0 and half the sample rate.
    pub fn lp(x: &[f64], f: f64) -> Vec<f64> {
        // Python passes no `fs`: the cutoff is a fraction of the Nyquist frequency.
        let (b, a) = dsp::butter_ba(FilterKind::Lp, &[f / (SRF / 2.0)], 2, 2.0);
        dsp::lfilter(&b, &a, x)
    }

    /// `pp_bridge._resample(x)`: from 44,100 Hz to 48,000 Hz, `scipy.signal.resample_poly(x, 160, 147)`.
    pub fn to_48k(x: &[f64]) -> Vec<f64> {
        dsp::resample_poly(x, 160, 147)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn lengths_and_times() {
            assert_eq!(len(1.0), 44100);
            // int((0.12 + .18) * SR) and int(.75 * (60 / 112) * SR): cut, not rounded.
            assert_eq!(len(0.12 + 0.18), 13230);
            assert_eq!(len(0.75 * (60.0 / 112.0)), 17718);
            assert_eq!(len(0.0), 0);
            // int() cuts toward zero: a length a fraction of a sample below zero is none.
            assert_eq!(len(-0.5 / 44100.0), 0);
            // has_len says where len would refuse.
            assert!(has_len(0.0) && has_len(1.0) && has_len(-0.5 / 44100.0));
            assert!(!has_len(-0.2) && !has_len(-1.5 / 44100.0) && !has_len(f64::NAN) && !has_len(f64::INFINITY));
            assert_eq!(tt(3), [0.0, 1.0 / 44100.0, 2.0 / 44100.0]);
            assert!(tt(0).is_empty());
        }

        #[test]
        #[should_panic(expected = "negative dimensions are not allowed (-0.2 s)")]
        fn a_negative_length_is_refused() {
            len(-0.2);
        }

        #[test]
        #[should_panic(expected = "cannot convert float NaN to integer")]
        fn a_length_must_be_a_number() {
            len(f64::NAN);
        }

        #[test]
        fn pitch() {
            assert_eq!(midi_hz(69.0), 440.0);
            assert_eq!(midi_hz(81.0), 880.0);
            assert_eq!(midi_hz(57.0), 220.0);
            assert!((midi_hz(60.0) - 261.6255653005986).abs() < 1e-12);
        }

        #[test]
        fn the_low_pass_and_the_resampler() {
            // A constant comes through the low-pass at its own level, and nothing through is nothing.
            let y = lp(&vec![1.0; 4000], 2500.0);
            assert_eq!(y.len(), 4000);
            assert!((y[3999] - 1.0).abs() < 1e-9, "{}", y[3999]);
            assert!(y[0] > 0.0 && y[0] < 0.05);
            assert!(lp(&[], 2500.0).is_empty());
            // 147 samples at 44,100 Hz are 160 at 48,000 Hz; any other length is rounded up.
            assert_eq!(to_48k(&[0.0; 147]).len(), 160);
            assert_eq!(to_48k(&[0.0; 148]).len(), 162);
            assert!(to_48k(&[]).is_empty());
            let flat = to_48k(&vec![1.0; 4410]);
            assert!((flat[2400] - 1.0).abs() < 1e-3, "{}", flat[2400]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stream_is_a_fresh_generator_of_seed_77() {
        let (mut a, mut b) = (PpNoise::start(), Generator::new(PpNoise::SEED));
        let mut want = vec![0.0; 64];
        b.fill_normal(0.0, 1.0, &mut want);
        assert_eq!(a.nz(64), want);
        // The stream goes on where the last call left it.
        b.fill_normal(0.0, 1.0, &mut want);
        assert_eq!(a.nz(64), want);
        assert!(a.nz(0).is_empty());
        // A copy draws on its own.
        let mut copy = a.clone();
        assert_eq!(copy.nz(8), a.nz(8));
    }
}
