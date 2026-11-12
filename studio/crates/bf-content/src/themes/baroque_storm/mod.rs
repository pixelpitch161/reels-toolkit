//! Theme: baroque storm (`projects/ballfight/kit/moods/baroque_storm.py`, #37 Clarinet vs Oboe). A stormy presto
//! with no woodwinds of its own: harpsichord continuo (repeated 8th-note bass, chords, 16th arpeggios later), a
//! racing minor string ostinato (violins in 16ths, cellos and basses driving 8ths, violas trembling in the back
//! half), timpani and thunder (under every captioned ability too). It thickens with parts (violin II, violas,
//! arpeggios, timpani on 3), never with level; after the K.O. it holds its breath on trembling strings until the
//! solo. Built for D minor (Dm Gm C F Bb A); other chords are voiced by rule, and its fixed runs are moved to the
//! episode's key.
//!
//! Here: the theme's voicings and its instruments, with the kit's names. The arrangement that reproduces the kit's
//! bed is [`BaroqueStormClassic`], in `classic.rs`.
//!
//! **Voicings.** The kit has two tables for the six chords of #37: the harpsichord's right hand ([`HARP_RH`]) and
//! violin I's 16th cell per beat ([`VLN`]: top, mid, peak, mid). Any other chord is voiced by rule: the chord's
//! first four tones from C#4 to G#5 for the right hand, and of the chord's tones from A4 to C6 the second, the
//! first, the third and the first again for the violins. [`harp_rh`] and [`vln`] are that, table first, for every
//! chord the harmony parses (12 roots x 13 qualities, checked against the kit's own answers in
//! `bf-harness/tests/theme_baroque_storm.rs`). The continuo's root, `Bed.root`, is `T.root_above(ch, 36)`.
//!
//! **The runs' offset.** The two fixed string runs (into the doors and into FIGHT!) were written in D; the bed
//! moves them by [`run_offset`] semitones: the episode's key from D, folded into -6 to 5.
//!
//! **Instruments.** Each is the kit's expression sample for sample, before its velocity ([`harpsi`], [`strng`],
//! [`timp`]): the kit makes each sound once per key of its cache and scales the cached sound by the velocity, and
//! the bed does the same (`classic.rs`). They take every argument of the kit's method in its order but the
//! velocity. Like the generators of `bf_sound::synth` they panic where Python raises (a length of no samples handed
//! to a filter).

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_core::py::fmod_py;
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip, noise};

pub use self::classic::BaroqueStormClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options, no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "baroque_storm",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(BaroqueStormClassic::new())),
};

/// What the interface says about the baroque storm.
pub static CARD: ThemeCard = ThemeCard {
    display: "Baroque storm",
    text: "A stormy presto: harpsichord, racing strings, timpani and thunder. No woodwinds of its own.",
    key: "D",
    scale: "minor",
    chord_loops: &[&["Dm", "Gm", "C", "F", "Bb", "Gm", "A", "A"]],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The kit configs that play the baroque storm (Table S4): their music as each `episode.py` sets it.
pub static PRESETS: [Preset; 1] = [Preset {
    config: "examples/clarinet_vs_oboe",
    fighters: ["CLARINET", "OBOE"],
    bpm: 140.0,
    key: "D",
    scale: "minor",
    intro: ["Dm", "A"],
    chord_loop: &["Dm", "Gm", "C", "F", "Bb", "Gm", "A", "A"],
    tail: &["Gm", "A"],
}];

/// `HARP_RH`: the harpsichord's right hand, as #37 voiced it.
pub const HARP_RH: [(&str, [i32; 4]); 6] = [
    ("Dm", [62, 65, 69, 74]),
    ("Gm", [62, 67, 70, 74]),
    ("C", [64, 67, 72, 76]),
    ("F", [65, 69, 72, 77]),
    ("Bb", [65, 70, 74, 77]),
    ("A", [61, 64, 69, 73]),
];

/// `VLN`: violin I's 16th cell per beat (top, mid, peak, mid), as #37 voiced it.
pub const VLN: [(&str, [i32; 4]); 6] = [
    ("Dm", [74, 69, 77, 69]),
    ("Gm", [74, 70, 79, 70]),
    ("C", [76, 72, 79, 72]),
    ("F", [77, 72, 81, 72]),
    ("Bb", [77, 74, 82, 74]),
    ("A", [76, 73, 81, 73]),
];

/// `strng`'s default vibrato depth.
pub const STRNG_VIB: f64 = 0.0;
/// `strng`'s default attack, in seconds.
pub const STRNG_ATT: f64 = 0.008;

fn table(of: &[(&str, [i32; 4])], chord: &str) -> Option<Vec<i32>> {
    of.iter().find(|(c, _)| *c == chord).map(|(_, notes)| notes.to_vec())
}

/// `Bed.root(ch)`: the continuo's root, `T.root_above(ch, 36)` (D2 G2 C2 F2 Bb2 A2 in D minor).
///
/// # Errors
/// Where Python raises: `chord` is not one of the episode's chords.
pub fn root(harmony: &Harmony, chord: &str) -> Result<i32, HarmonyError> {
    harmony.root_above(chord, 36)
}

/// `Bed.harp_rh(ch)`: the harpsichord's right hand: `HARP_RH.get(ch) or tuple(T.chord_tones(61, 80, ch)[:4])`.
///
/// # Errors
/// Where Python raises: `chord` is in neither the table nor the episode's chords.
pub fn harp_rh(harmony: &Harmony, chord: &str) -> Result<Vec<i32>, HarmonyError> {
    if let Some(notes) = table(&HARP_RH, chord) {
        return Ok(notes);
    }
    let mut tones = harmony.chord_tones(61, 80, chord)?;
    tones.truncate(4);
    Ok(tones)
}

/// `Bed.vln(ch)`: violin I's cell: the table's, else of the chord's tones from A4 to C6 the second, the first, the
/// third and the first again.
///
/// # Errors
/// Where Python raises: `chord` is in neither the table nor the episode's chords.
///
/// # Panics
/// Where Python raises `IndexError`: fewer than three tones in the range (no chord the harmony parses has).
pub fn vln(harmony: &Harmony, chord: &str) -> Result<Vec<i32>, HarmonyError> {
    if let Some(notes) = table(&VLN, chord) {
        return Ok(notes);
    }
    let t = harmony.chord_tones(69, 84, chord)?;
    assert!(t.len() >= 3, "vln({chord}): {t:?} has fewer than three tones");
    Ok(vec![t[1], t[0], t[2], t[0]])
}

/// `self.off`: how far the bed moves its fixed runs, written in D, for the episode's key:
/// `(T.KEY_PC - 2 + 6) % 12 - 6` (Python's `%` on ints: never negative).
pub fn run_offset(key_pc: i32) -> i32 {
    (key_pc - 2 + 6).rem_euclid(12) - 6
}

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// `Bed.harpsi(m, dur)` before its velocity: a harpsichord string: bright plucked additive (plucked near the
/// end), the 4' octave, a jack click. The click's noise comes from a new generator of seed `m`.
pub fn harpsi(m: i32, dur: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let f = synth::mtof(f64::from(m));
    let mut x = vec![0.0; n];
    // (f / 220) ** 0.35, a float power
    let bright = dmath::pow(f / 220.0, 0.35);
    for k in 1..32 {
        let kf = f64::from(k);
        let fk = f * kf;
        if fk > 12000.0 {
            break;
        }
        // a = abs(math.sin(math.pi * k * 0.13)) / k ** 0.75
        let a = dmath::sin(PI * kf * 0.13).abs() / dmath::pow(kf, 0.75);
        // x += a * np.sin(2 * np.pi * fk * t + k) * np.exp(-t * (1.2 + 0.35 * k) * (f / 220) ** 0.35)
        let w = 2.0 * PI * fk;
        let d = 1.2 + 0.35 * kf;
        for (v, &t) in x.iter_mut().zip(&t) {
            *v += a * dmath::sin(w * t + kf) * dmath::exp(-t * d * bright);
        }
    }
    let f2 = 2.0 * f;
    for k in 1..12 {
        let kf = f64::from(k);
        if f2 * kf > 12000.0 {
            break;
        }
        // x += 0.32 * abs(math.sin(math.pi * k * 0.2)) / k * np.sin(2 * np.pi * f2 * k * t) * np.exp(-t * (1.6 + 0.4 * k))
        let a = 0.32 * dmath::sin(PI * kf * 0.2).abs() / kf;
        let w = 2.0 * PI * f2 * kf;
        let d = 1.6 + 0.4 * kf;
        for (v, &t) in x.iter_mut().zip(&t) {
            *v += a * dmath::sin(w * t) * dmath::exp(-t * d);
        }
    }
    let seed = u64::try_from(m).expect("a harpsichord note's seed is its MIDI note, never below zero");
    let click = synth::bp(&noise(seed, n), 2500.0, 9000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let click = click[i] * dmath::exp(-t[i] / 0.004) * 0.6;
            (x[i] / 4.0 + click) * clip(t[i] / 0.0015, 0.0, 1.0)
        })
        .collect()
}

/// `Bed.strng(m, dur, cut, vib, att)` before its velocity: a string section note: three detuned saws (an
/// ensemble), a dark low-pass, a bow attack; vibrato on long notes. The note is 80 ms longer than `dur`, released
/// over 30 ms. It draws no noise.
pub fn strng(m: i32, dur: f64, cut: f64, vib: f64, att: f64) -> Vec<f64> {
    let n = length((dur + 0.08) * SRF);
    let t = synth::tt(n);
    let f = synth::mtof(f64::from(m));
    let mut x = vec![0.0; n];
    for (v, dt) in [-0.006, 0.0, 0.007].into_iter().enumerate() {
        let vf = v as f64;
        // fr = f * (1 + dt) * (1 + vib * np.sin(2 * np.pi * (5.2 + 0.4 * v) * t + v))
        let base = f * (1.0 + dt);
        let rate = 2.0 * PI * (5.2 + 0.4 * vf);
        let fr: Vec<f64> = t
            .iter()
            .map(|&t| base * (1.0 + vib * dmath::sin(rate * t + vf)))
            .collect();
        // x += 2 * ((np.cumsum(fr) / SR + 0.3 * v) % 1.0) - 1
        let lag = 0.3 * vf;
        for (o, c) in x.iter_mut().zip(dsp::cumsum(&fr)) {
            *o += 2.0 * fmod_py(c / SRF + lag, 1.0) - 1.0;
        }
    }
    let thirds: Vec<f64> = x.iter().map(|v| v / 3.0).collect();
    let x = synth::lp(&synth::hp(&thirds, 90.0, FILTER_O), cut, FILTER_O);
    (0..n)
        .map(|i| {
            // np.clip(t / att, 0, 1) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.03))
            let tail = if t[i] < dur {
                1.0
            } else {
                dmath::exp(-(t[i] - dur) / 0.03)
            };
            x[i] * (clip(t[i] / att, 0.0, 1.0) * tail)
        })
        .collect()
}

/// `Bed.timp(m)` before its velocity: a timpani stroke (1.1 s): a pitch that settles in 50 ms, two upper partials
/// that die first, and a mallet of low-passed noise from a new generator of seed `m + 5`.
pub fn timp(m: i32) -> Vec<f64> {
    let n = length(1.1 * SRF);
    let t = synth::tt(n);
    let f = synth::mtof(f64::from(m));
    let fr: Vec<f64> = t.iter().map(|&t| f * (1.0 + 0.04 * dmath::exp(-t / 0.05))).collect();
    let seed = u64::try_from(m + 5).expect("a timpani note's seed is its MIDI note + 5, never below zero");
    let mallet = synth::lp(&noise(seed, n), 1200.0, FILTER_O);
    dsp::cumsum(&fr)
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let (t, ph) = (t[i], 2.0 * PI * c / SRF);
            let x = dmath::sin(ph)
                + 0.5 * dmath::sin(1.5 * ph) * dmath::exp(-t / 0.3)
                + 0.25 * dmath::sin(2.0 * ph) * dmath::exp(-t / 0.2);
            let x = x * dmath::exp(-t / 0.55) * clip(t / 0.003, 0.0, 1.0);
            x + mallet[i] * dmath::exp(-t / 0.012) * 0.8
        })
        .collect()
}

/// The modern punch under beats 1 and 3: `A.hp(A.kick(1.0, 0.14, 150, 50)[:int(0.25 * SR)], 45)`. It draws no
/// noise.
pub fn punch() -> Vec<f64> {
    let x = synth::kick(1.0, 0.14, 150.0, 50.0);
    synth::hp(&x[..length(0.25 * SRF)], 45.0, FILTER_O)
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};

    use super::*;

    fn harmony(key: &str, chords: &[&str]) -> Harmony {
        let g = Grid::new(140.0);
        let all: Vec<String> = chords.iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2.min(all.len())], &all, &[], g.n_bars);
        Harmony::new(&g, key, scale("minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    #[test]
    fn voicings_are_the_tables_then_the_rule() {
        let h = harmony("D", &["Dm", "A", "Gm", "C", "F", "Bb", "Em", "E7"]);
        assert_eq!(harp_rh(&h, "Dm").unwrap(), [62, 65, 69, 74]);
        assert_eq!(vln(&h, "A").unwrap(), [76, 73, 81, 73]);
        assert_eq!((root(&h, "Dm").unwrap(), root(&h, "A").unwrap()), (38, 45));
        // Em's tones from C#4: E4 G4 B4 E5 G5 ...
        assert_eq!(harp_rh(&h, "Em").unwrap(), [64, 67, 71, 76]);
        // from A4: B4 E5 G5 B5 ...: (E5, B4, G5, B4)
        assert_eq!(vln(&h, "Em").unwrap(), [76, 71, 79, 71]);
        // The tables answer without the episode's chords; the rule needs the chord to be one of them.
        let plain = harmony("D", &["Dm", "A"]);
        assert_eq!(vln(&plain, "Bb").unwrap(), [77, 74, 82, 74]);
        assert!(harp_rh(&plain, "Em").is_err() && vln(&plain, "Em").is_err());
    }

    #[test]
    fn the_runs_move_from_d_into_minus_6_to_5() {
        let offs: Vec<i32> = (0..12).map(run_offset).collect();
        assert_eq!(offs, [-2, -1, 0, 1, 2, 3, 4, 5, -6, -5, -4, -3]);
    }

    #[test]
    fn instrument_lengths_follow_pythons_int() {
        assert_eq!(harpsi(50, 0.5).len(), 24000);
        assert_eq!(
            strng(50, 0.17, 900.0, STRNG_VIB, STRNG_ATT).len(),
            length((0.17 + 0.08) * SRF)
        );
        assert_eq!(timp(26).len(), 52800);
        assert_eq!(punch().len(), 12000);
        // a high note: the partials stop below 12 kHz
        assert!(harpsi(110, 0.3).iter().all(|v| v.is_finite()));
    }
}
