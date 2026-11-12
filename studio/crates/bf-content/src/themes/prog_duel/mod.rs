//! Theme: prog duel (`projects/ballfight/kit/moods/prog_duel.py`, #45 PIANO vs ORGAN). A dramatic minor 70s-prog
//! groove with no keyboards of its own (the two fighters are the keyboards): big drums (a deep kick, a fat backbeat
//! snare, toms, crashes, hats) and a driving, slightly overdriven bass guitar picking 8ths. Every other bar of the
//! loop is a 3+3+2 push (kick, bass accents and an octave jump on 8ths 1, 4 and 7) inside the 4/4 bar; tom fills close
//! each 4-bar phrase; the back half adds 16th hats, a ride on the pushes and heavier toms. Built for A minor
//! (Am F G E7 Am F Dm E7); any chord works (the bass plays each bar's root, fifth and octave).
//!
//! Here: the theme's pitch rules and its instruments as the kit computes them. The arrangement that reproduces the
//! kit's bed is [`ProgDuelClassic`], in `classic.rs`; it also holds the kit's per-bed cache of sounds (keyed
//! loosely: a snare's noise is its seed modulo 4, a bass note's duration, velocity and drive are rounded).
//!
//! **Voicings.** The mood voices no chord: its only pitch rules are [`root`] (`T.root_above(ch, 28)`) and [`fifth`]
//! (the root and a fifth), so the theme suite passes no voicing function and its `voicings.json` lists none.
//!
//! **No `hit`.** The class defines `boom` (a kick and the low floor tom) and no other optional hook.

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip};

pub use self::classic::ProgDuelClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options, no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "prog_duel",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(ProgDuelClassic::new())),
};

/// #45's loop in A minor.
const LOOP: [&str; 8] = ["Am", "F", "G", "E7", "Am", "F", "Dm", "E7"];

/// What the interface says about the prog duel.
pub static CARD: ThemeCard = ThemeCard {
    display: "Prog duel",
    text: "A dramatic 70s prog groove: big drums with 3+3+2 pushes and tom fills, and a driving overdriven bass \
           guitar. No keyboards of its own: it leaves the leads to the fighters.",
    key: "A",
    scale: "minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The kit configs that play the prog duel (Table S4): their music as each `episode.py` sets it.
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_piano_vs_organ",
    fighters: ["PIANO", "ORGAN"],
    bpm: 132.0,
    key: "A",
    scale: "minor",
    intro: ["Am", "E7"],
    chord_loop: &LOOP,
    tail: &[],
}];

/// The toms of a fill, high to low (`Bed.fill`), and of a heavy fill.
pub const TOMS: [i32; 4] = [196, 165, 131, 98];
pub const HEAVY_TOMS: [i32; 4] = [175, 147, 117, 87];

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// `r.normal(0, 1, n)` on a generator already in use.
fn normals(r: &mut Generator, n: usize) -> Vec<f64> {
    let mut x = vec![0.0; n];
    r.fill_normal(0.0, 1.0, &mut x);
    x
}

/// A noise seed from a MIDI note or a tom's frequency (`default_rng(int(m))`).
///
/// # Panics
/// On a negative number (`default_rng` refuses a negative seed; no rule of the mood gives one).
fn seed_of(m: i32) -> u64 {
    u64::try_from(m).expect("a seed of 0 or more")
}

/// `Bed.root(ch)`: the bass guitar's root, `T.root_above(ch, 28)` (E1 to D#2).
///
/// # Errors
/// Where Python raises: `ch` does not parse.
pub fn root(harmony: &Harmony, ch: &str) -> Result<i32, HarmonyError> {
    harmony.root_above(ch, 28)
}

/// `Bed.fifth(ch)`: the root and a perfect fifth.
///
/// # Errors
/// Where Python raises: `ch` does not parse.
pub fn fifth(harmony: &Harmony, ch: &str) -> Result<i32, HarmonyError> {
    Ok(root(harmony, ch)? + 7)
}

/// `Bed.kick()` before the cache and its velocity: `A.kick(1.0, 0.24, 150, 46)[:int(0.4 * SR)]` with a click of
/// seed 31's band-passed noise, high-passed at 34 Hz.
pub fn kick() -> Vec<f64> {
    let mut x = synth::kick(1.0, 0.24, 150.0, 46.0);
    x.truncate(length(0.4 * SRF));
    let n = x.len();
    let t = synth::tt(n);
    let click = synth::bp(&synth::noise(31, n), 2500.0, 7000.0, FILTER_O);
    let y: Vec<f64> = (0..n)
        .map(|i| x[i] + click[i] * dmath::exp(-t[i] / 0.004) * 0.35)
        .collect();
    synth::hp(&y, 34.0, FILTER_O)
}

/// `Bed.snare(1.0, seed)` before the cache: wires and a crack from one generator of `40 + seed % 4` (the wires
/// drawn first), a 185 Hz body, through a `tanh`.
pub fn snare(seed: u64) -> Vec<f64> {
    let n = length(0.5 * SRF);
    let t = synth::tt(n);
    let mut r = Generator::new(40 + seed % 4);
    let wires = synth::bp(&normals(&mut r, n), 1800.0, 9000.0, FILTER_O);
    let crack = synth::bp(&normals(&mut r, n), 3000.0, 6000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let wires = wires[i] * (dmath::exp(-t / 0.09) * 0.8 + dmath::exp(-t / 0.25) * 0.12);
            let body = dmath::sin(2.0 * PI * 185.0 * t + 2.0 * dmath::exp(-t / 0.01)) * dmath::exp(-t / 0.07) * 0.7;
            let crack = crack[i] * dmath::exp(-t / 0.006) * 0.6;
            dmath::np_tanh((wires + body + crack) * 1.4) * 0.8
        })
        .collect()
}

/// `Bed.hat(1.0, open_)` (its seed is unused): `A.hat(0.03, open_)`.
pub fn hat(open: bool) -> Vec<f64> {
    synth::hat(0.03, open)
}

/// `Bed.tom(f)` before the cache and its velocity: a sine falling onto `f` Hz from half as high again, and a thump
/// of `int(f)`'s band-passed noise.
pub fn tom(f: i32) -> Vec<f64> {
    let n = length(0.55 * SRF);
    let t = synth::tt(n);
    let ff = f64::from(f);
    let fr: Vec<f64> = t.iter().map(|&t| ff * (1.0 + 0.5 * dmath::exp(-t / 0.03))).collect();
    let thump = synth::bp(&synth::noise(seed_of(f), n), 200.0, 2500.0, FILTER_O);
    dsp::cumsum(&fr)
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let t = t[i];
            let ph = 2.0 * PI * c / SRF;
            let x = dmath::sin(ph) * dmath::exp(-t / 0.22) + 0.2 * thump[i] * dmath::exp(-t / 0.02);
            x * 0.9
        })
        .collect()
}

/// `Bed.ride(1.0)` (the kit makes it again each call; it is the same every time): three bell partials and seed 55's
/// high-passed noise.
pub fn ride() -> Vec<f64> {
    let n = length(0.6 * SRF);
    let t = synth::tt(n);
    let hiss = synth::hp(&synth::noise(55, n), 6000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            // Python's `sum` starts from 0.
            let mut bell = 0.0;
            for f in [3150.0, 4420.0, 5870.0] {
                bell += dmath::sin(2.0 * PI * f * t);
            }
            let x = bell * dmath::exp(-t / 0.35) * 0.15;
            x + hiss[i] * dmath::exp(-t / 0.08) * 0.2
        })
        .collect()
}

/// `Bed.bass_note(m, dur, vel, drive)` before the cache (the kit caches it with its velocity): a picked electric
/// bass, four harmonics under a pluck envelope with a pick click of `int(m)`'s noise, overdriven through a `tanh`
/// and low-passed at 1400 Hz.
///
/// # Panics
/// On a negative note (`default_rng` refuses a negative seed; no rule of the mood gives one).
pub fn bass_note(m: i32, dur: f64, vel: f64, drive: f64) -> Vec<f64> {
    let n = length((dur + 0.05) * SRF);
    let t = synth::tt(n);
    let f = synth::mtof(f64::from(m));
    let click = synth::bp(&synth::noise(seed_of(m), n), 1500.0, 4000.0, FILTER_O);
    let k = 1.3 + 0.7 * drive;
    let norm = dmath::np_tanh(k);
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let t = t[i];
            let ph = 2.0 * PI * f * t;
            let x = dmath::sin(ph)
                + 0.55 * dmath::sin(2.0 * ph) * dmath::exp(-t / 0.25)
                + 0.30 * dmath::sin(3.0 * ph) * dmath::exp(-t / 0.12)
                + 0.15 * dmath::sin(4.0 * ph) * dmath::exp(-t / 0.07);
            // `x *= np.exp(...) * np.clip(...) * np.clip(...)`: the right side first
            let env = dmath::exp(-t / 0.9) * clip(t / 0.003, 0.0, 1.0) * clip((dur + 0.05 - t) / 0.04, 0.0, 1.0);
            let x = x * env;
            let click = click[i] * dmath::exp(-t / 0.004) * 0.15;
            dmath::np_tanh((x + click) * k) / norm
        })
        .collect();
    synth::lp(&y, 1400.0, FILTER_O).iter().map(|v| v * vel).collect()
}

/// `Bed.vs`'s sound before its gain: a crack of seed 91's band-passed noise over a rumble of seed 92's low-passed
/// noise.
pub fn vs_crack() -> Vec<f64> {
    let n = length(1.4 * SRF);
    let t = synth::tt(n);
    let crack = synth::bp(&synth::noise(91, n), 1500.0, 9000.0, FILTER_O);
    let rumble = synth::lp(&synth::noise(92, n), 160.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let crack = crack[i] * dmath::exp(-t / 0.035);
            let rumble = rumble[i] * clip(t / 0.02, 0.0, 1.0) * dmath::exp(-t / 0.5) * 3.0;
            crack * 0.9 + rumble
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};

    use super::*;

    fn harmony(chords: &[&str]) -> Harmony {
        let g = Grid::new(132.0);
        let all: Vec<String> = chords.iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2.min(all.len())], &all, &[], g.n_bars);
        Harmony::new(&g, "A", scale("minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    #[test]
    fn the_bass_plays_roots_from_e1_up() {
        let h = harmony(&["Am", "E7", "F", "G", "Dm"]);
        assert_eq!(root(&h, "Am").unwrap(), 33);
        assert_eq!(root(&h, "E7").unwrap(), 28);
        assert_eq!((root(&h, "F").unwrap(), root(&h, "G").unwrap()), (29, 31));
        assert_eq!((root(&h, "Dm").unwrap(), fifth(&h, "Dm").unwrap()), (38, 45));
        assert!(root(&h, "Q").is_err());
    }

    #[test]
    fn instrument_lengths_follow_pythons_int() {
        assert_eq!(kick().len(), 19200);
        assert_eq!((snare(5).len(), tom(87).len(), ride().len()), (24000, 26400, 28800));
        assert_eq!((hat(false).len(), hat(true).len()), (5760, 19200));
        assert_eq!(bass_note(33, 0.2, 1.0, 1.0).len(), 12000);
        assert_eq!(vs_crack().len(), 67200);
    }

    #[test]
    fn a_snare_is_keyed_by_its_seed_modulo_4() {
        assert_eq!(snare(1), snare(5));
        assert_ne!(snare(1), snare(2));
    }
}
