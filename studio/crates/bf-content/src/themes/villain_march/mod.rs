//! Theme: villain march, the bed of `projects/ballfight_tuba_vs_flute/audio.py` (#32 v3), quick-ported by ear
//! (ruling M4-R16: close enough, no reference mood, no parity gates). A sinister military march with no bass and
//! no high lead of its own (the fighters are the bottom and the top): a snare ostinato and a heavy bass drum on 1
//! and 3, low brass "pah" chords on 2 and 4 (doubled on the off-beats as it heats up), a slow horn swell from bar 3,
//! a music-box counter-line (the villain's lullaby) from bar 4, and timpani on the downbeat with a crash every four
//! bars in the back half. Built for C harmonic minor (Cm Ab Db G).
//!
//! Here: the voicings and the instruments; the arrangement is [`VillainMarchClassic`], in `classic.rs`.
//!
//! **Voicings.** The episode's table of brass chords for its four chords, and for any other the chord's tones from
//! G3 to F#4 ([`voice`]). The music box plays the episode's lullaby over its own loop in C, and over any other a
//! line of the same shape from the chord's tones ([`lullaby`]).

mod classic;
pub mod layers;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_sound::SRF;
use bf_sound::synth::{self, FILTER_O, noise};

pub use self::classic::VillainMarchClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options, no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "villain_march",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(VillainMarchClassic::new())),
};

/// What the interface says about the villain march.
pub static CARD: ThemeCard = ThemeCard {
    display: "Villain march",
    text: "A sinister march: snare, bass drum, low brass and a music box. Leaves the bass and the top to the fighters.",
    key: "C",
    scale: "harmonic minor",
    chord_loops: &[&["Cm", "Ab", "Db", "G"]],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The episode that plays the villain march (#32 v3): its music as its `timeline.py` sets it.
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_tuba_vs_flute",
    fighters: ["TUBA", "FLUTE"],
    bpm: 126.0,
    key: "C",
    scale: "harmonic minor",
    intro: ["Cm", "G"],
    chord_loop: &["Cm", "Ab", "Db", "G"],
    tail: &["Ab", "G"],
}];

/// `VOICING`: the brass "pah" chords, as #32 voiced them.
const VOICING: [(&str, &[i32]); 4] = [
    ("Cm", &[55, 60, 63]),
    ("Ab", &[56, 60, 63]),
    ("Db", &[56, 59, 61, 65]),
    ("G", &[55, 59, 62, 65]),
];

/// The episode's loop, which its lullaby follows.
const LOOP: [&str; 4] = ["Cm", "Ab", "Db", "G"];

/// `MUSIC_BOX`'s quarters (the bed plays only those): the villain's lullaby, one bar a chord of [`LOOP`].
const LULLABY: [[i32; 4]; 4] = [[84, 87, 86, 83], [84, 80, 79, 80], [85, 89, 88, 83], [79, 86, 91, 86]];

/// The brass chord for `chord`: the table's, else its tones from G3 to F#4.
///
/// # Errors
/// `chord` is in neither the table nor the episode's chords.
pub fn voice(harmony: &Harmony, chord: &str) -> Result<Vec<i32>, HarmonyError> {
    if let Some((_, v)) = VOICING.iter().find(|(c, _)| *c == chord) {
        return Ok(v.to_vec());
    }
    harmony.chord_tones(55, 66, chord)
}

/// The chord's root from C3 up (C3 to B3): the brass's low note on the title card and the stinger.
///
/// # Errors
/// `chord` does not parse.
pub fn root(harmony: &Harmony, chord: &str) -> Result<i32, HarmonyError> {
    harmony.root_above(chord, 48)
}

/// The music box's four quarters on bar `b` (the bar's chord `chord`): the episode's lullaby where the loop is the
/// episode's own (in C), else the chord's root, its fifth, its third and the note a semitone under the root (the
/// lullaby's sinister leading tone), from G5 up. The bed plays them an octave higher.
///
/// # Errors
/// `chord` is not one of the episode's chords.
pub fn lullaby(harmony: &Harmony, b: usize, chord: &str) -> Result<[i32; 4], HarmonyError> {
    let own = harmony.key_pc == 0 && harmony.loop_.iter().map(String::as_str).eq(LOOP);
    if own {
        let lb = b.saturating_sub(harmony.loop_start) % LOOP.len();
        return Ok(LULLABY[lb]);
    }
    let r = harmony.root_above(chord, 79)?;
    let pcs = harmony.chord_pcs(chord)?;
    let up = |i: usize, or: i32| pcs.get(i).map_or(or, |p| (p - pcs[0]).rem_euclid(12));
    Ok([r, r + up(2, 7), r + up(1, 3), r - 1])
}

fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// `brass(ms, dur, att, cut, bite)`: the low brass section: two detuned saws a note, a bright "bwah" filter bite
/// at the attack.
pub fn brass(ms: &[i32], dur: f64, att: f64, cut: f64, bite: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    for (j, &m) in ms.iter().enumerate() {
        let f = synth::mtof(f64::from(m));
        for d in [-0.004, 0.004] {
            for (v, &t) in x.iter_mut().zip(&t) {
                let ph = (f * (1.0 + d) * t + 0.37 * j as f64).rem_euclid(1.0);
                *v += 2.0 * ph - 1.0;
            }
        }
    }
    let saws = 2.0 * ms.len().max(1) as f64;
    for v in &mut x {
        *v /= saws;
    }
    let env = synth::adsr(n, att, 0.08, 0.7, (dur * 0.3).min(0.06));
    let dark = synth::lp(&x, cut * 0.45, FILTER_O);
    let bright = synth::lp(&x, cut * 1.6, FILTER_O);
    (0..n)
        .map(|i| {
            let k = dmath::exp(-t[i] / 0.07);
            (dark[i] * (1.0 - k * bite) + bright[i] * k * bite) * env[i]
        })
        .collect()
}

/// `horn_pad(ms, dur)`: a slow horn swell.
pub fn horn_pad(ms: &[i32], dur: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    for &m in ms {
        let f = synth::mtof(f64::from(m));
        for (v, &t) in x.iter_mut().zip(&t) {
            *v += dmath::sin(2.0 * PI * f * t)
                + 0.35 * dmath::sin(4.0 * PI * f * t)
                + 0.15 * dmath::sin(6.0 * PI * f * t);
        }
    }
    let notes = ms.len().max(1) as f64;
    for v in &mut x {
        *v /= notes;
    }
    let env = synth::adsr(n, (dur * 0.3).min(0.5), 0.2, 0.85, (dur * 0.3).min(0.4));
    synth::lp(&x, 1300.0, FILTER_O)
        .iter()
        .enumerate()
        .map(|(i, v)| v * env[i] * (1.0 + 0.05 * dmath::sin(2.0 * PI * 4.8 * t[i])))
        .collect()
}

/// `music_box(m, vel)`: a music-box tine: a bright sine and an inharmonic partial, a short ring, a little flat.
pub fn music_box(m: i32, vel: f64) -> Vec<f64> {
    let n = length(0.9 * SRF);
    let f = synth::mtof(f64::from(m)) * (1.0 - 0.004);
    synth::tt(n)
        .iter()
        .map(|&t| {
            let mut x = dmath::sin(2.0 * PI * f * t) * dmath::exp(-t / 0.32)
                + 0.35 * dmath::sin(2.0 * PI * f * 2.76 * t) * dmath::exp(-t / 0.06);
            x += 0.15 * dmath::sin(2.0 * PI * f * 5.4 * t) * dmath::exp(-t / 0.02);
            x * vel
        })
        .collect()
}

/// `bass_drum(1.0)`: `A.kick(0.6, 0.45, 110, 40)`.
pub fn bass_drum_boom() -> Vec<f64> {
    synth::kick(0.6, 0.45, 110.0, 40.0)
}

/// `march_snare(1.0)`: the snare with a bright rattle (its noise is seed 31 every time).
pub fn march_snare_stroke() -> Vec<f64> {
    let n = length(0.22 * SRF);
    let t = synth::tt(n);
    let mut x = synth::snare(0.11);
    x.resize(n, 0.0);
    let rattle = synth::bp(&noise(31, n), 3000.0, 9000.0, FILTER_O);
    (0..n)
        .map(|i| x[i] + 0.35 * rattle[i] * dmath::exp(-t[i] / 0.07))
        .collect()
}

/// `timpani(m, vel)`: a timpani stroke on MIDI note `m`.
pub fn timpani(m: i32, vel: f64) -> Vec<f64> {
    let n = length(1.2 * SRF);
    let t = synth::tt(n);
    let f = synth::mtof(f64::from(m));
    let skin = synth::bp(&noise(u64::try_from(m.max(0)).unwrap_or(0), n), 100.0, 900.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let mut x = dmath::sin(2.0 * PI * f * t * (1.0 + 0.02 * dmath::exp(-t / 0.05)))
                + 0.4 * dmath::sin(2.0 * PI * f * 1.5 * t) * dmath::exp(-t / 0.2);
            x *= dmath::exp(-t / 0.5);
            x += 0.3 * skin[i] * dmath::exp(-t / 0.02);
            x * vel
        })
        .collect()
}

/// `roll(dur, f0, step)`: march snare strokes every `step` seconds, growing from `f0` to full, `dur` seconds long.
///
/// # Panics
/// When `step` is not above zero.
pub fn roll(stroke: &[f64], dur: f64, f0: f64, step: f64) -> Vec<f64> {
    assert!(step > 0.0, "roll: a stroke every {step} s");
    let n = length(dur * SRF);
    let mut out = vec![0.0; n + length(SRF)];
    let mut k = 0usize;
    loop {
        let t = k as f64 * step;
        if t >= dur - 1e-4 {
            break;
        }
        let v = (f0 + (1.0 - f0) * dmath::pow(t / dur, 1.5)) * 0.7;
        let a = length(t * SRF);
        for (o, s) in out[a..].iter_mut().zip(stroke) {
            *o += s * v;
        }
        k += 1;
    }
    out.truncate(n);
    out
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};

    use super::*;

    fn harmony(key: &str, chords: &[&str]) -> Harmony {
        let g = Grid::new(126.0);
        let all: Vec<String> = chords.iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2], &all, &[], g.n_bars);
        Harmony::new(&g, key, scale("harmonic minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    #[test]
    fn voicings_and_the_lullaby() {
        let h = harmony("C", &LOOP);
        // The rule gives the table's triads.
        assert_eq!(h.chord_tones(55, 66, "Cm").unwrap(), [55, 60, 63]);
        assert_eq!(h.chord_tones(55, 66, "Ab").unwrap(), [56, 60, 63]);
        assert_eq!(voice(&h, "G").unwrap(), [55, 59, 62, 65]);
        assert_eq!(root(&h, "Db").unwrap(), 49);
        // Its own loop: the episode's line, bar by bar from FIGHT!
        assert_eq!(lullaby(&h, 4, "Db").unwrap(), LULLABY[2]);
        assert_eq!(lullaby(&h, 9, "Ab").unwrap(), LULLABY[3]);
        let other = harmony("D", &["Dm", "Bb", "Eb", "A"]);
        assert_eq!(voice(&other, "Dm").unwrap(), [57, 62, 65]);
        assert_eq!(lullaby(&other, 4, "Dm").unwrap(), [86, 93, 89, 85]);
    }

    #[test]
    fn the_instruments_sound() {
        let stroke = march_snare_stroke();
        for x in [
            brass(&[55, 60, 63], 0.22, 0.012, 1500.0, 1.0),
            brass(&[60], 0.12, 0.012, 1500.0, 0.6),
            horn_pad(&[55, 60, 63], 1.9),
            music_box(96, 0.9),
            bass_drum_boom(),
            stroke.clone(),
            timpani(48, 0.9),
            roll(&stroke, 0.5, 0.15, 0.06),
        ] {
            assert!(x.iter().all(|v| v.is_finite()) && x.iter().any(|v| v.abs() > 0.01));
        }
        assert_eq!(roll(&stroke, 0.5, 0.15, 0.06).len(), 24000);
    }
}
