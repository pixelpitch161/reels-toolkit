//! HARP's voice: `pp_bridge.voice('HARP', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/harp60_lowpoly.py`'s `synth` on its modern scene
//! (the roster's scene key is `None`): `harp_tone` with the `modern` tone (bright 0.55, sustain 1.1, body 0.3). Up
//! to 25 partials below 9 kHz, each weighted by where the string is plucked (0.28 of its length) and dying faster the
//! higher it is, over a body sine; a 1.5 ms attack and a 0.15 s fade at the end of its ring (`dur + 1` s, between
//! 1.3 and 2.6 s), times the velocity. The note is made at 44,100 Hz and resampled to 48,000 Hz. It draws no noise.

use bf_core::dmath::{self, PI};
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::real::{Correction, Fill, Rework};
use bf_sound::voice::{PpNoise, Voice};

/// HARP's register in `pp_bridge`'s roster (`('HARP', 'harp60_lowpoly', None, (50, 84), 0, 1.0)`).
pub const REGISTER: (i32, i32) = (50, 84);

/// The HARP's added overtones: the harmonics its own note barely has, laid in at a real harp's level (dB
/// under the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (50, [-99.0, 20.0, 0.3, 14.7, -99.0, -99.0, -99.0, -99.0]),
    (51, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (52, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (53, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (54, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (55, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (56, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (57, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (58, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (59, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (60, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (61, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (62, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (63, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (64, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (65, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (66, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (67, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (68, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (69, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (70, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (71, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (72, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (73, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (74, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (75, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (76, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (77, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (78, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (79, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (80, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (81, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (82, [-99.0, -99.0, 2.1, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (83, [-99.0, -99.0, 1.4, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (84, [-99.0, -99.0, 0.7, -99.0, -99.0, -99.0, -99.0, -99.0]),
];

/// The HARP's overtone correction toward a real harp: gains in dB for its first eight harmonics, note by
/// note (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "HARP"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (50, [0.2, 0.0, -0.2, 0.3, -8.7, 3.9, 0.0, -1.2]),
    (51, [-3.9, 6.3, 6.6, -0.6, -30.0, -19.2, 6.1, -12.6]),
    (52, [-4.1, 6.3, 6.3, -0.8, -30.0, -19.2, 6.1, -13.1]),
    (53, [-4.2, 6.4, 6.1, -0.9, -30.0, -19.2, 5.9, -13.6]),
    (54, [-4.3, 6.4, 6.0, -1.1, -30.0, -19.1, 5.4, -14.0]),
    (55, [-4.3, 6.4, 5.9, -1.2, -30.0, -19.1, 4.8, -14.3]),
    (56, [-6.0, 6.4, 6.4, -0.4, -30.0, -17.2, -0.6, -6.9]),
    (57, [-5.7, 6.4, 6.5, -1.0, -30.0, -17.0, -1.0, -7.2]),
    (58, [-5.3, 6.4, 6.6, -1.7, -30.0, -16.7, -1.4, -7.5]),
    (59, [-4.9, 6.5, 6.7, -2.3, -30.0, -16.4, -1.8, -7.9]),
    (60, [0.0, -15.7, -20.1, -23.1, -15.1, -26.9, -9.9, -20.3]),
    (61, [0.0, -14.9, -20.4, -23.3, -15.3, -26.8, -9.7, -20.1]),
    (62, [0.0, -14.1, -20.8, -23.5, -15.5, -26.6, -9.5, -19.9]),
    (63, [0.0, -24.9, -24.3, -25.6, -25.1, -26.2, -9.3, -19.8]),
    (64, [0.0, -24.6, -25.1, -25.4, -26.0, -26.0, -9.1, -19.5]),
    (65, [0.0, -24.7, -26.0, -25.2, -27.1, -25.8, -8.9, -19.3]),
    (66, [0.0, -27.3, -14.2, -17.6, -29.3, -25.6, -8.7, -19.1]),
    (67, [0.0, -26.7, -14.8, -17.3, -29.5, -25.4, -8.4, -18.8]),
    (68, [0.0, -26.0, -15.5, -17.1, -29.3, -25.2, -8.2, -18.5]),
    (69, [0.0, -30.0, -13.5, -24.9, -28.8, -24.9, -7.9, -18.2]),
    (70, [0.0, -30.0, -11.8, -24.8, -28.6, -24.6, -7.6, -17.8]),
    (71, [0.0, -30.0, -10.3, -24.7, -28.4, -24.4, -7.4, -17.5]),
    (72, [0.0, -28.3, -3.9, -25.2, -15.6, -24.7, -7.2, -17.2]),
    (73, [0.0, -27.9, -4.3, -25.0, -16.4, -24.3, -6.8, -16.9]),
    (74, [0.0, -27.5, -4.6, -24.7, -17.4, -24.0, -6.5, -16.4]),
    (75, [0.0, -27.1, -4.9, -24.4, -18.5, -23.6, -6.1, -16.0]),
    (76, [0.0, -14.4, -4.2, -24.4, -12.5, -20.1, -5.7, -15.5]),
    (77, [0.0, -14.6, -5.0, -24.0, -13.4, -21.0, -5.2, -15.0]),
    (78, [0.0, -14.9, -5.6, -23.7, -14.4, -22.0, -4.7, -14.5]),
    (79, [0.0, -15.6, -6.0, -23.4, -15.6, -22.1, -4.3, -13.9]),
    (80, [0.0, -16.4, -6.3, -23.0, -17.1, -21.6, -3.8, -13.4]),
    (81, [0.0, -17.4, -6.7, -22.7, -18.9, -21.1, -3.3, -12.8]),
    (82, [0.5, -7.0, 0.0, -5.5, -12.1, -14.6, 6.2, -9.9]),
    (83, [0.5, -7.3, 0.0, -6.6, -14.5, -17.0, 4.9, -9.9]),
    (84, [0.2, -7.6, 0.0, -7.7, -17.4, -17.5, 3.1, -6.5]),
];

/// The Composed arrangement's HARP (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added; its own pluck and ring (its soft pluck comes through the correction within
/// 10 ms, and keeping its shape would pull its overtones off the real harp's).
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.0,
    stop: 0.0,
    keep_shape: false,
};

/// HARP's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct HarpVoice;

impl Voice for HarpVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&harp_tone(pp::midi_hz(midi), dur, vel, 0.55, 1.1, 0.3))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn rework(&self) -> Option<&'static Rework> {
        Some(&REWORK)
    }

    fn name(&self) -> &str {
        "HARP"
    }
}

/// `harp_tone(f, dur, v, bright, sus, body)` (no buzz, no wire, plucked at 0.28) at 44,100 Hz.
pub fn harp_tone(f: f64, dur: f64, v: f64, bright: f64, sus: f64, body: f64) -> Vec<f64> {
    let ring = (dur + 1.0).clamp(1.3, 2.6);
    let n = (ring * SRF) as usize;
    let mut x = vec![0.0; n];
    for k in 1..26 {
        let kf = f64::from(k);
        let fk = f * kf;
        if fk > 9000.0 {
            break;
        }
        let a = (dmath::sin(PI * kf * 0.28).abs() + 0.05) / dmath::pow(kf, 1.5 - 0.8 * bright);
        let dk = (1.0 + 0.5 * (kf - 1.0)) * (0.55 + f / 650.0) / sus;
        for (i, o) in x.iter_mut().enumerate() {
            let t = i as f64 / SRF;
            *o += a * dmath::sin(2.0 * PI * fk * t + kf * 0.7) * dmath::exp(-t * dk);
        }
    }
    for (i, o) in x.iter_mut().enumerate() {
        let t = i as f64 / SRF;
        *o += body * dmath::sin(2.0 * PI * f * t) * dmath::exp(-t * (0.8 + f / 900.0));
        *o *= (t / 0.0015).min(1.0) * ((ring - t) / 0.15).min(1.0) * v;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_rings_between_its_limits_and_draws_no_noise() {
        let raw = harp_tone(pp::midi_hz(62.0), 0.3, 0.8, 0.55, 1.1, 0.3);
        assert_eq!(raw.len(), (1.3 * SRF) as usize);
        assert_eq!(raw[0], 0.0);
        assert!(raw.iter().all(|v| v.is_finite()));
        let mut noise = PpNoise::start();
        let note = HarpVoice.note(&mut noise, 62.0, 0.8, 0.3);
        assert_eq!(note, pp::to_48k(&raw));
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
    }
}
