//! BASS GUITAR's voice (a from-scratch fighter, 2026-11-21): a picked electric bass. Two Karplus-Strong strings (the
//! shared `bf_sound::voice::pp::pluck`, the ACOUSTIC's), a hair apart, darker and longer-ringing than a guitar's,
//! through a low-pass for the pickup's warmth, with the pick's click on top (a short bright pluck two octaves up) and
//! a quick palm-mute fade when the note is let go. Made at 44,100 Hz and resampled to 48,000 Hz, like the other Pixel
//! Pitch voices. It draws no noise: its strings come from their own seeded generators.

use bf_sound::voice::engine2::amp::Amp;
use bf_sound::voice::engine2::plucked::PluckedParts;
use bf_sound::voice::engine2::space::{Home, Space};
use bf_sound::voice::engine2::{Instrument, Parts};
use bf_sound::voice::pp;
use bf_sound::voice::pp::pluck::ks_pluck;
use bf_sound::voice::{PpNoise, Voice};

/// BASS GUITAR's register: a four-string bass, E1 to G3 (and the upper frets to C4).
pub const REGISTER: (i32, i32) = (28, 60);

/// The Composed arrangement's BASS GUITAR (Voice Engine 2): a four-string electric bass built from how it works, on the
/// plucked-string engine (Aedan's pick B3: picked, one pickup, through a bass amp pushed for a little grit), with
/// every technique a bass has, its room kept off the low end (`docs/studio/voice-engine-2-instruments.md`, "Bass
/// guitar"). Classic keeps the note below.
pub static ENGINE2: Instrument = Instrument::built(
    "BASS GUITAR",
    Parts::Plucked(PluckedParts::B3),
    Space {
        width: 0.0,
        tilt: 0.0,
        home: Some(Home {
            length: 2.4,
            under: 15.0,
            hp: 120.0,
        }),
        amp: Some(Amp::BASS_GRIT),
    },
);

/// BASS GUITAR's voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct BassGuitarVoice;

impl Voice for BassGuitarVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn engine2(&self) -> Option<&'static Instrument> {
        Some(&ENGINE2)
    }

    fn name(&self) -> &str {
        "BASS GUITAR"
    }
}

/// One bass note at 44,100 Hz, before it is resampled: it rings for its held length (at least 0.12 s, at most 2 s)
/// and fades out over 0.08 s after it.
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    let f = pp::midi_hz(midi);
    let held = dur.clamp(0.12, 2.0);
    let ring = held + 0.08;
    let a = ks_pluck(f, ring, 0.55, 0.9994, 3);
    let b = ks_pluck(f * 1.0018, ring, 0.55, 0.9994, 61);
    let body: Vec<f64> = a.iter().zip(&b).map(|(&a, &b)| a * 0.7 + b * 0.3).collect();
    let mut x = pp::lp(&body, 1700.0);
    let click = ks_pluck(f * 4.0, 0.03, 1.0, 0.99, 11);
    for (v, c) in x.iter_mut().zip(&click) {
        *v += c * 0.18;
    }
    let n_held = pp::len(held);
    let n_fade = pp::len(0.08).max(1);
    for (i, v) in x.iter_mut().enumerate() {
        let g = if i < n_held {
            1.0
        } else {
            (1.0 - (i - n_held) as f64 / n_fade as f64).max(0.0)
        };
        *v *= g * vel * 1.6;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bass_is_its_own_pluck_and_draws_no_noise() {
        let mut noise = PpNoise::start();
        let note = BassGuitarVoice.note(&mut noise, 40.0, 0.9, 0.3);
        assert_eq!(note, pp::to_48k(&synth(40.0, 0.9, 0.3)));
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert!(note.iter().all(|v| v.is_finite()));
        assert!(note.iter().any(|v| v.abs() > 0.01), "it sounds");
        assert_ne!(
            synth(52.0, 0.9, 0.3),
            crate::fighters::acoustic::voice::synth(52.0, 0.9, 0.3)
        );
        assert_eq!(
            (BassGuitarVoice.register(), BassGuitarVoice.name()),
            (REGISTER, "BASS GUITAR")
        );
    }
}
