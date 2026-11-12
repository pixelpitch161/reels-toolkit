//! Theme: horror trap (`projects/ballfight/kit/moods/horror_trap.py`, #55 BASS CLARINET vs BASSOON). A half-time
//! minor trap beat with a haunted nursery on top and no woodwinds of its own (the two low reeds are the leads): a
//! long sub 808 on each bar's root that slides into the next root at a phrase end, a half-time kit (a tight kick on
//! 1 and a syncopated pickup, a clap and snare on 3, closed 8th hats that skitter into 16th and triplet rolls), a
//! detuned music box playing a 2-bar lullaby high up, creeping low strings swelling over every 2 bars, and reversed
//! swells into the big hits. Built for E minor (Em Em C B | Em Em Am B); any chord works (roots by rule).
//!
//! Two fighter moves change the bed, read from the run's events: inside LIGHTS OUT (`lights_out` to its eruption)
//! only the music box and the 808 play, both a little fuller; inside MOUNTAIN KING (`mountain` to `mountain_end`)
//! the 808 pumps the root on the beats and the hats follow the hops.
//!
//! Here: the theme's harmony helpers and its instruments as the kit computes them. The arrangement that reproduces
//! the kit's bed is [`HorrorTrapClassic`], in `classic.rs`; it also holds the kit's per-bed cache of sounds (keyed
//! loosely: a duration is `round(dur, 3)`, a clap's noise is its seed modulo 2, a hat's high-pass its seed modulo 3).
//!
//! **Voicings.** The mood has no method that voices one chord: its 808 plays [`root`] (a number), its hits' music
//! box chord is the harmony's own `chord_tones(88, 100, ch)`, and its lullaby is [`box_motif`], two bars over two
//! chords (proven for every chord the harmony parses by the theme's own test, from `voicings.json`).

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_core::py::py_max;
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip};

pub use self::classic::HorrorTrapClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options, no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "horror_trap",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(HorrorTrapClassic::new())),
};

/// #55's loop in E minor.
const LOOP: [&str; 8] = ["Em", "Em", "C", "B", "Em", "Em", "Am", "B"];

/// What the interface says about the horror trap.
pub static CARD: ThemeCard = ThemeCard {
    display: "Horror trap",
    text: "E minor half-time trap: a sliding sub 808, skittering hats, a detuned music-box lullaby, creeping low \
           strings and reversed swells into the hits. LIGHTS OUT drops it to the music box and the 808. Leaves the \
           leads to the fighters.",
    key: "E",
    scale: "minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The kit configs that play the horror trap (Table S4): their music as each `episode.py` sets it.
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_bassclarinet_vs_bassoon",
    fighters: ["BASS CLARINET", "BASSOON"],
    bpm: 140.0,
    key: "E",
    scale: "minor",
    intro: ["Em", "B"],
    chord_loop: &LOOP,
    tail: &["C", "B"],
}];

/// The music box's range: `T.chord_tones(88, 100, ch)` (E6 to E7).
pub const BOX_LO: i32 = 88;
pub const BOX_HI: i32 = 100;
/// `box_motif`'s fallback: `T.chord_tones(84, 100, ch)`.
const BOX_LO_FALLBACK: i32 = 84;
/// The strings' root: `T.root_above(ch, 40)`.
pub const STRINGS_LO: i32 = 40;

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// `Bed.root(ch)`: the 808's root, `T.root_above(ch, 24)` (C1 to B1).
///
/// # Errors
/// Where Python raises: `ch` does not parse.
pub fn root(harmony: &Harmony, ch: &str) -> Result<i32, HarmonyError> {
    harmony.root_above(ch, 24)
}

/// `(T.chord_tones(88, 100, ch) or T.chord_tones(84, 100, ch)) * 3)[:4]`: the music box's four tones of `ch`.
fn box_four(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    let mut a = harmony.chord_tones(BOX_LO, BOX_HI, ch)?;
    if a.is_empty() {
        a = harmony.chord_tones(BOX_LO_FALLBACK, BOX_HI, ch)?;
    }
    let n = a.len();
    Ok((0..4.min(3 * n)).map(|i| a[i % n]).collect())
}

/// `Bed.box_motif(ch, ch2)`: the music box's 2-bar lullaby over `ch` then `ch2`, 8 notes a bar, the chord's tones
/// from E6, with a semitone's sigh under each phrase end.
///
/// # Errors
/// Where Python raises: a chord is not one of the episode's chords. (Every chord has a tone from E6 to E7, so the
/// kit's `IndexError` on an empty list cannot happen; it is an error here too.)
pub fn box_motif(harmony: &Harmony, ch: &str, ch2: &str) -> Result<([i32; 8], [i32; 8]), HarmonyError> {
    let a = box_four(harmony, ch)?;
    let b = box_four(harmony, ch2)?;
    if a.len() < 4 || b.len() < 4 {
        return Err(HarmonyError::UnknownChord(format!(
            "{ch} / {ch2}: no music-box tone (Python's IndexError)"
        )));
    }
    let mut distinct = a.clone();
    distinct.sort_unstable();
    distinct.dedup();
    let six = if distinct.len() > 3 { a[3] } else { a[2] + 12 };
    let bar1 = [a[2], a[1], a[0], a[1], a[2], six, a[2], a[2] - 1];
    let bar2 = [b[1], b[0], b[1], b[2], b[1], b[0], b[0] - 1, b[0] - 2];
    Ok((bar1, bar2))
}

/// `np.where(t < dur, 1.0, np.exp(-(t - dur) / rel))`.
fn release(t: f64, dur: f64, rel: f64) -> f64 {
    if t < dur { 1.0 } else { dmath::exp(-(t - dur) / rel) }
}

/// `2 * np.pi * np.cumsum(f) / SR`.
fn phase(f: &[f64]) -> Vec<f64> {
    dsp::cumsum(f).iter().map(|&c| 2.0 * PI * c / SRF).collect()
}

/// `Bed.b808(m, dur, to, slide)` before the cache and its velocity: a sub 808, a sine with a pitch drop at the
/// attack, a long decay and a touch of saturation, low-passed at 90 Hz; `to` slides into that note over the last
/// `slide` seconds (the trap slide).
pub fn b808(m: i32, dur: f64, to: Option<i32>, slide: f64) -> Vec<f64> {
    let n = length((dur + 0.06) * SRF);
    let t = synth::tt(n);
    let f0 = synth::mtof(f64::from(m));
    let f: Vec<f64> = t
        .iter()
        .map(|&t| {
            let mut semis = 4.0 * dmath::exp(-t / 0.014);
            if let Some(to) = to {
                // `(to - m) * np.clip(...) ** 1.5`: a numpy power with no short cut
                let u = clip((t - (dur - slide)) / slide, 0.0, 1.0);
                semis += f64::from(to - m) * dmath::pow(u, 1.5);
            }
            f0 * dmath::pow(2.0, semis / 12.0)
        })
        .collect();
    let ph = phase(&f);
    let dec = py_max(0.25, dur * 0.9);
    let norm = dmath::np_tanh(1.1);
    let x: Vec<f64> = (0..n)
        .map(|i| {
            let t = t[i];
            let env = clip(t / 0.003, 0.0, 1.0) * dmath::exp(-t / dec) * release(t, dur, 0.02);
            dmath::np_tanh(1.1 * dmath::sin(ph[i]) * env) / norm
        })
        .collect();
    synth::lp(&x, 90.0, FILTER_O)
}

/// `Bed.kick(1.0)` before the cache: `A.hp(A.kick(1.0, 0.06, 170, 48)[:int(0.12 * SR)], 40)`.
pub fn kick() -> Vec<f64> {
    let k = synth::kick(1.0, 0.06, 170.0, 48.0);
    let n = length(0.12 * SRF).min(k.len());
    synth::hp(&k[..n], 40.0, FILTER_O)
}

/// `Bed.clap(1.0, seed)` before the cache: a trap clap layered with a tight snare, three quick band-passed bursts
/// of `140 + seed`'s noise (every draw from the one generator in the kit's order) and the snare high-passed above
/// the growl.
pub fn clap(seed: u64) -> Vec<f64> {
    let n = length(0.32 * SRF);
    let t = synth::tt(n);
    let mut r = Generator::new(140 + seed);
    let mut x = vec![0.0; n];
    for (j, d) in [0.0, 0.009, 0.019].into_iter().enumerate() {
        let a = length(d * SRF);
        let mut z = vec![0.0; n - a];
        r.fill_normal(0.0, 1.0, &mut z);
        let burst = synth::bp(&z, 900.0, 5000.0, FILTER_O);
        let dec = if j < 2 { 0.006 } else { 0.09 };
        // `A.tt(n - i0)` starts again from 0 at the burst's first sample.
        for (i, (o, v)) in x[a..].iter_mut().zip(&burst).enumerate() {
            *o += v * dmath::exp(-t[i] / dec);
        }
    }
    // `A.hp(A.snare(0.10), 450)[:n]`, padded to `n`
    let sn = synth::hp(&synth::snare(0.10), 450.0, FILTER_O);
    (0..n)
        .map(|i| x[i] * 0.9 + sn.get(i).copied().unwrap_or(0.0) * 0.5)
        .collect()
}

/// `Bed.hat(1.0, open_, seed)` before the cache: `A.hp(A.hat(0.18 if open_ else 0.022, open_), 7000 + 400 * (seed
/// % 3))`.
pub fn hat(open: bool, seed: u64) -> Vec<f64> {
    let h = synth::hat(if open { 0.18 } else { 0.022 }, open);
    synth::hp(&h, f64::from(7000 + 400 * (seed % 3) as u32), FILTER_O)
}

/// `Bed.tine(m, dur, 1.0, det)` before the cache: a music-box tine, a bright pluck with an inharmonic partial, two
/// tines `det` semitones apart (the beating detune) under a slow warped-tape wobble, high-passed at 300 Hz.
pub fn tine(m: i32, dur: f64, det: f64) -> Vec<f64> {
    let n = length((dur + 0.1) * SRF);
    let t = synth::tt(n);
    let wob: Vec<f64> = t
        .iter()
        .map(|&t| 0.06 * dmath::sin(2.0 * PI * 0.7 * t) + 0.03 * dmath::sin(2.0 * PI * 3.1 * t))
        .collect();
    let mut x = vec![0.0; n];
    for (d, g) in [(0.0, 1.0), (det, 0.7)] {
        let fm = synth::mtof(f64::from(m) + d);
        let f: Vec<f64> = wob.iter().map(|w| fm * dmath::pow(2.0, w / 12.0)).collect();
        let ph = phase(&f);
        for i in 0..n {
            let p = ph[i];
            let v = dmath::sin(p) + 0.30 * dmath::sin(5.93 * p) * dmath::exp(-t[i] / 0.05) + 0.12 * dmath::sin(2.0 * p);
            x[i] += g * v;
        }
    }
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let t = t[i];
            let env = clip(t / 0.0015, 0.0, 1.0) * dmath::exp(-t / 0.42) * release(t, dur, 0.05);
            x[i] * env / 1.7
        })
        .collect();
    synth::hp(&y, 300.0, FILTER_O)
}

/// `Bed.strings(ms, dur, trem)` before the cache: creeping low strings, three-voice detuned saws on `ms`, a dark
/// low-pass at 700 Hz and a slow swell (sul tasto); bowed tremolo when `trem`.
pub fn strings(ms: &[i32], dur: f64, trem: bool) -> Vec<f64> {
    let n = length((dur + 0.25) * SRF);
    let t = synth::tt(n);
    // Python's `sum(...)` starts from 0.
    let mut x = vec![0.0; n];
    for &m in ms {
        for (v, s) in x.iter_mut().zip(synth::saw(synth::mtof(f64::from(m)), n, 0.005, 3)) {
            *v += s;
        }
    }
    let k = ms.len().max(1) as f64;
    let x: Vec<f64> = x.iter().map(|v| v / k).collect();
    let x = synth::lp(&x, 700.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            // `np.clip(...) ** 1.4` on an array: numpy's power, no short cut for 1.4
            let mut env = dmath::pow(clip(t / (dur * 0.7), 0.0, 1.0), 1.4) * release(t, dur, 0.12);
            if trem {
                env *= 0.55 + 0.45 * dmath::sin(2.0 * PI * 6.5 * t).abs();
            }
            x[i] * env
        })
        .collect()
}

/// `Bed.rev_swell(dur, m)` before the cache: a reversed crash (and the reversed music-box tine `tine`, the bed's
/// `self.tine(m, dur, 1.0, det=0.25)`, when `m` is given), opening up as it rises, cut dead at the end.
pub fn rev_swell(dur: f64, tine: Option<&[f64]>) -> Vec<f64> {
    let n = length(dur * SRF);
    let crash = synth::crash(py_max(1.2, dur + 0.3));
    // `np.pad(x[:n], (0, n - len(x[:n])))[::-1]`
    let reversed = |x: &[f64]| -> Vec<f64> { (0..n).rev().map(|i| x.get(i).copied().unwrap_or(0.0)).collect() };
    let mut c = reversed(&crash);
    if let Some(tn) = tine {
        c = c.iter().zip(reversed(tn)).map(|(c, s)| c * 0.8 + s * 0.6).collect();
    }
    let t = synth::tt(n);
    let over = py_max(dur, 1e-3);
    let ramp = 0.004 * SRF;
    (0..n)
        .map(|i| {
            let fade = clip((n - i) as f64 / ramp, 0.0, 1.0);
            c[i] * dmath::pow(t[i] / over, 1.5) * fade
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};

    use super::*;

    fn harmony(chords: &[&str]) -> Harmony {
        let g = Grid::new(140.0);
        let all: Vec<String> = chords.iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2.min(all.len())], &all, &[], g.n_bars);
        Harmony::new(&g, "E", scale("minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    #[test]
    fn the_lullaby_and_the_roots_follow_the_rule() {
        let h = harmony(&["Em", "B", "C", "Am"]);
        // Em from E6: 88, 91, 95, 100 (four tones: the sixth note is a[3]); B: 90, 95, 99, then 90 again
        let (bar1, bar2) = box_motif(&h, "Em", "B").unwrap();
        assert_eq!(bar1, [95, 91, 88, 91, 95, 100, 95, 94]);
        assert_eq!(bar2, [95, 90, 95, 99, 95, 90, 89, 88]);
        // B's three tones repeat, so its sixth note is a[2] + 12
        assert_eq!(box_motif(&h, "B", "Em").unwrap().0, [99, 95, 90, 95, 99, 111, 99, 98]);
        let roots = (root(&h, "Em").unwrap(), root(&h, "B").unwrap(), root(&h, "C").unwrap());
        assert_eq!(roots, (28, 35, 24));
        assert!(box_motif(&h, "F#m", "Em").is_err());
    }

    #[test]
    fn instrument_lengths_follow_pythons_int() {
        assert_eq!(kick().len(), 5760);
        assert_eq!(clap(0).len(), 15360);
        assert_eq!((hat(true, 0).len(), hat(false, 2).len()), (19200, 5760));
        assert_eq!(b808(28, 0.5, None, 0.09).len(), 26880);
        assert_eq!(tine(88, 0.8, 0.18).len(), 43200);
        assert_eq!(strings(&[40, 47, 53], 1.0, false).len(), 60000);
        assert_eq!(rev_swell(0.5, None).len(), 24000);
    }
}
