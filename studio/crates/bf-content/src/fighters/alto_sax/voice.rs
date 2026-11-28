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

use bf_sound::voice::engine2::reed::ReedParts;
use bf_sound::voice::engine2::space::{Home, Space};
use bf_sound::voice::engine2::{Instrument, Parts};
use bf_sound::voice::pp::alto::{AltoTone, alto_tone, alto_tone_real};
use bf_sound::voice::pp::{self};
use bf_sound::voice::real::{self, Profile, Shape};
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

/// A real alto sax's first eight overtones, dB under the strongest (FluidR3, `bf-harness/tests/reference/fluidr3.json`,
/// written by `cargo run --release -p bf-harness --example references -- profile "alto sax" 49 81`).
#[rustfmt::skip]
pub const PROFILE: &Profile = &[
    (49, [-16.4, -2.6, -6.4, -8.1, 0.0, -4.7, -2.8, -10.2]),
    (50, [-16.6, -2.7, -6.5, -8.3, 0.0, -4.6, -2.5, -10.0]),
    (51, [-16.7, -2.8, -6.5, -8.5, 0.0, -4.5, -2.0, -9.7]),
    (52, [-10.4, -6.9, 0.0, -13.5, -1.0, -10.1, -23.9, -23.3]),
    (53, [-10.4, -6.9, 0.0, -13.3, -1.0, -10.2, -24.5, -24.5]),
    (54, [-12.4, -12.3, 0.0, -0.7, -12.0, -17.7, -13.9, -15.1]),
    (55, [-12.2, -12.1, 0.0, -0.5, -11.7, -17.8, -13.9, -15.0]),
    (56, [0.0, -6.8, -4.0, -7.7, -16.0, -7.6, -6.0, -23.2]),
    (57, [0.0, -6.7, -3.8, -7.7, -16.1, -7.4, -6.5, -23.6]),
    (58, [-1.8, 0.0, -3.7, -14.1, -16.8, -12.1, -14.5, -24.0]),
    (59, [-1.7, 0.0, -3.6, -13.8, -17.9, -12.4, -14.5, -24.1]),
    (60, [-2.9, 0.0, -6.2, -10.4, -6.7, -20.4, -14.0, -9.4]),
    (61, [-2.9, 0.0, -6.0, -10.1, -6.7, -20.3, -14.1, -9.3]),
    (62, [0.0, -13.0, -13.3, -10.4, -15.7, -19.7, -14.4, -11.8]),
    (63, [0.0, -12.9, -12.7, -10.6, -15.0, -19.7, -14.5, -11.8]),
    (64, [0.0, -4.5, -5.7, -9.8, -17.4, -12.1, -9.1, -18.0]),
    (65, [0.0, -4.5, -5.5, -9.4, -18.0, -11.8, -8.1, -17.7]),
    (66, [0.0, -3.5, -7.1, -23.3, -21.7, -17.5, -25.1, -29.4]),
    (67, [0.0, -3.4, -7.0, -22.7, -21.1, -17.9, -24.9, -29.0]),
    (68, [-7.2, 0.0, -6.5, -21.7, -5.7, -21.7, -31.5, -29.4]),
    (69, [-7.5, 0.0, -6.0, -20.7, -6.6, -21.3, -32.9, -28.1]),
    (70, [0.0, -13.0, -8.7, -25.1, -15.7, -25.8, -35.8, -48.5]),
    (71, [0.0, -12.6, -9.2, -24.6, -15.9, -25.9, -35.8, -48.6]),
    (72, [0.0, -12.2, -9.5, -9.3, -16.6, -26.6, -42.0, -35.6]),
    (73, [0.0, -11.9, -9.0, -9.0, -16.6, -26.1, -42.5, -36.2]),
    (74, [-3.3, 0.0, -8.4, -3.3, -18.1, -27.0, -38.5, -27.3]),
    (75, [-2.2, 0.0, -7.7, -3.4, -16.3, -26.6, -38.4, -27.2]),
    (76, [0.0, -4.4, -8.6, -9.8, -22.9, -49.2, -37.2, -35.5]),
    (77, [0.0, -5.1, -9.6, -11.5, -23.0, -49.7, -38.8, -37.1]),
    (78, [-1.8, 0.0, -0.8, -9.7, -30.4, -25.2, -30.9, -35.8]),
    (79, [-0.1, 0.0, -0.6, -7.6, -30.1, -25.3, -31.1, -34.2]),
    (80, [0.0, -14.1, -13.0, -17.7, -41.5, -33.3, -44.7, -45.3]),
    (81, [0.0, -13.9, -12.9, -17.8, -42.0, -33.5, -44.5, -45.3]),
];

/// The Composed arrangement's alto sax (sound upgrade, stage 3, S1): the real one's overtones at the note's pitch, its
/// time to speak and to stop (FluidR3: 28 ms to speak, 63 ms to a tenth after the release), and a reed buzz.
pub fn shape(f: f64) -> Shape {
    Shape {
        levels: real::levels(PROFILE, real::midi_of(f)),
        speak: 0.038,
        release: 0.027,
        buzz: 0.05,
    }
}

/// The Composed arrangement's note of MIDI `midi` at 44,100 Hz: [`alto_tone_real`] with [`TONE`] and [`shape`],
/// the `variant`th repeat a few cents and a little level off the first.
pub fn tone_real(
    rn: &mut bf_core::rng::Generator,
    midi: f64,
    dur: f64,
    vel: f64,
    tone: &AltoTone,
    variant: u64,
) -> Vec<f64> {
    let f = pp::midi_hz(midi);
    let (pitch, level) = if variant == 0 {
        (1.0, 1.0)
    } else {
        real::wobble(real::pitch_seed(f) ^ (variant << 32), 4.0, 0.04)
    };
    alto_tone_real(rn, f * pitch, dur, vel * level, tone, &shape(f))
}

/// The ALTO SAX's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct AltoSaxVoice;

/// The Composed arrangement's ALTO SAX (Voice Engine 2): built from how it works, on the reed engine (Aedan's pick As2,
/// the jazz alto: an open, bright mouthpiece, played with a singing vibrato), with every technique it has, in a light
/// room of its own (`docs/studio/voice-engine-2-instruments.md`, "The reeds"). Classic keeps the note below.
pub static ENGINE2: Instrument = Instrument::built(
    "ALTO SAX",
    Parts::Reed(ReedParts::AS2),
    Space {
        width: 0.0,
        tilt: 0.0,
        home: Some(Home {
            length: 2.6,
            under: 14.0,
            hp: 120.0,
        }),
        amp: None,
    },
);

impl Voice for AltoSaxVoice {
    fn engine2(&self) -> Option<&'static Instrument> {
        Some(&ENGINE2)
    }

    fn note(&self, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        // synth: alto_tone(_midi_hz(m), dur, v)
        pp::to_48k(&alto_tone(&mut noise.1.alto, pp::midi_hz(midi), dur, vel, &TONE))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn reworked(&self) -> bool {
        true
    }

    fn note_real(&self, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64, variant: u64) -> Vec<f64> {
        pp::to_48k(&tone_real(&mut noise.1.alto, midi, dur, vel, &TONE, variant))
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
    fn the_real_tone_is_its_own_and_its_repeats_differ() {
        let rn = || bf_core::rng::Generator::new(23);
        let today = alto_tone(&mut rn(), pp::midi_hz(60.0), 0.5, 0.8, &TONE);
        let real0 = tone_real(&mut rn(), 60.0, 0.5, 0.8, &TONE, 0);
        assert_eq!(real0.len(), today.len());
        assert_ne!(real0, today);
        assert!(real0.iter().all(|v| v.is_finite()));
        assert_eq!(real0, tone_real(&mut rn(), 60.0, 0.5, 0.8, &TONE, 0));
        let real1 = tone_real(&mut rn(), 60.0, 0.5, 0.8, &TONE, 1);
        assert_ne!(real0, real1);
        assert_ne!(real1, tone_real(&mut rn(), 60.0, 0.5, 0.8, &TONE, 2));
        let notes: Vec<i32> = PROFILE.iter().map(|(m, _)| *m).collect();
        assert_eq!(notes, (REGISTER.0..=REGISTER.1).collect::<Vec<_>>());
        // the voice's Composed note is the real tone from the alto generator, at 48 kHz
        let mut noise = PpNoise::start();
        let mut alto = noise.1.alto.clone();
        let want = pp::to_48k(&tone_real(&mut alto, 60.0, 0.5, 0.8, &TONE, 1));
        let v = AltoSaxVoice;
        assert!(v.reworked());
        assert_eq!(v.note_real(&mut noise, 60.0, 0.8, 0.5, 1), want);
    }

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
