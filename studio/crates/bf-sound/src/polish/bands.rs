//! F2, first half: three ranges, each evened out by itself. Today one compressor's gain is shared by everything,
//! so a tuba note pulls the piccolo down with it. Here the mix is split at about 200 Hz and 3 kHz, each range is
//! gently compressed against its own level, and the three are summed again.
//!
//! The split is clean: the low and the high range are cut out with filters run forward and backward (no shift in
//! time), and the middle is what is left, so the three always add up to the mix. The stems go through the same
//! split and take the gains the mix's ranges got, so they still add up to it.

use bf_core::dmath;

use crate::SRF;
use crate::dsp::{self, FilterKind, butter_sos};

/// Where the ranges part, in Hz.
pub const SPLIT: (f64, f64) = (200.0, 3000.0);

/// One range's compressor: its ratio, attack and release in seconds, and its threshold against the range's
/// median level while it sounds, in dB.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Comp {
    pub ratio: f64,
    pub attack: f64,
    pub release: f64,
    pub threshold: f64,
}

/// The three ranges' compressors: lows, middle, highs.
pub const COMPS: [Comp; 3] = [
    Comp {
        ratio: 3.0,
        attack: 0.02,
        release: 0.18,
        threshold: -2.0,
    },
    Comp {
        ratio: 1.6,
        attack: 0.015,
        release: 0.2,
        threshold: 0.0,
    },
    Comp {
        ratio: 2.2,
        attack: 0.005,
        release: 0.12,
        threshold: -1.0,
    },
];

/// `x` through a filter forward and then backward: no shift in time, twice the slope.
fn both_ways(kind: FilterKind, hz: f64, x: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let sos = butter_sos(kind, &[hz], 2, SRF);
    let mut y = dsp::sosfilt_stereo(&sos, x);
    y.reverse();
    let mut y = dsp::sosfilt_stereo(&sos, &y);
    y.reverse();
    y
}

/// `x` in its three ranges: lows, middle, highs. They add up to `x`.
pub fn split(x: &[[f64; 2]]) -> [Vec<[f64; 2]>; 3] {
    if x.is_empty() {
        return [Vec::new(), Vec::new(), Vec::new()];
    }
    let lo = both_ways(FilterKind::Lp, SPLIT.0, x);
    let hi = both_ways(FilterKind::Hp, SPLIT.1, x);
    let mid = (0..x.len())
        .map(|i| [x[i][0] - lo[i][0] - hi[i][0], x[i][1] - lo[i][1] - hi[i][1]])
        .collect();
    [lo, mid, hi]
}

/// The gain a compressor gives each sample of `x` (1 where it leaves it alone): the level is the larger of a
/// fast and a slow follower (attack and release), and above the threshold the gain is
/// `(threshold / level) ^ (1 - 1 / ratio)`.
pub fn gains(x: &[[f64; 2]], comp: Comp) -> Vec<f64> {
    let power: Vec<f64> = x.iter().map(|f| (f[0] * f[0] + f[1] * f[1]) / 2.0).collect();
    let follow = |seconds: f64| -> Vec<f64> {
        let a = dmath::exp(-1.0 / (seconds * SRF));
        dsp::lfilter(&[1.0 - a], &[1.0, -a], &power)
            .iter()
            .map(|&p| (p.max(0.0) + 1e-14).sqrt())
            .collect()
    };
    let (fast, slow) = (follow(comp.attack), follow(comp.release));
    let env: Vec<f64> = fast.iter().zip(&slow).map(|(&f, &s)| f.max(s)).collect();
    let top = env.iter().copied().fold(0.0, f64::max);
    let on: Vec<f64> = env.iter().copied().filter(|&e| e > top * 0.02).collect();
    if on.is_empty() {
        return vec![1.0; x.len()];
    }
    let thr = dsp::percentile(&on, 50.0) * dmath::pow(10.0, comp.threshold / 20.0);
    let slope = 1.0 - 1.0 / comp.ratio;
    env.iter()
        .map(|&e| if e > thr { dmath::pow(thr / e, slope) } else { 1.0 })
        .collect()
}

/// The three ranges of `x` under `gains` (one curve a range), summed.
fn apply(x: &[[f64; 2]], gains: &[Vec<f64>; 3]) -> Vec<[f64; 2]> {
    let parts = split(x);
    (0..x.len())
        .map(|i| {
            let mut f = [0.0; 2];
            for (part, g) in parts.iter().zip(gains) {
                f[0] += part[i][0] * g[i];
                f[1] += part[i][1] * g[i];
            }
            f
        })
        .collect()
}

/// The mix `x` with each range compressed by itself, and each of `stems` under the same gains.
pub fn compress<'a>(x: &mut Vec<[f64; 2]>, stems: impl Iterator<Item = &'a mut Vec<[f64; 2]>>) {
    if x.is_empty() {
        return;
    }
    let parts = split(x);
    let curves = [0, 1, 2].map(|k| gains(&parts[k], COMPS[k]));
    *x = (0..x.len())
        .map(|i| {
            let mut f = [0.0; 2];
            for (part, g) in parts.iter().zip(&curves) {
                f[0] += part[i][0] * g[i];
                f[1] += part[i][1] * g[i];
            }
            f
        })
        .collect();
    drop(parts);
    for stem in stems {
        if stem.len() == x.len() {
            *stem = apply(stem, &curves);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn tone(hz: f64, level: f64, n: usize) -> Vec<[f64; 2]> {
        (0..n)
            .map(|i| {
                let v = level * dmath::sin(2.0 * PI * hz * i as f64 / SRF);
                [v, v]
            })
            .collect()
    }

    fn rms(x: &[[f64; 2]]) -> f64 {
        (x.iter().map(|f| f[0] * f[0] + f[1] * f[1]).sum::<f64>() / (2 * x.len()) as f64).sqrt()
    }

    /// The three ranges add up to the sound, and each tone lands in its own.
    #[test]
    fn the_ranges_add_up_and_part_the_sound() {
        let n = 48_000;
        let (low, mid, high) = (tone(60.0, 0.3, n), tone(800.0, 0.3, n), tone(6000.0, 0.3, n));
        let x: Vec<[f64; 2]> = (0..n).map(|i| [low[i][0] + mid[i][0] + high[i][0]; 2]).collect();
        let parts = split(&x);
        for i in (0..n).step_by(101) {
            let sum = parts[0][i][0] + parts[1][i][0] + parts[2][i][0];
            assert!((sum - x[i][0]).abs() < 1e-12, "at {i}");
        }
        // away from the ends each range holds its tone and little else
        let inner = 4_800..n - 4_800;
        for (part, of) in parts.iter().zip([&low, &mid, &high]) {
            let rest: Vec<[f64; 2]> = inner.clone().map(|i| [part[i][0] - of[i][0]; 2]).collect();
            assert!(
                rms(&rest) < 0.1 * rms(&of[inner.clone()]),
                "{} of {}",
                rms(&rest),
                rms(&of[inner.clone()])
            );
        }
        assert!(split(&[])[0].is_empty());
    }

    /// A loud low note does not pull a high tone down: the high range keeps its own gain (F2's point).
    #[test]
    fn a_low_note_leaves_the_highs_alone() {
        let n = 96_000;
        let high = tone(5000.0, 0.05, n);
        // the low note comes in for the second second, 20 dB over the high tone
        let low: Vec<[f64; 2]> = tone(60.0, 0.5, n)
            .iter()
            .enumerate()
            .map(|(i, f)| if i >= n / 2 { *f } else { [0.0; 2] })
            .collect();
        let mut x: Vec<[f64; 2]> = (0..n).map(|i| [high[i][0] + low[i][0]; 2]).collect();
        let mut stems = [high.clone(), low.clone()];
        compress(&mut x, stems.iter_mut());
        let high_part = |x: &[[f64; 2]], from: usize, to: usize| rms(&split(&x[from..to])[2][2_400..to - from - 2_400]);
        let before = high_part(&x, 4_800, n / 2 - 4_800);
        let under = high_part(&x, n / 2 + 9_600, n - 4_800);
        let moved = 20.0 * dmath::log10(under / before);
        assert!(
            moved.abs() < 1.0,
            "the high tone moved by {moved:.2} dB under the low note"
        );
        // the stems still add up to the mix
        for i in (0..n).step_by(211) {
            assert!((stems[0][i][0] + stems[1][i][0] - x[i][0]).abs() < 1e-9, "at {i}");
        }
        // and the low note itself was held back
        assert!(rms(&stems[1][n / 2 + 9_600..]) < rms(&low[n / 2 + 9_600..]));
    }
}
