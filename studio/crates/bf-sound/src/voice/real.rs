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

/// A voice's per-note overtone correction toward its real instrument, in rising MIDI order: the gain in dB of each of
/// its first [`SET`] harmonics, the eighth's gain also for everything above it. Written by
/// `cargo run --release -p bf-harness --example references -- correct "<FIGHTER>"`, which measures the voice's own
/// notes against the real ones and refines the gains until the corrected notes match.
pub type Correction = [(i32, [f64; SET])];

/// A voice's per-note added overtones, in rising MIDI order: for each of its first [`SET`] harmonics, the level in
/// dB under the fundamental of a partial laid in beside the voice's own, [`NONE`] for none. For the harmonics a voice
/// barely has, which a [`Correction`]'s peaks cannot lift. Written by `references -- correct` with the correction.
pub type Fill = [(i32, [f64; SET])];

/// A [`Fill`] level that adds nothing.
pub const NONE: f64 = -99.0;

/// How a voice is reworked from its own notes, for a voice whose make-up is not opened up as the saxes' are: the
/// overtones it barely has added ([`Fill`], empty for none), an overtone [`Correction`], the real instrument's slower
/// start (`speak`, seconds to full level, 0 to keep the voice's own) and quicker stop (`stop`, an extra time constant
/// after the held length, 0 to keep the voice's own). A plucked or struck voice keeps its own loudness moment by
/// moment (`keep_shape`, [`follow`]): the correction's narrow peaks restore its low harmonics over tens of
/// milliseconds, which would soften its snap into a swell.
#[derive(Clone, Copy, Debug)]
pub struct Rework {
    pub fill: &'static Fill,
    pub correction: &'static Correction,
    pub speak: f64,
    pub stop: f64,
    pub keep_shape: bool,
}

/// `x` (a note at `f` Hz, `sr` Hz) with partials added at `db` (dB beside its fundamental, under it when negative,
/// [`NONE`] for none) for its second to [`SET`]th harmonics. The note's own fundamental is taken out as an analytic signal (the positive band of
/// its spectrum from half to one and a half times `f`, with soft edges), and each added harmonic is that signal's
/// phase times `k` (plus a fixed offset of its own) at its level, so the partials follow the note's vibrato, bends
/// and swells.
pub fn fill(x: &[f64], f: f64, sr: f64, db: &[f64; SET]) -> Vec<f64> {
    let want: Vec<(usize, f64, f64, f64)> = db
        .iter()
        .enumerate()
        .skip(1)
        .filter(|&(k, &d)| d > NONE + 1.0 && f * ((k + 1) as f64) < 0.45 * sr)
        .map(|(k, &d)| {
            // Schroeder's phases (pi k (k - 1) / 8 for harmonic k): the added partials do not all peak together,
            // so the note stays no peakier than a played one
            let h = (k + 1) as f64;
            let phase = dmath::PI * h * (h - 1.0) / SET as f64;
            (k + 1, dmath::pow(10.0, d / 20.0), dmath::cos(phase), dmath::sin(phase))
        })
        .collect();
    if want.is_empty() || x.is_empty() || f <= 0.0 {
        return x.to_vec();
    }
    let n = (x.len() + (0.1 * sr) as usize).next_power_of_two();
    let fft = crate::dsp::fft::Fft::new(n);
    let mut re = vec![0.0; n];
    re[..x.len()].copy_from_slice(x);
    let mut im = vec![0.0; n];
    fft.forward(&mut re, &mut im);
    let bin = sr / n as f64;
    for i in 0..n {
        let nu = i as f64 * bin;
        // flat from 0.75 f to 1.25 f, a raised cosine down to nothing at 0.5 f and 1.5 f; negative frequencies out
        let w = if i > n / 2 {
            0.0
        } else {
            let d = (nu - f).abs() / f;
            if d <= 0.25 {
                1.0
            } else if d >= 0.5 {
                0.0
            } else {
                0.5 + 0.5 * dmath::cos(dmath::PI * (d - 0.25) / 0.25)
            }
        };
        re[i] *= 2.0 * w;
        im[i] *= 2.0 * w;
    }
    fft.inverse(&mut re, &mut im);
    let mut y = x.to_vec();
    for (i, v) in y.iter_mut().enumerate() {
        let (a, b) = (re[i], im[i]);
        let m = (a * a + b * b).sqrt();
        if m <= 1e-12 {
            continue;
        }
        let (ur, ui) = (a / m, b / m);
        let (mut pr, mut pi, mut k) = (ur, ui, 1);
        for &(h, g, c, sn) in &want {
            while k < h {
                (pr, pi) = (pr * ur - pi * ui, pr * ui + pi * ur);
                k += 1;
            }
            *v += g * m * (pr * c - pi * sn);
        }
    }
    y
}

/// A peaking filter (a biquad) over `x` at `sr` Hz: `db` of gain at `fc`, `q` its quality, 0 dB far from it.
fn peak(x: &[f64], fc: f64, q: f64, db: f64, sr: f64) -> Vec<f64> {
    let a = dmath::pow(10.0, db / 40.0);
    let w0 = 2.0 * dmath::PI * fc / sr;
    let alpha = dmath::sin(w0) / (2.0 * q);
    let cw = dmath::cos(w0);
    let a0 = 1.0 + alpha / a;
    let (b0, b1, b2) = ((1.0 + alpha * a) / a0, -2.0 * cw / a0, (1.0 - alpha * a) / a0);
    let (a1, a2) = (-2.0 * cw / a0, (1.0 - alpha / a) / a0);
    let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
    x.iter()
        .map(|&v| {
            let y = b0 * v + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
            (x2, x1) = (x1, v);
            (y2, y1) = (y1, y);
            y
        })
        .collect()
}

/// `x` (a note at `f` Hz, `sr` Hz) with its first [`SET`] harmonics turned by `db`: everything by the eighth's gain,
/// and a narrow peak or dip (a third of the pitch wide) at each of the first seven by the rest of its own; at the
/// note's own loudness.
pub fn correct(x: &[f64], f: f64, sr: f64, db: &[f64; SET]) -> Vec<f64> {
    let rest = db[SET - 1];
    let mut y: Vec<f64> = x.to_vec();
    for (k, d) in db.iter().enumerate() {
        let fc = f * (k + 1) as f64;
        let d = d - rest;
        if !(fc > 0.0 && fc < 0.45 * sr) || d == 0.0 {
            continue;
        }
        y = peak(&y, fc, 3.0 * (k + 1) as f64, d, sr);
    }
    let (a, b) = (
        x.iter().map(|v| v * v).sum::<f64>(),
        y.iter().map(|v| v * v).sum::<f64>(),
    );
    if b > 0.0 && a > 0.0 {
        let k = (a / b).sqrt();
        y.iter_mut().for_each(|v| *v *= k);
    }
    y
}

/// The reworked note of MIDI `midi`, held `dur` s, from the voice's own note `x` at `sr` Hz: the overtones it lacks
/// added ([`fill`]) and its overtones corrected ([`correct`]), its start slowed and its stop quickened as `r` says, and the `variant`th repeat ([`wobble`]) a few
/// cents off (read through a hair faster or slower) and a little louder or quieter.
pub fn rework(x: &[f64], midi: f64, dur: f64, sr: f64, r: &Rework, variant: u64) -> Vec<f64> {
    let f = 440.0 * dmath::pow(2.0, (midi - 69.0) / 12.0);
    let mut y = if r.fill.is_empty() {
        correct(x, f, sr, &levels(r.correction, midi))
    } else {
        // at the voice's own loudness, not the louder filled note's
        let mut y = correct(
            &fill(x, f, sr, &levels(r.fill, midi)),
            f,
            sr,
            &levels(r.correction, midi),
        );
        let (a, b) = (
            x.iter().map(|v| v * v).sum::<f64>(),
            y.iter().map(|v| v * v).sum::<f64>(),
        );
        if a > 0.0 && b > 0.0 {
            let k = (a / b).sqrt();
            y.iter_mut().for_each(|v| *v *= k);
        }
        y
    };
    if r.keep_shape {
        y = follow(&y, x, f, sr);
        let (a, b) = (
            x.iter().map(|v| v * v).sum::<f64>(),
            y.iter().map(|v| v * v).sum::<f64>(),
        );
        if a > 0.0 && b > 0.0 {
            let k = (a / b).sqrt();
            y.iter_mut().for_each(|v| *v *= k);
        }
    }
    for (i, v) in y.iter_mut().enumerate() {
        let t = i as f64 / sr;
        if r.speak > 0.0 && t < r.speak {
            *v *= t / r.speak;
        }
        if r.stop > 0.0 && t > dur {
            *v *= dmath::exp(-(t - dur) / r.stop);
        }
    }
    if variant == 0 {
        return y;
    }
    let (pitch, level) = wobble(pitch_seed(f) ^ (variant << 32), 4.0, 0.04);
    (0..y.len())
        .map(|i| {
            let p = i as f64 * pitch;
            let j = p as usize;
            let fr = p - j as f64;
            let a = y.get(j).copied().unwrap_or(0.0);
            let b = y.get(j + 1).copied().unwrap_or(0.0);
            (a + fr * (b - a)) * level
        })
        .collect()
}

/// `y` (a rework of `x`, a note at `f` Hz, `sr` Hz) as loud as `x` moment by moment: both measured over two of the
/// note's periods (at least 5 ms) centred on each sample, the gain between them held within 24 dB either way (and
/// where `x` is near silent, `y` is left as it is).
pub fn follow(y: &[f64], x: &[f64], f: f64, sr: f64) -> Vec<f64> {
    let w = ((2.0 / f).max(0.005) * sr) as usize;
    // running sums of squares, so each sample's level costs two lookups
    let sums = |z: &[f64]| {
        let mut pre = vec![0.0; z.len() + 1];
        for (i, v) in z.iter().enumerate() {
            pre[i + 1] = pre[i] + v * v;
        }
        pre
    };
    let level = |pre: &[f64], i: usize| {
        let n = pre.len() - 1;
        let (a, b) = (i.saturating_sub(w / 2), (i + w - w / 2).min(n));
        if b <= a {
            0.0
        } else {
            ((pre[b] - pre[a]) / w as f64).max(0.0).sqrt()
        }
    };
    let (px, py) = (sums(x), sums(y));
    let eps = 1e-3 * (0..x.len()).map(|i| level(&px, i)).fold(0.0, f64::max);
    y.iter()
        .enumerate()
        .map(|(i, v)| v * ((level(&px, i) + eps) / (level(&py, i) + eps)).clamp(1.0 / 16.0, 16.0))
        .collect()
}

/// A part's notes as the Composed arrangement plays them: each note the part makes itself, reworked ([`rework`]),
/// its repeats taking turns among [`TURNS`] variants, kept by the part's own key for the note and the variant.
#[derive(Debug, Default)]
pub struct RealNotes {
    repeats: Repeats,
    notes: Mutex<BTreeMap<RealKey, std::sync::Arc<[f64]>>>,
}

/// A kept note of [`RealNotes`]: the part's key for the note, and the variant.
type RealKey = (Vec<u64>, u64);

impl RealNotes {
    /// The next repeat of the note `key` (whatever the part keeps its note by): made by `make` from the variant
    /// the first time that variant is asked for, kept after.
    pub fn get(&self, key: &[u64], make: impl FnOnce(u64) -> Vec<f64>) -> std::sync::Arc<[f64]> {
        let variant = self.repeats.next(key);
        let mut notes = self.notes.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let k = (key.to_vec(), variant);
        if let Some(x) = notes.get(&k) {
            return std::sync::Arc::clone(x);
        }
        let x: std::sync::Arc<[f64]> = make(variant).into();
        notes.insert(k, std::sync::Arc::clone(&x));
        x
    }
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

    /// The level in dB of `x`'s component at `f` Hz (a projection over the whole of `x`, at `sr` Hz).
    fn level_at(x: &[f64], f: f64, sr: f64) -> f64 {
        let (mut c, mut s) = (0.0, 0.0);
        for (i, v) in x.iter().enumerate() {
            let w = 2.0 * dmath::PI * f * i as f64 / sr;
            c += v * dmath::cos(w);
            s += v * dmath::sin(w);
        }
        let n = x.len() as f64;
        10.0 * dmath::log10((c * c + s * s) / (n * n) + 1e-30)
    }

    /// A steady note at `f` Hz with eight equal harmonics, `n` samples at 48 kHz.
    fn even_note(f: f64, n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| {
                let t = i as f64 / 48_000.0;
                (1..=SET)
                    .map(|k| dmath::sin(2.0 * dmath::PI * f * k as f64 * t))
                    .sum::<f64>()
                    / 8.0
            })
            .collect()
    }

    #[test]
    fn a_correction_turns_each_harmonic_by_its_gain_and_keeps_the_loudness() {
        let f = 110.0;
        let x = even_note(f, 48_000);
        let db = [-12.0, 0.0, 6.0, -6.0, 0.0, -20.0, 3.0, 0.0];
        let y = correct(&x, f, 48_000.0, &db);
        assert_eq!(y.len(), x.len());
        let energy = |v: &[f64]| v.iter().map(|a| a * a).sum::<f64>();
        assert!((energy(&y) / energy(&x) - 1.0).abs() < 1e-9);
        // each harmonic moves by its own gain against the second's (after the settling of the first 0.2 s)
        let (xs, ys) = (&x[9_600..], &y[9_600..]);
        let moved = |k: usize| {
            let fk = f * (k + 1) as f64;
            level_at(ys, fk, 48_000.0) - level_at(xs, fk, 48_000.0)
        };
        let base = moved(1);
        for (k, want) in db.iter().enumerate() {
            let got = moved(k) - base;
            assert!(
                (got - want).abs() < 1.5,
                "harmonic {}: {got:.2} dB, wanted {want}",
                k + 1
            );
        }
        // no correction is the note itself
        assert_eq!(correct(&x, f, 48_000.0, &[0.0; SET]), x);
    }

    #[test]
    fn a_rework_shapes_the_start_and_stop_and_its_repeats_differ() {
        const FLAT: Rework = Rework {
            fill: &[],
            correction: &[],
            speak: 0.0,
            stop: 0.0,
            keep_shape: false,
        };
        const SHAPED: Rework = Rework {
            fill: &[],
            correction: &[],
            speak: 0.05,
            stop: 0.02,
            keep_shape: false,
        };
        let x = even_note(220.0, 24_000);
        assert_eq!(rework(&x, 57.0, 0.3, 48_000.0, &FLAT, 0), x);
        let y = rework(&x, 57.0, 0.3, 48_000.0, &SHAPED, 0);
        // about half as loud at 25 ms, the same at 0.2 s, and far quieter 0.1 s after the release
        let t = 1_201.0 / 48_000.0;
        assert!((y[1_201] / x[1_201] - t / 0.05).abs() < 1e-9);
        assert_eq!(y[9_601], x[9_601]);
        assert!((y[19_211] / x[19_211]).abs() < 0.01);
        let (a, b) = (
            rework(&x, 57.0, 0.3, 48_000.0, &FLAT, 1),
            rework(&x, 57.0, 0.3, 48_000.0, &FLAT, 2),
        );
        assert_eq!(a.len(), x.len());
        assert!(a != x && a != b && a.iter().all(|v| v.is_finite()));
        assert_eq!(a, rework(&x, 57.0, 0.3, 48_000.0, &FLAT, 1));
    }

    #[test]
    fn a_fill_adds_the_missing_harmonics_in_step_with_the_note() {
        let f = 220.0;
        // a pure tone, steady, then one sliding up a whole tone over its second half second
        let steady: Vec<f64> = (0..24_000)
            .map(|i| 0.5 * dmath::sin(2.0 * dmath::PI * f * i as f64 / 48_000.0))
            .collect();
        let mut add = [NONE; SET];
        (add[2], add[4]) = (-6.0, -20.0);
        let y = fill(&steady, f, 48_000.0, &add);
        assert_eq!(y.len(), steady.len());
        let (ys, xs) = (&y[4_800..19_200], &steady[4_800..19_200]);
        let under = |k: f64| level_at(ys, k * f, 48_000.0) - level_at(ys, f, 48_000.0);
        assert!((under(3.0) - -6.0).abs() < 1.0, "third {:.2}", under(3.0));
        assert!((under(5.0) - -20.0).abs() < 1.0, "fifth {:.2}", under(5.0));
        assert!(
            under(2.0) < -60.0 && under(4.0) < -60.0,
            "{:.1} {:.1}",
            under(2.0),
            under(4.0)
        );
        assert!((level_at(ys, f, 48_000.0) - level_at(xs, f, 48_000.0)).abs() < 0.1);
        // nothing to add is the note itself
        assert_eq!(fill(&steady, f, 48_000.0, &[NONE; SET]), steady);
        // on a slide the added third climbs with it: its energy sits at three times the pitch of the moment
        let (mut ph, mut slide) = (0.0, Vec::new());
        for i in 0..48_000 {
            let t = i as f64 / 48_000.0;
            let fi = if t < 0.5 {
                f
            } else {
                f * (1.0 + 0.122 * (t - 0.5) / 0.5)
            };
            ph += 2.0 * dmath::PI * fi / 48_000.0;
            slide.push(0.5 * dmath::sin(ph));
        }
        let mut third = [NONE; SET];
        third[2] = -6.0;
        let y = fill(&slide, f, 48_000.0, &third);
        let tail = &y[43_200..46_800];
        let top = f * 1.122 * (1.0 - 0.1 * 0.122 / 1.122);
        assert!(level_at(tail, 3.0 * top, 48_000.0) - level_at(tail, 3.0 * f, 48_000.0) > 12.0);
    }

    #[test]
    fn a_rework_that_keeps_its_shape_keeps_the_plucks_snap() {
        // a low pluck: every harmonic at once, dying away; the correction cuts everything above the eighth and
        // lifts the first three back with narrow peaks, which take tens of milliseconds to build up
        let f = 55.0;
        let x: Vec<f64> = even_note(f, 24_000)
            .iter()
            .enumerate()
            .map(|(i, v)| v * dmath::exp(-(i as f64) / 48_000.0 / 0.15))
            .collect();
        static LOW: [(i32, [f64; SET]); 1] = [(33, [0.0, 0.0, 0.0, -20.0, -20.0, -20.0, -20.0, -20.0])];
        let soft = Rework {
            fill: &[],
            correction: &LOW,
            speak: 0.0,
            stop: 0.0,
            keep_shape: false,
        };
        let snap = Rework {
            keep_shape: true,
            ..soft
        };
        // the level over the first 5 ms against the strongest 5 ms of the first half second, in dB
        let start = |z: &[f64]| {
            let w = 240;
            let rms = |a: usize| (z[a..a + w].iter().map(|v| v * v).sum::<f64>() / w as f64).sqrt();
            let top = (0..z.len().min(24_000) - w).step_by(w).map(rms).fold(0.0, f64::max);
            20.0 * dmath::log10(rms(0) / top)
        };
        let (own, soft, snap) = (
            start(&x),
            start(&rework(&x, 33.0, 0.5, 48_000.0, &soft, 0)),
            start(&rework(&x, 33.0, 0.5, 48_000.0, &snap, 0)),
        );
        assert!(
            soft < own - 4.0,
            "the peaks soften the start: {soft:.1} against {own:.1} dB"
        );
        assert!((snap - own).abs() < 2.0, "kept: {snap:.1} against {own:.1} dB");
    }

    #[test]
    fn real_notes_take_turns_and_are_kept() {
        let notes = RealNotes::default();
        let made = std::cell::Cell::new(0);
        let mut got = Vec::new();
        for _ in 0..4 {
            got.push(notes.get(&[60, 1], |v| {
                made.set(made.get() + 1);
                vec![v as f64]
            }));
        }
        assert_eq!(got.iter().map(|x| x[0]).collect::<Vec<_>>(), [0.0, 1.0, 2.0, 0.0]);
        assert_eq!(made.get(), 3, "the fourth is the first again, kept");
        assert!(std::sync::Arc::ptr_eq(&got[0], &got[3]));
    }
}
