//! A plucked string instrument's body: what turns the strings' push on the bridge into sound (`docs/studio/voice-
//! engine-2-instruments.md`, "Acoustic guitar", "Ukulele", "Harp"). A guitar's or a ukulele's box and a harp's
//! soundboard each have a few strong low resonances (the air in the box, the top's first ways of bending) and many
//! small ones above; the force goes straight out a little too. The sum is then shaped so its average response (over
//! thirds of an octave) follows a target, keeping its own peaks and dips inside each third: the body's character
//! from its resonances, its overall tone from the listening rounds (the sketch `pl.py`, `body_ir` and `fit`).
//!
//! The same resonances tell the strings how readily the bridge moves at each frequency ([`admittance`]): where the
//! body takes the string's energy, so where a note dies sooner.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use bf_core::dmath;
use bf_core::rng::Generator;

use crate::dsp::{self, FilterKind, butter_sos, fft::Fft};

/// The many small resonances: from `lo` to `hi` Hz (spread evenly in log frequency), `count` of them, each with
/// quality `q`; their part of the response is `gain` (the strong ones' is [`Body::low`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dense {
    pub lo: f64,
    pub hi: f64,
    pub count: usize,
    pub q: f64,
    pub gain: f64,
}

/// A body's make-up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    /// How long its response is, seconds.
    pub length: f64,
    /// The straight part (the force going out as it is).
    pub direct: f64,
    /// The strong low resonances' part.
    pub low: f64,
    /// Under this the body gives nothing back, Hz.
    pub hp: f64,
    /// How readily the bridge moves between the resonances, at 1 kHz (rising to 4 times that at 4 kHz).
    pub floor_y: f64,
    /// The strong low resonances: frequency (Hz), quality, gain.
    pub modes: &'static [(f64, f64, f64)],
    pub dense: Dense,
    /// The average response, dB at [`CENTRES`] (made relative to 1 kHz).
    pub target: [f64; 12],
}

/// The third-octave frequencies the target is given at, Hz.
pub const CENTRES: [f64; 12] = [
    63.0, 100.0, 160.0, 250.0, 400.0, 630.0, 1000.0, 1600.0, 2500.0, 4000.0, 6300.0, 10_000.0,
];

/// A dreadnought steel-string guitar's low resonances: the air's (Helmholtz) mode, the top's first modes.
const DREAD_MODES: [(f64, f64, f64); 7] = [
    (101.0, 14.0, 1.0),
    (196.0, 22.0, 1.1),
    (234.0, 18.0, 0.5),
    (390.0, 28.0, 0.55),
    (520.0, 30.0, 0.45),
    (680.0, 32.0, 0.35),
    (860.0, 35.0, 0.3),
];
/// A soprano ukulele's: the air and the top an octave or so above a guitar's.
const SOPRANO_MODES: [(f64, f64, f64); 6] = [
    (265.0, 12.0, 0.9),
    (420.0, 20.0, 1.0),
    (520.0, 18.0, 0.5),
    (760.0, 26.0, 0.55),
    (980.0, 28.0, 0.45),
    (1250.0, 30.0, 0.35),
];
/// A concert pedal harp's: a long tapered soundboard on a deep box, many low modes.
const PEDAL_HARP_MODES: [(f64, f64, f64); 8] = [
    (88.0, 8.0, 0.6),
    (130.0, 14.0, 0.9),
    (180.0, 18.0, 1.0),
    (250.0, 20.0, 0.9),
    (330.0, 22.0, 0.8),
    (430.0, 25.0, 0.7),
    (560.0, 28.0, 0.55),
    (720.0, 30.0, 0.45),
];

/// The bodies Aedan picked in the listening rounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BodyKind {
    /// A dreadnought steel-string guitar, heard a little close: a strong low end, a clear top (A6).
    Dreadnought,
    /// A soprano ukulele, heard a little close (U6).
    Soprano,
    /// A concert pedal harp: a soft top end (H3).
    PedalHarp,
}

impl BodyKind {
    /// Its make-up (the sketch's `dread_b`, `soprano_b` and `pedalharp`).
    pub fn body(self) -> Body {
        match self {
            BodyKind::Dreadnought => Body {
                length: 0.6,
                direct: 0.5,
                low: 1.0,
                hp: 60.0,
                floor_y: 0.02,
                modes: &DREAD_MODES,
                dense: Dense {
                    lo: 700.0,
                    hi: 12_000.0,
                    count: 1000,
                    q: 35.0,
                    gain: 0.6,
                },
                target: [-10.0, 0.0, -4.0, -2.0, -6.0, -6.0, -5.0, -3.0, -1.0, -1.0, -3.0, -8.0],
            },
            BodyKind::Soprano => Body {
                length: 0.35,
                direct: 0.55,
                low: 0.9,
                hp: 140.0,
                floor_y: 0.03,
                modes: &SOPRANO_MODES,
                dense: Dense {
                    lo: 1100.0,
                    hi: 13_000.0,
                    count: 800,
                    q: 30.0,
                    gain: 0.65,
                },
                target: [-30.0, -20.0, -10.0, 0.0, -1.0, -3.0, -2.0, 0.0, 1.0, 1.0, -2.0, -7.0],
            },
            BodyKind::PedalHarp => Body {
                length: 0.8,
                direct: 0.4,
                low: 0.55,
                hp: 50.0,
                floor_y: 0.015,
                modes: &PEDAL_HARP_MODES,
                dense: Dense {
                    lo: 220.0,
                    hi: 9000.0,
                    count: 1500,
                    q: 40.0,
                    gain: 0.8,
                },
                target: [-18.0, -13.0, -9.0, -5.0, -2.0, 0.0, 0.0, -1.0, -4.0, -9.0, -15.0, -24.0],
            },
        }
    }
}

/// The two microphones' seeds (numpy's `default_rng(11 + mic)` in the sketch: the same bodies).
const SEEDS: [u64; 2] = [11, 12];
/// The correction's length at 48 kHz, samples (85 ms).
const FIT_TAPS: f64 = 4096.0;
/// How far the correction may lift or cut, dB.
const FIT_RANGE: (f64, f64) = (-40.0, 30.0);
/// How fast the target keeps falling above its last centre, dB an octave.
const FALL_PAST_TOP: f64 = 6.0;

/// The body's response at sample rate `fs`, made once per body and rate: what both microphones of the sketch share
/// (the middle of the two; the voice's own width spreads it again), fitted to the target, then high-passed.
pub fn response(kind: BodyKind, fs: f64) -> Arc<Vec<f64>> {
    type Made = Mutex<HashMap<(BodyKind, u64), Arc<Vec<f64>>>>;
    static MADE: OnceLock<Made> = OnceLock::new();
    let made = MADE.get_or_init(|| Mutex::new(HashMap::new()));
    let at = (kind, fs.to_bits());
    if let Some(r) = made.lock().map_or(None, |m| m.get(&at).cloned()) {
        return r;
    }
    // Two callers can miss at once and both make it: the first one kept is everyone's.
    let r = Arc::new(make(&kind.body(), fs));
    match made.lock() {
        Ok(mut m) => Arc::clone(m.entry(at).or_insert(r)),
        Err(_) => r,
    }
}

/// `a sin(2 pi f t + ph) e^(-t / tau)` added into `out`, by turning a phasor one sample at a time.
fn ring(out: &mut [f64], a: f64, f: f64, tau: f64, ph: f64, fs: f64) {
    let r = dmath::exp(-1.0 / (tau * fs));
    let w = dmath::TAU * f / fs;
    let (sr, si) = (r * dmath::cos(w), r * dmath::sin(w));
    let (mut zr, mut zi) = (a * dmath::cos(ph), a * dmath::sin(ph));
    for v in out.iter_mut() {
        *v += zi;
        (zr, zi) = (zr * sr - zi * si, zr * si + zi * sr);
    }
}

fn energy(x: &[f64]) -> f64 {
    x.iter().map(|v| v * v).sum::<f64>().sqrt()
}

/// One microphone's response, before the fit: the straight part, the strong resonances (each normalised to the
/// same energy as a whole) and the small ones, the draws in the sketch's order.
fn raw(b: &Body, seed: u64, fs: f64) -> Vec<f64> {
    let n = (b.length * fs) as usize;
    let mut rng = Generator::new(seed);
    let mut low = vec![0.0; n];
    for &(f, q, g) in b.modes {
        let tau = q / (dmath::PI * f);
        let ph = rng.uniform(-0.3, 0.3);
        if f < 0.5 * fs {
            ring(&mut low, g / tau.sqrt(), f, tau, ph, fs);
        }
    }
    let d = b.dense;
    let (lo, hi) = (dmath::ln(d.lo), dmath::ln(d.hi));
    let f: Vec<f64> = (0..d.count).map(|_| dmath::exp(rng.uniform(lo, hi))).collect();
    let tau: Vec<f64> = f.iter().map(|f| d.q / (dmath::PI * f)).collect();
    let a: Vec<f64> = tau.iter().map(|t| rng.standard_normal() / t.sqrt()).collect();
    let ph: Vec<f64> = (0..d.count).map(|_| rng.uniform(0.0, dmath::TAU)).collect();
    let mut dense = vec![0.0; n];
    for i in 0..d.count {
        if f[i] > 0.45 * fs {
            continue;
        }
        let m = ((8.0 * tau[i] * fs) as usize).min(n);
        ring(&mut dense[..m], a[i], f[i], tau[i], ph[i], fs);
    }
    let (e1, e2) = (energy(&low), energy(&dense));
    let (e1, e2) = (if e1 > 0.0 { e1 } else { 1.0 }, if e2 > 0.0 { e2 } else { 1.0 });
    let mut ir: Vec<f64> = low
        .iter()
        .zip(&dense)
        .map(|(l, s)| b.low * l / e1 + d.gain * s / e2)
        .collect();
    if let Some(v) = ir.first_mut() {
        *v += b.direct;
    }
    ir
}

/// `np.interp`: `fp` at `x` along the rising `xp`, held at the ends.
fn interp(x: f64, xp: &[f64], fp: &[f64]) -> f64 {
    let n = xp.len();
    if x <= xp[0] {
        return fp[0];
    }
    if x >= xp[n - 1] {
        return fp[n - 1];
    }
    let j = xp.partition_point(|&v| v <= x);
    let (x0, x1) = (xp[j - 1], xp[j]);
    fp[j - 1] + (fp[j] - fp[j - 1]) * (x - x0) / (x1 - x0)
}

/// The power spectrum `p` (at the frequencies `f`) in dB, averaged over a third of an octave around each frequency.
fn smooth_db(p: &[f64], f: &[f64]) -> Vec<f64> {
    let mut c = Vec::with_capacity(p.len());
    let mut sum = 0.0;
    for v in p {
        sum += v;
        c.push(sum);
    }
    let (down, up) = (dmath::pow(2.0, -0.5 / 3.0), dmath::pow(2.0, 0.5 / 3.0));
    let last = c.len() - 1;
    f.iter()
        .map(|&fk| {
            let lo = f.partition_point(|&v| v < fk * down);
            let hi = f.partition_point(|&v| v < fk * up).max(lo + 1);
            let m = (c[hi.min(last)] - c[lo]) / (hi - lo) as f64;
            10.0 * dmath::log10(m.max(1e-30))
        })
        .collect()
}

/// `ir` reshaped so its average response follows `target` (relative to 1 kHz), by a minimum-phase correction that
/// leaves the peaks and dips inside each third alone; energy 1 at 48 kHz (the same level at any rate).
fn fit(ir: &[f64], target: &[f64; 12], fs: f64) -> Vec<f64> {
    let nf = ir.len().max((1.365 * fs) as usize).next_power_of_two();
    let fft = Fft::new(nf);
    let (mut re, mut im) = (vec![0.0; nf], vec![0.0; nf]);
    re[..ir.len()].copy_from_slice(ir);
    fft.forward(&mut re, &mut im);
    let half = nf / 2 + 1;
    let power: Vec<f64> = (0..half).map(|k| re[k] * re[k] + im[k] * im[k]).collect();
    let f: Vec<f64> = (0..half).map(|k| k as f64 * fs / nf as f64).collect();
    let sm = smooth_db(&power, &f);
    let logc: Vec<f64> = CENTRES.iter().map(|&c| dmath::ln(c)).collect();
    let logf: Vec<f64> = f[1..].iter().map(|&v| dmath::ln(v.max(1e-9))).collect();
    let at_1k = dmath::ln(1000.0);
    let reference = interp(at_1k, &logf, &sm[1..]) - interp(at_1k, &logc, target);
    let top = CENTRES[CENTRES.len() - 1];
    // the correction's log magnitude, mirrored into a whole spectrum
    let mut full = vec![0.0; nf];
    for k in 0..half {
        let ff = f[k].clamp(CENTRES[0], top);
        let past = dmath::log2(f[k].max(1.0) / top).max(0.0);
        let tg = interp(dmath::ln(ff), &logc, target) - FALL_PAST_TOP * past;
        let gain_db = (tg - (sm[k] - reference)).clamp(FIT_RANGE.0, FIT_RANGE.1);
        full[k] = gain_db / 20.0 * std::f64::consts::LN_10;
    }
    for k in 1..nf / 2 {
        full[nf - k] = full[k];
    }
    // minimum phase from the real cepstrum: fold the cepstrum onto its causal half and exponentiate
    let mut ci = vec![0.0; nf];
    fft.inverse(&mut full, &mut ci);
    let mut cep = full;
    for v in &mut cep[1..nf / 2] {
        *v *= 2.0;
    }
    for v in &mut cep[nf / 2 + 1..] {
        *v = 0.0;
    }
    let mut cim = vec![0.0; nf];
    fft.forward(&mut cep, &mut cim);
    let (mut er, mut ei) = (vec![0.0; nf], vec![0.0; nf]);
    for k in 0..nf {
        let g = dmath::exp(cep[k]);
        er[k] = g * dmath::cos(cim[k]);
        ei[k] = g * dmath::sin(cim[k]);
    }
    fft.inverse(&mut er, &mut ei);
    // its first 85 ms under the falling half of a Hann window
    let taps = ((FIT_TAPS * fs / 48_000.0).round() as usize).clamp(16, nf);
    let m = (2 * taps - 1) as f64;
    let hmin: Vec<f64> = (0..taps)
        .map(|i| er[i] * (0.5 - 0.5 * dmath::cos(dmath::TAU * (taps + i) as f64 / m)))
        .collect();
    let mut out = dsp::fftconvolve(ir, &hmin);
    out.truncate(ir.len());
    // energy 1 at 48 kHz; at another rate the same response spreads over more (or fewer) samples, so the energy
    // that gives the same level in the audible band is 48 kHz over the rate
    let e = energy(&out) * (fs / 48_000.0).sqrt();
    if e > 0.0 {
        for v in &mut out {
            *v /= e;
        }
    }
    out
}

/// The middle of the two microphones' responses, fitted, high-passed.
fn make(b: &Body, fs: f64) -> Vec<f64> {
    let (m0, m1) = (raw(b, SEEDS[0], fs), raw(b, SEEDS[1], fs));
    let mid: Vec<f64> = m0.iter().zip(&m1).map(|(a, c)| 0.5 * (a + c)).collect();
    let fitted = fit(&mid, &b.target, fs);
    dsp::sosfilt(&butter_sos(FilterKind::Hp, &[b.hp], 2, fs), &fitted)
}

/// The strings' force `x` through body `kind` at sample rate `fs`, as long as `x`.
pub fn apply(x: &[f64], kind: BodyKind, fs: f64) -> Vec<f64> {
    if x.is_empty() {
        return Vec::new();
    }
    let mut y = dsp::oaconvolve(x, &response(kind, fs));
    y.truncate(x.len());
    y
}

/// How readily the bridge moves at each of the frequencies `f` (0 to 1, the most of them 1, plus a small floor
/// rising with frequency): where the strings lose energy to the body. As the sketch, scaled to the largest of the
/// frequencies asked about (a note's own modes).
pub fn admittance(kind: BodyKind, f: &[f64]) -> Vec<f64> {
    let b = kind.body();
    let y: Vec<f64> = f
        .iter()
        .map(|&fk| {
            b.modes
                .iter()
                .map(|&(fm, q, g)| {
                    let r = fk / fm;
                    g * (r / q) / ((1.0 - r * r) * (1.0 - r * r) + (r / q) * (r / q))
                })
                .sum::<f64>()
        })
        .collect();
    let most = y.iter().copied().fold(0.0, f64::max);
    y.iter()
        .zip(f)
        .map(|(&v, &fk)| {
            let v = if most > 0.0 { v / most.max(1e-9) } else { v };
            v + b.floor_y * (fk / 1000.0).clamp(0.0, 4.0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SRF;

    /// The response's power over the third octave around each centre, dB relative to 1 kHz's.
    fn thirds(ir: &[f64], fs: f64) -> Vec<f64> {
        let nf = ir.len().next_power_of_two() * 2;
        let fft = Fft::new(nf);
        let (mut re, mut im) = (vec![0.0; nf], vec![0.0; nf]);
        re[..ir.len()].copy_from_slice(ir);
        fft.forward(&mut re, &mut im);
        let band = |c: f64| {
            let (lo, hi) = (c * dmath::pow(2.0, -1.0 / 6.0), c * dmath::pow(2.0, 1.0 / 6.0));
            let (a, b) = ((lo * nf as f64 / fs) as usize, (hi * nf as f64 / fs) as usize);
            let p = (a..b).map(|k| re[k] * re[k] + im[k] * im[k]).sum::<f64>() / (b - a).max(1) as f64;
            10.0 * dmath::log10(p.max(1e-30))
        };
        let at_1k = band(1000.0);
        CENTRES.iter().map(|&c| band(c) - at_1k).collect()
    }

    #[test]
    fn each_body_follows_its_target_and_is_made_once() {
        for kind in [BodyKind::Dreadnought, BodyKind::Soprano, BodyKind::PedalHarp] {
            let b = kind.body();
            let r = response(kind, SRF);
            assert!(Arc::ptr_eq(&r, &response(kind, SRF)), "{kind:?} made twice");
            assert_eq!(r.len(), (b.length * SRF) as usize);
            assert!(r.iter().all(|v| v.is_finite()));
            // over the body's own range (the high-pass and the last octave aside), its level aside: the shape
            // follows the target to about a decibel on average, as the sketch's own two microphones do; a gap
            // between two strong resonances (the soprano's at 630 Hz) stays a gap, as in the sketch
            let got = thirds(&r, SRF);
            let off: Vec<f64> = got
                .iter()
                .zip(&b.target)
                .zip(CENTRES)
                .filter(|(_, c)| *c >= 2.0 * b.hp && *c <= 6300.0)
                .map(|((g, t), _)| g - t)
                .collect();
            let mean = off.iter().sum::<f64>() / off.len() as f64;
            let mean_abs = off.iter().map(|o| (o - mean).abs()).sum::<f64>() / off.len() as f64;
            let most = off.iter().map(|o| (o - mean).abs()).fold(0.0, f64::max);
            assert!(
                mean_abs < 1.8 && most < 6.0,
                "{kind:?}: {mean_abs:.1} dB on average, {most:.1} at most"
            );
        }
    }

    #[test]
    fn bodies_work_at_other_rates() {
        for fs in [44_100.0, 96_000.0] {
            let r = response(BodyKind::Soprano, fs);
            assert_eq!(r.len(), (0.35 * fs) as usize);
            assert!(r.iter().all(|v| v.is_finite()));
            let got = thirds(&r, fs);
            let want = thirds(&response(BodyKind::Soprano, SRF), SRF);
            for (i, (g, w)) in got.iter().zip(&want).enumerate().skip(3).take(7) {
                assert!(
                    (g - w).abs() < 1.5,
                    "{fs} Hz at {} Hz: {g:.1} against {w:.1}",
                    CENTRES[i]
                );
            }
        }
    }

    #[test]
    fn the_bridge_moves_most_at_the_strong_resonances() {
        let f = [101.0, 150.0, 196.0, 3000.0];
        let y = admittance(BodyKind::Dreadnought, &f);
        assert!(y[2] > y[1] && y[0] > y[1], "{y:?}");
        assert!((y.iter().copied().fold(0.0, f64::max) - 1.0).abs() < 0.1, "{y:?}");
        // far over the resonances only the floor is left
        assert!((y[3] - 0.06).abs() < 0.01, "{y:?}");
    }
}
