//! ELECTRIC's voice: `pp_bridge.voice(<its roster pp>, midi, vel, dur)`.
//!
//! The roster's `pp` for ELECTRIC is an internal Pixel Pitch voice id that carries a retired fighter's name
//! (`bf_core::fightfile::RETIRED[0]`, ruling M3-R9): it is read from that constant, never written here, and never
//! reaches the screen. `pp_bridge` plays one note through `series/instrument-history/electric60_lowpoly.py`'s
//! `synth` on its first scene (key `modern`, the modern electric): one Karplus-Strong string
//! (`piano_lowpoly.ks_pluck`, the shared `bf_sound::voice::pp::pluck`) ringing `max(dur + 0.3, 0.5)` s, bright 1.0,
//! decay 0.999, seeded 0 (the note's index), driven seven times into `electric_lowpoly.amp` (a tanh at drive 7,
//! two one-pole low-passes at tone 0.5) and let go by `electric_lowpoly.damp` 0.03 s after the held length (release
//! 0.06 s), times the velocity. The note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! The voice draws no noise: its string comes from its own seeded generator, so the shared stream is left where it
//! was.

use bf_core::dmath;
use bf_core::py::py_max;
use bf_sound::dsp;
use bf_sound::voice::pp;
use bf_sound::voice::pp::pluck::ks_pluck;
use bf_sound::voice::real::{Correction, Fill, Rework};
use bf_sound::voice::{PpNoise, Voice};

/// The voice's register in `pp_bridge`'s roster (`(52, 76)`); also ELECTRIC's roster `register`.
pub const REGISTER: (i32, i32) = (52, 76);

/// The ELECTRIC's added overtones: the harmonics its own note barely has, laid in at a real overdriven guitar's level (dB
/// under the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (52, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (53, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (54, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (55, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (56, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (57, [-99.0, -99.0, -99.0, -99.0, 8.0, -99.0, -99.0, -99.0]),
    (58, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (59, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (60, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (61, [-99.0, -99.0, -99.0, 2.6, -99.0, -99.0, -99.0, -99.0]),
    (62, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (63, [-99.0, -99.0, -99.0, -99.0, 20.5, 24.1, 21.4, 18.6]),
    (64, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (65, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (66, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (67, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 0.2]),
    (68, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 0.4]),
    (69, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 0.4]),
    (70, [-99.0, -99.0, -99.0, -99.0, 4.9, 8.8, 3.3, -3.4]),
    (71, [-99.0, -99.0, -99.0, -99.0, 5.1, 9.3, 2.4, -3.5]),
    (72, [-99.0, -99.0, 2.8, -1.0, 5.3, 9.8, 1.1, -3.1]),
    (73, [-99.0, -99.0, 6.2, 8.6, 11.8, -3.2, -3.4, -99.0]),
    (74, [-99.0, -99.0, 5.7, 9.3, 12.3, -2.6, -3.8, -7.7]),
    (75, [-99.0, 8.3, 5.1, 9.9, 12.7, -1.7, -4.6, -7.5]),
    (76, [-99.0, 7.7, 4.6, 10.6, 12.9, -0.5, -5.4, -7.3]),
];

/// The ELECTRIC's overtone correction toward a real overdriven guitar: gains in dB for its first eight harmonics, note by
/// note (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "ELECTRIC"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (52, [10.8, 13.6, -7.6, 3.5, -6.2, -3.9, 4.3, -12.0]),
    (53, [24.0, -0.5, -8.7, 7.1, -7.5, 1.0, -2.0, -10.7]),
    (54, [-15.3, -16.8, -17.1, -11.5, -20.1, -4.2, -20.6, -15.7]),
    (55, [-4.9, -16.0, -15.6, -11.1, -18.3, -8.9, -21.1, -9.3]),
    (56, [-3.5, -14.9, -10.8, -10.4, -9.5, -10.7, -16.0, -11.3]),
    (57, [-19.7, -19.1, -8.3, -11.1, -18.4, -16.3, -6.7, -7.0]),
    (58, [20.8, 8.4, 6.2, 5.1, 15.2, 7.3, 6.8, 24.0]),
    (59, [-12.0, -26.3, -27.3, -27.1, -25.9, -20.1, -25.4, -13.9]),
    (60, [3.5, -11.2, -15.0, -6.3, -13.1, -2.4, 3.2, 6.1]),
    (61, [6.4, -13.3, -10.6, 7.7, -4.7, 6.1, 3.7, 6.2]),
    (62, [12.0, -16.0, -10.5, 0.3, 3.0, 10.7, 7.7, 6.3]),
    (63, [18.6, -15.6, -6.3, -0.7, 0.4, 0.5, 0.7, 1.0]),
    (64, [5.4, -16.8, 2.3, -0.5, 14.3, 11.9, 10.0, 4.4]),
    (65, [4.1, -18.0, 3.6, -0.7, 6.7, 13.0, 13.4, 4.1]),
    (66, [1.4, -3.9, 4.6, -5.3, -1.1, 6.5, 5.3, 12.1]),
    (67, [4.0, -3.3, 6.5, -1.5, -1.6, 7.6, 5.2, 4.0]),
    (68, [-1.1, -2.3, 0.7, 12.6, 2.5, 8.7, 9.0, -1.3]),
    (69, [-2.5, -4.2, 0.6, 5.6, 8.7, 3.7, 12.3, -3.9]),
    (70, [1.1, 0.2, 11.5, 12.2, 0.4, 0.0, 0.7, 0.4]),
    (71, [0.8, -0.2, 13.7, 12.6, -0.5, 0.0, 1.0, 1.2]),
    (72, [0.2, -0.1, 0.5, 1.6, -0.5, 0.0, 0.6, -0.0]),
    (73, [-1.0, 11.7, -0.4, 0.1, 0.0, -0.2, -0.5, 11.3]),
    (74, [-0.3, 13.4, -1.0, -0.5, 0.0, -0.7, 0.8, -0.4]),
    (75, [0.1, 0.9, -0.1, 0.4, 0.0, 0.5, -0.0, 0.2]),
    (76, [0.0, 1.1, -0.2, 0.3, 0.0, 0.3, 0.4, 0.2]),
];

/// The Composed arrangement's ELECTRIC (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added, and a quicker stop (a 20 ms fade after its length, the real one's muted
/// strings); it keeps its own pluck's snap.
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.0,
    stop: 0.02,
    keep_shape: true,
};

/// ELECTRIC's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct ElectricVoice;

impl Voice for ElectricVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn rework(&self) -> Option<&'static Rework> {
        Some(&REWORK)
    }

    /// The roster's `pp` (an internal id, ruling M3-R9).
    fn name(&self) -> &str {
        bf_core::fightfile::RETIRED[0]
    }
}

/// `electric_lowpoly.amp(x, drive, tone=.5)`: `np.tanh(x * drive) / np.tanh(drive)` (for a drive above 0), then
/// twice `lfilter([1 - tone], [1, -tone], x)`.
fn amp(x: &[f64], drive: f64, tone: f64) -> Vec<f64> {
    let mut y: Vec<f64> = if drive > 0.0 {
        let td = dmath::np_tanh(drive);
        x.iter().map(|&v| dmath::np_tanh(v * drive) / td).collect()
    } else {
        x.to_vec()
    };
    for _ in 0..2 {
        y = dsp::lfilter(&[1.0 - tone], &[1.0, -tone], &y);
    }
    y
}

/// `electric_lowpoly.damp(x, dur, rel=.06)`: `x * np.where(t < dur + .03, 1.0, np.exp(-(t - dur - .03) / rel))`.
fn damp(x: &[f64], dur: f64, rel: f64) -> Vec<f64> {
    let t = pp::tt(x.len());
    x.iter()
        .zip(t)
        .map(|(&v, t)| {
            v * if t < dur + 0.03 {
                1.0
            } else {
                dmath::exp(-(t - dur - 0.03) / rel)
            }
        })
        .collect()
}

/// `electric60_lowpoly.synth` for the one note `pp_bridge.voice` asks of the `modern` scene (its index 0, a chord
/// size of 1: neither muted nor the final chord, no strum offset), at 44,100 Hz before it is resampled:
///
/// ```python
/// x = P.ks_pluck(f, max(dur + .3, .5), 1.0, .999, idx)
/// x = E.damp(E.amp(x*7, 7.0, .5), dur, .06)
/// return x*v
/// ```
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    let f = pp::midi_hz(midi);
    let x = ks_pluck(f, py_max(dur + 0.3, 0.5), 1.0, 0.999, 0);
    let x: Vec<f64> = x.iter().map(|&v| v * 7.0).collect();
    damp(&amp(&x, 7.0, 0.5), dur, 0.06).iter().map(|&v| v * vel).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_voice_draws_no_noise_and_is_the_synth_at_48k() {
        let mut noise = PpNoise::start();
        let note = ElectricVoice.note(&mut noise, 64.0, 0.9, 0.25);
        let raw = synth(64.0, 0.9, 0.25);
        assert_eq!(note, pp::to_48k(&raw));
        // int(max(0.25 + 0.3, 0.5) * 44100) samples
        assert_eq!(raw.len(), 24255);
        assert_eq!(synth(64.0, 0.9, 0.1).len(), 22050);
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert_eq!(ElectricVoice.register(), REGISTER);
        assert_eq!(ElectricVoice.name(), bf_core::fightfile::RETIRED[0]);
    }
}
