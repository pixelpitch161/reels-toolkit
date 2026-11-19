//! `trumpet_lowpoly.brass`: the Pixel Pitch brass tone that three kit voices are made of. The TRUMPET's voice
//! plays it with its scene's tone (`trumpet_lowpoly.TONE`), the TUBA's (`tuba60_lowpoly.synth`) with a round
//! fundamental added and the HORN's (`horn60_lowpoly.synth`) twice, a hair apart, for a section. Each of those
//! callers, its tone table and what it adds belongs to its own fighter's voice.
//!
//! Not ported here, because no voice calls them: the module's `bend`, `fingering` and `brass_fx` draw and animate
//! the instrument's model (the Studio reads its models from their exported faces), and the TRUMPET fighter's own
//! valve fingering is `kit/fighters/trumpet.py`'s, in its part.

use bf_core::dmath::{self, PI};

use super::{SRF, clip01, len, min1, tt};
use crate::dsp;

/// `brass`'s tone arguments with their Python defaults ([`Brass::default`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Brass {
    /// `bright=1.`: how far the bright set of harmonic weights reaches up.
    pub bright: f64,
    /// `vib=.004`: the vibrato's depth, a fraction of the frequency.
    pub vib: f64,
    /// `scoop=.03`: how far under the pitch the note starts, a fraction of the frequency.
    pub scoop: f64,
    /// `rough=0.`: a growl an octave down (0 is none: Python's `if rough:`).
    pub rough: f64,
    /// `tilt=.75`: the roll-off of the harmonics, `1 / k ** tilt`.
    pub tilt: f64,
}

impl Default for Brass {
    fn default() -> Brass {
        Brass {
            bright: 1.0,
            vib: 0.004,
            scoop: 0.03,
            rough: 0.0,
            tilt: 0.75,
        }
    }
}

/// `brass(f, dur, vel, bright, vib, scoop, rough, tilt)`: `int((dur + .12) * SR)` samples at 44,100 Hz of a tone at
/// `f` Hz held for `dur` seconds at velocity `vel`: up to 29 harmonics (those up to 7 kHz) summed with a dark and a
/// bright set of weights, blended by the note's own level (a loud note is brighter); a scoop up into the pitch, a
/// vibrato from 0.18 s, a 35 ms attack and a 30 ms release after the held length.
///
/// A fundamental above 7 kHz leaves no harmonic: Python divides zero by zero and the note is NaN, here too.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or so far below -0.12 s that the note has a negative number of
/// samples.
pub fn brass(f: f64, dur: f64, vel: f64, tone: &Brass) -> Vec<f64> {
    let n = len(dur + 0.12);
    let t = tt(n);
    // env = minimum(1, t / .035) ** 1.5 * where(t < dur, 1, exp(-(t - dur) / .03)) * (.88 + .12 * exp(-t * 3))
    let env: Vec<f64> = t
        .iter()
        .map(|&t| {
            let hold = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.03) };
            dmath::pow(min1(t / 0.035), 1.5) * hold * (0.88 + 0.12 * dmath::exp(-t * 3.0))
        })
        .collect();
    // fr = f * (1 - scoop * exp(-t / .025)) * (1 + vib * sin(2 pi 5.2 t) * clip((t - .18) / .2, 0, 1))
    let wobble = 2.0 * PI * 5.2;
    let fr: Vec<f64> = t
        .iter()
        .map(|&t| {
            f * (1.0 - tone.scoop * dmath::exp(-t / 0.025))
                * (1.0 + tone.vib * dmath::sin(wobble * t) * clip01((t - 0.18) / 0.2))
        })
        .collect();
    // ph = 2 pi cumsum(fr) / SR
    let ph: Vec<f64> = dsp::cumsum(&fr).iter().map(|&c| 2.0 * PI * c / SRF).collect();
    let (mut dark, mut brt) = (vec![0.0; n], vec![0.0; n]);
    let (mut sd, mut sb) = (0.0, 0.0);
    for k in 1..30 {
        let k = f64::from(k);
        if f * k > 7000.0 {
            break;
        }
        // a1 = math.exp(-k / 2.5) / k ** tilt; a2 = math.exp(-k / (5 + 6 * bright)) / k ** tilt
        let a1 = dmath::exp(-k / 2.5) / dmath::pow(k, tone.tilt);
        let a2 = dmath::exp(-k / (5.0 + 6.0 * tone.bright)) / dmath::pow(k, tone.tilt);
        for i in 0..n {
            let s = dmath::sin(k * ph[i]);
            dark[i] += a1 * s;
            brt[i] += a2 * s;
        }
        sd += a1;
        sb += a2;
    }
    (0..n)
        .map(|i| {
            // mb = clip(env * vel, 0, 1) ** 1.3: how much of the bright set
            let mb = dmath::pow(clip01(env[i] * vel), 1.3);
            let mut x = (dark[i] / sd * (1.0 - mb) + brt[i] / sb * mb) * env[i];
            if tone.rough != 0.0 {
                // x = x + rough * sin(ph * .5) * env * .3
                x += tone.rough * dmath::sin(ph[i] * 0.5) * env[i] * 0.3;
            }
            x * vel
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(x: &[f64]) -> f64 {
        x.iter().fold(0.0f64, |p, v| p.max(v.abs()))
    }

    #[test]
    fn the_tone_has_its_length_its_pitch_and_its_shape() {
        let tone = Brass::default();
        // int((dur + .12) * 44100) samples.
        assert_eq!(brass(440.0, 0.12, 0.8, &tone).len(), 10584);
        assert_eq!(brass(440.0, 0.0, 0.8, &tone).len(), 5292);
        // 441 Hz repeats every 100 samples between the scoop and the vibrato, and not every 50.
        let x = brass(441.0, 0.5, 0.9, &tone);
        assert_eq!(x[0], 0.0);
        let level = peak(&x[4410..17640]);
        assert!((0.3..1.2).contains(&level), "{level}");
        let apart = |lag: usize| (5292..7400).fold(0.0f64, |d, i| d.max((x[i + lag] - x[i]).abs()));
        assert!(apart(100) < 0.03 * level, "{} of {level}", apart(100));
        assert!(apart(50) > 0.3 * level, "{} of {level}", apart(50));
        assert!(
            peak(&x[x.len() - 441..]) < 0.05 * level,
            "{}",
            peak(&x[x.len() - 441..])
        );
    }

    #[test]
    fn rough_adds_a_growl_and_a_high_note_has_no_harmonic() {
        let plain = brass(220.0, 0.3, 0.9, &Brass::default());
        let rough = brass(
            220.0,
            0.3,
            0.9,
            &Brass {
                rough: 0.25,
                ..Brass::default()
            },
        );
        assert_eq!(plain.len(), rough.len());
        assert!(plain.iter().zip(&rough).any(|(a, b)| (a - b).abs() > 1e-3));
        // 7,000 Hz is the last harmonic that counts; above it there is none and the note is 0 / 0.
        assert!(brass(7000.0, 0.1, 0.8, &Brass::default()).iter().all(|v| v.is_finite()));
        assert!(brass(7000.5, 0.1, 0.8, &Brass::default()).iter().all(|v| v.is_nan()));
    }

    #[test]
    #[should_panic(expected = "negative dimensions are not allowed")]
    fn a_note_cannot_end_long_before_it_starts() {
        brass(440.0, -0.2, 0.8, &Brass::default());
    }
}
