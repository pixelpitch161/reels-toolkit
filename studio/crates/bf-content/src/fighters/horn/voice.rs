//! The HORN's voice: `pp_bridge.voice('HORN', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/horn60_lowpoly.py`'s `synth` on the scene its
//! roster selects for the HORN, the module's first: the modern double horn (key `modern`). That is two notes of
//! `brass` (the shared block of `trumpet_lowpoly`, `bf_sound::voice::pp::brass`) with the tone `TONE['modern']`
//! (`bright=.55, vib=.002, scoop=.02, tilt=1.0`, `rough` at its default): the note itself and one a touch sharp
//! (x 1.002), mixed `.65` and `.35` for a section's width. The note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! The voice draws no noise: the shared stream is left where it was.

use bf_sound::voice::pp;
use bf_sound::voice::pp::brass::{Brass, brass};
use bf_sound::voice::{PpNoise, Voice};

/// The HORN's register in `pp_bridge`'s roster (`('HORN', 'horn60_lowpoly', None, (48, 70), 0, 1.0)`). As a
/// fighter it plays from 53 to 77; `kit.music.inst_gain` measures the voice on the middle of this one.
pub const REGISTER: (i32, i32) = (48, 70);

/// `TONE['modern']`: `dict(bright=.55, vib=.002, scoop=.02, tilt=1.0)`.
const TONE: Brass = Brass {
    bright: 0.55,
    vib: 0.002,
    scoop: 0.02,
    rough: 0.0,
    tilt: 1.0,
};

/// The HORN's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct HornVoice;

impl Voice for HornVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "HORN"
    }

    /// Down to -0.12 s: the tone is `int((dur + .12) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.12)
    }
}

/// `horn60_lowpoly.synth` for the one note `pp_bridge.voice` asks of the modern scene, at 44,100 Hz before it is
/// resampled: `x = brass(midi_hz(m), dur, v, **tone)`, `y = brass(midi_hz(m) * 1.002, dur, v, **tone)`,
/// `.65 * x + .35 * y[:len(x)]`. `midi` is a whole number.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or so far below -0.12 s that the note has a negative number of
/// samples.
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    let f = pp::midi_hz(midi);
    let x = brass(f, dur, vel, &TONE);
    let y = brass(f * 1.002, dur, vel, &TONE);
    x.iter().zip(&y).map(|(a, b)| 0.65 * a + 0.35 * b).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_voice_draws_no_noise_and_is_the_synth_at_48k() {
        let mut noise = PpNoise::start();
        let note = HornVoice.note(&mut noise, 62.0, 0.8, 0.25);
        let raw = synth(62.0, 0.8, 0.25);
        assert_eq!(note, pp::to_48k(&raw));
        // int((dur + .12) * 44100) samples
        assert_eq!(raw.len(), 16317);
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert_eq!((HornVoice.register(), HornVoice.name()), (REGISTER, "HORN"));
        assert!(HornVoice.can_hold(0.0) && HornVoice.can_hold(-0.12));
        assert!(!HornVoice.can_hold(-0.2) && !HornVoice.can_hold(f64::NAN));
        assert!(synth(62.0, 0.8, -0.12).is_empty());
    }

    #[test]
    fn a_note_is_two_horns_a_touch_apart() {
        let f = pp::midi_hz(62.0);
        let (x, y) = (brass(f, 0.3, 0.8, &TONE), brass(f * 1.002, 0.3, 0.8, &TONE));
        let own = synth(62.0, 0.8, 0.3);
        assert_eq!(own.len(), x.len());
        assert_eq!(own[1000], 0.65 * x[1000] + 0.35 * y[1000]);
        assert_ne!(own, x);
    }
}
