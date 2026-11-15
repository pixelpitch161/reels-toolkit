//! Theme: noir jazz, the bed of `projects/ballfight_trumpet_vs_sax/audio.py` (#29), quick-ported by ear (ruling
//! M4-R16: close enough, no reference mood, no parity gates). A fast, smoky C minor combo: a walking upright bass,
//! brushes (a swirl, taps on 2 and 4, a swung ride) that switch to sticks for the last third of the fight, muted
//! piano comping on the Charleston (rootless voicings, played on the PIANO's voice) and a dark pad. No horn of its
//! own: the leads belong to the fighters. Built for C minor (i - bVI7 - iv - V7).
//!
//! Here: the voicings and the instruments; the arrangement is [`NoirJazzClassic`], in `classic.rs`.
//!
//! **Voicings.** The episode's tables (the bass's root and chord tones, the piano's rootless voicing, the pad) for
//! its four chords, and for any other chord the same shape built from the chord's own tones ([`bass_root`],
//! [`bass_tones`], [`comp_voicing`], [`pad_voicing`]).

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_sound::SRF;
use bf_sound::synth::{self, FILTER_O, clip, noise};

pub use self::classic::NoirJazzClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The PIANO's Pixel Pitch voice id, which the bed's comping plays.
pub const PIANO: &str = "PIANO";

/// The theme as the registry lists it: no options; its bed plays the PIANO's voice.
pub static THEME: Theme = Theme {
    name: "noir_jazz",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[PIANO],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(NoirJazzClassic::new())),
};

/// #29's loop: i - bVI7 - iv - V7 in C minor.
const LOOP: [&str; 4] = ["Cm", "Ab7", "Fm", "G7"];

/// What the interface says about the noir jazz.
pub static CARD: ThemeCard = ThemeCard {
    display: "Noir jazz",
    text: "Fast, smoky noir jazz: a walking upright bass, brushes that switch to sticks, muted piano comping and a \
           dark pad. Leaves the horns to the fighters.",
    key: "C",
    scale: "minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The two stand-in pairs that play the noir jazz at #29's tempo, key and loop: its own lead and two saxes
/// (`tools/studio_export/older/configs/noir_jazz_a`, `noir_jazz_b`).
pub static PRESETS: [Preset; 2] = [
    Preset {
        config: "noir_jazz_a",
        fighters: ["TRUMPET", "TENOR SAX"],
        bpm: 132.0,
        key: "C",
        scale: "minor",
        intro: ["Cm", "G7"],
        chord_loop: &LOOP,
        tail: &[],
    },
    Preset {
        config: "noir_jazz_b",
        fighters: ["TRUMPET", "ALTO SAX"],
        bpm: 132.0,
        key: "C",
        scale: "minor",
        intro: ["Cm", "G7"],
        chord_loop: &LOOP,
        tail: &[],
    },
];

/// `SWING`: the off-beat 8th at 60 % of the beat.
pub const SWING: f64 = 0.6;

/// `CHORD_TONES_LO`: the walking bass's root and chord tones, as #29 voiced them.
const TONES_LO: [(&str, [i32; 4]); 4] = [
    ("Cm", [36, 39, 43, 46]),
    ("Ab7", [32, 36, 39, 42]),
    ("Fm", [41, 44, 36, 39]),
    ("G7", [43, 47, 38, 41]),
];

/// `VOICE`: the piano's rootless voicing, as #29 voiced it (its G7 altered).
const VOICE: [(&str, [i32; 4]); 4] = [
    ("Cm", [63, 67, 70, 74]),
    ("Ab7", [60, 63, 66, 70]),
    ("Fm", [56, 60, 63, 67]),
    ("G7", [65, 68, 71, 75]),
];

/// The chord's intervals above its root: third, fifth and seventh (a triad takes the minor seventh when minor and
/// the major seventh when major).
fn shape(harmony: &Harmony, chord: &str) -> Result<(i32, i32, i32), HarmonyError> {
    let pcs = harmony.chord_pcs(chord)?;
    let up = |i: usize, or: i32| pcs.get(i).map_or(or, |p| (p - pcs[0]).rem_euclid(12));
    let third = up(1, 3);
    let seventh = up(3, if third == 3 { 10 } else { 11 });
    Ok((third, up(2, 7), seventh))
}

/// `BASS_ROOT`: the walking bass's root, from Ab1 up (Ab1 to G2), as #29 voiced its four.
///
/// # Errors
/// `chord` is not one of the episode's chords.
pub fn bass_root(harmony: &Harmony, chord: &str) -> Result<i32, HarmonyError> {
    harmony.root_above(chord, 32)
}

/// `CHORD_TONES_LO[chord]`: the table's, else the root, its third, and its fifth and seventh (each an octave down
/// when it would pass Bb2).
///
/// # Errors
/// `chord` is not one of the episode's chords.
pub fn bass_tones(harmony: &Harmony, chord: &str) -> Result<[i32; 4], HarmonyError> {
    if let Some((_, v)) = TONES_LO.iter().find(|(c, _)| *c == chord) {
        return Ok(*v);
    }
    let r = bass_root(harmony, chord)?;
    let (third, fifth, seventh) = shape(harmony, chord)?;
    let low = |m: i32| if m > 46 { m - 12 } else { m };
    Ok([r, r + third, low(r + fifth), low(r + seventh)])
}

/// `VOICE[chord]`: the table's, else the rootless voicing: the third from Ab3 up, then the fifth, the seventh and
/// the ninth above it.
///
/// # Errors
/// `chord` is not one of the episode's chords.
pub fn comp_voicing(harmony: &Harmony, chord: &str) -> Result<[i32; 4], HarmonyError> {
    if let Some((_, v)) = VOICE.iter().find(|(c, _)| *c == chord) {
        return Ok(*v);
    }
    let (third, fifth, seventh) = shape(harmony, chord)?;
    let root = harmony.root_above(chord, 0)?;
    let t = 56 + (root + third - 56).rem_euclid(12);
    let r = t - third;
    let above = |m: i32, floor: i32| if m <= floor { m + 12 } else { m };
    let f = above(r + fifth, t);
    let s = above(r + seventh, f);
    let n = above(r + 14, s);
    Ok([t, f, s, n])
}

/// `PAD[chord]`: the root from F2 up, its fifth, and its third an octave up.
///
/// # Errors
/// `chord` is not one of the episode's chords.
pub fn pad_voicing(harmony: &Harmony, chord: &str) -> Result<[i32; 3], HarmonyError> {
    let r = harmony.root_above(chord, 41)?;
    let (third, fifth, _) = shape(harmony, chord)?;
    Ok([r, r + fifth, r + 12 + third])
}

fn length(x: f64) -> usize {
    if x > 0.0 { x as usize } else { 0 }
}

/// `upright(m, dur, vel)`: an upright bass pluck: a round fundamental, a woody thump, a quick decay.
pub fn upright(m: i32, dur: f64, vel: f64) -> Vec<f64> {
    let n = length((dur + 0.1) * SRF);
    let f = synth::mtof(f64::from(m));
    let t = synth::tt(n);
    let thump = synth::bp(&noise(m.unsigned_abs().into(), n), 300.0, 1600.0, FILTER_O);
    let fade = length(0.05 * SRF).min(n).max(1);
    let k = dmath::tanh(1.6);
    let x: Vec<f64> = (0..n)
        .map(|i| {
            let t = t[i];
            let mut x = dmath::sin(2.0 * PI * f * t)
                + 0.35 * dmath::sin(4.0 * PI * f * t) * dmath::exp(-t / 0.12)
                + 0.12 * dmath::sin(6.0 * PI * f * t) * dmath::exp(-t / 0.06);
            x *= dmath::exp(-t / (dur * 0.7).max(0.18)) * clip(t / 0.003, 0.0, 1.0);
            x += 0.25 * thump[i] * dmath::exp(-t / 0.008);
            if i + fade >= n {
                x *= (n - 1 - i) as f64 / fade as f64;
            }
            dmath::tanh(1.6 * x) / k
        })
        .collect();
    synth::lp(&x, 1500.0, FILTER_O).iter().map(|v| v * vel).collect()
}

/// `ride(1.0, bell)`: a ride cymbal (or its bell): inharmonic partials, a wash and a stick tick.
pub fn ride(bell: bool) -> Vec<f64> {
    let n = length(1.1 * SRF);
    let t = synth::tt(n);
    let parts: &[f64] = if bell {
        &[1220.0, 2950.0, 4420.0]
    } else {
        &[520.0, 1173.0, 2406.0, 3605.0, 5110.0, 6893.0]
    };
    let wash = synth::bp(&noise(31, n), 5000.0, 14000.0, FILTER_O);
    let tick = synth::bp(&noise(32, n), 3000.0, 9000.0, FILTER_O);
    let dec = if bell { 0.35 } else { 0.55 };
    (0..n)
        .map(|i| {
            let mut x = 0.0;
            let mut w = 1.0;
            for (j, f) in parts.iter().enumerate() {
                x += dmath::sin(2.0 * PI * f * t[i] + j as f64) * w;
                w *= 0.9;
            }
            x = x / parts.len() as f64 * dmath::exp(-t[i] / dec);
            x + 0.5 * wash[i] * dmath::exp(-t[i] / 0.18) + 0.6 * tick[i] * dmath::exp(-t[i] / 0.006)
        })
        .collect()
}

/// `brush_tap(vel, seed)`: a brush slapping the snare head.
pub fn brush_tap(vel: f64, seed: u64) -> Vec<f64> {
    let n = length(0.22 * SRF);
    let t = synth::tt(n);
    synth::bp(&noise(40 + seed, n), 1600.0, 7500.0, FILTER_O)
        .iter()
        .zip(&t)
        .map(|(v, &t)| v * clip(t / 0.006, 0.0, 1.0) * dmath::exp(-t / 0.07) * vel)
        .collect()
}

/// `swirl(dur, seed)`: the brush circling the snare head: soft noise sweeping once per beat.
pub fn swirl(dur: f64, beat: f64, seed: u64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    synth::bp(&noise(50 + seed, n), 2500.0, 9000.0, FILTER_O)
        .iter()
        .zip(&t)
        .map(|(v, &t)| {
            let ph = (t / beat).rem_euclid(1.0);
            let s = dmath::sin(PI * ph);
            v * (0.35 + 0.65 * s * s)
        })
        .collect()
}

/// `dark_pad(ms, dur, cut)`: detuned saws, low-passed, a slow swell.
pub fn dark_pad(ms: &[i32], dur: f64, cut: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    for &m in ms {
        let f = synth::mtof(f64::from(m));
        for d in [-0.004, 0.004] {
            for (v, &t) in x.iter_mut().zip(&t) {
                let ph = (f * (1.0 + d) * t).rem_euclid(1.0);
                *v += 2.0 * ph - 1.0;
            }
        }
    }
    let k = 2.0 * ms.len().max(1) as f64;
    let env = synth::adsr(n, 0.3, 0.2, 0.85, 0.3);
    synth::lp(&x, cut, FILTER_O)
        .iter()
        .zip(&env)
        .map(|(v, e)| v / k * e)
        .collect()
}

/// `roll(dur, f0, step)`: a brush roll growing from `f0` to full over `dur` seconds.
pub fn roll(dur: f64, f0: f64, step: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let mut out = vec![0.0; n + length(SRF)];
    let mut t = 0.0;
    while t < dur - 1e-4 {
        let a = length(t * SRF);
        let s = brush_tap(1.0, (length(t * 100.0) % 7) as u64);
        let g = 0.6 * (f0 + (1.0 - f0) * dmath::pow(t / dur, 1.5));
        for (o, v) in out[a..].iter_mut().zip(&s) {
            *o += v * g;
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

    fn harmony(chords: &[&str]) -> Harmony {
        let g = Grid::new(132.0);
        let all: Vec<String> = chords.iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2], &all, &[], g.n_bars);
        Harmony::new(&g, "C", scale("minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    #[test]
    fn the_rule_gives_the_tables_shape() {
        let h = harmony(&["Cm", "Ab7", "Fm", "G7", "Dm", "Eb", "Bb7"]);
        let pads = [
            ("Cm", [48, 55, 63]),
            ("Ab7", [44, 51, 60]),
            ("Fm", [41, 48, 56]),
            ("G7", [43, 50, 59]),
        ];
        for (ch, v) in pads {
            assert_eq!(pad_voicing(&h, ch).unwrap(), v, "{ch}");
        }
        let want = [("Cm", 36), ("Ab7", 32), ("Fm", 41), ("G7", 43)];
        for (ch, r) in want {
            assert_eq!(bass_root(&h, ch).unwrap(), r, "{ch}");
        }
        // The rule gives the tables' answers for the tables' chords (but the altered G7).
        for (ch, v) in TONES_LO {
            let r = bass_root(&h, ch).unwrap();
            let (third, fifth, seventh) = shape(&h, ch).unwrap();
            let low = |m: i32| if m > 46 { m - 12 } else { m };
            assert_eq!([r, r + third, low(r + fifth), low(r + seventh)], v, "{ch}");
        }
        assert_eq!(comp_voicing(&h, "Dm").unwrap(), [65, 69, 72, 76]);
        assert_eq!(comp_voicing(&h, "Bb7").unwrap(), [62, 65, 68, 72]);
        assert_eq!(bass_tones(&h, "Eb").unwrap(), [39, 43, 46, 38]);
    }

    #[test]
    fn the_instruments_sound() {
        for x in [
            upright(36, 0.4, 1.0),
            ride(false),
            ride(true),
            brush_tap(1.0, 3),
            swirl(0.5, 0.45, 1),
            dark_pad(&[48, 55, 63], 0.6, 900.0),
            roll(0.5, 0.2, 0.05),
        ] {
            assert!(x.iter().all(|v| v.is_finite()));
            assert!(x.iter().any(|v| v.abs() > 0.02));
        }
    }
}
