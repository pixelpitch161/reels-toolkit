//! Theme: brass trailer (`projects/ballfight/kit/moods/brass_trailer.py`, #34 Trombone vs French Horn; #54 THE
//! REMATCH plays it as the sequel). A cinematic trailer bed with no brass of its own: the two brass fighters are the
//! brass section. Low strings play a spiccato 16th ostinato over the bar's root; taiko play the trailer pattern on
//! beats 1 and 3, growing through the fight; a braam (a low detuned saw stack with a filter swell and grit) lands on
//! every loop's downbeat; a high string tremolo carries the back half's tension (and holds under the whole title
//! card); trailer ticking 8ths come in once the fight heats up.
//!
//! **The option `sequel`** (the kit's `TRAILER = dict(sequel=True)`): darker and bigger, still with no brass: a
//! contrabass octave under the ostinato's 8ths, a dark cello pad under every bar, braams on every second bar (longer,
//! with a sine sub under them), the taiko ensemble from the start, a war-drum fill into each loop, the full trailer
//! under the winner's solo, big taiko on every beat and a double braam swell into the seam. The braam and taiko buses
//! are louder and the hall is darker.
//!
//! Here: the theme's voicings and its instruments, with the kit's names. The arrangement that reproduces the kit's
//! bed is [`BrassTrailerClassic`], in `classic.rs`.
//!
//! **Voicings.** The mood has no tables (#34's were what the rule gives for its chords): the bar's root is
//! [`root`] (`T.root_above(ch, 33)`, A1 to G#2), the tremolo's voicing [`trem`] (the chord's three lowest tones from
//! C#5) and the sequel's pad [`pad_notes`] (the three lowest from A2). [`trem`] and [`pad_notes`] are checked
//! against the kit's own answers for every chord the harmony parses (12 roots x 13 qualities) in
//! `bf-harness/tests/theme_brass_trailer.rs`.
//!
//! **Instruments.** Each is the kit's expression sample for sample ([`strings_low`], [`tremolo`], [`braam`],
//! [`taiko`] and [`tick`] before their velocity, [`sub`], [`pad`]), with the kit method's arguments in its order.
//! Like the generators of `bf_sound::synth` they panic where Python raises (a length of no samples handed to a
//! filter).

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::fightfile::OptionValue;
use bf_core::harmony::{Harmony, HarmonyError};
use bf_core::py::py_min;
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip, noise};
use bf_sound::traits::Bed;

pub use self::classic::BrassTrailerClassic;
use super::{OptionKind, Preset, Theme, ThemeOption, ThemeOptions};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: one option (`sequel`), no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "brass_trailer",
    ported: true,
    card: &CARD,
    options: &OPTIONS,
    voices: &[],
    own_voice: super::no_own_voice,
    classic,
};

/// The option the mood reads besides the music: `TRAILER = dict(sequel=True)`.
pub static OPTIONS: [ThemeOption; 1] = [ThemeOption {
    key: "sequel",
    kit: "TRAILER.sequel",
    kind: OptionKind::Bool,
    default: Some(OptionValue::Bool(false)),
}];

/// The classic bed for the fight's options: the sequel when `sequel` is true (`bool(TRAILER.get('sequel'))`).
fn classic(options: &ThemeOptions) -> Result<Box<dyn Bed>, ContentError> {
    let sequel = match options.get("sequel") {
        None => false,
        Some(OptionValue::Bool(b)) => *b,
        Some(_) => {
            return Err(ContentError::field(
                "music.theme_options.sequel",
                "must be true or false",
            ));
        }
    };
    Ok(Box::new(BrassTrailerClassic::new(sequel)))
}

/// What the interface says about the brass trailer.
pub static CARD: ThemeCard = ThemeCard {
    display: "Brass trailer",
    text: "A cinematic trailer: low strings, taiko, braams and a string tremolo. No brass of its own: the fighters \
           are the brass section.",
    key: "D",
    scale: "minor",
    chord_loops: &[
        &["Dm", "Bb", "Gm", "A"],
        &["Dm", "Bb", "Gm", "A", "Dm", "Bb", "Eb", "A"],
    ],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The kit configs that play the brass trailer (Table S4): their music as each `episode.py` sets it.
pub static PRESETS: [Preset; 2] = [
    Preset {
        config: "examples/trombone_vs_horn",
        fighters: ["TROMBONE", "HORN"],
        bpm: 122.0,
        key: "D",
        scale: "minor",
        intro: ["Dm", "A"],
        chord_loop: &["Dm", "Bb", "Gm", "A"],
        tail: &["A"],
    },
    Preset {
        config: "ballfight_horn_vs_trombone_rematch",
        fighters: ["HORN", "TROMBONE"],
        bpm: 122.0,
        key: "D",
        scale: "minor",
        intro: ["Dm", "A"],
        chord_loop: &["Dm", "Bb", "Gm", "A", "Dm", "Bb", "Eb", "A"],
        tail: &["Dm", "Bb", "Gm", "A", "A"],
    },
];

/// `OST`: the 16th ostinato's intervals over the bar's root.
pub const OST: [i32; 16] = [0, 0, 12, 0, 0, 12, 0, 7, 0, 0, 12, 0, 7, 0, 12, 10];

/// `braam`'s default velocity.
pub const BRAAM_VEL: f64 = 1.0;
/// `braam`'s default swell, in seconds.
pub const BRAAM_SWELL: f64 = 0.06;
/// `strings_low`'s default velocity.
pub const STRINGS_LOW_VEL: f64 = 1.0;

/// `Bed.root(ch)`: the bar's root in the low register, A1 to G#2, `T.root_above(ch, 33)` (Dm 38, Bb 34, Gm 43,
/// A 33).
///
/// # Errors
/// Where Python raises: `chord` does not parse.
pub fn root(harmony: &Harmony, chord: &str) -> Result<i32, HarmonyError> {
    harmony.root_above(chord, 33)
}

/// `Bed.trem(ch)`: the high tremolo's voicing, `tuple(T.chord_tones(73, 84, ch)[:3])`.
///
/// # Errors
/// Where Python raises: `chord` is not one of the episode's chords.
pub fn trem(harmony: &Harmony, chord: &str) -> Result<Vec<i32>, HarmonyError> {
    let mut t = harmony.chord_tones(73, 84, chord)?;
    t.truncate(3);
    Ok(t)
}

/// `Bed.pad_notes(ch)`: the sequel's cello pad, `tuple(T.chord_tones(45, 57, ch)[:3])`.
///
/// # Errors
/// Where Python raises: `chord` is not one of the episode's chords.
pub fn pad_notes(harmony: &Harmony, chord: &str) -> Result<Vec<i32>, HarmonyError> {
    let mut t = harmony.chord_tones(45, 57, chord)?;
    t.truncate(3);
    Ok(t)
}

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// `np.maximum(x, 0)`: `x` unless it is below zero (a NaN stays).
fn at_least_zero(x: f64) -> f64 {
    if x >= 0.0 || x.is_nan() { x } else { 0.0 }
}

/// `np.sign(x)`.
fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else if x == 0.0 {
        0.0
    } else {
        x
    }
}

/// `Bed.strings_low(m, dur, vel)`: a spiccato cello or bass note: three detuned saws, a short bow envelope, dark
/// (the low-pass opens with the velocity). The note is 80 ms longer than `dur`.
pub fn strings_low(m: i32, dur: f64, vel: f64) -> Vec<f64> {
    let n = length((dur + 0.08) * SRF);
    let x = synth::saw(synth::mtof(f64::from(m)), n, 0.006, 3);
    let x = synth::lp(&x, 700.0 + 500.0 * vel, FILTER_O);
    let env = synth::adsr(n, 0.004, 0.06, 0.35, 0.05);
    x.iter().zip(&env).map(|(x, e)| x * e * vel).collect()
}

/// `Bed.tremolo(ms, dur, att)`: the high string tremolo pad: three detuned saws per note, their mean, bowed at
/// 15 Hz, band-limited from 900 Hz to 5.2 kHz.
pub fn tremolo(ms: &[i32], dur: f64, att: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    for &m in ms {
        for (o, s) in x.iter_mut().zip(synth::saw(synth::mtof(f64::from(m)), n, 0.004, 3)) {
            *o += s;
        }
    }
    let count = ms.len().max(1) as f64;
    // 2 * np.pi * 15.0 * t
    let w = 2.0 * PI * 15.0;
    for (o, &t) in x.iter_mut().zip(&t) {
        *o /= count;
        *o *= 0.75 + 0.25 * sign(dmath::sin(w * t));
    }
    let y = synth::lp(&synth::hp(&x, 900.0, FILTER_O), 5200.0, FILTER_O);
    let env = synth::adsr(n, att, 0.2, 0.9, py_min(0.3, dur * 0.3));
    y.iter().zip(&env).map(|(y, e)| y * e).collect()
}

/// `Bed.braam(root, dur, vel, swell)`: the trailer braam: a low detuned saw stack (the root, the octave, the
/// twelfth) through a low-pass that closes over 16 bands, saturated, under a swell of `swell` seconds.
///
/// # Panics
/// Where Python raises: so few samples that a band has none of its own.
pub fn braam(root: i32, dur: f64, vel: f64, swell: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    for (m, g) in [(root, 1.0), (root + 12, 0.7), (root + 19, 0.35)] {
        for (o, s) in x.iter_mut().zip(synth::saw(synth::mtof(f64::from(m)), n, 0.01, 4)) {
            *o += g * s;
        }
    }
    let nb = 16usize;
    let mut out = vec![0.0; n];
    for k in 0..nb {
        let a = ((k * n) as f64 / nb as f64) as usize;
        let b = (((k + 1) * n) as f64 / nb as f64) as usize;
        assert!(b > a, "braam: {n} samples are too few for 16 bands");
        let u = (a + b) as f64 / 2.0 / n as f64;
        let fc = 200.0 + 1500.0 * dmath::exp(-u * 4.0) * vel;
        let band = synth::lp(&x[a.saturating_sub(4096)..b], fc, FILTER_O);
        out[a..b].copy_from_slice(&band[band.len() - (b - a)..]);
    }
    let top = dmath::np_tanh(2.2);
    let decay = dur * 0.45;
    (0..n)
        .map(|i| {
            let o = dmath::np_tanh(2.2 * out[i]) / top;
            let env = clip(t[i] / swell, 0.0, 1.0) * dmath::exp(-at_least_zero(t[i] - swell) / decay);
            o * env * vel
        })
        .collect()
}

/// `Bed.taiko(vel, big, seed)` before its velocity: a big low drum: a pitched body sweeping down and a skin slap
/// of band-passed noise from a new generator of `seed` (0.9 s when `big`, else 0.55 s).
pub fn taiko(big: bool, seed: u64) -> Vec<f64> {
    let dur = if big { 0.9 } else { 0.55 };
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let sweep = if big { 130.0 } else { 160.0 };
    let f: Vec<f64> = t.iter().map(|&t| 52.0 + sweep * dmath::exp(-t / 0.045)).collect();
    let body_dec = if big { 0.32 } else { 0.2 };
    let slap = synth::bp(&noise(seed, n), 180.0, 1400.0, FILTER_O);
    dsp::cumsum(&f)
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let body = dmath::sin(2.0 * PI * c / SRF) * dmath::exp(-t[i] / body_dec);
            body + 0.45 * (slap[i] * dmath::exp(-t[i] / 0.018))
        })
        .collect()
}

/// `Bed.tick(vel, seed)` before its velocity: trailer ticking, a dry wooden click (50 ms of band-passed noise
/// from a new generator of `seed`).
pub fn tick(seed: u64) -> Vec<f64> {
    let n = length(0.05 * SRF);
    let t = synth::tt(n);
    let x = synth::bp(&noise(seed, n), 2500.0, 7000.0, FILTER_O);
    x.iter().zip(&t).map(|(x, &t)| x * dmath::exp(-t / 0.006)).collect()
}

/// The sound `Bed.sub(root, dur)` makes (the bed caches it as the kit does): a sine sub an octave under a braam,
/// swelling in over 80 ms and dying with it.
pub fn sub(root: i32, dur: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    // 2 * np.pi * A.mtof(root) * t
    let w = 2.0 * PI * synth::mtof(f64::from(root));
    let decay = dur * 0.5;
    synth::tt(n)
        .iter()
        .map(|&t| {
            let env = clip(t / 0.08, 0.0, 1.0) * dmath::exp(-at_least_zero(t - 0.08) / decay);
            dmath::sin(w * t) * env
        })
        .collect()
}

/// The sound `Bed.pad(ms, dur)` makes (the bed caches it as the kit does): a dark cello pad, detuned saws (their
/// mean), a slow bow, low-passed; 100 ms longer than `dur`.
///
/// # Panics
/// Where Python raises: no notes (the kit's sum of nothing is a number, which the filter refuses).
pub fn pad(ms: &[i32], dur: f64) -> Vec<f64> {
    assert!(!ms.is_empty(), "pad: no notes");
    let n = length((dur + 0.1) * SRF);
    // sum(...) starts from 0
    let mut x = vec![0.0; n];
    for &m in ms {
        for (o, s) in x.iter_mut().zip(synth::saw(synth::mtof(f64::from(m)), n, 0.008, 3)) {
            *o += s;
        }
    }
    let count = ms.len() as f64;
    let x: Vec<f64> = x.iter().map(|v| v / count).collect();
    let y = synth::lp(&x, 900.0, FILTER_O);
    let env = synth::adsr(n, 0.25, 0.3, 0.85, 0.12);
    y.iter().zip(&env).map(|(y, e)| y * e).collect()
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};

    use super::*;

    fn harmony(key: &str, chords: &[&str]) -> Harmony {
        let g = Grid::new(122.0);
        let all: Vec<String> = chords.iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2.min(all.len())], &all, &[], g.n_bars);
        Harmony::new(&g, key, scale("minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    #[test]
    fn voicings_are_34s_for_its_chords() {
        let h = harmony("D", &["Dm", "A", "Bb", "Gm", "Eb"]);
        // #34's ROOT: Dm 38, Bb 34, Gm 43, A 33
        let roots: Vec<i32> = ["Dm", "Bb", "Gm", "A"].iter().map(|c| root(&h, c).unwrap()).collect();
        assert_eq!(roots, [38, 34, 43, 33]);
        // from C#5: Dm is D5 F5 A5, A is C#5 E5 A5
        assert_eq!(trem(&h, "Dm").unwrap(), [74, 77, 81]);
        assert_eq!(trem(&h, "A").unwrap(), [73, 76, 81]);
        // from A2: Dm is A2 D3 F3
        assert_eq!(pad_notes(&h, "Dm").unwrap(), [45, 50, 53]);
        assert!(trem(&h, "Em").is_err());
    }

    #[test]
    fn instrument_lengths_follow_pythons_int() {
        assert_eq!(strings_low(50, 0.5, 1.0).len(), length(0.58 * SRF));
        assert_eq!(taiko(true, 0).len(), 43200);
        assert_eq!(taiko(false, 3).len(), 26400);
        assert_eq!(tick(1).len(), 2400);
        assert_eq!(braam(26, 0.42, 0.9, 0.01).len(), 20160);
        assert_eq!(sub(26, 0.8).len(), 38400);
        assert_eq!(pad(&[45, 50, 53], 1.0).len(), 52800);
        assert!(tremolo(&[74, 77, 81], 0.5, 0.3).iter().all(|v| v.is_finite()));
        assert_eq!(tremolo(&[], 0.2, 0.05).len(), 9600);
        assert_eq!((sign(-0.0), sign(-2.0), sign(3.0)), (0.0, -1.0, 1.0));
    }

    #[test]
    fn a_fight_files_options_follow_the_schema() {
        use bf_core::fightfile::FightFile;
        let ep = || FightFile::parse(include_str!("../../../../../fights/flute_vs_trombone.toml")).unwrap();
        let ff = ep();
        assert_eq!(THEME.options_of(&ff).unwrap()["sequel"], OptionValue::Bool(false));
        let with = |k: &str, v: OptionValue| {
            let mut f = ep();
            f.music.theme_options.insert(k.into(), v);
            THEME.options_of(&f).map_err(|e| e.to_string())
        };
        assert_eq!(
            with("sequel", OptionValue::Bool(true)).unwrap()["sequel"],
            OptionValue::Bool(true)
        );
        assert_eq!(
            with("plugged", OptionValue::Str("FLUTE".into())).unwrap_err(),
            r#"music.theme_options.plugged: brass_trailer has no option "plugged" (it has: sequel)"#
        );
        assert_eq!(
            with("sequel", OptionValue::Str("yes".into())).unwrap_err(),
            "music.theme_options.sequel: must be true or false"
        );
    }

    #[test]
    fn the_option_picks_the_sequel() {
        let mut o = ThemeOptions::new();
        assert!(classic(&o).is_ok());
        o.insert("sequel".into(), OptionValue::Bool(true));
        assert_eq!(classic(&o).unwrap().reverb(), (2.8, 0.62, 0.24));
        o.insert("sequel".into(), OptionValue::Bool(false));
        assert_eq!(classic(&o).unwrap().reverb(), (2.4, 0.6, 0.22));
        o.insert("sequel".into(), OptionValue::Int(1));
        assert_eq!(
            classic(&o).err().unwrap().to_string(),
            "music.theme_options.sequel: must be true or false"
        );
    }
}
