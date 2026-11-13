//! SYNTH's voice: `pp_bridge.voice('SYNTH', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/synth60_lowpoly.py`'s `synth` on its first scene
//! (the roster's scene key is `None`): `modern`, the KORG MINILOGUE XD. That is `v_modern`: two band-limited saws
//! 0.4 % apart and a square an octave down (`bl`), through the module's state-variable low-pass (`svf_lp`, its cutoff
//! falling from `600 + 4200 vel` Hz), an ADSR (3 ms, 0.16 s, 0.5, 0.1 s), then `echo` (two repeats a dotted 8th apart
//! at the module's 116 BPM, 0.3 and 0.09), times the velocity. The note is made at 44,100 Hz and resampled to
//! 48,000 Hz. It draws no noise (the series' shared stream is not touched), and `pp_bridge` empties the module's
//! note cache before each note, so the port has none.

use bf_core::dmath::{self, PI};
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::{PpNoise, Voice};

/// SYNTH's register in `pp_bridge`'s roster (`('SYNTH', 'synth60_lowpoly', None, (57, 81), ...)`).
pub const REGISTER: (i32, i32) = (57, 81);
/// `BEAT_S = 60 / BPM` at the module's 116 BPM.
const BEAT_S: f64 = 60.0 / 116.0;

/// `bl(ph, fmax, kind, kmax=48)`: a band-limited oscillator from its harmonics (`saw`: every harmonic over its
/// number, x 0.6; `sq`: the odd ones, x 0.8).
fn bl(ph: &[f64], fmax: f64, square: bool) -> Vec<f64> {
    // K = int(max(1, min(kmax, (SR * .42) // max(fmax, 20.))))
    let fm = if fmax > 20.0 { fmax } else { 20.0 };
    let q = bf_core::py::floor_div(SRF * 0.42, fm);
    let kk = if q < 48.0 { q } else { 48.0 };
    let kk = if kk > 1.0 { kk } else { 1.0 } as usize;
    let mut x = vec![0.0; ph.len()];
    let step = if square { 2 } else { 1 };
    for k in (1..=kk).step_by(step) {
        let kf = k as f64;
        for (xi, &p) in x.iter_mut().zip(ph) {
            *xi += dmath::sin(kf * p) / kf;
        }
    }
    let g = if square { 0.8 } else { 0.6 };
    x.iter().map(|v| v * g).collect()
}

/// `svf_lp(x, fc, q)`: the module's 2-pole state-variable low-pass (Simper TPT) with a cutoff per sample.
fn svf_lp(x: &[f64], fc: &[f64], q: f64) -> Vec<f64> {
    let k = 1.0 / q;
    let mut out = vec![0.0; x.len()];
    let (mut ic1, mut ic2) = (0.0, 0.0);
    for i in 0..x.len() {
        // gg = np.tan(np.pi * np.clip(fc, 20, SR * .45) / SR)
        let gg = dmath::tan(PI * fc[i].clamp(20.0, SRF * 0.45) / SRF);
        let a1 = 1.0 / (1.0 + gg * (gg + k));
        let a2 = gg * a1;
        let a3 = gg * a2;
        let v3 = x[i] - ic2;
        let v1 = a1 * ic1 + a2 * v3;
        let v2 = ic2 + a2 * ic1 + a3 * v3;
        ic1 = 2.0 * v1 - ic1;
        ic2 = 2.0 * v2 - ic2;
        out[i] = v2;
    }
    out
}

/// `adsr(t, dur, a, d, s, r)` of the module.
fn adsr(t: f64, dur: f64, a: f64, d: f64, s: f64, r: f64) -> f64 {
    let e = if t < a { t / a } else { s + (1.0 - s) * dmath::exp(-(t - a) / d) };
    e * if t < dur { 1.0 } else { dmath::exp(-(t - dur) / r) }
}

/// `v_modern(f, dur, v)`.
fn v_modern(f: f64, dur: f64, v: f64) -> Vec<f64> {
    let n = pp::len(dur + 0.35);
    let t = pp::tt(n);
    let w = 2.0 * PI * f;
    let ph: Vec<f64> = t.iter().map(|&t| w * t).collect();
    let a: Vec<f64> = ph.iter().map(|p| p * 1.004).collect();
    let b: Vec<f64> = ph.iter().map(|p| p * 0.996).collect();
    let c: Vec<f64> = ph.iter().map(|p| p * 0.5).collect();
    let (xa, xb, xc) = (bl(&a, f, false), bl(&b, f, false), bl(&c, f * 0.5, true));
    let x: Vec<f64> = (0..n).map(|i| 0.5 * xa[i] + 0.5 * xb[i] + 0.3 * xc[i]).collect();
    let fc: Vec<f64> = t.iter().map(|&t| 600.0 + 4200.0 * v * dmath::exp(-t / 0.13)).collect();
    let y: Vec<f64> = svf_lp(&x, &fc, 1.3)
        .iter()
        .zip(&t)
        .map(|(y, &t)| y * adsr(t, dur, 0.003, 0.16, 0.5, 0.1))
        .collect();
    echo(&y, 0.75 * BEAT_S, 0.3, 2).iter().map(|e| e * v).collect()
}

/// `echo(x, sec, fb, n)`: `n` repeats `sec` apart, the k-th at `fb ** k`.
fn echo(x: &[f64], sec: f64, fb: f64, n: usize) -> Vec<f64> {
    let d = (sec * SRF) as usize;
    let mut out = x.to_vec();
    out.resize(x.len() + d * n, 0.0);
    for k in 1..=n {
        let g = dmath::pow(fb, k as f64);
        for (i, v) in x.iter().enumerate() {
            out[d * k + i] += v * g;
        }
    }
    out
}

/// SYNTH's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct SynthVoice;

impl Voice for SynthVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&v_modern(pp::midi_hz(midi), dur, vel))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "SYNTH"
    }

    /// Down to -0.35 s: the note is `int((dur + .35) * SR)` samples.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.35)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_is_its_length_and_draws_no_noise() {
        let mut noise = PpNoise::start();
        let mut before = noise.clone();
        let x = SynthVoice.note(&mut noise, 69.0, 0.8, 0.12);
        assert_eq!(noise.0.random(), before.0.random());
        let raw = v_modern(440.0, 0.12, 0.8);
        assert_eq!(raw.len(), pp::len(0.47) + 2 * (0.75 * BEAT_S * SRF) as usize);
        assert_eq!(x, pp::to_48k(&raw));
        assert!(x.iter().all(|v| v.is_finite()) && x.iter().any(|v| v.abs() > 0.01));
    }
}
