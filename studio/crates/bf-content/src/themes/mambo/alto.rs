//! The alto voice the mambo bed plays for its sax section: `pp_bridge.voice(ALTO, midi, vel, dur)`, where
//! [`ALTO`](super::ALTO) is the voice's `pp_bridge` id (`RETIRED[1]`: an internal id only, never shown).
//!
//! `pp_bridge` plays one note through `series/instrument-history/altosax60_lowpoly.py`'s `synth` on the module's
//! first scene, the modern alto (key `modern`, no glide): `sax_tone(midi_hz(m), dur, v, **TONES['modern'])`
//! (`bf_sound::voice::pp::alto::alto_tone` with [`TONE`]), then, because the scene is the modern one, two echoes
//! of the whole note, 0.75 of a beat of that video (96 BPM) and twice that later, at 0.22 and 0.08 of its level.
//! The note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! It is a voice of the theme, never a fighter (no roster fighter plays it). Its breath is drawn from the old alto
//! module's own generator (`altosax60_lowpoly._RN`, the build's `PpNoise.1.alto`), which the ALTO SAX's voice and
//! part use too: wherever that stands (a fresh generator of seed 23 at the start of a build, and whatever the
//! ALTO SAX's part last seeded it with after that).

use std::sync::Arc;

use bf_sound::voice::pp::alto::{AltoTone, alto_tone};
use bf_sound::voice::pp::{self};
use bf_sound::voice::{PpNoise, Voice};

use super::ALTO;

/// The old alto's register in `pp_bridge`'s roster (its entry's `(56, 78)`): its level is measured at the middle.
pub const REGISTER: (i32, i32) = (56, 78);

/// `TONES['modern']`: `bright=.62, vib=.004, vrate=5.0, formant=1100, fboost=.9, breath=.045, scoop=.035,
/// att=.03`; every other argument at the tone function's default (`gl=None`: the modern scene has no glides).
pub const TONE: AltoTone = AltoTone {
    bright: 0.62,
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

/// `altosax60_lowpoly.BPM`: the tempo of the alto's video, which sets the echoes' delay.
const BPM: f64 = 96.0;

/// The modern scene's echoes: (delay in delays, level), after the note itself.
const ECHOES: [(usize, f64); 2] = [(1, 0.22), (2, 0.08)];

/// The tone's release and tail past the held length, in seconds: a tone is `int((dur + .16) * SR)` samples.
pub const TAIL: f64 = 0.16;

/// The old alto's Pixel Pitch voice, the mambo's sax section.
#[derive(Clone, Copy, Debug, Default)]
pub struct AltoVoice;

impl Voice for AltoVoice {
    fn note(&self, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(noise, midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        ALTO
    }

    /// Down to -0.16 s: numpy refuses a negative number of samples.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + TAIL)
    }
}

/// [`Theme::own_voice`](crate::themes::Theme::own_voice) of the mambo: the alto for its id, nothing else.
pub fn own_voice(pp: &str) -> Option<Arc<dyn Voice>> {
    (pp == ALTO).then(|| Arc::new(AltoVoice) as Arc<dyn Voice>)
}

/// `altosax60_lowpoly.synth` for the one note `pp_bridge.voice` asks of the modern scene: the note at 44,100 Hz,
/// before it is resampled. `midi` is a whole number.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or below -0.16 s (a negative number of samples).
pub fn synth(noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    // x = sax_tone(midi_hz(m), dur, v, gl=None, **TONES['modern'])
    let x = alto_tone(&mut noise.1.alto, pp::midi_hz(midi), dur, vel, &TONE);
    // _echo(x, .75 * BEAT_S, (.22, .08)): d = int(dl * SR); y = zeros(len(x) + d * 2); y[:len(x)] += x;
    // y[d * j:d * j + len(x)] += a * x for j, a in enumerate(taps, 1)
    let d = pp::len(0.75 * (60.0 / BPM));
    let mut y = vec![0.0; x.len() + d * ECHOES.len()];
    for (o, &v) in y.iter_mut().zip(&x) {
        *o += v;
    }
    for (j, a) in ECHOES {
        for (o, &v) in y[d * j..].iter_mut().zip(&x) {
            *o += a * v;
        }
    }
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tone_is_the_old_altos_modern_one() {
        let d = AltoTone::default();
        assert_eq!(
            (TONE.odd, TONE.growl, TONE.slap, TONE.gl),
            (d.odd, d.growl, d.slap, d.gl)
        );
        assert_eq!((TONE.bright, TONE.breath), (0.62, 0.045));
    }

    #[test]
    fn a_note_rings_into_two_echoes_and_draws_only_the_alto_generator() {
        let mut noise = PpNoise::start();
        let shared = noise.0.clone();
        let fresh = noise.1.alto.clone();
        let x = synth(&mut noise, 67.0, 0.55, 0.2);
        // int((0.2 + .16) * 44100) = 15876 samples of tone, then two delays of int(.75 * .625 * 44100) = 20671
        assert_eq!(x.len(), 15876 + 2 * 20671);
        // the note is over before its first echo: each echo is the note, quieter
        let peak = |s: &[f64]| s.iter().fold(0.0f64, |p, v| p.max(v.abs()));
        let p0 = peak(&x[..15876]);
        assert!((peak(&x[20671..20671 + 15876]) - 0.22 * p0).abs() < 1e-12 * p0.max(1.0));
        assert_eq!(noise.0.clone().random().to_bits(), shared.clone().random().to_bits());
        assert_ne!(
            noise.1.alto.clone().random().to_bits(),
            fresh.clone().random().to_bits()
        );
        assert_eq!(
            AltoVoice.note(&mut PpNoise::start(), 67.0, 0.55, 0.2).len(),
            (x.len() * 160).div_ceil(147)
        );
        assert!(AltoVoice.can_hold(-0.1) && !AltoVoice.can_hold(-0.2));
    }

    #[test]
    fn the_theme_hands_out_the_alto_for_its_id_only() {
        assert_eq!(own_voice(ALTO).map(|v| v.name().to_string()), Some(ALTO.to_string()));
        assert!(own_voice("ALTO SAX").is_none() && own_voice("PIANO").is_none());
    }
}
