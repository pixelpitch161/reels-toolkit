//! Theme: dark sea shanty, the bed of `projects/ballfight_bagpipes_vs_accordion/audio.py` (#30 v3), quick-ported by
//! ear (ruling M4-R16: close enough, no reference mood, no parity gates). A ship's crew on a wooden deck: boots
//! stomping on 1 and 3, gang claps on 2 and 4, a low male choir humming the chord, a bass singer's "dum" on the
//! stomps, a stomp pickup on the "and" of 4 as the fight heats up, a bodhran-ish frame drum driving 8ths in the back
//! half. No lead and no accordion of its own: the top and the chords belong to the fighters. Built for A dorian
//! (Am G Am Em).
//!
//! Here: the voicings and the instruments; the arrangement is [`SeaShantyClassic`], in `classic.rs`.
//!
//! **Voicings.** The episode's tables (the bass singer's root and fifth, the choir's spread chord) for its four
//! chords, and for any other chord the same shape built from the chord's own tones ([`bass`], [`choir_voicing`]).

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip, noise};

pub use self::classic::SeaShantyClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options, no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "sea_shanty",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(SeaShantyClassic::new())),
};

/// What the interface says about the sea shanty.
pub static CARD: ThemeCard = ThemeCard {
    display: "Sea shanty",
    text: "A dark sea shanty: stomping boots, gang claps and a low choir. Leaves the tune to the fighters.",
    key: "A",
    scale: "dorian",
    chord_loops: &[&["Am", "G", "Am", "Em"]],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The episode that plays the sea shanty (#30): its music as its `timeline.py` sets it.
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_bagpipes_vs_accordion",
    fighters: ["BAGPIPES", "ACCORDION"],
    bpm: 122.0,
    key: "A",
    scale: "dorian",
    intro: ["Am", "E"],
    chord_loop: &["Am", "G", "Am", "Em"],
    tail: &["E"],
}];

/// `BASS`: the bass singer's root (on beat 1) and fifth (on beat 3), as #30 voiced them.
const BASS: [(&str, (i32, i32)); 4] = [("Am", (45, 40)), ("G", (43, 38)), ("Em", (40, 47)), ("E", (40, 47))];

/// `CHOIR`: the choir's spread chord, as #30 voiced it.
const CHOIR: [(&str, &[i32]); 4] = [
    ("Am", &[33, 45, 52, 57, 60]),
    ("G", &[31, 43, 50, 55, 59]),
    ("Em", &[28, 40, 47, 52, 55]),
    ("E", &[28, 40, 47, 52, 56]),
];

/// The chord's root from E2 up (E2 to D#3) and its third and fifth above the root, in semitones (a chord of
/// fewer tones takes a minor third and a perfect fifth for what it lacks).
fn shape(harmony: &Harmony, chord: &str) -> Result<(i32, i32, i32), HarmonyError> {
    let r = harmony.root_above(chord, 40)?;
    let pcs = harmony.chord_pcs(chord)?;
    let up = |i: usize, or: i32| pcs.get(i).map_or(or, |p| (p - pcs[0]).rem_euclid(12));
    Ok((r, up(1, 3), up(2, 7)))
}

/// The bass singer's (root, fifth) for `chord`: the table's, else the root from E2 up and the fifth under it when
/// it would pass B2.
///
/// # Errors
/// `chord` is in neither the table nor the episode's chords.
pub fn bass(harmony: &Harmony, chord: &str) -> Result<(i32, i32), HarmonyError> {
    if let Some((_, rf)) = BASS.iter().find(|(c, _)| *c == chord) {
        return Ok(*rf);
    }
    let (r, _, fifth) = shape(harmony, chord)?;
    let f = r + fifth;
    Ok((r, if f > 47 { f - 12 } else { f }))
}

/// The choir's spread chord for `chord`: the table's, else the root an octave down, the root, its fifth, the root
/// an octave up and the third above that.
///
/// # Errors
/// `chord` is in neither the table nor the episode's chords.
pub fn choir_voicing(harmony: &Harmony, chord: &str) -> Result<Vec<i32>, HarmonyError> {
    if let Some((_, v)) = CHOIR.iter().find(|(c, _)| *c == chord) {
        return Ok(v.to_vec());
    }
    let (r, third, fifth) = shape(harmony, chord)?;
    Ok(vec![r - 12, r, r + fifth, r + 12, r + 12 + third])
}

/// The chord's root, third, fifth and octave from E2 up (the bass singer's walk into the title slam).
///
/// # Errors
/// `chord` is not one of the episode's chords.
pub fn walk(harmony: &Harmony, chord: &str) -> Result<[i32; 4], HarmonyError> {
    let (r, third, fifth) = shape(harmony, chord)?;
    Ok([r, r + third, r + fifth, r + 12])
}

fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// `stomp(vel, seed)`: a boot on a wooden deck: a low thump and the board's knock.
pub fn stomp(vel: f64, seed: u64) -> Vec<f64> {
    let n = length(0.35 * SRF);
    let t = synth::tt(n);
    let f: Vec<f64> = t.iter().map(|&t| 52.0 + 70.0 * dmath::exp(-t / 0.03)).collect();
    let knock = synth::bp(&noise(seed, n), 180.0, 1400.0, FILTER_O);
    dsp::cumsum(&f)
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let body = dmath::sin(2.0 * PI * c / SRF) * dmath::exp(-t[i] / 0.13);
            let knock = knock[i] * dmath::exp(-t[i] / 0.022);
            (dmath::tanh(1.8 * body) * 0.9 + 0.55 * knock) * vel
        })
        .collect()
}

/// `gang_clap(vel)` before its velocity: a few hands clapping a hair apart (it draws no noise of its own).
pub fn gang_clap_hands() -> Vec<f64> {
    let mut out = vec![0.0; length(0.32 * SRF)];
    let c = synth::clap();
    for (j, d) in [0.0, 0.006, 0.013, 0.019].into_iter().enumerate() {
        let i = length(d * SRF);
        let w = 1.0 - 0.15 * j as f64;
        for (o, s) in out[i..].iter_mut().zip(&c) {
            *o += s * w;
        }
    }
    synth::bp(&out, 600.0, 7000.0, FILTER_O)
        .iter()
        .map(|v| v * 0.6)
        .collect()
}

/// `frame_drum(low, 1.0)`: a bodhran-ish frame drum: a pitched skin thump (`low`) or a rim tap.
pub fn frame_drum(low: bool) -> Vec<f64> {
    let n = length(0.3 * SRF);
    let t = synth::tt(n);
    if low {
        let f: Vec<f64> = t.iter().map(|&t| 82.0 + 40.0 * dmath::exp(-t / 0.02)).collect();
        let slap = synth::bp(&noise(5, n), 300.0, 2000.0, FILTER_O);
        dsp::cumsum(&f)
            .iter()
            .enumerate()
            .map(|(i, c)| {
                dmath::sin(2.0 * PI * c / SRF) * dmath::exp(-t[i] / 0.09) + 0.25 * slap[i] * dmath::exp(-t[i] / 0.01)
            })
            .collect()
    } else {
        let tap = synth::bp(&noise(6, n), 1200.0, 5000.0, FILTER_O);
        (0..n)
            .map(|i| {
                tap[i] * dmath::exp(-t[i] / 0.018) * 0.7
                    + 0.3 * dmath::sin(2.0 * PI * 410.0 * t[i]) * dmath::exp(-t[i] / 0.03)
            })
            .collect()
    }
}

/// The choir's vowel: the hum (`'m'`) or the open "oh" (`'o'`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vowel {
    /// The closed hum.
    M,
    /// The open "oh".
    O,
}

/// `choir(ms, dur, att, rel, vowel)`: a low male choir: three detuned saws a note with a slow vibrato, through
/// "mm" or "oh" formants.
pub fn choir(ms: &[i32], dur: f64, att: f64, rel: f64, vowel: Vowel) -> Vec<f64> {
    let n = length((dur + rel) * SRF);
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    for &m in ms {
        let f = synth::mtof(f64::from(m));
        for (d, ph0) in [(-0.004, 0.1), (0.0, 0.5), (0.005, 0.8)] {
            let rate = 4.6 + 3.0 * d * 100.0;
            let fr: Vec<f64> = t
                .iter()
                .map(|&t| f * (1.0 + d) * (1.0 + 0.004 * dmath::sin(2.0 * PI * rate * t + ph0 * 6.0)))
                .collect();
            for (v, c) in x.iter_mut().zip(dsp::cumsum(&fr)) {
                let ph = (c / SRF + ph0).rem_euclid(1.0);
                *v += 2.0 * ph - 1.0;
            }
        }
    }
    let voices = 3.0 * ms.len().max(1) as f64;
    for v in &mut x {
        *v /= voices;
    }
    let ((a, b, ga), (c, d, gb)) = match vowel {
        Vowel::M => ((180.0, 420.0, 1.0), (700.0, 1100.0, 0.25)),
        Vowel::O => ((300.0, 650.0, 1.0), (750.0, 1250.0, 0.45)),
    };
    let lo = synth::bp(&x, a, b, FILTER_O);
    let hi = synth::bp(&x, c, d, FILTER_O);
    let y: Vec<f64> = lo.iter().zip(&hi).map(|(l, h)| l * ga + h * gb).collect();
    synth::lp(&y, 1600.0, FILTER_O)
        .iter()
        .zip(&t)
        .map(|(v, &t)| v * clip(t / att, 0.0, 1.0) * clip((dur + rel - t) / rel, 0.0, 1.0))
        .collect()
}

/// `bass_voice(m, dur, vel)`: the bass singer's "dum" on the stomps.
pub fn bass_voice(m: i32, dur: f64, vel: f64) -> Vec<f64> {
    let n = length((dur + 0.1) * SRF);
    let t = synth::tt(n);
    let f = synth::mtof(f64::from(m));
    let x: Vec<f64> = t
        .iter()
        .map(|&t| {
            let ph = (f * t).rem_euclid(1.0);
            (2.0 * ph - 1.0) * 0.6 + dmath::sin(2.0 * PI * f * t)
        })
        .collect();
    let lo = synth::bp(&x, 60.0, 380.0, FILTER_O);
    let hi = synth::bp(&x, 500.0, 900.0, FILTER_O);
    let tail = (dur * 0.6).max(0.12);
    (0..n)
        .map(|i| {
            let env = clip(t[i] / 0.02, 0.0, 1.0) * dmath::exp(-(t[i] - 0.05).max(0.0) / tail);
            dmath::tanh(1.4 * (lo[i] + 0.3 * hi[i]) * env) * vel
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};

    use super::*;

    fn harmony(chords: &[&str]) -> Harmony {
        let g = Grid::new(122.0);
        let all: Vec<String> = chords.iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2], &all, &[], g.n_bars);
        Harmony::new(&g, "A", scale("dorian").unwrap(), bars, Some(all), 2).unwrap()
    }

    #[test]
    fn the_rule_gives_the_tables_shape() {
        let h = harmony(&["Am", "G", "Em", "E", "Dm", "F", "C7", "Bb"]);
        // The rule gives the tables' answers for the tables' chords.
        for (ch, rf) in BASS {
            let (r, _, fifth) = shape(&h, ch).unwrap();
            let f = r + fifth;
            assert_eq!((r, if f > 47 { f - 12 } else { f }), rf, "{ch}");
        }
        for (ch, v) in CHOIR {
            let (r, third, fifth) = shape(&h, ch).unwrap();
            assert_eq!([r - 12, r, r + fifth, r + 12, r + 12 + third], v, "{ch}");
        }
        assert_eq!(bass(&h, "Dm").unwrap(), (50, 45));
        assert_eq!(choir_voicing(&h, "Dm").unwrap(), [38, 50, 57, 62, 65]);
        assert_eq!(choir_voicing(&h, "Bb").unwrap(), [34, 46, 53, 58, 62]);
        assert_eq!(walk(&h, "E").unwrap(), [40, 44, 47, 52]);
    }

    #[test]
    fn the_instruments_sound() {
        for x in [
            stomp(1.0, 3),
            gang_clap_hands(),
            frame_drum(true),
            frame_drum(false),
            choir(&[33, 45, 52], 0.5, 0.05, 0.3, Vowel::M),
            choir(&[], 0.2, 0.01, 0.1, Vowel::O),
            bass_voice(45, 0.34, 1.0),
        ] {
            assert!(x.iter().all(|v| v.is_finite()));
        }
        assert!(stomp(1.0, 3).iter().any(|v| v.abs() > 0.3));
        assert!(
            choir(&[45, 52], 0.5, 0.05, 0.3, Vowel::O)
                .iter()
                .any(|v| v.abs() > 0.01)
        );
    }
}
