//! Theme: spaghetti western, the bed of `projects/ballfight_ukulele_vs_acoustic/audio.py` (#33 v3, Ukulele vs
//! Acoustic). A standoff on the descending Am - G - F - E cadence: a galloping coconut-shell clip-clop over a frame
//! drum, a twangy baritone guitar (tremolo and a spring tank, kept low and dark) on the bass line, a whistled
//! original theme with a dotted-8th echo, a wordless choir and a tolling bell from the middle of the fight, tremolo
//! strings and a snare ostinato for the final exchange; whip cracks on the count and the K.O. Built for A minor at
//! 129.84 BPM.
//!
//! A quick port (ruling M4-R16): close to the episode by ear, not sample for sample. The episode's tables
//! ([`BASS`], [`CHOIR`], [`STRINGS`]) answer first; any other chord is voiced by the same rule (the root and fifth
//! in the twang's octave, the chord's tones in the choir's and the strings'). The whistle's theme moves to the key.

mod classic;
pub mod layers;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError, parse_chord};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip, noise};

pub use self::classic::SpaghettiWesternClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options, no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "spaghetti_western",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(SpaghettiWesternClassic::new())),
};

/// What the interface says about the spaghetti western.
pub static CARD: ThemeCard = ThemeCard {
    display: "Spaghetti western",
    text: "A standoff: a galloping clip-clop, a twangy baritone guitar, a whistled theme, a choir and a bell.",
    key: "A",
    scale: "minor",
    chord_loops: &[&["Am", "G", "F", "E"]],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The episode that plays the spaghetti western, its music as its `timeline.py` sets it.
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_ukulele_vs_acoustic",
    fighters: ["UKULELE", "ACOUSTIC"],
    bpm: 130.0,
    key: "A",
    scale: "minor",
    intro: ["Am", "E"],
    chord_loop: &["Am", "G", "F", "E"],
    tail: &["Am", "F", "E"],
}];

/// `BASS`: the twang's (root on 1, fifth on 3) for #33's chords.
pub const BASS: [(&str, (i32, i32)); 4] = [("Am", (45, 52)), ("G", (43, 50)), ("F", (41, 48)), ("E", (40, 47))];

/// `CHOIR`: the choir's chords, as #33 voiced them.
pub const CHOIR: [(&str, &[i32]); 4] = [
    ("Am", &[57, 60, 64]),
    ("G", &[55, 59, 62]),
    ("F", &[57, 60, 65]),
    ("E", &[56, 59, 64]),
];

/// `STRINGS`: the tremolo strings' chords, as #33 voiced them.
pub const STRINGS: [(&str, &[i32]); 4] = [
    ("Am", &[57, 64, 69, 72]),
    ("G", &[55, 62, 67, 71]),
    ("F", &[53, 60, 65, 69]),
    ("E", &[52, 59, 64, 68]),
];

/// `T.WHISTLE`: the standoff theme over 8 bars, in A minor: (bar in the phrase, beat, MIDI note, beats).
pub const WHISTLE: [(usize, f64, i32, f64); 29] = [
    (0, 0.0, 76, 1.5),
    (0, 1.5, 81, 0.5),
    (0, 2.0, 79, 0.5),
    (0, 2.5, 76, 1.5),
    (1, 0.0, 74, 2.0),
    (1, 2.0, 71, 0.5),
    (1, 2.5, 74, 0.5),
    (1, 3.0, 79, 1.0),
    (2, 0.0, 77, 1.5),
    (2, 1.5, 76, 0.5),
    (2, 2.0, 72, 2.0),
    (3, 0.0, 71, 1.0),
    (3, 1.0, 68, 1.0),
    (3, 2.0, 64, 2.0),
    (4, 0.0, 69, 0.5),
    (4, 0.5, 72, 0.5),
    (4, 1.0, 76, 1.0),
    (4, 2.0, 81, 2.0),
    (5, 0.0, 83, 1.0),
    (5, 1.0, 81, 0.5),
    (5, 1.5, 79, 0.5),
    (5, 2.0, 74, 2.0),
    (6, 0.0, 84, 1.5),
    (6, 1.5, 81, 0.5),
    (6, 2.0, 77, 1.0),
    (6, 3.0, 76, 1.0),
    (7, 0.0, 80, 1.0),
    (7, 1.0, 77, 1.0),
    (7, 2.0, 76, 2.0),
];

/// `WHIP_PRE`: a whip cracks 0.10 s into its sound, so it is placed that much early.
pub const WHIP_PRE: f64 = 0.10;

fn table<T: Clone>(of: &[(&str, T)], chord: &str) -> Option<T> {
    of.iter().find(|(c, _)| *c == chord).map(|(_, v)| v.clone())
}

/// The twang's root and fifth for `chord`: the table's, else the root from E2 to D#3 and the fifth above it.
///
/// # Errors
/// `chord` does not parse.
pub fn bass(chord: &str) -> Result<(i32, i32), HarmonyError> {
    if let Some(v) = table(&BASS, chord) {
        return Ok(v);
    }
    let r = 40 + (parse_chord(chord)?.root - 4).rem_euclid(12);
    Ok((r, r + 7))
}

/// The choir's chord: the table's, else the chord's lowest three tones from G3 up.
///
/// # Errors
/// `chord` is in neither the table nor the episode's chords.
pub fn choir_notes(harmony: &Harmony, chord: &str) -> Result<Vec<i32>, HarmonyError> {
    if let Some(v) = table(&CHOIR, chord) {
        return Ok(v.to_vec());
    }
    Ok(harmony.chord_tones(55, 67, chord)?.into_iter().take(3).collect())
}

/// The tremolo strings' chord: the table's, else the chord's root from E3 up, then three tones spread above it.
///
/// # Errors
/// `chord` is in neither the table nor the episode's chords.
pub fn strings_notes(harmony: &Harmony, chord: &str) -> Result<Vec<i32>, HarmonyError> {
    if let Some(v) = table(&STRINGS, chord) {
        return Ok(v.to_vec());
    }
    let r = harmony.root_above(chord, 52)?;
    let above = harmony.chord_tones(r + 7, r + 20, chord)?;
    let mut out = vec![r];
    out.extend(above.into_iter().take(3));
    Ok(out)
}

/// The semitones from A to the key, folded to -6..5 (the whistle's theme and the bells move by it).
pub fn key_shift(harmony: &Harmony) -> i32 {
    (harmony.key_pc - 9 + 6).rem_euclid(12) - 6
}

fn length(x: f64) -> usize {
    if x > 0.0 { x as usize } else { 0 }
}

/// `ks(m, dur, decay, bright, seed)`: a Karplus-Strong string, the noise burst of `seed + m` round a delay line.
pub fn ks(m: i32, dur: f64, decay: f64, bright: f64, seed: u64) -> Vec<f64> {
    let f = synth::mtof(f64::from(m));
    let period = (SRF / f).round().max(2.0) as usize;
    let n = length(dur * SRF);
    let mut rng = bf_core::rng::Generator::new(seed + m.max(0) as u64);
    let exc: Vec<f64> = (0..period).map(|_| rng.uniform(-1.0, 1.0)).collect();
    let smooth = dsp::convolve_same(&exc, &[1.0 / 6.0; 6]);
    let mut y = vec![0.0; n];
    for i in 0..n {
        let mut v = if i < period {
            bright * exc[i] + (1.0 - bright) * smooth[i]
        } else {
            0.0
        };
        if i >= period {
            v += 0.5 * decay * y[i - period];
        }
        if i > period {
            v += 0.5 * decay * y[i - period - 1];
        }
        y[i] = v;
    }
    y
}

/// `twang(m, dur, vel)`: the baritone guitar: a bright pluck through a little tremolo and a spring tank, kept dark.
pub fn twang(m: i32, dur: f64, vel: f64) -> Vec<f64> {
    let x = ks(m, dur + 0.3, 0.997, 0.85, 0);
    let t = synth::tt(x.len());
    let k = dmath::tanh(1.8);
    let x: Vec<f64> = x
        .iter()
        .zip(&t)
        .map(|(v, &t)| dmath::tanh(1.8 * v * (1.0 + 0.22 * dmath::sin(2.0 * PI * 6.2 * t))) / k)
        .collect();
    let mut y = x.clone();
    for (dl, g) in [(0.037, 0.32), (0.053, 0.24), (0.071, 0.18)] {
        let d = (dl * SRF) as usize;
        let mut c = x.clone();
        for i in d..c.len() {
            c[i] += g * c[i - d];
        }
        for ((o, cv), xv) in y.iter_mut().zip(&c).zip(&x) {
            *o += 0.5 * cv - 0.5 * xv;
        }
    }
    let mut y = synth::bp(&y, 70.0, 1900.0, FILTER_O);
    let fd = ((0.06 * SRF) as usize).min(y.len());
    let n = y.len();
    for (j, v) in y[n - fd..].iter_mut().enumerate() {
        *v *= 1.0 - j as f64 / (fd.max(2) - 1) as f64;
    }
    y.iter().map(|v| v * vel).collect()
}

/// `hoof(vel, hi, seed)`: a coconut-shell clip-clop: two hollow tones and a click.
pub fn hoof(vel: f64, hi: bool, seed: u64) -> Vec<f64> {
    let n = length(0.09 * SRF);
    let t = synth::tt(n);
    let f = if hi { 1080.0 } else { 880.0 };
    let click = synth::bp(&noise(seed, n), 1800.0, 6000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let x = dmath::sin(2.0 * PI * f * t) * dmath::exp(-t / 0.018)
                + 0.4 * dmath::sin(2.0 * PI * f * 1.9 * t) * dmath::exp(-t / 0.008)
                + 0.3 * click[i] * dmath::exp(-t / 0.003);
            x * vel
        })
        .collect()
}

/// `thump(vel)`: a low frame drum under the gallop.
pub fn thump(vel: f64) -> Vec<f64> {
    synth::kick(0.6, 0.22, 118.0, 56.0).iter().map(|v| v * vel).collect()
}

/// `whip(vel, seed)`: a short swish, then the crack at +0.10 s ([`WHIP_PRE`]).
pub fn whip(vel: f64, seed: u64) -> Vec<f64> {
    let n = length(0.22 * SRF);
    let t = synth::tt(n);
    let x = noise(seed, n);
    let swish = synth::bp(&x, 900.0, 5000.0, FILTER_O);
    let hiss = synth::hp(&x, 2500.0, FILTER_O);
    let f: Vec<f64> = t
        .iter()
        .map(|&t| 4200.0 * dmath::exp(-(t - 0.10).max(0.0) / 0.006) + 1100.0)
        .collect();
    let ph = dsp::cumsum(&f);
    (0..n)
        .map(|i| {
            let t = t[i];
            if t < 0.10 {
                swish[i] * clip(t / 0.10, 0.0, 1.0).powi(2) * 0.35 * vel
            } else {
                let tc = t - 0.10;
                let crack = hiss[i] * dmath::exp(-tc / 0.006)
                    + 0.6 * dmath::sin(2.0 * PI * ph[i] / SRF) * dmath::exp(-tc / 0.012);
                crack * vel
            }
        })
        .collect()
}

/// `bell(m, vel, dur)`: a tolling church bell: inharmonic partials with their own decays, a strike of noise.
pub fn bell(m: i32, vel: f64, dur: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let f = synth::mtof(f64::from(m));
    let strike = synth::bp(&noise(3, n), 800.0, 4000.0, FILTER_O);
    let partials = [
        (0.5, 0.5, 3.0),
        (1.0, 1.0, 2.2),
        (1.19, 0.55, 1.6),
        (1.5, 0.3, 1.1),
        (2.0, 0.75, 1.3),
        (2.61, 0.3, 0.7),
        (3.0, 0.22, 0.6),
        (4.12, 0.18, 0.4),
    ];
    (0..n)
        .map(|i| {
            let t = t[i];
            let mut x = 0.0;
            for (r, a, d) in partials {
                x += a * dmath::sin(2.0 * PI * f * r * t + r) * dmath::exp(-t / d);
            }
            x += 0.4 * strike[i] * dmath::exp(-t / 0.01);
            x * vel * 0.5
        })
        .collect()
}

/// The mean of detuned saws on `ms`.
fn saws(ms: &[i32], n: usize, det: f64, voices: usize) -> Vec<f64> {
    let mut x = vec![0.0; n];
    for &m in ms {
        for (o, v) in x.iter_mut().zip(synth::saw(synth::mtof(f64::from(m)), n, det, voices)) {
            *o += v;
        }
    }
    let k = ms.len().max(1) as f64;
    x.iter().map(|v| v / k).collect()
}

/// `choir(ms, dur, att, rel)`: a wordless "aah": detuned saws through three vowel formants.
pub fn choir(ms: &[i32], dur: f64, att: f64, rel: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let x = saws(ms, n, 0.008, 3);
    let a = synth::bp(&x, 620.0, 900.0, FILTER_O);
    let b = synth::bp(&x, 1050.0, 1350.0, FILTER_O);
    let c = synth::bp(&x, 2400.0, 2900.0, FILTER_O);
    let t = synth::tt(n);
    let env = synth::adsr(n, att, 0.2, 0.85, rel.min(dur * 0.4));
    (0..n)
        .map(|i| (a[i] + 0.5 * b[i] + 0.18 * c[i]) * (1.0 + 0.04 * dmath::sin(2.0 * PI * 5.0 * t[i])) * env[i])
        .collect()
}

/// `tremolo_strings(ms, dur, k0, k1)`: a saw chord shivering on the 32nds (`s16` the 16th), swelling from `k0` to
/// `k1`.
pub fn tremolo_strings(ms: &[i32], dur: f64, k0: f64, k1: f64, s16: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let x = saws(ms, n, 0.005, 2);
    let t = synth::tt(n);
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let trem = 0.55 + 0.45 * dmath::sin(PI * t[i] / (s16 / 2.0)).abs();
            let sw = k0 + (k1 - k0) * clip(t[i] / dur, 0.0, 1.0);
            x[i] * trem * sw
        })
        .collect();
    let env = synth::adsr(n, 0.08, 0.1, 1.0, 0.05);
    synth::lp(&y, 2800.0, FILTER_O)
        .iter()
        .zip(&env)
        .map(|(v, e)| v * e)
        .collect()
}

/// `whistle(m, dur, vel, m_prev)`: a near-pure tone with breath, a glide in from the previous note, vibrato that
/// blooms late.
pub fn whistle(m: i32, dur: f64, vel: f64, m_prev: Option<i32>) -> Vec<f64> {
    let n = length((dur + 0.12) * SRF);
    let t = synth::tt(n);
    let f1 = synth::mtof(f64::from(m));
    let f0 = m_prev.map_or(f1 * 0.985, |p| synth::mtof(f64::from(p)));
    let f: Vec<f64> = t
        .iter()
        .map(|&t| {
            let f = f1 + (f0 - f1) * dmath::exp(-t / 0.035);
            f * (1.0 + 0.0075 * clip((t - 0.16) / 0.3, 0.0, 1.0) * dmath::sin(2.0 * PI * 5.6 * t))
        })
        .collect();
    let ph = dsp::cumsum(&f);
    let br = noise(m.max(0) as u64, n);
    let breath = synth::bp(&br, f1 * 0.85, f1 * 1.2, FILTER_O);
    let air = synth::bp(&br, 4000.0, 9000.0, FILTER_O);
    let x: Vec<f64> = (0..n)
        .map(|i| {
            let p = 2.0 * PI * ph[i] / SRF;
            let x = dmath::sin(p) + 0.05 * dmath::sin(2.0 * p) + 0.015 * dmath::sin(3.0 * p);
            let x = x + 0.10 * breath[i] + 0.012 * air[i];
            let env = clip(t[i] / 0.035, 0.0, 1.0) * clip((dur + 0.08 - t[i]) / 0.08, 0.0, 1.0);
            x * env
        })
        .collect();
    synth::lp(&x, 7000.0, FILTER_O).iter().map(|v| v * vel).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_instruments_sound() {
        for x in [
            twang(45, 0.6, 1.0),
            hoof(1.0, true, 3),
            thump(1.0),
            whip(1.0, 2),
            bell(45, 1.0, 1.0),
            choir(&[57, 60, 64], 0.5, 0.05, 0.2),
            tremolo_strings(&[57, 64, 69, 72], 0.5, 0.4, 1.0, 0.1155),
            whistle(76, 0.4, 1.0, Some(71)),
        ] {
            assert!(!x.is_empty() && x.iter().all(|v| v.is_finite()));
            assert!(x.iter().any(|v| v.abs() > 0.02));
        }
        assert_eq!(bass("Am"), Ok((45, 52)));
        assert_eq!(bass("Dm"), Ok((50, 57)));
        assert_eq!(whip(1.0, 2).len(), 10560);
    }
}
