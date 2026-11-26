//! OCTOBASS's voice: CONTRABASS's bowed and plucked string synth ([`crate::fighters::contrabass::voice::synth`]) at
//! the octobass's pitch, with the same note an octave up mixed in under it so its lowest notes still read on a
//! phone's small speakers (the real instrument's fundamental is felt more than heard).

use bf_sound::voice::pp;
use bf_sound::voice::real::{self, Correction, Rework};
use bf_sound::voice::{PpNoise, Voice};

use crate::fighters::contrabass::voice::synth;

/// Its register: Bb0 to Bb2.
pub const REGISTER: (i32, i32) = (22, 46);

/// How much of the octave above is mixed in.
const UPPER: f64 = 0.55;

/// The OCTOBASS's overtone correction toward a real contrabass: gains in dB for its first eight harmonics, note by
/// note, for its bowed notes (FluidR3, `bf-harness/tests/reference/fluidr3.json`; FluidR3 has no octobass, so the
/// reference is its nearest relative). Written by
/// `cargo run --release -p bf-harness --example references -- correct "OCTOBASS"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (22, [-4.1, 0.0, -6.0, -2.5, -0.4, -17.4, -0.2, -15.2]),
    (23, [-1.8, 0.0, -3.3, 1.5, 3.0, -15.0, 2.6, -11.1]),
    (24, [-2.5, 0.0, -3.6, 2.1, 3.5, -16.0, 3.1, -10.3]),
    (25, [-4.1, 0.0, -4.0, 2.5, 3.6, -17.5, 3.4, -10.0]),
    (26, [-6.2, 0.0, -4.3, 2.8, 3.4, -18.1, 4.1, -9.1]),
    (27, [-8.6, 0.0, -4.4, 3.0, 3.2, -18.4, 4.7, -8.2]),
    (28, [-11.0, 0.0, -4.2, 3.1, 3.2, -17.9, 5.6, -7.1]),
    (29, [-13.3, 0.0, -3.6, 3.1, 3.5, -17.2, 6.3, -6.7]),
    (30, [-10.1, 0.0, -7.5, 1.9, -12.1, -23.8, 1.7, -5.2]),
    (31, [-11.3, 0.0, -7.0, 2.0, -11.4, -21.3, 2.8, -5.5]),
    (32, [-12.4, 0.0, -6.7, 2.3, -10.6, -17.9, 2.9, -5.1]),
    (33, [6.6, 0.0, 7.5, -13.5, 8.2, -4.2, 14.7, -9.3]),
    (34, [5.6, 0.0, 7.4, -13.6, 8.9, -4.0, 13.9, -9.6]),
    (35, [-1.7, 0.0, 7.5, -14.3, 9.8, -5.0, 13.7, -10.7]),
    (36, [1.5, -19.7, -0.6, -17.2, -3.1, -10.7, -12.9, -16.5]),
    (37, [1.2, -19.5, -1.1, -16.2, -3.6, -11.4, -13.0, -16.3]),
    (38, [1.0, -19.4, -1.3, -15.2, -4.4, -11.9, -11.7, -16.2]),
    (39, [1.9, 0.0, -5.5, -15.9, 11.4, -5.8, -7.8, -14.8]),
    (40, [1.9, 0.0, -5.2, -15.2, 11.0, -5.9, -7.7, -14.7]),
    (41, [2.0, 0.0, -4.5, -14.6, 11.0, -5.7, -7.5, -14.6]),
    (42, [0.6, -5.3, -3.2, -6.5, -15.0, -20.0, -7.9, -11.7]),
    (43, [0.6, -5.4, -2.3, -6.0, -14.6, -19.5, -7.5, -11.0]),
    (44, [0.5, -5.6, -1.4, -5.8, -14.3, -18.6, -6.7, -10.1]),
    (45, [0.4, -22.6, -0.4, -22.0, -8.3, -9.5, -10.5, 1.6]),
    (46, [0.4, -23.3, -0.4, -20.8, -9.0, -9.2, -10.1, 0.2]),
];

/// The Composed arrangement's OCTOBASS (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, a slower start (80 ms to full level) and a quicker stop, nearer the real one's.
pub const REWORK: Rework = Rework {
    fill: &[],
    correction: CORRECTION,
    speak: 0.08,
    stop: 0.055,
    keep_shape: false,
};

/// OCTOBASS's voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct OctobassVoice;

impl Voice for OctobassVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        let low = synth(midi.round(), vel, dur);
        let high = synth(midi.round() + 12.0, vel, dur);
        let n = low.len().max(high.len());
        let x: Vec<f64> = (0..n)
            .map(|i| low.get(i).copied().unwrap_or(0.0) + UPPER * high.get(i).copied().unwrap_or(0.0))
            .collect();
        pp::to_48k(&x)
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn rework(&self) -> Option<&'static Rework> {
        Some(&REWORK)
    }

    /// A bowed note (0.5 s or more) takes [`REWORK`]; a plucked one only its repeats' differences.
    fn note_real(&self, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64, variant: u64) -> Vec<f64> {
        let x = self.note(noise, midi, vel, dur);
        let how = if dur >= 0.5 {
            &REWORK
        } else {
            &crate::fighters::contrabass::voice::PLUCKED
        };
        real::rework(&x, midi, dur, bf_sound::SRF, how, variant)
    }

    fn name(&self) -> &str {
        "OCTOBASS"
    }

    /// Any finite length, as the contrabass synth's.
    fn can_hold(&self, dur: f64) -> bool {
        dur.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_low_note_sounds() {
        let mut noise = PpNoise::start();
        let x = OctobassVoice.note(&mut noise, 22.0, 0.9, 0.8);
        assert!(x.iter().all(|v| v.is_finite()) && x.iter().any(|v| v.abs() > 0.01));
    }
}
