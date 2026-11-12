//! Theme: big-band mambo (`projects/ballfight/kit/moods/mambo.py`, #50 Trumpet vs Trombone). A minor Latin big-band
//! groove with no trumpets and no trombones of its own (the fighters are the brass section): the piano's montuno (a
//! two-bar syncopated figure of the chord in octaves, the PIANO's Pixel Pitch voice), the contrabass tumbao (the
//! fifth on the and of 2, the next chord's root on 4, held over the bar line: the CONTRABASS's voice), congas,
//! timbales (the cascara on the shell, the mambo bell in the back half, rimshots and an abanico roll into every
//! phrase, and a fill into every ability the fighters fire), a cowbell, quiet 2-3 claves, and an alto-sax section
//! pad kept low (the old Pixel Pitch alto's voice). Built for G minor (Gm Cm D7 Eb7); any chord works (voicings by
//! rule).
//!
//! Here: the theme's harmony helpers and its synthesised percussion as the kit computes them. The arrangement that
//! reproduces the kit's bed is [`MamboClassic`], in `classic.rs`; it also holds the kit's per-bed cache of sounds
//! (the sax pad is built there from the alto's notes, as the kit builds it).
//!
//! **Voicings.** The mood has no table: [`tones`] (the chord's tones from G3 up, one octave, rising) is its rule
//! for every chord the harmony parses; the montuno takes them from G4, the pad from A3.
//!
//! **The voices.** The bed plays three Pixel Pitch voices itself through `M.note`: the CONTRABASS's and the PIANO's
//! (from their merged ports, `fighters::contrabass` and `fighters::piano`) and the old alto's, whose `pp_bridge` id
//! is [`ALTO`] (`RETIRED[1]`, an internal id only: never shown). The alto is a voice of the theme, never a fighter
//! ([`alto`]); [`Theme::own_voice`] hands it to the build.

pub mod alto;
mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip, noise};

pub use self::classic::MamboClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The old alto's `pp_bridge` id, the voice of the sax pad: `RETIRED[1]`, an internal id (rulings R14, M3-R9).
pub const ALTO: &str = bf_core::fightfile::RETIRED[1];

/// The theme as the registry lists it: no options; its bed plays the CONTRABASS's and the PIANO's voices and the
/// old alto's, its own.
pub static THEME: Theme = Theme {
    name: "mambo",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &["CONTRABASS", "PIANO", ALTO],
    own_voice: alto::own_voice,
    classic: |_| Ok(Box::new(MamboClassic::new())),
};

/// #50's loop: i - iv - V7 - i - i - iv - bVI7 - V7 in G minor.
const LOOP: [&str; 8] = ["Gm", "Cm", "D7", "Gm", "Gm", "Cm", "Eb7", "D7"];

/// What the interface says about the mambo.
pub static CARD: ThemeCard = ThemeCard {
    display: "Mambo",
    text: "A dark big-band mambo: a piano montuno, a contrabass tumbao, congas, timbales, a cowbell and a low sax \
           section pad, with a timbale fill into every ability. Leaves the brass to the fighters.",
    key: "G",
    scale: "harmonic minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The kit configs that play the mambo (Table S4): their music as each `episode.py` sets it.
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_trumpet_vs_trombone",
    fighters: ["TRUMPET", "TROMBONE"],
    bpm: 132.0,
    key: "G",
    scale: "harmonic minor",
    intro: ["Gm", "D7"],
    chord_loop: &LOOP,
    tail: &[],
}];

/// `CASCARA`: the shell's 8ths over two bars.
pub const CASCARA: [bool; 16] = bits([1, 0, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 0, 1]);
/// `CLAVE`: the 2-3 son clave, 8ths over two bars.
pub const CLAVE: [bool; 16] = bits([0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0]);
/// `MONTUNO`: the piano's hits (8ths) in bars A and B.
pub const MONTUNO: [[usize; 5]; 2] = [[0, 2, 3, 5, 6], [1, 2, 4, 5, 7]];
/// `CONGA`: muff / slap / open over one bar of 8ths.
pub const CONGA: [Conga; 8] = [
    Conga::Muff,
    Conga::Muff,
    Conga::Slap,
    Conga::Muff,
    Conga::Muff,
    Conga::Muff,
    Conga::Open,
    Conga::Open,
];

const fn bits(x: [u8; 16]) -> [bool; 16] {
    let mut out = [false; 16];
    let mut i = 0;
    while i < 16 {
        out[i] = x[i] != 0;
        i += 1;
    }
    out
}

/// A conga stroke: the kit's `kind` (`'o'`, `'s'`, `'m'`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Conga {
    /// `'o'`: the open tone, a ringing skin.
    Open,
    /// `'s'`: the slap, a sharp crack.
    Slap,
    /// `'m'`: the muff, the heel-toe pat.
    Muff,
}

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// `np.sign(x)`: -1, 0 or 1 (a NaN stays NaN).
fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

/// `Bed.root(ch)`: the bass's root, `T.root_above(ch, 36)` (C2 to B2).
///
/// # Errors
/// Where Python raises: `ch` does not parse.
pub fn root(harmony: &Harmony, ch: &str) -> Result<i32, HarmonyError> {
    harmony.root_above(ch, 36)
}

/// `Bed.fifth(ch)`: the chord's fifth above the bass's root (a perfect fifth for a chord of fewer than three tones).
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn fifth(harmony: &Harmony, ch: &str) -> Result<i32, HarmonyError> {
    let r = root(harmony, ch)?;
    let pcs = harmony.chord_pcs(ch)?;
    Ok(r + if pcs.len() > 2 {
        (pcs[2] - pcs[0]).rem_euclid(12)
    } else {
        7
    })
}

/// `Bed.tones(ch, lo)`: the chord's tones from `lo` up, one octave (root, third, fifth, seventh if any), rising.
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn tones_from(harmony: &Harmony, ch: &str, lo: i32) -> Result<Vec<i32>, HarmonyError> {
    let mut out: Vec<i32> = harmony
        .chord_pcs(ch)?
        .iter()
        .map(|&pc| lo + (pc - lo).rem_euclid(12))
        .collect();
    out.sort_unstable();
    Ok(out)
}

/// `Bed.tones(ch)`: [`tones_from`] at Python's default floor, G3 (MIDI 55): the mood's voicing rule.
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn tones(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    tones_from(harmony, ch, 55)
}

/// `Bed.conga(kind, 1.0, low)` before the cache and its velocity: a conga (the tumba when `low`): a pitched skin
/// gliding down onto its note, with `300 + (kind == 's') + 2 * low`'s noise band-passed on the attack.
pub fn conga(kind: Conga, low: bool) -> Vec<f64> {
    let n = length(0.35 * SRF);
    let t = synth::tt(n);
    let base = if low { 168.0 } else { 232.0 };
    let f: Vec<f64> = t
        .iter()
        .map(|&t| base * (1.0 + 0.10 * dmath::exp(-t / 0.012)))
        .collect();
    let ph: Vec<f64> = dsp::cumsum(&f).iter().map(|&c| 2.0 * PI * c / SRF).collect();
    let skin: Vec<f64> = (0..n)
        .map(|i| dmath::sin(ph[i]) + 0.35 * dmath::sin(1.59 * ph[i]) * dmath::exp(-t[i] / 0.03))
        .collect();
    let seed = 300 + u64::from(kind == Conga::Slap) + 2 * u64::from(low);
    let mut rng = Generator::new(seed);
    let mut raw = vec![0.0; n];
    rng.fill_normal(0.0, 1.0, &mut raw);
    let (lo, hi) = match kind {
        Conga::Open => (600.0, 3000.0),
        Conga::Slap => (900.0, 6000.0),
        Conga::Muff => (400.0, 2000.0),
    };
    let nz = synth::bp(&raw, lo, hi, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let x = match kind {
                Conga::Open => skin[i] * dmath::exp(-t / 0.16) + 0.25 * nz[i] * dmath::exp(-t / 0.008),
                Conga::Slap => 0.55 * skin[i] * dmath::exp(-t / 0.035) + 1.1 * nz[i] * dmath::exp(-t / 0.018),
                Conga::Muff => 0.6 * skin[i] * dmath::exp(-t / 0.025) + 0.15 * nz[i] * dmath::exp(-t / 0.006),
            };
            x * clip(t / 0.0015, 0.0, 1.0)
        })
        .collect()
}

/// The timbale's partials: (ratio, level), summed from 0 in this order.
const TIMBALE_PARTS: [(f64, f64); 4] = [(1.0, 1.0), (1.48, 0.55), (2.13, 0.35), (2.9, 0.2)];

/// `Bed.timbale(1.0, hi, rim)` before the cache and its velocity: a bright metal-shell drum (520 Hz high, 390 Hz
/// low), ringing, with `410 + hi`'s noise as the stick's crack (a rimshot's is longer and louder).
pub fn timbale(hi: bool, rim: bool) -> Vec<f64> {
    let n = length(0.5 * SRF);
    let t = synth::tt(n);
    let f = if hi { 520.0 } else { 390.0 };
    let mut x = vec![0.0f64; n];
    for (k, a) in TIMBALE_PARTS {
        // np.sin(2 * np.pi * f * k * t + k) * a
        let w = 2.0 * PI * f * k;
        for (o, &t) in x.iter_mut().zip(&t) {
            *o += dmath::sin(w * t + k) * a;
        }
    }
    let crack = synth::bp(&noise(410 + u64::from(hi), n), 1800.0, 9000.0, FILTER_O);
    let (fall, level) = if rim { (0.02, 1.0) } else { (0.006, 0.35) };
    (0..n)
        .map(|i| {
            let t = t[i];
            let body = x[i] * dmath::exp(-t / 0.22) / 2.1;
            let crack = crack[i] * dmath::exp(-t / fall);
            (body + crack * level) * clip(t / 0.001, 0.0, 1.0)
        })
        .collect()
}

/// `Bed.shell(1.0)` before the cache and its velocity: the cascara, a stick on the timbale's metal shell (a dry
/// click of seed 420's noise with a little 1620 Hz ping).
pub fn shell() -> Vec<f64> {
    let n = length(0.08 * SRF);
    let t = synth::tt(n);
    let click = synth::bp(&noise(420, n), 2500.0, 8000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let ping = dmath::sin(2.0 * PI * 1620.0 * t) * dmath::exp(-t / 0.012);
            (0.6 * ping + click[i] * dmath::exp(-t / 0.006)) * clip(t / 0.0008, 0.0, 1.0)
        })
        .collect()
}

/// `Bed.bell(1.0, mouth)` before the cache and its velocity: the mambo bell (cowbell), two square partials (562 and
/// 838 Hz) band-passed, the mouth open and ringing or the heel damped.
pub fn bell(mouth: bool) -> Vec<f64> {
    let n = length(0.3 * SRF);
    let t = synth::tt(n);
    let x: Vec<f64> = t
        .iter()
        .map(|&t| sign(dmath::sin(2.0 * PI * 562.0 * t)) * 0.5 + sign(dmath::sin(2.0 * PI * 838.0 * t)) * 0.5)
        .collect();
    let y = synth::bp(&x, 450.0, 3200.0, FILTER_O);
    let fall = if mouth { 0.09 } else { 0.03 };
    (0..n)
        .map(|i| {
            let t = t[i];
            y[i] * dmath::exp(-t / fall) * clip(t / 0.0008, 0.0, 1.0)
        })
        .collect()
}

/// `Bed.clave(1.0)` before the cache and its velocity: a clave click at 2480 Hz.
pub fn clave() -> Vec<f64> {
    let n = length(0.06 * SRF);
    synth::tt(n)
        .iter()
        .map(|&t| dmath::sin(2.0 * PI * 2480.0 * t) * dmath::exp(-t / 0.014) * clip(t / 0.0006, 0.0, 1.0))
        .collect()
}

/// `Bed.bombo(1.0)` before the cache and its velocity: a soft bass drum for weight under the band hits,
/// `A.hp(A.kick(0.6, 0.24, 120, 50), 40)[:int(0.3 * SR)]`.
pub fn bombo() -> Vec<f64> {
    let x = synth::hp(&synth::kick(0.6, 0.24, 120.0, 50.0), 40.0, FILTER_O);
    x[..length(0.3 * SRF)].to_vec()
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
        Harmony::new(&g, "G", scale("harmonic minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    /// The rule on #50's chords: the tones from G3 (or the floor given) up, rising; the bass's root and fifth.
    #[test]
    fn the_rule_voices_the_mambo_chords() {
        let h = harmony(&["Gm", "Cm", "D7", "Eb7"]);
        assert_eq!(tones(&h, "Gm").unwrap(), [55, 58, 62]);
        assert_eq!(tones(&h, "D7").unwrap(), [57, 60, 62, 66]);
        assert_eq!(tones_from(&h, "Eb7", 67).unwrap(), [67, 70, 73, 75]);
        assert_eq!(tones_from(&h, "Cm", 57).unwrap(), [60, 63, 67]);
        assert_eq!((root(&h, "Gm").unwrap(), fifth(&h, "Gm").unwrap()), (43, 50));
        assert_eq!((root(&h, "D7").unwrap(), fifth(&h, "D7").unwrap()), (38, 45));
        assert_eq!(tones(&h, "F#m"), Err(HarmonyError::UnknownChord("F#m".into())));
    }

    #[test]
    fn instrument_lengths_follow_pythons_int() {
        assert_eq!(conga(Conga::Open, false).len(), 16800);
        assert_eq!((timbale(true, false).len(), shell().len()), (24000, 3840));
        assert_eq!((bell(true).len(), clave().len(), bombo().len()), (14400, 2880, 14400));
    }

    /// numpy's sign: zero at zero (the bell's first sample); the cascara and clave patterns as the kit writes them.
    #[test]
    fn the_small_rules() {
        assert_eq!((sign(0.0), sign(-0.5), sign(2.0)), (0.0, -1.0, 1.0));
        assert!(sign(f64::NAN).is_nan());
        assert_eq!(bell(true)[0], 0.0);
        assert_eq!(CASCARA.iter().filter(|b| **b).count(), 10);
        assert_eq!(CLAVE.iter().filter(|b| **b).count(), 5);
        // the conga strokes draw from their own seeds: a slap is not an open tone
        assert_ne!(conga(Conga::Slap, false), conga(Conga::Open, false));
    }
}
