//! PICCOLO's voice: a small, bright, nearly pure tone. The fundamental with a little second and third harmonic, a
//! quick 12 ms attack with a breathy "chiff" (band-passed noise) on the front, a fast narrow vibrato that comes in
//! after 0.15 s on longer notes, and a 40 ms release. Louder is brighter and breathier. Made at 48,000 Hz directly;
//! it draws no noise from the shared stream (its breath is seeded by the note).

use bf_core::dmath::{self, PI};
use bf_sound::synth;
use bf_sound::voice::real::{Correction, Fill, Rework};
use bf_sound::voice::{PpNoise, Voice};

/// The piccolo's register: D5 to C8 (it sounds an octave above the written note).
pub const REGISTER: (i32, i32) = (74, 108);

/// Samples a second.
const SR: f64 = 48_000.0;

/// The PICCOLO's added overtones: the harmonics its own note barely has, laid in at a real piccolo's level (dB
/// under the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (74, [-99.0, -99.0, -99.0, -23.6, -32.4, -36.5, -45.7, -99.0]),
    (75, [-99.0, -99.0, -99.0, -24.0, -33.3, -36.8, -46.1, -99.0]),
    (76, [-99.0, -99.0, -99.0, -25.0, -33.6, -37.4, -46.7, -99.0]),
    (77, [-99.0, -99.0, -99.0, -28.4, -36.5, -48.5, -99.0, -99.0]),
    (78, [-99.0, -99.0, -99.0, -27.3, -37.6, -45.9, -99.0, -99.0]),
    (79, [-99.0, -99.0, -99.0, -24.1, -30.4, -47.1, -99.0, -99.0]),
    (80, [-99.0, -99.0, -99.0, -24.5, -30.4, -48.0, -99.0, -99.0]),
    (81, [-99.0, -99.0, -99.0, -29.3, -39.5, -99.0, -99.0, -99.0]),
    (82, [-99.0, -99.0, -99.0, -29.9, -41.0, -99.0, -99.0, -99.0]),
    (83, [-99.0, -99.0, -99.0, -33.6, -45.0, -99.0, -99.0, -99.0]),
    (84, [-99.0, -99.0, -99.0, -33.4, -43.8, -99.0, -99.0, -99.0]),
    (85, [-99.0, -99.0, -99.0, -45.0, -99.0, -99.0, -99.0, -99.0]),
    (86, [-99.0, -99.0, -99.0, -45.2, -99.0, -99.0, -99.0, -99.0]),
    (87, [-99.0, -99.0, -99.0, -46.0, -99.0, -99.0, -99.0, -99.0]),
    (88, [-99.0, -99.0, -99.0, -99.0, -48.4, -99.0, -99.0, -99.0]),
    (89, [-99.0, -99.0, -99.0, -40.1, -47.9, -99.0, -99.0, -99.0]),
    (90, [-99.0, -99.0, -99.0, -41.9, -99.0, -99.0, -99.0, -99.0]),
    (91, [-99.0, -99.0, -99.0, -44.0, -49.2, -99.0, -99.0, -99.0]),
    (92, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (93, [-99.0, -99.0, -99.0, -99.0, -41.4, -99.0, -99.0, -99.0]),
    (94, [-99.0, -99.0, -99.0, -99.0, -42.8, -99.0, -99.0, -99.0]),
    (95, [-99.0, -99.0, -99.0, -99.0, -44.5, -99.0, -99.0, -99.0]),
    (96, [-99.0, -99.0, -99.0, -99.0, -46.5, -99.0, -99.0, -99.0]),
    (97, [-99.0, -99.0, -99.0, -99.0, -48.9, -99.0, -99.0, -99.0]),
    (98, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (99, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (100, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (101, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (102, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (103, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (104, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (105, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (106, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (107, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (108, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
];

/// The PICCOLO's overtone correction toward a real piccolo: gains in dB for its first eight harmonics, note by
/// note (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "PICCOLO"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (74, [0.0, 3.4, 5.7, 2.7, 3.3, 3.2, 3.8, 0.0]),
    (75, [0.0, 2.9, 6.2, 2.8, 3.4, 3.2, 3.8, 0.0]),
    (76, [0.0, 2.2, 6.8, 3.2, 3.2, 3.4, 4.3, 0.0]),
    (77, [0.0, -15.5, 8.3, 3.3, 2.9, 2.9, 0.0, 0.0]),
    (78, [0.0, -15.1, 8.5, 3.1, 2.7, 4.0, 0.0, 0.0]),
    (79, [0.0, -7.3, 5.4, 3.2, 3.2, 3.9, 0.0, 0.0]),
    (80, [0.0, -8.6, 6.3, 3.0, 3.3, 3.9, 0.0, 0.0]),
    (81, [0.0, -16.4, 6.3, 3.0, 3.8, 0.0, 0.0, 0.0]),
    (82, [0.0, -16.0, 7.2, 2.9, 3.9, 0.0, 0.0, 0.0]),
    (83, [0.0, -12.2, 11.2, 2.7, 5.3, 0.0, 0.0, 0.0]),
    (84, [0.0, -12.3, 11.0, 2.9, 5.0, 0.0, 0.0, 0.0]),
    (85, [0.0, 9.5, -9.0, 3.6, 0.0, 0.0, 0.0, 0.0]),
    (86, [0.0, 9.8, -7.7, 4.4, 0.0, 0.0, 0.0, 0.0]),
    (87, [0.0, 9.4, -6.2, 4.0, 2.9, 0.0, 0.0, 0.0]),
    (88, [0.0, 8.7, -5.9, 15.5, 4.3, 0.0, 0.0, 0.0]),
    (89, [0.0, -6.3, -1.9, 4.1, 4.4, 0.0, 0.0, 0.0]),
    (90, [0.0, -9.4, -1.5, 4.6, 0.0, 0.0, 0.0, 0.0]),
    (91, [0.0, -13.0, -1.8, 4.5, 5.2, 0.0, 0.0, 0.0]),
    (92, [0.0, -14.0, -1.5, 15.4, 2.8, 0.0, 0.0, 0.0]),
    (93, [0.0, -9.1, 2.0, 0.0, 5.2, 0.0, 0.0, 0.0]),
    (94, [0.0, -10.2, 0.7, 0.0, 4.8, 0.0, 0.0, 0.0]),
    (95, [0.0, -11.0, -0.2, 0.0, 5.1, 0.0, 0.0, 0.0]),
    (96, [0.0, -12.2, -1.7, 0.0, 4.9, 0.0, 0.0, 0.0]),
    (97, [0.0, -13.5, -3.3, 0.0, 5.0, 0.0, 0.0, 0.0]),
    (98, [0.0, -14.1, -4.8, 0.0, 0.0, 0.0, 0.0, 0.0]),
    (99, [0.0, -12.6, -4.4, 0.0, 0.0, 0.0, 0.0, 0.0]),
    (100, [0.0, -11.4, -3.9, 0.0, 0.0, 0.0, 0.0, 0.0]),
    (101, [0.0, -11.2, -4.9, 0.0, 0.0, 0.0, 0.0, 0.0]),
    (102, [0.0, -11.1, -5.7, 0.0, 0.0, 0.0, 0.0, 0.0]),
    (103, [0.0, -10.9, -6.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    (104, [0.0, -10.5, -6.5, 0.0, 0.0, 0.0, 0.0, 0.0]),
    (105, [0.0, -9.9, -8.3, 0.0, 0.0, 0.0, 0.0, 0.0]),
    (106, [0.0, -9.4, -9.4, 0.0, 0.0, 0.0, 0.0, 0.0]),
    (107, [0.0, -9.6, -9.9, 0.0, 0.0, 0.0, 0.0, 0.0]),
    (108, [0.0, -9.6, -11.1, 0.0, 0.0, 0.0, 0.0, 0.0]),
];

/// The Composed arrangement's PICCOLO (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added, and a slower start (40 ms to full level), nearer the real one's.
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.04,
    stop: 0.0,
    keep_shape: false,
};

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

    fn rework(&self) -> Option<&'static Rework> {
        Some(&REWORK)
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
