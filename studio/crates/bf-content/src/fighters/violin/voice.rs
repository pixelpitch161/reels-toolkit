//! VIOLIN's voice: `pp_bridge.voice('VIOLIN', midi, vel, dur)`, which is `violin_lowpoly.bowed(midi_hz(m), dur,
//! vel, 'modern')`: the bowed tone CELLO's port already holds ([`crate::fighters::cello::voice::bowed`]) with
//! `TONE['modern']` (`tilt=.95, form=[(450, .8, .4), (3000, 1.3, .45)], vib=.006, vdel=.1, att=.03`). Made at
//! 44,100 Hz and resampled to 48,000 Hz. It draws one number a note from the violin module's generator, the same
//! stream the CELLO draws from.

use bf_sound::voice::pp;
use bf_sound::voice::{PpNoise, Voice};

use crate::fighters::cello::voice::{Tone, bowed};

/// VIOLIN's register in `pp_bridge`'s roster (`('VIOLIN', 'violin_lowpoly', 'modern', (62, 88), 0, 1.0)`).
pub const REGISTER: (i32, i32) = (62, 88);

/// `TONE['modern']`. It has two formants; the third slot adds nothing (gain 0).
pub const MODERN: Tone = Tone {
    tilt: 0.95,
    form: [(450.0, 0.8, 0.4), (3000.0, 1.3, 0.45), (1000.0, 0.0, 1.0)],
    vib: 0.006,
    vdel: 0.1,
    att: 0.03,
};

/// VIOLIN's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct ViolinVoice;

impl Voice for ViolinVoice {
    fn note(&self, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        let phase = noise.1.violin.random();
        pp::to_48k(&bowed(pp::midi_hz(midi), dur, vel, &MODERN, phase))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "VIOLIN"
    }

    /// Down to -0.15 s, as the CELLO's (the same tone's length).
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.15)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_sounds_and_draws_from_the_violin_generator() {
        let mut noise = PpNoise::start();
        let x = ViolinVoice.note(&mut noise, 76.0, 0.8, 0.2);
        assert!(x.iter().all(|v| v.is_finite()) && x.iter().any(|v| v.abs() > 0.01));
        let mut fresh = PpNoise::start();
        fresh.1.violin.random();
        assert_eq!(noise.1.violin.random().to_bits(), fresh.1.violin.random().to_bits());
    }
}
