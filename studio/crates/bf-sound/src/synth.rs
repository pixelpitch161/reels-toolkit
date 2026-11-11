//! `reelkit/audio.py`'s building blocks at 48,000 Hz: time and pitch helpers, the filter shorthands, envelopes,
//! oscillators, drums, effects, tonal voices, the reverb and the tape stop. Same names as the Python.
//!
//! Rust has no default arguments, so every function takes all of Python's arguments, in Python's order, and the
//! default of each optional one is a constant here named `<FUNCTION>_<ARGUMENT>` (`KICK_DEC`, `THUD_SEED`, ...):
//! `A.kick()` is `kick(KICK_PUNCH, KICK_DEC, KICK_F0, KICK_F1)`, `A.thud(0.22, 95, 55, seed=3)` is
//! `thud(0.22, 95.0, 55.0, 3)`. Numbers Python passes as ints (`f0=150`, a MIDI note, `cut=1600`) are `f64` here:
//! the conversion is exact.
//!
//! Each sample is computed with the same operations in the same order as numpy computes it; `sin`, `exp` and
//! `pow` come from `dmath` (numpy's own loops can differ from it in the last bit), sums that numpy takes in a
//! special order come from [`crate::dsp`], and noise is `np.random.default_rng(seed).normal(0, 1, n)` through
//! `bf_core::rng::Generator`. Where Python would raise (a negative length, an empty signal handed to a filter, a
//! band of a sweep with no sample of its own) these functions panic.

use bf_core::dmath::{self, PI};
use bf_core::py::fmod_py;
use bf_core::rng::Generator;

use crate::SR;
use crate::dsp::{self, FilterKind, Sos};

/// The sample rate as a float.
const SRF: f64 = SR as f64;

/// The filters' default order: `o=2` of `sos`, `hp`, `lp` and `bp`.
pub const FILTER_O: usize = 2;
/// `adsr`'s default attack, in seconds.
pub const ADSR_A: f64 = 0.005;
/// `adsr`'s default decay, in seconds.
pub const ADSR_D: f64 = 0.1;
/// `adsr`'s default sustain level.
pub const ADSR_S: f64 = 0.6;
/// `adsr`'s default release, in seconds.
pub const ADSR_R: f64 = 0.05;
/// `expenv`'s default attack, in seconds.
pub const EXPENV_ATTACK: f64 = 0.002;
/// `saw`'s default detune.
pub const SAW_DET: f64 = 0.0;
/// `saw`'s default number of voices.
pub const SAW_VOICES: usize = 1;
/// `square`'s default duty cycle.
pub const SQUARE_DUTY: f64 = 0.5;
/// `kick`'s default `punch`.
pub const KICK_PUNCH: f64 = 1.0;
/// `kick`'s default decay, in seconds.
pub const KICK_DEC: f64 = 0.32;
/// `kick`'s default start frequency, in Hz.
pub const KICK_F0: f64 = 150.0;
/// `kick`'s default end frequency, in Hz.
pub const KICK_F1: f64 = 46.0;
/// `snare`'s default decay, in seconds.
pub const SNARE_DEC: f64 = 0.14;
/// `hat`'s default decay, in seconds.
pub const HAT_DEC: f64 = 0.035;
/// `hat`'s default `open_`.
pub const HAT_OPEN: bool = false;
/// `crash`'s default length, in seconds.
pub const CRASH_DUR: f64 = 2.0;
/// `whoosh`'s default low band centre, in Hz.
pub const WHOOSH_LO: f64 = 300.0;
/// `whoosh`'s default high band centre, in Hz.
pub const WHOOSH_HI: f64 = 5000.0;
/// `whoosh`'s default direction: rising.
pub const WHOOSH_RISE: bool = true;
/// `whoosh`'s default noise seed.
pub const WHOOSH_SEED: u64 = 6;
/// `thud`'s default length, in seconds.
pub const THUD_DUR: f64 = 0.32;
/// `thud`'s default start frequency, in Hz.
pub const THUD_F0: f64 = 120.0;
/// `thud`'s default end frequency, in Hz.
pub const THUD_F1: f64 = 48.0;
/// `thud`'s default noise seed.
pub const THUD_SEED: u64 = 7;
/// `pluck`'s default length, in seconds.
pub const PLUCK_DUR: f64 = 1.2;
/// `pad`'s default low-pass cutoff, in Hz.
pub const PAD_CUT: f64 = 1600.0;
/// `pad`'s default attack, in seconds.
pub const PAD_ATT: f64 = 0.3;
/// `chime`'s default length, in seconds.
pub const CHIME_DUR: f64 = 1.2;
/// `reverb`'s default impulse-response length, in seconds.
pub const REVERB_DUR: f64 = 2.4;
/// `reverb`'s default decay time, in seconds.
pub const REVERB_DECAY: f64 = 0.6;
/// `reverb`'s default noise seed.
pub const REVERB_SEED: u64 = 3;
/// `tape_stop`'s default length, in seconds.
pub const TAPE_STOP_LENGTH: f64 = 1.5;

/// Python's `int(x)` for a length: cut toward zero.
///
/// # Panics
/// For a negative (or NaN) length, where numpy would refuse to make the array.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// `np.random.default_rng(seed).normal(0, 1, n)`.
fn noise(seed: u64, n: usize) -> Vec<f64> {
    let mut x = vec![0.0; n];
    Generator::new(seed).fill_normal(0.0, 1.0, &mut x);
    x
}

/// `np.clip(x, 0, 1)`.
fn clip01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

// ---------------------------------------------------------------- time, pitch, filters

/// `tt(n)`: the time of each of `n` samples, in seconds (`np.arange(n) / SR`).
pub fn tt(n: usize) -> Vec<f64> {
    (0..n).map(|i| i as f64 / SRF).collect()
}

/// `mtof(m)`: the frequency of MIDI note `m`, `440.0 * 2 ** ((m - 69) / 12)`.
pub fn mtof(m: f64) -> f64 {
    440.0 * dmath::pow(2.0, (m - 69.0) / 12.0)
}

/// `sos(kind, f, o)`: `butter(o, f, kind, fs=SR, output='sos')`; `f` holds one cutoff, or two for a band-pass.
pub fn sos(kind: FilterKind, f: &[f64], o: usize) -> Sos {
    dsp::butter_sos(kind, f, o, SRF)
}

/// `hp(x, f, o)`: `x` through a Butterworth high-pass at `f` Hz.
///
/// # Panics
/// Where scipy raises: `x` is empty, or `f` is not between 0 and half the sample rate.
pub fn hp(x: &[f64], f: f64, o: usize) -> Vec<f64> {
    dsp::sosfilt(&sos(FilterKind::Hp, &[f], o), x)
}

/// `lp(x, f, o)`: `x` through a Butterworth low-pass at `f` Hz.
///
/// # Panics
/// As [`hp`].
pub fn lp(x: &[f64], f: f64, o: usize) -> Vec<f64> {
    dsp::sosfilt(&sos(FilterKind::Lp, &[f], o), x)
}

/// `bp(x, lo, hi, o)`: `x` through a Butterworth band-pass from `lo` to `hi` Hz.
///
/// # Panics
/// Where scipy raises: `x` is empty, or the edges are not ascending between 0 and half the sample rate.
pub fn bp(x: &[f64], lo: f64, hi: f64, o: usize) -> Vec<f64> {
    dsp::sosfilt(&sos(FilterKind::Bp, &[lo, hi], o), x)
}

// ---------------------------------------------------------------- envelopes

/// `adsr(n, a, d, s, r)`: an attack, decay, sustain, release envelope of `n` samples (times in seconds, `s` the
/// sustain level). The release multiplies the end of whatever is there, and is left out when it would cover the
/// whole envelope.
pub fn adsr(n: usize, a: f64, d: f64, s: f64, r: f64) -> Vec<f64> {
    let mut e = vec![1.0 * s; n];
    let (na, nd, nr) = (length(a * SRF), length(d * SRF), length(r * SRF));
    let na = na.min(n);
    e[..na].copy_from_slice(&dsp::linspace(0.0, 1.0, na));
    let nd = nd.min(n - na);
    e[na..na + nd].copy_from_slice(&dsp::linspace(1.0, s, nd));
    if nr > 0 && nr < n {
        for (v, fade) in e[n - nr..].iter_mut().zip(dsp::linspace(1.0, 0.0, nr)) {
            *v *= fade;
        }
    }
    e
}

/// `expenv(n, decay, attack)`: a linear attack into an exponential decay, `clip(t / max(attack, 1e-4), 0, 1) *
/// exp(-t / decay)`.
pub fn expenv(n: usize, decay: f64, attack: f64) -> Vec<f64> {
    let attack = if 1e-4 > attack { 1e-4 } else { attack };
    tt(n)
        .iter()
        .map(|&t| clip01(t / attack) * dmath::exp(-t / decay))
        .collect()
}

// ---------------------------------------------------------------- oscillators

/// `saw(f, n, det, voices)`: `voices` sawtooth waves around `f` Hz, each detuned by `det` times its distance
/// from the middle voice and started `0.37` of a cycle after the one before; their mean.
pub fn saw(f: f64, n: usize, det: f64, voices: usize) -> Vec<f64> {
    let t = tt(n);
    let mut out = vec![0.0; n];
    for v in 0..voices {
        let fv = f * (1.0 + det * (v as f64 - (voices as f64 - 1.0) / 2.0));
        let phase = v as f64 * 0.37;
        for (o, &t) in out.iter_mut().zip(&t) {
            *o += 2.0 * fmod_py(t * fv + phase, 1.0) - 1.0;
        }
    }
    for o in &mut out {
        *o /= voices as f64;
    }
    out
}

/// `square(f, n, duty)`: a square wave at `f` Hz, +1 for the first `duty` of each cycle and -1 for the rest.
pub fn square(f: f64, n: usize, duty: f64) -> Vec<f64> {
    tt(n)
        .iter()
        .map(|&t| if fmod_py(t * f, 1.0) < duty { 1.0 } else { -1.0 })
        .collect()
}

/// `tri(f, n)`: a triangle wave at `f` Hz, starting at its peak.
pub fn tri(f: f64, n: usize) -> Vec<f64> {
    tt(n)
        .iter()
        .map(|&t| 2.0 * (2.0 * fmod_py(t * f, 1.0) - 1.0).abs() - 1.0)
        .collect()
}

/// The phase `2 * np.pi * np.cumsum(f) / SR` of a tone whose frequency falls from `f0` to `f1` Hz as
/// `f1 + (f0 - f1) * exp(-t / fall)`.
fn falling_phase(t: &[f64], f0: f64, f1: f64, fall: f64) -> Vec<f64> {
    let f: Vec<f64> = t.iter().map(|&t| f1 + (f0 - f1) * dmath::exp(-t / fall)).collect();
    dsp::cumsum(&f).iter().map(|&c| 2.0 * PI * c / SRF).collect()
}

// ---------------------------------------------------------------- drums

/// `kick(punch, dec, f0, f1)`: half a second of a sine falling from `f0` to `f1` Hz with a click of noise
/// (seed 1) on its front.
pub fn kick(punch: f64, dec: f64, f0: f64, f1: f64) -> Vec<f64> {
    let n = length(0.5 * SRF);
    let t = tt(n);
    let ph = falling_phase(&t, f0, f1, 0.04);
    let click = noise(1, n);
    (0..n)
        .map(|i| dmath::sin(ph[i]) * dmath::exp(-t[i] / dec) * punch + 0.3 * dmath::exp(-t[i] / 0.004) * click[i])
        .collect()
}

/// `snare(dec)`: 0.35 s of band-passed noise (seed 2) over a short 190 Hz tone.
pub fn snare(dec: f64) -> Vec<f64> {
    let n = length(0.35 * SRF);
    let t = tt(n);
    let rattle = bp(&noise(2, n), 1500.0, 8000.0, FILTER_O);
    (0..n)
        .map(|i| {
            rattle[i] * dmath::exp(-t[i] / dec) * 0.7
                + dmath::sin(2.0 * PI * 190.0 * t[i]) * dmath::exp(-t[i] / 0.05) * 0.5
        })
        .collect()
}

/// `clap()`: 0.3 s of band-passed noise (seed 3) under three quick bursts and a tail.
pub fn clap() -> Vec<f64> {
    let n = length(0.3 * SRF);
    let t = tt(n);
    let hiss = bp(&noise(3, n), 900.0, 5000.0, FILTER_O);
    (0..n)
        .map(|i| {
            // Python's `sum(...)` over the three bursts starts from 0.
            let mut env = 0.0;
            for d in [0.0, 0.011, 0.022] {
                let since = (t[i] - d).max(0.0);
                let on = if t[i] >= d { 1.0 } else { 0.0 };
                env += dmath::exp(-since / 0.008) * on;
            }
            let env = env + dmath::exp(-t[i] / 0.09) * 0.6;
            hiss[i] * env * 0.5
        })
        .collect()
}

/// `hat(dec, open_)`: high-passed noise (seed 4): 0.12 s decaying over `dec` seconds, or 0.4 s decaying over
/// 0.18 s when open.
pub fn hat(dec: f64, open: bool) -> Vec<f64> {
    let (seconds, decay) = if open { (0.4, 0.18) } else { (0.12, dec) };
    let n = length(seconds * SRF);
    let t = tt(n);
    let hiss = hp(&noise(4, n), 7000.0, FILTER_O);
    (0..n).map(|i| hiss[i] * dmath::exp(-t[i] / decay) * 0.5).collect()
}

/// `crash(dur)`: `dur` seconds of high-passed noise (seed 5), decaying.
pub fn crash(dur: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = tt(n);
    let hiss = hp(&noise(5, n), 4000.0, FILTER_O);
    (0..n)
        .map(|i| hiss[i] * dmath::exp(-t[i] / (dur * 0.35)) * 0.5)
        .collect()
}

/// `whoosh(dur, lo, hi, rise, seed)`: `dur` seconds of noise swept through 24 bands from `lo` to `hi` Hz (or
/// down, if `rise` is false), each band filtered with a 2,048-sample run-in, under a `sin^2` swell.
///
/// # Panics
/// Where Python raises: under 24 samples, when a band has no sample of its own.
pub fn whoosh(dur: f64, lo: f64, hi: f64, rise: bool, seed: u64) -> Vec<f64> {
    let n = length(dur * SRF);
    let x = noise(seed, n);
    let mut out = vec![0.0; n];
    let nb = 24;
    for k in 0..nb {
        let u = (k as f64 + 0.5) / nb as f64;
        let fc = lo * dmath::pow(hi / lo, if rise { u } else { 1.0 - u });
        let a = ((k * n) as f64 / nb as f64) as usize;
        let b = (((k + 1) * n) as f64 / nb as f64) as usize;
        // A band with no sample of its own: Python fails there, on filtering nothing or on storing `[-0:]` (the
        // whole filtered run) into no room.
        assert!(b > a, "whoosh: {n} samples are too few for 24 bands");
        let top = SRF / 2.0 - 100.0;
        let high = fc * 1.25;
        let band = bp(
            &x[a.saturating_sub(2048)..b],
            fc * 0.8,
            if top < high { top } else { high },
            FILTER_O,
        );
        // `[-(b - a):]` of the filtered run.
        out[a..b].copy_from_slice(&band[band.len() - (b - a)..]);
    }
    let swell = dsp::linspace(0.0, PI, n);
    out.iter()
        .zip(&swell)
        .map(|(&o, &w)| {
            let s = dmath::sin(w);
            o * (s * s)
        })
        .collect()
}

/// `thud(dur, f0, f1, seed)`: a soft low thud, a sine falling from `f0` to `f1` Hz with a puff of low-passed
/// noise, faded in over 2 ms.
pub fn thud(dur: f64, f0: f64, f1: f64, seed: u64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = tt(n);
    let ph = falling_phase(&t, f0, f1, 0.03);
    let low = lp(&noise(seed, n), 700.0, FILTER_O);
    (0..n)
        .map(|i| {
            let body = dmath::sin(ph[i]) * dmath::exp(-t[i] / (dur * 0.28));
            let puff = low[i] * dmath::exp(-t[i] / 0.035) * 0.5;
            (body + puff) * clip01(t[i] / 0.002)
        })
        .collect()
}

// ---------------------------------------------------------------- tonal

/// `pluck(m, dur)`: MIDI note `m` as a sine with a short-lived octave, under `expenv(n, dur * 0.3)`.
pub fn pluck(m: f64, dur: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let f = mtof(m);
    let t = tt(n);
    let env = expenv(n, dur * 0.3, EXPENV_ATTACK);
    (0..n)
        .map(|i| {
            (dmath::sin(2.0 * PI * f * t[i]) + 0.4 * dmath::sin(4.0 * PI * f * t[i]) * dmath::exp(-t[i] / 0.15))
                * env[i]
        })
        .collect()
}

/// `pad(ms, dur, cut, att)`: the MIDI notes `ms` as detuned three-voice saws, averaged, low-passed at `cut` Hz,
/// under a slow envelope.
///
/// # Panics
/// Where Python raises: `ms` is empty.
pub fn pad(ms: &[f64], dur: f64, cut: f64, att: f64) -> Vec<f64> {
    assert!(!ms.is_empty(), "pad: no notes");
    let n = length(dur * SRF);
    // Python's `sum(...)` over the notes starts from 0.
    let mut x = vec![0.0; n];
    for &m in ms {
        for (v, s) in x.iter_mut().zip(saw(mtof(m), n, 0.006, 3)) {
            *v += s;
        }
    }
    let count = ms.len() as f64;
    for v in &mut x {
        *v /= count;
    }
    let release = dur * 0.4;
    let env = adsr(n, att, 0.2, 0.8, if release < 0.6 { release } else { 0.6 });
    lp(&x, cut, FILTER_O).iter().zip(&env).map(|(v, e)| v * e).collect()
}

/// `bass(m, dur)`: MIDI note `m` as a sine under a tight envelope.
pub fn bass(m: f64, dur: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = tt(n);
    let w = 2.0 * PI * mtof(m);
    let env = adsr(n, 0.004, 0.1, 0.8, 0.04);
    (0..n).map(|i| dmath::sin(w * t[i]) * env[i]).collect()
}

/// `chime(m, dur)`: MIDI note `m` as a sine with a partial at 2.76 times its frequency, under
/// `expenv(n, dur * 0.35)`.
pub fn chime(m: f64, dur: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = tt(n);
    let f = mtof(m);
    let env = expenv(n, dur * 0.35, EXPENV_ATTACK);
    (0..n)
        .map(|i| (dmath::sin(2.0 * PI * f * t[i]) + 0.3 * dmath::sin(2.0 * PI * f * 2.76 * t[i])) * env[i])
        .collect()
}

// ---------------------------------------------------------------- mixing

/// `reverb(x, dur, decay, seed)`: `x` convolved, channel by channel, with `dur` seconds of decaying stereo noise
/// (low-passed at 5 kHz, silent for its first 20 ms, scaled so the louder channel has unit energy), cut to the
/// length of `x`. The wet signal only.
///
/// # Panics
/// Where Python raises: `dur` is under one sample (there is no tail to filter).
pub fn reverb(x: &[[f64; 2]], dur: f64, decay: f64, seed: u64) -> Vec<[f64; 2]> {
    let mut r = Generator::new(seed);
    let n = length(dur * SRF);
    let t = tt(n);
    // The left channel's noise is drawn first, all of it, then the right's.
    let (mut left, mut right) = (vec![0.0; n], vec![0.0; n]);
    r.fill_normal(0.0, 1.0, &mut left);
    r.fill_normal(0.0, 1.0, &mut right);
    let ir: Vec<[f64; 2]> = (0..n)
        .map(|i| {
            let fall = dmath::exp(-t[i] / decay);
            [left[i] * fall, right[i] * fall]
        })
        .collect();
    let mut ir = dsp::sosfilt_stereo(&sos(FilterKind::Lp, &[5000.0], FILTER_O), &ir);
    let quiet = length(0.02 * SRF).min(n);
    ir[..quiet].fill([0.0; 2]);
    let squares: Vec<[f64; 2]> = ir.iter().map(|&[l, r]| [l * l, r * r]).collect();
    // `np.sqrt((ir ** 2).sum(0)).max()`
    let [sl, sr] = dsp::sum_axis0(&squares).map(f64::sqrt);
    let norm = if sr > sl { sr } else { sl };
    let channel = |c: usize| -> Vec<f64> {
        let dry: Vec<f64> = x.iter().map(|frame| frame[c]).collect();
        let tail: Vec<f64> = ir.iter().map(|frame| frame[c] / norm).collect();
        dsp::fftconvolve(&dry, &tail)
    };
    let (wet_l, wet_r) = (channel(0), channel(1));
    (0..x.len().min(wet_l.len())).map(|i| [wet_l[i], wet_r[i]]).collect()
}

/// `tape_stop(x, t0, length)`: the buffer with a pitch-down-to-silence starting at `t0` seconds: over `length`
/// seconds the playback rate falls as `linspace(1, 0, n) ** 1.5` while the level fades as its square root;
/// everything after is silent.
///
/// # Panics
/// Where Python raises: `x` ends before `t0 + length`.
pub fn tape_stop(x: &[[f64; 2]], t0: f64, length: f64) -> Vec<[f64; 2]> {
    let i0 = self::length(t0 * SRF);
    let n = self::length(length * SRF);
    let mut y = x.to_vec();
    if n > 0 {
        assert!(
            i0 + n <= x.len(),
            "tape_stop: the buffer has {} samples, the stop ends at sample {}",
            x.len(),
            i0 + n
        );
        let seg = &x[i0..(i0 + n * 2).min(x.len())];
        let ramp = dsp::linspace(1.0, 0.0, n);
        let rate: Vec<f64> = ramp.iter().map(|&v| dmath::pow(v, 1.5)).collect();
        let pos = dsp::cumsum(&rate);
        let at: Vec<f64> = (0..seg.len()).map(|i| i as f64).collect();
        let read = |c: usize| -> Vec<f64> {
            let channel: Vec<f64> = seg.iter().map(|frame| frame[c]).collect();
            dsp::interp(&pos, &at, &channel)
        };
        let (left, right) = (read(0), read(1));
        for i in 0..n {
            // `** 0.5` on an array is numpy's square root.
            let fade = ramp[i].sqrt();
            y[i0 + i] = [left[i] * fade, right[i] * fade];
        }
    }
    let end = (i0 + n).min(y.len());
    y[end..].fill([0.0; 2]);
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_and_pitch() {
        assert_eq!(tt(3), [0.0, 1.0 / 48000.0, 2.0 / 48000.0]);
        assert!(tt(0).is_empty());
        assert_eq!(mtof(69.0), 440.0);
        assert_eq!(mtof(81.0), 880.0);
        assert_eq!(mtof(57.0), 220.0);
        assert!((mtof(60.0) - 261.6255653005986).abs() < 1e-12);
    }

    #[test]
    fn envelope_shapes() {
        // 5 ms up, 100 ms down to the sustain level, a 50 ms release that scales what is there.
        let e = adsr(48000, ADSR_A, ADSR_D, ADSR_S, ADSR_R);
        assert_eq!((e[0], e[239], e[240]), (0.0, 1.0, 1.0));
        assert_eq!((e[240 + 4799], e[10000]), (0.6, 0.6));
        assert_eq!(e[47999], 0.0);
        assert!(e[48000 - 2400] == 0.6 && e[48000 - 1200] < 0.31);
        // Shorter than its attack: only the start of the ramp fits; and no release when it would cover it all.
        assert_eq!(adsr(3, 1.0, 1.0, 0.5, 1.0), dsp::linspace(0.0, 1.0, 3));
        assert!(adsr(0, ADSR_A, ADSR_D, ADSR_S, ADSR_R).is_empty());
        let e = expenv(4800, 0.1, EXPENV_ATTACK);
        assert_eq!(e[0], 0.0);
        assert_eq!(e[96], dmath::exp(-0.002 / 0.1));
        assert!(e[95] < e[96] && e[97] < e[96]);
    }

    #[test]
    fn oscillator_shapes() {
        // 12,000 Hz at 48,000 Hz: four samples a cycle.
        assert_eq!(saw(12000.0, 4, SAW_DET, SAW_VOICES), [-1.0, -0.5, 0.0, 0.5]);
        assert_eq!(square(12000.0, 4, SQUARE_DUTY), [1.0, 1.0, -1.0, -1.0]);
        assert_eq!(square(12000.0, 4, 0.25), [1.0, -1.0, -1.0, -1.0]);
        assert_eq!(tri(12000.0, 5), [1.0, 0.0, -1.0, 0.0, 1.0]);
        // Three voices: the mean of three saws, the outer two detuned and each started later in its cycle.
        let three = saw(1000.0, 64, 0.01, 3);
        let voice = |f: f64, phase: f64, i: usize| 2.0 * fmod_py(i as f64 / 48000.0 * f + phase, 1.0) - 1.0;
        for (i, v) in three.iter().enumerate() {
            let want = (voice(990.0, 0.0, i) + voice(1000.0, 0.37, i) + voice(1010.0, 0.74, i)) / 3.0;
            assert!((v - want).abs() < 1e-12, "sample {i}");
        }
    }

    #[test]
    fn lengths_follow_pythons_int() {
        assert_eq!(kick(KICK_PUNCH, KICK_DEC, KICK_F0, KICK_F1).len(), 24000);
        assert_eq!(snare(SNARE_DEC).len(), 16800);
        assert_eq!(clap().len(), 14400);
        assert_eq!(hat(HAT_DEC, HAT_OPEN).len(), 5760);
        assert_eq!(hat(HAT_DEC, true).len(), 19200);
        assert_eq!(crash(0.5).len(), 24000);
        assert_eq!(thud(THUD_DUR, THUD_F0, THUD_F1, THUD_SEED).len(), 15360);
        assert_eq!(whoosh(0.6, WHOOSH_LO, WHOOSH_HI, WHOOSH_RISE, WHOOSH_SEED).len(), 28800);
        assert_eq!(pluck(60.0, 0.1).len(), 4800);
        assert_eq!(pad(&[60.0, 64.0], 0.1, PAD_CUT, PAD_ATT).len(), 4800);
        assert_eq!(bass(40.0, 0.1).len(), 4800);
        assert_eq!(chime(84.0, 0.1).len(), 4800);
    }

    #[test]
    fn a_filtered_sound_of_no_length_is_refused() {
        // scipy's sosfilt raises on an empty signal, so these raise in Python.
        let refused = |f: &dyn Fn()| {
            let payload = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).expect_err("it did not panic");
            let text = payload.downcast_ref::<String>().cloned();
            text.or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                .expect("a panic with a message")
        };
        assert!(refused(&|| drop(hp(&[], 100.0, FILTER_O))).contains("sosfilt: x is empty"));
        assert!(refused(&|| drop(lp(&[], 100.0, FILTER_O))).contains("sosfilt: x is empty"));
        assert!(refused(&|| drop(bp(&[], 100.0, 200.0, FILTER_O))).contains("sosfilt: x is empty"));
        assert!(refused(&|| drop(crash(0.0))).contains("sosfilt: x is empty"));
        assert!(refused(&|| drop(thud(0.0, THUD_F0, THUD_F1, THUD_SEED))).contains("sosfilt: x is empty"));
        assert!(refused(&|| drop(pad(&[60.0], 0.0, PAD_CUT, PAD_ATT))).contains("sosfilt: x is empty"));
        assert!(
            refused(&|| drop(reverb(&[[1.0, 1.0]], 0.0, REVERB_DECAY, REVERB_SEED))).contains("sosfilt: x is empty")
        );
        let few = "whoosh: 0 samples are too few for 24 bands";
        assert!(refused(&|| drop(whoosh(0.0, WHOOSH_LO, WHOOSH_HI, WHOOSH_RISE, WHOOSH_SEED))).contains(few));
        // The unfiltered ones are simply empty, as in Python; so is a reverb of nothing.
        assert!(pluck(60.0, 0.0).is_empty() && bass(40.0, 0.0).is_empty() && chime(84.0, 0.0).is_empty());
        assert!(reverb(&[], REVERB_DUR, REVERB_DECAY, REVERB_SEED).is_empty());
    }

    #[test]
    #[should_panic(expected = "whoosh: 10 samples are too few for 24 bands")]
    fn a_whoosh_needs_a_sample_per_band() {
        whoosh(10.5 / 48000.0, WHOOSH_LO, WHOOSH_HI, WHOOSH_RISE, WHOOSH_SEED);
    }

    #[test]
    fn reverb_of_an_impulse_is_its_impulse_response() {
        // 0.2 s of input, 0.1 s of tail: silent for 20 ms, then noise, then nothing after the tail.
        let mut x = vec![[0.0; 2]; 9600];
        x[0] = [1.0, 1.0];
        let wet = reverb(&x, 0.1, 0.05, REVERB_SEED);
        assert_eq!(wet.len(), 9600);
        assert!(wet[..960].iter().all(|f| f[0].abs() < 1e-12 && f[1].abs() < 1e-12));
        assert!(wet[960..4800].iter().any(|f| f[0].abs() > 1e-3));
        assert!(wet[4800..].iter().all(|f| f[0].abs() < 1e-12 && f[1].abs() < 1e-12));
        // The louder channel's energy is 1.
        let energy = |c: usize| wet.iter().map(|f| f[c] * f[c]).sum::<f64>();
        assert!(
            (energy(0).max(energy(1)) - 1.0).abs() < 1e-9,
            "{} {}",
            energy(0),
            energy(1)
        );
        // Shorter than the tail: cut to the input's length.
        assert_eq!(reverb(&x[..100], 0.1, 0.05, REVERB_SEED).len(), 100);
    }

    #[test]
    fn tape_stop_leaves_the_start_and_silences_the_end() {
        let x: Vec<[f64; 2]> = (0..4800).map(|i| [f64::from(i), -f64::from(i)]).collect();
        let y = tape_stop(&x, 0.01, 0.02);
        assert_eq!(y.len(), 4800);
        assert_eq!(y[..480], x[..480]);
        // The stop reads on from sample 480 at a falling rate: its first sample is one step in, at full level.
        assert_eq!(y[480], [481.0, -481.0]);
        assert!(y[480..1440].iter().all(|f| f[0] >= 0.0 && f[0] == -f[1]));
        assert_eq!(y[1439], [0.0, 0.0]);
        assert!(y[1440..].iter().all(|f| *f == [0.0, 0.0]));
    }

    #[test]
    #[should_panic(expected = "tape_stop: the buffer has 100 samples, the stop ends at sample 1440")]
    fn tape_stop_needs_room() {
        tape_stop(&[[0.0; 2]; 100], 0.01, 0.02);
    }
}
