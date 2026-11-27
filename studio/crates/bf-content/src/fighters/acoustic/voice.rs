//! ACOUSTIC's voice: `pp_bridge.voice('ACOUSTIC', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/acoustic_lowpoly.py`'s `synth` on the scene its
//! roster selects for the ACOUSTIC (key `steel`, the modern steel-string): two Karplus-Strong strings
//! (`piano_lowpoly.ks_pluck`, the shared `bf_sound::voice::pp::pluck`), the note's pitch at 0.7 and a string tuned
//! 1.0025 sharp at 0.3, each ringing 1.8 s whatever the held length, bright 0.9, decay 0.9988, seeded 0 and 50 (the
//! note's index and 50 more), times the velocity. The note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! The voice draws no noise: its strings come from their own seeded generators, so the shared stream is left where
//! it was. It is also the stadium rock bed's unplugged strums (`M.note('ACOUSTIC', ...)`).

use bf_sound::voice::engine2::plucked::PluckedParts;
use bf_sound::voice::engine2::space::{Home, Space};
use bf_sound::voice::engine2::{Instrument, Parts};
use bf_sound::voice::pp;
use bf_sound::voice::pp::pluck::ks_pluck;
use bf_sound::voice::{PpNoise, Voice};

/// The ACOUSTIC's register in `pp_bridge`'s roster (`('ACOUSTIC', 'acoustic_lowpoly', 'steel', (40, 76), 0, 1.0)`);
/// also its roster `register`. `kit.music.inst_gain` measures the voice on the middle of it.
pub const REGISTER: (i32, i32) = (40, 76);

/// The Composed arrangement's ACOUSTIC (Voice Engine 2): a dreadnought steel-string guitar built from how it works,
/// on the plucked-string engine (Aedan's pick A6: new steel strings, plucked by the fingers, through the box, a light
/// studio polish), with every technique a guitar has, in a small room of its own
/// (`docs/studio/voice-engine-2-instruments.md`, "Acoustic guitar"). Classic keeps the note below.
pub static ENGINE2: Instrument = Instrument::built(
    "ACOUSTIC",
    Parts::Plucked(PluckedParts::A6),
    Space {
        width: 0.3,
        tilt: 0.0,
        home: Some(Home {
            length: 2.4,
            under: 15.0,
            hp: 120.0,
        }),
        amp: None,
    },
);

/// The ACOUSTIC's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct AcousticVoice;

impl Voice for AcousticVoice {
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
        "ACOUSTIC"
    }
}

/// `acoustic_lowpoly.synth` for the one note `pp_bridge.voice` asks of the `steel` scene (its index 0, a chord size
/// of 1: no strum offset), at 44,100 Hz before it is resampled:
///
/// ```python
/// f = midi_hz(m); ring = 1.8
/// x = P.ks_pluck(f, ring, .9, .9988, idx)*.7 + P.ks_pluck(f*1.0025, ring, .9, .9988, idx + 50)*.3
/// return x*v
/// ```
///
/// The held length `dur` is not read: the strings ring their 1.8 s.
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    let _ = dur;
    let f = pp::midi_hz(midi);
    let ring = 1.8;
    let a = ks_pluck(f, ring, 0.9, 0.9988, 0);
    let b = ks_pluck(f * 1.0025, ring, 0.9, 0.9988, 50);
    a.iter().zip(&b).map(|(&a, &b)| (a * 0.7 + b * 0.3) * vel).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_voice_draws_no_noise_and_is_the_synth_at_48k() {
        let mut noise = PpNoise::start();
        let note = AcousticVoice.note(&mut noise, 52.0, 0.9, 0.25);
        let raw = synth(52.0, 0.9, 0.25);
        assert_eq!(note, pp::to_48k(&raw));
        // int(1.8 * 44100) samples, whatever the held length
        assert_eq!(raw.len(), 79380);
        assert_eq!(synth(52.0, 0.9, 3.0), raw);
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert_eq!((AcousticVoice.register(), AcousticVoice.name()), (REGISTER, "ACOUSTIC"));
    }
}
