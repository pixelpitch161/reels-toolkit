//! The extra voices of the Studio's second batch of themes (2026-11-18): the sounds no older theme had (a piano, a
//! Rhodes, chip-tune waves, a supersaw, a wobble bass, a log drum, a breathy flute, a banjo and a slap bass), each a
//! function of the note, the length and the velocity, like the older themes' voices the batch also borrows.

use bf_core::dmath::{PI, cos, exp, sin, tanh};
use bf_sound::SRF;
use bf_sound::synth::{self, FILTER_O, clip};

/// The samples for `dur` seconds and a `tail` after.
fn n_of(dur: f64, tail: f64) -> usize {
    ((dur.max(0.02) + tail) * SRF) as usize
}

/// A release over `rel` seconds once `t` passes `dur`.
fn rel(t: f64, dur: f64, rel: f64) -> f64 {
    if t < dur { 1.0 } else { exp(-(t - dur) / rel) }
}

/// An acoustic piano note: struck partials that die faster the higher they are, and a felt thump.
pub fn piano(m: i32, dur: f64, v: f64) -> Vec<f64> {
    let f = synth::mtof(f64::from(m));
    let n = n_of(dur, 0.25);
    let thump = synth::noise(m as u64 + 7, n);
    let x: Vec<f64> = synth::tt(n)
        .iter()
        .zip(&thump)
        .map(|(&t, &w)| {
            let mut s = 0.0;
            for k in 1..=6 {
                let kf = f64::from(k);
                let fk = f * kf * (1.0 + 0.0004 * kf * kf);
                s += sin(2.0 * PI * fk * t) * exp(-t * (0.9 + 1.4 * kf)) / kf;
            }
            (0.7 * s + 0.05 * w * exp(-t / 0.01)) * clip(t / 0.003, 0.0, 1.0) * rel(t, dur, 0.12)
        })
        .collect();
    synth::lp(&x, 4200.0, FILTER_O).into_iter().map(|s| s * v).collect()
}

/// A Rhodes tine: FM, the index falling from bright to round, a slow tremolo.
pub fn rhodes(m: i32, dur: f64, v: f64) -> Vec<f64> {
    let f = synth::mtof(f64::from(m));
    let n = n_of(dur, 0.3);
    let x: Vec<f64> = synth::tt(n)
        .iter()
        .map(|&t| {
            let idx = 0.4 + 1.6 * exp(-t / 0.25);
            let tine = sin(2.0 * PI * f * t + idx * sin(2.0 * PI * f * t));
            let trem = 1.0 - 0.22 * (0.5 + 0.5 * sin(2.0 * PI * 4.6 * t));
            tine * trem * clip(t / 0.008, 0.0, 1.0) * exp(-t / (dur.max(0.1) * 1.4)) * rel(t, dur, 0.08)
        })
        .collect();
    synth::lp(&x, 2200.0, FILTER_O).into_iter().map(|s| s * v).collect()
}

/// A chip-tune square wave of `duty`, a hard gate with a tiny click guard.
pub fn chip(m: i32, dur: f64, duty: f64, v: f64) -> Vec<f64> {
    let n = n_of(dur, 0.02);
    let sq = synth::square(synth::mtof(f64::from(m)), n, duty);
    synth::tt(n)
        .iter()
        .zip(sq)
        .map(|(&t, s)| 0.5 * s * v * clip(t / 0.002, 0.0, 1.0) * rel(t, dur, 0.01))
        .collect()
}

/// A chip-tune triangle (the old consoles' bass channel).
pub fn chip_tri(m: i32, dur: f64, v: f64) -> Vec<f64> {
    let n = n_of(dur, 0.02);
    let x = synth::tri(synth::mtof(f64::from(m)), n);
    synth::tt(n)
        .iter()
        .zip(x)
        .map(|(&t, s)| s * v * clip(t / 0.002, 0.0, 1.0) * rel(t, dur, 0.01))
        .collect()
}

/// Chip-tune noise drums: a short burst, `bright` for a hat, dark for a snare or a kick.
pub fn chip_noise(dur: f64, bright: bool, seed: u64) -> Vec<f64> {
    let n = n_of(dur, 0.0);
    let w = synth::noise(seed, n);
    let x: Vec<f64> = w
        .iter()
        .enumerate()
        .map(|(i, s)| s.signum() * exp(-(i as f64 / SRF) / (dur * 0.35)))
        .collect();
    if bright {
        synth::hp(&x, 6000.0, FILTER_O)
    } else {
        synth::lp(&x, 2500.0, FILTER_O)
    }
}

/// A supersaw: seven detuned saws, low-passed at `cut`, with a soft attack.
pub fn supersaw(m: i32, dur: f64, cut: f64, v: f64) -> Vec<f64> {
    let n = n_of(dur, 0.15);
    let x = synth::saw(synth::mtof(f64::from(m)), n, 0.012, 7);
    let x: Vec<f64> = synth::tt(n)
        .iter()
        .zip(x)
        .map(|(&t, s)| 0.6 * s * clip(t / 0.01, 0.0, 1.0) * rel(t, dur, 0.1))
        .collect();
    synth::lp(&x, cut, FILTER_O).into_iter().map(|s| s * v).collect()
}

/// A wobble bass: a fat saw crossfaded between dark and bright `rate` times a second, over a sine sub.
pub fn wobble(m: i32, dur: f64, rate: f64, v: f64) -> Vec<f64> {
    let f = synth::mtof(f64::from(m));
    let n = n_of(dur, 0.05);
    let x = synth::saw(f, n, 0.006, 3);
    let (dark, bright) = (synth::lp(&x, 260.0, FILTER_O), synth::lp(&x, 2400.0, FILTER_O));
    synth::tt(n)
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            let l = 0.5 - 0.5 * cos(2.0 * PI * rate * t);
            let s = (1.0 - l) * dark[i] + l * bright[i] + 0.5 * sin(2.0 * PI * f * t);
            tanh(1.4 * s) * v * clip(t / 0.004, 0.0, 1.0) * rel(t, dur, 0.03)
        })
        .collect()
}

/// An amapiano log drum: a round, pitched thump that bends down onto the note.
pub fn log_drum(m: i32, dur: f64, v: f64) -> Vec<f64> {
    let f = synth::mtof(f64::from(m));
    let n = n_of(dur.min(0.6), 0.1);
    let mut ph = 0.0;
    synth::tt(n)
        .iter()
        .map(|&t| {
            ph += 2.0 * PI * f * (1.0 + 0.6 * exp(-t / 0.015)) / SRF;
            let s = sin(ph) + 0.3 * sin(2.0 * ph);
            tanh(1.5 * s) * v * exp(-t / 0.22) * clip(t / 0.002, 0.0, 1.0)
        })
        .collect()
}

/// A breathy wooden flute (a shakuhachi or a bansuri): a sine with a late vibrato and air around it.
pub fn breath_flute(m: i32, dur: f64, v: f64) -> Vec<f64> {
    let f = synth::mtof(f64::from(m));
    let n = n_of(dur, 0.12);
    let air = synth::bp(
        &synth::noise(m as u64 + 3, n),
        f * 0.8,
        (f * 2.5).min(15_000.0),
        FILTER_O,
    );
    let mut ph = 0.0;
    synth::tt(n)
        .iter()
        .zip(&air)
        .map(|(&t, &a)| {
            let vib = 1.0 + 0.006 * clip((t - 0.15) / 0.3, 0.0, 1.0) * sin(2.0 * PI * 5.2 * t);
            ph += 2.0 * PI * f * vib / SRF;
            let s = sin(ph) + 0.15 * sin(2.0 * ph);
            (0.8 * s + 0.35 * a) * v * clip(t / 0.06, 0.0, 1.0) * rel(t, dur, 0.1)
        })
        .collect()
}

/// A banjo: a bright, short, twangy pluck with the drum-head's quick decay.
pub fn banjo(m: i32, dur: f64, v: f64) -> Vec<f64> {
    let f = synth::mtof(f64::from(m));
    let n = n_of(dur.min(0.5), 0.05);
    let x: Vec<f64> = synth::tt(n)
        .iter()
        .map(|&t| {
            let mut s = 0.0;
            for k in 1..=7 {
                let kf = f64::from(k);
                s += sin(2.0 * PI * f * kf * t) * exp(-t * (6.0 + 4.0 * kf)) / kf.sqrt();
            }
            0.5 * s * clip(t / 0.001, 0.0, 1.0)
        })
        .collect();
    synth::hp(&x, 180.0, FILTER_O).into_iter().map(|s| s * v).collect()
}

/// A slapped electric bass: a thumb pop of bright harmonics on a round body.
pub fn slap(m: i32, dur: f64, v: f64) -> Vec<f64> {
    let f = synth::mtof(f64::from(m));
    let n = n_of(dur.min(0.6), 0.05);
    synth::tt(n)
        .iter()
        .map(|&t| {
            let body = sin(2.0 * PI * f * t) + 0.5 * sin(4.0 * PI * f * t);
            let pop = (sin(6.0 * PI * f * t) + 0.6 * sin(10.0 * PI * f * t)) * exp(-t / 0.03);
            (0.8 * body * exp(-t / 0.35) + 0.6 * pop) * v * clip(t / 0.002, 0.0, 1.0) * rel(t, dur, 0.04)
        })
        .collect()
}
