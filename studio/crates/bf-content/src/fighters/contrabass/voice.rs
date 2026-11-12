//! CONTRABASS's instrument voice: `pp_bridge.voice('CONTRABASS', midi, vel, dur)`, and the three articulations its
//! part plays (`series/instrument-history/contrabass_lowpoly.py`, lines 324 to 407).
//!
//! - [`bowed`]: arco. Up to 59 harmonics (those up to 4.2 kHz) on a delayed vibrato, each weighted by the bass's
//!   body formants (`_form_gain`), with bow-hair noise (band-passed 300 to 2,200 Hz, a chuff at the start), an
//!   attack, a release and a dark one-pole filter.
//! - [`pizz`]: jazz pizzicato. Up to 39 slightly sharp partials (those up to 5 kHz), the high ones dying first, over
//!   a low-passed thump.
//! - [`slap`]: rockabilly slap. A band-passed click and a wooden clack over a bright pizz note; with no pitch, the
//!   unpitched ghost slap alone.
//!
//! Each draws from a generator of its own seeded by its pitch (`default_rng(int(f * 10) % 100003)` for the bow,
//! `int(f * 7)` for the thump, `int(f * 13)` or 4711 for the slap), so the shared noise stream is left where it was.
//! `pp_bridge`'s note is the module's `synth`: pizz below half a second, arco from there. Notes are made at 44,100 Hz
//! and resampled to 48,000 Hz.

use bf_core::dmath::{self, PI};
use bf_core::py;
use bf_core::rng::Generator;
use bf_sound::dsp::{self, FilterKind};
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::{PpNoise, Voice};

/// The CONTRABASS's register in `pp_bridge`'s roster (`('CONTRABASS', 'contrabass_lowpoly', 'ball', (28, 60), ...)`).
pub const REGISTER: (i32, i32) = (28, 60);

/// `FORM`: the body's formants, `(Hz, gain, log-width)`.
const FORM: [(f64, f64, f64); 5] = [
    (62.0, 0.9, 0.35),
    (112.0, 1.0, 0.35),
    (220.0, 0.85, 0.40),
    (500.0, 0.55, 0.45),
    (1100.0, 0.35, 0.45),
];

/// CONTRABASS's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct ContrabassVoice;

impl Voice for ContrabassVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "CONTRABASS"
    }

    /// A number that is finite: `int(n)` of a length raises on NaN and infinity, and a pizz or a slap rings for at
    /// least 0.85 s / 0.52 s, a bowed note (from 0.5 s) for more, so no finite length is too short.
    fn can_hold(&self, dur: f64) -> bool {
        dur.is_finite()
    }
}

/// `_midi_hz(m)`: `440.0 * 2 ** ((m - 69) / 12.0)`.
pub fn midi_hz(m: i32) -> f64 {
    440.0 * dmath::pow(2.0, f64::from(m - 69) / 12.0)
}

/// `synth(s, idx)` for the one note `pp_bridge.voice` asks: `bowed(f, dur, v) if dur >= .5 else pizz(f, dur, v)`, at
/// 44,100 Hz, before it is resampled. `midi` is a whole number.
///
/// # Panics
/// As [`bowed`] and [`pizz`].
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    let f = 440.0 * dmath::pow(2.0, (midi - 69.0) / 12.0);
    if dur >= 0.5 {
        bowed(f, dur, vel)
    } else {
        pizz(f, dur, vel, 1.0)
    }
}

/// `_form_gain(fk)`: `.12 + sum(a * math.exp(-(math.log(fk / fc) / w) ** 2) for fc, a, w in FORM)`.
pub fn form_gain(fk: f64) -> f64 {
    let mut s = 0.0;
    for (fc, a, w) in FORM {
        s += a * dmath::exp(-dmath::pow(dmath::ln(fk / fc) / w, 2.0));
    }
    0.12 + s
}

/// `np.minimum(1, x)`: 1 where `x` is larger, `x` otherwise (a NaN stays NaN).
fn min1(x: f64) -> f64 {
    if x.is_nan() || x < 1.0 { x } else { 1.0 }
}

/// `int(x) % 100003` of a positive pitch product: a generator's seed.
///
/// # Panics
/// When `x` is not a number an integer holds (Python raises there, or makes a seed no pitch of the kit reaches).
fn seed(x: f64) -> u64 {
    assert!(x.is_finite() && x.abs() < 9.0e18, "cannot make a seed of {x}");
    py::imod_py(x as i64, 100_003) as u64
}

/// `rng.standard_normal(n)`: `n` draws.
fn normals(rng: &mut Generator, n: usize) -> Vec<f64> {
    (0..n).map(|_| rng.standard_normal()).collect()
}

/// `bowed(f, dur, vel)` with its defaults (`att=.07, vib=.004, vrate=4.6, vdel=.22, noise=.06`): the arco note at
/// `f` Hz held `dur` seconds, `int((dur + .22) * SR)` samples at 44,100 Hz.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or the note would have a negative number of samples.
pub fn bowed(f: f64, dur: f64, vel: f64) -> Vec<f64> {
    let (att, vib, vrate, vdel, noise) = (0.07, 0.004, 4.6, 0.22, 0.06);
    let rel = 0.22;
    let n = pp::len(dur + rel);
    let t = pp::tt(n);
    // deterministic vibrato phase per pitch
    let ph0 = py::fmod_py(f * 5.31, 2.0 * PI);
    // fm = 1 + vib * np.clip((t - vdel) / .3, 0, 1) * np.sin(2 * np.pi * vrate * t + ph0)
    let w = 2.0 * PI * vrate;
    let ffm: Vec<f64> = t
        .iter()
        .map(|&t| {
            let fm = 1.0 + vib * bf_sound::synth::clip((t - vdel) / 0.3, 0.0, 1.0) * dmath::sin(w * t + ph0);
            f * fm
        })
        .collect();
    // ph = 2 * np.pi * np.cumsum(f * fm) / SR
    let ph: Vec<f64> = dsp::cumsum(&ffm).iter().map(|&c| 2.0 * PI * c / SRF).collect();
    let mut sig = vec![0.0; n];
    let mut tot = 0.0;
    for k in 1..60 {
        let kf = f64::from(k);
        let fk = f * kf;
        if fk > 4200.0 {
            break;
        }
        let low = if k == 1 && f < 60.0 { 0.45 } else { 1.0 };
        let gk = form_gain(fk) / dmath::pow(kf, 0.95) * low;
        let shift = 0.37 * kf;
        for (s, &ph) in sig.iter_mut().zip(&ph) {
            *s += gk * dmath::sin(kf * ph + shift);
        }
        tot += gk;
    }
    for s in &mut sig {
        *s /= tot;
    }
    // bow hair + rosin, deterministic per pitch
    let mut rng = Generator::new(seed(f * 10.0));
    let sos = dsp::butter_sos(FilterKind::Bp, &[300.0, 2200.0], 2, SRF);
    let nz = dsp::sosfilt(&sos, &normals(&mut rng, n));
    for ((s, &nz), &t) in sig.iter_mut().zip(&nz).zip(&t) {
        // chuff = np.exp(-t / .05) * 2.2: the hair catching the string
        let chuff = dmath::exp(-t / 0.05) * 2.2;
        *s += nz * noise * (0.7 + chuff);
    }
    let held = py::py_max(dur, 0.05);
    let env: Vec<f64> = t
        .iter()
        .map(|&t| {
            // np.minimum(1, t / att) ** 1.5 * np.where(t < dur, 1.0, np.exp(-(t - dur) / (rel / 4)))
            let tail = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / (rel / 4.0)) };
            let env = dmath::pow(min1(t / att), 1.5) * tail;
            env * (0.84 + 0.16 * min1(t / held))
        })
        .collect();
    // dark: the bass's heavy top filters the highs
    let sig = dsp::lfilter(&[0.22], &[1.0, -0.78], &sig);
    sig.iter().zip(&env).map(|(&s, &e)| s * e * vel * 1.6).collect()
}

/// `pizz(f, dur, vel, bright)`: the plucked note at `f` Hz, `int((max(dur, .35) + .5) * SR)` samples at 44,100 Hz.
///
/// # Panics
/// Where Python raises: `dur` is not a number.
pub fn pizz(f: f64, dur: f64, vel: f64, bright: f64) -> Vec<f64> {
    let ring = py::py_max(dur, 0.35) + 0.5;
    let n = pp::len(ring);
    let t = pp::tt(n);
    let mut sig = vec![0.0; n];
    let mut tot = 0.0;
    for k in 1..40 {
        let kf = f64::from(k);
        // a thick string: slightly sharp upper partials
        let fk = f * kf * (1.0 + 0.0007 * kf * kf);
        if fk > 5000.0 {
            break;
        }
        let low = if k == 1 && f < 60.0 { 0.55 } else { 1.0 };
        let gk = form_gain(fk) / dmath::pow(kf, 1.25 - 0.25 * bright) * low;
        let dec = 0.9 / (1.0 + 0.55 * (kf - 1.0)) * if f < 80.0 { 1.0 } else { 80.0 / f };
        let w = 2.0 * PI * fk;
        let shift = 0.5 * kf;
        for (s, &t) in sig.iter_mut().zip(&t) {
            *s += gk * dmath::sin(w * t + shift) * dmath::exp(-t / dec);
        }
        tot += gk;
    }
    for s in &mut sig {
        *s /= tot;
    }
    let mut rng = Generator::new(seed(f * 7.0));
    let sos = dsp::butter_sos(FilterKind::Lp, &[180.0], 2, SRF);
    let thump = dsp::sosfilt(&sos, &normals(&mut rng, n));
    for ((s, &th), &t) in sig.iter_mut().zip(&thump).zip(&t) {
        *s += th * dmath::exp(-t / 0.018) * 2.5;
    }
    let out = dsp::lfilter(&[0.5], &[1.0, -0.5], &sig);
    out.iter()
        .zip(&t)
        .map(|(&s, &t)| {
            let tail = if t < dur + 0.15 {
                1.0
            } else {
                dmath::exp(-(t - dur - 0.15) / 0.06)
            };
            let env = min1(t / 0.003) * tail;
            s * env * vel * 1.7
        })
        .collect()
}

/// `slap(f, dur, vel)`: a slap note at `f` Hz or, with `f` `None`, the unpitched ghost slap;
/// `int((max(dur, .12) + .4) * SR)` samples at 44,100 Hz.
///
/// # Panics
/// Where Python raises: `dur` is not a number.
pub fn slap(f: Option<f64>, dur: f64, vel: f64) -> Vec<f64> {
    let n = pp::len(py::py_max(dur, 0.12) + 0.4);
    let t = pp::tt(n);
    let mut rng = Generator::new(match f {
        None => 4711,
        Some(f) => seed(f * 13.0),
    });
    let click_sos = dsp::butter_sos(FilterKind::Bp, &[1800.0, 6500.0], 2, SRF);
    let click = dsp::sosfilt(&click_sos, &normals(&mut rng, n));
    let clack_sos = dsp::butter_sos(FilterKind::Bp, &[500.0, 1600.0], 2, SRF);
    let clack = dsp::sosfilt(&clack_sos, &normals(&mut rng, n));
    let mut out: Vec<f64> = (0..n)
        .map(|i| {
            let click = click[i] * dmath::exp(-t[i] / 0.006);
            let clack = clack[i] * dmath::exp(-t[i] / 0.014);
            click * 1.4 + clack * 1.1
        })
        .collect();
    if let Some(f) = f {
        // out[:len(x)] += x[:n] * .9
        let x = pizz(f, dur, 1.0, 1.6);
        for (o, &x) in out.iter_mut().zip(&x) {
            *o += x * 0.9;
        }
    }
    for o in &mut out {
        *o *= vel;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_lengths_are_the_modules() {
        assert_eq!(bowed(55.0, 1.0, 0.9).len(), pp::len(1.22));
        assert_eq!(pizz(55.0, 0.1, 0.9, 1.0).len(), pp::len(0.85));
        assert_eq!(pizz(55.0, 0.6, 0.9, 1.0).len(), pp::len(1.1));
        assert_eq!(slap(None, 0.08, 0.8).len(), pp::len(0.52));
        assert_eq!(slap(Some(55.0), 0.3, 1.0).len(), pp::len(0.7));
    }

    #[test]
    fn a_formant_gain_peaks_near_the_body() {
        assert!(form_gain(112.0) > form_gain(3000.0));
        assert!((form_gain(1e9) - 0.12).abs() < 1e-12);
    }

    #[test]
    fn the_bridge_note_is_pizz_below_half_a_second() {
        assert_eq!(synth(40.0, 0.8, 0.49), pizz(midi_hz(40), 0.49, 0.8, 1.0));
        assert_eq!(synth(40.0, 0.8, 0.5), bowed(midi_hz(40), 0.5, 0.8));
    }
}
