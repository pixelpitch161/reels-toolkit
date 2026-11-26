//! TIMPANI's voice: a tuned calfskin-style head on a copper kettle, struck by a felt mallet. A kettle drum's pitch is
//! its (1,1) membrane mode, with the overtones the kettle tunes near harmonic: x 1.5, x 2 and x 2.5 (slightly off,
//! as real drums are: 1.504, 1.992, 2.44), each dying faster than the one below. The felt strike is a soft, low
//! band of noise and a quick downward pitch drop at the attack (the head's tension settling). Louder is brighter and
//! rings longer. Made at 48,000 Hz directly; its strike noise is seeded by the note, never the shared stream.

use bf_core::dmath::{self, PI};
use bf_sound::synth;
use bf_sound::voice::real::Rework;
use bf_sound::voice::{PpNoise, Voice};

/// A four-drum concert set from the 32" drum's D2 to the 23" drum's A3, with a little pedal room each way.
pub const REGISTER: (i32, i32) = (36, 60);

/// Samples a second.
const SR: f64 = 48_000.0;

/// The Composed arrangement's TIMPANI (sound upgrade, stage 3, S1): a drum's overtones are not a note's harmonics,
/// so there are none to set against the real timpani (the overtone measure does not fit it); only the small
/// differences between its repeated strokes (a hair of pitch, a little level).
pub const REWORK: Rework = Rework {
    correction: &[],
    speak: 0.0,
    stop: 0.0,
};

/// TIMPANI's voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct TimpaniVoice;

impl Voice for TimpaniVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        stroke(midi, vel, dur, 0.0)
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn rework(&self) -> Option<&'static Rework> {
        Some(&REWORK)
    }

    fn name(&self) -> &str {
        "TIMPANI"
    }

    /// A struck head rings on its own: any `dur` down to -0.3 s has a note.
    fn can_hold(&self, dur: f64) -> bool {
        dur.is_finite() && dur > -0.3
    }
}

/// One stroke of MIDI note `m` at velocity `vel`, let ring `dur` seconds (at least 0.35 s, then a 30 ms damp), its
/// pitch gliding `gliss` semitones over the note (a pedal glissando; 0 for an ordinary stroke).
pub fn stroke(m: f64, vel: f64, dur: f64, gliss: f64) -> Vec<f64> {
    let v = vel.clamp(0.0, 1.0);
    let ring = (1.6 - (m - 40.0) * 0.04).clamp(0.6, 1.9) * (0.7 + 0.4 * v);
    let len = (dur.max(0.0) + 0.35).min(ring * 2.2 + 0.05).max(0.35);
    let n = (len * SR) as usize;
    let t = synth::tt(n);
    let seed = (m.round() as i64).rem_euclid(128) as u64 * 53 + 11;
    let felt = synth::bp(&synth::noise(seed, n), 90.0, 900.0, 2);
    let f0 = synth::mtof(m);
    let fd = (0.03 * SR) as usize;
    let partials = [
        (1.0, 1.0, 1.0),
        (1.504, 0.45 + 0.25 * v, 0.55),
        (1.992, 0.30 * v + 0.1, 0.35),
        (2.44, 0.18 * v, 0.2),
    ];
    let mut ph = [0.0f64; 4];
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let ti = t[i];
        // the strike's pitch drop (a quarter tone settling in 60 ms) and the pedal's glide
        let semis = 0.5 * dmath::exp(-ti / 0.06) + gliss * (ti / len).min(1.0);
        let f = f0 * dmath::exp(semis * std::f64::consts::LN_2 / 12.0);
        let mut x = 0.0;
        for (j, &(r, a, d)) in partials.iter().enumerate() {
            ph[j] += 2.0 * PI * f * r / SR;
            x += a * dmath::sin(ph[j]) * dmath::exp(-ti / (ring * d));
        }
        let thump = felt[i] * dmath::exp(-ti / 0.025) * (0.5 + 0.6 * v);
        let att = (ti / 0.004).min(1.0);
        let damp = if i + fd >= n { (n - i) as f64 / fd as f64 } else { 1.0 };
        out.push((0.55 * x + thump) * att * damp * vel);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stroke_rings_and_dies() {
        let x = stroke(45.0, 0.9, 0.5, 0.0);
        assert!(x.len() >= (0.35 * SR) as usize);
        assert!(x.iter().all(|v| v.is_finite()));
        let peak = x.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        assert!(peak > 0.2 && peak < 2.0, "{peak}");
        assert!(x.last().unwrap().abs() < 1e-3);
        let g = stroke(45.0, 0.9, 0.5, 5.0);
        assert_eq!(g.len(), x.len());
        assert_ne!(g, x);
        assert_eq!(TimpaniVoice.name(), "TIMPANI");
    }
}
