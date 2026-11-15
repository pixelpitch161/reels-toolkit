//! Theme: Balkan fire (the bed of `projects/ballfight_drums_vs_violin/audio.py`, #28 DRUMS vs VIOLIN), quick-ported
//! by ear (ruling M4-R16: close enough, no parity gates). A dark Balkan oom-pah with **no drums of its own** (in #28
//! the DRUMS fighter was the kit and the VIOLIN the lead): an upright bass on 1 and 3 (a chromatic pickup into the
//! next bar as the fire builds), musette accordion stabs on 2 and 4 (8th-note "pah-pah" later), a bellows pad, and a
//! cimbalom shimmer of 16th arpeggios building through the back half. Only the count-in, the fills into FIGHT! and
//! the last bar's build use a snare and sticks. Built for D harmonic minor (Dm Dm Gm A); any chord works (voicings
//! by rule).
//!
//! Here: the voicings and the instruments, after the episode's functions of the same names. The arrangement is
//! [`BalkanFireClassic`], in `classic.rs`.
//!
//! **Voicings.** The episode's tables are rules here, for every chord the harmony parses: [`bass`] the root from A1
//! up with its fifth within the octave (D2 A1, G2 D2, A1 E2), [`voicing`] the chord in root position from G3 up, [`cimb`]
//! the chord's tones from C#5 to D6.

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip, noise};

pub use self::classic::BalkanFireClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options, no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "balkan_fire",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(BalkanFireClassic::new())),
};

/// #28's loop, i - i - iv - V.
const LOOP: [&str; 4] = ["Dm", "Dm", "Gm", "A"];

/// What the interface says about the Balkan fire.
pub static CARD: ThemeCard = ThemeCard {
    display: "Balkan fire",
    text: "A dark, fast Balkan oom-pah: upright bass, musette accordion stabs and a cimbalom shimmer that builds \
           through the fight. No drums of its own: the fighters keep the beat.",
    key: "D",
    scale: "harmonic minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The episode that played the Balkan fire (#28; `tools/studio_export/older/configs/drums_vs_violin`).
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_drums_vs_violin",
    fighters: ["DRUMS", "VIOLIN"],
    bpm: 140.0,
    key: "D",
    scale: "harmonic minor",
    intro: ["Dm", "A"],
    chord_loop: &LOOP,
    tail: &["Gm", "A"],
}];

/// Python's `int(x)` for a length (never below 0).
fn length(x: f64) -> usize {
    if x > 0.0 { x as usize } else { 0 }
}

/// `BASS[ch]`: the root from A1 up (on 1) and its fifth within A1 to G#2 (on 3).
///
/// # Errors
/// `ch` does not parse.
pub fn bass(harmony: &Harmony, ch: &str) -> Result<(i32, i32), HarmonyError> {
    let r = harmony.root_above(ch, 33)?;
    let f5 = if r + 7 <= 44 { r + 7 } else { r - 5 };
    Ok((r, f5))
}

/// `VOICING[ch]`: the accordion's chord, in root position from G3 up (D4 F4 A4, G3 Bb3 D4, A3 C#4 E4).
///
/// # Errors
/// `ch` does not parse.
pub fn voicing(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    let root = harmony.root_above(ch, 55)?;
    let mut v = harmony.chord_tones(root, root + 11, ch)?;
    v.truncate(4);
    Ok(v)
}

/// `CIMB[ch]`: the cimbalom's arpeggio tones, the chord's from C#5 to D6 (at most four).
///
/// # Errors
/// `ch` does not parse.
pub fn cimb(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    let mut v = harmony.chord_tones(73, 86, ch)?;
    v.truncate(4);
    Ok(v)
}

/// `upright(m, dur, vel)`: an upright bass pluck: a round fundamental, a woody thump, a quick decay.
pub fn upright(m: i32, dur: f64, vel: f64) -> Vec<f64> {
    let n = length((dur + 0.1) * SRF);
    let f = synth::mtof(f64::from(m));
    let t = synth::tt(n);
    let thump = synth::bp(&noise(u64::from(m.unsigned_abs()), n), 300.0, 1600.0, FILTER_O);
    let dec = if dur * 0.7 > 0.18 { dur * 0.7 } else { 0.18 };
    let fd = length(0.05 * SRF).min(n);
    let fade = dsp::linspace(1.0, 0.0, fd);
    let x: Vec<f64> = (0..n)
        .map(|i| {
            let t = t[i];
            let w = 2.0 * PI * f * t;
            let mut x = dmath::sin(w)
                + 0.35 * dmath::sin(2.0 * w) * dmath::exp(-t / 0.12)
                + 0.12 * dmath::sin(3.0 * w) * dmath::exp(-t / 0.06);
            x *= dmath::exp(-t / dec) * clip(t / 0.003, 0.0, 1.0);
            x += 0.25 * thump[i] * dmath::exp(-t / 0.008);
            if i >= n - fd {
                x *= fade[i - (n - fd)];
            }
            dmath::tanh(1.6 * x) / dmath::tanh(1.6)
        })
        .collect();
    synth::lp(&x, 1500.0, FILTER_O).iter().map(|v| v * vel).collect()
}

/// `reed(ms, dur, att, rel, cut, musette)`: accordion reeds: a pulse and a saw a note, three reeds detuned (the
/// musette), a little bellows tremolo.
pub fn reed(ms: &[i32], dur: f64, att: f64, rel: f64, cut: f64, musette: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    for &m in ms {
        let f = synth::mtof(f64::from(m));
        for d in [-musette, 0.0, musette] {
            for (o, &t) in x.iter_mut().zip(&t) {
                let ph = (f * (1.0 + d) * t).rem_euclid(1.0);
                *o += 0.6 * if ph < 0.32 { 1.0 } else { -0.45 } + 0.4 * (2.0 * ph - 1.0);
            }
        }
    }
    let k = 3.0 * (ms.len().max(1) as f64);
    for (o, &t) in x.iter_mut().zip(&t) {
        *o = *o / k * (1.0 + 0.06 * dmath::sin(2.0 * PI * 5.3 * t));
    }
    let r = if rel < dur * 0.4 { rel } else { dur * 0.4 };
    let env = synth::adsr(n, att, 0.08, 0.8, r);
    synth::lp(&x, cut, FILTER_O)
        .iter()
        .zip(&env)
        .map(|(v, e)| v * e)
        .collect()
}

/// `reed`'s defaults: attack, release, cut-off and the musette's detune.
pub const REED_ATT: f64 = 0.01;
pub const REED_REL: f64 = 0.05;
pub const REED_CUT: f64 = 2600.0;
pub const MUSETTE: f64 = 0.006;

/// `reed(ms, dur)` with its defaults: an accordion stab.
pub fn stab(ms: &[i32], dur: f64) -> Vec<f64> {
    reed(ms, dur, REED_ATT, REED_REL, REED_CUT, MUSETTE)
}

/// `cimbalom(m, vel)`: hammered strings: a bright attack, two slightly detuned strings, ringing.
pub fn cimbalom(m: i32, vel: f64) -> Vec<f64> {
    let n = length(0.7 * SRF);
    let f = synth::mtof(f64::from(m));
    synth::tt(n)
        .iter()
        .map(|&t| {
            let w = 2.0 * PI * f * t;
            let x = (dmath::sin(w) + dmath::sin(w * 1.004)) * 0.5
                + 0.4 * dmath::sin(2.0 * w) * dmath::exp(-t / 0.08)
                + 0.2 * dmath::sin(3.01 * w) * dmath::exp(-t / 0.05);
            x * dmath::exp(-t / 0.22) * vel
        })
        .collect()
}

/// `sticks(vel)`: the sticks clicked together for the count.
pub fn sticks(vel: f64) -> Vec<f64> {
    let n = length(0.09 * SRF);
    let t = synth::tt(n);
    let click = synth::bp(&noise(9, n), 1800.0, 6500.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            (click[i] * dmath::exp(-t / 0.012)
                + 0.6 * dmath::sin(2.0 * PI * 2350.0 * t) * dmath::exp(-t / 0.02)
                + 0.3 * dmath::sin(2.0 * PI * 3900.0 * t) * dmath::exp(-t / 0.01))
                * vel
        })
        .collect()
}

/// `roll(dur, f0, step)`: a snare roll swelling from `f0` to full (#28's rolled on the DRUMS fighter's snare; the
/// bed's own is a plain snare stroke, so it sounds under any pair).
pub fn roll(dur: f64, f0: f64, step: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let mut out = vec![0.0; n + bf_sound::SR];
    let stroke = synth::snare(0.06);
    let mut t = 0.0;
    while t < dur - 1e-4 {
        let a = length(t * SRF);
        let u = t / dur;
        let g = (f0 + (1.0 - f0) * dmath::pow(u, 1.5)) * (0.4 + 0.6 * u);
        for (o, s) in out[a..].iter_mut().zip(&stroke) {
            *o += s * g;
        }
        t += step;
    }
    out.truncate(n);
    out
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};

    use super::*;

    #[test]
    fn the_rules_give_the_episodes_tables() {
        let g = Grid::new(140.0);
        let all: Vec<String> = ["Dm", "Gm", "A"].iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2], &all, &[], g.n_bars);
        let h = Harmony::new(&g, "D", scale("harmonic minor").unwrap(), bars, Some(all), 2).unwrap();
        assert_eq!(bass(&h, "Dm").unwrap(), (38, 33));
        assert_eq!(bass(&h, "Gm").unwrap(), (43, 38));
        assert_eq!(bass(&h, "A").unwrap(), (33, 40));
        assert_eq!(voicing(&h, "Dm").unwrap(), [62, 65, 69]);
        assert_eq!(voicing(&h, "A").unwrap(), [57, 61, 64]);
        assert_eq!(cimb(&h, "Dm").unwrap(), [74, 77, 81, 86]);
    }

    #[test]
    fn the_instruments_sound() {
        for x in [
            upright(38, 0.42, 1.0),
            stab(&[62, 65, 69], 0.2),
            cimbalom(74, 1.0),
            sticks(1.0),
            roll(0.5, 0.3, 0.05),
        ] {
            let peak = x.iter().fold(0.0f64, |a, v| a.max(v.abs()));
            assert!(peak > 0.01 && peak.is_finite(), "{peak}");
        }
    }
}
