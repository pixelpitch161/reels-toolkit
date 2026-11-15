//! Theme: gothic horror (the bed of `projects/ballfight_organ_vs_harp/audio.py`, #35 v4 ORGAN vs HARP), quick-ported
//! by ear (ruling M4-R16: close enough, no parity gates). A slow burn into a pounding back half, with no organ and no
//! harp of its own (the fighters are the leads): a choir "aah" pad on the chord, a tolling church bell, a low drone, a
//! ticking clock, a heartbeat kick and a low-string pulse (the burn); a kick, toms, a timpani and the full cello
//! ostinato (the climb); four on the floor, timpani on 1 and 3, staccato choir hits on the off-beats (the pound).
//! Built for D minor (Dm Bb Gm A); any chord works (voicings by rule).
//!
//! Here: the voicings and the instruments, after the episode's functions of the same names. The arrangement is
//! [`GothicHorrorClassic`], in `classic.rs`.
//!
//! **Voicings.** The episode's tables (`VOICING`, `PEDAL`) are its chords' tones from G3 to F#4 and the root from A1
//! up; [`voicing`] and [`pedal`] are that rule, for every chord the harmony parses.

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip, noise};

pub use self::classic::GothicHorrorClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options, no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "gothic_horror",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(GothicHorrorClassic::new())),
};

/// #35's slow-burn loop.
const LOOP: [&str; 4] = ["Dm", "Dm", "Bb", "A"];
/// #35's climb.
const CLIMB: [&str; 4] = ["Dm", "Gm", "Bb", "A"];
/// #35's pound.
const POUND: [&str; 4] = ["Dm", "Bb", "Gm", "A"];

/// What the interface says about the gothic horror.
pub static CARD: ThemeCard = ThemeCard {
    display: "Gothic horror",
    text: "A slow burn in a haunted cathedral: a choir, a tolling bell, a ticking clock and a heartbeat, building to \
           pounding timpani and staccato choir hits. Leaves the leads to the fighters.",
    key: "D",
    scale: "minor",
    chord_loops: &[&LOOP, &CLIMB, &POUND],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The episode that played the gothic horror (#35; `tools/studio_export/older/configs/organ_vs_harp`), once for each
/// of its card's loops (the climb and the pound in the episode's key and tempo), so every loop brings its key.
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "ballfight_organ_vs_harp",
        fighters: ["ORGAN", "HARP"],
        bpm: 126.0,
        key: "D",
        scale: "minor",
        intro: ["Dm", "A"],
        chord_loop: &LOOP,
        tail: &["Dm", "A"],
    },
    Preset {
        config: "ballfight_organ_vs_harp",
        fighters: ["ORGAN", "HARP"],
        bpm: 126.0,
        key: "D",
        scale: "minor",
        intro: ["Dm", "A"],
        chord_loop: &CLIMB,
        tail: &["Dm", "A"],
    },
    Preset {
        config: "ballfight_organ_vs_harp",
        fighters: ["ORGAN", "HARP"],
        bpm: 126.0,
        key: "D",
        scale: "minor",
        intro: ["Dm", "A"],
        chord_loop: &POUND,
        tail: &["Dm", "A"],
    },
];

/// Python's `int(x)` for a length (never below 0).
fn length(x: f64) -> usize {
    if x > 0.0 { x as usize } else { 0 }
}

/// `VOICING[ch]`: the chord's tones from G3 to F#4 (the lowest three).
///
/// # Errors
/// `ch` does not parse.
pub fn voicing(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    let mut v = harmony.chord_tones(55, 66, ch)?;
    v.truncate(3);
    while v.len() < 3 {
        let top = v.last().copied().unwrap_or(57);
        v.push(top + 12);
    }
    Ok(v)
}

/// `PEDAL[ch]`: the chord's root from A1 up (D2, Bb1, G2, A1 in D minor).
///
/// # Errors
/// `ch` does not parse.
pub fn pedal(harmony: &Harmony, ch: &str) -> Result<i32, HarmonyError> {
    harmony.root_above(ch, 33)
}

/// The choir's default vowel ("aah").
pub const AAH: (f64, f64, f64) = (700.0, 1150.0, 2650.0);
/// The staccato hits' vowel ("hah").
pub const HAH: (f64, f64, f64) = (800.0, 1250.0, 2700.0);

/// `choir(ms, dur, att, rel, vowel, vel, seed)`: a choir "aah": four detuned saw sections a note, each with its own
/// slow vibrato, through three vowel formants and a little low end.
pub fn choir(ms: &[i32], dur: f64, att: f64, rel: f64, vowel: (f64, f64, f64), vel: f64, seed: u64) -> Vec<f64> {
    let n = length((dur + rel) * SRF);
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    let mut rng = Generator::new(seed);
    for &m in ms {
        let f = synth::mtof(f64::from(m));
        for d in [-0.006, -0.002, 0.003, 0.007] {
            let rate = 5.0 + rng.uniform(-0.4, 0.4);
            let ph0 = rng.uniform(0.0, 6.0);
            let mut ph = 0.0;
            for (o, &t) in x.iter_mut().zip(&t) {
                let vib = 1.0 + 0.004 * dmath::sin(2.0 * PI * rate * t + ph0);
                ph += f * (1.0 + d) * vib / SRF;
                *o += 2.0 * ph.rem_euclid(1.0) - 1.0;
            }
        }
    }
    let k = 4.0 * (ms.len().max(1) as f64);
    for o in &mut x {
        *o /= k;
    }
    let (f1, f2, f3) = vowel;
    let a = synth::bp(&x, f1 * 0.8, f1 * 1.2, FILTER_O);
    let b = synth::bp(&x, f2 * 0.88, f2 * 1.12, FILTER_O);
    let c = synth::bp(&x, f3 * 0.93, f3 * 1.07, FILTER_O);
    let low = synth::lp(&x, 450.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let late = (t - dur).max(0.0);
            let env = dmath::pow(clip(t / att, 0.0, 1.0), 1.5) * dmath::exp(-late / (rel * 0.4));
            (a[i] + b[i] * 0.55 + c[i] * 0.22 + low[i] * 0.35) * env * vel
        })
        .collect()
}

/// `bell(m, dur, vel)`: a tolling church bell: hum, prime, minor-third tierce, quint, nominal and upper partials,
/// each with its own decay and a slow beat, and a short strike.
pub fn bell(m: i32, dur: f64, vel: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let f = synth::mtof(f64::from(m));
    let mut x = vec![0.0; n];
    for (r, a, dec) in [
        (0.5, 0.6, 2.6),
        (1.0, 0.8, 1.9),
        (1.19, 0.55, 1.5),
        (1.5, 0.35, 1.2),
        (2.0, 0.6, 1.0),
        (2.52, 0.28, 0.7),
        (3.0, 0.2, 0.55),
        (4.07, 0.14, 0.4),
        (5.2, 0.08, 0.25),
    ] {
        for (o, &t) in x.iter_mut().zip(&t) {
            *o += a
                * dmath::sin(2.0 * PI * f * r * t + r)
                * dmath::exp(-t / dec)
                * (1.0 + 0.15 * dmath::sin(2.0 * PI * 1.3 * r * t));
        }
    }
    let k = length(0.006 * SRF).min(n);
    let strike = noise(u64::from(m.unsigned_abs()), k);
    let fade = dsp::linspace(1.0, 0.0, k);
    for i in 0..k {
        x[i] += strike[i] * 0.8 * fade[i];
    }
    x.iter().map(|v| v * vel * 0.5).collect()
}

/// `tick(vel, hi)`: the clock: a dry woody tick (the tock a little lower).
pub fn tick(vel: f64, hi: bool) -> Vec<f64> {
    let n = length(0.06 * SRF);
    let t = synth::tt(n);
    let f = if hi { 2400.0 } else { 1700.0 };
    let wood = synth::bp(&noise(2, n), 1500.0, 5000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            (dmath::sin(2.0 * PI * f * t) * dmath::exp(-t / 0.008) + 0.4 * wood[i] * dmath::exp(-t / 0.004)) * vel
        })
        .collect()
}

/// `heart(vel)`: a heartbeat kick: soft, deep, short.
pub fn heart(vel: f64) -> Vec<f64> {
    synth::lp(&synth::kick(0.6, 0.25, 90.0, 42.0), 400.0, FILTER_O)
        .iter()
        .map(|v| v * vel)
        .collect()
}

/// `timp(m, vel, dur)`: a timpani stroke: a tuned drum whose pitch drops a touch after the stroke, and the felt.
pub fn timp(m: i32, vel: f64, dur: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let f0 = synth::mtof(f64::from(m));
    let felt = synth::lp(&noise(u64::from(m.unsigned_abs()), n), 900.0, FILTER_O);
    let mut ph = 0.0;
    (0..n)
        .map(|i| {
            let t = t[i];
            ph += 2.0 * PI * f0 * (1.0 + 0.06 * dmath::exp(-t / 0.05)) / SRF;
            let body = dmath::sin(ph)
                + 0.5 * dmath::sin(1.5 * ph) * dmath::exp(-t / 0.3)
                + 0.3 * dmath::sin(1.99 * ph) * dmath::exp(-t / 0.2);
            let x = body * dmath::exp(-t / 0.55) + 0.4 * felt[i] * dmath::exp(-t / 0.02);
            dmath::tanh(1.3 * x) * vel
        })
        .collect()
}

/// `cello(m, dur, vel)`: low strings, staccato: three detuned saws, a bowed attack, dark.
pub fn cello(m: i32, dur: f64, vel: f64) -> Vec<f64> {
    let n = length((dur + 0.08) * SRF);
    let x = synth::lp(&synth::saw(synth::mtof(f64::from(m)), n, 0.004, 3), 1500.0, FILTER_O);
    let env = synth::adsr(n, 0.008, 0.06, 0.6, 0.05);
    x.iter().zip(&env).map(|(v, e)| v * e * vel).collect()
}

/// `drone(m, dur, vel)`: a low sine with a dark saw under it, slowly breathing.
pub fn drone(m: i32, dur: f64, vel: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let f = synth::mtof(f64::from(m));
    let saw = synth::lp(&synth::saw(f, n, 0.003, 2), 300.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let env = clip(t / 0.4, 0.0, 1.0) * clip((dur - t) / 0.4, 0.0, 1.0);
            (dmath::sin(2.0 * PI * f * t) + 0.4 * saw[i]) * env * (1.0 + 0.1 * dmath::sin(2.0 * PI * 0.25 * t)) * vel
        })
        .collect()
}

/// `roll(dur, m, f0, step)`: a timpani roll, swelling from `f0` to full.
pub fn roll(dur: f64, m: i32, f0: f64, step: f64) -> Vec<f64> {
    let n = length(dur * SRF);
    let mut out = vec![0.0; n + 2 * bf_sound::SR];
    let stroke = timp(m, 0.6, 0.5);
    let mut t = 0.0;
    let mut j = 0usize;
    while t < dur - 1e-4 {
        let a = length(t * SRF);
        let g = (f0 + (1.0 - f0) * dmath::pow(t / dur, 1.5)) * (0.85 + 0.15 * (j % 2) as f64);
        for (o, s) in out[a..].iter_mut().zip(&stroke) {
            *o += s * g;
        }
        t += step;
        j += 1;
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
    fn the_rule_gives_the_episodes_tables() {
        let g = Grid::new(126.0);
        let all: Vec<String> = ["Dm", "Bb", "Gm", "A"].iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2], &all, &[], g.n_bars);
        let h = Harmony::new(&g, "D", scale("minor").unwrap(), bars, Some(all), 2).unwrap();
        assert_eq!(voicing(&h, "Dm").unwrap(), [57, 62, 65]);
        assert_eq!(voicing(&h, "Bb").unwrap(), [58, 62, 65]);
        assert_eq!(voicing(&h, "Gm").unwrap(), [55, 58, 62]);
        assert_eq!(voicing(&h, "A").unwrap(), [57, 61, 64]);
        let pedals: Vec<i32> = ["Dm", "Bb", "Gm", "A"].iter().map(|c| pedal(&h, c).unwrap()).collect();
        assert_eq!(pedals, [38, 34, 43, 33]);
    }

    #[test]
    fn the_instruments_sound() {
        for x in [
            choir(&[50, 57, 62], 0.5, 0.1, 0.2, AAH, 1.0, 3),
            bell(50, 1.0, 1.0),
            tick(1.0, true),
            heart(1.0),
            timp(38, 1.0, 0.8),
            cello(38, 0.15, 1.0),
            drone(26, 1.0, 1.0),
            roll(0.5, 45, 0.3, 0.06),
        ] {
            let peak = x.iter().fold(0.0f64, |a, v| a.max(v.abs()));
            assert!(peak > 0.01 && peak.is_finite(), "{peak}");
        }
    }
}
