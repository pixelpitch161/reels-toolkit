//! ALTO SAX's voice: `pp_bridge.voice('ALTO SAX', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/altosax_lowpoly.py`'s `synth`, which is
//! `alto_tone(_midi_hz(m), dur, v)`: the old alto's tone function (`altosax60_lowpoly`, here
//! `bf_sound::voice::pp::alto::alto_tone`) with that module's modern tone a shade brighter
//! (`TONE = dict(AS.TONES["modern"], bright=.72)`), and without the history video's echo. The note is made at
//! 44,100 Hz and resampled to 48,000 Hz.
//!
//! Its breath is drawn from the old alto module's own generator (`altosax60_lowpoly._RN`, the build's
//! `PpNoise.1.alto`), wherever that stands: a fresh generator of seed 23 at the start of a build, and whatever the
//! ALTO SAX's part last seeded it with after that (`sound.rs`). The series' shared noise stream is not touched.

use bf_sound::voice::pp::alto::{AltoTone, alto_tone};
use bf_sound::voice::pp::{self};
use bf_sound::voice::{PpNoise, Voice};

/// The ALTO SAX's register in `pp_bridge`'s roster (`('ALTO SAX', 'altosax_lowpoly', 'ball', (49, 81), ...)`).
pub const REGISTER: (i32, i32) = (49, 81);

/// `TONE = dict(AS.TONES["modern"], bright=.72)`: `bright=.62, vib=.004, vrate=5.0, formant=1100, fboost=.9,
/// breath=.045, scoop=.035, att=.03` with `bright` raised to 0.72; every other argument at the tone function's
/// default.
pub const TONE: AltoTone = AltoTone {
    bright: 0.72,
    vib: 0.004,
    vrate: 5.0,
    odd: 0.12,
    scoop: 0.035,
    att: 0.03,
    formant: 1100.0,
    fboost: 0.9,
    breath: 0.045,
    growl: 0.0,
    slap: 0.0,
    gl: None,
};

/// The tone's release and tail past the held length, in seconds: a note is `int((dur + .16) * SR)` samples.
pub const TAIL: f64 = 0.16;

/// The ALTO SAX's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct AltoSaxVoice;

impl Voice for AltoSaxVoice {
    fn note(&self, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        // synth: alto_tone(_midi_hz(m), dur, v)
        pp::to_48k(&alto_tone(&mut noise.1.alto, pp::midi_hz(midi), dur, vel, &TONE))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "ALTO SAX"
    }

    /// Down to -0.16 s: numpy refuses a negative number of samples.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + TAIL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tone_is_the_old_altos_modern_one_a_shade_brighter() {
        let d = AltoTone::default();
        assert_eq!(
            (TONE.odd, TONE.growl, TONE.slap, TONE.gl),
            (d.odd, d.growl, d.slap, d.gl)
        );
        assert_eq!(TONE.bright, 0.72);
    }

    #[test]
    fn a_note_is_its_held_length_and_tail_at_48_khz_and_draws_only_the_alto_generator() {
        let mut noise = PpNoise::start();
        let shared = noise.0.clone();
        let alto = noise.1.alto.clone();
        let x = AltoSaxVoice.note(&mut noise, 65.0, 0.8, 0.2);
        // int((0.2 + .16) * 44100) = 15876 samples, resampled by 160 / 147
        assert_eq!(x.len(), 17_280);
        assert!(x.iter().all(|v| v.is_finite()));
        assert_eq!(noise.0.clone().random().to_bits(), shared.clone().random().to_bits());
        assert_ne!(noise.1.alto.clone().random().to_bits(), alto.clone().random().to_bits());
        assert!(AltoSaxVoice.can_hold(-0.1));
        assert!(!AltoSaxVoice.can_hold(-0.2));
    }
}
