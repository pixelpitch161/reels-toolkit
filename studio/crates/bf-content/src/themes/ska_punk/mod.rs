//! Theme: ska punk (the band of `projects/ballfight_bassoon_vs_barisax/audio.py`, #39 v4 BASSOON vs BARI SAX),
//! quick-ported by ear (ruling M4-R16: close enough, no parity gates). Fast, dark ska-punk with **no horns of its
//! own** (in #39 the two low reeds were the hook and the bass): driving punk drums (kick, snare on 2 and 4, 8th hats,
//! crashes; every-beat kick, double-time snare, open hats and a ride as it thickens), a guitar upstroke skank on every
//! off-beat and an organ bubble (the off-beat chord and its 16th bounce), tom fills into every 4th bar. The ring
//! ropes play a plucked bass under the band hits and the victory. It builds with parts and density, never with level.
//! Built for E minor at the fastest tempo of the series (Em C D B); any chord works (voicings by rule).
//!
//! Here: the voicings and the instruments, after the episode's functions of the same names. The arrangement is
//! [`SkaPunkClassic`], in `classic.rs`.
//!
//! **Voicings.** The episode's tables are rules here, for every chord the harmony parses: [`skank_tones`] the
//! chord's top three tones from E4 to G5, [`organ_voicing`] its three tones from E3 up, [`rope_notes`] its root from
//! B1 up and the fifth above.

mod classic;
pub mod layers;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::synth::{self, FILTER_O, clip, noise};

pub use self::classic::SkaPunkClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options, no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "ska_punk",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(SkaPunkClassic::new())),
};

/// #39's loop, i - bVI - bVII - V.
const LOOP: [&str; 4] = ["Em", "C", "D", "B"];

/// What the interface says about the ska punk.
pub static CARD: ThemeCard = ThemeCard {
    display: "Ska punk",
    text: "Fast, dark ska-punk: driving punk drums, a guitar skank on every off-beat and an organ bubble, thickening \
           as the fight goes. No horns of its own: the fighters are the hook and the bass.",
    key: "E",
    scale: "minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 43),
};

/// The episode that played the ska punk (#39 v4; `tools/studio_export/older/configs/bassoon_vs_barisax`). The first
/// (the default) is at 140 BPM, where the seed search finds fair fights readily; the second is the episode's own 170
/// BPM, where fair seeds are rarer (7 in 4,000 balanced against 10 at 140).
pub static PRESETS: [Preset; 2] = [
    Preset {
        config: "ballfight_bassoon_vs_barisax",
        fighters: ["BASSOON", "BARI SAX"],
        bpm: 140.0,
        key: "E",
        scale: "minor",
        intro: ["Em", "B"],
        chord_loop: &LOOP,
        tail: &["B"],
    },
    Preset {
        config: "ballfight_bassoon_vs_barisax",
        fighters: ["BASSOON", "BARI SAX"],
        bpm: 170.0,
        key: "E",
        scale: "minor",
        intro: ["Em", "B"],
        chord_loop: &LOOP,
        tail: &["B"],
    },
];

/// Python's `int(x)` for a length (never below 0).
fn length(x: f64) -> usize {
    if x > 0.0 { x as usize } else { 0 }
}

/// The guitar's three strings: the chord's top three tones from E4 to G5 (Em G4 B4 E5... as `chord_tones(64, 79)[-3:]`).
///
/// # Errors
/// `ch` does not parse.
pub fn skank_tones(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    let v = harmony.chord_tones(64, 79, ch)?;
    Ok(v[v.len().saturating_sub(3)..].to_vec())
}

/// `ORG_V[ch]`: the organ's mid voicing, the chord's three tones from E3 up (Em E3 G3 B3, B F#3 B3 D#4).
///
/// # Errors
/// `ch` does not parse.
pub fn organ_voicing(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    let mut v = harmony.chord_tones(52, 66, ch)?;
    v.truncate(3);
    Ok(v)
}

/// `ROPE_NOTES[ch][:2]`: the ropes' bass, the root from B1 up and its fifth (Em E2 B2, B B1 F#2).
///
/// # Errors
/// `ch` does not parse.
pub fn rope_notes(harmony: &Harmony, ch: &str) -> Result<(i32, i32), HarmonyError> {
    let r = harmony.root_above(ch, 35)?;
    Ok((r, r + 7))
}

/// `rope_pluck(m, dur, vel, seed)`: a ring rope as a bass string: a plucked low tone, a fibrous buzz on the attack.
pub fn rope_pluck(m: i32, dur: f64, vel: f64, seed: u64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let f = synth::mtof(f64::from(m));
    let buzz = synth::bp(&noise(seed + u64::from(m.unsigned_abs()), n), 300.0, 2500.0, FILTER_O);
    let len = if dur > 0.3 { dur } else { 0.3 };
    (0..n)
        .map(|i| {
            let t = t[i];
            let mut x = 0.0;
            for k in 1..9 {
                let kf = f64::from(k);
                x += (1.0 / dmath::pow(kf, 1.1))
                    * dmath::sin(2.0 * PI * f * kf * t + 0.3 * kf)
                    * dmath::exp(-t * (1.1 + 1.4 * kf) / len);
            }
            let b = buzz[i] * dmath::exp(-t / 0.025) * 0.4;
            dmath::tanh(1.6 * (x + b)) * clip(t / 0.002, 0.0, 1.0) * vel
        })
        .collect()
}

/// `skank(ch, vel, seed)`: the guitar's upstroke: a choked high triad (high string first, a few ms strum), a pick
/// scratch, about 70 ms long.
pub fn skank(tones: &[i32], vel: f64, seed: u64) -> Vec<f64> {
    let n = length(0.11 * SRF);
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    for (j, &m) in tones.iter().rev().enumerate() {
        let d = length(j as f64 * 0.004 * SRF).min(n);
        let f = synth::mtof(f64::from(m));
        for (o, &tt) in x[d..].iter_mut().zip(&t) {
            let mut y = 0.0;
            for k in 1..7 {
                let kf = f64::from(k);
                y += (1.0 / kf) * dmath::sin(2.0 * PI * f * kf * tt + kf);
            }
            *o += y * dmath::exp(-tt / 0.035);
        }
    }
    let scratch = synth::bp(&noise(seed + 9, n), 2000.0, 7000.0, FILTER_O);
    let y: Vec<f64> = (0..n)
        .map(|i| dmath::tanh(1.8 * (x[i] / 3.0 + scratch[i] * dmath::exp(-t[i] / 0.006) * 0.5)))
        .collect();
    synth::hp(&y, 700.0, FILTER_O).iter().map(|v| v * vel).collect()
}

/// `organ(ms, dur, vel)`: a ska organ stab: drawbar harmonics, a key click, a little leslie wobble; thin (the low
/// end is the fighters').
pub fn organ(ms: &[i32], dur: f64, vel: f64) -> Vec<f64> {
    let n = length((dur + 0.02) * SRF);
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    for &m in ms {
        let f = synth::mtof(f64::from(m));
        for (h, a) in [(1.0, 1.0), (2.0, 0.7), (3.0, 0.45), (4.0, 0.35), (6.0, 0.15)] {
            for (o, &tt) in x.iter_mut().zip(&t) {
                *o += a * dmath::sin(2.0 * PI * f * h * tt * (1.0 + 0.002 * dmath::sin(2.0 * PI * 6.3 * tt)));
            }
        }
    }
    let k = ms.len().max(1) as f64 * 2.6;
    let click = synth::bp(&noise(ms.len() as u64, n), 1500.0, 6000.0, FILTER_O);
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let tt = t[i];
            let env = clip(tt / 0.003, 0.0, 1.0) * if tt < dur { 1.0 } else { dmath::exp(-(tt - dur) / 0.01) };
            (x[i] / k + click[i] * dmath::exp(-tt / 0.003) * 0.4) * env
        })
        .collect();
    synth::hp(&y, 280.0, FILTER_O).iter().map(|v| v * vel).collect()
}

/// `ride(seed)`: a bright ride and its bell.
pub fn ride(seed: u64) -> Vec<f64> {
    let n = length(0.5 * SRF);
    let x = synth::hp(&noise(seed + 31, n), 5000.0, FILTER_O);
    synth::tt(n)
        .iter()
        .zip(&x)
        .map(|(&t, v)| {
            v * dmath::exp(-t / 0.18) * 0.35 + 0.12 * dmath::sin(2.0 * PI * 3100.0 * t) * dmath::exp(-t / 0.3)
        })
        .collect()
}

/// `KICK`: a tight punk kick, out of a low honk's way.
pub fn kick() -> Vec<f64> {
    let mut x = synth::kick(1.0, 0.10, 170.0, 58.0);
    x.truncate(length(0.2 * SRF));
    synth::hp(&x, 55.0, FILTER_O)
}

/// `SNARE`: a snare with a clap layered in.
pub fn snare() -> Vec<f64> {
    let mut x = synth::snare(0.11);
    for (o, c) in x.iter_mut().zip(synth::clap()) {
        *o += c * 0.35;
    }
    x
}

/// `tom(m)`: a tom, its pitch dropping into `m`.
pub fn tom(m: i32) -> Vec<f64> {
    let n = length(0.3 * SRF);
    let f0 = synth::mtof(f64::from(m));
    let mut ph = 0.0;
    synth::tt(n)
        .iter()
        .map(|&t| {
            ph += f0 * (1.0 + 0.3 * dmath::exp(-t / 0.03)) / SRF;
            dmath::sin(2.0 * PI * ph) * dmath::exp(-t / 0.12)
        })
        .collect()
}

/// `thunder(dur, seed, crack)`: a low rumble, with a crack on top.
pub fn thunder(dur: f64, seed: u64, crack: bool) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let mut g = Generator::new(seed);
    let mut draw = vec![0.0; n];
    g.fill_normal(0.0, 1.0, &mut draw);
    let rumble = synth::lp(&draw, 140.0, FILTER_O);
    let cr = if crack {
        g.fill_normal(0.0, 1.0, &mut draw);
        synth::bp(&draw, 1500.0, 9000.0, FILTER_O)
    } else {
        vec![0.0; n]
    };
    (0..n)
        .map(|i| {
            rumble[i] * 3.0 * clip(t[i] / 0.03, 0.0, 1.0) * dmath::exp(-t[i] / (dur * 0.35))
                + cr[i] * dmath::exp(-t[i] / 0.03) * 0.9
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};

    use super::*;

    #[test]
    fn the_rules_give_the_episodes_tables() {
        let g = Grid::new(170.0);
        let all: Vec<String> = ["Em", "B", "C", "D"].iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2], &all, &[], g.n_bars);
        let h = Harmony::new(&g, "E", scale("minor").unwrap(), bars, Some(all), 2).unwrap();
        assert_eq!(organ_voicing(&h, "Em").unwrap(), [52, 55, 59]);
        assert_eq!(organ_voicing(&h, "C").unwrap(), [52, 55, 60]);
        assert_eq!(organ_voicing(&h, "D").unwrap(), [54, 57, 62]);
        assert_eq!(organ_voicing(&h, "B").unwrap(), [54, 59, 63]);
        assert_eq!(rope_notes(&h, "Em").unwrap(), (40, 47));
        assert_eq!(rope_notes(&h, "C").unwrap(), (36, 43));
        assert_eq!(rope_notes(&h, "B").unwrap(), (35, 42));
        assert_eq!(skank_tones(&h, "Em").unwrap().len(), 3);
    }

    #[test]
    fn the_instruments_sound() {
        for x in [
            rope_pluck(40, 1.0, 1.0, 0),
            skank(&[67, 71, 76], 1.0, 0),
            organ(&[52, 55, 59], 0.07, 1.0),
            ride(0),
            kick(),
            snare(),
            tom(47),
            thunder(1.4, 91, true),
        ] {
            let peak = x.iter().fold(0.0f64, |a, v| a.max(v.abs()));
            assert!(peak > 0.01 && peak.is_finite(), "{peak}");
        }
    }
}
