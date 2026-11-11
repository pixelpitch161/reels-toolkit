//! `Shake`: a little camera shake on heavy hits.

use bf_core::dmath;
use bf_core::py::{py_max, py_min};

use super::Timeline;

/// `Shake`: each [`Shake::add`] is a decaying jolt of some screen pixels; the strongest one alive counts, held to
/// `cap`. The ring adds the offset to the camera's `cx` and `cy`.
#[derive(Clone, Debug)]
pub struct Shake {
    tl: Timeline<f64>,
    /// The largest offset in pixels.
    pub cap: f64,
    /// The time constant of a jolt's decay in seconds.
    pub decay: f64,
}

impl Shake {
    /// `Shake(cap, decay, span)`. The Python defaults are 2.5, 0.09 and 0.45.
    pub fn new(cap: f64, decay: f64, span: f64) -> Shake {
        Shake {
            tl: Timeline::new(span),
            cap,
            decay,
        }
    }

    /// `add(t0, px)`: a jolt of `px` pixels at `t0`.
    pub fn add(&mut self, t0: f64, px: f64) {
        self.tl.add(t0, px);
    }

    /// The number of jolts added.
    pub fn len(&self) -> usize {
        self.tl.len()
    }

    /// Whether no jolt was added.
    pub fn is_empty(&self) -> bool {
        self.tl.len() == 0
    }

    /// `offset(t)`: the camera's `(dx, dy)` at `t`.
    pub fn offset(&self, t: f64) -> (f64, f64) {
        let mut a = 0.0;
        for (t0, _, &px) in self.tl.alive(t) {
            a = py_max(a, px * dmath::exp(-(t - t0) / self.decay));
        }
        let a = py_min(a, self.cap);
        (
            a * dmath::sin(t * 2.0 * dmath::PI * 23.0),
            a * 0.7 * dmath::cos(t * 2.0 * dmath::PI * 29.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_jolt_no_shake_and_the_cap_holds() {
        let mut s = Shake::new(2.5, 0.09, 0.45);
        assert!(s.is_empty());
        assert_eq!(s.offset(1.0), (0.0, 0.0));
        s.add(1.0, 40.0);
        assert_eq!(s.len(), 1);
        assert_eq!(s.offset(0.999), (0.0, 0.0));
        let (dx, dy) = s.offset(1.01);
        assert!(dx.abs() <= 2.5 && dy.abs() <= 2.5 * 0.7 && dx != 0.0);
        // past the look-back the jolt is gone, whatever is left of it
        assert_eq!(s.offset(1.46), (0.0, 0.0));
    }
}
