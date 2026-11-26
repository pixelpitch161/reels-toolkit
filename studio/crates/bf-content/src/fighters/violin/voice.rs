//! VIOLIN's voice: `pp_bridge.voice('VIOLIN', midi, vel, dur)`, which is `violin_lowpoly.bowed(midi_hz(m), dur,
//! vel, 'modern')`: the bowed tone CELLO's port already holds ([`crate::fighters::cello::voice::bowed`]) with
//! `TONE['modern']` (`tilt=.95, form=[(450, .8, .4), (3000, 1.3, .45)], vib=.006, vdel=.1, att=.03`). Made at
//! 44,100 Hz and resampled to 48,000 Hz. It draws one number a note from the violin module's generator, the same
//! stream the CELLO draws from.

use bf_sound::voice::pp;
use bf_sound::voice::real::{Correction, Fill, Rework};
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

/// The VIOLIN's added overtones: the harmonics its own note barely has, laid in at a real violin's level (dB under
/// the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (62, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (63, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (64, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (65, [-99.0, -99.0, -3.7, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (66, [-99.0, -99.0, -3.1, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (67, [-99.0, -99.0, -2.7, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (68, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (69, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (70, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (71, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (72, [-99.0, -3.4, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (73, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (74, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (75, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (76, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (77, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (78, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (79, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -21.7]),
    (80, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -21.6]),
    (81, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -30.7, -28.4]),
    (82, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -30.1, -27.7]),
    (83, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -29.9, -27.4]),
    (84, [-99.0, -99.0, -99.0, -99.0, -99.0, -16.5, -28.2, -25.7]),
    (85, [-99.0, -99.0, -99.0, -99.0, -99.0, -14.1, -28.3, -25.2]),
    (86, [-99.0, -99.0, -99.0, -99.0, -99.0, -12.3, -29.1, -24.8]),
    (87, [-99.0, -99.0, -99.0, -99.0, -29.1, -31.8, -35.0, -41.3]),
    (88, [-99.0, -99.0, -99.0, -99.0, -28.4, -32.1, -34.3, -40.7]),
];

/// The VIOLIN's overtone correction toward a real violin: gains in dB for its first eight harmonics, note by note
/// (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "VIOLIN"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (62, [0.0, -9.0, 4.1, -2.3, -8.8, -15.2, -6.0, -12.7]),
    (63, [0.0, -7.0, 5.7, -0.5, -8.6, -15.5, -5.3, -12.5]),
    (64, [0.0, -4.6, 7.2, 1.0, -8.6, -15.1, -4.6, -12.3]),
    (65, [0.0, 2.8, -0.2, 10.0, -6.7, 4.9, 4.6, -3.7]),
    (66, [0.0, 6.1, 0.1, 11.4, -8.0, 5.2, 6.4, -2.3]),
    (67, [0.0, 8.5, 0.6, 11.7, -9.2, 5.9, 8.0, -0.1]),
    (68, [0.0, 0.8, -3.7, -2.3, 1.8, -3.0, -8.8, 2.2]),
    (69, [0.0, 1.4, -3.9, -2.6, -0.1, -3.9, -9.6, 3.1]),
    (70, [0.0, 1.7, -4.2, -3.8, -1.8, -3.5, -7.6, 3.5]),
    (71, [0.0, 2.0, -4.8, -5.1, -3.0, -2.7, -6.4, 3.9]),
    (72, [0.0, 0.3, 5.3, 8.1, 8.1, 4.2, 5.6, 6.4]),
    (73, [0.0, 13.0, 1.8, 4.2, 4.2, 2.7, 7.7, 4.9]),
    (74, [0.0, 10.1, -2.1, 2.7, 1.1, 0.9, 8.8, 5.9]),
    (75, [0.0, 12.9, 4.3, 4.1, -1.1, 1.7, -4.2, 6.1]),
    (76, [0.0, 11.4, 2.4, 3.9, 0.3, 2.7, -3.3, 6.1]),
    (77, [0.0, 9.9, 0.7, 4.1, 2.8, 6.1, -1.8, 5.8]),
    (78, [0.0, -0.0, 0.4, -0.3, 4.6, -4.6, 0.9, 9.7]),
    (79, [0.0, -1.2, -1.9, -1.5, 4.1, -4.3, 0.9, 6.8]),
    (80, [0.0, -1.9, -4.5, -2.4, 3.6, -4.2, 0.5, 7.2]),
    (81, [0.0, 3.2, -12.9, -6.5, -13.3, -2.8, 6.9, 6.8]),
    (82, [0.1, 0.6, -13.4, -6.6, -12.0, -2.8, 7.5, 6.7]),
    (83, [0.2, -1.1, -12.5, -5.2, -9.7, -2.0, 6.9, 7.2]),
    (84, [-4.8, 0.0, -6.2, -10.4, -0.0, 1.3, 1.3, 2.0]),
    (85, [-3.0, 0.0, -3.0, -7.0, 2.3, 3.3, 3.3, 3.5]),
    (86, [-1.3, 0.0, 0.1, -3.7, 4.7, 4.3, 5.1, 5.4]),
    (87, [4.1, 0.0, 2.2, -4.5, 9.8, 9.6, 10.0, 10.5]),
    (88, [4.1, 0.0, 3.6, -3.1, 10.0, 9.9, 10.0, 10.2]),
];

/// The Composed arrangement's bowed VIOLIN (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added, and a slower start (80 ms to full level), nearer the real bow's swell
/// (which takes a third of a second and would blur fast lines). Its plucks stay its own.
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.08,
    stop: 0.0,
    keep_shape: false,
    tail: &[],
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

    fn rework(&self) -> Option<&'static Rework> {
        Some(&REWORK)
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
