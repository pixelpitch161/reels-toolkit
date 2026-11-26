//! PIANO's instrument voice: `pp_bridge.voice('PIANO', midi, vel, dur)`.
//!
//! `pp_bridge` plays the note through `series/instrument-history/piano_lowpoly.py`'s `inst_note('grand', midi, vel,
//! 0.0, 3)`: the modern grand. That is `additive`, a sum of up to 14 slightly stretched harmonics (a string's
//! inharmonicity, `B`), each with its own level (a loud note has more of the upper ones) and its own decay (the upper
//! ones die sooner, a high note sooner than a low one), each harmonic doubled by a copy a hair sharp so the tone
//! beats, under a 2 ms attack. A note below MIDI 40 gets a second, quieter tone an octave up. The note is always 2.6 s
//! long (the held length `dur` is not read: a piano string rings until it has died away), made at 44,100 Hz and
//! resampled to 48,000 Hz.
//!
//! The voice draws no noise: the shared stream is left where it was.
//!
//! Not ported, because `pp_bridge` never asks for them: the six older keyboards of `inst_note` (the monochord to the
//! square piano), the 5 s note of the series' last chord (`nt >= 12 * BEAT`; `pp_bridge` passes `nt = 0.0`) and
//! `additive`'s vibrato (`vibr`, the clavichord's).

use bf_core::dmath::{self, PI};
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::real::{Correction, Fill, Rework};
use bf_sound::voice::{PpNoise, Voice};

/// PIANO's register in `pp_bridge`'s roster (`('PIANO', 'piano_lowpoly', 'grand', (48, 84), ...)`).
pub const REGISTER: (i32, i32) = (48, 84);

/// The grand's note length in seconds (`dur = 5.0 if nt >= 12 * BEAT else 2.6`, with `nt = 0.0`).
const DUR: f64 = 2.6;

/// `additive`'s arguments after `f`, `dur` and `vel`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Additive {
    /// `harm`: the most harmonics summed.
    pub harm: u32,
    /// `B`: the string's inharmonicity (harmonic `k` sounds at `f * k * sqrt(1 + B * k * k)`).
    pub b: f64,
    /// `tilt`: the roll-off of the harmonics, `1 / k ** tilt`.
    pub tilt: f64,
    /// `d1`: the first harmonic's decay rate, per second.
    pub d1: f64,
    /// `attack`: the attack, in seconds.
    pub attack: f64,
}

impl Default for Additive {
    /// Python's defaults: `harm=12, B=0.0004, tilt=1.2, d1=1.2, attack=0.004` (and `vibr=0`).
    fn default() -> Additive {
        Additive {
            harm: 12,
            b: 0.0004,
            tilt: 1.2,
            d1: 1.2,
            attack: 0.004,
        }
    }
}

/// `additive(f, dur, vel, harm, B, tilt, d1, attack)` of `piano_lowpoly` without vibrato: `int(dur * SR)` samples at
/// 44,100 Hz.
///
/// # Panics
/// Where Python raises: a `dur` that is not a number or below zero.
pub fn additive(f: f64, dur: f64, vel: f64, a: &Additive) -> Vec<f64> {
    let n = pp::len(dur);
    let tt = pp::tt(n);
    let mut sig = vec![0.0; n];
    for k in 1..=a.harm {
        let k = f64::from(k);
        let fk = f * k * (1.0 + a.b * k * k).sqrt();
        if fk > SRF / 2.2 {
            break;
        }
        let amp = dmath::pow(vel, 0.12 * (k - 1.0)) / dmath::pow(k, a.tilt);
        let dk = a.d1 * (1.0 + 0.45 * (k - 1.0)) * (1.0 + f / 900.0);
        // ph = 2 * math.pi * fk * tt
        let w = 2.0 * PI * fk;
        for (s, &t) in sig.iter_mut().zip(&tt) {
            let ph = w * t;
            // sig += amp * np.exp(-dk * tt) * (np.sin(ph) + 0.6 * np.sin(ph * 1.0007 + 1.3))
            *s += amp * dmath::exp(-dk * t) * (dmath::sin(ph) + 0.6 * dmath::sin(ph * 1.0007 + 1.3));
        }
    }
    // sig * np.minimum(1, tt / attack)
    sig.iter()
        .zip(&tt)
        .map(|(&s, &t)| {
            let rise = t / a.attack;
            s * if rise < 1.0 { rise } else { 1.0 }
        })
        .collect()
}

/// `inst_note('grand', m, v, 0.0, 3)`: the grand's note at 44,100 Hz, before it is resampled. `m` is a whole number.
pub fn inst_note(m: f64, v: f64) -> Vec<f64> {
    let f = pp::midi_hz(m);
    let grand = Additive {
        harm: 14,
        tilt: 1.1,
        d1: 0.7,
        attack: 0.002,
        ..Additive::default()
    };
    // x = additive(f, dur, v, harm=14, tilt=1.1, d1=.7, attack=.002) * .5 * (.4 + v)
    let mut x: Vec<f64> = additive(f, DUR, v, &grand)
        .into_iter()
        .map(|s| s * 0.5 * (0.4 + v))
        .collect();
    if m < 40.0 {
        // x = x + additive(f * 2, dur, v, harm=8, tilt=1.2, d1=.8)[:len(x)] * .2
        let octave = Additive {
            harm: 8,
            tilt: 1.2,
            d1: 0.8,
            ..Additive::default()
        };
        let up = additive(f * 2.0, DUR, v, &octave);
        for (s, u) in x.iter_mut().zip(&up) {
            *s += u * 0.2;
        }
    }
    x
}

/// The PIANO's added overtones: the harmonics its own note barely has, laid in at a real piano's level (dB
/// under the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (48, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (49, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -16.2, -99.0]),
    (50, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -16.3, -99.0]),
    (51, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -16.5, -17.5]),
    (52, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -16.4, -17.7]),
    (53, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -16.4, -17.9]),
    (54, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -16.3, -18.1]),
    (55, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -14.3, -99.0]),
    (56, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -14.2, -99.0]),
    (57, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -14.2, -99.0]),
    (58, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -14.2, -99.0]),
    (59, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (60, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (61, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (62, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (63, [-99.0, -99.0, -99.0, -99.0, -10.4, -99.0, -99.0, -99.0]),
    (64, [-99.0, -99.0, -99.0, -99.0, -10.5, -99.0, -99.0, -99.0]),
    (65, [-99.0, -99.0, -99.0, -99.0, -10.5, -99.0, -99.0, -99.0]),
    (66, [-99.0, -99.0, -99.0, -99.0, -10.5, -99.0, -99.0, -99.0]),
    (67, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (68, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (69, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (70, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (71, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (72, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (73, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (74, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (75, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (76, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (77, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (78, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (79, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (80, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (81, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (82, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (83, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (84, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
];

/// The PIANO's overtone correction toward a real piano: gains in dB for its first eight harmonics, note by
/// note (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "PIANO"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (48, [-3.7, 7.2, 0.9, 5.3, 3.0, 8.1, 12.1, 3.6]),
    (49, [-3.8, 7.2, 0.9, 5.3, 3.2, 8.8, -3.9, 3.8]),
    (50, [-3.8, 7.3, 1.0, 5.3, 3.4, 9.2, -3.9, 3.5]),
    (51, [0.0, 8.3, -4.8, -0.6, 2.5, -0.5, 0.1, 0.1]),
    (52, [0.0, 8.2, -4.8, -0.5, 2.9, -0.1, 0.1, 0.1]),
    (53, [0.0, 8.1, -4.8, -0.5, 3.3, 0.2, 0.1, 0.1]),
    (54, [0.0, 8.0, -4.8, -0.3, 3.8, 0.4, 0.0, 0.1]),
    (55, [0.0, 3.4, -2.8, -0.3, 7.1, 2.7, 0.5, 13.8]),
    (56, [0.0, 3.4, -2.8, 0.1, 7.4, 2.9, 0.5, 13.0]),
    (57, [0.0, 3.5, -2.7, 0.5, 7.8, 3.0, 0.4, 12.1]),
    (58, [0.0, 3.6, -2.7, 0.9, 8.1, 2.9, 0.3, 11.0]),
    (59, [0.0, 2.9, -2.2, 8.3, 2.7, 11.5, 13.8, 13.2]),
    (60, [0.0, 2.8, -2.1, 8.6, 2.7, 10.8, 12.8, 11.9]),
    (61, [0.0, 2.7, -2.1, 8.8, 2.4, 9.8, 11.6, 10.6]),
    (62, [0.0, 2.7, -2.0, 9.1, 2.0, 8.5, 10.3, 9.3]),
    (63, [0.0, 4.7, -1.4, 6.2, 0.6, 11.4, -8.0, 12.2]),
    (64, [0.0, 4.6, -1.0, 5.7, 0.5, 10.8, -7.6, 11.2]),
    (65, [0.0, 4.5, -0.5, 5.0, 0.5, 10.4, -7.2, 10.6]),
    (66, [0.0, 4.4, -0.1, 4.2, 0.5, 10.1, -6.9, 10.1]),
    (67, [0.0, -11.9, -1.4, 1.7, -2.3, 6.5, -1.9, -6.3]),
    (68, [0.0, -11.9, -1.2, 1.8, -4.0, 4.6, -3.1, -7.3]),
    (69, [0.0, -11.9, -1.1, 1.7, -5.7, 2.8, -4.4, -8.4]),
    (70, [0.0, -11.9, -1.2, 1.2, -7.6, 0.8, -5.8, -9.5]),
    (71, [0.0, 3.3, 9.5, 1.4, 9.8, 5.2, -3.3, 8.4]),
    (72, [0.0, 3.3, 9.8, 0.0, 9.3, 4.5, -4.5, 8.6]),
    (73, [0.0, 3.3, 9.9, -1.4, 8.8, 3.7, -5.2, 9.1]),
    (74, [0.0, 3.4, 9.9, -2.7, 8.2, 2.5, -5.9, 9.7]),
    (75, [0.0, 5.1, -3.0, 8.2, 9.3, 2.9, -18.2, -14.4]),
    (76, [0.0, 5.5, -3.3, 8.3, 9.2, 2.6, -18.4, -13.9]),
    (77, [0.0, 5.9, -3.6, 8.6, 9.2, 2.4, -18.7, -13.8]),
    (78, [0.0, 6.2, -3.9, 8.9, 9.2, 2.5, -19.1, -13.9]),
    (79, [0.0, 6.6, -7.6, 5.5, 3.1, -15.1, -20.9, -19.2]),
    (80, [0.0, 6.9, -8.4, 6.4, 2.9, -14.9, -20.9, -19.4]),
    (81, [0.0, 6.8, -9.2, 7.0, 2.8, -15.1, -21.6, -19.7]),
    (82, [0.0, 6.1, -9.6, 7.2, 2.5, -16.0, -23.5, -20.2]),
    (83, [0.0, 5.0, -10.1, 6.9, 2.1, -17.4, -24.4, -20.8]),
    (84, [0.0, 3.4, -10.5, 6.2, 1.5, -19.4, -25.0, -21.3]),
];

/// The Composed arrangement's PIANO (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added, and the damper's quicker stop (a 70 ms fade after its length), nearer the
/// real one's; it keeps its own strike's snap.
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.0,
    stop: 0.07,
    keep_shape: true,
};

/// PIANO's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct PianoVoice;

impl Voice for PianoVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, _dur: f64) -> Vec<f64> {
        pp::to_48k(&inst_note(midi, vel))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn rework(&self) -> Option<&'static Rework> {
        Some(&REWORK)
    }

    fn name(&self) -> &str {
        "PIANO"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(x: &[f64]) -> f64 {
        x.iter().fold(0.0f64, |p, v| p.max(v.abs()))
    }

    #[test]
    fn the_note_is_always_2_6_seconds_and_dies_away() {
        // int(2.6 * 44100) samples, whatever the held length; 124,800 at 48 kHz.
        let x = inst_note(60.0, 0.8);
        assert_eq!(x.len(), 114_660);
        let mut noise = PpNoise::start();
        for dur in [0.0, 0.12, 5.0, -3.0] {
            assert_eq!(PianoVoice.note(&mut noise, 60.0, 0.8, dur), pp::to_48k(&x));
        }
        assert_eq!(pp::to_48k(&x).len(), 124_800);
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert_eq!((PianoVoice.register(), PianoVoice.name()), (REGISTER, "PIANO"));
        // it starts from nothing (the attack), is loudest early, and has faded by the end
        assert_eq!(x[0], 0.0);
        assert!(peak(&x[..4410]) > 4.0 * peak(&x[x.len() - 4410..]));
    }

    #[test]
    fn a_low_note_has_its_octave_and_a_high_note_fewer_harmonics() {
        // below MIDI 40 the octave tone is added: the note is not the plain tone
        let f = pp::midi_hz(36.0);
        let grand = Additive {
            harm: 14,
            tilt: 1.1,
            d1: 0.7,
            attack: 0.002,
            ..Additive::default()
        };
        let plain: Vec<f64> = additive(f, DUR, 0.8, &grand)
            .iter()
            .map(|s| s * 0.5 * (0.4 + 0.8))
            .collect();
        assert_ne!(inst_note(36.0, 0.8), plain);
        let f = pp::midi_hz(40.0);
        let plain: Vec<f64> = additive(f, DUR, 0.8, &grand)
            .iter()
            .map(|s| s * 0.5 * (0.4 + 0.8))
            .collect();
        assert_eq!(inst_note(40.0, 0.8), plain);
        // a harmonic above SR / 2.2 is left out: at 8 kHz only two are summed, at 30 kHz none
        assert!(peak(&additive(8000.0, 0.1, 0.8, &grand)) > 0.1);
        assert!(additive(30_000.0, 0.1, 0.8, &grand).iter().all(|&s| s == 0.0));
    }
}
