//! A piano's soundboard: what turns the strings' push on the bridge into sound (`docs/studio/voice-engine-2-
//! instruments.md`, "Piano"). The strings' force goes straight out (the board as a whole) and through many small
//! resonances of the board, each about as damped as a real soundboard's (the low ones ringing longest), then through
//! the board's average response: a little less low end, less in the upper middle (where the strike point lifts the
//! 2nd to 4th overtones) and a little more up top. Fitted in the listening rounds so the riff's octave bands and the
//! high notes' overtones sit inside four real pianos' (the sketch `wg.py`, Aedan's pick P16).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use bf_core::dmath;
use bf_core::rng::Generator;

use super::eq::{self, Band};
use crate::dsp::{self, FilterKind, butter_sos};

/// How many resonances the board has.
const MODES: usize = 1400;
/// The lowest and highest, Hz.
const LOWEST: f64 = 45.0;
const HIGHEST: f64 = 10_000.0;
/// How long the board's response is, seconds.
const LENGTH: f64 = 0.5;
/// The straight part and the resonances' part (the resonances' energy is 1).
const DIRECT: f64 = 1.0;
const MODAL: f64 = 0.45;
/// The resonances' seed (numpy's `default_rng(7)` in the sketch: the same board).
const SEED: u64 = 7;
/// The board's average response.
pub const BODY: [Band; 3] = [
    Band::Low {
        hz: 200.0,
        db: -2.5,
        q: 0.7,
    },
    Band::Peak {
        hz: 1500.0,
        db: -3.0,
        q: 1.0,
    },
    Band::High {
        hz: 3000.0,
        db: 4.0,
        q: 0.7,
    },
];
/// Under this the board gives nothing back, Hz.
const FLOOR_HZ: f64 = 30.0;

/// The board's response at sample rate `fs`, made once per rate: the straight part and the resonances, through
/// [`BODY`] and the floor's high-pass.
pub fn response(fs: f64) -> Arc<Vec<f64>> {
    static MADE: OnceLock<Mutex<HashMap<u64, Arc<Vec<f64>>>>> = OnceLock::new();
    let made = MADE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(r) = made.lock().map_or(None, |m| m.get(&fs.to_bits()).cloned()) {
        return r;
    }
    let r = Arc::new(make(fs));
    if let Ok(mut m) = made.lock() {
        m.entry(fs.to_bits()).or_insert_with(|| Arc::clone(&r));
    }
    r
}

/// Each resonance a damped sine, summed, as the sketch's two microphones heard it (each its own share of each
/// resonance, the draws in the sketch's order); the board is the middle of the two, what both speakers share
/// (the voice's own width spreads it again).
fn make(fs: f64) -> Vec<f64> {
    let len = (LENGTH * fs) as usize;
    let mut rng = Generator::new(SEED);
    let (lo, hi) = (dmath::ln(LOWEST), dmath::ln(HIGHEST));
    let f: Vec<f64> = (0..MODES).map(|_| dmath::exp(rng.uniform(lo, hi))).collect();
    let tau: Vec<f64> = f
        .iter()
        .map(|&f| (30.0 + 30.0 * (200.0 / f).sqrt()) / (std::f64::consts::PI * f))
        .collect();
    let a0: Vec<f64> = tau.iter().map(|t| rng.standard_normal() / t.sqrt()).collect();
    let ph0: Vec<f64> = (0..MODES).map(|_| rng.uniform(0.0, dmath::TAU)).collect();
    let mut mid = vec![0.0; len];
    for _ in 0..2 {
        let a: Vec<f64> = a0.iter().map(|a| a * (1.0 + 0.3 * rng.standard_normal())).collect();
        let ph: Vec<f64> = ph0.iter().map(|p| p + 0.5 * rng.standard_normal()).collect();
        let mut ir = vec![0.0; len];
        for i in 0..MODES {
            if f[i] >= 0.5 * fs {
                continue;
            }
            // a sin(2 pi f t + ph) e^(-t / tau), by turning a phasor one sample at a time
            let r = dmath::exp(-1.0 / (tau[i] * fs));
            let w = dmath::TAU * f[i] / fs;
            let (sr, si) = (r * dmath::cos(w), r * dmath::sin(w));
            let (mut zr, mut zi) = (a[i] * dmath::cos(ph[i]), a[i] * dmath::sin(ph[i]));
            for v in ir.iter_mut() {
                *v += zi;
                (zr, zi) = (zr * sr - zi * si, zr * si + zi * sr);
            }
        }
        let energy = ir.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-30);
        for (m, v) in mid.iter_mut().zip(&ir) {
            *m += 0.5 * MODAL * v / energy;
        }
    }
    if let Some(v) = mid.first_mut() {
        *v += DIRECT;
    }
    let shaped = eq::apply(&mid, &BODY, fs);
    dsp::sosfilt(&butter_sos(FilterKind::Hp, &[FLOOR_HZ], 2, fs), &shaped)
}

/// The strings' force `x` through the board at sample rate `fs`, as long as `x`.
pub fn apply(x: &[f64], fs: f64) -> Vec<f64> {
    if x.is_empty() {
        return Vec::new();
    }
    let mut y = dsp::oaconvolve(x, &response(fs));
    y.truncate(x.len());
    y
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SRF;

    #[test]
    fn the_board_is_made_once_and_is_its_own() {
        let a = response(SRF);
        let b = response(SRF);
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(a.len(), 24_000);
        assert!(a.iter().all(|v| v.is_finite()));
        // the straight part leads, the resonances ring on under it and die away
        let e = |r: std::ops::Range<usize>| a[r].iter().map(|v| v * v).sum::<f64>();
        assert!(a[0].abs() > 0.5);
        assert!(e(1..2400) > 10.0 * e(19_200..24_000));
    }

    #[test]
    fn it_keeps_the_length_and_passes_a_click_as_its_response() {
        let mut x = vec![0.0; 1000];
        x[0] = 1.0;
        let y = apply(&x, SRF);
        assert_eq!(y.len(), 1000);
        let r = response(SRF);
        assert!(y.iter().zip(r.iter()).all(|(a, b)| (a - b).abs() < 1e-9));
        assert!(apply(&[], SRF).is_empty());
    }
}
