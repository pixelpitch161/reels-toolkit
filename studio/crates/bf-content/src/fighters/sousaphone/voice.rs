//! SOUSAPHONE's instrument voice: `pp_bridge.voice('SOUSAPHONE', midi, vel, dur)`, and the tone its part plays.
//!
//! `pp_bridge` plays one note through `series/instrument-history/sousaphone_lowpoly.py`'s `synth`, which is
//! `sousa_tone(midi_hz(m), dur, v, blat=.25)`: a punchy marching bass. Up to 39 harmonics (those up to 8 kHz) on a
//! pitch that drops into the note, a soft clip for a little edge, the round low end of the fundamental, an accent
//! bump that settles to about 80 %, and a tongued "tah": a burst of band-limited noise in the first 30 ms. `blat`
//! (0 to 1) is the bell-blast accent: a bigger scoop and bump, a lip buzz on the upper partials and more top. The
//! note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! The tone calls nothing of the trumpet's or the tuba's modules (the module imports them for its model only).
//!
//! The "tah" draws from a generator of its own, seeded from the pitch and the length
//! (`default_rng(int(f * 7 + dur * 997) % 100003)`), so the shared noise stream is left where it was.
//!
//! The part (`sound.rs`) calls [`sousa_tone`] itself with its own `blat`, as the Python part does.

use bf_core::dmath::{self, PI};
use bf_core::py;
use bf_core::rng::Generator;
use bf_sound::dsp;
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::{PpNoise, Voice};

/// The SOUSAPHONE's register in `pp_bridge`'s roster (`('SOUSAPHONE', 'sousaphone_lowpoly', 'ball', (28, 58), ...)`).
pub const REGISTER: (i32, i32) = (28, 58);

/// `synth`'s `blat`: the accent of the note `pp_bridge.voice` plays.
pub const VOICE_BLAT: f64 = 0.25;

/// `sousa_tone`'s default `bright`.
const BRIGHT: f64 = 0.9;

/// SOUSAPHONE's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct SousaphoneVoice;

impl Voice for SousaphoneVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "SOUSAPHONE"
    }

    /// Above -0.14 s: the tone is `int((dur + .14) * SR)` samples; numpy refuses a negative count, and a note of
    /// no samples cannot be convolved (the "tah").
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.14) && pp::len(dur + 0.14) > 0
    }
}

/// `sousaphone_lowpoly.synth` for the one note `pp_bridge.voice` asks: `sousa_tone(midi_hz(m), dur, v, blat=.25)`,
/// at 44,100 Hz, before it is resampled. `midi` is a whole number.
///
/// # Panics
/// As [`sousa_tone`].
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    sousa_tone(pp::midi_hz(midi), dur, vel, VOICE_BLAT)
}

/// `sousa_tone(f, dur, v, blat=blat, bright=.9)`: the tone at `f` Hz held for `dur` seconds at velocity `v`,
/// `int((dur + .14) * SR)` samples at 44,100 Hz.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or so far below -0.14 s that the note has a negative number of
/// samples; a note of no samples at all (numpy cannot convolve an empty array); a pitch and length whose seed
/// `int(f * 7 + dur * 997)` is not a whole number an integer holds.
pub fn sousa_tone(f: f64, dur: f64, v: f64, blat: f64) -> Vec<f64> {
    let n = pp::len(dur + 0.14);
    let t = pp::tt(n);
    // fr = f * (1 - (.018 + .05 * blat) * np.exp(-t / (.02 + .02 * blat))): the drop into the note
    let (depth, fall) = (0.018 + 0.05 * blat, 0.02 + 0.02 * blat);
    let fr: Vec<f64> = t.iter().map(|&t| f * (1.0 - depth * dmath::exp(-t / fall))).collect();
    // ph = 2 * np.pi * np.cumsum(fr) / SR
    let ph: Vec<f64> = dsp::cumsum(&fr).iter().map(|&c| 2.0 * PI * c / SRF).collect();
    let mut x = vec![0.0; n];
    let mut tot = 0.0;
    let br = BRIGHT + 0.5 * blat;
    for k in 1..40 {
        let k = f64::from(k);
        if f * k > 8000.0 {
            break;
        }
        // a = k ** -.8 * math.exp(-k / (4.0 + 7.0 * br * v))
        let a = dmath::pow(k, -0.8) * dmath::exp(-k / (4.0 + 7.0 * br * v));
        let shift = 0.21 * k;
        for (x, &ph) in x.iter_mut().zip(&ph) {
            *x += a * dmath::sin(k * ph + shift);
        }
        tot += a;
    }
    let tot = tot + 1e-9;
    for x in &mut x {
        *x /= tot;
    }
    let mut buzz = vec![0.0; n];
    if blat != 0.0 {
        // lip buzz on the upper partials
        for k in 5..14 {
            let k = f64::from(k);
            for (b, &ph) in buzz.iter_mut().zip(&ph) {
                *b += dmath::sin(k * ph) / k;
            }
        }
        // buzz *= (1 + np.sin(2 * np.pi * 38 * t)) * .5 * blat * .55
        let rasp = 2.0 * PI * 38.0;
        for (b, &t) in buzz.iter_mut().zip(&t) {
            *b *= (1.0 + dmath::sin(rasp * t)) * 0.5 * blat * 0.55;
        }
    }
    // x = np.tanh((x + buzz) * (1.2 + 1.3 * blat)) / np.tanh(1.2 + 1.3 * blat): a little edge, more on a blat
    let drive = 1.2 + 1.3 * blat;
    let top = dmath::tanh(drive);
    // body = np.sin(2 * np.pi * np.cumsum(fr) / SR): the round low end; x = x + .38 * body
    let rel = if dur < 0.3 { 0.025 } else { 0.05 };
    let seed = py::imod_py(whole(f * 7.0 + dur * 997.0), 100_003) as u64;
    let mut rng = Generator::new(seed);
    // rng.standard_normal(n), twice: the first for the fast average, the second for the slow one
    let first: Vec<f64> = (0..n).map(|_| rng.standard_normal()).collect();
    let second: Vec<f64> = (0..n).map(|_| rng.standard_normal()).collect();
    let fast = dsp::convolve_same(&first, &[1.0 / 6.0; 6]);
    let slow = dsp::convolve_same(&second, &[1.0 / 40.0; 40]);
    let bump = 0.35 + 0.3 * blat;
    (0..n)
        .map(|i| {
            let t = t[i];
            let x = dmath::tanh((x[i] + buzz[i]) * drive) / top;
            let x = x + 0.38 * dmath::sin(ph[i]);
            // env = np.minimum(1, t / .012) ** 1.2 * np.where(t < dur, 1, np.exp(-(t - dur) / rel))
            let rise = if t / 0.012 < 1.0 { t / 0.012 } else { 1.0 };
            let hold = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / rel) };
            let env = dmath::pow(rise, 1.2) * hold;
            // env *= .8 + (.35 + .3 * blat) * np.exp(-t / .07): the accent bump, settling
            let env = env * (0.8 + bump * dmath::exp(-t / 0.07));
            // x = x + tah * .16 * (1 + blat) * np.exp(-t / .012): the tongued "tah"
            let tah = fast[i] - slow[i];
            let x = x + tah * 0.16 * (1.0 + blat) * dmath::exp(-t / 0.012);
            x * env * v * 0.8
        })
        .collect()
}

/// `int(x)` for the generator's seed.
///
/// # Panics
/// When `x` is not a number an `i64` holds (Python raises for NaN and infinity; a seed that large never comes from
/// a pitch and a length).
fn whole(x: f64) -> i64 {
    assert!(
        x.is_finite() && x.abs() < 9.0e18,
        "cannot convert float {x} to an integer seed"
    );
    x as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(x: &[f64]) -> f64 {
        x.iter().fold(0.0f64, |p, v| p.max(v.abs()))
    }

    #[test]
    fn the_note_has_its_length_and_dies_away() {
        // int((dur + .14) * 44100) samples
        assert_eq!(synth(34.0, 0.8, 0.12).len(), 11466);
        assert_eq!(synth(34.0, 0.8, 1.0).len(), 50274);
        let x = sousa_tone(110.25, 0.5, 0.9, 0.0);
        assert_eq!(x.len(), pp::len(0.64));
        // it starts from nothing (the attack is a power of t) and has died away by its end
        assert_eq!(x[0], 0.0);
        assert!(peak(&x[x.len() - 441..]) < 0.05, "{}", peak(&x[x.len() - 441..]));
        let level = peak(&x[4410..17640]);
        assert!((0.3..1.6).contains(&level), "{level}");
        assert!(x.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn a_blat_buzzes_and_bumps() {
        // the same pitch and length draw the same "tah", so the difference is the blat alone
        let (plain, blat) = (sousa_tone(98.0, 0.5, 0.9, 0.0), sousa_tone(98.0, 0.5, 0.9, 1.0));
        assert_eq!(plain.len(), blat.len());
        let apart = (2000..12000).fold(0.0f64, |d, i| d.max((blat[i] - plain[i]).abs()));
        assert!(apart > 0.1, "{apart}");
        // the accent bump is bigger on a blat: the first 50 ms are louder
        assert!(peak(&blat[..2205]) > peak(&plain[..2205]));
    }

    #[test]
    fn the_tah_is_seeded_by_pitch_and_length_and_the_shared_noise_stays() {
        let mut noise = PpNoise::start();
        let note = SousaphoneVoice.note(&mut noise, 40.0, 0.9, 0.25);
        let raw = synth(40.0, 0.9, 0.25);
        assert_eq!(note, pp::to_48k(&raw));
        assert_eq!((raw.len(), note.len()), (17199, 18720));
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        // the same note again is the same samples
        assert_eq!(raw, synth(40.0, 0.9, 0.25));
        assert_eq!(
            (SousaphoneVoice.register(), SousaphoneVoice.name()),
            (REGISTER, "SOUSAPHONE")
        );
        // what it can hold is what Python does not raise for
        assert!(SousaphoneVoice.can_hold(0.0) && SousaphoneVoice.can_hold(-0.1));
        assert!(
            !SousaphoneVoice.can_hold(-0.14),
            "a note of no samples cannot be convolved"
        );
        assert!(!SousaphoneVoice.can_hold(-0.2) && !SousaphoneVoice.can_hold(f64::INFINITY));
    }

    #[test]
    fn the_voice_is_the_tone_with_a_quarter_blat() {
        assert_eq!(synth(46.0, 0.6, 0.12), sousa_tone(pp::midi_hz(46.0), 0.12, 0.6, 0.25));
    }
}
