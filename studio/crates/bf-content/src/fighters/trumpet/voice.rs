//! The TRUMPET's voice: `pp_bridge.voice('TRUMPET', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/trumpet_lowpoly.py`'s `synth` on the scene its
//! roster selects for the TRUMPET, the module's first: the modern trumpet (key `modern`). That is `brass` (the
//! shared block, `bf_sound::voice::pp::brass`) with the tone `TONE['modern']` (`bright=1.1, vib=.006, scoop=.03,
//! rough=0`, `tilt` at its default): up to 29 harmonics summed with a dark and a bright set of weights, blended by
//! the note's own level; a scoop up into the pitch, a vibrato from 0.18 s, a 35 ms attack and a 30 ms release
//! after the held length. The note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! The voice draws no noise: the shared stream is left where it was. The `0.7` level of a note inside a chord is
//! not taken: the lone note `pp_bridge` plays has a chord size of 1.

use bf_sound::voice::pp;
use bf_sound::voice::pp::brass::{Brass, brass};
use bf_sound::voice::{PpNoise, Voice};

/// The TRUMPET's register in `pp_bridge`'s roster (`('TRUMPET', 'trumpet_lowpoly', None, (60, 82), 0, 1.0)`). As a
/// fighter it plays from 60 to 84; `kit.music.inst_gain` measures the voice on the middle of this one.
pub const REGISTER: (i32, i32) = (60, 82);

/// `TONE['modern']`: `dict(bright=1.1, vib=.006, scoop=.03, rough=0)`.
const TONE: Brass = Brass {
    bright: 1.1,
    vib: 0.006,
    scoop: 0.03,
    rough: 0.0,
    tilt: 0.75,
};

/// The TRUMPET's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct TrumpetVoice;

impl Voice for TrumpetVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "TRUMPET"
    }

    /// Down to -0.12 s: the tone is `int((dur + .12) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.12)
    }
}

/// `trumpet_lowpoly.synth` for the one note `pp_bridge.voice` asks of the modern scene: `brass(midi_hz(m), dur, v,
/// **TONE['modern'])` at 44,100 Hz, before it is resampled (times 1.0: the chord size is 1). `midi` is a whole
/// number.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or so far below -0.12 s that the note has a negative number of
/// samples.
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    brass(pp::midi_hz(midi), dur, vel, &TONE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_voice_draws_no_noise_and_is_the_synth_at_48k() {
        let mut noise = PpNoise::start();
        let note = TrumpetVoice.note(&mut noise, 72.0, 0.9, 0.25);
        let raw = synth(72.0, 0.9, 0.25);
        assert_eq!(note, pp::to_48k(&raw));
        // int((dur + .12) * 44100) samples
        assert_eq!(raw.len(), 16317);
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert_eq!((TrumpetVoice.register(), TrumpetVoice.name()), (REGISTER, "TRUMPET"));
        // what it can hold is what Python does not raise for
        assert!(TrumpetVoice.can_hold(0.0) && TrumpetVoice.can_hold(-0.12));
        assert!(!TrumpetVoice.can_hold(-0.2) && !TrumpetVoice.can_hold(f64::NAN));
        assert!(synth(72.0, 0.8, -0.12).is_empty());
    }

    #[test]
    fn the_tone_is_the_modern_trumpets() {
        // brighter and with more vibrato than the block's default tone
        let default = brass(pp::midi_hz(72.0), 0.5, 0.9, &Brass::default());
        let own = synth(72.0, 0.9, 0.5);
        assert_eq!(default.len(), own.len());
        assert_ne!(default, own);
    }
}
