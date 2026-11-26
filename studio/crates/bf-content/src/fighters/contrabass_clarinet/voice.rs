//! CONTRABASS CLARINET's voice: BASS CLARINET's reed synth ([`crate::fighters::bass_clarinet::voice::bass_reed`])
//! an octave lower, with a growl that grows toward the bottom of its range: the reed flaps so slowly down there that
//! each flap is heard (the "motorboat").

use bf_sound::voice::pp;
use bf_sound::voice::real::{Correction, Fill, Rework};
use bf_sound::voice::{PpNoise, Voice};

use crate::fighters::bass_clarinet::voice::bass_reed;

/// Its register: D1 to Bb3.
pub const REGISTER: (i32, i32) = (26, 58);

/// The growl at MIDI note `m`: 0.6 at D1, easing to 0.15 by D3 and none above.
pub fn growl(m: f64) -> f64 {
    if m >= 50.0 {
        0.0
    } else {
        0.15 + 0.45 * ((50.0 - m) / 24.0).clamp(0.0, 1.0)
    }
}

/// The CONTRABASS CLARINET's added overtones: the harmonics its own note barely has, laid in at a real clarinet's level (dB
/// under the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (26, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -5.8, -25.0]),
    (27, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -4.3, -24.4]),
    (28, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -3.0, -24.1]),
    (29, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -2.0, -24.2]),
    (30, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.3, -24.6]),
    (31, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.8, -25.2]),
    (32, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.4, -25.9]),
    (33, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.1, -99.0]),
    (34, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 0.1, -99.0]),
    (35, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 0.1, -99.0]),
    (36, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 0.1, -99.0]),
    (37, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 0.1, -99.0]),
    (38, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.0, -99.0]),
    (39, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.2, -99.0]),
    (40, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.4, -27.5]),
    (41, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.6, -26.1]),
    (42, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.9, -24.6]),
    (43, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.2, -23.2]),
    (44, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.5, -21.8]),
    (45, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.7, -20.7]),
    (46, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.8, -19.7]),
    (47, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.8, -18.8]),
    (48, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.7, -18.1]),
    (49, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.5, -17.4]),
    (50, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.2, -17.0]),
    (51, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.8, -16.6]),
    (52, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.3, -16.3]),
    (53, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 0.1, -16.0]),
    (54, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 1.5, -10.4]),
    (55, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 1.5, -10.0]),
    (56, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 1.5, -9.8]),
    (57, [-99.0, -99.0, -99.0, -99.0, 0.1, -16.2, -7.1, -28.2]),
    (58, [-99.0, -99.0, -99.0, -99.0, 0.2, -15.9, -7.0, -28.0]),
];

/// The CONTRABASS CLARINET's overtone correction toward a real clarinet (FluidR3 has no contrabass clarinet): gains in dB for its first eight harmonics, note by
/// note (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "CONTRABASS CLARINET"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (26, [0.0, -16.9, 5.8, -9.3, 7.8, -0.1, -1.2, -0.2]),
    (27, [0.0, -15.2, 6.3, -10.2, 8.7, 0.7, -1.1, -0.1]),
    (28, [0.0, -14.3, 6.7, -11.2, 9.3, 0.8, -1.0, -0.2]),
    (29, [0.0, -12.6, 7.0, -11.5, 9.7, 1.6, -1.0, -0.3]),
    (30, [0.0, -11.7, 7.2, -12.7, 10.0, 1.8, -1.0, -0.3]),
    (31, [0.0, -10.8, 7.3, -13.2, 10.0, 1.8, -0.9, -0.4]),
    (32, [0.3, -9.5, 7.6, -13.4, 10.2, 2.4, -0.6, -0.0]),
    (33, [0.6, -7.8, 10.2, -12.5, 12.5, 3.5, 0.2, 14.1]),
    (34, [0.5, -6.8, 10.3, -11.6, 12.3, 2.5, 0.3, 15.4]),
    (35, [0.5, -6.4, 9.9, -11.7, 11.8, 1.8, 0.2, 14.8]),
    (36, [0.6, -6.3, 8.8, -11.9, 10.6, 1.1, 0.0, 10.7]),
    (37, [0.6, -5.8, 8.7, -11.5, 10.3, 0.5, 0.1, 11.1]),
    (38, [0.8, -5.1, 9.2, -11.5, 10.8, 0.4, 0.5, 14.0]),
    (39, [0.7, -5.0, 8.8, -11.4, 10.2, -0.4, 0.4, 13.9]),
    (40, [0.6, -5.8, 6.3, -11.7, 7.7, -2.8, -0.4, 0.6]),
    (41, [0.3, -6.0, 5.7, -11.6, 7.0, -3.6, -0.6, -0.1]),
    (42, [0.1, -6.3, 5.1, -11.5, 6.6, -4.6, -0.9, -0.1]),
    (43, [0.0, -6.5, 4.6, -11.4, 6.3, -4.9, -0.8, 0.0]),
    (44, [0.0, -6.7, 4.2, -10.9, 6.2, -4.8, -0.8, -0.1]),
    (45, [0.0, -6.8, 3.8, -11.2, 6.3, -4.9, -0.6, 0.3]),
    (46, [0.0, -7.2, 3.4, -10.7, 6.6, -4.8, -0.5, 0.1]),
    (47, [0.0, -7.6, 3.0, -10.4, 7.1, -2.8, -0.5, 0.1]),
    (48, [0.0, -8.1, 2.7, -9.8, 7.7, -2.9, -0.4, 0.2]),
    (49, [0.0, -8.5, 2.3, -9.9, 8.7, 0.8, -0.5, -0.1]),
    (50, [0.0, -8.9, 2.1, -9.3, 10.1, 2.9, -0.5, 0.1]),
    (51, [0.0, -9.2, 1.9, -8.4, 11.6, 3.1, -0.5, -0.0]),
    (52, [0.0, -9.5, 1.9, -7.6, 13.2, 4.9, -0.5, 0.1]),
    (53, [0.6, -9.3, 2.6, -6.0, 15.5, 4.0, 0.0, 0.6]),
    (54, [0.8, -9.8, -4.4, 0.4, 10.6, 12.9, 0.0, 0.8]),
    (55, [0.9, -10.3, -4.1, 2.6, 11.7, 14.5, 0.0, 0.8]),
    (56, [0.7, -11.0, -3.9, 4.1, 12.0, 10.7, 0.0, 0.7]),
    (57, [-0.3, -8.1, -21.8, -6.5, 0.4, 0.3, -0.7, 0.1]),
    (58, [-0.5, -8.0, -21.2, -5.5, 0.2, 0.2, -0.8, -0.2]),
];

/// The Composed arrangement's CONTRABASS CLARINET (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added, a slower start (70 ms to full level) and a quicker stop (a 50 ms fade after its length), nearer the real one's.
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.07,
    stop: 0.05,
    keep_shape: false,
};

/// CONTRABASS CLARINET's voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct ContraVoice;

impl Voice for ContraVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&bass_reed(pp::midi_hz(midi), dur, vel, growl(midi)))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn rework(&self) -> Option<&'static Rework> {
        Some(&REWORK)
    }

    fn name(&self) -> &str {
        "CONTRABASS CLARINET"
    }

    /// Down to -0.16 s, as the reed synth's.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_low_note_sounds_and_growls() {
        let mut noise = PpNoise::start();
        let x = ContraVoice.note(&mut noise, 26.0, 0.9, 0.4);
        assert!(x.iter().all(|v| v.is_finite()) && x.iter().any(|v| v.abs() > 0.01));
        assert!(growl(26.0) > growl(40.0) && growl(60.0) == 0.0);
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
    }
}
