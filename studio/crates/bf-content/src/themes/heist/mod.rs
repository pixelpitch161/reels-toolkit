//! Theme: heist (`projects/ballfight/kit/moods/heist.py`, #52 OBOE vs BASSOON). A sneaky minor caper with no
//! woodwinds of its own (the two double reeds are the leads): brushes (a soft kick on 1, a swish on 2 and 4, rim
//! clicks on the swung off-beats, a ride in the back half); a walking upright with a chromatic approach into the next
//! chord; pizzicato off-beat plucks that turn into a creeping arpeggio; muted vibes stabs; and the tiptoe tension (a
//! quiet high tremolo string on the chord's fifth, a clock tick on the 16ths before the K.O.). Built for D minor
//! (Dm Dm Bb A7 | Dm Gm Bb A7); any chord works (roots and voicings by rule).
//!
//! Here: the theme's harmony helpers and its instruments as the kit computes them. The arrangement that reproduces
//! the kit's bed is `HeistClassic`, in `classic.rs`; it also holds the kit's per-bed cache of sounds (keyed
//! loosely: a duration is `round(dur, 3)`, a brush's noise is its seed modulo 3, a rim's modulo 2).
//!
//! **Voicings.** The mood has no table: [`voicing`] is its rule for every chord the harmony parses (the chord's
//! tones from `lo` up to an octave above it, the first `n`).
//!
//! **No `boom`.** The class defines `hit` and no other optional hook, so a fight whose fighter's part calls
//! `bed.boom` cannot play under this theme (ruling M3-R2: such pairs are refused).

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_sound::SRF;
use bf_sound::synth::{self, FILTER_O, clip};

use super::{NOT_PORTED_CARD, Theme};

/// The theme as the registry lists it.
pub static THEME: Theme = Theme {
    name: "heist",
    ported: false,
    card: &NOT_PORTED_CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: classic::classic,
};

/// `voicing`'s defaults: `lo = 57`, `n = 4`.
pub const VOICING_LO: i32 = 57;
pub const VOICING_N: usize = 4;

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// `Bed.root(ch)`: the upright's root, `T.root_above(ch, 33)` (A1 to G#2).
///
/// # Errors
/// Where Python raises: `ch` does not parse.
pub fn root(harmony: &Harmony, ch: &str) -> Result<i32, HarmonyError> {
    harmony.root_above(ch, 33)
}

/// `Bed.voicing(ch)`: the chord's tones from A3 up, the first four ([`voicing_from`] at the defaults).
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn voicing(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    voicing_from(harmony, ch, VOICING_LO, VOICING_N)
}

/// `Bed.voicing(ch, lo, n)`: the chord's tones from `lo` to `lo + 12` (both ends included), the first `n`.
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn voicing_from(harmony: &Harmony, ch: &str, lo: i32, n: usize) -> Result<Vec<i32>, HarmonyError> {
    let mut tones = harmony.chord_tones(lo, lo + 12, ch)?;
    tones.truncate(n);
    Ok(tones)
}

/// `Bed.walk(ch, nxt)`: four walking-bass notes for a bar of `ch` heading to `nxt`: the root, a chord tone up,
/// another, a chromatic approach into the next root.
///
/// # Errors
/// Where Python raises: a chord does not parse.
pub fn walk(harmony: &Harmony, ch: &str, nxt: &str) -> Result<[i32; 4], HarmonyError> {
    let r = root(harmony, ch)?;
    let tones = harmony.chord_tones(r, r + 12, ch)?;
    let n_r = root(harmony, nxt)?;
    let third = if tones.len() > 1 { tones[1] } else { r + 3 };
    let fifth = if tones.len() > 2 { tones[2] } else { r + 7 };
    let mut appr = if n_r < r + 12 { n_r + 1 } else { n_r - 1 };
    if (appr - fifth).abs() > 7 {
        appr = n_r - 1;
    }
    Ok([r, third, fifth, appr])
}

/// `np.where(t < dur, 1.0, np.exp(-(t - dur) / rel))`.
fn release(t: f64, dur: f64, rel: f64) -> f64 {
    if t < dur { 1.0 } else { dmath::exp(-(t - dur) / rel) }
}

/// `Bed.kick()` before the cache and its velocity: `A.hp(A.kick(0.9, 0.18, 120, 46), 35)[:int(0.3 * SR)]`.
pub fn kick() -> Vec<f64> {
    let x = synth::hp(&synth::kick(0.9, 0.18, 120.0, 46.0), 35.0, FILTER_O);
    let n = length(0.3 * SRF).min(x.len());
    x[..n].to_vec()
}

/// `Bed.brush(1.0, seed)` before the cache: a band-passed swell and fall of `60 + seed`'s noise, and a tap of
/// `70 + seed`'s.
pub fn brush(seed: u64) -> Vec<f64> {
    let n = length(0.24 * SRF);
    let t = synth::tt(n);
    let x = synth::bp(&synth::noise(60 + seed, n), 1800.0, 9000.0, FILTER_O);
    let tap = synth::bp(&synth::noise(70 + seed, n), 900.0, 4000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            // `np.clip(t - 0.03, 0, None)`
            let late = t - 0.03;
            let late = if late < 0.0 { 0.0 } else { late };
            let env = clip(t / 0.03, 0.0, 1.0) * dmath::exp(-late / 0.07);
            let tap = tap[i] * dmath::exp(-t / 0.012);
            x[i] * env * 0.9 + tap * 0.6
        })
        .collect()
}

/// `Bed.rim(1.0, seed)` before the cache: a hard, short, woody tick (two decaying sines and a burst of `90 + seed`'s
/// band-passed noise).
pub fn rim(seed: u64) -> Vec<f64> {
    let n = length(0.06 * SRF);
    let t = synth::tt(n);
    let noise = synth::bp(&synth::noise(90 + seed, n), 2500.0, 8000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let x = dmath::sin(2.0 * PI * 1650.0 * t) * dmath::exp(-t / 0.006)
                + dmath::sin(2.0 * PI * 820.0 * t) * dmath::exp(-t / 0.01) * 0.5;
            x + noise[i] * dmath::exp(-t / 0.003) * 0.5
        })
        .collect()
}

/// `Bed.ride()` before the cache: high-passed noise of seed 31 and three bell partials.
pub fn ride() -> Vec<f64> {
    let n = length(0.5 * SRF);
    let t = synth::tt(n);
    let hiss = synth::hp(&synth::noise(31, n), 5000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let x = hiss[i] * dmath::exp(-t / 0.18) * 0.5;
            // Python's `sum` starts from 0.
            let mut bell = 0.0;
            for f in [3150.0, 4630.0, 5960.0] {
                bell += dmath::sin(2.0 * PI * f * t);
            }
            x + bell * dmath::exp(-t / 0.25) * 0.05
        })
        .collect()
}

/// `Bed.upright(m, dur)` before the cache: a plucked upright, a thumpy sine with a little 2nd and 3rd harmonic and a
/// soft attack thud of `int(m)`'s low-passed noise, high-passed at 30 Hz.
///
/// # Panics
/// On a negative note (`default_rng` refuses a negative seed; no voicing of the mood gives one).
pub fn upright(m: i32, dur: f64) -> Vec<f64> {
    let n = length((dur + 0.1) * SRF);
    let t = synth::tt(n);
    let f = synth::mtof(f64::from(m));
    let seed = u64::try_from(m).expect("a MIDI note of 0 or more");
    let thud = synth::lp(&synth::noise(seed, n), 400.0, FILTER_O);
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let t = t[i];
            let x = dmath::sin(2.0 * PI * f * t)
                + 0.35 * dmath::sin(4.0 * PI * f * t) * dmath::exp(-t / 0.08)
                + 0.12 * dmath::sin(6.0 * PI * f * t) * dmath::exp(-t / 0.04);
            let env = clip(t / 0.004, 0.0, 1.0) * dmath::exp(-t / 0.45) * release(t, dur, 0.03);
            let thud = thud[i] * dmath::exp(-t / 0.01) * 0.3;
            x * env + thud
        })
        .collect();
    synth::hp(&y, 30.0, FILTER_O)
}

/// `Bed.pizz(ms)` before the cache: a short plucked chord (bright attack, quick fall), high-passed at 140 Hz.
pub fn pizz(ms: &[i32]) -> Vec<f64> {
    let n = length(0.35 * SRF);
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    for &m in ms {
        let f = synth::mtof(f64::from(m));
        for (o, &t) in x.iter_mut().zip(&t) {
            *o += (dmath::sin(2.0 * PI * f * t)
                + 0.5 * dmath::sin(4.0 * PI * f * t) * dmath::exp(-t / 0.03)
                + 0.25 * dmath::sin(6.0 * PI * f * t) * dmath::exp(-t / 0.015))
                * dmath::exp(-t / 0.09);
        }
    }
    let k = ms.len().max(1) as f64;
    let x: Vec<f64> = x.iter().map(|v| v / k).collect();
    synth::hp(&x, 140.0, FILTER_O)
        .iter()
        .zip(&t)
        .map(|(v, &t)| v * clip(t / 0.002, 0.0, 1.0))
        .collect()
}

/// `Bed.vibes(ms, dur)` before the cache: muted vibes, soft-mallet bars with the motor tremolo, low-passed at
/// 2600 Hz.
pub fn vibes(ms: &[i32], dur: f64) -> Vec<f64> {
    let n = length((dur + 0.2) * SRF);
    let t = synth::tt(n);
    // Python's `sum` starts from 0.
    let mut x = vec![0.0; n];
    for &m in ms {
        let f = synth::mtof(f64::from(m));
        for (o, &t) in x.iter_mut().zip(&t) {
            *o += dmath::sin(2.0 * PI * f * t) + 0.25 * dmath::sin(2.0 * PI * f * 3.9 * t) * dmath::exp(-t / 0.05);
        }
    }
    let k = ms.len().max(1) as f64;
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let t = t[i];
            let trem = 1.0 - 0.35 * (0.5 + 0.5 * dmath::sin(2.0 * PI * 5.5 * t));
            let env = clip(t / 0.003, 0.0, 1.0) * dmath::exp(-t / 0.35) * release(t, dur, 0.05);
            x[i] / k * trem * env
        })
        .collect();
    synth::lp(&y, 2600.0, FILTER_O)
}

/// `Bed.trem(m, dur)` before the cache: a high tremolo string note, three detuned saws band-passed and bowed fast.
pub fn trem(m: i32, dur: f64) -> Vec<f64> {
    let n = length((dur + 0.2) * SRF);
    let t = synth::tt(n);
    let x = synth::bp(
        &synth::saw(synth::mtof(f64::from(m)), n, 0.006, 3),
        900.0,
        5000.0,
        FILTER_O,
    );
    x.iter()
        .zip(&t)
        .map(|(v, &t)| {
            let tr = 0.6 + 0.4 * dmath::sin(2.0 * PI * 7.5 * t).abs();
            let env = clip(t / 0.15, 0.0, 1.0) * release(t, dur, 0.08);
            v * tr * env
        })
        .collect()
}

/// `Bed.tick()` before the cache: a muted clock tick.
pub fn tick() -> Vec<f64> {
    let n = length(0.03 * SRF);
    synth::tt(n)
        .iter()
        .map(|&t| dmath::sin(2.0 * PI * 2900.0 * t) * dmath::exp(-t / 0.002))
        .collect()
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};

    use super::*;

    fn harmony(chords: &[&str]) -> Harmony {
        let g = Grid::new(118.0);
        let all: Vec<String> = chords.iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2.min(all.len())], &all, &[], g.n_bars);
        Harmony::new(&g, "D", scale("minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    #[test]
    fn the_rule_voices_the_caper_chords() {
        let h = harmony(&["Dm", "A7", "Bb", "Gm"]);
        assert_eq!(voicing(&h, "Dm").unwrap(), [57, 62, 65, 69]);
        assert_eq!(voicing(&h, "A7").unwrap(), [57, 61, 64, 67]);
        assert_eq!(voicing_from(&h, "Gm", 62, 3).unwrap(), [62, 67, 70]);
        assert_eq!((root(&h, "Dm").unwrap(), root(&h, "Bb").unwrap()), (38, 34));
        assert_eq!(voicing(&h, "F#m"), Err(HarmonyError::UnknownChord("F#m".into())));
    }

    #[test]
    fn the_walk_approaches_the_next_root() {
        let h = harmony(&["Dm", "A7", "Bb", "Gm"]);
        // Dm to Dm: D2 F2 A2, then D#2 (6 below the fifth, kept)
        assert_eq!(walk(&h, "Dm", "Dm").unwrap(), [38, 41, 45, 39]);
        // Dm to Bb: B1 is 10 below A2, so the approach is A1 (Bb1 - 1)
        assert_eq!(walk(&h, "Dm", "Bb").unwrap(), [38, 41, 45, 33]);
        // A7 to Dm: A1 C#2 E2, then D#2
        assert_eq!(walk(&h, "A7", "Dm").unwrap(), [33, 37, 40, 39]);
    }

    #[test]
    fn instrument_lengths_follow_pythons_int() {
        assert_eq!(kick().len(), 14400);
        assert_eq!((brush(0).len(), rim(0).len(), ride().len(), tick().len()), (11520, 2880, 24000, 1440));
        assert_eq!(upright(38, 0.5).len(), 28800);
        assert_eq!(pizz(&[57, 62]).len(), 16800);
        assert_eq!(vibes(&[62, 65, 69], 0.4).len(), 28800);
        assert_eq!(trem(81, 1.0).len(), 57600);
    }
}
