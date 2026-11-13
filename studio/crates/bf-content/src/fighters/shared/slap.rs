//! `series/instrument-history/bassclarinet_lowpoly.py` `slap`: the slap-tongue pop, a hollow pitched thock on a
//! click. The BASS CLARINET's part plays it for its SLAP TONGUE and the EUPHONIUM's for a copied one, both as
//! `pp_bridge._resample(slap(_midi_hz(m), vel)) * 0.9` (that wrapper and its note cache are each part's own).
//!
//! The click is `default_rng(7).standard_normal(n)`, a fresh generator every call, so a slap draws from none of
//! the shared noise.

use bf_core::dmath::{self, PI};
use bf_core::rng::Generator;
use bf_sound::voice::pp;

/// `slap`'s default `dur`: the pop's length before its 80 ms tail, in seconds.
pub const DUR: f64 = 0.09;

/// `slap(f, v, dur=.09)`: `int((dur + .08) * SR)` samples at 44,100 Hz, the pop at `f` Hz and velocity `v`.
///
/// # Panics
/// Where Python raises: `dur` is not a number or gives a negative number of samples.
pub fn slap(f: f64, v: f64, dur: f64) -> Vec<f64> {
    let n = pp::len(dur + 0.08);
    let t = pp::tt(n);
    let mut rng = Generator::new(7);
    let (w1, w3, w5, w180) = (2.0 * PI * f, 2.0 * PI * 3.0 * f, 2.0 * PI * 5.0 * f, 2.0 * PI * 180.0);
    t.iter()
        .map(|&t| {
            // click = rng.standard_normal(n) * exp(-t / .0025)
            let click = rng.standard_normal() * dmath::exp(-t / 0.0025);
            // body = sin(2 pi f t) + .5 sin(2 pi 3 f t) + .22 sin(2 pi 5 f t); body *= exp(-t / .045) * minimum(1, t / .0015)
            let body = dmath::sin(w1 * t) + 0.5 * dmath::sin(w3 * t) + 0.22 * dmath::sin(w5 * t);
            let rise = t / 0.0015;
            let rise = if rise.is_nan() || rise < 1.0 { rise } else { 1.0 };
            let body = body * (dmath::exp(-t / 0.045) * rise);
            // knock = sin(2 pi 180 t) * exp(-t / .02)
            let knock = dmath::sin(w180 * t) * dmath::exp(-t / 0.02);
            (0.9 * body + 0.35 * click + 0.5 * knock) * v * 0.8
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pop_is_short_and_starts_from_its_click() {
        let x = slap(110.0, 1.0, DUR);
        assert_eq!(x.len(), 7496); // int(0.17 * 44100): the sum is a hair under 0.17
        // at t = 0 only the click sounds: the first standard normal of generator 7, times .35 * .8
        let first = Generator::new(7).standard_normal();
        assert_eq!(x[0].to_bits(), ((0.35 * first) * 1.0 * 0.8).to_bits());
        assert!(slap(110.0, 1.0, -0.08).is_empty());
        // the same call twice is the same pop
        assert_eq!(slap(220.0, 0.7, DUR), slap(220.0, 0.7, DUR));
    }
}
