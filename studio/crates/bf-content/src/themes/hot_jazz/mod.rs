//! Theme: dark hot jazz (`projects/ballfight/kit/moods/hot_jazz.py`, #46 Trumpet vs Clarinet). A fast minor
//! two-beat from a 1920s hot band with no horns and no clarinet of its own (the fighters are the front line): the
//! tuba on 1 and 3 (root, then the fifth: the TUBA's own Pixel Pitch voice), a rhythm guitar chopping the chord on 2
//! and 4 (a muted plucked strum, the dominant's flat ninth on top), brushed cymbals on every beat with a light swung
//! skip, a felt bass drum on 1 and 3, brush taps on 2 and 4 that become snare press rolls in the back half, a press
//! roll into every phrase, woodblock clicks on the off-beats and crashes on the phrase lines. Built for D minor
//! (Dm Gm Bb7 A7); any chord works (voicings by rule).
//!
//! Here: the theme's harmony helpers and its instruments as the kit computes them. The arrangement that reproduces
//! the kit's bed is [`HotJazzClassic`], in `classic.rs`; it also holds the kit's per-bed cache of sounds (the
//! guitar's chop and strum are built there from the cached plucks, as the kit builds them).
//!
//! **Voicings.** The mood has no table: [`voicing`] is its rule for every chord the harmony parses (the root low,
//! the third, the fifth, the seventh or the doubled root; a dominant seventh's flat ninth in place of the fifth),
//! each the lowest such note from D3 up that no earlier tone took (else an octave up), in rising order.
//!
//! **The steel string.** `pluck` runs a low-passed burst through `scipy.signal.lfilter([1.0], a, exc)` with
//! `a = [1, 0, ..., 0, -ring / 2, -ring / 2]`, the same Karplus-Strong loop as the bossa noir's nylon strings, so it
//! uses that theme's [`karplus_strong`](super::bossa_noir::karplus_strong) (scipy's bits) and
//! [`loop_length`](super::bossa_noir::loop_length).
//!
//! **The tuba.** The bed plays the TUBA's voice itself, `M.note('TUBA', ...)`: the theme lists `TUBA` in its
//! `voices`, so the build hands the ported TUBA voice (`fighters::tuba`) to the music context.

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_core::py::py_max;
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::synth::{self, FILTER_O, clip, noise};

pub use self::classic::HotJazzClassic;
use super::bossa_noir::{karplus_strong, loop_length};
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options; its bed plays the TUBA's voice.
pub static THEME: Theme = Theme {
    name: "hot_jazz",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &["TUBA"],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(HotJazzClassic::new())),
};

/// #46's loop: i - iv - bVI7 - V7 in D minor.
const LOOP: [&str; 8] = ["Dm", "Dm", "Gm", "Gm", "Bb7", "A7", "Dm", "A7"];

/// What the interface says about the hot jazz.
pub static CARD: ThemeCard = ThemeCard {
    display: "Hot jazz",
    text: "A fast, dark minor two-beat from a 1920s hot band: tuba on 1 and 3, a rhythm guitar chopping 2 and 4, \
           brushes and snare press rolls. Leaves the front line to the fighters.",
    key: "D",
    scale: "harmonic minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The kit configs that play the hot jazz (Table S4): their music as each `episode.py` sets it.
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_trumpet_vs_clarinet",
    fighters: ["TRUMPET", "CLARINET"],
    bpm: 140.0,
    key: "D",
    scale: "harmonic minor",
    intro: ["Dm", "A7"],
    chord_loop: &LOOP,
    tail: &["Bb7", "A7"],
}];

/// `SWING`: the off-beat 8th at 58 % of the beat (a light swing at a fast two-beat).
pub const SWING: f64 = 0.58;

/// `pluck`'s default `bright` and `ring`.
pub const PLUCK_BRIGHT: f64 = 0.55;
pub const PLUCK_RING: f64 = 0.994;
/// `chop`'s default `dur`.
pub const CHOP_DUR: f64 = 0.15;
/// `tuba`'s default velocity.
pub const TUBA_VEL: f64 = 0.9;

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// `Bed.root(ch)`: the tuba's root, `T.root_above(ch, 36)` (C2 to B2).
///
/// # Errors
/// Where Python raises: `ch` does not parse.
pub fn root(harmony: &Harmony, ch: &str) -> Result<i32, HarmonyError> {
    harmony.root_above(ch, 36)
}

/// `Bed.fifth(ch)`: the chord's fifth above the tuba's root (a perfect fifth for a chord of fewer than three
/// tones), an octave down when it would be above D3.
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
    Ok(if f <= 50 { f } else { f - 12 })
}

/// `Bed.voicing(ch)`: the guitar's four-note chop (D3 to F4): the root low, the third and the fifth, the seventh
/// (or the doubled root) on top; a chord with a major third and a minor seventh takes its flat ninth in place of the
/// fifth. Each tone is the lowest such note from D3 (MIDI 50) up that no earlier tone took (else an octave up); the
/// notes in rising order.
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn voicing(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    let pcs = harmony.chord_pcs(ch)?;
    let r = pcs[0];
    let mut tones = vec![r, pcs[1], pcs[2], if pcs.len() > 3 { pcs[3] } else { r }];
    if pcs.len() > 3 && (pcs[1] - r).rem_euclid(12) == 4 && (pcs[3] - r).rem_euclid(12) == 10 {
        tones[2] = (r + 1).rem_euclid(12);
    }
    let mut out: Vec<i32> = Vec::new();
    for pc in tones {
        let mut m = 50 + (pc - 50).rem_euclid(12);
        while out.contains(&m) {
            m += 12;
        }
        out.push(m);
    }
    out.sort_unstable();
    Ok(out)
}

/// `Bed.pluck(m, dur, seed, bright, ring)` before the cache: a plucked steel string (Karplus-Strong: a filtered
/// noise burst through a tuned delay loop). The burst is `N` normals from a new generator of `1000 + m + 37 * seed`,
/// low-passed once at `1500 + 5000 * bright` Hz.
///
/// # Panics
/// Where Python raises: the note is shorter than its loop (`exc[:N] = ...` would not fit), or the seed is negative.
pub fn pluck(m: i32, dur: f64, seed: i64, bright: f64, ring: f64) -> Vec<f64> {
    let period = loop_length(m);
    let n = length(dur * SRF);
    assert!(n >= period, "pluck: {n} samples hold no loop of {period}");
    let burst_seed = 1000 + i64::from(m) + 37 * seed;
    let burst = noise(u64::try_from(burst_seed).expect("a noise seed"), period);
    let mut exc = vec![0.0; n];
    exc[..period].copy_from_slice(&synth::lp(&burst, 1500.0 + 5000.0 * bright, FILTER_O));
    karplus_strong(&exc, period, -0.5 * ring)
}

/// `Bed.kick(1.0)` before the cache and its velocity: a felt bass drum, soft, high-passed so the tuba stays on top:
/// `A.hp(A.kick(0.55, 0.22, 110, 52), 42)[:int(0.32 * SR)]`.
pub fn kick_felt() -> Vec<f64> {
    let x = synth::hp(&synth::kick(0.55, 0.22, 110.0, 52.0), 42.0, FILTER_O);
    x[..length(0.32 * SRF)].to_vec()
}

/// The brushed cymbal's partials, in Hz.
const CYMBAL_PARTS: [f64; 6] = [512.0, 1187.0, 2411.0, 3631.0, 5093.0, 6911.0];

/// `Bed.cymbal(1.0, seed)` before the cache: a brushed cymbal, six partials (Python's `sum` from 0, the `i`-th at
/// phase `i` and level `0.88 ** i`, over their count) under a soft swish of `31 + seed`'s noise, no stick ping.
pub fn cymbal(seed: u64) -> Vec<f64> {
    let n = length(0.9 * SRF);
    let t = synth::tt(n);
    let mut x = vec![0.0f64; n];
    for (i, &f) in CYMBAL_PARTS.iter().enumerate() {
        let level = dmath::pow(0.88, i as f64);
        for (o, &t) in x.iter_mut().zip(&t) {
            *o += dmath::sin(2.0 * PI * f * t + i as f64) * level;
        }
    }
    let k = CYMBAL_PARTS.len() as f64;
    let swish = synth::bp(&noise(31 + seed, n), 3500.0, 12000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let x = x[i] / k * dmath::exp(-t / 0.42);
            let swish = swish[i] * (clip(t / 0.012, 0.0, 1.0) * dmath::exp(-t / 0.10));
            (x * 0.8 + swish * 0.7) * clip(t / 0.003, 0.0, 1.0)
        })
        .collect()
}

/// `Bed.brush(1.0, seed)` before the cache: a brush tap on the snare, from `40 + seed`'s noise.
pub fn brush(seed: u64) -> Vec<f64> {
    let n = length(0.2 * SRF);
    let t = synth::tt(n);
    let x = synth::bp(&noise(40 + seed, n), 1600.0, 7500.0, FILTER_O);
    x.iter()
        .zip(&t)
        .map(|(v, &t)| v * clip(t / 0.005, 0.0, 1.0) * dmath::exp(-t / 0.06))
        .collect()
}

/// `Bed.press(dur, v, seed, v0)` before the cache and its last `* v`: a snare press roll, a buzz of tiny bounces
/// (a rattle at about 28 Hz on the snares) with the drum's body under it. One generator of `60 + seed` draws the
/// buzz's `n` normals, then the rattle's phase (`uniform(0, 3)`). With `v0`, a crescendo from `v0` to `v`.
pub fn press(dur: f64, v: f64, seed: u64, v0: Option<f64>) -> Vec<f64> {
    let n = length((dur + 0.08) * SRF);
    let t = synth::tt(n);
    let mut rng = Generator::new(60 + seed);
    let mut raw = vec![0.0; n];
    rng.fill_normal(0.0, 1.0, &mut raw);
    let buzz = synth::bp(&raw, 1500.0, 8500.0, FILTER_O);
    let phase = rng.uniform(0.0, 3.0);
    let ratio = v0.map(|v0| v0 / py_max(v, 1e-6));
    (0..n)
        .map(|i| {
            let t = t[i];
            let rattle = 0.55 + 0.45 * dmath::sin(PI * 28.0 * t + phase).abs();
            let body = dmath::sin(2.0 * PI * 196.0 * t) * dmath::exp(-t / 0.05) * 0.5;
            let rel = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.03) };
            let env = clip(t / 0.004, 0.0, 1.0) * rel;
            let env = match ratio {
                None => env * (0.55 + 0.45 * dmath::exp(-t / 0.06)),
                Some(r) => env * (r + (1.0 - r) * dmath::pow(clip(t / dur, 0.0, 1.05), 1.4)),
            };
            (buzz[i] * rattle + body) * env
        })
        .collect()
}

/// `Bed.woodblock(1.0, hi)` before the cache: a woodblock click, high (1850 Hz) or low (1320 Hz).
pub fn woodblock(hi: bool) -> Vec<f64> {
    let n = length(0.09 * SRF);
    let t = synth::tt(n);
    let f = if hi { 1850.0 } else { 1320.0 };
    t.iter()
        .map(|&t| {
            let x = dmath::sin(2.0 * PI * f * t) + 0.4 * dmath::sin(2.0 * PI * f * 2.7 * t) * dmath::exp(-t / 0.008);
            x * dmath::exp(-t / 0.022) * clip(t / 0.001, 0.0, 1.0)
        })
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

    use super::*;

    fn harmony(chords: &[&str]) -> Harmony {
        let g = Grid::new(140.0);
        let all: Vec<String> = chords.iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2.min(all.len())], &all, &[], g.n_bars);
        Harmony::new(&g, "D", scale("harmonic minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    /// The rule on #46's chords: the A7 and Bb7 take their flat ninths in place of the fifth.
    #[test]
    fn the_rule_voices_the_hot_jazz_chords() {
        let h = harmony(&["Dm", "Gm", "Bb7", "A7"]);
        assert_eq!(voicing(&h, "Dm").unwrap(), [50, 53, 57, 62]);
        assert_eq!(voicing(&h, "Gm").unwrap(), [50, 55, 58, 67]);
        assert_eq!(voicing(&h, "Bb7").unwrap(), [50, 56, 58, 59]);
        assert_eq!(voicing(&h, "A7").unwrap(), [55, 57, 58, 61]);
        assert_eq!((root(&h, "Dm").unwrap(), fifth(&h, "Dm").unwrap()), (38, 45));
        assert_eq!((root(&h, "Bb7").unwrap(), fifth(&h, "Bb7").unwrap()), (46, 41));
        assert_eq!(voicing(&h, "F#m"), Err(HarmonyError::UnknownChord("F#m".into())));
    }

    #[test]
    fn instrument_lengths_follow_pythons_int() {
        assert_eq!(pluck(50, 0.2, 0, PLUCK_BRIGHT, PLUCK_RING).len(), 9600);
        assert_eq!(kick_felt().len(), 15360);
        assert_eq!((cymbal(0).len(), brush(0).len()), (43200, 9600));
        assert_eq!(press(0.2, 1.0, 0, None).len(), 13440);
        assert_eq!((woodblock(true).len(), choke(1.0, 0).len()), (4320, 11520));
    }

    /// A crescendo roll and a plain one differ; both are the same length.
    #[test]
    fn a_crescendo_press_roll_and_a_plain_one_differ() {
        assert_ne!(press(0.2, 1.0, 3, None), press(0.2, 1.0, 3, Some(0.35)));
        assert_eq!(press(0.2, 1.0, 3, Some(1.0)).len(), press(0.2, 1.0, 3, None).len());
    }
}
