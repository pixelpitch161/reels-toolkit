//! Theme: drift phonk, the bed of `projects/ballfight/audio.py` (#27), quick-ported by ear (ruling M4-R16: close
//! enough, no reference mood, no parity gates). A menacing E minor groove: a pitched 808 cowbell riff (a two-bar
//! phrase with the flat 2nd), a distorted 808 that holds the root and slides into the next bar's, a hard kick on 1,
//! the "and" of 2 and 3, a clap on 2 and 4, 16th hats (open on the offbeats as the fight heats up) and a dark pad.
//! No lead and no fighter voice of its own: the tune belongs to the fighters. Built for E minor (E5 E5 C5 B5).
//!
//! Here: the voicings and the instruments; the arrangement is [`DriftPhonkClassic`], in `classic.rs`.
//!
//! **Voicings.** The episode's tables (the 808's root, the pad's triad) for its three chords, and for any other
//! chord the same shape built from the chord's own tones ([`bass_root`], [`pad_voicing`]). The cowbell riff is the
//! episode's, moved to the fight's key, its 7th step raised to the leading tone over a chord that has it
//! ([`riff`]).

mod classic;
pub mod layers;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip};

pub use self::classic::DriftPhonkClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options, no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "drift_phonk",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(DriftPhonkClassic::new())),
    composed: Some(|o, c, s| Ok(Box::new(layers::composed(o, c, s)?))),
};

/// #27's loop: E5 E5 C5 B5.
const LOOP: [&str; 4] = ["E", "E", "C", "B"];

/// What the interface says about the drift phonk.
pub static CARD: ThemeCard = ThemeCard {
    display: "Drift phonk",
    text: "A menacing drift phonk: a pitched cowbell riff, a sliding distorted 808, a hard kick, claps and 16th \
           hats over a dark pad. Leaves the tune to the fighters.",
    key: "E",
    scale: "minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The two stand-in pairs that play the drift phonk at #27's tempo, key and loop
/// (`tools/studio_export/older/configs/drift_phonk_a`, `drift_phonk_b`).
pub static PRESETS: [Preset; 2] = [
    Preset {
        config: "drift_phonk_a",
        fighters: ["PIANO", "ELECTRIC"],
        bpm: 130.0,
        key: "E",
        scale: "minor",
        intro: ["E", "B"],
        chord_loop: &LOOP,
        tail: &["E", "C", "B"],
    },
    Preset {
        config: "drift_phonk_b",
        fighters: ["ELECTRIC", "TUBA"],
        bpm: 130.0,
        key: "E",
        scale: "minor",
        intro: ["E", "B"],
        chord_loop: &LOOP,
        tail: &["E", "C", "B"],
    },
];

/// `BASS_ROOT`: the 808's root, as #27 voiced it.
const BASS_ROOT: [(&str, i32); 3] = [("E", 28), ("C", 36), ("B", 35)];

/// The pad's triad, as #27 voiced it (its E is minor: the key's tonic, named as a power chord).
const PAD: [(&str, [i32; 3]); 3] = [("E", [52, 55, 59]), ("C", [48, 52, 55]), ("B", [47, 51, 54])];

const R: Option<i32> = None;

/// `CB`: the two-bar cowbell riff in E (`None` is a rest), in 16ths.
const CB: [[Option<i32>; 16]; 2] = [
    [
        Some(76),
        R,
        Some(76),
        Some(79),
        R,
        Some(76),
        Some(77),
        R,
        Some(76),
        R,
        Some(83),
        R,
        Some(81),
        Some(79),
        Some(77),
        R,
    ],
    [
        Some(76),
        R,
        Some(76),
        Some(79),
        R,
        Some(76),
        Some(77),
        R,
        Some(84),
        R,
        Some(83),
        R,
        Some(79),
        Some(77),
        Some(76),
        Some(74),
    ],
];

/// The 808's root for `chord`: the table's, else the root from E1 up (E1 to D#2).
///
/// # Errors
/// `chord` is not one of the episode's chords.
pub fn bass_root(harmony: &Harmony, chord: &str) -> Result<i32, HarmonyError> {
    if let Some((_, r)) = BASS_ROOT.iter().find(|(c, _)| *c == chord) {
        return Ok(*r);
    }
    harmony.root_above(chord, 28)
}

/// The pad's triad for `chord`: the table's, else the root from B2 up, its third and fifth.
///
/// # Errors
/// `chord` is not one of the episode's chords.
pub fn pad_voicing(harmony: &Harmony, chord: &str) -> Result<[i32; 3], HarmonyError> {
    if let Some((_, v)) = PAD.iter().find(|(c, _)| *c == chord) {
        return Ok(*v);
    }
    let r = harmony.root_above(chord, 47)?;
    let pcs = harmony.chord_pcs(chord)?;
    let up = |i: usize, or: i32| pcs.get(i).map_or(or, |p| (p - pcs[0]).rem_euclid(12));
    Ok([r, r + up(1, 3), r + up(2, 7)])
}

/// Bar `b`'s cowbell riff (the phrase's second bar on odd bars) under `chord`: #27's, moved from E to the fight's
/// key (the nearer way), its 7th step raised to the leading tone over a chord that has the leading tone (#27's D
/// to D# over B).
///
/// # Errors
/// `chord` is not one of the episode's chords.
pub fn riff(harmony: &Harmony, b: usize, chord: &str) -> Result<[Option<i32>; 16], HarmonyError> {
    let mut shift = (harmony.key_pc - 4).rem_euclid(12);
    if shift > 6 {
        shift -= 12;
    }
    let pcs = harmony.chord_pcs(chord)?;
    let seventh = (harmony.key_pc + 10).rem_euclid(12);
    let leading = (harmony.key_pc + 11).rem_euclid(12);
    let raise = pcs.contains(&leading) && !pcs.contains(&seventh);
    let mut out = CB[b % 2];
    for m in out.iter_mut().flatten() {
        *m += shift;
        if raise && m.rem_euclid(12) == seventh {
            *m += 1;
        }
    }
    Ok(out)
}

fn length(x: f64) -> usize {
    if x > 0.0 { x as usize } else { 0 }
}

/// `cowbell(m, dur)`: the 808 cowbell: two detuned squares, band-passed, quickly decaying and driven.
pub fn cowbell(m: i32, dur: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let f = synth::mtof(f64::from(m));
    let a = synth::square(f, n, 0.5);
    let b = synth::square(f * 1.48, n, 0.5);
    let x: Vec<f64> = a.iter().zip(&b).map(|(a, b)| a * 0.6 + b * 0.4).collect();
    let env = synth::expenv(n, 0.075, 0.001);
    let k = dmath::tanh(2.2);
    synth::bp(&x, 350.0, 5200.0, FILTER_O)
        .iter()
        .zip(&env)
        .map(|(v, e)| dmath::tanh(v * e * 2.2) / k)
        .collect()
}

/// `b808(m0, m1, dur, slide)`: the distorted 808: holds `m0`, then slides to `m1` over its last `slide` seconds.
pub fn b808(m0: i32, m1: i32, dur: f64, slide: f64) -> Vec<f64> {
    let n = length((dur + 0.05) * SRF);
    let t = synth::tt(n);
    let ts = (dur - slide).max(0.0);
    let (f0, f1) = (synth::mtof(f64::from(m0)), synth::mtof(f64::from(m1)));
    let f: Vec<f64> = t
        .iter()
        .map(|&t| {
            if t < ts {
                f0
            } else {
                f0 * dmath::pow(f1 / f0, clip((t - ts) / slide.max(1e-3), 0.0, 1.0))
            }
        })
        .collect();
    let fade = length(0.05 * SRF).min(n).max(1);
    let k = dmath::tanh(2.4);
    let x: Vec<f64> = dsp::cumsum(&f)
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let ph = 2.0 * PI * c / SRF;
            let x = dmath::sin(ph) + 0.12 * dmath::sin(2.0 * ph);
            let mut env = clip(t[i] / 0.003, 0.0, 1.0) * dmath::exp(-t[i] / (dur * 0.9).max(0.35));
            if i + fade >= n {
                env *= (n - 1 - i) as f64 / fade as f64;
            }
            dmath::tanh(2.4 * x * env) / k
        })
        .collect();
    synth::lp(&x, 1400.0, FILTER_O)
}

/// `kick(punch)`: the hard phonk kick, driven.
pub fn kick(punch: f64) -> Vec<f64> {
    let k = dmath::tanh(1.6);
    synth::kick(punch, 0.22, 170.0, 48.0)
        .iter()
        .map(|v| dmath::tanh(v * 1.6) / k)
        .collect()
}

/// `roll(dur, f0, step)`: a snare roll growing from `f0` to full over `dur` seconds.
pub fn roll(dur: f64, f0: f64, step: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let mut out = vec![0.0; n + length(SRF)];
    let s = synth::snare(0.05);
    let mut t = 0.0;
    while t < dur - 1e-4 {
        let a = length(t * SRF);
        let g = f0 + (1.0 - f0) * dmath::pow(t / dur, 1.5);
        for (o, v) in out[a..].iter_mut().zip(&s) {
            *o += v * g;
        }
        t += step;
    }
    out.truncate(n);
    out
}

/// `shimmer(ms, dur)`: a soft high chord of sines, each with its own tremolo (under the stinger).
pub fn shimmer(ms: &[i32], dur: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let env = synth::adsr(n, 0.25, 0.3, 0.7, 0.6);
    let k = ms.len().max(1) as f64;
    (0..n)
        .map(|i| {
            let mut x = 0.0;
            for (j, &m) in ms.iter().enumerate() {
                let f = synth::mtof(f64::from(m));
                let trem = 0.7 + 0.3 * dmath::sin(2.0 * PI * (5.0 + j as f64) * t[i]);
                x += dmath::sin(2.0 * PI * f * t[i]) * trem;
            }
            x / k * env[i]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};

    use super::*;

    fn harmony(key: &str, chords: &[&str]) -> Harmony {
        let g = Grid::new(130.0);
        let all: Vec<String> = chords.iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2], &all, &[], g.n_bars);
        Harmony::new(&g, key, scale("minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    #[test]
    fn the_rule_gives_the_tables_shape() {
        let h = harmony("E", &["E", "B", "C", "Am", "Dm"]);
        for (ch, r) in BASS_ROOT {
            assert_eq!(h.root_above(ch, 28).unwrap(), r, "{ch}");
        }
        assert_eq!(pad_voicing(&h, "Am").unwrap(), [57, 60, 64]);
        assert_eq!(bass_root(&h, "Dm").unwrap(), 38);
        // #27's riff in E, its D raised over B.
        assert_eq!(riff(&h, 1, "E").unwrap()[15], Some(74));
        assert_eq!(riff(&h, 1, "B").unwrap()[15], Some(75));
        // In D minor the riff sits a tone lower, its C raised over A.
        let d = harmony("D", &["Dm", "A", "Bb"]);
        assert_eq!(riff(&d, 0, "Dm").unwrap()[0], Some(74));
        assert_eq!(riff(&d, 1, "A").unwrap()[15], Some(73));
    }

    #[test]
    fn the_instruments_sound() {
        for x in [
            cowbell(76, 0.16),
            b808(28, 40, 0.5, 0.1),
            b808(28, 28, 0.01, 0.0),
            kick(1.0),
            roll(0.5, 0.2, 0.05),
            shimmer(&[88, 91], 0.4),
        ] {
            assert!(x.iter().all(|v| v.is_finite()));
            assert!(x.iter().any(|v| v.abs() > 0.05));
        }
    }
}
