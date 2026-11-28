//! SOPRANO SAX's voice (a from-scratch fighter, 2026-11-16): the shared alto-sax reed block (`bf_sound::voice::pp::alto`,
//! ALTO SAX's) with a soprano's tone: brighter and more nasal (a higher formant with more boost, stronger odd
//! harmonics), a quicker attack and a faster, slightly wider vibrato, so it buzzes. Made at 44,100 Hz and resampled to
//! 48,000 Hz, like the other Pixel Pitch voices. It draws only the alto generator, as ALTO SAX does.

use bf_sound::voice::engine2::reed::ReedParts;
use bf_sound::voice::engine2::space::{Home, Space};
use bf_sound::voice::engine2::{Instrument, Parts};
use bf_sound::voice::pp::alto::{AltoTone, alto_tone, alto_tone_real};
use bf_sound::voice::pp::{self};
use bf_sound::voice::real::{self, Profile, Shape};
use bf_sound::voice::{PpNoise, Voice};

/// SOPRANO SAX's register (B-flat soprano, played here from B-flat 3 to D6).
pub const REGISTER: (i32, i32) = (58, 86);

/// A soprano's tone: brighter, more nasal and buzzier than ALTO SAX's.
pub const TONE: AltoTone = AltoTone {
    bright: 0.9,
    vib: 0.006,
    vrate: 5.8,
    odd: 0.22,
    scoop: 0.025,
    att: 0.02,
    formant: 1650.0,
    fboost: 1.25,
    breath: 0.035,
    growl: 0.0,
    slap: 0.0,
    gl: None,
};

/// Seconds of tail after the held length.
pub const TAIL: f64 = 0.14;

/// A real soprano sax's first eight overtones, dB under the strongest (FluidR3, `bf-harness/tests/reference/fluidr3.json`,
/// written by `cargo run --release -p bf-harness --example references -- profile "soprano sax" 58 86`).
#[rustfmt::skip]
pub const PROFILE: &Profile = &[
    (58, [-3.7, 0.0, -10.4, -3.6, -13.5, -5.9, -10.2, -12.7]),
    (59, [-3.7, 0.0, -10.5, -3.6, -13.6, -6.0, -10.1, -12.4]),
    (60, [0.0, -2.1, -14.0, -5.5, -13.0, -6.9, -7.2, -10.8]),
    (61, [0.0, -2.2, -13.9, -5.5, -13.1, -7.0, -7.0, -10.8]),
    (62, [0.0, -2.5, -14.5, -3.7, -7.7, -3.3, -7.7, -12.0]),
    (63, [0.0, -2.5, -14.3, -3.7, -7.8, -3.4, -7.5, -11.7]),
    (64, [0.0, -3.2, -9.0, -4.4, -3.0, -5.5, -12.8, -11.6]),
    (65, [0.0, -3.3, -8.8, -4.6, -3.1, -5.7, -13.7, -11.7]),
    (66, [-15.8, 0.0, -24.0, -10.7, -6.1, -2.7, -9.2, -6.3]),
    (67, [-15.5, 0.0, -23.7, -10.6, -6.2, -2.9, -9.3, -6.2]),
    (68, [-4.3, 0.0, -7.3, -1.5, -0.9, -5.7, -9.3, -24.5]),
    (69, [-4.3, 0.0, -7.4, -1.5, -0.9, -5.9, -9.2, -24.1]),
    (70, [-0.7, -2.8, 0.0, -15.7, -1.4, -15.2, -17.9, -30.4]),
    (71, [-0.7, -2.7, 0.0, -15.7, -1.4, -15.2, -18.1, -30.4]),
    (72, [-0.3, 0.0, -14.4, -13.1, -4.8, -16.9, -29.8, -29.9]),
    (73, [-0.3, 0.0, -14.3, -12.9, -4.8, -16.7, -30.2, -29.9]),
    (74, [-4.0, 0.0, -5.8, -7.0, -15.6, -35.6, -42.3, -42.2]),
    (75, [-3.9, 0.0, -5.8, -7.0, -15.7, -36.2, -42.2, -42.7]),
    (76, [-3.8, 0.0, -5.8, -7.0, -15.8, -36.8, -42.2, -43.3]),
    (77, [-3.5, -2.0, 0.0, -3.2, -45.3, -31.7, -43.9, -45.5]),
    (78, [-3.5, -2.0, 0.0, -3.2, -45.8, -31.9, -44.1, -45.8]),
    (79, [-4.2, 0.0, -0.6, -11.3, -28.5, -39.5, -50.0, -50.0]),
    (80, [-4.4, 0.0, -0.8, -11.6, -28.6, -40.0, -50.0, -50.0]),
    (81, [0.0, -2.3, -14.5, -23.6, -34.1, -43.9, -50.0, -50.0]),
    (82, [0.0, -2.3, -14.4, -24.0, -33.2, -43.1, -50.0, -50.0]),
    (83, [0.0, -2.2, -14.2, -24.1, -32.4, -42.3, -50.0, -50.0]),
    (84, [0.0, -1.5, -18.1, -37.9, -41.2, -44.0, -50.0, -49.3]),
    (85, [0.0, -1.5, -17.7, -37.3, -41.0, -44.3, -50.0, -50.0]),
    (86, [0.0, -1.6, -17.6, -37.0, -41.1, -44.8, -50.0, -50.0]),
];

/// The Composed arrangement's soprano sax (sound upgrade, stage 3, S1): the real one's overtones at the note's pitch, its
/// time to speak and to stop (FluidR3: 22 ms to speak, 63 ms to a tenth after the release), and a reed buzz.
pub fn shape(f: f64) -> Shape {
    Shape {
        levels: real::levels(PROFILE, real::midi_of(f)),
        speak: 0.03,
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

/// SOPRANO SAX's voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct SopranoSaxVoice;

/// The Composed arrangement's SOPRANO SAX (Voice Engine 2): built from how it works, on the reed engine (Aedan's pick
/// Ss2, the jazz soprano: an open, bright mouthpiece, played with a quick vibrato), with every technique it has, in a
/// light room of its own (`docs/studio/voice-engine-2-instruments.md`, "The reeds"). Classic keeps the note below.
pub static ENGINE2: Instrument = Instrument::built(
    "SOPRANO SAX",
    Parts::Reed(ReedParts::SS2),
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

impl Voice for SopranoSaxVoice {
    fn engine2(&self) -> Option<&'static Instrument> {
        Some(&ENGINE2)
    }

    fn note(&self, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
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
        "SOPRANO SAX"
    }

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
        let today = alto_tone(&mut rn(), pp::midi_hz(70.0), 0.5, 0.8, &TONE);
        let real0 = tone_real(&mut rn(), 70.0, 0.5, 0.8, &TONE, 0);
        assert_eq!(real0.len(), today.len());
        assert_ne!(real0, today);
        assert!(real0.iter().all(|v| v.is_finite()));
        assert_eq!(real0, tone_real(&mut rn(), 70.0, 0.5, 0.8, &TONE, 0));
        let real1 = tone_real(&mut rn(), 70.0, 0.5, 0.8, &TONE, 1);
        assert_ne!(real0, real1);
        assert_ne!(real1, tone_real(&mut rn(), 70.0, 0.5, 0.8, &TONE, 2));
        let notes: Vec<i32> = PROFILE.iter().map(|(m, _)| *m).collect();
        assert_eq!(notes, (REGISTER.0..=REGISTER.1).collect::<Vec<_>>());
        // the voice's Composed note is the real tone from the alto generator, at 48 kHz
        let mut noise = PpNoise::start();
        let mut alto = noise.1.alto.clone();
        let want = pp::to_48k(&tone_real(&mut alto, 70.0, 0.5, 0.8, &TONE, 1));
        let v = SopranoSaxVoice;
        assert!(v.reworked());
        assert_eq!(v.note_real(&mut noise, 70.0, 0.8, 0.5, 1), want);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)] // the two TONE tables checked against each other
    fn a_note_is_finite_and_brighter_than_the_altos() {
        let mut noise = PpNoise::start();
        let x = SopranoSaxVoice.note(&mut noise, 74.0, 0.8, 0.2);
        assert!(!x.is_empty() && x.iter().all(|v| v.is_finite()));
        assert!(TONE.bright > crate::fighters::alto_sax::voice::TONE.bright);
        assert_eq!(
            (SopranoSaxVoice.register(), SopranoSaxVoice.name()),
            (REGISTER, "SOPRANO SAX")
        );
    }
}
