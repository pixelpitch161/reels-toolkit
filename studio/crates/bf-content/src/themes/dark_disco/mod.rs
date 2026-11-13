//! Theme: dark disco (`projects/ballfight/kit/moods/dark_disco.py`, #51 ALTO SAX vs TENOR SAX). A minor
//! four-on-the-floor groove with no reeds of its own (the two fighters are the disco leads): a four-on-the-floor kick,
//! a clap on 2 and 4, open hi-hats on every off-beat 8th and closed 16ths under them in the back half; an
//! octave-jumping filtered-saw bass with chromatic pickups in the back half; staccato string stabs; a filtered minor
//! pad breathing open and shut; and the mirror ball's quiet high glints on hashed 16ths. Built for A minor
//! (Am F Dm E7 | Am F G E7); any chord works (roots and voicings by rule).
//!
//! Here: the theme's harmony helpers and its instruments as the kit computes them. The arrangement that reproduces
//! the kit's bed is [`DarkDiscoClassic`], in `classic.rs`; it also holds the kit's per-bed cache of sounds (keyed
//! loosely: a duration is `round(dur, 3)`, a clap's noise is its seed modulo 3, a hat's modulo 4).
//!
//! **Voicings.** The mood has no table: [`voicing`] is its rule for every chord the harmony parses (the chord's
//! tones from `lo` up to an octave above it, the first four).
//!
//! **No `boom`.** The class defines `hit` and no other optional hook, so a fight whose fighter's part calls
//! `bed.boom` cannot play under this theme (ruling M3-R2: such pairs are refused).

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::synth::{self, FILTER_O, clip};

pub use self::classic::DarkDiscoClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options, no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "dark_disco",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(DarkDiscoClassic::new())),
};

/// #51's loop in A minor.
const LOOP: [&str; 8] = ["Am", "F", "Dm", "E7", "Am", "F", "G", "E7"];

/// What the interface says about the dark disco.
pub static CARD: ThemeCard = ThemeCard {
    display: "Dark disco",
    text: "A minor four-on-the-floor: octave bass, open hats on the off-beats, string stabs, a breathing pad and a \
           mirror-ball shimmer. Leaves the leads to the fighters.",
    key: "A",
    scale: "minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The kit configs that play the dark disco (Table S4): their music as each `episode.py` sets it.
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_altosax_vs_tenorsax",
    fighters: ["ALTO SAX", "TENOR SAX"],
    bpm: 118.0,
    key: "A",
    scale: "minor",
    intro: ["Am", "E7"],
    chord_loop: &LOOP,
    tail: &["F", "G", "E7", "E7"],
}];

/// `stab`'s default `dur` and `lo`.
pub const STAB_DUR: f64 = 0.16;
pub const STAB_LO: i32 = 57;
/// The pad's voicing starts here (`self.voicing(ch, 52)`).
pub const PAD_LO: i32 = 52;

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// `r.normal(0, 1, n)`: `n` standard normals drawn from `r` (the clap's draws share one generator).
fn normals(r: &mut Generator, n: usize) -> Vec<f64> {
    let mut x = vec![0.0; n];
    r.fill_normal(0.0, 1.0, &mut x);
    x
}

/// `Bed.root(ch)`: the bass's root, `T.root_above(ch, 33)` (A1 to G#2).
///
/// # Errors
/// Where Python raises: `ch` does not parse.
pub fn root(harmony: &Harmony, ch: &str) -> Result<i32, HarmonyError> {
    harmony.root_above(ch, 33)
}

/// `Bed.voicing(ch)`: the stab voicing from A3 up ([`voicing_from`] at `lo = 57`).
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn voicing(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    voicing_from(harmony, ch, STAB_LO)
}

/// `Bed.voicing(ch, lo)`: the chord's tones from `lo` to `lo + 12` (both ends included), the first four.
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn voicing_from(harmony: &Harmony, ch: &str, lo: i32) -> Result<Vec<i32>, HarmonyError> {
    let mut tones = harmony.chord_tones(lo, lo + 12, ch)?;
    tones.truncate(4);
    Ok(tones)
}

/// `sum(A.saw(A.mtof(m), n, det=det, voices=3) for m in ms) / max(1, len(ms))` (Python's `sum` starts from 0).
fn saw_chord(ms: &[i32], n: usize, det: f64) -> Vec<f64> {
    let mut x = vec![0.0; n];
    for &m in ms {
        for (v, s) in x.iter_mut().zip(synth::saw(synth::mtof(f64::from(m)), n, det, 3)) {
            *v += s;
        }
    }
    let k = ms.len().max(1) as f64;
    x.iter().map(|v| v / k).collect()
}

/// The kit's banded filter sweep: `out[a:b] = A.lp(x[max(0, a - run):b], fc(a, b))[-(b - a):]` for `nb` bands
/// `a, b = int(k * n / nb), int((k + 1) * n / nb)`.
fn banded(x: &[f64], nb: usize, run: usize, fc: impl Fn(usize, usize) -> f64) -> Vec<f64> {
    let n = x.len();
    let mut out = vec![0.0; n];
    for k in 0..nb {
        let a = ((k * n) as f64 / nb as f64) as usize;
        let b = (((k + 1) * n) as f64 / nb as f64) as usize;
        assert!(b > a, "a band of no samples ({n} samples in {nb} bands)");
        let y = synth::lp(&x[a.saturating_sub(run)..b], fc(a, b), FILTER_O);
        out[a..b].copy_from_slice(&y[y.len() - (b - a)..]);
    }
    out
}

/// `Bed.kick(1.0, big)` before the cache and its velocity: `A.hp(A.kick(1.0, 0.30 if big else 0.20, 160, 44),
/// 32)[:int((0.5 if big else 0.32) * SR)]`.
pub fn kick(big: bool) -> Vec<f64> {
    let x = synth::hp(
        &synth::kick(1.0, if big { 0.30 } else { 0.20 }, 160.0, 44.0),
        32.0,
        FILTER_O,
    );
    let n = length(if big { 0.5 } else { 0.32 } * SRF).min(x.len());
    x[..n].to_vec()
}

/// `Bed.clap(1.0, seed)` before the cache: three band-passed bursts of `80 + seed`'s noise and a snare under them,
/// every draw from the one generator in the kit's order.
pub fn clap(seed: u64) -> Vec<f64> {
    let n = length(0.25 * SRF);
    let t = synth::tt(n);
    let mut r = Generator::new(80 + seed);
    let mut x = vec![0.0; n];
    for (j, d) in [0.0, 0.009, 0.019].into_iter().enumerate() {
        let a = length(d * SRF);
        let burst = synth::bp(&normals(&mut r, n - a), 900.0, 5200.0, FILTER_O);
        let dec = if j < 2 { 0.011 } else { 0.06 };
        // `A.tt(n - a)` starts again from 0 at the burst's first sample.
        for (i, (o, v)) in x[a..].iter_mut().zip(&burst).enumerate() {
            *o += v * dmath::exp(-t[i] / dec);
        }
    }
    let rattle = synth::bp(&normals(&mut r, n), 2000.0, 9000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let sn = rattle[i] * dmath::exp(-t / 0.08) * 0.5
                + dmath::sin(2.0 * PI * 200.0 * t) * dmath::exp(-t / 0.04) * 0.4;
            x[i] * 0.6 + sn
        })
        .collect()
}

/// `Bed.hat(1.0, open_, seed)` before the cache: high-passed noise of `40 + seed`, open (long) or closed.
pub fn hat(open: bool, seed: u64) -> Vec<f64> {
    let n = length(if open { 0.32 } else { 0.07 } * SRF);
    let t = synth::tt(n);
    let x = synth::hp(&synth::noise(40 + seed, n), 7500.0, FILTER_O);
    let dec = if open { 0.11 } else { 0.02 };
    x.iter()
        .zip(&t)
        .map(|(v, &t)| v * dmath::exp(-t / dec) * clip(t / 0.001, 0.0, 1.0))
        .collect()
}

/// `Bed.bass(m, dur)` before the cache: a filtered-saw disco bass, a plucky filter envelope (12 bands closing over
/// the note) on a two-voice saw and a sine sub.
pub fn bass(m: i32, dur: f64) -> Vec<f64> {
    let n = length((dur + 0.05) * SRF);
    let t = synth::tt(n);
    let f = synth::mtof(f64::from(m));
    let saw = synth::saw(f, n, 0.004, 2);
    let x: Vec<f64> = (0..n)
        .map(|i| saw[i] * 0.7 + dmath::sin(2.0 * PI * f * t[i]) * 0.6)
        .collect();
    let out = banded(&x, 12, 1024, |a, b| {
        300.0 + 1500.0 * dmath::exp(-((a + b) as f64 / 2.0 / SRF) / 0.07)
    });
    out.iter()
        .zip(&t)
        .map(|(o, &t)| {
            let rel = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.02) };
            o * (clip(t / 0.003, 0.0, 1.0) * rel)
        })
        .collect()
}

/// `Bed.stab(ch, 1.0, dur, lo)` before the cache: a staccato string stab, three-voice detuned saws on the voicing,
/// band-passed, a quick bite.
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn stab(harmony: &Harmony, ch: &str, dur: f64, lo: i32) -> Result<Vec<f64>, HarmonyError> {
    let ms = voicing_from(harmony, ch, lo)?;
    let n = length((dur + 0.12) * SRF);
    let t = synth::tt(n);
    let x = synth::bp(&saw_chord(&ms, n, 0.007), 350.0, 2600.0, FILTER_O);
    let tail = dmath::exp(-dur / 0.25);
    Ok(x.iter()
        .zip(&t)
        .map(|(v, &t)| {
            let shape = if t < dur {
                dmath::exp(-t / 0.25)
            } else {
                tail * dmath::exp(-(t - dur) / 0.05)
            };
            v * (clip(t / 0.006, 0.0, 1.0) * shape)
        })
        .collect())
}

/// `Bed.pad(ch, dur)` before the cache: a saw chord (the voicing from E3) whose low-pass breathes open and shut over
/// 16 bands, high-passed at 160 Hz.
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn pad(harmony: &Harmony, ch: &str, dur: f64) -> Result<Vec<f64>, HarmonyError> {
    let ms = voicing_from(harmony, ch, PAD_LO)?;
    let n = length((dur + 0.3) * SRF);
    let t = synth::tt(n);
    let x = saw_chord(&ms, n, 0.005);
    let out = banded(&x, 16, 2048, |a, b| {
        let u = (a + b) as f64 / 2.0 / n as f64;
        // `np.sin(np.pi * u) ** 2` on a numpy scalar: the power, not a product.
        500.0 + 700.0 * dmath::pow(dmath::sin(PI * u), 2.0)
    });
    let out = synth::hp(&out, 160.0, FILTER_O);
    Ok(out
        .iter()
        .zip(&t)
        .map(|(o, &t)| {
            let rel = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.12) };
            o * (clip(t / 0.25, 0.0, 1.0) * rel)
        })
        .collect())
}

/// `Bed.glint(m)` before the cache: a mirror-ball glint, `A.chime(int(m), 0.5) * 0.6`.
pub fn glint(m: i32) -> Vec<f64> {
    synth::chime(f64::from(m), 0.5).iter().map(|v| v * 0.6).collect()
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
        Harmony::new(&g, "A", scale("minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    /// As the kit's `voicings.json` gives them for #51's chords.
    #[test]
    fn the_rule_voices_the_disco_chords() {
        let h = harmony(&["Am", "F", "Dm", "E7", "G"]);
        let want: [(&str, &[i32]); 5] = [
            ("Am", &[57, 60, 64, 69]),
            ("F", &[57, 60, 65, 69]),
            ("Dm", &[57, 62, 65, 69]),
            ("E7", &[59, 62, 64, 68]),
            ("G", &[59, 62, 67]),
        ];
        for (ch, notes) in want {
            assert_eq!(voicing(&h, ch).unwrap(), notes, "{ch}");
        }
        assert_eq!(voicing_from(&h, "Am", PAD_LO).unwrap(), [52, 57, 60, 64]);
        assert_eq!((root(&h, "Am").unwrap(), root(&h, "E7").unwrap()), (33, 40));
        assert_eq!(voicing(&h, "F#m"), Err(HarmonyError::UnknownChord("F#m".into())));
    }

    #[test]
    fn instrument_lengths_follow_pythons_int() {
        assert_eq!((kick(false).len(), kick(true).len()), (15360, 24000));
        assert_eq!(clap(0).len(), 12000);
        assert_eq!((hat(true, 0).len(), hat(false, 0).len()), (15360, 3360));
        assert_eq!(bass(45, 0.2).len(), 12000);
        assert_eq!(glint(96).len(), 24000);
    }
}
