//! F1: one loudness for every video. The finished mix is brought to -14 LUFS integrated (BS.1770, gated) with its
//! true peak at or under -1 dB: measure, set the gain, hold the peaks, measure again.

use crate::heard;
use crate::polish::limit;

/// The loudness every Composed soundtrack is brought to, in LUFS, how near it has to land, and the highest true
/// peak, in dB.
pub const TARGET: f64 = -14.0;
pub const WITHIN: f64 = 0.5;
pub const PEAK: f64 = -1.0;

/// How near the loudness is brought before the work stops, in LU (well inside [`WITHIN`]).
const NEAR: f64 = 0.2;

/// What [`to_target`] reached.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reached {
    /// The integrated loudness, in LUFS (-99 for silence).
    pub lufs: f64,
    /// The true peak, in dB.
    pub true_peak: f64,
}

/// The mix `x` brought to `lufs` with its true peak at or under `peak` dB, and each of `stems` under the same
/// gains. Silence is left alone.
pub fn to_target(x: &mut [[f64; 2]], stems: &mut [&mut Vec<[f64; 2]>], lufs: f64, peak: f64) -> Reached {
    let reached = |x: &[[f64; 2]]| Reached {
        lufs: heard::integrated(x),
        true_peak: heard::db(heard::true_peak(x)),
    };
    let mut now = reached(x);
    if now.lufs <= -70.0 {
        return now;
    }
    let mut all = |x: &mut [[f64; 2]], curve: Option<&[f64]>, g: f64| {
        let one = |y: &mut [[f64; 2]]| {
            for (i, f) in y.iter_mut().enumerate() {
                let k = g * curve.map_or(1.0, |c| c[i]);
                f[0] *= k;
                f[1] *= k;
            }
        };
        one(x);
        for stem in stems.iter_mut() {
            if curve.is_none_or(|c| c.len() == stem.len()) {
                one(stem);
            }
        }
    };
    // The sample peaks are held a little under the true-peak bar; the gap grows if a round shows it too small.
    let mut ceiling = heard::gain(peak - 0.3);
    for _ in 0..4 {
        let off = lufs - now.lufs;
        if off.abs() <= NEAR && now.true_peak <= peak {
            break;
        }
        if now.true_peak > peak && off.abs() <= NEAR {
            ceiling *= heard::gain(peak - now.true_peak - 0.05);
        }
        all(x, None, heard::gain(off));
        if let Some(curve) = limit::curve(x, ceiling) {
            all(x, Some(&curve), 1.0);
        }
        now = reached(x);
    }
    // Whatever the rounds left over the bar is taken off by a plain gain, which moves the true peak by exactly as
    // much (and the loudness by the same few hundredths of a dB).
    if now.true_peak > peak {
        all(x, None, heard::gain(peak - now.true_peak - 0.02));
        now = reached(x);
    }
    now
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SRF;
    use bf_core::dmath;
    use std::f64::consts::PI;

    /// A melody of tones with loud and soft bars, `level` at its loudest.
    fn song(level: f64) -> Vec<[f64; 2]> {
        let n = 8 * 48_000;
        (0..n)
            .map(|i| {
                let t = i as f64 / SRF;
                let bar = (t as usize) % 4;
                let hz = [220.0, 330.0, 440.0, 660.0][bar];
                let swell = [1.0, 0.5, 0.8, 0.3][bar];
                let v = level * swell * (dmath::sin(2.0 * PI * hz * t) + 0.5 * dmath::sin(2.0 * PI * 3.0 * hz * t));
                [v, 0.8 * v]
            })
            .collect()
    }

    /// A quiet and a far too loud mix both land within half a LU of -14 with the true peak at or under -1 dB, and
    /// the stem takes the same gains.
    #[test]
    fn every_mix_lands_on_the_target() {
        for level in [0.01, 0.2, 2.5] {
            let mut x = song(level);
            let mut stem = x.clone();
            let r = to_target(&mut x, &mut [&mut stem], TARGET, PEAK);
            assert!((r.lufs - TARGET).abs() <= WITHIN, "level {level}: {:.2} LUFS", r.lufs);
            assert!(
                r.true_peak <= PEAK + 1e-9,
                "level {level}: true peak {:.2} dB",
                r.true_peak
            );
            assert_eq!(r.lufs, heard::integrated(&x));
            assert_eq!(x, stem, "level {level}");
        }
    }

    /// Silence stays silence.
    #[test]
    fn silence_is_left_alone() {
        let mut x = vec![[0.0; 2]; 48_000];
        let r = to_target(&mut x, &mut [], TARGET, PEAK);
        assert_eq!(r.lufs, -99.0);
        assert!(x.iter().all(|f| f[0] == 0.0 && f[1] == 0.0));
    }
}
