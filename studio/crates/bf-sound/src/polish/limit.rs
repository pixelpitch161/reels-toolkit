//! F2, second half: a limiter that looks ahead. In place of today's `tanh` squash, which bends every loud sample,
//! the gain comes down just before a peak (it sees 5 ms ahead), holds it under the ceiling, and lets go over
//! 80 ms. No sample ever passes the ceiling and none is clipped.

use std::collections::VecDeque;

use bf_core::dmath;

use crate::SRF;

/// How far the limiter looks ahead and how long it takes to let go, in seconds.
pub const LOOK: f64 = 0.005;
pub const RELEASE: f64 = 0.08;

/// The gain the limiter gives each sample of `x` to keep it at or under `ceiling`, or `None` when no sample
/// passes it (the limiter then leaves the sound exactly as it is).
///
/// Each sample's gain is what the largest peak within half the look-ahead on either side needs; it comes down at
/// once and goes back up over the release; then it is smoothed over the same half look-ahead, so it reaches a
/// peak's gain exactly at the peak, starts down a whole look-ahead before it, and never steps.
pub fn curve(x: &[[f64; 2]], ceiling: f64) -> Option<Vec<f64>> {
    let peak = |f: &[f64; 2]| f[0].abs().max(f[1].abs());
    if x.iter().all(|f| peak(f) <= ceiling) {
        return None;
    }
    let len = x.len();
    let half = ((LOOK * SRF) as usize / 2).max(2);
    // need[i]: the gain that holds the largest peak from i - half to i + half
    let mut need = vec![1.0; len];
    let mut near: VecDeque<usize> = VecDeque::new();
    let mut next = 0;
    for (i, slot) in need.iter_mut().enumerate() {
        while next < len && next <= i + half {
            while near.back().is_some_and(|&j| peak(&x[j]) <= peak(&x[next])) {
                near.pop_back();
            }
            near.push_back(next);
            next += 1;
        }
        while near.front().is_some_and(|&j| j + half < i) {
            near.pop_front();
        }
        let top = near.front().map_or(0.0, |&j| peak(&x[j]));
        *slot = (ceiling / (top + 1e-12)).min(1.0);
    }
    // down at once, back up slowly
    let back = dmath::exp(-1.0 / (RELEASE * SRF));
    let mut cur = 1.0f64;
    let held: Vec<f64> = need
        .iter()
        .map(|&n| {
            cur = if n < cur {
                n
            } else {
                (1.0 - (1.0 - cur) * back).min(1.0)
            };
            cur
        })
        .collect();
    // two running means, each a quarter of the look-ahead to either side: half of it together, so every gain
    // that goes into a sample's mean already holds that sample's peak
    let quarter = half / 2;
    let mean = |g: &[f64]| -> Vec<f64> {
        let at = |i: i64| g[i.clamp(0, len as i64 - 1) as usize];
        let q = quarter as i64;
        let mut run: f64 = (-q..=q).map(at).sum();
        (0..len as i64)
            .map(|i| {
                let out = run / (2 * q + 1) as f64;
                run += at(i + 1 + q) - at(i - q);
                out
            })
            .collect()
    };
    let smooth = mean(&mean(&held));
    // the running sums round, so the sample's own peak has the last word
    Some(
        smooth
            .iter()
            .zip(x)
            .map(|(&g, f)| g.min(ceiling / (peak(f) + 1e-12)).min(1.0))
            .collect(),
    )
}

/// `x` under `curve`.
pub fn apply(x: &mut [[f64; 2]], curve: &[f64]) {
    for (f, g) in x.iter_mut().zip(curve) {
        f[0] *= g;
        f[1] *= g;
    }
}

/// `x` held at or under `ceiling`.
pub fn limit(x: &mut [[f64; 2]], ceiling: f64) {
    if let Some(curve) = curve(x, ceiling) {
        apply(x, &curve);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn tone(level: f64, n: usize) -> Vec<[f64; 2]> {
        (0..n)
            .map(|i| {
                let v = level * dmath::sin(2.0 * PI * 440.0 * i as f64 / SRF);
                [v, -v]
            })
            .collect()
    }

    /// A quiet sound comes back sample for sample.
    #[test]
    fn a_quiet_sound_is_left_exactly_as_it_is() {
        let x = tone(0.5, 4_800);
        let mut y = x.clone();
        limit(&mut y, 0.89);
        assert_eq!(x, y);
        assert_eq!(curve(&x, 0.89), None);
        assert_eq!(curve(&[], 0.89), None);
    }

    /// A burst over the ceiling is held under it, the gain is already down when it starts, and the quiet sound
    /// well before and well after is all but untouched.
    #[test]
    fn a_peak_is_held_under_the_ceiling() {
        let mut x = tone(0.5, 48_000);
        for f in &mut x[20_000..22_000] {
            f[0] *= 3.0;
            f[1] *= 3.0;
        }
        let g = curve(&x, 0.89).expect("the burst passes the ceiling");
        let mut y = x.clone();
        apply(&mut y, &g);
        assert!(y.iter().all(|f| f[0].abs() <= 0.89 && f[1].abs() <= 0.89));
        assert!(g.iter().all(|&v| v > 0.0 && v <= 1.0));
        // looking ahead: the gain has started down before the burst
        assert!(g[19_990] < 0.999, "{}", g[19_990]);
        // no steps
        let step = g.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f64::max);
        assert!(step < 0.02, "the gain steps by {step}");
        // far from the burst the sound is as it was
        assert!(g[..15_000].iter().all(|&v| v > 0.999));
        assert!(g[46_000..].iter().all(|&v| v > 0.99));
    }
}
