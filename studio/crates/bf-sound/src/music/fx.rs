//! The shared sound effects: `clank`, `hiss`, `roar`, `thunder`, `riser`.

use bf_core::dmath::{self, PI};
use bf_core::py::fmod_py;
use bf_core::rng::Generator;

use super::error::{MusicError, Samples};
use super::num::{SRF, band_span, check_band, check_some, clip, noise, py_min, samples};
use crate::dsp;
use crate::synth::{self, FILTER_O};

/// `clank`'s default noise seed.
pub const CLANK_SEED: u64 = 0;
/// `clank`'s default length, in seconds.
pub const CLANK_DUR: f64 = 0.4;
/// `clank`'s default partials: (frequency ratio, decay in seconds).
pub const CLANK_DECAYS: [(f64, f64); 3] = [(1.0, 0.12), (2.76, 0.06), (5.4, 0.03)];
/// `hiss`'s default length, in seconds.
pub const HISS_DUR: f64 = 0.09;
/// `hiss`'s default noise seed.
pub const HISS_SEED: u64 = 0;
/// `hiss`'s default lower band edge, in Hz.
pub const HISS_LO: f64 = 3500.0;
/// `hiss`'s default upper band edge, in Hz.
pub const HISS_HI: f64 = 10000.0;
/// `hiss`'s default decay, in seconds.
pub const HISS_DEC: f64 = 0.03;
/// `roar`'s default noise seed.
pub const ROAR_SEED: u64 = 3;
/// `thunder`'s default length, in seconds.
pub const THUNDER_DUR: f64 = 1.6;
/// `thunder`'s default noise seed.
pub const THUNDER_SEED: u64 = 0;
/// `thunder`'s default `crack`: with the crack on top.
pub const THUNDER_CRACK: bool = true;
/// `riser`'s default `over`: how far past its end the riser runs, in seconds.
pub const RISER_OVER: f64 = 0.012;
/// `riser`'s default noise seed.
pub const RISER_SEED: u64 = 11;

/// The bands of [`riser`].
const RISER_BANDS: usize = 20;

/// `clank(m, seed, dur, decays)`: metal on canvas: a short metallic ring on MIDI note `m`, a sine for each
/// `(frequency ratio, decay)` of `decays`, with a click of band-passed noise on its front.
///
/// # Errors
/// Where Python raises: `dur` is under one sample or not a number.
pub fn clank(m: f64, seed: u64, dur: f64, decays: &[(f64, f64)]) -> Samples {
    const H: &str = "clank";
    let n = samples(H, dur)?;
    check_some(H, n, dur)?;
    let t = synth::tt(n);
    let f = synth::mtof(m);
    // Python's `sum(...)` over the partials starts from 0.
    let mut x = vec![0.0; n];
    for &(r, d) in decays {
        let w = 2.0 * PI * f * r;
        for (v, &t) in x.iter_mut().zip(&t) {
            *v += dmath::sin(w * t) * dmath::exp(-t / d);
        }
    }
    let click = synth::bp(&noise(seed, n), 2000.0, 8000.0, FILTER_O);
    Ok((0..n)
        .map(|i| x[i] + 0.3 * click[i] * dmath::exp(-t[i] / 0.005))
        .collect())
}

/// `hiss(dur, seed, lo, hi, dec)`: a puff of noise between `lo` and `hi` Hz, decaying over `dec` seconds.
///
/// # Errors
/// Where Python raises: `dur` is under one sample or not a number, or the band does not rise between 0 and half
/// the sample rate.
pub fn hiss(dur: f64, seed: u64, lo: f64, hi: f64, dec: f64) -> Samples {
    const H: &str = "hiss";
    let n = samples(H, dur)?;
    check_some(H, n, dur)?;
    check_band(H, lo, hi)?;
    let t = synth::tt(n);
    let band = synth::bp(&noise(seed, n), lo, hi, FILTER_O);
    Ok((0..n).map(|i| band[i] * dmath::exp(-t[i] / dec)).collect())
}

/// `roar(dur, seed)`: the crowd: two bands of noise that swell over a quarter of a second and die away.
///
/// # Errors
/// Where Python raises: `dur` is under one sample or not a number.
pub fn roar(dur: f64, seed: u64) -> Samples {
    const H: &str = "roar";
    let n = samples(H, dur)?;
    check_some(H, n, dur)?;
    let x = noise(seed, n);
    let low = synth::bp(&x, 300.0, 2600.0, FILTER_O);
    let high = synth::bp(&x, 900.0, 5000.0, FILTER_O);
    let t = synth::tt(n);
    Ok((0..n)
        .map(|i| {
            let since = t[i] - 0.6;
            let late = if since > 0.0 { since } else { 0.0 };
            let env = clip(t[i] / 0.25, 0.0, 1.0) * dmath::exp(-late / (dur * 0.45));
            (low[i] * 0.5 + high[i] * 0.25) * env
        })
        .collect())
}

/// `thunder(dur, seed, crack)`: a rolling thunder rumble, with a crack on top if `crack` (#37). The rumble and
/// the crack take their noise from one generator, the rumble first.
///
/// # Errors
/// Where Python raises: `dur` is under one sample or not a number.
pub fn thunder(dur: f64, seed: u64, crack: bool) -> Samples {
    const H: &str = "thunder";
    let n = samples(H, dur)?;
    check_some(H, n, dur)?;
    let t = synth::tt(n);
    let mut r = Generator::new(seed);
    let mut draw = vec![0.0; n];
    r.fill_normal(0.0, 1.0, &mut draw);
    let low = synth::lp(&draw, 140.0, FILTER_O);
    let phase = seed as f64;
    let mut x: Vec<f64> = (0..n)
        .map(|i| {
            let rumble = low[i] * 3.0 * clip(t[i] / 0.03, 0.0, 1.0) * dmath::exp(-t[i] / (dur * 0.35));
            rumble * (0.7 + 0.3 * dmath::sin(2.0 * PI * 3.1 * t[i] + phase))
        })
        .collect();
    if crack {
        r.fill_normal(0.0, 1.0, &mut draw);
        let high = synth::bp(&draw, 1500.0, 9000.0, FILTER_O);
        for i in 0..n {
            x[i] += high[i] * dmath::exp(-t[i] / 0.03) * 0.9;
        }
    }
    Ok(x)
}

/// `riser(dur, over, seed)`: a noise and saw riser that builds right up to (and `over` seconds past) its end:
/// no fade, so the loop seam stays loud. The noise is filtered in 20 bands in time that climb from 300 Hz to
/// 6 kHz, each with a run-in of 2,048 samples.
///
/// # Errors
/// Where Python raises: `dur + over` is under 20 samples (a band with no sample of its own) or not a number, or
/// a band's centre is not a number (a `dur` of 0 with fewer than 40 samples: the first sample divides 0 by 0).
/// With a `dur` of 0 and more samples than that Python gives NaN for every sample, and so does this.
pub fn riser(dur: f64, over: f64, seed: u64) -> Samples {
    const H: &str = "riser";
    let n = samples(H, dur + over)?;
    if n < RISER_BANDS {
        return Err(MusicError::new(
            H,
            format!("{n} sample(s) are too few for {RISER_BANDS} bands"),
        ));
    }
    let t = synth::tt(n);
    let u: Vec<f64> = t.iter().map(|&t| clip(t / dur, 0.0, 1.05)).collect();
    let x = noise(seed, n);
    let f: Vec<f64> = u.iter().map(|&u| 300.0 * dmath::pow(20.0, u)).collect();
    let rate: Vec<f64> = u.iter().map(|&u| 55.0 * dmath::pow(2.0, u * 2.5)).collect();
    let sweep: Vec<f64> = dsp::cumsum(&rate)
        .iter()
        .map(|&c| {
            let ph = 2.0 * PI * c / SRF;
            2.0 * fmod_py(ph / (2.0 * PI), 1.0) - 1.0
        })
        .collect();
    let mut out = vec![0.0; n];
    for k in 0..RISER_BANDS {
        let (a, b) = band_span(k, n, RISER_BANDS);
        let fc = f[(n - 1).min((a + b) / 2)];
        let hi = py_min(fc * 1.4, SRF / 2.0 - 200.0);
        check_band(H, fc * 0.7, hi)?;
        let band = synth::bp(&x[a.saturating_sub(2048)..b], fc * 0.7, hi, FILTER_O);
        out[a..b].copy_from_slice(&band[band.len() - (b - a)..]);
    }
    let low = synth::lp(&sweep, 2500.0, FILTER_O);
    Ok((0..n).map(|i| (out[i] + 0.35 * low[i]) * (u[i] * u[i])).collect())
}
