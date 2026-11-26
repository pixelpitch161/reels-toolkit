//! DRUMS's voice: `pp_bridge.voice('DRUMS', midi, vel, dur)`, close to it by ear (ruling M4-R16).
//!
//! `pp_bridge` snaps the note to the nearest of the kit's drums (`DRUM_KEYS`: kick 36, snare 38, floor tom 41, low
//! tom 45, high tom 48, crash 49) and plays it through `series/instrument-history/drums60_lowpoly.py`'s `synth` on
//! its first scene, the modern kit: a pitch-dropping driven kick with a sub, a bright snare (body tone over band-passed
//! rattle), pitch-bent toms and a long washy crash, times the velocity. Made straight at 48 kHz; the noise comes from
//! the series' shared stream ([`PpNoise`]).

use bf_core::dmath::{self, PI};
use bf_sound::SRF;
use bf_sound::synth;
use bf_sound::voice::real::Rework;
use bf_sound::voice::{PpNoise, Voice};

/// DRUMS's register in `pp_bridge`'s roster (`('DRUMS', 'drums60_lowpoly', None, (36, 50), 0, 1.0)`).
pub const REGISTER: (i32, i32) = (36, 50);

/// `DRUM_KEYS`: the kit's drums, as GM numbers.
pub const DRUM_KEYS: [i32; 6] = [36, 38, 41, 45, 48, 49];

/// The Composed arrangement's DRUMS (sound upgrade, stage 3, S1): a kit has no overtones to set, so only the small
/// differences between its repeated strokes (a hair of pitch, a little level), so a fill never sounds like a loop.
pub const REWORK: Rework = Rework {
    fill: &[],
    correction: &[],
    speak: 0.0,
    stop: 0.0,
    keep_shape: false,
};

/// DRUMS's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct DrumsVoice;

/// The time of each of `n` samples.
fn tt(n: usize) -> Vec<f64> {
    (0..n).map(|i| i as f64 / SRF).collect()
}

/// A sine whose frequency follows `f` (`sin(2 pi cumsum(f) / SR)`).
fn swept(f: impl Fn(f64) -> f64, t: &[f64]) -> Vec<f64> {
    let mut ph = 0.0;
    t.iter()
        .map(|&t| {
            ph += f(t);
            dmath::sin(2.0 * PI * ph / SRF)
        })
        .collect()
}

/// `kick_s(46, 140, 8, .35, .5, 1.8, .4)`: the modern kick.
fn kick(noise: &mut PpNoise) -> Vec<f64> {
    let (f0, f1, dec, click, drive, sub) = (46.0, 140.0, 8.0, 0.35, 1.8, 0.4);
    let n = (0.5 * SRF) as usize;
    let t = tt(n);
    let mut x: Vec<f64> = swept(|t| f0 + (f1 - f0) * dmath::exp(-t * 35.0), &t)
        .into_iter()
        .zip(&t)
        .map(|(s, &t)| s * dmath::exp(-t * dec))
        .collect();
    let k = (0.004 * SRF) as usize;
    for (o, z) in x.iter_mut().zip(noise.nz(k)) {
        *o += z * click;
    }
    x.iter()
        .zip(&t)
        .map(|(&v, &t)| dmath::tanh(drive * (v + sub * dmath::sin(2.0 * PI * f0 * t) * dmath::exp(-t * dec * 0.6))))
        .collect()
}

/// `snare(1.) * 1.1 + snare_s(240, 22, 35, 2000, 9000, .4, len) * .5`: the modern snare.
fn snare(noise: &mut PpNoise) -> Vec<f64> {
    let n = (0.25 * SRF) as usize;
    let t = tt(n);
    let rattle = synth::bp(&noise.nz(n), 900.0, 7000.0, 2);
    let crack = synth::bp(&noise.nz(n), 2000.0, 9000.0, 2);
    (0..n)
        .map(|i| {
            let body = rattle[i] * dmath::exp(-t[i] * 17.0) * 0.32
                + dmath::sin(2.0 * PI * 186.0 * t[i]) * dmath::exp(-t[i] * 30.0) * 0.55;
            let top = crack[i] * dmath::exp(-t[i] * 22.0) * 0.4
                + dmath::sin(2.0 * PI * 240.0 * t[i]) * dmath::exp(-t[i] * 35.0) * 0.6;
            body * 1.1 + top * 0.5
        })
        .collect()
}

/// `C.tom(f0, 1.)`: a tom bent down from 40 % sharp.
fn tom(f0: f64) -> Vec<f64> {
    let t = tt((0.35 * SRF) as usize);
    swept(|t| f0 * (1.0 + 0.4 * dmath::exp(-t * 20.0)), &t)
        .into_iter()
        .zip(&t)
        .map(|(s, &t)| s * dmath::exp(-t * 9.0))
        .collect()
}

/// `P.crash(1.)`: 2.4 s of differenced noise, a 4 ms attack and a slow decay.
fn crash(noise: &mut PpNoise) -> Vec<f64> {
    let n = (2.4 * SRF) as usize;
    let z = noise.nz(n + 1);
    (0..n)
        .map(|i| {
            let t = i as f64 / SRF;
            (z[i + 1] - z[i]) * dmath::exp(-t * 2.0) * (t / 0.004).min(1.0) * 0.16
        })
        .collect()
}

/// One stroke of drum `m` (snapped to [`DRUM_KEYS`]) at velocity `vel`.
pub fn stroke(noise: &mut PpNoise, m: f64, vel: f64) -> Vec<f64> {
    let key = DRUM_KEYS
        .iter()
        .copied()
        .min_by(|a, b| (f64::from(*a) - m).abs().total_cmp(&(f64::from(*b) - m).abs()))
        .unwrap_or(36);
    let x = match key {
        36 => kick(noise),
        38 => snare(noise),
        41 => tom(115.0),
        45 => tom(165.0),
        48 => tom(210.0),
        _ => crash(noise),
    };
    x.into_iter().map(|v| v * vel).collect()
}

impl Voice for DrumsVoice {
    fn note(&self, noise: &mut PpNoise, midi: f64, vel: f64, _dur: f64) -> Vec<f64> {
        stroke(noise, midi, vel)
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn rework(&self) -> Option<&'static Rework> {
        Some(&REWORK)
    }

    fn name(&self) -> &str {
        "DRUMS"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_drum_is_finite_and_snaps_to_the_kit() {
        let mut noise = PpNoise::start();
        for m in [36.0, 37.0, 38.0, 41.0, 45.0, 48.0, 49.0, 60.0] {
            let x = DrumsVoice.note(&mut noise, m, 0.9, 0.2);
            assert!(!x.is_empty() && x.iter().all(|v| v.is_finite()), "drum {m}");
        }
        let mut a = PpNoise::start();
        assert_eq!(stroke(&mut a, 48.0, 1.0).len(), (0.35 * SRF) as usize);
    }
}
