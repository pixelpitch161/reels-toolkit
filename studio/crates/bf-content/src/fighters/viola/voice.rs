//! VIOLA's voice: `pp_bridge.voice('VIOLA', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/viola_lowpoly.py`'s `synth` on the scene its roster
//! selects for the VIOLA (`('VIOLA', 'viola_lowpoly', 'ball', (48, 79), 0, 1.12)`): the scene does not change the
//! sound, `synth` is `bowed(_midi_hz(m), dur, v)`, ported here. The note is made at 44,100 Hz and resampled to
//! 48,000 Hz.
//!
//! `bowed` draws its bow-hair noise from a generator of its own, seeded by the pitch
//! (`np.random.default_rng(int(f * 10) % 100003)`), and its vibrato phase from the pitch too: it draws nothing from
//! the series' shared noise stream or from any module's generator, so a VIOLA note is the same whenever it is made.

use bf_core::dmath::{self, PI};
use bf_core::py::{fmod_py, py_max};
use bf_core::rng::Generator;
use bf_sound::dsp::{self, FilterKind};
use bf_sound::voice::pp;
use bf_sound::voice::real::{Correction, Fill, Rework};
use bf_sound::voice::{PpNoise, Voice};

/// The VIOLA's register in `pp_bridge`'s roster. `kit.music.inst_gain` measures the voice on the middle of it.
pub const REGISTER: (i32, i32) = (48, 79);

/// `FORM`: the body's formants, `(centre Hz, gain, width in natural-log units)`.
pub const FORM: [(f64, f64, f64); 3] = [(300.0, 0.7, 0.45), (620.0, 1.0, 0.40), (1650.0, 0.55, 0.40)];

/// `bowed`'s release in seconds (`rel = .16`).
const REL: f64 = 0.16;

/// `np.clip(x, 0, 1)`: numpy's rule (a NaN stays NaN).
fn clip01(x: f64) -> f64 {
    let x = if x < 0.0 { 0.0 } else { x };
    if x > 1.0 { 1.0 } else { x }
}

/// `np.minimum(1, x)`: 1 where `x` is larger, `x` otherwise (a NaN stays NaN).
fn min1(x: f64) -> f64 {
    if x.is_nan() || x < 1.0 { x } else { 1.0 }
}

/// `viola_lowpoly.bowed(f, dur, vel)` with its default tone, at 44,100 Hz:
///
/// ```python
/// rel = .16
/// n = int((dur + rel) * SR); t = np.arange(n) / SR
/// ph0 = (f * 7.13) % (2 * math.pi)
/// fm = 1 + vib * np.clip((t - vdel) / .25, 0, 1) * np.sin(2 * np.pi * vrate * t + ph0)
/// ph = 2 * np.pi * np.cumsum(f * fm) / SR
/// sig = np.zeros(n); tot = 0.0
/// for k in range(1, 40):
///     fk = f * k
///     if fk > 5000: break
///     gk = (.22 + sum(a * math.exp(-(math.log(fk / fc) / w) ** 2) for fc, a, w in FORM)) / k ** tilt
///     sig += gk * np.sin(k * ph); tot += gk
/// sig /= tot
/// rng = np.random.default_rng(int(f * 10) % 100003)
/// nz = sosfilt(butter(2, [700, 2600], 'band', fs=SR, output='sos'), rng.standard_normal(n))
/// sig += nz * noise * (.6 + .8 * np.exp(-t / .06))
/// env = np.minimum(1, t / att) ** 1.3 * np.where(t < dur, 1.0, np.exp(-(t - dur) / (rel / 4)))
/// env = env * (.86 + .14 * np.minimum(1, t / max(dur, .05)))
/// sig = lfilter([.30], [1, -.70], sig)
/// return sig * env * vel
/// ```
///
/// with `att=.045, vib=.0055, vrate=5.2, vdel=.14, tilt=1.12, noise=.035`.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or so far below -0.16 s that the note has no samples (numpy refuses
/// a negative count, scipy's sosfilt an empty note).
pub fn bowed(f: f64, dur: f64, vel: f64) -> Vec<f64> {
    let (att, vib, vrate, vdel, tilt, noise) = (0.045, 0.0055, 5.2, 0.14, 1.12, 0.035);
    let n = pp::len(dur + REL);
    let t = pp::tt(n);
    let ph0 = fmod_py(f * 7.13, 2.0 * PI);
    let w = 2.0 * PI * vrate;
    let ffm: Vec<f64> = t
        .iter()
        .map(|&t| f * (1.0 + vib * clip01((t - vdel) / 0.25) * dmath::sin(w * t + ph0)))
        .collect();
    let ph: Vec<f64> = dsp::cumsum(&ffm).iter().map(|&c| 2.0 * PI * c / pp::SRF).collect();
    let mut sig = vec![0.0; n];
    let mut tot = 0.0;
    for k in 1..40 {
        let kf = f64::from(k);
        let fk = f * kf;
        if fk > 5000.0 {
            break;
        }
        // sum(...) starts from the int 0, which adds nothing to the first term
        let mut s = 0.0;
        for &(fc, a, wd) in &FORM {
            s += a * dmath::exp(-dmath::pow(dmath::ln(fk / fc) / wd, 2.0));
        }
        let gk = (0.22 + s) / dmath::pow(kf, tilt);
        for (v, &x) in sig.iter_mut().zip(&ph) {
            *v += gk * dmath::sin(kf * x);
        }
        tot += gk;
    }
    for v in &mut sig {
        *v /= tot;
    }
    // int(f * 10) % 100003: a positive pitch, so Python's modulo is the remainder
    let seed = ((f * 10.0) as u64) % 100_003;
    let mut rng = Generator::new(seed);
    let raw: Vec<f64> = (0..n).map(|_| rng.standard_normal()).collect();
    let sos = dsp::butter_sos(FilterKind::Bp, &[700.0, 2600.0], 2, pp::SRF);
    let nz = dsp::sosfilt(&sos, &raw);
    for ((v, &z), &t) in sig.iter_mut().zip(&nz).zip(&t) {
        *v += z * noise * (0.6 + 0.8 * dmath::exp(-t / 0.06));
    }
    let hold = py_max(dur, 0.05);
    let sig = dsp::lfilter(&[0.30], &[1.0, -0.70], &sig);
    sig.iter()
        .zip(&t)
        .map(|(&s, &t)| {
            let tail = if t < dur {
                1.0
            } else {
                dmath::exp(-(t - dur) / (REL / 4.0))
            };
            let env = dmath::pow(min1(t / att), 1.3) * tail;
            let env = env * (0.86 + 0.14 * min1(t / hold));
            s * env * vel
        })
        .collect()
}

/// `viola_lowpoly.synth(s, 0)` for the one note `pp_bridge.voice` asks of the ball scene, at 44,100 Hz before it is
/// resampled: `bowed(_midi_hz(m), dur, v)`.
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    bowed(pp::midi_hz(midi), dur, vel)
}

/// The VIOLA's added overtones: the harmonics its own note barely has, laid in at a real viola's level (dB under
/// the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (48, [-99.0, -99.0, 27.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (49, [-99.0, -99.0, 26.9, -99.0, 12.3, -99.0, -99.0, -99.0]),
    (50, [-99.0, -99.0, 27.0, -99.0, 12.7, -99.0, -99.0, -99.0]),
    (51, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (52, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (53, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (54, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (55, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (56, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (57, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (58, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (59, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (60, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (61, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (62, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (63, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (64, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (65, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (66, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -12.8, -99.0]),
    (67, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -12.6, -24.1]),
    (68, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -13.1, -24.2]),
    (69, [-99.0, -99.0, -99.0, -99.0, -99.0, -8.4, -19.4, -12.2]),
    (70, [-99.0, -99.0, -99.0, -99.0, -12.2, -8.8, -18.6, -12.2]),
    (71, [-99.0, -99.0, -99.0, -99.0, -11.6, -9.0, -18.2, -12.2]),
    (72, [-99.0, -99.0, -99.0, -1.2, -6.3, -9.2, -17.7, -13.9]),
    (73, [-99.0, -99.0, -99.0, -3.5, -6.3, -11.5, -19.4, -16.1]),
    (74, [-99.0, -99.0, -99.0, -4.5, -6.4, -12.3, -19.8, -17.3]),
    (75, [-99.0, -99.0, -99.0, -13.4, -12.9, -18.8, -99.0, -99.0]),
    (76, [-99.0, -99.0, -99.0, -13.7, -10.1, -17.1, -99.0, -34.4]),
    (77, [-99.0, 2.5, -99.0, -13.7, -8.3, -15.7, -99.0, -35.1]),
    (78, [-99.0, 3.1, 0.9, -3.5, -5.0, -11.6, -22.2, -24.5]),
    (79, [-99.0, -99.0, 1.4, -2.5, -4.4, -11.2, -22.4, -23.7]),
];

/// The VIOLA's overtone correction toward a real viola: gains in dB for its first eight harmonics, note by note
/// (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "VIOLA"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (48, [-0.1, 14.5, 0.0, 7.5, 14.3, 9.1, 13.8, 12.9]),
    (49, [-0.3, 14.6, 0.0, 9.3, 0.6, 12.1, 14.2, 13.9]),
    (50, [-0.2, 15.0, 0.0, 10.7, 0.4, 14.0, 14.8, 13.7]),
    (51, [-8.8, -4.5, 2.6, -3.9, -8.2, -14.9, -1.9, 5.3]),
    (52, [-9.3, -4.2, 2.3, -2.9, -6.8, -12.7, -0.7, 4.6]),
    (53, [-10.3, -4.0, 1.9, -1.7, -4.8, -11.2, -0.6, 4.0]),
    (54, [-6.3, 0.0, -7.5, -6.9, -1.4, 6.0, -1.5, 3.1]),
    (55, [-7.2, 1.0, -7.2, -5.7, -0.5, 6.0, -0.8, 3.4]),
    (56, [-8.2, 1.8, -7.0, -4.5, -0.2, 5.5, -1.0, 2.3]),
    (57, [0.0, -8.2, -7.9, -8.9, -0.4, 1.4, 4.1, 3.3]),
    (58, [0.0, -7.7, -5.1, -7.8, 1.2, 2.0, 6.7, 4.4]),
    (59, [0.0, -7.2, -2.8, -7.1, 1.7, 2.3, 7.3, 4.4]),
    (60, [0.0, 4.6, 6.2, -8.7, 9.6, 4.9, 12.2, 8.7]),
    (61, [0.0, 4.7, 7.8, -8.7, 8.3, 4.1, 12.1, 10.9]),
    (62, [0.0, 4.6, 9.1, -9.0, 7.6, 4.3, 12.1, 11.2]),
    (63, [0.0, -2.0, 2.9, 9.6, 9.5, 7.7, 8.9, 13.0]),
    (64, [0.0, -0.6, 5.6, 8.8, 10.0, 8.7, 12.1, 14.8]),
    (65, [0.0, 1.3, 7.4, 7.6, 10.9, 9.6, 13.5, 14.6]),
    (66, [0.0, -7.0, 11.1, 12.4, 4.6, 10.1, 2.1, 13.3]),
    (67, [0.0, -7.0, 10.7, 12.3, 5.0, 11.2, 1.8, 3.8]),
    (68, [0.0, -5.8, 10.7, 12.4, 6.4, 12.5, 1.8, 4.0]),
    (69, [-1.8, 9.9, 9.1, 8.0, 11.5, 1.4, 0.1, 2.4]),
    (70, [-1.9, 11.5, 9.0, 9.5, 2.1, 3.0, 1.4, 3.1]),
    (71, [-2.1, 12.9, 9.2, 11.2, 1.2, 2.2, 1.3, 3.5]),
    (72, [0.0, 8.8, 9.1, 1.5, 2.6, 3.8, 3.3, 4.6]),
    (73, [0.0, 7.7, 9.6, 2.1, 4.3, 5.1, 4.0, 5.9]),
    (74, [0.0, 7.3, 11.6, 3.7, 4.0, 4.9, 4.7, 6.1]),
    (75, [0.0, 14.5, 9.6, 1.9, 4.6, 6.1, 7.3, 14.0]),
    (76, [-2.0, 13.1, 8.0, -0.1, 2.2, 3.6, 6.5, 3.7]),
    (77, [0.1, 0.0, 12.9, 2.5, 4.2, 5.6, 10.9, 5.7]),
    (78, [-0.2, 0.0, 1.6, 2.5, 3.7, 4.6, 4.5, 4.9]),
    (79, [-3.4, 10.1, -1.4, 0.2, 0.8, 2.0, 2.0, 2.3]),
];

/// The Composed arrangement's bowed VIOLA (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added, and a slower start (80 ms to full level), nearer the real bow's swell
/// (which takes a third of a second and would blur fast lines). Its plucks stay its own.
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.08,
    stop: 0.0,
    keep_shape: false,
};

/// The VIOLA's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct ViolaVoice;

impl Voice for ViolaVoice {
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
        "VIOLA"
    }

    /// Down to -0.16 s: the tone is `int((dur + .16) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + REL) && pp::len(dur + REL) > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_draws_nothing_from_the_shared_noise() {
        let mut noise = PpNoise::start();
        let note = ViolaVoice.note(&mut noise, 55.0, 0.85, 0.25);
        assert_eq!(note, pp::to_48k(&bowed(pp::midi_hz(55.0), 0.25, 0.85)));
        let mut fresh = PpNoise::start();
        assert_eq!(noise.nz(4), fresh.nz(4));
        assert_eq!(noise.1.violin.random().to_bits(), fresh.1.violin.random().to_bits());
        assert_eq!(noise.1.alto.random().to_bits(), fresh.1.alto.random().to_bits());
        // int((dur + .16) * 44100) samples
        assert_eq!(bowed(220.0, 0.25, 0.8).len(), pp::len(0.25 + 0.16));
        assert_eq!(pp::len(0.5 + 0.16), 29106);
        assert!(ViolaVoice.can_hold(-0.15) && !ViolaVoice.can_hold(-0.2) && !ViolaVoice.can_hold(f64::NAN));
        assert_eq!((ViolaVoice.register(), ViolaVoice.name()), (REGISTER, "VIOLA"));
    }

    #[test]
    fn the_first_sample_is_silent_and_the_note_is_the_same_every_time() {
        let x = bowed(130.8, 0.3, 0.9);
        assert_eq!(x[0], 0.0);
        assert!(x.iter().all(|v| v.is_finite()));
        assert_eq!(x, bowed(130.8, 0.3, 0.9));
        // the harmonics stop at 5 kHz: a high note has few, and still sounds
        let hi = bowed(pp::midi_hz(79.0), 0.2, 0.8);
        assert!(hi.iter().any(|v| v.abs() > 1e-3));
    }
}
