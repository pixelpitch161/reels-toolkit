//! Theme: minor bossa noir (`projects/ballfight/kit/moods/bossa_noir.py`, #49 Flute vs Clarinet). A sly, smoky
//! late-night bossa nova with no woodwinds of its own (the fighters are the front line): a nylon-string guitar
//! comping the 2-bar "3-2" bossa figure on the 8ths with the thumb's bass under every chord, a soft upright-style
//! bass (root on 1, fifth on the and of 2, fifth on 3, a pickup on the and of 4), brushes on every 8th with a felt
//! bass drum and cross-stick rim clicks on the clave, and a dark Rhodes pad holding each bar's chord; the back half
//! adds ghost strokes, a shaker, the Rhodes an octave up and bass pickups into the next root. Built for D minor
//! (Dm7 Dm6 Gm7 C7 Fmaj7 Bbmaj7 Em7b5 A7); any chord works (voicings by rule).
//!
//! Here: the theme's harmony helpers and its instruments as the kit computes them. The arrangement that reproduces
//! the kit's bed is [`BossaNoirClassic`], in `classic.rs`; it also holds the kit's per-bed cache of sounds (which
//! the kit keys loosely, so a cached sound can be another call's: see there).
//!
//! **Voicings.** The mood has no table: [`voicing`] is its rule for every chord the harmony parses (the third, the
//! seventh or the root, the fifth, and a colour tone: the dominant's flat ninth, the half-diminished chord's
//! eleventh, otherwise the ninth), each placed from E3 up. The Rhodes takes the same rule from G3 ([`voicing_from`]).
//!
//! **The nylon string.** `pluck` runs a filtered burst through `scipy.signal.lfilter([1.0], a, exc)` with
//! `a = [1, 0, ..., 0, -ring / 2, -ring / 2]` (a Karplus-Strong loop of `N` samples). [`karplus_strong`] is that
//! filter's transposed direct form II, in scipy's operations, without walking the `N` zero taps per sample: a zero
//! tap only passes its delay along unchanged (adding `x * 0` and subtracting `y * 0` leaves every non-zero value as
//! it is), so only a delay that holds a zero is walked, to keep its sign. A test holds it to `dsp::lfilter`'s bits.

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_core::py::round_half_even;
use bf_sound::SRF;
use bf_sound::synth::{self, FILTER_O, clip, noise};

pub use self::classic::BossaNoirClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options, no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "bossa_noir",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(BossaNoirClassic::new())),
};

/// The minor bossa's ii-V cycle of #49.
const LOOP: [&str; 8] = ["Dm7", "Dm6", "Gm7", "C7", "Fmaj7", "Bbmaj7", "Em7b5", "A7"];

/// What the interface says about the bossa noir.
pub static CARD: ThemeCard = ThemeCard {
    display: "Bossa noir",
    text: "A smoky late-night minor bossa: nylon strings, upright bass, brushes, a dark Rhodes. Leaves the melody and \
           the counter-line to the fighters.",
    key: "D",
    scale: "harmonic minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The kit configs that play the bossa noir (Table S4): their music as each `episode.py` sets it.
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_flute_vs_clarinet",
    fighters: ["FLUTE", "CLARINET"],
    bpm: 132.0,
    key: "D",
    scale: "harmonic minor",
    intro: ["Dm7", "A7"],
    chord_loop: &LOOP,
    tail: &[],
}];

/// `CLAVE`: the bossa's 2-bar "3-2" figure on the 8ths (an even bar, an odd bar).
pub const CLAVE: [&[usize]; 2] = [&[0, 3, 6], &[2, 5]];

/// `pluck`'s default `ring`.
pub const PLUCK_RING: f64 = 0.996;
/// `comp_chord`'s default `dur` (a ghost stroke's) and `roll`.
pub const COMP_DUR: f64 = 0.32;
pub const COMP_ROLL: f64 = 0.004;

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// `Bed.root(ch)`: the bass's root, `T.root_above(ch, 33)` (A1 to G#2).
///
/// # Errors
/// Where Python raises: `ch` does not parse.
pub fn root(harmony: &Harmony, ch: &str) -> Result<i32, HarmonyError> {
    harmony.root_above(ch, 33)
}

/// `Bed.fifth(ch)`: the chord's fifth above the bass's root (a perfect fifth for a chord of fewer than three
/// tones), an octave down when it would be above B2.
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn fifth(harmony: &Harmony, ch: &str) -> Result<i32, HarmonyError> {
    let r = root(harmony, ch)?;
    let pcs = harmony.chord_pcs(ch)?;
    let f = r + if pcs.len() > 2 {
        (pcs[2] - pcs[0]).rem_euclid(12)
    } else {
        7
    };
    Ok(if f <= 47 { f } else { f - 12 })
}

/// `Bed.voicing(ch)`: the guitar's four-note shape from E3 up ([`voicing_from`] at `lo = 52`).
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn voicing(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    voicing_from(harmony, ch, 52)
}

/// `Bed.voicing(ch, lo)`: the third, the seventh (the sixth, or the root again for a triad), the fifth and a
/// colour tone on top (the dominant seventh's flat ninth, the half-diminished chord's eleventh, otherwise the
/// ninth), each the lowest such note at or above `lo` that no earlier tone took (else an octave up), in rising
/// order.
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn voicing_from(harmony: &Harmony, ch: &str, lo: i32) -> Result<Vec<i32>, HarmonyError> {
    let pcs = harmony.chord_pcs(ch)?;
    let r = pcs[0];
    let mut tones = vec![pcs[1], if pcs.len() > 3 { pcs[3] } else { r }, pcs[2]];
    let third = (pcs[1] - r).rem_euclid(12);
    let sev = (pcs.len() > 3).then(|| (pcs[3] - r).rem_euclid(12));
    if third == 4 && sev == Some(10) {
        tones.push((r + 1).rem_euclid(12));
    } else if (pcs[2] - r).rem_euclid(12) == 6 {
        tones.push((r + 5).rem_euclid(12));
    } else {
        tones.push((r + 2).rem_euclid(12));
    }
    let mut out: Vec<i32> = Vec::new();
    for pc in tones {
        let mut m = lo + (pc - lo).rem_euclid(12);
        while out.contains(&m) {
            m += 12;
        }
        out.push(m);
    }
    out.sort_unstable();
    Ok(out)
}

/// The length `N` of `pluck`'s loop for MIDI note `m`: `max(2, int(round(SR / f)))`.
pub fn loop_length(m: i32) -> usize {
    let f = synth::mtof(f64::from(m));
    (round_half_even(SRF / f) as i64).max(2) as usize
}

/// `lfilter([1.0], a, exc)` for `a` of length `period + 2`: `a[0] = 1`, `a[period] = a[period + 1] = tap`, every
/// other coefficient zero (the kit's `tap` is `-0.5 * ring`). The transposed direct form II of scipy's kernel:
/// `y = z[0] + 1 * x`; the delays `z[k - 1] = z[k] + x * 0 - y * 0` for `k < period`, which pass a non-zero value
/// along unchanged; `z[period - 1] = z[period] + x * 0 - y * tap`; `z[period] = x * 0 - y * tap`.
///
/// # Panics
/// When `period` is below 2.
pub fn karplus_strong(exc: &[f64], period: usize, tap: f64) -> Vec<f64> {
    assert!(period >= 2, "karplus_strong: a loop of {period} samples");
    let n = exc.len();
    let mut y = vec![0.0; n];
    // w[m]: the value written into z[period - 1] at sample m (it reaches z[0] at sample m + period - 1, after
    // period - 1 passes through the zero taps, and is read at sample m + period).
    let mut w = vec![0.0; n];
    let mut z_last = 0.0f64; // z[period]
    for i in 0..n {
        let x = exc[i];
        let z0 = if i < period {
            // the start's delays: +0.0, which stays +0.0 through zero taps
            0.0
        } else {
            let mut z = w[i - period];
            if z == 0.0 {
                // A zero keeps or changes its sign at each zero tap: walk it as scipy does.
                for s in i - period + 1..i {
                    z = (z + exc[s] * 0.0) - y[s] * 0.0;
                }
            }
            z
        };
        let yi = z0 + 1.0 * x;
        y[i] = yi;
        w[i] = (z_last + x * 0.0) - yi * tap;
        z_last = x * 0.0 - yi * tap;
    }
    y
}

/// `Bed.pluck(m, dur, seed, bright, ring)` before the cache: a plucked nylon string (Karplus-Strong: a soft,
/// filtered burst through a tuned loop; dark and round). The burst is `N` normals from a new generator of
/// `2000 + m + 37 * seed`, low-passed twice at `900 + 3000 * bright` Hz (the flesh of the finger).
///
/// # Panics
/// Where Python raises: the note is shorter than its loop (`exc[:N] = ...` would not fit).
pub fn pluck(m: i32, dur: f64, seed: i64, bright: f64, ring: f64) -> Vec<f64> {
    let period = loop_length(m);
    let n = length(dur * SRF);
    assert!(n >= period, "pluck: {n} samples hold no loop of {period}");
    let burst_seed = 2000 + i64::from(m) + 37 * seed;
    let burst = noise(u64::try_from(burst_seed).expect("a noise seed"), period);
    let fc = 900.0 + 3000.0 * bright;
    let mut exc = vec![0.0; n];
    exc[..period].copy_from_slice(&synth::lp(&synth::lp(&burst, fc, FILTER_O), fc, FILTER_O));
    let y = karplus_strong(&exc, period, -0.5 * ring);
    let t = synth::tt(n);
    y.iter().zip(&t).map(|(v, t)| v * clip(t / 0.003, 0.0, 1.0)).collect()
}

/// `Bed.kick(1.0)` before the cache and its velocity: a felt bass drum, `A.hp(A.kick(0.5, 0.20, 100, 50),
/// 38)[:int(0.30 * SR)]`.
pub fn kick_felt() -> Vec<f64> {
    let x = synth::hp(&synth::kick(0.5, 0.20, 100.0, 50.0), 38.0, FILTER_O);
    x[..length(0.30 * SRF)].to_vec()
}

/// `Bed.brush(1.0, seed, long)` before the cache: a brush on the snare, a tap (short) or a swirl (a sweep across
/// the beat), from `40 + seed`'s noise.
pub fn brush(seed: u64, long: bool) -> Vec<f64> {
    let n = length(if long { 0.42 } else { 0.16 } * SRF);
    let t = synth::tt(n);
    let x = synth::bp(&noise(40 + seed, n), 1800.0, 8000.0, FILTER_O);
    let span = n as f64 / SRF;
    x.iter()
        .zip(&t)
        .map(|(v, &t)| {
            let env = if long {
                dmath::pow(dmath::sin(PI * clip(t / span, 0.0, 1.0)), 1.5)
            } else {
                clip(t / 0.004, 0.0, 1.0) * dmath::exp(-t / 0.05)
            };
            v * env
        })
        .collect()
}

/// `Bed.rim(1.0, seed)` before the cache: a cross-stick click on the rim (woody, hollow), from `70 + seed`'s noise.
pub fn rim(seed: u64) -> Vec<f64> {
    let n = length(0.10 * SRF);
    let t = synth::tt(n);
    let click = synth::bp(&noise(70 + seed, n), 1500.0, 6500.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let tone = dmath::sin(2.0 * PI * 1650.0 * t) * 0.6 + dmath::sin(2.0 * PI * 520.0 * t) * 0.8;
            let click = click[i] * dmath::exp(-t / 0.004);
            (tone * dmath::exp(-t / 0.018) + click) * clip(t / 0.0006, 0.0, 1.0)
        })
        .collect()
}

/// `Bed.shaker(1.0, seed, accent)` before the cache: high-passed noise of `90 + seed`, longer when accented.
pub fn shaker(seed: u64, accent: bool) -> Vec<f64> {
    let n = length(0.12 * SRF);
    let t = synth::tt(n);
    let x = synth::hp(&noise(90 + seed, n), 6000.0, FILTER_O);
    let dec = if accent { 0.035 } else { 0.022 };
    x.iter()
        .zip(&t)
        .map(|(v, &t)| v * clip(t / 0.012, 0.0, 1.0) * dmath::exp(-t / dec))
        .collect()
}

/// `Bed.choke(v, seed)` (never cached): a crash choked by hand (the band hits' "pah"), from `80 + seed`'s noise.
pub fn choke(v: f64, seed: u64) -> Vec<f64> {
    let n = length(0.24 * SRF);
    let t = synth::tt(n);
    let hiss = synth::hp(&noise(80 + seed, n), 4000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let x = hiss[i] * dmath::exp(-t[i] / 0.06);
            x * clip((0.22 - t[i]) / 0.06, 0.0, 1.0) * v
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};
    use bf_sound::dsp;

    use super::*;

    fn harmony(chords: &[&str]) -> Harmony {
        let g = Grid::new(132.0);
        let all: Vec<String> = chords.iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2.min(all.len())], &all, &[], g.n_bars);
        Harmony::new(&g, "D", scale("harmonic minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    fn bits(x: &[f64]) -> Vec<u64> {
        x.iter().map(|v| v.to_bits()).collect()
    }

    /// The loop is scipy's lfilter on the kit's coefficients, bit for bit.
    #[test]
    fn the_loop_is_scipys_lfilter() {
        for (m, ring, n) in [(52, 0.996, 3000), (33, 0.993, 2500), (79, 0.998, 1800), (100, 0.5, 400)] {
            let period = loop_length(m);
            let mut exc = noise(7 + m as u64, n);
            for v in &mut exc[period.min(n)..] {
                *v = 0.0;
            }
            // zeros and a negative zero inside the burst, so zero delays are walked
            exc[1] = -0.0;
            exc[2] = 0.0;
            let mut a = vec![0.0; period + 2];
            a[0] = 1.0;
            a[period] = -0.5 * ring;
            a[period + 1] = -0.5 * ring;
            let want = dsp::lfilter(&[1.0], &a, &exc);
            assert_eq!(bits(&karplus_strong(&exc, period, -0.5 * ring)), bits(&want), "m {m}");
        }
        // a silent excitation: every delay holds a zero
        let exc = vec![-0.0; 50];
        let mut a = vec![0.0; 7];
        a[0] = 1.0;
        a[5] = -0.4;
        a[6] = -0.4;
        assert_eq!(
            bits(&karplus_strong(&exc, 5, -0.4)),
            bits(&dsp::lfilter(&[1.0], &a, &exc))
        );
    }

    /// As the kit's `voicings.json` gives them for #49's chords.
    #[test]
    fn the_rule_voices_the_bossa_chords() {
        let h = harmony(&LOOP);
        let want: [(&str, &[i32]); 8] = [
            ("Dm7", &[52, 53, 57, 60]),
            ("Dm6", &[52, 53, 57, 59]),
            ("Gm7", &[53, 57, 58, 62]),
            ("C7", &[52, 55, 58, 61]),
            ("Fmaj7", &[52, 55, 57, 60]),
            ("Bbmaj7", &[53, 57, 60, 62]),
            ("Em7b5", &[55, 57, 58, 62]),
            ("A7", &[52, 55, 58, 61]),
        ];
        for (ch, notes) in want {
            assert_eq!(voicing(&h, ch).unwrap(), notes, "{ch}");
        }
        assert_eq!((root(&h, "Dm7").unwrap(), fifth(&h, "Dm7").unwrap()), (38, 45));
        assert_eq!((root(&h, "A7").unwrap(), fifth(&h, "A7").unwrap()), (33, 40));
        assert_eq!((root(&h, "Bbmaj7").unwrap(), fifth(&h, "Bbmaj7").unwrap()), (34, 41));
        assert_eq!(voicing(&h, "F#m"), Err(HarmonyError::UnknownChord("F#m".into())));
    }

    #[test]
    fn instrument_lengths_follow_pythons_int() {
        assert_eq!(pluck(52, 0.4, 0, 0.34, PLUCK_RING).len(), 19200);
        assert_eq!(kick_felt().len(), 14400);
        assert_eq!((brush(0, true).len(), brush(0, false).len()), (20160, 7680));
        assert_eq!(
            (rim(0).len(), shaker(0, true).len(), choke(1.0, 0).len()),
            (4800, 5760, 11520)
        );
        assert_eq!(loop_length(69), 109);
    }
}
