//! BASS DRUM's instrument voice: `pp_bridge.voice('BASS DRUM', midi, vel, dur)`, and the strokes its part plays.
//!
//! `pp_bridge` plays one note through `series/instrument-history/bassdrum_lowpoly.py`'s `synth`: a tuned membrane
//! struck with a felt mallet (the circular membrane's partials, the higher ones dying fast; a pitch drop as the head
//! relaxes; the felt's soft thump; the shell ringing a fifth over). The note picks one of three strokes:
//!
//! - MIDI 24 and up, velocity under 0.9: a mallet hit, tight and short, damped after the held length ([`bd_hit`]);
//! - MIDI 24 and up, velocity 0.9 and up: the BOOM, a big open hit with a sub an octave under and a long ring;
//! - under MIDI 24: a ROLL on MIDI + 24 for the held length, single strokes on 32nds at 126 BPM swelling to a BOOM
//!   on the last stroke ([`bd_roll`]).
//!
//! The note is made at 44,100 Hz and resampled to 48,000 Hz. Each stroke draws its felt and air from a generator of
//! its own, seeded from the pitch (`default_rng(int(f * 13 + seed * 977 + 7 * boom) % 100003)`), and a roll its
//! timing from another (`default_rng(int(f * 31 + dur * 101 + seed) % 100003)`), so the shared noise stream is
//! left where it was. The part (`sound.rs`) calls [`bd_hit`] and [`bd_roll`] itself, as the Python part does.

use bf_core::dmath::{self, PI};
use bf_core::py;
use bf_core::rng::Generator;
use bf_sound::dsp;
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::{PpNoise, Voice};

/// The BASS DRUM's register in `pp_bridge`'s roster (`('BASS DRUM', 'bassdrum_lowpoly', 'ball', (28, 50), ...)`).
pub const REGISTER: (i32, i32) = (28, 50);

/// `bassdrum_lowpoly.BPM`: the tempo of the voice's roll.
pub const BPM: f64 = 126.0;

/// `MODES`: the membrane's partials as `(ratio, amplitude, decay share)`.
const MODES: [(f64, f64, f64); 6] = [
    (1.0, 1.0, 1.0),
    (1.59, 0.55, 0.45),
    (2.14, 0.38, 0.3),
    (2.30, 0.30, 0.26),
    (2.65, 0.2, 0.2),
    (2.92, 0.14, 0.16),
];

/// BASS DRUM's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct BassDrumVoice;

impl Voice for BassDrumVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "BASS DRUM"
    }

    /// A finite held length a roll has room for: a roll's length is `int((dur + 1.4) * SR)` samples and its strokes
    /// `int(round(dur / s32))`, which Python cannot take for a length that is not finite. (A hit takes any length.)
    fn can_hold(&self, dur: f64) -> bool {
        dur.is_finite() && pp::has_len(dur + 1.4)
    }
}

/// `int(x)` of a seed: Python raises for a number that is not finite.
fn whole(x: f64) -> i64 {
    assert!(
        x.is_finite() && x.abs() < 9.0e18,
        "cannot convert float {x} to an integer seed"
    );
    x as i64
}

/// `bassdrum_lowpoly.synth` for the one note `pp_bridge.voice` asks, at 44,100 Hz before it is resampled: a roll
/// on `midi + 24` under MIDI 24, else a hit, the BOOM from velocity 0.9.
///
/// # Panics
/// As [`bd_roll`] and [`bd_hit`].
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    if midi < 24.0 {
        return bd_roll(pp::midi_hz(midi + 24.0), dur, vel, BPM, 0);
    }
    bd_hit(pp::midi_hz(midi), vel, vel >= 0.9, 0, Some(dur))
}

/// `bd_hit(f, v, boom=False, seed=0, dur=None)`: one stroke on a drum tuned to `f`. `boom` is the big open hit; a
/// mallet hit with a `dur` is damped after it (a short tap).
///
/// # Panics
/// Where Python raises: a pitch whose seed `int(f * 13 + seed * 977 + ...)` is not a whole number an integer
/// holds.
pub fn bd_hit(f: f64, v: f64, boom: bool, seed: i64, dur: Option<f64>) -> Vec<f64> {
    let ring = if boom { 1.25 } else { 0.42 };
    let n = pp::len(ring * 1.6 + 0.05);
    let t = pp::tt(n);
    let (depth, fall) = if boom { (0.42, 0.028) } else { (0.22, 0.018) };
    let drop: Vec<f64> = t.iter().map(|&t| depth * dmath::exp(-t / fall)).collect();
    let mut x = vec![0.0; n];
    for (ratio, amp, dk) in MODES {
        // fr = f * ratio * (1 + drop); ph = 2 * np.pi * np.cumsum(fr) / SR
        let fr: Vec<f64> = drop.iter().map(|&d| f * ratio * (1.0 + d)).collect();
        let tau = ring * dk;
        for ((x, c), &t) in x.iter_mut().zip(dsp::cumsum(&fr)).zip(&t) {
            let ph = 2.0 * PI * c / SRF;
            *x += amp * dmath::sin(ph + ratio) * dmath::exp(-t / tau);
        }
    }
    if boom {
        // the sub under it, and air
        let fr: Vec<f64> = drop.iter().map(|&d| f * 0.5 * (1.0 + d * 1.3)).collect();
        let tau = ring * 0.8;
        for ((x, c), &t) in x.iter_mut().zip(dsp::cumsum(&fr)).zip(&t) {
            let ph = 2.0 * PI * c / SRF;
            *x += 0.65 * dmath::sin(ph) * dmath::exp(-t / tau);
        }
    }
    let key = f * 13.0 + (seed * 977) as f64 + if boom { 7.0 } else { 0.0 };
    let mut rng = Generator::new(py::imod_py(whole(key), 100_003) as u64);
    let noise: Vec<f64> = (0..n).map(|_| rng.standard_normal()).collect();
    let felt = dsp::convolve_same(&noise, &[1.0 / 48.0; 48]);
    let a6 = dsp::convolve_same(&noise, &[1.0 / 6.0; 6]);
    let a60 = dsp::convolve_same(&noise, &[1.0 / 60.0; 60]);
    let (felt_k, air_tau, air_k) = if boom { (2.2, 0.09, 0.10) } else { (1.6, 0.03, 0.05) };
    let w_shell = 2.0 * PI * f * 1.5 * 2.1;
    let drive = if boom { 1.25 } else { 1.0 };
    let top = dmath::np_tanh(drive);
    (0..n)
        .map(|i| {
            let t = t[i];
            let felt = felt[i] * dmath::exp(-t / 0.006) * felt_k;
            let air = (a6[i] - a60[i]) * dmath::exp(-t / air_tau) * air_k;
            let shell = 0.12 * dmath::sin(w_shell * t) * dmath::exp(-t / 0.05);
            let y = x[i] + felt + air + shell;
            let mut y = dmath::np_tanh(y * drive) / top;
            // x *= np.minimum(1.0, t / .0015)
            let rise = t / 0.0015;
            y *= if rise < 1.0 { rise } else { 1.0 };
            if let Some(dur) = dur
                && !boom
            {
                // damped (a short tap)
                y *= if t < dur + 0.06 {
                    1.0
                } else {
                    dmath::exp(-(t - dur - 0.06) / 0.05)
                };
            }
            y * v * 0.9
        })
        .collect()
}

/// `bd_roll(f, dur, v, bpm=BPM, seed=0)`: single strokes on 32nds at `bpm` for `dur` s, the two mallets slightly
/// uneven, swelling to an accent: the last stroke is a BOOM.
///
/// # Panics
/// Where Python raises: `dur` is not finite, or so far below zero that the roll has a negative number of samples.
pub fn bd_roll(f: f64, dur: f64, v: f64, bpm: f64, seed: i64) -> Vec<f64> {
    let s32 = 60.0 / bpm / 8.0;
    let k = (py::round_half_even(dur / s32) as i64).max(2);
    let mut out = vec![0.0; pp::len(dur + 1.4)];
    let key = f * 31.0 + dur * 101.0 + seed as f64;
    let mut rng = Generator::new(py::imod_py(whole(key), 100_003) as u64);
    for i in 0..k {
        let u = i as f64 / (k - 1).max(1) as f64;
        let last = i == k - 1;
        let odd = i % 2 == 1;
        let lag = if odd { 0.004 } else { 0.0 };
        let ts = py::py_max(0.0, i as f64 * s32 + lag + rng.uniform(-0.002, 0.002));
        let vel = if last {
            v * 1.05
        } else {
            let hand = if odd { 0.9 } else { 1.0 };
            v * ((0.38 + 0.42 * u) * hand)
        };
        let x = bd_hit(f, vel, last, i + seed, if last { None } else { Some(s32) });
        let a = (ts * SRF) as usize;
        let m = x.len().min(out.len().saturating_sub(a));
        for (o, &x) in out[a..a + m].iter_mut().zip(&x[..m]) {
            *o += x;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_voice_draws_no_noise_and_picks_its_stroke_by_the_note() {
        let mut noise = PpNoise::start();
        let note = BassDrumVoice.note(&mut noise, 38.0, 0.7, 0.2);
        assert_eq!(note, pp::to_48k(&synth(38.0, 0.7, 0.2)));
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        let f = pp::midi_hz(38.0);
        // a tap is int((.42 * 1.6 + .05) * SR) samples, a boom int((1.25 * 1.6 + .05) * SR)
        assert_eq!(synth(38.0, 0.7, 0.2), bd_hit(f, 0.7, false, 0, Some(0.2)));
        assert_eq!(synth(38.0, 0.7, 0.2).len(), pp::len(0.42 * 1.6 + 0.05));
        assert_eq!(synth(38.0, 0.95, 0.2), bd_hit(f, 0.95, true, 0, Some(0.2)));
        assert_eq!(synth(38.0, 0.95, 0.2).len(), pp::len(1.25 * 1.6 + 0.05));
        // the boom does not read the held length; the tap does
        assert_eq!(synth(38.0, 0.95, 0.2), synth(38.0, 0.95, 1.0));
        assert_ne!(synth(38.0, 0.7, 0.2), synth(38.0, 0.7, 0.02));
        // under MIDI 24 a roll on midi + 24
        assert_eq!(synth(14.0, 0.8, 1.0), bd_roll(f, 1.0, 0.8, BPM, 0));
        assert_eq!(synth(14.0, 0.8, 1.0).len(), pp::len(2.4));
        assert!(BassDrumVoice.can_hold(0.0) && !BassDrumVoice.can_hold(f64::NAN));
        assert!(!BassDrumVoice.can_hold(-1.5));
        assert_eq!((BassDrumVoice.register(), BassDrumVoice.name()), (REGISTER, "BASS DRUM"));
    }
}
