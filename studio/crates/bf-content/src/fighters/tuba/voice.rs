//! TUBA's voice: `pp_bridge.voice('TUBA', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/tuba60_lowpoly.py`'s `synth` on the scene its
//! roster selects for the TUBA, the module's first: the modern CC tuba (key `modern`). That is `brass` (the shared
//! block, `bf_sound::voice::pp::brass`) with the tone `TONE['modern']` (`bright=.45, vib=.002, scoop=.02,
//! tilt=1.0`, `rough` at its default), and a little extra fundamental for the tuba's round low end: a sine at the
//! note's pitch with a 30 ms attack and a 30 ms release after the held length, at 0.35 of the velocity. The note is
//! made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! The voice draws no noise: the shared stream is left where it was.

use bf_core::dmath::{self, PI};
use bf_sound::voice::pp;
use bf_sound::voice::pp::brass::{Brass, brass};
use bf_sound::voice::real::{Correction, Rework};
use bf_sound::voice::{PpNoise, Voice};

/// The TUBA's register in `pp_bridge`'s roster (`('TUBA', 'tuba60_lowpoly', None, (33, 55), 0, 1.0)`); also its
/// roster `register`. `kit.music.inst_gain` measures the voice on the middle of it.
pub const REGISTER: (i32, i32) = (33, 55);

/// `TONE['modern']`: `dict(bright=.45, vib=.002, scoop=.02, tilt=1.0)`.
const TONE: Brass = Brass {
    bright: 0.45,
    vib: 0.002,
    scoop: 0.02,
    rough: 0.0,
    tilt: 1.0,
};

/// The TUBA's overtone correction toward a real tuba: gains in dB for its first eight harmonics, note by
/// note (FluidR3, `bf-harness/tests/reference/fluidr3.json`; the reference is the tuba). Written by
/// `cargo run --release -p bf-harness --example references -- correct "TUBA"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (33, [-13.0, 11.1, 8.6, 15.3, 13.8, 19.9, 12.6, 12.6]),
    (34, [-13.0, 11.1, 8.6, 15.3, 13.8, 20.0, 12.6, 12.6]),
    (35, [-14.7, 11.1, 13.3, 10.6, 17.5, 15.7, 12.6, 8.2]),
    (36, [-14.8, 11.1, 13.3, 10.6, 17.5, 15.7, 12.6, 8.2]),
    (37, [-14.7, 11.1, 13.3, 10.6, 17.5, 15.7, 12.7, 8.2]),
    (38, [-14.8, 11.1, 13.3, 10.6, 17.5, 15.7, 12.7, 8.2]),
    (39, [-14.7, 11.1, 13.3, 10.6, 17.5, 15.7, 12.7, 8.2]),
    (40, [0.0, 6.1, 9.8, 7.6, 6.9, 5.6, 2.6, 0.1]),
    (41, [0.0, 6.1, 9.8, 7.6, 6.9, 5.6, 2.6, 0.0]),
    (42, [0.0, 6.1, 9.8, 7.6, 6.9, 5.6, 2.6, 0.0]),
    (43, [0.0, 6.1, 9.8, 7.6, 6.9, 5.6, 2.6, 0.0]),
    (44, [0.0, 6.1, 9.8, 7.6, 6.9, 5.6, 2.6, 0.0]),
    (45, [0.0, -6.8, 2.9, -2.8, -7.7, -10.9, -28.8, -30.0]),
    (46, [0.0, -6.8, 2.9, -2.8, -7.7, -10.9, -28.8, -30.0]),
    (47, [0.0, -6.8, 2.9, -2.7, -7.7, -10.9, -28.8, -30.0]),
    (48, [0.0, -6.9, 2.9, -2.8, -7.7, -10.9, -28.8, -30.0]),
    (49, [0.0, -6.9, 2.8, -2.8, -7.7, -10.9, -28.8, -30.0]),
    (50, [0.0, 9.0, 0.3, -3.4, -17.5, -28.1, -17.5, -28.8]),
    (51, [0.0, 8.9, 0.3, -3.4, -17.5, -28.1, -17.5, -28.8]),
    (52, [0.0, 8.8, 0.3, -3.5, -17.6, -28.2, -17.5, -28.8]),
    (53, [0.0, 8.8, 0.3, -3.5, -17.6, -28.1, -17.6, -28.9]),
    (54, [0.0, 8.7, 0.2, -3.5, -17.6, -28.2, -17.6, -28.9]),
    (55, [0.0, -9.8, -18.0, -27.7, -24.6, -30.0, -30.0, -30.0]),
];

/// The Composed arrangement's TUBA (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, and a slower start (50 ms to full level), nearer the real one's.
pub const REWORK: Rework = Rework {
    correction: CORRECTION,
    speak: 0.05,
    stop: 0.0,
};

/// The TUBA's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct TubaVoice;

impl Voice for TubaVoice {
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
        "TUBA"
    }

    /// Down to -0.12 s: the tone is `int((dur + .12) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.12)
    }
}

/// `tuba60_lowpoly.synth` for the one note `pp_bridge.voice` asks of the modern scene, at 44,100 Hz before it is
/// resampled:
///
/// ```python
/// x = T.brass(midi_hz(m), dur, v, **TONE[key])
/// t = _t(len(x)); body = np.sin(2*np.pi*midi_hz(m)*t)*np.minimum(1, t/.03)*np.where(t < dur, 1, np.exp(-(t - dur)/.03))
/// return x + .35*body*v
/// ```
///
/// # Panics
/// Where Python raises: `dur` is not a number, or so far below -0.12 s that the note has a negative number of
/// samples.
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    let f = pp::midi_hz(midi);
    let x = brass(f, dur, vel, &TONE);
    let w = 2.0 * PI * f;
    let t = pp::tt(x.len());
    x.iter()
        .zip(&t)
        .map(|(&x, &t)| {
            let rise = t / 0.03;
            let rise = if rise < 1.0 { rise } else { 1.0 };
            let hold = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.03) };
            let body = dmath::sin(w * t) * rise * hold;
            x + 0.35 * body * vel
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_voice_draws_no_noise_and_is_the_synth_at_48k() {
        let mut noise = PpNoise::start();
        let note = TubaVoice.note(&mut noise, 41.0, 0.9, 0.25);
        let raw = synth(41.0, 0.9, 0.25);
        assert_eq!(note, pp::to_48k(&raw));
        // int((dur + .12) * 44100) samples
        assert_eq!(raw.len(), 16317);
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert_eq!((TubaVoice.register(), TubaVoice.name()), (REGISTER, "TUBA"));
        assert!(TubaVoice.can_hold(0.0) && TubaVoice.can_hold(-0.12));
        assert!(!TubaVoice.can_hold(-0.2) && !TubaVoice.can_hold(f64::NAN));
        assert!(synth(41.0, 0.8, -0.12).is_empty());
    }

    #[test]
    fn the_body_is_an_extra_fundamental_over_the_brass() {
        let f = pp::midi_hz(41.0);
        let plain = brass(f, 0.5, 0.9, &TONE);
        let own = synth(41.0, 0.9, 0.5);
        assert_eq!(plain.len(), own.len());
        // the first sample is silent; 0.2 s in, the body is a sine at the pitch, 0.35 of the velocity
        assert_eq!(own[0], 0.0);
        let i = 8820;
        let t = i as f64 / pp::SRF;
        assert_eq!(own[i], plain[i] + 0.35 * dmath::sin(2.0 * PI * f * t) * 0.9);
    }
}
