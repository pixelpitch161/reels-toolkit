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
        let plain: Vec<f64> = additive(f, DUR, 0.8, &grand).iter().map(|s| s * 0.5 * (0.4 + 0.8)).collect();
        assert_ne!(inst_note(36.0, 0.8), plain);
        let f = pp::midi_hz(40.0);
        let plain: Vec<f64> = additive(f, DUR, 0.8, &grand).iter().map(|s| s * 0.5 * (0.4 + 0.8)).collect();
        assert_eq!(inst_note(40.0, 0.8), plain);
        // a harmonic above SR / 2.2 is left out: at 8 kHz only two are summed, at 30 kHz none
        assert!(peak(&additive(8000.0, 0.1, 0.8, &grand)) > 0.1);
        assert!(additive(30_000.0, 0.1, 0.8, &grand).iter().all(|&s| s == 0.0));
    }
}
