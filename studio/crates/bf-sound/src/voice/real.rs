//! Voices toward the real instrument (sound upgrade, stage 3, S1), Composed only.
//!
//! A built voice keeps its own make-up (its formants, its growl, its breath, its envelope) and so the series'
//! character; what moves is the balance of its first eight overtones, toward the levels a recording of the real
//! instrument has at that pitch (FluidR3, measured by `bf-harness`'s `timbre` and written as a [`Profile`]).
//! [`reshape`] does it for an additive voice: the first eight harmonics take the profile's levels, the higher ones keep
//! the voice's own slope from the eighth, and the whole keeps the voice's strongest level. [`wobble`] gives repeated
//! notes their small seeded differences (a few cents of pitch, a little level).

use std::collections::BTreeMap;
use std::sync::Mutex;

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

/// How many variants the repeats of one note take turns among.
pub const TURNS: u64 = 3;

/// A part's count of how often it has asked for each note, so that under the Composed arrangement the repeats of a
/// note take turns among [`TURNS`] variants ([`wobble`]): the first time a note is asked for it is variant 0, the
/// next time 1, then 2, then 0 again. A part makes one with its build, so the count starts over with every build.
#[derive(Debug, Default)]
pub struct Repeats(Mutex<BTreeMap<Vec<u64>, u64>>);

impl Repeats {
    /// The variant of this asking for the note `key` (whatever identifies the note: its pitch, length, level...).
    pub fn next(&self, key: &[u64]) -> u64 {
        let mut seen = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let n = seen.entry(key.to_vec()).or_insert(0);
        *n += 1;
        (*n - 1) % TURNS
    }
}

/// The MIDI note of `f` Hz (A4 = 440 Hz = 69), not rounded.
pub fn midi_of(f: f64) -> f64 {
    69.0 + 12.0 * dmath::log2(f / 440.0)
}

/// How a reworked voice's note is shaped: the overtone levels at its pitch ([`levels`]), how long it takes to reach
/// full level and its release's time constant, in seconds, and how deep its reed buzz is ([`buzz`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    pub levels: [f64; SET],
    pub speak: f64,
    pub release: f64,
    pub buzz: f64,
}

/// A reed's buzz on a 44.1 kHz tone: a slow, rough flicker of its level (a normal noise every 720 samples, about
/// 16 ms, joined by straight lines), `depth` deep, from `seed`.
pub fn buzz(x: &mut [f64], seed: u64, depth: f64) {
    let mut g = Generator::new(seed);
    let raw: Vec<f64> = (0..x.len() / 720 + 3).map(|_| g.standard_normal()).collect();
    for (i, o) in x.iter_mut().enumerate() {
        let p = i as f64 / 720.0;
        let j = p as usize;
        let fr = p - j as f64;
        *o *= 1.0 + depth * (raw[j] + fr * (raw[j + 1] - raw[j]));
    }
}

/// The seed of a note's buzz and variants: its pitch in tenths of a hertz, as the Pixel Pitch voices seed their
/// breath.
pub fn pitch_seed(f: f64) -> u64 {
    ((f * 10.0) as i64).rem_euclid(100_003) as u64
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
    fn a_buzz_flickers_the_level_a_little_and_is_seeded() {
        let mut a = vec![1.0; 4410];
        let mut b = a.clone();
        buzz(&mut a, 3, 0.06);
        buzz(&mut b, 3, 0.06);
        assert_eq!(a, b);
        assert!(a.iter().all(|v| (v - 1.0).abs() < 0.06 * 5.0));
        assert!(a.iter().any(|v| (v - 1.0).abs() > 0.01));
        let mut c = vec![1.0; 4410];
        buzz(&mut c, 4, 0.06);
        assert_ne!(a, c);
    }

    #[test]
    fn repeats_take_turns() {
        let r = Repeats::default();
        let a: Vec<u64> = (0..7).map(|_| r.next(&[60, 1])).collect();
        assert_eq!(a, [0, 1, 2, 0, 1, 2, 0]);
        assert_eq!(r.next(&[61, 1]), 0, "another note counts by itself");
        assert!((midi_of(440.0) - 69.0).abs() < 1e-12 && (midi_of(220.0) - 57.0).abs() < 1e-12);
    }

    #[test]
    fn a_wobble_is_small_and_seeded() {
        let (p, l) = wobble(7, 6.0, 0.05);
        assert_eq!((p, l), wobble(7, 6.0, 0.05));
        assert_ne!((p, l), wobble(8, 6.0, 0.05));
        assert!((dmath::log2(p) * 1200.0).abs() <= 6.0 && (l - 1.0).abs() <= 0.05);
    }
}
