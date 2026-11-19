//! MARIMBA's voice: a rosewood bar struck by a yarn mallet. The fundamental rings longest; the bar's tuned overtone
//! (four times the fundamental, slightly flat as real bars are: x 3.93) and a high partial (x 9.2) die quickly; a
//! short band-passed noise click is the mallet. Louder is brighter (more overtone, more click). Made at 48,000 Hz
//! directly; it draws no noise from the shared stream (its click is seeded by the note).

use bf_core::dmath::{self, PI};
use bf_sound::synth;
use bf_sound::voice::{PpNoise, Voice};

/// The marimba's register: a five-octave concert marimba, C2 to C7.
pub const REGISTER: (i32, i32) = (36, 96);

/// Samples a second.
const SR: f64 = 48_000.0;

/// MARIMBA's voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct MarimbaVoice;

impl Voice for MarimbaVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        bar(midi, vel, dur)
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "MARIMBA"
    }

    /// A bar rings on its own: any `dur` down to -0.3 s has a note.
    fn can_hold(&self, dur: f64) -> bool {
        dur.is_finite() && dur > -0.3
    }
}

/// One struck bar of MIDI note `m` at velocity `vel`, held `dur` seconds (a marimba cannot hold: `dur` only sets how
/// long the ring is let go before it is damped, at least 0.3 s), with a 15 ms damp at the end.
pub fn bar(m: f64, vel: f64, dur: f64) -> Vec<f64> {
    let f = synth::mtof(m);
    // low bars ring longer
    let ring = (0.55 - (m - 48.0) * 0.007).clamp(0.18, 0.75);
    let len = (dur.max(0.0) + 0.3).min(ring * 4.0 + 0.05).max(0.3);
    let n = (len * SR) as usize;
    let t = synth::tt(n);
    let seed = (m.round() as i64).rem_euclid(128) as u64 * 31 + 7;
    let click = synth::bp(&synth::noise(seed, n), (900.0 + f * 2.0).min(5400.0), 6000.0, 2);
    let bright = 0.15 + 0.25 * vel.clamp(0.0, 1.0);
    let fd = (0.015 * SR) as usize;
    (0..n)
        .map(|i| {
            let t = t[i];
            let x = dmath::sin(2.0 * PI * f * t) * dmath::exp(-t / ring)
                + bright * dmath::sin(2.0 * PI * 3.93 * f * t) * dmath::exp(-t / (ring * 0.18))
                + 0.06 * dmath::sin(2.0 * PI * 9.2 * f * t) * dmath::exp(-t / 0.015);
            let c = click[i] * dmath::exp(-t / 0.003) * (0.15 + 0.25 * vel.clamp(0.0, 1.0));
            let att = (t / 0.0015).min(1.0);
            let damp = if i + fd >= n { (n - i) as f64 / fd as f64 } else { 1.0 };
            (x + c) * att * damp * vel
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bar_rings_and_dies() {
        let x = bar(72.0, 0.9, 0.2);
        assert!(x.len() >= (0.3 * SR) as usize);
        assert!(x.iter().all(|v| v.is_finite()));
        let peak = x.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        assert!(peak > 0.3 && peak < 2.0, "{peak}");
        assert!(x.last().unwrap().abs() < 1e-3);
        assert_eq!(MarimbaVoice.name(), "MARIMBA");
    }
}
