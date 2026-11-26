//! Voices toward the real instrument (sound upgrade, stage 3, S1), Composed only.
//!
//! A built voice keeps its own make-up (its formants, its growl, its breath, its envelope) and so the series'
//! character; what moves is the balance of its first eight overtones, toward the levels a recording of the real
//! instrument has at that pitch (FluidR3, measured by `bf-harness`'s `timbre` and written as a [`Profile`]).
//! [`reshape`] does it for an additive voice: the first eight harmonics take the profile's levels, the higher ones keep
//! the voice's own slope from the eighth, and the whole keeps the voice's strongest level. [`wobble`] gives repeated
//! notes their small seeded differences (a few cents of pitch, a little level).

use bf_core::dmath;
use bf_core::rng::Generator;

/// How many overtones a profile sets.
pub const SET: usize = 8;

/// A real instrument's overtone levels, dB under the strongest of the first eight, at MIDI notes in rising order.
pub type Profile = [(i32, [f64; SET])];

/// The profile's levels at MIDI note `midi`: interpolated in dB between the two nearest notes of the table, the
/// table's end note beyond its range. An empty table gives a flat profile.
pub fn levels(p: &Profile, midi: f64) -> [f64; SET] {
    let Some(first) = p.first() else {
        return [0.0; SET];
    };
    if midi <= f64::from(first.0) {
        return first.1;
    }
    for w in p.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if midi <= f64::from(b.0) {
            let t = (midi - f64::from(a.0)) / f64::from(b.0 - a.0).max(1.0);
            let mut out = [0.0; SET];
            for (k, o) in out.iter_mut().enumerate() {
                *o = a.1[k] + t * (b.1[k] - a.1[k]);
            }
            return out;
        }
    }
    p[p.len() - 1].1
}

/// Reshapes an additive voice's harmonic amplitudes `a` (`a[0]` the fundamental) toward `target` (dB under the
/// strongest, as [`levels`] gives): the first [`SET`] take `target`'s levels under the strongest of the voice's own
/// first eight, and each higher harmonic keeps its own amplitude scaled by the eighth's change, so the voice's slope
/// and formants above the eighth stay. A zero amplitude stays zero.
pub fn reshape(a: &mut [f64], target: &[f64; SET]) {
    let n = a.len().min(SET);
    if n == 0 {
        return;
    }
    let top = a[..n].iter().copied().fold(0.0, f64::max);
    if top <= 0.0 {
        return;
    }
    let eighth = a[n - 1];
    for (k, v) in a.iter_mut().enumerate().take(n) {
        if *v != 0.0 {
            *v = top * dmath::pow(10.0, target[k] / 20.0);
        }
    }
    let scale = if eighth > 0.0 { a[n - 1] / eighth } else { 0.0 };
    for v in a.iter_mut().skip(n) {
        *v *= scale;
    }
}

/// A repeated note's small seeded differences: `(pitch factor, level factor)`, the pitch within `cents` either way
/// and the level within `level` (a share) either way, from `seed`.
pub fn wobble(seed: u64, cents: f64, level: f64) -> (f64, f64) {
    let mut g = Generator::new(seed);
    let c = (g.random() * 2.0 - 1.0) * cents;
    let l = (g.random() * 2.0 - 1.0) * level;
    (dmath::pow(2.0, c / 1200.0), 1.0 + l)
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: &Profile = &[
        (40, [0.0, -10.0, -20.0, -30.0, -40.0, -50.0, -60.0, -70.0]),
        (44, [-20.0; SET]),
    ];

    #[test]
    fn the_levels_are_interpolated_between_the_notes_and_held_beyond() {
        assert_eq!(levels(P, 30.0), P[0].1);
        assert_eq!(levels(P, 50.0), P[1].1);
        let mid = levels(P, 42.0);
        assert!(
            (mid[0] + 10.0).abs() < 1e-12 && (mid[7] + 45.0).abs() < 1e-12,
            "{mid:?}"
        );
        assert_eq!(levels(&[], 60.0), [0.0; SET]);
    }

    #[test]
    fn reshaping_sets_the_first_eight_and_keeps_the_slope_above() {
        let mut a: Vec<f64> = (1..=12).map(|k| 1.0 / f64::from(k)).collect();
        let before = a.clone();
        let t = [-20.0, -2.0, -2.0, -1.0, -14.0, 0.0, -11.0, -21.0];
        reshape(&mut a, &t);
        for k in 0..SET {
            assert!((20.0 * dmath::log10(a[k]) - t[k]).abs() < 1e-9, "{a:?}");
        }
        let s = a[7] / before[7];
        for k in SET..12 {
            assert!((a[k] / before[k] - s).abs() < 1e-12);
        }
    }

    #[test]
    fn a_wobble_is_small_and_seeded() {
        let (p, l) = wobble(7, 6.0, 0.05);
        assert_eq!((p, l), wobble(7, 6.0, 0.05));
        assert_ne!((p, l), wobble(8, 6.0, 0.05));
        assert!((dmath::log2(p) * 1200.0).abs() <= 6.0 && (l - 1.0).abs() <= 0.05);
    }
}
