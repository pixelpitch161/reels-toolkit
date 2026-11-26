//! Theme: brass house (the bed of `projects/ballfight_trombone_vs_tuba/audio.py`, #36 TROMBONE vs TUBA),
//! quick-ported by ear (ruling M4-R16: close enough, no parity gates). A dark minor house groove with **no brass and
//! no bass of its own** (in #36 the TUBA was the low end and the TROMBONE the hook): a short four-on-the-floor kick
//! that never masks a held bass note, claps on 2 and 4, off-beat hats, a 16th shaker, a filtered minor pad pumping
//! under the kick, and syncopated synth chord stabs (high, short, not brass). The back half opens up (open hats, a
//! ride, a brighter pad, more stabs) after a filter-sweep build. Built for F minor (Fm Db Bbm C); any chord works
//! (voicings by rule).
//!
//! Here: the voicings and the instruments, after the episode's functions of the same names. The arrangement is
//! [`BrassHouseClassic`], in `classic.rs`.
//!
//! **Voicings.** The episode's tables are rules here, for every chord the harmony parses: [`pad_voicing`] the
//! chord's tones from F4 up (four), [`stab_voicing`] its tones from C5 up (three).

mod classic;
pub mod layers;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_sound::SRF;
use bf_sound::synth::{self, FILTER_O, noise};

pub use self::classic::BrassHouseClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options, no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "brass_house",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(BrassHouseClassic::new())),
    composed: Some(|o, c, s| Ok(Box::new(layers::composed(o, c, s)?))),
};

/// #36's loop, i - bVI - iv - V.
const LOOP: [&str; 4] = ["Fm", "Db", "Bbm", "C"];

/// What the interface says about the brass house.
pub static CARD: ThemeCard = ThemeCard {
    display: "Brass house",
    text: "A dark minor house groove: a short four-on-the-floor kick, claps, hats and a pumping pad with chord stabs. \
           No bass and no brass of its own: the fighters are the low end and the hook.",
    key: "F",
    scale: "minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The episode that played the brass house (#36; TROMBONE vs TUBA's own music, Q5).
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_trombone_vs_tuba",
    fighters: ["TROMBONE", "TUBA"],
    bpm: 126.0,
    key: "F",
    scale: "minor",
    intro: ["Fm", "C"],
    chord_loop: &LOOP,
    tail: &["Db", "C"],
}];

/// Python's `int(x)` for a length (never below 0).
fn length(x: f64) -> usize {
    if x > 0.0 { x as usize } else { 0 }
}

/// `PAD[ch]`: the pad's chord, its tones from F4 up (at most four: Bbm F4 Bb4 Db5 F5, C G4 C5 E5 G5).
///
/// # Errors
/// `ch` does not parse.
pub fn pad_voicing(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    let mut v = harmony.chord_tones(65, 84, ch)?;
    v.truncate(4);
    Ok(v)
}

/// `STAB[ch]`: the stab's chord, its tones from C5 up (three: Fm C5 F5 Ab5, Db Db5 F5 Ab5, C C5 E5 G5).
///
/// # Errors
/// `ch` does not parse.
pub fn stab_voicing(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    let mut v = harmony.chord_tones(72, 86, ch)?;
    v.truncate(3);
    Ok(v)
}

/// `kick(vel)`: a short, punchy house kick: the body is over in about 0.2 s, so a held bass note stays on top.
pub fn kick(vel: f64) -> Vec<f64> {
    let mut x = synth::kick(1.0, 0.16, 160.0, 52.0);
    x.truncate(length(0.32 * SRF));
    synth::hp(&x, 48.0, FILTER_O).iter().map(|v| v * vel).collect()
}

/// `hat(vel, open_, seed)`: high-passed noise, closed (90 ms) or open (320 ms).
pub fn hat(vel: f64, open: bool, seed: u64) -> Vec<f64> {
    let n = length(if open { 0.32 } else { 0.09 } * SRF);
    let dec = if open { 0.12 } else { 0.025 };
    let x = synth::hp(&noise(seed, n), 7500.0, FILTER_O);
    synth::tt(n)
        .iter()
        .zip(&x)
        .map(|(&t, v)| v * dmath::exp(-t / dec) * 0.5 * vel)
        .collect()
}

/// `ride(vel, seed)`: a washy ride with its bell.
pub fn ride(vel: f64, seed: u64) -> Vec<f64> {
    let n = length(0.6 * SRF);
    let x = synth::hp(&noise(seed, n), 5200.0, FILTER_O);
    synth::tt(n)
        .iter()
        .zip(&x)
        .map(|(&t, v)| {
            (v * dmath::exp(-t / 0.25) * 0.4 + 0.18 * dmath::sin(2.0 * PI * 3170.0 * t) * dmath::exp(-t / 0.3)) * vel
        })
        .collect()
}

/// `shaker(vel, seed)`: a 70 ms band of noise.
pub fn shaker(vel: f64, seed: u64) -> Vec<f64> {
    let n = length(0.07 * SRF);
    let x = synth::bp(&noise(seed, n), 4000.0, 10000.0, FILTER_O);
    synth::tt(n)
        .iter()
        .zip(&x)
        .map(|(&t, v)| v * synth::clip(t / 0.01, 0.0, 1.0) * dmath::exp(-t / 0.02) * vel)
        .collect()
}

/// `stab(ms, dur, cut)`: a house chord stab: detuned saws and a square an octave down, a fast filter envelope
/// (synth, not brass).
pub fn stab(ms: &[i32], dur: f64, cut: f64) -> Vec<f64> {
    let n = length((dur + 0.12) * SRF);
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    for &m in ms {
        for (o, s) in x.iter_mut().zip(synth::saw(synth::mtof(f64::from(m)), n, 0.008, 2)) {
            *o += s;
        }
    }
    let k = ms.len().max(1) as f64;
    for o in &mut x {
        *o /= k;
    }
    if let Some(&m) = ms.first() {
        for (o, s) in x.iter_mut().zip(synth::square(synth::mtof(f64::from(m - 12)), n, 0.4)) {
            *o += 0.3 * s;
        }
    }
    let mut out = vec![0.0; n];
    let nb = 10;
    for kb in 0..nb {
        let (a, b) = (kb * n / nb, (kb + 1) * n / nb);
        if b <= a {
            continue;
        }
        let fc = 400.0 + cut * dmath::exp(-((a + b) as f64 / 2.0 / SRF) / 0.05);
        let y = synth::lp(&x[a.saturating_sub(2048)..b], fc, FILTER_O);
        out[a..b].copy_from_slice(&y[y.len() - (b - a)..]);
    }
    let env = synth::adsr(n, 0.003, 0.05, 0.4, 0.08);
    (0..n)
        .map(|i| out[i] * env[i] * synth::clip((dur + 0.06 - t[i]) / 0.06, 0.0, 1.0))
        .collect()
}

/// `pump(n, t0, depth)`: the sidechain pump: the pad ducks under every kick (beats from `t0`) and swells back.
pub fn pump(n: usize, t0: f64, beat: f64, depth: f64) -> Vec<f64> {
    synth::tt(n)
        .iter()
        .map(|&t| 1.0 - depth * dmath::exp(-(t + t0).rem_euclid(beat) / 0.13))
        .collect()
}

/// `pad_bar(ch, b, dur, cut, att)`: the minor pad for one bar from `t0`, pumping under the kick.
pub fn pad_bar(ms: &[i32], t0: f64, beat: f64, dur: f64, cut: f64, att: f64) -> Vec<f64> {
    let ms: Vec<f64> = ms.iter().map(|&m| f64::from(m)).collect();
    let mut x = synth::pad(&ms, dur, cut, att);
    x.truncate(length(dur * SRF));
    let p = pump(x.len(), t0, beat, 0.6);
    x.iter().zip(&p).map(|(v, g)| v * g).collect()
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};

    use super::*;

    #[test]
    fn the_rules_give_the_episodes_tables() {
        let g = Grid::new(126.0);
        let all: Vec<String> = ["Fm", "C", "Db", "Bbm"].iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2], &all, &[], g.n_bars);
        let h = Harmony::new(&g, "F", scale("minor").unwrap(), bars, Some(all), 2).unwrap();
        assert_eq!(pad_voicing(&h, "Bbm").unwrap(), [65, 70, 73, 77]);
        assert_eq!(pad_voicing(&h, "C").unwrap(), [67, 72, 76, 79]);
        assert_eq!(stab_voicing(&h, "Fm").unwrap(), [72, 77, 80]);
        assert_eq!(stab_voicing(&h, "Db").unwrap(), [73, 77, 80]);
        assert_eq!(stab_voicing(&h, "Bbm").unwrap(), [73, 77, 82]);
        assert_eq!(stab_voicing(&h, "C").unwrap(), [72, 76, 79]);
    }

    #[test]
    fn the_instruments_sound() {
        for x in [
            kick(1.0),
            hat(0.9, false, 10),
            hat(0.9, true, 20),
            ride(0.8, 30),
            shaker(1.0, 40),
            stab(&[72, 77, 80], 0.1, 2200.0),
            pad_bar(&[65, 68, 72, 77], 0.0, 0.48, 1.9, 1200.0, 0.06),
        ] {
            let peak = x.iter().fold(0.0f64, |a, v| a.max(v.abs()));
            assert!(peak > 0.01 && peak.is_finite(), "{peak}");
        }
    }
}
