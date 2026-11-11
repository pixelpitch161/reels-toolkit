//! The timing helpers of `reelkit/fx.py`: `env_ad`, `_smooth` and `FlashBudget`.

use std::cmp::Ordering;
use std::collections::VecDeque;

use bf_core::dmath;
use bf_core::py::{py_max, py_min, round_nd};

/// `env_ad(u, attack, decay, hold)`: 0 before `u = 0`, eases up over `attack`, holds for `hold`, then decays
/// exponentially with the time constant `decay`. The Python defaults are 0.03, 0.25 and 0.0.
pub fn env_ad(u: f64, attack: f64, decay: f64, hold: f64) -> f64 {
    if u < 0.0 {
        return 0.0;
    }
    if u < attack {
        let x = u / attack;
        return x * x * (3.0 - 2.0 * x);
    }
    let u2 = u - attack - hold;
    if u2 < 0.0 { 1.0 } else { dmath::exp(-u2 / decay) }
}

/// `_smooth(x)`: the smooth step of `x` held to 0..1.
pub(crate) fn smooth(x: f64) -> f64 {
    let x = py_min(1.0, py_max(0.0, x));
    x * x * (3.0 - 2.0 * x)
}

/// `FlashBudget`: which bright events may stay bright. Greedy in time order: an event is bright only if fewer
/// than `max_per_s` bright ones started in the `window` seconds before it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlashBudget {
    /// `max_per_s`: the bright events allowed in one window. Python's default is 3.
    pub max: usize,
    /// The window in seconds. Python's default is 1.0.
    pub window: f64,
}

impl FlashBudget {
    /// `FlashBudget(max_per_s, window)`.
    pub fn new(max_per_s: usize, window: f64) -> FlashBudget {
        FlashBudget { max: max_per_s, window }
    }

    /// `assign(times, priority)`: for every candidate, whether it stays bright. The candidates are taken in the
    /// order of `(round(time, 4), -priority, index)`; `None` is a priority of 0 for all.
    ///
    /// # Panics
    /// If `priority` is given and is not as long as `times`.
    pub fn assign(&self, times: &[f64], priority: Option<&[f64]>) -> Vec<bool> {
        let n = times.len();
        if let Some(p) = priority {
            assert_eq!(p.len(), n, "assign: {} priorities for {n} times", p.len());
        }
        let pr = |i: usize| priority.map_or(0.0, |p| p[i]);
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| {
            round_nd(times[a], 4)
                .partial_cmp(&round_nd(times[b], 4))
                .unwrap_or(Ordering::Equal)
                .then((-pr(a)).partial_cmp(&-pr(b)).unwrap_or(Ordering::Equal))
                .then(a.cmp(&b))
        });
        let mut ok = vec![false; n];
        let mut acc: VecDeque<f64> = VecDeque::new();
        for i in order {
            let t = times[i];
            while acc.front().is_some_and(|&first| first <= t - self.window) {
                acc.pop_front();
            }
            if acc.len() < self.max {
                ok[i] = true;
                acc.push_back(t);
            }
        }
        ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_ad_rises_holds_and_decays() {
        assert_eq!(env_ad(-0.001, 0.03, 0.25, 0.0), 0.0);
        assert_eq!(env_ad(0.0, 0.03, 0.25, 0.0), 0.0);
        assert_eq!(env_ad(0.015, 0.03, 0.25, 0.0), 0.5);
        assert_eq!(env_ad(0.03, 0.03, 0.25, 0.0), 1.0);
        assert_eq!(env_ad(0.1, 0.03, 0.25, 0.2), 1.0);
        assert!((env_ad(0.28, 0.03, 0.25, 0.0) - 0.36787944117144233).abs() < 1e-15);
        assert_eq!(smooth(-1.0), 0.0);
        assert_eq!(smooth(0.5), 0.5);
        assert_eq!(smooth(7.0), 1.0);
    }

    #[test]
    fn the_budget_lets_three_through_a_second() {
        let b = FlashBudget::new(3, 1.0);
        assert_eq!(
            b.assign(&[0.0, 0.1, 0.2, 0.3, 1.0, 1.05], None),
            [true, true, true, false, true, false]
        );
        // equal times: the higher priority first, then the order given
        assert_eq!(
            FlashBudget::new(1, 1.0).assign(&[2.0, 2.0, 2.0], Some(&[1.0, 3.0, 3.0])),
            [false, true, false]
        );
        assert!(b.assign(&[], None).is_empty());
    }
}
