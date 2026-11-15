//! Theme: industrial, the bed of `projects/ballfight_barisax_vs_bassclarinet/audio.py` (#40 v2, Bari Sax vs Bass
//! Clarinet). A dark, driving construction site with no bass instrument of its own: a tight distorted kick, an anvil
//! backbeat (an anvil ring over a short snare), metal ticks on the 8ths (16ths as it thickens), a struck-pipe ostinato
//! on the chord tones up high, a distorted grinder chugging power chords on syncopated 16ths (high-passed: the low
//! end is the fighters'), sheet-metal crashes on the phrase, hydraulic hisses and air-wrench bursts as fills. Built
//! for G minor at 125.90 BPM.
//!
//! A quick port (ruling M4-R16): close to the episode by ear, not sample for sample. It reads the fight as the
//! episode does: the kick steps back where a swing honk bottoms out, the site holds off its extra chug while a
//! wrecking ball is up on the crane, and a machine hit lands under every captioned ability. Any chord is voiced by
//! the episode's rule (the pipes on the chord tones from G4 to A5, the grinder's power chord from D4 up).

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{HarmonyError, parse_chord};
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip, noise};

pub use self::classic::IndustrialClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options, no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "industrial",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(IndustrialClassic::new())),
    composed: None,
};

/// #40's loop: i - bVI - iv - V in G minor.
const LOOP: [&str; 4] = ["Gm", "Eb", "Cm", "D"];

/// What the interface says about the industrial bed.
pub static CARD: ThemeCard = ThemeCard {
    display: "Industrial",
    text: "A construction site: a distorted kick, an anvil backbeat, struck pipes, a grinder and sheet metal. No bass \
           of its own: the fighters carry the low end.",
    key: "G",
    scale: "minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The episode that plays the industrial bed, its music as its `timeline.py` sets it.
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_barisax_vs_bassclarinet",
    fighters: ["BARI SAX", "BASS CLARINET"],
    bpm: 126.0,
    key: "G",
    scale: "minor",
    intro: ["Gm", "D"],
    chord_loop: &LOOP,
    tail: &["Eb", "D"],
}];

/// `PIPE_PAT`: the pipe ostinato over the bar's chord tones (8ths).
pub const PIPE_PAT: [usize; 8] = [0, 2, 1, 2, 0, 3, 1, 2];

/// `GRIND_16`: the grinder's chug (16ths); 14 and 15 join as it thickens.
pub const GRIND_16: [usize; 5] = [0, 3, 6, 10, 12];

fn length(x: f64) -> usize {
    if x > 0.0 { x as usize } else { 0 }
}

/// `_modes(f0, modes, dur, click, seed, band)`: a struck metal object: damped partials `(ratio, amp, decay)` over
/// `f0` with their own phases, and a strike click.
pub fn struck(f0: f64, modes: &[(f64, f64, f64)], dur: f64, click: f64, seed: u64, band: (f64, f64)) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let mut rng = Generator::new(seed);
    let mut x = vec![0.0; n];
    for &(ratio, a, d) in modes {
        let ph = rng.uniform(0.0, 2.0 * PI);
        for (o, &t) in x.iter_mut().zip(&t) {
            *o += a * dmath::sin(2.0 * PI * f0 * ratio * t + ph) * dmath::exp(-t / d);
        }
    }
    let clk = synth::bp(&noise(seed + 1000, n), band.0, band.1, FILTER_O);
    for i in 0..n {
        x[i] += click * clk[i] * dmath::exp(-t[i] / 0.004);
        x[i] *= clip(t[i] / 0.0008, 0.0, 1.0);
    }
    x
}

/// `anvil(seed)`: an anvil strike: a bright inharmonic ring (about 1.2 kHz) and a hard click.
pub fn anvil(seed: u64) -> Vec<f64> {
    let modes = [
        (1.0, 1.0, 0.22),
        (2.71, 0.6, 0.10),
        (5.18, 0.35, 0.05),
        (8.3, 0.2, 0.025),
    ];
    let x = struck(
        1180.0 * (1.0 + 0.01 * seed as f64),
        &modes,
        0.4,
        0.6,
        seed,
        (2500.0, 10000.0),
    );
    synth::hp(&x, 700.0, FILTER_O).iter().map(|v| v * 0.55).collect()
}

/// `backbeat(seed)`: the industrial snare: a short snare body and the anvil ring.
pub fn backbeat(seed: u64) -> Vec<f64> {
    let snare = synth::hp(&synth::snare(0.10), 220.0, FILTER_O);
    let a = anvil(seed % 4);
    let mut x = vec![0.0; snare.len().max(a.len())];
    for (o, s) in x.iter_mut().zip(&snare) {
        *o += 0.8 * s;
    }
    for (o, s) in x.iter_mut().zip(&a) {
        *o += s;
    }
    x
}

/// `pipe(m, seed)`: a struck steel pipe on MIDI `m`: free-bar partials, a slow beat between two near modes.
pub fn pipe(m: i32, seed: u64) -> Vec<f64> {
    let modes = [
        (1.0, 1.0, 0.45),
        (1.004, 0.5, 0.45),
        (2.756, 0.45, 0.16),
        (5.404, 0.22, 0.06),
    ];
    let x = struck(
        synth::mtof(f64::from(m)),
        &modes,
        0.7,
        0.25,
        seed % 3 + 20,
        (2500.0, 10000.0),
    );
    synth::hp(&x, 420.0, FILTER_O).iter().map(|v| v * 0.6).collect()
}

/// `tick(seed, open)`: a metal tick (a chain link, a rivet): a short bright noise and a tiny high ring.
pub fn tick(seed: u64, open: bool) -> Vec<f64> {
    let h = synth::hat(if open { 0.06 } else { 0.022 }, open);
    let f = 6100.0 + 300.0 * (seed % 3) as f64;
    let t = synth::tt(h.len());
    h.iter()
        .zip(&t)
        .map(|(v, &t)| v + 0.12 * dmath::sin(2.0 * PI * f * t) * dmath::exp(-t / 0.02))
        .collect()
}

/// `KICK`: tight and distorted, high-passed out of the fighters' way.
pub fn kick() -> Vec<f64> {
    let mut k = synth::kick(1.0, 0.08, 170.0, 64.0);
    k.truncate(length(0.17 * SRF));
    let k = synth::hp(&k, 60.0, FILTER_O);
    let k: Vec<f64> = k.iter().map(|v| dmath::tanh(2.2 * v) * 0.75).collect();
    synth::hp(&k, 70.0, FILTER_O)
}

/// The grinder's root for `chord`: its root from D4 to C#5.
///
/// # Errors
/// `chord` does not parse.
pub fn grind_root(chord: &str) -> Result<i32, HarmonyError> {
    Ok(62 + (parse_chord(chord)?.root - 62).rem_euclid(12))
}

/// `grind(root, dur, seed)`: the grinder: a distorted power chord (root, fifth, octave) gated short, high-passed at
/// 480 Hz so the low end stays the fighters'.
pub fn grind(root: i32, dur: f64, seed: u64) -> Vec<f64> {
    let n = length((dur + 0.02) * SRF);
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    for m in [root, root + 7, root + 12] {
        let f = synth::mtof(f64::from(m));
        for dt in [-0.004, 0.004] {
            for (o, &t) in x.iter_mut().zip(&t) {
                let ph = (f * (1.0 + dt) * t + 0.3 * seed as f64).rem_euclid(1.0);
                *o += 2.0 * ph - 1.0;
            }
        }
    }
    let x: Vec<f64> = x
        .iter()
        .zip(&t)
        .map(|(v, &t)| {
            let env = clip(t / 0.003, 0.0, 1.0) * if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.008) };
            dmath::tanh(5.0 * v / 6.0) * env
        })
        .collect();
    let y = synth::hp(&x, 480.0, FILTER_O);
    synth::lp(&y, 4200.0, FILTER_O).iter().map(|v| v * 0.5).collect()
}

/// `sheet(seed, dur)`: a sheet-metal crash: many inharmonic modes wobbling as the sheet flexes, high-passed.
pub fn sheet(seed: u64, dur: f64) -> Vec<f64> {
    let mut r = Generator::new(seed + 50);
    let modes: Vec<(f64, f64, f64)> = (0..16)
        .map(|_| {
            let f = r.uniform(220.0, 3400.0) / 220.0;
            let a = r.uniform(0.3, 1.0);
            (f, a, r.uniform(0.15, 0.7))
        })
        .collect();
    let x = struck(220.0, &modes, dur, 0.4, seed + 50, (800.0, 9000.0));
    let t = synth::tt(x.len());
    let air = synth::bp(&noise(seed + 51, x.len()), 1500.0, 9000.0, FILTER_O);
    let x: Vec<f64> = (0..x.len())
        .map(|i| {
            let t = t[i];
            x[i] * (1.0 + 0.35 * dmath::sin(2.0 * PI * 6.5 * t) * dmath::exp(-t / 0.5))
                + 0.5 * air[i] * dmath::exp(-t / 0.25)
        })
        .collect();
    synth::hp(&x, 400.0, 4).iter().map(|v| v * 0.16).collect()
}

/// `hiss(dur, seed)`: a hydraulic hiss: high air noise, the filter falling as the pressure drops.
pub fn hiss(dur: f64, seed: u64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let x = noise(seed + 70, n);
    let lo = synth::bp(&x, 1200.0, 6000.0, FILTER_O);
    let hi = synth::bp(&x, 4000.0, 11000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let k = clip(t[i] / dur, 0.0, 1.0);
            let y = lo[i] * (0.4 + 0.6 * k) + hi[i] * (1.0 - k);
            y * clip(t[i] / 0.02, 0.0, 1.0) * dmath::exp(-t[i] / (dur * 0.45)) * 0.5
        })
        .collect()
}

/// `wrench(dur, seed)`: an air-wrench burst: a motor whine spinning up and the ratchet's clicks at about 32 Hz.
pub fn wrench(dur: f64, seed: u64) -> Vec<f64> {
    let n = length(dur * SRF);
    let t = synth::tt(n);
    let f: Vec<f64> = t
        .iter()
        .map(|&t| 260.0 + 520.0 * (1.0 - dmath::exp(-t / 0.06)))
        .collect();
    let ph: Vec<f64> = dsp::cumsum(&f).iter().map(|p| 2.0 * PI * p / SRF).collect();
    let raw: Vec<f64> = ph
        .iter()
        .map(|&p| {
            let s = dmath::sin(p);
            let sq = if s > 0.0 {
                1.0
            } else if s < 0.0 {
                -1.0
            } else {
                0.0
            };
            sq * 0.5 + dmath::sin(2.0 * p) * 0.3
        })
        .collect();
    let whine = synth::bp(&raw, 300.0, 3200.0, FILTER_O);
    let clicks = synth::bp(&noise(seed + 90, n), 1500.0, 7000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let gate = if dmath::sin(2.0 * PI * 32.0 * t) > 0.85 {
                1.0
            } else {
                0.0
            };
            let env = clip(t / 0.01, 0.0, 1.0) * clip((dur - t) / 0.03, 0.0, 1.0);
            (whine[i] * 0.35 + clicks[i] * gate * 0.7) * env
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_instruments_sound() {
        for x in [
            anvil(1),
            backbeat(5),
            pipe(67, 2),
            tick(1, false),
            tick(2, true),
            kick(),
            grind(67, 0.09, 1),
            sheet(3, 1.2),
            hiss(0.5, 1),
            wrench(0.4, 2),
        ] {
            assert!(!x.is_empty() && x.iter().all(|v| v.is_finite()));
            assert!(x.iter().any(|v| v.abs() > 0.01));
        }
        assert_eq!(grind_root("Gm"), Ok(67));
        assert_eq!(grind_root("D"), Ok(62));
        assert_eq!(grind_root("C#m"), Ok(73));
    }
}
