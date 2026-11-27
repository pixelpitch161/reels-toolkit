//! HARP's voice: `pp_bridge.voice('HARP', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/harp60_lowpoly.py`'s `synth` on its modern scene
//! (the roster's scene key is `None`): `harp_tone` with the `modern` tone (bright 0.55, sustain 1.1, body 0.3). Up
//! to 25 partials below 9 kHz, each weighted by where the string is plucked (0.28 of its length) and dying faster the
//! higher it is, over a body sine; a 1.5 ms attack and a 0.15 s fade at the end of its ring (`dur + 1` s, between
//! 1.3 and 2.6 s), times the velocity. The note is made at 44,100 Hz and resampled to 48,000 Hz. It draws no noise.

use bf_core::dmath::{self, PI};
use bf_sound::voice::engine2::plucked::PluckedParts;
use bf_sound::voice::engine2::space::{Home, Space};
use bf_sound::voice::engine2::{Instrument, Parts};
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::{PpNoise, Voice};

/// HARP's register in `pp_bridge`'s roster (`('HARP', 'harp60_lowpoly', None, (50, 84), 0, 1.0)`).
pub const REGISTER: (i32, i32) = (50, 84);

/// The Composed arrangement's HARP (Voice Engine 2): a concert pedal harp built from how it works, on the
/// plucked-string engine (Aedan's pick H3: gut strings plucked a little nearer the soundboard, through its long
/// soundboard and deep box), with every technique a harp has, in a light room of its own
/// (`docs/studio/voice-engine-2-instruments.md`, "Harp"). Classic keeps the note below.
pub static ENGINE2: Instrument = Instrument::built(
    "HARP",
    Parts::Plucked(PluckedParts::H3),
    Space {
        width: 0.35,
        tilt: 0.0,
        home: Some(Home {
            length: 3.3,
            under: 14.0,
            hp: 120.0,
        }),
        amp: None,
    },
);

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

    fn engine2(&self) -> Option<&'static Instrument> {
        Some(&ENGINE2)
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
