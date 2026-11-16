//! PICCOLO's voice: a small, bright, nearly pure tone. The fundamental with a little second and third harmonic, a
//! quick 12 ms attack with a breathy "chiff" (band-passed noise) on the front, a fast narrow vibrato that comes in
//! after 0.15 s on longer notes, and a 40 ms release. Louder is brighter and breathier. Made at 48,000 Hz directly;
//! it draws no noise from the shared stream (its breath is seeded by the note).

use bf_core::dmath::{self, PI};
use bf_sound::synth;
use bf_sound::voice::{PpNoise, Voice};

/// The piccolo's register: D5 to C8 (it sounds an octave above the written note).
pub const REGISTER: (i32, i32) = (74, 108);

/// Samples a second.
const SR: f64 = 48_000.0;

/// PICCOLO's voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct PiccoloVoice;

impl Voice for PiccoloVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        tone(midi, vel, dur)
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "PICCOLO"
    }

    /// Down to -0.04 s: the release alone is a note.
    fn can_hold(&self, dur: f64) -> bool {
        dur.is_finite() && dur > -0.04
    }
}

/// One piccolo note of MIDI note `m` at velocity `vel`, held `dur` seconds, then a 40 ms release.
pub fn tone(m: f64, vel: f64, dur: f64) -> Vec<f64> {
    let f = synth::mtof(m);
    let dur = dur.max(0.0);
    let n = ((dur + 0.06) * SR) as usize;
    let t = synth::tt(n);
    let seed = (m.round() as i64).rem_euclid(128) as u64 * 17 + 3;
    let breath = synth::bp(&synth::noise(seed, n), f * 0.8, (f * 3.0).min(16_000.0), 2);
    let v = vel.clamp(0.0, 1.0);
    let bright = 0.12 + 0.18 * v;
    let mut ph = 0.0;
    (0..n)
        .map(|i| {
            let t = t[i];
            let vib = 1.0 + 0.004 * dmath::sin(2.0 * PI * 6.2 * t) * ((t - 0.15) / 0.2).clamp(0.0, 1.0);
            ph += 2.0 * PI * f * vib / SR;
            let x = dmath::sin(ph) + bright * (0.5 * dmath::sin(2.0 * ph) + 0.2 * dmath::sin(3.0 * ph));
            let chiff = breath[i] * (0.25 + 0.3 * v) * dmath::exp(-t / 0.025) + breath[i] * 0.04;
            let rise = (t / 0.012).min(1.0);
            let hold = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.012) };
            (0.8 * x + chiff) * rise * hold * vel
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_sounds_and_ends() {
        let x = tone(93.0, 0.9, 0.25);
        assert!(x.len() >= (0.3 * SR) as usize);
        assert!(x.iter().all(|v| v.is_finite()));
        let peak = x.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        assert!(peak > 0.3 && peak < 2.0, "{peak}");
        assert!(x.last().unwrap().abs() < 0.05);
        assert_eq!(PiccoloVoice.name(), "PICCOLO");
    }
}
