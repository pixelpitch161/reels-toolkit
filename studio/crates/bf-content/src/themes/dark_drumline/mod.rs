//! Theme: dark drumline (`projects/ballfight/kit/moods/dark_drumline.py`, #38 Bass Clarinet vs Euphonium, ported
//! for #53 Flute vs Sousaphone). A minor marching-band battle cadence with no low brass or woodwinds of its own (the
//! fighters play the band's bass line and melody): the snare line (accents, taps, ghosts, flams and diddle rolls on
//! a 16-step pattern: A and B bars and a fill ending each 4-bar phrase), quads tuned high, five pitched marching bass
//! drums on the chord tones (unison hits and split runs), the cymbal line (a crash on each phrase, choked crunches on
//! 2 and 4 later), and the pit: a high marimba ostinato on the chord's tones and timpani on the roots. It thickens
//! with the bars, never with level; every captioned ability gets a unison hit (it reads the story's captions).
//!
//! Here: the theme's options, its harmony helpers and its instruments as the kit computes them. The arrangement
//! that reproduces the kit's bed is [`DarkDrumlineClassic`], in `classic.rs`, with the kit's per-bed cache.
//!
//! **Options** (`DRUMLINE = dict(bass_drums=False, after_ko=True)`, #56): `bass_drums = false` drops the bed's own
//! five bass drums everywhere (the cadence's unison hits and splits, the unison hits, the last bar's beats) for an
//! episode whose fighter is the bass drum; with `after_ko = true` as well they come back from the K.O. on.
//!
//! **Voicings.** [`bd_notes`] tunes the five bass drums: #38's table ([`BD_38`]) for its four chords, else the four
//! highest chord tones from C3 to E-flat 4 and the root from G2. The timpani's root, [`troot`], is
//! `T.root_above(ch, 36)`.
//!
//! **The pit for a fighter.** BASS DRUM's part doubles its 1812 OVERTURE hymn on the bed's timpani and marimba when
//! the bed has both (`_pit`, under `hasattr`): this bed has them ([`bf_sound::traits::Bed::has_pit`]).

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::fightfile::OptionValue;
use bf_core::harmony::{Harmony, HarmonyError};
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip, noise};

pub use self::classic::DarkDrumlineClassic;
use super::{OptionKind, Preset, Theme, ThemeOption};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: the two `DRUMLINE` options; no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "dark_drumline",
    ported: true,
    card: &CARD,
    options: &OPTIONS,
    voices: &[],
    own_voice: super::no_own_voice,
    classic: classic::classic,
};

/// `DRUMLINE`'s two keys: `bass_drums` (`dl.get('bass_drums', True)`) and `after_ko` (default false).
pub static OPTIONS: [ThemeOption; 2] = [
    ThemeOption {
        key: "bass_drums",
        kit: "DRUMLINE.bass_drums",
        kind: OptionKind::Bool,
        default: Some(OptionValue::Bool(true)),
    },
    ThemeOption {
        key: "after_ko",
        kit: "DRUMLINE.after_ko",
        kind: OptionKind::Bool,
        default: Some(OptionValue::Bool(false)),
    },
];

/// #38's and #53's loop: i - VI - iv - V in C minor.
const LOOP: [&str; 4] = ["Cm", "Ab", "Fm", "G"];

/// What the interface says about the dark drumline.
pub static CARD: ThemeCard = ThemeCard {
    display: "Dark drumline",
    text: "A minor marching-band battle cadence: snares, high quads, five pitched bass drums, cymbals, and a pit of \
           marimba and timpani. No low brass or woodwinds of its own: the fighters play the bass line and the tune.",
    key: "C",
    scale: "minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The kit configs that play the dark drumline (Table S4): their music as each `episode.py` sets it (#56 also sets
/// `DRUMLINE = dict(bass_drums=False, after_ko=True)`).
pub static PRESETS: [Preset; 2] = [
    Preset {
        config: "ballfight_flute_vs_sousaphone",
        fighters: ["FLUTE", "SOUSAPHONE"],
        bpm: 126.0,
        key: "C",
        scale: "minor",
        intro: ["Cm", "G"],
        chord_loop: &LOOP,
        tail: &["Ab", "G"],
    },
    Preset {
        config: "ballfight_bassdrum_vs_tuba",
        fighters: ["BASS DRUM", "TUBA"],
        bpm: 126.0,
        key: "C",
        scale: "minor",
        intro: ["Cm", "G"],
        chord_loop: &LOOP,
        tail: &["Ab", "G"],
    },
];

/// `SN`: the snare line's 16 steps (`A` accent, `t` tap, `g` ghost, `f` flam, `r` diddle roll, `.` rest) for the A
/// bars, the B bars and the fill ending each 4-bar phrase (#38's, unchanged).
pub const SN: [&str; 3] = ["A.tgA.ttA.tgArrt", "f.t.A.tgf.tgA.rr", "A.A.AttAttArrrrr"];
/// `SN_LIGHT`: the same for a light bar (and a bar below 0.3 density).
pub const SN_LIGHT: [&str; 3] = ["A.g.A.g.A.g.A.g.", "A.g.A.g.A.g.Agrg", "A.g.A.g.A.gtArrr"];
/// `TEN`: the quads' drum numbers (1 high to 4 low), `lo` and `hi`.
pub const TEN: [&str; 2] = ["..1...2...1...3.", "..1.2.3.1.2.3434"];
/// `BASS`: the bass drums (`U` unison, 1 to 5 the splits, high to low), `lo` and `hi`.
pub const BASS: [&str; 2] = ["U.......U.......", "U...1.2.U..34.5."];
/// `BD_38`: #38's bass-drum tunings for its four chords.
pub const BD_38: [(&str, [i32; 5]); 4] = [
    ("Cm", [60, 55, 51, 48, 43]),
    ("Ab", [63, 60, 56, 51, 44]),
    ("Fm", [60, 56, 53, 48, 41]),
    ("G", [62, 59, 55, 50, 43]),
];
/// `tenor(i)`'s four pitches in Hz, high to low.
pub const TENORS: [f64; 4] = [494.0, 415.0, 349.0, 294.0];

/// `Bed.bd_notes(ch)`: the five bass drums' notes, high to low: [`BD_38`]'s for its chords, else the four highest
/// chord tones from C3 to E-flat 4 (`sorted(T.chord_tones(48, 63, ch), reverse=True)[:4]`) and the root at or above
/// G2.
///
/// # Errors
/// Where Python raises: `ch` is neither in the table nor one of the episode's chords.
pub fn bd_notes(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    if let Some((_, notes)) = BD_38.iter().find(|(c, _)| *c == ch) {
        return Ok(notes.to_vec());
    }
    let mut tones = harmony.chord_tones(48, 63, ch)?;
    tones.sort_unstable_by(|a, b| b.cmp(a));
    tones.truncate(4);
    tones.push(harmony.root_above(ch, 43)?);
    Ok(tones)
}

/// `Bed.troot(ch)`: the timpani's root, `T.root_above(ch, 36)` (C2 to B2).
///
/// # Errors
/// Where Python raises: `ch` does not parse.
pub fn troot(harmony: &Harmony, ch: &str) -> Result<i32, HarmonyError> {
    harmony.root_above(ch, 36)
}

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// A noise seed from a MIDI note (`default_rng` refuses a negative one; no note of the mood is).
fn seed_of(m: i32, what: &str) -> u64 {
    u64::try_from(m).unwrap_or_else(|_| panic!("{what}: a noise seed of {m}, below zero"))
}

/// `np.sin(2 * np.pi * np.cumsum(fr) / SR)` for a gliding pitch `fr`.
fn glide(fr: &[f64]) -> Vec<f64> {
    dsp::cumsum(fr).iter().map(|&c| dmath::sin(2.0 * PI * c / SRF)).collect()
}

/// `Bed.msnare(kind)` before the cache: a marching snare (a high-tension crack, tight wires, a little ring) of the
/// kind `A` (accent), `t` (tap) or `g` (ghost) (`f` is in the kit's level table too). Both noises come from one new
/// generator of `5 + ord(kind)`: the wires' normals first, then the crack's.
///
/// # Panics
/// On a kind the kit's level table lacks (Python's `KeyError`; the bed passes only `A`, `t` and `g`).
pub fn msnare(kind: char) -> Vec<f64> {
    let level = match kind {
        'A' | 'f' => 1.0,
        't' => 0.42,
        'g' => 0.20,
        other => panic!("msnare: no snare kind {other:?}"),
    };
    let n = length(0.28 * SRF);
    let t = synth::tt(n);
    let mut rng = Generator::new(5 + u64::from(kind));
    let mut w = vec![0.0; n];
    rng.fill_normal(0.0, 1.0, &mut w);
    let mut c = vec![0.0; n];
    rng.fill_normal(0.0, 1.0, &mut c);
    let wires = synth::bp(&w, 1800.0, 9500.0, FILTER_O);
    let crack = synth::bp(&c, 3000.0, 11000.0, FILTER_O);
    let (wire_dec, crack_k) = if kind == 'A' { (0.075, 1.6) } else { (0.05, 0.8) };
    (0..n)
        .map(|i| {
            let t = t[i];
            let wires = wires[i] * dmath::exp(-t / wire_dec);
            let crack = crack[i] * dmath::exp(-t / 0.004) * crack_k;
            let body = (0.5 * dmath::sin(2.0 * PI * 380.0 * t) + 0.6 * dmath::sin(2.0 * PI * 620.0 * t))
                * dmath::exp(-t / 0.03);
            let x = (0.8 * wires + crack + 0.7 * body) * clip(t / 0.0008, 0.0, 1.0);
            x * level
        })
        .collect()
}

/// `Bed.tenor(i)` before the cache: quad `i` (0 high to 3 low), tuned high so a low fighter owns the low mids: a
/// gliding body and a stick of `31 + i`'s band-passed noise, high-passed at 260 Hz.
pub fn tenor(i: usize) -> Vec<f64> {
    let f = TENORS[i];
    let n = length(0.32 * SRF);
    let t = synth::tt(n);
    let fr: Vec<f64> = t.iter().map(|&t| f * (1.0 + 0.12 * dmath::exp(-t / 0.02))).collect();
    let body = glide(&fr);
    let stick = synth::bp(&noise(31 + i as u64, n), 1500.0, 7000.0, FILTER_O);
    let x: Vec<f64> = (0..n)
        .map(|j| {
            let t = t[j];
            0.7 * (body[j] * dmath::exp(-t / 0.13)) + 0.8 * (stick[j] * dmath::exp(-t / 0.004))
        })
        .collect();
    synth::hp(&x, 260.0, FILTER_O)
}

/// `Bed.bassdrum(m)` before the cache (and its velocity, always 1 in the bed): a pitched marching bass drum (felt
/// mallet): a low tuned body two octaves under the note and a soft beater of `int(m)`'s low-passed noise.
pub fn bassdrum(m: i32) -> Vec<f64> {
    let f = synth::mtof(f64::from(m - 24));
    let n = length(0.45 * SRF);
    let t = synth::tt(n);
    let fr: Vec<f64> = t.iter().map(|&t| f * (1.0 + 0.25 * dmath::exp(-t / 0.015))).collect();
    let body = glide(&fr);
    let beat = synth::lp(&noise(seed_of(m, "bassdrum"), n), 500.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let body = body[i] * dmath::exp(-t / 0.22);
            let beat = beat[i] * dmath::exp(-t / 0.008);
            (body + 0.35 * beat) * clip(t / 0.002, 0.0, 1.0)
        })
        .collect()
}

/// `Bed.choke(seed)` (not cached): a choked cymbal crunch of `40 + seed`'s band-passed noise.
pub fn choke(seed: u64) -> Vec<f64> {
    let n = length(0.22 * SRF);
    let t = synth::tt(n);
    let x = synth::bp(&noise(40 + seed, n), 3000.0, 12000.0, FILTER_O);
    (0..n)
        .map(|i| x[i] * dmath::exp(-t[i] / 0.045) * clip(t[i] / 0.002, 0.0, 1.0))
        .collect()
}

/// `Bed.marimba(m)` before the cache (and its velocity, always 1): a marimba bar (the fundamental, the 3.93 and 9.2
/// partials dying first) and a mallet click of `m`'s band-passed noise.
pub fn marimba(m: i32) -> Vec<f64> {
    let n = length(0.7 * SRF);
    let t = synth::tt(n);
    let f = synth::mtof(f64::from(m));
    let click = synth::bp(&noise(seed_of(m, "marimba"), n), 1000.0, 4000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let x = dmath::sin(2.0 * PI * f * t) * dmath::exp(-t / 0.35)
                + 0.25 * dmath::sin(2.0 * PI * 3.93 * f * t) * dmath::exp(-t / 0.06)
                + 0.08 * dmath::sin(2.0 * PI * 9.2 * f * t) * dmath::exp(-t / 0.02);
            let click = click[i] * dmath::exp(-t / 0.003) * 0.3;
            (x + click) * clip(t / 0.001, 0.0, 1.0)
        })
        .collect()
}

/// `Bed.timp(m)` before the cache (and its velocity, always 1): a timpani stroke (1.1 s): a pitch that settles in
/// 50 ms, two upper partials that die first, and a mallet of `m + 5`'s low-passed noise.
pub fn timp(m: i32) -> Vec<f64> {
    let n = length(1.1 * SRF);
    let t = synth::tt(n);
    let f = synth::mtof(f64::from(m));
    let fr: Vec<f64> = t.iter().map(|&t| f * (1.0 + 0.04 * dmath::exp(-t / 0.05))).collect();
    let mallet = synth::lp(&noise(seed_of(m + 5, "timp"), n), 700.0, FILTER_O);
    dsp::cumsum(&fr)
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let (t, ph) = (t[i], 2.0 * PI * c / SRF);
            let x = dmath::sin(ph)
                + 0.3 * dmath::sin(1.5 * ph) * dmath::exp(-t / 0.2)
                + 0.12 * dmath::sin(2.0 * ph) * dmath::exp(-t / 0.15);
            let x = x * dmath::exp(-t / 0.55) * clip(t / 0.003, 0.0, 1.0);
            x + mallet[i] * dmath::exp(-t / 0.012) * 0.5
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bf_core::fightfile::FightFile;
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};

    use super::*;

    fn harmony(key: &str, chords: &[&str]) -> Harmony {
        let g = Grid::new(126.0);
        let all: Vec<String> = chords.iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2.min(all.len())], &all, &[], g.n_bars);
        Harmony::new(&g, key, scale("minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    #[test]
    fn bass_drums_are_the_table_then_the_rule() {
        let h = harmony("C", &["Cm", "G", "Ab", "Fm", "Gm", "Eb"]);
        assert_eq!(bd_notes(&h, "Cm").unwrap(), [60, 55, 51, 48, 43]);
        assert_eq!(bd_notes(&h, "G").unwrap(), [62, 59, 55, 50, 43]);
        // Gm from C3 to Eb4: D3 G3 Bb3 D4 G4 is past it, so D4 Bb3 G3 D3, then G2
        assert_eq!(bd_notes(&h, "Gm").unwrap(), [62, 58, 55, 50, 43]);
        // Eb: Eb4 Bb3 G3 Eb3, then the root from G2 up: Eb3
        assert_eq!(bd_notes(&h, "Eb").unwrap(), [63, 58, 55, 51, 51]);
        assert_eq!((troot(&h, "Cm").unwrap(), troot(&h, "Ab").unwrap()), (36, 44));
        // the table answers without the episode's chords; the rule needs the chord to be one of them
        let plain = harmony("C", &["Cm", "G"]);
        assert_eq!(bd_notes(&plain, "Fm").unwrap(), [60, 56, 53, 48, 41]);
        assert!(bd_notes(&plain, "Gm").is_err());
    }

    #[test]
    fn instrument_lengths_follow_pythons_int() {
        assert_eq!(msnare('A').len(), 13440);
        assert_eq!(tenor(3).len(), 15360);
        assert_eq!(bassdrum(43).len(), 21600);
        assert_eq!(choke(5).len(), 10560);
        assert_eq!(marimba(84).len(), 33600);
        assert_eq!(timp(36).len(), 52800);
        // each kind has its own generator (5 + ord(kind)), not the accent's noise at another level
        assert_ne!(msnare('t')[100], msnare('A')[100] * 0.42);
        assert!(msnare('g').iter().all(|v| v.is_finite()));
    }

    fn episode() -> FightFile {
        FightFile::parse(include_str!("../../../../../fights/flute_vs_trombone.toml")).expect("the episode")
    }

    #[test]
    fn options_follow_the_schema() {
        let mut ff = episode();
        let got = THEME.options_of(&ff).unwrap();
        assert_eq!(
            (got["bass_drums"].clone(), got["after_ko"].clone()),
            (OptionValue::Bool(true), OptionValue::Bool(false))
        );
        ff.music.theme_options.insert("bass_drums".into(), OptionValue::Bool(false));
        ff.music.theme_options.insert("after_ko".into(), OptionValue::Bool(true));
        let got = THEME.options_of(&ff).unwrap();
        assert_eq!(
            (got["bass_drums"].clone(), got["after_ko"].clone()),
            (OptionValue::Bool(false), OptionValue::Bool(true))
        );
        let err = |key: &str, v: OptionValue| {
            let mut f = episode();
            f.music.theme_options.insert(key.into(), v);
            THEME.options_of(&f).unwrap_err().to_string()
        };
        assert_eq!(
            err("bassdrums", OptionValue::Bool(false)),
            r#"music.theme_options.bassdrums: dark_drumline has no option "bassdrums" (it has: bass_drums, after_ko)"#
        );
        assert_eq!(
            err("bass_drums", OptionValue::Str("off".into())),
            "music.theme_options.bass_drums: must be true or false"
        );
        assert_eq!(
            err("after_ko", OptionValue::Int(1)),
            "music.theme_options.after_ko: must be true or false"
        );
    }
}
