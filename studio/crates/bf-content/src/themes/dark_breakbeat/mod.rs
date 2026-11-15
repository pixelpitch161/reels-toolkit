//! Theme: dark breakbeat, the bed of `projects/ballfight_turntables_vs_synth/audio.py` (#31 v3, Turntables vs
//! Synth). A chopped, crunchy original breakbeat (kick, snare and clap, ghost notes and hats with a touch of swing,
//! a stutter on every fourth bar), a reese bass holding the root with a push on the "and" of 3, and cold detuned
//! saw pads. It leaves the top to the fighters. Built for F minor (Fm Db Bbm C) at 137.70 BPM.
//!
//! A quick port (ruling M4-R16): close to the episode by ear, not sample for sample. The episode's tables
//! ([`BASS_ROOT`], [`PAD`]) answer first; any other chord is voiced by the same rule (the root in the reese's
//! octave, the chord's tones in the pad's).

mod classic;
pub mod layers;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError, parse_chord};
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip};

pub use self::classic::DarkBreakbeatClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options, no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "dark_breakbeat",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(DarkBreakbeatClassic::new())),
};

/// What the interface says about the dark breakbeat.
pub static CARD: ThemeCard = ThemeCard {
    display: "Dark breakbeat",
    text: "A chopped, crunchy breakbeat with a reese bass and cold pads. Leaves the top to the fighters.",
    key: "F",
    scale: "minor",
    chord_loops: &[&["Fm", "Db", "Bbm", "C"]],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The episode that plays the dark breakbeat, its music as its `timeline.py` sets it.
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_turntables_vs_synth",
    fighters: ["TURNTABLES", "SYNTH"],
    bpm: 138.0,
    key: "F",
    scale: "minor",
    intro: ["Fm", "C"],
    chord_loop: &["Fm", "Db", "Bbm", "C"],
    tail: &["C"],
}];

/// `BASS_ROOT`: the reese's root for #31's chords.
pub const BASS_ROOT: [(&str, i32); 4] = [("Fm", 29), ("Db", 25), ("Bbm", 34), ("C", 24)];

/// `PAD`: the cold pad's chords, as #31 voiced them.
pub const PAD: [(&str, &[i32]); 4] = [
    ("Fm", &[53, 56, 60, 63]),
    ("Db", &[49, 53, 56, 60]),
    ("Bbm", &[49, 53, 58, 61]),
    ("C", &[48, 52, 55, 58]),
];

/// The reese's root of `chord`: the table's, else the chord's root from C1 to B1.
///
/// # Errors
/// `chord` does not parse.
pub fn bass_root(chord: &str) -> Result<i32, HarmonyError> {
    match BASS_ROOT.iter().find(|(c, _)| *c == chord) {
        Some((_, m)) => Ok(*m),
        None => Ok(24 + parse_chord(chord)?.root),
    }
}

/// The cold pad's chord: the table's, else the chord's lowest four tones from C#3 up.
///
/// # Errors
/// `chord` is in neither the table nor the episode's chords.
pub fn pad(harmony: &Harmony, chord: &str) -> Result<Vec<i32>, HarmonyError> {
    if let Some((_, notes)) = PAD.iter().find(|(c, _)| *c == chord) {
        return Ok(notes.to_vec());
    }
    Ok(harmony.chord_tones(49, 66, chord)?.into_iter().take(4).collect())
}

/// Python's `int(x)` for a length (never below zero).
fn length(x: f64) -> usize {
    if x > 0.0 { x as usize } else { 0 }
}

/// `kick(v)`: `A.kick(1.0, 0.30, 140, 44) * v`.
pub fn kick(v: f64) -> Vec<f64> {
    synth::kick(1.0, 0.30, 140.0, 44.0).iter().map(|x| x * v).collect()
}

/// `snare(1.0)`: `A.snare(0.12) * 0.8` with `A.clap() * 0.25` over it.
pub fn snare_hit() -> Vec<f64> {
    let a = synth::snare(0.12);
    let b = synth::clap();
    let n = a.len().max(b.len());
    (0..n)
        .map(|i| a.get(i).map_or(0.0, |v| v * 0.8) + b.get(i).map_or(0.0, |v| v * 0.25))
        .collect()
}

/// `crunch(x, bits=6)`: the sampled break's grit, a soft clip and a little bit reduction.
pub fn crunch(x: &[f64]) -> Vec<f64> {
    let q = 64.0;
    let k = dmath::tanh(1.8);
    x.iter()
        .map(|v| {
            let y = dmath::tanh(1.8 * v) / k;
            (y * q).round_ties_even() / q * 0.6 + y * 0.4
        })
        .collect()
}

/// The episode's block filter: `x` low-passed block by block at `cut(k)` for block `k`, each block filtered with
/// `hist` samples of the signal before it.
pub(crate) fn swept_lp(x: &[f64], blocks: usize, hist: usize, cut: impl Fn(usize) -> f64) -> Vec<f64> {
    let n = x.len();
    let mut out = vec![0.0; n];
    for k in 0..blocks {
        let (a, b) = (k * n / blocks, (k + 1) * n / blocks);
        if b <= a {
            continue;
        }
        let y = synth::lp(&x[a.saturating_sub(hist)..b], cut(k), 2);
        out[a..b].copy_from_slice(&y[y.len() - (b - a)..]);
    }
    out
}

/// `reese(m, dur, vel, cut0, cut1)`: three detuned saws and two an octave up over a sub sine, the filter opening
/// and closing over the note.
pub fn reese(m: i32, dur: f64, vel: f64, cut0: f64, cut1: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    if n == 0 {
        return Vec::new();
    }
    let f = synth::mtof(f64::from(m));
    let t = synth::tt(n);
    let lo = synth::saw(f, n, 0.012, 3);
    let hi = synth::saw(2.0 * f, n, 0.008, 2);
    let x: Vec<f64> = lo.iter().zip(&hi).map(|(a, b)| a + 0.5 * b).collect();
    let nb = 12;
    let out = swept_lp(&x, nb, 1024, |k| {
        cut0 + (cut1 - cut0) * dmath::sin(PI * (k as f64 + 0.5) / nb as f64)
    });
    let env = synth::adsr(n, 0.008, 0.1, 0.9, (dur * 0.3).min(0.06));
    (0..n)
        .map(|i| dmath::tanh(1.4 * (0.55 * out[i] + 0.7 * dmath::sin(2.0 * PI * f * t[i]))) * env[i] * vel)
        .collect()
}

/// `reese(m, dur, vel)` with the episode's default filter (320 to 900 Hz).
pub fn reese_plain(m: i32, dur: f64, vel: f64) -> Vec<f64> {
    reese(m, dur, vel, 320.0, 900.0)
}

/// `cold_pad(ms, dur, cut)`: detuned saws, low-passed, a slow swell.
pub fn cold_pad(ms: &[i32], dur: f64, cut: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    if n == 0 || ms.is_empty() {
        return vec![0.0; n];
    }
    let mut x = vec![0.0; n];
    for &m in ms {
        for (o, v) in x.iter_mut().zip(synth::saw(synth::mtof(f64::from(m)), n, 0.006, 3)) {
            *o += v;
        }
    }
    let k = ms.len() as f64;
    let x: Vec<f64> = x.iter().map(|v| v / k).collect();
    let env = synth::adsr(n, 0.35, 0.3, 0.8, (dur * 0.3).min(0.4));
    synth::lp(&x, cut, 2).iter().zip(&env).map(|(v, e)| v * e).collect()
}

/// `sub_drop(dur, f0, f1)`: a sine falling from `f0` to `f1` Hz, driven.
pub fn sub_drop(dur: f64, f0: f64, f1: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let f: Vec<f64> = t.iter().map(|&t| f1 + (f0 - f1) * dmath::exp(-t / 0.25)).collect();
    dsp::cumsum(&f)
        .iter()
        .zip(&t)
        .map(|(c, &t)| {
            let x = dmath::sin(2.0 * PI * c / SRF);
            dmath::tanh(2.2 * x) * dmath::exp(-t / (dur * 0.6)) * clip(t / 0.002, 0.0, 1.0)
        })
        .collect()
}

/// The VS thunder crack: band-passed noise cracking over a low rumble (the episodes' seeds 91 and 92).
pub fn thunder_crack() -> Vec<f64> {
    let nn = (1.4 * SRF) as usize;
    let tt = synth::tt(nn);
    let mut draw = vec![0.0; nn];
    Generator::new(91).fill_normal(0.0, 1.0, &mut draw);
    let crack = synth::bp(&draw, 1500.0, 9000.0, FILTER_O);
    Generator::new(92).fill_normal(0.0, 1.0, &mut draw);
    let rumble = synth::lp(&draw, 160.0, FILTER_O);
    (0..nn)
        .map(|i| {
            let crack = crack[i] * dmath::exp(-tt[i] / 0.035);
            let rumble = rumble[i] * (tt[i] / 0.02).clamp(0.0, 1.0) * dmath::exp(-tt[i] / 0.5) * 3.0;
            crack * 0.9 + rumble
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_instruments_sound() {
        for x in [
            reese_plain(29, 0.5, 1.0),
            cold_pad(&[53, 56, 60, 63], 0.6, 1100.0),
            sub_drop(1.2, 95.0, 28.0),
            crunch(&kick(1.0)),
            crunch(&snare_hit()),
        ] {
            assert!(!x.is_empty() && x.iter().all(|v| v.is_finite()));
            assert!(x.iter().any(|v| v.abs() > 0.05));
        }
        assert_eq!(reese_plain(29, 0.5, 1.0).len(), 24000);
        assert_eq!(bass_root("Fm"), Ok(29));
        assert_eq!(bass_root("Eb"), Ok(27));
    }
}
