//! Theme: orchestral drill, the bed of `projects/ballfight_viola_vs_violin/audio.py` (#42, Viola vs Violin). A UK
//! drill beat (a sliding 808, skittering hi-hat rolls in 16th triplets and 32nds, the half-time clap on beat 3, rim
//! clicks) under a dark staccato string bed of cellos and basses only: the CELLO's Pixel Pitch voice and a bowed
//! saw an octave down. No violin or viola in the bed: the fighters carry the music. Built for C harmonic minor at
//! 141.64 BPM.
//!
//! A quick port (ruling M4-R16): close to the episode by ear, not sample for sample. The episode's 808 roots
//! ([`ROOT_808`]) answer first; any other chord is voiced by the same rule (the 808 on the root from F1 to E2, the
//! cellos on the chord tones from C2 to G3, the basses an octave over the 808).

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{HarmonyError, parse_chord};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip, noise};

pub use self::classic::OrchestralDrillClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The CELLO's Pixel Pitch voice id, which the bed plays itself.
pub const CELLO: &str = "CELLO";

/// The theme as the registry lists it: no options; its bed plays the CELLO's voice.
pub static THEME: Theme = Theme {
    name: "orchestral_drill",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[CELLO],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(OrchestralDrillClassic::new())),
};

/// #42's loop: i - bVI - iv - V in C harmonic minor, two bars each.
const LOOP: [&str; 8] = ["Cm", "Cm", "Ab", "Ab", "Fm", "Fm", "G", "G"];

/// What the interface says about the orchestral drill.
pub static CARD: ThemeCard = ThemeCard {
    display: "Orchestral drill",
    text: "A UK drill beat (a sliding 808, skittering hats, a half-time clap) under dark staccato cellos and basses. \
           Leaves the melody to the fighters.",
    key: "C",
    scale: "harmonic minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The episode that plays the orchestral drill, its music as its `timeline.py` sets it.
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_viola_vs_violin",
    fighters: ["VIOLA", "VIOLIN"],
    bpm: 140.0,
    key: "C",
    scale: "harmonic minor",
    intro: ["Cm", "G"],
    chord_loop: &LOOP,
    tail: &["Ab", "G"],
}];

/// `CHORD_ROOT`: the 808's roots for #42's chords (C2 Ab1 F1 G1).
pub const ROOT_808: [(&str, i32); 4] = [("Cm", 36), ("Ab", 32), ("Fm", 29), ("G", 31)];

/// `P808`: the 808's rhythm per bar: (16th, length in 16ths, degree, slide into the next).
pub const P808: [(usize, usize, i32, bool); 4] =
    [(0, 6, 0, false), (6, 4, 0, true), (10, 4, 12, false), (14, 2, 7, true)];

/// `HAT_ROLLS`: the hat rolls (16th triplets or 32nds) at fixed 16ths of the bar; `true` is a triplet.
pub const HAT_ROLLS: [(usize, bool); 3] = [(3, true), (11, false), (15, true)];

/// `CELLO_PAT`: the cellos' staccato stabs: (16th, chord tone).
pub const CELLO_PAT: [(usize, usize); 7] = [(0, 0), (3, 1), (6, 2), (8, 0), (10, 1), (11, 2), (14, 1)];

/// The 808's root for `chord`: the table's, else the chord's root from F1 to E2.
///
/// # Errors
/// `chord` does not parse.
pub fn root_808(chord: &str) -> Result<i32, HarmonyError> {
    if let Some((_, r)) = ROOT_808.iter().find(|(c, _)| *c == chord) {
        return Ok(*r);
    }
    Ok(29 + (parse_chord(chord)?.root - 5).rem_euclid(12))
}

fn length(x: f64) -> usize {
    if x > 0.0 { x as usize } else { 0 }
}

/// `sub808(m0, m1, dur, slide, vel)`: the 808: a sine sub with a punchy pitch drop on the attack, saturated,
/// gliding to `m1` over the last `slide` seconds.
pub fn sub808(m0: i32, m1: i32, dur: f64, slide: f64, vel: f64) -> Vec<f64> {
    let n = length((dur + 0.05) * SRF);
    let t = synth::tt(n);
    let (f0, f1) = (synth::mtof(f64::from(m0)), synth::mtof(f64::from(m1)));
    let f: Vec<f64> = t
        .iter()
        .map(|&t| {
            let u = if slide > 0.0 {
                clip((t - (dur - slide)) / slide.max(1e-3), 0.0, 1.0)
            } else {
                0.0
            };
            f0 * (f1 / f0).powf(u * u * (3.0 - 2.0 * u)) * (1.0 + 0.5 * dmath::exp(-t / 0.02))
        })
        .collect();
    let ph = dsp::cumsum(&f);
    let k = dmath::tanh(2.2);
    let x: Vec<f64> = (0..n)
        .map(|i| {
            let t = t[i];
            let x = dmath::tanh(2.2 * dmath::sin(2.0 * PI * ph[i] / SRF)) / k;
            let env = clip(t / 0.004, 0.0, 1.0)
                * clip((dur + 0.05 - t) / 0.05, 0.0, 1.0)
                * (0.75 + 0.25 * dmath::exp(-t / 0.15));
            x * env
        })
        .collect();
    synth::lp(&x, 900.0, FILTER_O).iter().map(|v| v * vel).collect()
}

/// `hat(vel, open)`.
pub fn hat(vel: f64, open: bool) -> Vec<f64> {
    synth::hat(if open { 0.12 } else { 0.03 }, open)
        .iter()
        .map(|v| v * vel)
        .collect()
}

/// `clap(vel)`: a clap over a snare body.
pub fn clap(vel: f64) -> Vec<f64> {
    let (c, s) = (synth::clap(), synth::snare(0.16));
    (0..c.len().max(s.len()))
        .map(|i| (c.get(i).unwrap_or(&0.0) * 0.8 + s.get(i).unwrap_or(&0.0) * 0.6) * vel)
        .collect()
}

/// `rim(vel, seed)`: a rim click.
pub fn rim(vel: f64, seed: u64) -> Vec<f64> {
    let n = length(0.06 * SRF);
    let t = synth::tt(n);
    let nz = synth::bp(&noise(seed, n), 2500.0, 7000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            (dmath::sin(2.0 * PI * 1700.0 * t) * dmath::exp(-t / 0.008) + 0.5 * nz[i] * dmath::exp(-t / 0.006)) * vel
        })
        .collect()
}

/// `tom(f0, dec, vel)`: a tom with a pitch drop and a stick noise.
pub fn tom(f0: f64, dec: f64, vel: f64) -> Vec<f64> {
    let n = length(0.45 * SRF);
    let t = synth::tt(n);
    let f: Vec<f64> = t.iter().map(|&t| f0 * (1.0 + 0.6 * dmath::exp(-t / 0.03))).collect();
    let ph = dsp::cumsum(&f);
    let nz = synth::bp(&noise(f0 as u64, n), 300.0, 3000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            (dmath::sin(2.0 * PI * ph[i] / SRF) * dmath::exp(-t / dec) + 0.3 * nz[i] * dmath::exp(-t / 0.02)) * vel
        })
        .collect()
}

/// `contrabass(m, dur, vel)`: the basses: a bowed saw, low-passed (the bed's floor between 808 hits).
pub fn contrabass(m: i32, dur: f64, vel: f64) -> Vec<f64> {
    let n = length((dur + 0.08) * SRF);
    let x = synth::saw(synth::mtof(f64::from(m)), n, 0.004, 2);
    let t = synth::tt(n);
    let y: Vec<f64> = (0..n)
        .map(|i| x[i] * clip(t[i] / 0.02, 0.0, 1.0) * clip((dur + 0.08 - t[i]) / 0.08, 0.0, 1.0))
        .collect();
    synth::lp(&y, 380.0, FILTER_O).iter().map(|v| v * vel * 0.6).collect()
}

/// VS's crack: a bright noise crack over a low rumble.
pub fn crack() -> Vec<f64> {
    let n = length(1.4 * SRF);
    let t = synth::tt(n);
    let c = synth::bp(&noise(91, n), 1500.0, 9000.0, FILTER_O);
    let r = synth::lp(&noise(92, n), 160.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            c[i] * dmath::exp(-t / 0.035) * 0.9 + r[i] * clip(t / 0.02, 0.0, 1.0) * dmath::exp(-t / 0.5) * 3.0
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_instruments_sound() {
        for x in [
            sub808(36, 48, 0.4, 0.08, 1.0),
            hat(0.8, false),
            hat(0.8, true),
            clap(1.0),
            rim(0.8, 3),
            tom(200.0, 0.18, 1.0),
            contrabass(48, 1.0, 0.8),
            crack(),
        ] {
            assert!(!x.is_empty() && x.iter().all(|v| v.is_finite()));
            assert!(x.iter().any(|v| v.abs() > 0.02));
        }
        assert_eq!(root_808("Ab"), Ok(32));
        assert_eq!(root_808("Dm"), Ok(38));
        assert_eq!(root_808("Eb"), Ok(39));
    }
}
