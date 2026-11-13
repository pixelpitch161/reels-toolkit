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
use bf_sound::voice::{PpNoise, Voice};

/// The voice's register in `pp_bridge`'s roster (`(52, 76)`); also ELECTRIC's roster `register`.
pub const REGISTER: (i32, i32) = (52, 76);

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
