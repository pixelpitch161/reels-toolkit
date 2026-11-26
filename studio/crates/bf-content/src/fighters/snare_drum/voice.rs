//! SNARE DRUM's voice: a stick on a tensioned snare. A short tuned thump of the batter head (its pitch set a little
//! by the note) and a band-passed noise burst for the snare wires rattling under the bottom head; louder is brighter
//! and rattles longer. A rim shot adds a hard click of the stick on the hoop and a ringing crack; a press roll is a
//! dense buzz. Made at 48,000 Hz directly; it draws no noise from the shared stream (its noise is seeded by the note).

use bf_core::dmath::{self, PI};
use bf_sound::synth;
use bf_sound::voice::real::Rework;
use bf_sound::voice::{PpNoise, Voice};

/// The snare's register: the head is tuned around C5; notes only shade its pitch.
pub const REGISTER: (i32, i32) = (60, 84);

/// Samples a second.
const SR: f64 = 48_000.0;

/// How a stroke is played.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stroke {
    /// A plain tap (an accent is a louder tap).
    Tap,
    /// A rim shot: stick on head and hoop at once.
    Rim,
}

/// The Composed arrangement's SNARE DRUM (sound upgrade, stage 3, S1): a drum has no overtones to set, so only the
/// small differences between its repeated strokes (a hair of pitch, a little level).
pub const REWORK: Rework = Rework {
    fill: &[],
    correction: &[],
    speak: 0.0,
    stop: 0.0,
    keep_shape: false,
    tail: &[],
};

/// SNARE DRUM's voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct SnareDrumVoice;

impl Voice for SnareDrumVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        if dur > 0.3 {
            roll(midi, vel, dur)
        } else {
            hit(midi, vel, Stroke::Tap)
        }
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn rework(&self) -> Option<&'static Rework> {
        Some(&REWORK)
    }

    fn name(&self) -> &str {
        "SNARE DRUM"
    }

    /// A long note is a press roll: any `dur` has a note.
    fn can_hold(&self, dur: f64) -> bool {
        dur.is_finite() && dur > -0.3
    }
}

/// One stroke on the head tuned near MIDI note `m` at velocity `vel`.
pub fn hit(m: f64, vel: f64, how: Stroke) -> Vec<f64> {
    let v = vel.clamp(0.0, 1.0);
    let f0 = 190.0 * synth::mtof(m) / synth::mtof(72.0).max(1.0);
    let f0 = f0.clamp(140.0, 280.0);
    let rim = how == Stroke::Rim;
    let len = if rim { 0.32 } else { 0.16 + 0.08 * v };
    let n = (len * SR) as usize;
    let t = synth::tt(n);
    let seed = (m.round() as i64).rem_euclid(128) as u64 * 17 + if rim { 5 } else { 3 };
    let wires = synth::bp(&synth::noise(seed, n), 1800.0, 9000.0, 2);
    let click = synth::hp(&synth::noise(seed + 101, n), 4000.0, 2);
    let w_dec = 0.035 + 0.04 * v + if rim { 0.03 } else { 0.0 };
    (0..n)
        .map(|i| {
            let t = t[i];
            let head = dmath::sin(2.0 * PI * f0 * t) * dmath::exp(-t / 0.03)
                + 0.4 * dmath::sin(2.0 * PI * f0 * 1.6 * t) * dmath::exp(-t / 0.018);
            let w = wires[i] * dmath::exp(-t / w_dec) * (0.6 + 0.6 * v);
            let c = click[i] * dmath::exp(-t / 0.002) * (0.3 + 0.4 * v);
            let crack = if rim {
                0.6 * dmath::sin(2.0 * PI * 920.0 * t) * dmath::exp(-t / 0.05) + 0.5 * click[i] * dmath::exp(-t / 0.006)
            } else {
                0.0
            };
            let att = (t / 0.0008).min(1.0);
            let fade = if i + 240 >= n { (n - i) as f64 / 240.0 } else { 1.0 };
            (head * 0.8 + w + c + crack) * att * fade * v * if rim { 0.6 } else { 0.55 }
        })
        .collect()
}

/// A press roll near MIDI note `m` at velocity `vel` for `dur` seconds: the wires buzzing under fast bounces.
pub fn roll(m: f64, vel: f64, dur: f64) -> Vec<f64> {
    let v = vel.clamp(0.0, 1.0);
    let len = dur.clamp(0.05, 8.0) + 0.08;
    let n = (len * SR) as usize;
    let t = synth::tt(n);
    let seed = (m.round() as i64).rem_euclid(128) as u64 * 13 + 29;
    let wires = synth::bp(&synth::noise(seed, n), 1500.0, 8000.0, 2);
    let f0 = 190.0;
    (0..n)
        .map(|i| {
            let t = t[i];
            // about 28 bounces a second, each a small burst
            let ph = (t * 28.0).fract();
            let env = dmath::exp(-ph / 0.25);
            let body = 0.3 * dmath::sin(2.0 * PI * f0 * t) * env;
            let att = (t / 0.01).min(1.0);
            let tail = if t > dur { dmath::exp(-(t - dur) / 0.03) } else { 1.0 };
            (wires[i] * (0.4 + 0.6 * env) + body) * att * tail * v * 0.7
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stroke_cracks_and_dies() {
        for how in [Stroke::Tap, Stroke::Rim] {
            let x = hit(72.0, 0.9, how);
            assert!(x.iter().all(|v| v.is_finite()));
            let peak = x.iter().fold(0.0f64, |a, v| a.max(v.abs()));
            assert!(peak > 0.2 && peak < 3.0, "{peak}");
            assert!(x.last().unwrap().abs() < 1e-2);
        }
        let r = roll(72.0, 0.8, 1.0);
        assert!(r.len() as f64 > 1.0 * SR);
        assert!(r.iter().all(|v| v.is_finite()));
        assert_eq!(SnareDrumVoice.name(), "SNARE DRUM");
    }
}
