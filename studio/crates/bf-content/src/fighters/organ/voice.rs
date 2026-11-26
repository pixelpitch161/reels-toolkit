//! ORGAN's voice: `pp_bridge.voice('ORGAN', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/organ60_lowpoly.py`'s `synth` on the scene its
//! roster selects for the ORGAN: Saint-Sulpice (key `sulp`), the Cavaille-Coll reeds. That is `v_sulp`: the
//! additive pipe voice `organ` with four ranks (a reed at pitch, one an octave down two cents sharp, one an octave
//! up two cents flat, a principal at pitch), each a stack of harmonics up to `HMAX` (0.42 of the sample rate), a
//! 10 ms attack, a 0.1 s release after the held length, and a "chiff" of band-passed noise over the first 60 ms;
//! then `tanh(1.3 x)`. The note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! The chiff's noise is the note's own: `np.random.default_rng(seed)` with `seed = idx + int(s['t0'] * 10)`, which
//! for the one event `pp_bridge` makes (index 0) on the scene at 28.0 s is 280, every note. All `n` normals are
//! drawn and the first 60 ms used. The series' shared noise stream is not touched.
//!
//! The module keeps a cache of notes by `(scene t0, event index)`, `_NOTE`. `pp_bridge` empties it before every
//! note (with its one made-up event every call would otherwise return the first note), so every note is made
//! afresh: the port has no cache. The module's filter cache `_bp_cache` is keyed by the rounded band edges, which
//! differ between any two whole MIDI notes of the register, so it never hands a note another note's filter.
//!
//! MIDI note 1 is the module's marker for the Wurlitzer's car horn (`HORN`): `synth` plays `v_horn` for it in
//! any scene, so `pp_bridge.voice('ORGAN', 1, ..)` does too, and so does this. No part plays it and no reference
//! vector holds it.
//!
//! `organ`'s branches for `vib`, `drift`, `wind` and `trem` are not ported: `v_sulp` leaves all four off.

use bf_core::dmath::{self, PI};
use bf_core::py::{py_max, py_min};
use bf_core::rng::Generator;
use bf_sound::dsp;
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::real::{Correction, Fill, Rework};
use bf_sound::voice::{PpNoise, Voice};

/// The ORGAN's register in `pp_bridge`'s roster (`('ORGAN', 'organ60_lowpoly', 'sulp', (36, 84), ...)`).
pub const REGISTER: (i32, i32) = (36, 84);

/// `HMAX = SR * .42`: no harmonic above this.
const HMAX: f64 = SRF * 0.42;
/// `HORN = 1`: the marker pitch of the Wurlitzer's car horn.
const HORN: f64 = 1.0;
/// `REED`: a reed rank's harmonics.
const REED: [f64; 20] = [
    1.0, 0.95, 0.86, 0.78, 0.7, 0.63, 0.56, 0.5, 0.45, 0.4, 0.36, 0.32, 0.28, 0.25, 0.22, 0.2, 0.18, 0.16, 0.14, 0.12,
];
/// `PRIN`: a principal rank's harmonics.
const PRIN: [f64; 12] = [1.0, 0.55, 0.32, 0.2, 0.14, 0.1, 0.07, 0.05, 0.035, 0.025, 0.018, 0.012];
/// `v_sulp`'s attack, in seconds.
const ATT: f64 = 0.01;
/// `v_sulp`'s release, in seconds.
const REL: f64 = 0.1;
/// `v_sulp`'s chiff level.
const CHIFF: f64 = 0.1;
/// `idx + int(s['t0'] * 10)` for `pp_bridge`'s one event (index 0) on the Saint-Sulpice scene (`t0 = 28.0`).
pub const SEED: u64 = 280;

/// A rank of `organ`: `(spectrum, pitch multiple, level, detune in cents)`.
type Rank = (&'static [f64], f64, f64, f64);

/// `v_sulp`'s ranks: trompette, bombarde, clairon, and a principal.
const SULP: [Rank; 4] = [
    (&REED, 1.0, 1.0, 0.0),
    (&REED, 0.5, 0.55, 2.0),
    (&REED, 2.0, 0.45, -2.0),
    (&PRIN, 1.0, 0.35, 0.0),
];

/// The ORGAN's added overtones: the harmonics its own note barely has, laid in at a real church organ's level (dB
/// under the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (36, [-99.0, 11.5, 6.9, 7.3, -99.0, -3.1, -99.0, 0.9]),
    (37, [-99.0, -99.0, 1.6, 5.1, -99.0, -0.6, -99.0, -99.0]),
    (38, [-99.0, -99.0, 1.1, 3.7, -99.0, -0.9, -99.0, -99.0]),
    (39, [-99.0, -99.0, 0.8, -99.0, -99.0, -1.2, -99.0, -99.0]),
    (40, [-99.0, -99.0, 3.8, 5.4, -99.0, -99.0, -99.0, -99.0]),
    (41, [-99.0, -99.0, 2.9, 3.8, -99.0, -99.0, -99.0, -99.0]),
    (42, [-99.0, -99.0, 1.9, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (43, [-99.0, 11.7, 1.9, 13.4, -99.0, 2.3, -99.0, -99.0]),
    (44, [-99.0, 11.4, 0.9, 12.6, -99.0, 1.9, -99.0, -99.0]),
    (45, [-99.0, 11.3, -0.0, 11.9, -99.0, 1.5, -99.0, -99.0]),
    (46, [-99.0, -99.0, 2.8, -99.0, -99.0, -2.4, -99.0, -99.0]),
    (47, [-99.0, -99.0, 3.1, -99.0, -99.0, -3.3, -99.0, -99.0]),
    (48, [-99.0, -99.0, 3.4, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (49, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (50, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (51, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (52, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (53, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (54, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (55, [-99.0, -99.0, -99.0, 7.2, -99.0, -99.0, -99.0, -99.0]),
    (56, [-99.0, -99.0, -99.0, 7.3, -99.0, -99.0, -99.0, -99.0]),
    (57, [-99.0, -99.0, -99.0, 7.2, -99.0, -99.0, -99.0, -99.0]),
    (58, [-99.0, -99.0, -99.0, 9.8, -99.0, -99.0, -99.0, 6.3]),
    (59, [-99.0, 9.1, -99.0, 10.8, -99.0, -99.0, -99.0, 7.5]),
    (60, [-99.0, 11.6, -99.0, 11.9, -99.0, -99.0, -99.0, 9.1]),
    (61, [-99.0, -99.0, 6.3, 4.8, -99.0, -99.0, -99.0, 1.1]),
    (62, [-99.0, -99.0, 5.2, 4.2, -99.0, -99.0, -99.0, 0.0]),
    (63, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.0]),
    (64, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.9]),
    (65, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (66, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (67, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (68, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (69, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (70, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (71, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (72, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (73, [-99.0, -99.0, -99.0, -99.0, -0.4, 8.4, -99.0, -99.0]),
    (74, [-99.0, -99.0, -99.0, -99.0, -0.4, 8.0, -99.0, -99.0]),
    (75, [-99.0, -99.0, -99.0, -99.0, -0.3, 7.6, -99.0, -99.0]),
    (76, [-99.0, 13.2, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (77, [-99.0, 13.7, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (78, [-99.0, 14.1, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (79, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (80, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (81, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (82, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (83, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (84, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
];

/// The ORGAN's overtone correction toward a real church organ: gains in dB for its first eight harmonics, note by
/// note (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "ORGAN"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (36, [1.1, 0.0, 1.0, 1.4, 4.6, 0.2, -13.8, 1.3]),
    (37, [-7.3, 8.8, -6.6, -5.0, -1.9, -4.5, -20.8, 10.5]),
    (38, [-7.2, 7.6, -7.0, -5.3, -3.8, -5.3, -20.9, 7.2]),
    (39, [-7.6, 6.0, -8.1, 7.4, -6.6, -6.6, -21.5, 3.3]),
    (40, [-7.9, 9.1, -6.8, -3.9, -15.9, 7.5, -22.8, 11.6]),
    (41, [-8.2, 7.8, -6.9, -3.3, -19.1, 6.9, -23.3, 11.9]),
    (42, [-8.0, 7.2, -6.9, 11.3, -22.3, 6.7, -23.1, 12.4]),
    (43, [-2.2, -2.3, -1.6, 0.0, -11.6, 1.2, -18.6, 10.4]),
    (44, [-2.0, -2.2, -1.6, 0.2, -11.6, 1.4, -18.9, 10.2]),
    (45, [-1.3, -1.5, -1.0, 0.8, -10.7, 2.0, -18.5, 10.7]),
    (46, [-3.9, 8.4, -4.6, 6.5, -18.3, -1.1, -28.8, 0.5]),
    (47, [-3.2, 8.9, -3.9, 7.6, -18.9, -0.1, -29.3, 0.7]),
    (48, [-2.4, 9.7, -3.1, 8.9, -20.3, 16.5, -29.4, 1.2]),
    (49, [0.0, 5.1, 10.2, 10.2, -17.2, 6.9, -18.3, 6.0]),
    (50, [0.0, 5.5, 10.9, 10.5, -17.6, 6.0, -17.9, 6.6]),
    (51, [0.0, 5.8, 11.4, 10.9, -18.0, 5.1, -17.4, 7.0]),
    (52, [-3.5, 6.3, 7.5, -3.8, -24.9, 6.2, -22.5, -6.0]),
    (53, [-4.1, 6.4, 7.0, -3.1, -25.5, 4.2, -21.2, -5.8]),
    (54, [-4.7, 6.6, 6.4, -2.1, -26.1, 2.6, -19.7, -4.9]),
    (55, [-0.8, 11.8, 6.4, 0.0, 8.9, 7.2, -9.9, 12.1]),
    (56, [-0.7, 11.7, 6.1, 0.0, 9.0, 6.5, -8.4, 11.8]),
    (57, [-0.6, 11.3, 5.4, 0.0, 9.0, 5.6, -6.6, 11.3]),
    (58, [0.7, 15.6, 11.1, 0.0, 10.4, 13.0, -1.0, 0.4]),
    (59, [0.9, -0.3, 12.1, 0.0, 11.6, 13.7, 1.9, 0.4]),
    (60, [1.2, 0.3, 12.7, 0.1, 13.4, 15.2, 4.9, 0.8]),
    (61, [1.4, 9.6, 0.0, 0.9, 4.9, -4.1, 1.1, 1.6]),
    (62, [1.6, 8.7, 0.0, 0.9, 4.5, -3.2, 1.1, 1.9]),
    (63, [-5.1, 0.3, 7.6, 8.1, -3.4, -8.9, -6.6, -4.7]),
    (64, [-6.4, 7.3, -6.1, 4.3, -4.4, 4.7, -5.7, -6.1]),
    (65, [-6.7, 7.3, -6.6, 2.1, -3.4, 5.1, -8.5, 8.7]),
    (66, [-6.1, 7.2, -6.6, 0.2, -2.4, 5.3, -10.6, 8.4]),
    (67, [-2.3, 6.7, 2.5, 5.9, -5.1, 3.4, -29.9, 14.1]),
    (68, [-2.6, 6.8, 2.5, 6.1, -2.9, 6.3, -30.0, 15.1]),
    (69, [-3.6, 4.4, 0.6, 3.8, -4.2, 3.3, -30.0, 10.2]),
    (70, [0.0, 6.4, -8.9, 0.4, -0.1, 3.8, -20.6, -1.0]),
    (71, [0.0, 6.3, -10.3, 0.3, 1.8, 4.0, -21.2, -2.3]),
    (72, [0.0, 6.4, -11.6, 0.4, 3.5, 3.6, -21.1, -3.1]),
    (73, [-0.1, 7.8, 5.6, 1.3, 0.8, 0.0, -7.7, -0.5]),
    (74, [0.0, 8.5, 5.0, -0.5, 1.0, 0.0, -7.7, -1.8]),
    (75, [0.1, 8.9, 4.3, -1.1, 1.3, 0.0, -7.4, -1.5]),
    (76, [0.9, 0.0, 4.8, 10.1, 10.5, -2.8, -15.0, -0.8]),
    (77, [1.1, 0.0, 6.9, 11.7, 9.2, -2.9, -13.3, -0.1]),
    (78, [1.2, 0.0, 9.6, 13.3, 8.1, -3.1, -12.4, 0.5]),
    (79, [-9.3, 1.6, -1.4, -0.8, -8.7, -2.0, -23.9, -7.1]),
    (80, [-8.6, 0.4, -1.3, 0.4, -9.7, -1.6, -24.8, -6.7]),
    (81, [-6.8, 0.0, 0.3, 2.0, -9.3, 0.5, -25.2, -5.1]),
    (82, [-13.9, 0.0, -4.6, -3.8, -13.2, -9.8, -30.0, -11.4]),
    (83, [-12.0, 0.0, -2.1, -2.5, -11.3, -8.2, -29.7, -9.5]),
    (84, [-10.1, 0.0, 0.1, -1.8, -9.3, -6.6, -28.5, -8.0]),
];

/// The Composed arrangement's ORGAN (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added, and a slower start (80 ms to full level), nearer the real pipes' (which
/// take a fifth of a second and would blur fast lines).
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.08,
    stop: 0.0,
    keep_shape: false,
};

/// The ORGAN's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct OrganVoice;

impl Voice for OrganVoice {
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
        "ORGAN"
    }

    /// Down to -0.53 s: the note is `int((dur + 5 * rel + .03) * SR)` samples, and numpy refuses a negative
    /// count. (The car horn, MIDI note 1, is `int((dur + .1) * SR)` samples: held for less than -0.1 s it
    /// panics here where Python raises.)
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 5.0 * REL + 0.03)
    }
}

/// `bpf(x, lo, hi)`: `x` through `butter(2, [lo / (SR / 2), hi / (SR / 2)], "band")` with `lfilter`, the edges
/// held to 20 Hz and 0.45 of the sample rate.
fn bpf(x: &[f64], lo: f64, hi: f64) -> Vec<f64> {
    // lo, hi = max(20., lo), min(SR * .45, hi)
    pp::bp(x, py_max(20.0, lo), py_min(SRF * 0.45, hi))
}

/// `organ(f, dur, v, ranks, att, rel, chiff, seed=seed)`: the additive pipe-organ voice, at 44,100 Hz.
#[allow(clippy::too_many_arguments)] // the Python signature
fn organ(f: f64, dur: f64, v: f64, ranks: &[Rank], att: f64, rel: f64, chiff: f64, seed: u64) -> Vec<f64> {
    let n = pp::len(dur + 5.0 * rel + 0.03);
    let t = pp::tt(n);
    // ph = 2 * np.pi * np.cumsum(np.full(n, f)) / SR
    let ph: Vec<f64> = dsp::cumsum(&vec![f; n]).iter().map(|&c| 2.0 * PI * c / SRF).collect();
    let mut x = vec![0.0; n];
    for &(spec, mult, amp, cents) in ranks {
        // d = mult * 2 ** (cents / 1200)
        let d = mult * dmath::pow(2.0, cents / 1200.0);
        for (i, &a) in spec.iter().enumerate() {
            let k = (i + 1) as f64;
            if k * d * f > HMAX {
                break;
            }
            // x += amp * a * np.sin(k * d * ph + .7 * k)
            let (level, turn, shift) = (amp * a, k * d, 0.7 * k);
            for (o, &p) in x.iter_mut().zip(&ph) {
                *o += level * dmath::sin(turn * p + shift);
            }
        }
    }
    // e = np.where(t < att, .5 - .5 * np.cos(np.pi * t / att), 1.) * np.where(t < dur, 1., np.exp(-(t - dur) / rel))
    for (o, &t) in x.iter_mut().zip(&t) {
        let rise = if t < att {
            0.5 - 0.5 * dmath::cos(PI * t / att)
        } else {
            1.0
        };
        let hold = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / rel) };
        *o *= rise * hold;
    }
    let mut rng = Generator::new(seed);
    if chiff != 0.0 {
        // nz = rng.normal(0, 1, n); k = int(.06 * SR)
        let mut nz = vec![0.0; n];
        rng.fill_normal(0.0, 1.0, &mut nz);
        let k = pp::len(0.06).min(n);
        // c = bpf(nz[:k], min(2.5 * f, 7000), min(5.5 * f, 12000)) * np.exp(-t[:k] / .014);  x[:k] += chiff * c * 3
        let c = bpf(&nz[..k], py_min(2.5 * f, 7000.0), py_min(5.5 * f, 12000.0));
        for i in 0..k {
            x[i] += chiff * (c[i] * dmath::exp(-t[i] / 0.014)) * 3.0;
        }
    }
    for o in &mut x {
        *o *= v;
    }
    x
}

/// `v_sulp(f, dur, v, seed)`: Cavaille-Coll reeds: trompette, bombarde, clairon.
fn v_sulp(f: f64, dur: f64, v: f64, seed: u64) -> Vec<f64> {
    organ(f, dur, v, &SULP, ATT, REL, CHIFF, seed)
        .iter()
        .map(|&s| dmath::np_tanh(1.3 * s))
        .collect()
}

/// `v_horn(dur, v)`: the electric car horn, "ah-OO-ga".
fn v_horn(dur: f64, v: f64) -> Vec<f64> {
    let n = pp::len(dur + 0.1);
    let t = pp::tt(n);
    // fr = np.where(t < .1, 330 + 900 * t, np.where(t < .32, 420, 420 - 140 * np.clip((t - .32) / .12, 0, 1)))
    let fr: Vec<f64> = t
        .iter()
        .map(|&t| {
            if t < 0.1 {
                330.0 + 900.0 * t
            } else if t < 0.32 {
                420.0
            } else {
                420.0 - 140.0 * ((t - 0.32) / 0.12).clamp(0.0, 1.0)
            }
        })
        .collect();
    let ph: Vec<f64> = dsp::cumsum(&fr).iter().map(|&c| 2.0 * PI * c / SRF).collect();
    // x = np.sign(np.sin(ph)) * .6 + .4 * np.sin(2 * ph) + .3 * np.sin(3 * ph)
    let x: Vec<f64> = ph
        .iter()
        .map(|&p| {
            let s = dmath::sin(p);
            let sign = if s > 0.0 {
                1.0
            } else if s < 0.0 {
                -1.0
            } else {
                s
            };
            sign * 0.6 + 0.4 * dmath::sin(2.0 * p) + 0.3 * dmath::sin(3.0 * p)
        })
        .collect();
    let band = bpf(&x, 380.0, 2600.0);
    (0..n)
        .map(|i| {
            let t = t[i];
            // bpf(x, 380, 2600) * np.minimum(1, t / .01) * np.where(t < dur, 1., np.exp(-(t - dur) / .03))
            let rise = if t / 0.01 < 1.0 { t / 0.01 } else { 1.0 };
            let hold = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.03) };
            let mut s = band[i] * rise * hold;
            // x *= np.where((t > .3) & (t < .33), .4, 1.)
            s *= if t > 0.3 && t < 0.33 { 0.4 } else { 1.0 };
            dmath::np_tanh(2.2 * s) * v
        })
        .collect()
}

/// `organ60_lowpoly.synth` for the one note `pp_bridge.voice` asks of the Saint-Sulpice scene: the note at 44,100
/// Hz, before it is resampled. `midi` is a whole number.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or so far below zero that the note has a negative number of
/// samples.
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    if midi == HORN {
        v_horn(dur, vel)
    } else {
        v_sulp(pp::midi_hz(midi), dur, vel, SEED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two detunes are `2 ** (±2 / 1200)`: constants of the kit. These are Python's own bits (CPython 3.13 on
    /// the reference PC, `(2 ** (2 / 1200)).hex()`).
    #[test]
    fn the_detunes_are_pythons() {
        assert_eq!(dmath::pow(2.0, 2.0 / 1200.0).to_bits(), 0x3FF0_04BC_0FFE_7799);
        assert_eq!(dmath::pow(2.0, -2.0 / 1200.0).to_bits(), 0x3FEF_F68A_AC83_F92D);
        assert_eq!(dmath::pow(2.0, 0.0 / 1200.0), 1.0);
    }

    #[test]
    fn a_note_has_its_length_its_pitch_and_its_release() {
        // int((0.25 + 0.5 + .03) * 44100) samples, then 160 / 147 as many at 48 kHz
        let raw = synth(60.0, 0.8, 0.25);
        assert_eq!(raw.len(), 34_398);
        let note = OrganVoice.note(&mut PpNoise::start(), 60.0, 0.8, 0.25);
        assert_eq!(note.len(), 37_440);
        assert_eq!(note, pp::to_48k(&raw));
        // middle C: at least its 65 periods' upward zero crossings in a quarter of a second (the reeds' harmonics
        // add more)
        let ups = raw[441..11_025]
            .windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count();
        assert!((60..=400).contains(&ups), "{ups}");
        // tanh holds it inside 1; it dies away after the held length
        assert!(raw.iter().all(|s| s.abs() < 1.0));
        let end = raw[raw.len() - 441..].iter().fold(0.0f64, |p, s| p.max(s.abs()));
        assert!(end < 0.05, "{end}");
    }

    #[test]
    fn the_voice_leaves_the_shared_noise_alone_and_repeats() {
        let (mut a, mut b) = (PpNoise::start(), PpNoise::start());
        let first = OrganVoice.note(&mut a, 48.0, 0.9, 0.12);
        assert_eq!(a.nz(4), b.nz(4), "the shared stream is where it was");
        // every note is made afresh from the same seed: the same note twice is the same samples
        assert_eq!(first, OrganVoice.note(&mut a, 48.0, 0.9, 0.12));
        assert_ne!(first, OrganVoice.note(&mut a, 50.0, 0.9, 0.12));
        assert_eq!((OrganVoice.register(), OrganVoice.name()), (REGISTER, "ORGAN"));
    }

    #[test]
    fn high_ranks_stop_below_the_harmonic_limit() {
        // at C6 the reed an octave up has 8 harmonics below 18,522 Hz, the reed an octave down all 20
        let f = pp::midi_hz(84.0);
        let count = |mult: f64, spec: &[f64]| (1..=spec.len()).take_while(|&k| k as f64 * mult * f <= HMAX).count();
        assert_eq!((count(2.0, &REED), count(1.0, &REED), count(0.5, &REED)), (8, 17, 20));
    }

    #[test]
    fn what_it_can_hold_is_what_python_does_not_raise_for() {
        assert!(OrganVoice.can_hold(0.0) && OrganVoice.can_hold(-0.1) && OrganVoice.can_hold(-0.53));
        assert!(!OrganVoice.can_hold(-0.54) && !OrganVoice.can_hold(f64::NAN));
        assert!(synth(60.0, 0.8, -0.53).is_empty());
        // a note of no held length is its attack and release
        assert_eq!(synth(60.0, 0.8, 0.0).len(), 23_373);
        // the horn is a tenth of a second longer than it is held
        let horn = synth(1.0, 0.8, 0.5);
        assert_eq!(horn.len(), pp::len(0.6));
        assert!(horn.iter().all(|s| s.is_finite() && s.abs() <= 0.8));
    }
}
