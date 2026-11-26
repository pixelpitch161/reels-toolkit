//! A note rebuilt from an instrument's recipe ([`super::recipe`]).
//!
//! The note at a pitch and strength is a blend of the recorded notes nearest it: the two nearest recorded pitches on
//! the two nearest recorded strengths ([`RecipeSet::nearest`]). Every 64 samples the player reads each overtone's
//! level off the recipe at that moment of the note, blends the recorded notes' levels in decibels (an overtone one of
//! them lacks counts as 100 dB under that note's loudest), and moves the overtone's amplitude smoothly to it across
//! the next 64 samples. Each overtone is a steady tone at the recorded overtone's place over its note's fundamental,
//! moved to the pitch asked for, so a piano's stretched overtones stay stretched; nothing is made at or above 0.45 of
//! the sample rate. The noise between the overtones is 24 bands of seeded noise, each at its band's recorded level.
//!
//! Past the recipe's last moment the note carries on as the instrument does ([`Carry`]): a piano's tone keeps dying
//! at the rate it was dying, an organ's holds. Repeats of a note ([variants](render)) differ a little in strength,
//! brightness, tuning, the overtones' starting phases and the noise, as a player's repeats do.

use std::sync::OnceLock;

use bf_core::dmath;
use bf_core::rng::Generator;

use super::recipe::{BANDS, Note, RecipeSet, band_edges};
use crate::SRF;
use crate::dsp::{FilterKind, Sos, butter_sos, sosfilt};

/// The samples between two readings of the recipe.
pub const BLOCK: usize = 64;
/// Nothing is made at or above this frequency, Hz.
pub const TOP_HZ: f64 = 0.45 * SRF;
/// An overtone this many dB under the loudest is left out for the block.
const SKIP_DB: f64 = 70.0;
/// An overtone this many dB under the loudest for [`STOP_AFTER`] seconds is stopped for good.
const STOP_DB: f64 = 90.0;
const STOP_AFTER: f64 = 0.2;
/// An overtone a blended note lacks counts as this many dB under that note's loudest.
const MISSING_DB: f64 = 100.0;
/// The note's first samples fade in over this many samples (1 ms), so it never starts with a click.
const FADE_IN: usize = 48;
const LN10_20: f64 = std::f64::consts::LN_10 / 20.0;

/// How a note goes on past its recipe's last moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Carry {
    /// Each overtone keeps falling at its rate over the recipe's last 1.5 s (never rising): a struck string.
    Decay,
    /// Each overtone and noise band holds its level of the recipe's last 0.5 s: a pipe on steady wind.
    Steady,
}

/// What a part does to a note over time, each as a function of seconds into the note.
pub struct Shaping<'a> {
    /// How hard, 0 to 1, picking between the recorded strengths.
    pub strength: &'a dyn Fn(f64) -> f64,
    /// The pitch moved by this many cents.
    pub pitch_cents: &'a dyn Fn(f64) -> f64,
    /// The whole note louder by this many dB.
    pub level_db: &'a dyn Fn(f64) -> f64,
    /// The overtones brighter by this many dB per octave over the fundamental.
    pub tilt_db: &'a dyn Fn(f64) -> f64,
    /// The noise louder by this many dB.
    pub noise_db: &'a dyn Fn(f64) -> f64,
    /// The note stops: from `at` seconds everything falls with this time constant, seconds.
    pub stop: Option<(f64, f64)>,
    /// How the note goes on past the recipe.
    pub carry: Carry,
}

/// Nothing, for a [`Shaping`] that changes nothing over time.
pub fn zero(_: f64) -> f64 {
    0.0
}

/// A recorded note as the player reads it.
struct View<'s> {
    note: &'s Note,
    /// Its loudest overtone, dB.
    top: f64,
    /// Overtone number to the index of its partial.
    by_number: Vec<Option<usize>>,
    /// Per partial, past the recipe: its level at the recipe's end and its fall, dB a second (silent if it ended
    /// before the recipe did).
    carry: Vec<(f64, f64)>,
    /// Per band, past the noise's end.
    noise_carry: [f64; BANDS],
}

fn view<'s>(set: &'s RecipeSet, i: usize, carry: Carry) -> View<'s> {
    let note = &set.notes[i];
    let g = &set.grid;
    let last = g.len() - 1;
    let end = g[last];
    let top = note
        .partials
        .iter()
        .flat_map(|p| p.level.iter())
        .map(|&v| f64::from(v))
        .fold(-200.0, f64::max);
    let max_n = note.partials.iter().map(|p| usize::from(p.number)).max().unwrap_or(0);
    let mut by_number = vec![None; max_n + 1];
    for (j, p) in note.partials.iter().enumerate() {
        by_number[usize::from(p.number)] = Some(j);
    }
    let span = |secs: f64| (0..=last).filter(move |&k| g[k] >= end - secs);
    let carry_of = |p: &super::recipe::Partial| {
        if p.end() <= last {
            return (f64::NEG_INFINITY, 0.0);
        }
        let at_end = f64::from(p.level_at(last));
        match carry {
            Carry::Steady => {
                let v: Vec<f64> = span(0.5)
                    .map(|k| f64::from(p.level_at(k)))
                    .filter(|v| v.is_finite())
                    .collect();
                (mean_db(&v), 0.0)
            }
            Carry::Decay => {
                let pts: Vec<(f64, f64)> = span(1.5)
                    .map(|k| (g[k], f64::from(p.level_at(k))))
                    .filter(|q| q.1.is_finite())
                    .collect();
                (at_end, slope(&pts).min(0.0))
            }
        }
    };
    let carry_v = note.partials.iter().map(carry_of).collect();
    let mut noise_carry = [f64::NEG_INFINITY; BANDS];
    if carry == Carry::Steady && note.noise.len() == g.len() {
        for (b, c) in noise_carry.iter_mut().enumerate() {
            let v: Vec<f64> = span(0.5)
                .map(|k| f64::from(note.noise[k][b]))
                .filter(|v| v.is_finite())
                .collect();
            *c = mean_db(&v);
        }
    }
    View {
        note,
        top,
        by_number,
        carry: carry_v,
        noise_carry,
    }
}

/// The mean of levels in dB, taken in power.
fn mean_db(v: &[f64]) -> f64 {
    if v.is_empty() {
        return f64::NEG_INFINITY;
    }
    let p = v.iter().map(|&d| dmath::exp(2.0 * d * LN10_20)).sum::<f64>() / v.len() as f64;
    dmath::ln(p) / (2.0 * LN10_20)
}

/// The least-squares slope of `(x, y)` points (0 for fewer than two).
fn slope(p: &[(f64, f64)]) -> f64 {
    if p.len() < 2 {
        return 0.0;
    }
    let n = p.len() as f64;
    let (mx, my) = (
        p.iter().map(|q| q.0).sum::<f64>() / n,
        p.iter().map(|q| q.1).sum::<f64>() / n,
    );
    let sxx: f64 = p.iter().map(|q| (q.0 - mx) * (q.0 - mx)).sum();
    if sxx <= 0.0 {
        return 0.0;
    }
    p.iter().map(|q| (q.0 - mx) * (q.1 - my)).sum::<f64>() / sxx
}

/// Where `t` falls on the grid: the point at or before it and how far on toward the next, or `None` past the end.
fn locate(grid: &[f64], t: f64) -> Option<(usize, f64)> {
    let last = grid.len() - 1;
    if t >= grid[last] {
        return None;
    }
    let i = grid.partition_point(|&g| g <= t).saturating_sub(1).min(last - 1);
    Some((i, ((t - grid[i]) / (grid[i + 1] - grid[i])).clamp(0.0, 1.0)))
}

/// Between two levels in dB: in dB when both sound, in amplitude when one is silent.
fn between(a: f64, b: f64, f: f64) -> f64 {
    match (a.is_finite(), b.is_finite()) {
        (true, true) => a + (b - a) * f,
        (false, false) => f64::NEG_INFINITY,
        _ => {
            let amp = (1.0 - f) * amp_of(a) + f * amp_of(b);
            if amp > 0.0 {
                dmath::ln(amp) / LN10_20
            } else {
                f64::NEG_INFINITY
            }
        }
    }
}

fn amp_of(db: f64) -> f64 {
    if db.is_finite() { dmath::exp(db * LN10_20) } else { 0.0 }
}

impl View<'_> {
    /// Partial `j`'s level at `t` (where `at` places it), dB.
    fn level(&self, j: usize, t: f64, at: Option<(usize, f64)>, end: f64) -> f64 {
        let p = &self.note.partials[j];
        match at {
            Some((i, f)) => between(f64::from(p.level_at(i)), f64::from(p.level_at(i + 1)), f),
            None => {
                let (l, s) = self.carry[j];
                l + s * (t - end)
            }
        }
    }

    /// Partial `j`'s pitch offset at `t`, cents.
    fn cents(&self, j: usize, at: Option<(usize, f64)>) -> f64 {
        let p = &self.note.partials[j];
        match at {
            Some((i, f)) => {
                let (a, b) = (f64::from(p.cents_at(i)), f64::from(p.cents_at(i + 1)));
                a + (b - a) * f
            }
            None => f64::from(p.cents_at(p.end().saturating_sub(1))),
        }
    }

    /// Band `b`'s noise level at `t`, dB.
    fn noise(&self, b: usize, at: Option<(usize, f64)>) -> f64 {
        let rows = &self.note.noise;
        match at {
            Some((i, f)) if i + 1 < rows.len() => between(f64::from(rows[i][b]), f64::from(rows[i + 1][b]), f),
            Some((i, _)) if i < rows.len() => f64::from(rows[i][b]),
            Some(_) if rows.len() < usize::MAX => f64::NEG_INFINITY,
            _ => self.noise_carry[b],
        }
    }
}

/// The 24 bands' filters and how much each takes off unit noise, made once.
fn bands() -> &'static [(Sos, f64); BANDS] {
    static B: OnceLock<[(Sos, f64); BANDS]> = OnceLock::new();
    B.get_or_init(|| {
        let e = band_edges();
        std::array::from_fn(|b| {
            let sos = butter_sos(FilterKind::Bp, &[e[b], e[b + 1].min(TOP_HZ)], 2, SRF);
            let mut g = Generator::new(0x5eed_0000 + b as u64);
            let mut w = vec![0.0; 96_000];
            g.fill_normal(0.0, 1.0, &mut w);
            let y = sosfilt(&sos, &w);
            let rms = (y[4800..].iter().map(|v| v * v).sum::<f64>() / (y.len() - 4800) as f64).sqrt();
            (sos, 1.0 / rms.max(1e-12))
        })
    })
}

/// The note at MIDI pitch `midi` from `set`, shaped by `s`, `len` samples long. `variant` 0 is the note as recorded;
/// each other variant is a slightly different repeat. `seed` makes the note's own randomness (its phases and noise):
/// the same arguments always give the same samples.
pub fn render(set: &RecipeSet, midi: f64, s: &Shaping, len: usize, variant: u64, seed: u64) -> Vec<f64> {
    let mut out = vec![0.0; len];
    if set.notes.is_empty() || len == 0 || set.grid.len() < 2 {
        return out;
    }
    let mut g = Generator::new(seed ^ variant.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    let (dv_strength, dv_tilt, dv_cents) = if variant == 0 {
        (0.0, 0.0, 0.0)
    } else {
        (g.uniform(-0.04, 0.04), g.uniform(-0.6, 0.6), g.uniform(-1.5, 1.5))
    };
    // the recorded notes this pitch can draw on: on every layer, the two nearest pitches
    let mut cand: Vec<usize> = Vec::new();
    for l in &set.layers {
        for (i, _) in set.nearest(midi, l.strength).iter() {
            if !cand.contains(&i) {
                cand.push(i);
            }
        }
    }
    let views: Vec<(usize, View)> = cand.iter().map(|&i| (i, view(set, i, s.carry))).collect();
    let find = |i: usize| views.iter().find(|v| v.0 == i).map(|v| &v.1);
    let mut numbers: Vec<usize> = views
        .iter()
        .flat_map(|(_, v)| v.note.partials.iter().map(|p| usize::from(p.number)))
        .collect();
    numbers.sort_unstable();
    numbers.dedup();
    let end = set.grid[set.grid.len() - 1];
    let base_hz = 440.0 * dmath::exp((midi - 69.0) / 12.0 * std::f64::consts::LN_2);
    // the overtones' oscillators: phase (as a unit complex number), last amplitude, time spent far under the loudest
    let mut osc: Vec<(f64, f64, f64, f64, bool)> = numbers
        .iter()
        .map(|_| {
            let ph = g.uniform(0.0, dmath::TAU);
            (dmath::cos(ph), dmath::sin(ph), f64::NAN, 0.0, false)
        })
        .collect();
    // the noise: only the bands any of the notes has, only for as long as it lasts
    let noise_rows = views.iter().map(|(_, v)| v.note.noise.len()).max().unwrap_or(0);
    let noise_len = if s.carry == Carry::Steady && noise_rows == set.grid.len() {
        len
    } else {
        let last = noise_rows.min(set.grid.len() - 1);
        ((set.grid[last] + 0.05) * SRF) as usize
    }
    .min(len);
    let mut noise: Vec<Option<Vec<f64>>> = vec![None; BANDS];
    if noise_len > 0 {
        for (b, slot) in noise.iter_mut().enumerate() {
            let any = views.iter().any(|(_, v)| v.note.noise.iter().any(|r| r[b].is_finite()));
            if any {
                let (sos, scale) = &bands()[b];
                let mut ng =
                    Generator::new(seed ^ 0xb00b ^ (b as u64) << 40 ^ variant.wrapping_mul(0x2545_f491_4f6c_dd1d));
                let mut w = vec![0.0; noise_len];
                ng.fill_normal(0.0, *scale, &mut w);
                *slot = Some(sosfilt(sos, &w));
            }
        }
    }
    let mut noise_prev = [f64::NAN; BANDS];
    let mut levels = vec![f64::NEG_INFINITY; numbers.len()];
    let mut freqs = vec![0.0; numbers.len()];
    let mut i0 = 0;
    while i0 < len {
        let n = BLOCK.min(len - i0);
        let t = (i0 + n) as f64 / SRF;
        let at = locate(&set.grid, t);
        let strength = (s.strength)(t) + dv_strength;
        let corners = set.nearest(midi, strength);
        let gain: f64 = corners
            .iter()
            .map(|(i, w)| w * set.layers[set.notes[i].layer].gain_db)
            .sum();
        let stop_db = match s.stop {
            Some((a, tau)) if t > a => -(t - a) / tau / LN10_20 / 1.0,
            _ => 0.0,
        };
        let base = gain + (s.level_db)(t) + stop_db;
        let tilt = (s.tilt_db)(t) + dv_tilt;
        let hz = base_hz * dmath::exp(((s.pitch_cents)(t) + dv_cents) / 1200.0 * std::f64::consts::LN_2);
        let mut loudest = f64::NEG_INFINITY;
        for (q, &k) in numbers.iter().enumerate() {
            let (mut db, mut sounding) = (0.0, false);
            let (mut ratio, mut cents, mut rw) = (0.0, 0.0, 0.0);
            for (i, w) in corners.iter() {
                let Some(v) = find(i) else { continue };
                let j = v.by_number.get(k).copied().flatten();
                let lv = j.map_or(f64::NEG_INFINITY, |j| v.level(j, t, at, end));
                if lv.is_finite() {
                    sounding = true;
                    db += w * lv;
                } else {
                    db += w * (v.top - MISSING_DB);
                }
                if let Some(j) = j {
                    ratio += w * v.note.partials[j].ratio;
                    cents += w * v.cents(j, at);
                    rw += w;
                }
            }
            if !sounding || rw <= 0.0 {
                levels[q] = f64::NEG_INFINITY;
                continue;
            }
            let ratio = ratio / rw;
            freqs[q] = hz * ratio * dmath::exp(cents / rw / 1200.0 * std::f64::consts::LN_2);
            let l = db + base + tilt * dmath::log2(ratio.max(1e-3));
            levels[q] = l;
            if freqs[q] < TOP_HZ {
                loudest = loudest.max(l);
            }
        }
        for (q, o) in osc.iter_mut().enumerate() {
            let f = freqs[q];
            let w = dmath::TAU * f / SRF;
            let quiet = levels[q] < loudest - STOP_DB;
            o.3 = if quiet { o.3 + n as f64 / SRF } else { 0.0 };
            if o.3 >= STOP_AFTER && t > 0.1 {
                o.4 = true;
            }
            let target = if o.4 || f >= TOP_HZ || levels[q] < loudest - SKIP_DB {
                0.0
            } else {
                amp_of(levels[q])
            };
            let from = if o.2.is_nan() { target } else { o.2 };
            o.2 = target;
            if from == 0.0 && target == 0.0 {
                // keep its phase running, so it comes back where it would be
                let (c, sn) = (dmath::cos(w * n as f64), dmath::sin(w * n as f64));
                (o.0, o.1) = (o.0 * c - o.1 * sn, o.0 * sn + o.1 * c);
                continue;
            }
            let (rc, rs) = (dmath::cos(w), dmath::sin(w));
            let step = (target - from) / n as f64;
            let (mut zr, mut zi) = (o.0, o.1);
            for (j, y) in out[i0..i0 + n].iter_mut().enumerate() {
                *y += (from + step * (j + 1) as f64) * zi;
                (zr, zi) = (zr * rc - zi * rs, zr * rs + zi * rc);
            }
            let m = (zr * zr + zi * zi).sqrt();
            (o.0, o.1) = (zr / m, zi / m);
        }
        if i0 < noise_len {
            let nb = n.min(noise_len - i0);
            let ndb = (s.noise_db)(t);
            for (b, buf) in noise.iter().enumerate() {
                let Some(buf) = buf else { continue };
                let (mut db, mut sounding) = (0.0, false);
                for (i, w) in corners.iter() {
                    let Some(v) = find(i) else { continue };
                    let lv = v.noise(b, at);
                    if lv.is_finite() {
                        sounding = true;
                        db += w * lv;
                    } else {
                        db += w * (v.top - MISSING_DB);
                    }
                }
                let target = if sounding { amp_of(db + base + ndb) } else { 0.0 };
                let from = if noise_prev[b].is_nan() { target } else { noise_prev[b] };
                noise_prev[b] = target;
                let step = (target - from) / nb as f64;
                for (j, y) in out[i0..i0 + nb].iter_mut().enumerate() {
                    *y += (from + step * (j + 1) as f64) * buf[i0 + j];
                }
            }
        }
        i0 += n;
    }
    for (i, y) in out.iter_mut().take(FADE_IN).enumerate() {
        *y *= 0.5 - 0.5 * dmath::cos(std::f64::consts::PI * i as f64 / FADE_IN as f64);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::recipe::{Layer, Partial, grid};
    use super::*;

    /// A one-layer set of steady notes at `pitches`, each with `parts` overtones at `level(note, k)` dB.
    fn steady(pitches: &[f64], parts: u16, end: f64, level: impl Fn(usize, u16) -> f32) -> RecipeSet {
        let g = grid(end);
        let n = g.len();
        RecipeSet {
            name: "T".into(),
            grid: g,
            layers: vec![Layer {
                name: "one".into(),
                strength: 1.0,
                gain_db: 0.0,
            }],
            notes: pitches
                .iter()
                .enumerate()
                .map(|(i, &m)| Note {
                    layer: 0,
                    midi: m,
                    f0: 440.0 * dmath::pow(2.0, (m - 69.0) / 12.0),
                    b: 0.0,
                    release: 0.1,
                    partials: (1..=parts)
                        .map(|k| Partial {
                            number: k,
                            ratio: f64::from(k),
                            first: 0,
                            level: vec![level(i, k); n],
                            cents: vec![0.0; n],
                        })
                        .collect(),
                    noise: vec![],
                })
                .collect(),
        }
    }

    fn shaping(carry: Carry, stop: Option<(f64, f64)>) -> Shaping<'static> {
        Shaping {
            strength: &|_| 1.0,
            pitch_cents: &zero,
            level_db: &zero,
            tilt_db: &zero,
            noise_db: &zero,
            stop,
            carry,
        }
    }

    /// The level of the tone at `f` Hz in `x` around `t` s, dB (a Blackman-windowed sum over 0.1 s).
    fn tone_db(x: &[f64], f: f64, t: f64) -> f64 {
        tone_db_over(x, f, t, 0.05)
    }

    /// The same over a window `half` seconds either side.
    fn tone_db_over(x: &[f64], f: f64, t: f64, half: f64) -> f64 {
        let (c, h) = ((t * SRF) as usize, (half * SRF) as usize);
        let (mut re, mut im, mut ws) = (0.0, 0.0, 0.0);
        for (n, v) in x.iter().enumerate().take(c + h).skip(c - h) {
            let a = dmath::TAU * (n + h - c) as f64 / (2 * h) as f64;
            let w = 0.42 - 0.5 * dmath::cos(a) + 0.08 * dmath::cos(2.0 * a);
            let ph = dmath::TAU * (f * n as f64 / SRF).fract();
            re += v * w * dmath::cos(ph);
            im += v * w * dmath::sin(ph);
            ws += w;
        }
        20.0 * dmath::log10(2.0 * (re * re + im * im).sqrt() / ws)
    }

    fn rms(x: &[f64]) -> f64 {
        (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt()
    }

    #[test]
    fn a_recorded_note_is_rebuilt_at_its_levels() {
        let set = steady(&[57.0], 8, 2.0, |_, k| -10.0 - 3.0 * f32::from(k));
        let x = render(&set, 57.0, &shaping(Carry::Steady, None), 48_000, 0, 1);
        for k in 1..=8u16 {
            let got = tone_db(&x, 220.0 * f64::from(k), 0.5);
            assert!((got - (-10.0 - 3.0 * f64::from(k))).abs() < 0.3, "{k}: {got}");
        }
    }

    #[test]
    fn between_recordings_the_levels_are_interpolated() {
        let set = steady(
            &[48.0, 54.0],
            3,
            1.0,
            |i, k| if k == 2 { [-6.0, -18.0][i] } else { -10.0 },
        );
        let x = render(&set, 51.0, &shaping(Carry::Steady, None), 48_000, 0, 1);
        let f = 440.0 * dmath::pow(2.0, (51.0 - 69.0) / 12.0);
        let got = tone_db(&x, 2.0 * f, 0.5);
        assert!((got + 12.0).abs() < 0.5, "{got}");
    }

    #[test]
    fn nothing_is_made_above_0_45_of_the_rate() {
        let set = steady(&[107.0], 8, 1.0, |_, _| -10.0);
        let x = render(&set, 107.0, &shaping(Carry::Steady, None), 32_768, 0, 1);
        let f = 440.0 * dmath::pow(2.0, (107.0 - 69.0) / 12.0);
        assert!(f * 6.0 > TOP_HZ);
        for k in 6..=8 {
            assert!(tone_db(&x, f * f64::from(k), 0.3) < -100.0, "{k}");
        }
    }

    #[test]
    fn pitches_past_the_recordings_take_the_nearest_tone() {
        let set = steady(&[36.0, 96.0], 6, 1.0, |i, k| {
            if i == 0 {
                -6.0 * f32::from(k)
            } else {
                -3.0 * f32::from(k)
            }
        });
        for (m, near) in [(12.0, 0usize), (120.0, 1)] {
            let x = render(&set, m, &shaping(Carry::Steady, None), 96_000, 0, 1);
            assert!(x.iter().all(|v| v.is_finite()));
            let f = 440.0 * dmath::pow(2.0, (m - 69.0) / 12.0);
            // a window long enough to tell 16 Hz apart
            let d = tone_db_over(&x, f, 1.0, 0.5) - tone_db_over(&x, 2.0 * f, 1.0, 0.5);
            assert!((d - [6.0, 3.0][near]).abs() < 0.5 || 2.0 * f >= TOP_HZ, "{m}: {d}");
        }
    }

    #[test]
    fn a_note_longer_than_its_recipe_carries_on() {
        let g = grid(1.6);
        let mut set = steady(&[60.0], 4, 1.6, |_, _| -10.0);
        // a falling note: 6 dB a second
        for p in &mut set.notes[0].partials {
            for (j, v) in p.level.iter_mut().enumerate() {
                *v = -10.0 - 6.0 * g[j] as f32;
            }
        }
        let steady_x = render(
            &steady(&[60.0], 4, 1.6, |_, _| -10.0),
            60.0,
            &shaping(Carry::Steady, None),
            192_000,
            0,
            1,
        );
        let decay_x = render(&set, 60.0, &shaping(Carry::Decay, None), 192_000, 0, 1);
        let f = 261.625_565;
        let (a, b) = (tone_db(&steady_x, f, 1.5), tone_db(&steady_x, f, 3.9));
        assert!((a - b).abs() < 1.0, "{a} {b}");
        let (a, b) = (tone_db(&decay_x, f, 2.0), tone_db(&decay_x, f, 3.5));
        assert!(((a - b) - 9.0).abs() < 1.5, "{a} {b}");
        let blk = |x: &[f64], i: usize| x[i * BLOCK..(i + 1) * BLOCK].to_vec();
        assert_ne!(blk(&steady_x, 2000), blk(&steady_x, 2500));
    }

    #[test]
    fn the_stop_falls_with_its_time_constant() {
        let set = steady(&[60.0], 4, 1.0, |_, _| -10.0);
        let x = render(&set, 60.0, &shaping(Carry::Steady, Some((0.5, 0.07))), 48_000, 0, 1);
        let f = 261.625_565;
        let early = rms(&x[(0.3 * SRF) as usize..(0.45 * SRF) as usize]);
        let late = rms(&x[(0.69 * SRF) as usize..(0.71 * SRF) as usize]);
        let d = 20.0 * dmath::log10(early / late);
        assert!((d - 24.8).abs() < 1.5, "{d}");
        assert!(tone_db(&x, f, 0.25).is_finite());
    }

    #[test]
    fn repeats_differ_a_little() {
        let set = steady(&[60.0], 8, 1.0, |_, k| -10.0 - 2.0 * f32::from(k));
        let sh = shaping(Carry::Steady, None);
        let v: Vec<Vec<f64>> = (0..3).map(|i| render(&set, 60.0, &sh, 24_000, i, 9)).collect();
        for a in 0..3 {
            for b in a + 1..3 {
                let d: Vec<f64> = v[a].iter().zip(&v[b]).map(|(x, y)| x - y).collect();
                assert!(rms(&d) > 0.02 * rms(&v[0]), "{a} {b}");
            }
            let l = 20.0 * dmath::log10(rms(&v[a]) / rms(&v[0]));
            assert!(l.abs() < 1.5, "{a}: {l}");
        }
    }

    #[test]
    fn the_same_note_is_the_same_everywhere() {
        let set = steady(&[60.0, 66.0], 8, 1.0, |i, k| -10.0 - f32::from(k) * (1.0 + i as f32));
        let sh = shaping(Carry::Decay, Some((0.4, 0.1)));
        let a = render(&set, 62.0, &sh, 30_000, 2, 5);
        let b = render(&set, 62.0, &sh, 30_000, 2, 5);
        assert_eq!(a, b);
        let c = std::thread::scope(|s| {
            s.spawn(|| render(&set, 62.0, &shaping(Carry::Decay, Some((0.4, 0.1))), 30_000, 2, 5))
                .join()
                .unwrap()
        });
        assert_eq!(a, c);
    }
}
