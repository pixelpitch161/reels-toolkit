//! OBOE's instrument voice: `pp_bridge.voice('OBOE', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/oboe_lowpoly.py`'s `synth`, which is
//! `double_reed(_midi_hz(m), dur, v)` with every other argument at its default: the nasal double reed. A conical
//! bore sounds every harmonic; the oboe's colour is its formants (a strong band round 1.2 kHz, a second near 3 kHz)
//! over a weak fundamental in the low register. A tongued attack with a short buzz of reed noise, a vibrato that
//! blooms after the attack, a quick release. The pitch may be fractional. The note is made at 44,100 Hz and
//! resampled to 48,000 Hz.
//!
//! The reed noise is the note's own: a generator seeded from the pitch (`int(f * 10) % 100003`), so the voice draws
//! nothing from the series' shared noise stream.

use bf_core::dmath::{self, PI};
use bf_core::py::{py_max, py_min};
use bf_core::rng::Generator;
use bf_sound::dsp::{self, FilterKind};
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::{PpNoise, Voice};

/// OBOE's register in `pp_bridge`'s roster (`('OBOE', 'oboe_lowpoly', 'ball', (58, 91), 0, 1.0)`).
pub const REGISTER: (i32, i32) = (58, 91);

/// `FORMANTS`: `(centre Hz, width Hz, gain)`.
const FORMANTS: [(f64, f64, f64); 3] = [(1200.0, 420.0, 1.9), (2950.0, 650.0, 0.95), (4400.0, 900.0, 0.25)];
/// `double_reed`'s default `bright`.
const BRIGHT: f64 = 0.6;
/// `double_reed`'s default `vib`: the vibrato's depth, as a fraction of the frequency.
const VIB: f64 = 0.0045;
/// `double_reed`'s default `vrate`: the vibrato's rate, in Hz.
const VRATE: f64 = 5.3;
/// `double_reed`'s default `att`: the attack, in seconds.
const ATT: f64 = 0.022;
/// `double_reed`'s default `buzz`: the level of the reed noise on the attack.
const BUZZ: f64 = 0.06;
/// `double_reed`'s default `breath`: the level of the reed noise under the whole note.
const BREATH: f64 = 0.012;

/// OBOE's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct OboeVoice;

impl Voice for OboeVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&double_reed(pp::midi_hz(midi), dur, vel))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "OBOE"
    }

    /// Down to -0.12 s: the tone is `int((dur + .12) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.12)
    }
}

/// `double_reed(f, dur, v)`: the tone at `f` Hz held for `dur` seconds at velocity `v`, `int((dur + .12) * SR)`
/// samples at 44,100 Hz.
///
/// # Panics
/// Where Python raises: `dur` is not a number or below -0.12 s (a negative number of samples), or `f` is not a
/// number a seed can be made of.
pub fn double_reed(f: f64, dur: f64, v: f64) -> Vec<f64> {
    let n = pp::len(dur + 0.12);
    let t = pp::tt(n);
    // fr = f * (1 - .012 * exp(-t / .03)) * (1 + vib * sin(2 pi vrate t) * clip((t - .16) / .22, 0, 1))
    let w = 2.0 * PI * VRATE;
    let fr: Vec<f64> = t
        .iter()
        .map(|&t| {
            let bloom = ((t - 0.16) / 0.22).clamp(0.0, 1.0);
            f * (1.0 - 0.012 * dmath::exp(-t / 0.03)) * (1.0 + VIB * dmath::sin(w * t) * bloom)
        })
        .collect();
    let ph: Vec<f64> = dsp::cumsum(&fr).iter().map(|&c| 2.0 * PI * c / SRF).collect();
    let mut x = vec![0.0; n];
    let mut tot = 0.0;
    for k in 1..48 {
        let kf = f64::from(k);
        let fk = f * kf;
        if fk > 9500.0 {
            break;
        }
        // a = k ** -.45 * (.28 + sum(g * math.exp(-((fk - c) / w) ** 2) for c, w, g in FORMANTS))
        let mut formants = 0.0;
        for (c, w, g) in FORMANTS {
            formants += g * dmath::exp(-dmath::pow((fk - c) / w, 2.0));
        }
        let mut a = dmath::pow(kf, -0.45) * (0.28 + formants);
        // louder = brighter
        a *= dmath::exp(-fk / (2600.0 + 4200.0 * BRIGHT * v));
        if k == 1 {
            // a weak fundamental down low
            a *= py_min(0.9, py_max(0.4, (f - 200.0) / 400.0));
        }
        let shift = 0.7 * kf;
        for (o, &p) in x.iter_mut().zip(&ph) {
            *o += a * dmath::sin(kf * p + shift);
        }
        tot += a;
    }
    let level = tot + 1e-9;
    // deterministic noise per pitch: np.random.default_rng(int(f * 10) % 100003).standard_normal(n)
    let seed = f * 10.0;
    assert!(seed.is_finite(), "cannot convert float {seed} to integer");
    let seed = (seed as i64).rem_euclid(100_003) as u64;
    let mut rng = Generator::new(seed);
    let nz: Vec<f64> = (0..n).map(|_| rng.standard_normal()).collect();
    let nb = dsp::sosfilt(&dsp::butter_sos(FilterKind::Bp, &[1800.0, 4200.0], 2, SRF), &nz);
    (0..n)
        .map(|i| {
            let t = t[i];
            // env = minimum(1, t / att) ** 1.2 * where(t < dur, 1, exp(-(t - dur) / .035)); env *= 1 + .12 * exp(-t / .05)
            let rise = if t / ATT < 1.0 { t / ATT } else { 1.0 };
            let hold = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.035) };
            let env = dmath::pow(rise, 1.2) * hold * (1.0 + 0.12 * dmath::exp(-t / 0.05));
            // x /= tot + 1e-9; x += nb * (buzz * exp(-t / .03) + breath) * .6; return x * env * v
            let tone = x[i] / level + nb[i] * (BUZZ * dmath::exp(-t / 0.03) + BREATH) * 0.6;
            tone * env * v
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tone_has_its_length_its_pitch_and_its_own_noise() {
        let x = double_reed(440.0, 0.5, 1.0);
        // int((0.5 + .12) * 44100)
        assert_eq!(x.len(), 27342);
        // it starts from silence (the attack) and has died away by the end of its release (exp(-0.11 / 0.035) of it)
        assert_eq!(x[0], 0.0);
        let end = x[x.len() - 441..].iter().fold(0.0f64, |p, v| p.max(v.abs()));
        assert!(end < 0.05, "{end}");
        // the same pitch is the same note (its noise is seeded from the pitch), whatever came before
        assert_eq!(double_reed(440.0, 0.5, 1.0), x);
        // every harmonic sounds: the tone is not a sine (its level is well under a sine's at this velocity)
        let peak = x[4410..17640].iter().fold(0.0f64, |p, v| p.max(v.abs()));
        assert!((0.2..1.2).contains(&peak), "{peak}");
    }

    #[test]
    fn the_voice_is_the_double_reed_at_48k() {
        let mut noise = PpNoise::start();
        let note = OboeVoice.note(&mut noise, 69.0, 0.9, 0.25);
        let raw = double_reed(pp::midi_hz(69.0), 0.25, 0.9);
        assert_eq!(note, pp::to_48k(&raw));
        assert_eq!((OboeVoice.register(), OboeVoice.name()), (REGISTER, "OBOE"));
        // the shared noise stream is left where it was
        let mut fresh = PpNoise::start();
        assert_eq!(noise.nz(4), fresh.nz(4));
        assert!(OboeVoice.can_hold(0.0) && OboeVoice.can_hold(-0.12));
        assert!(!OboeVoice.can_hold(-0.13) && !OboeVoice.can_hold(f64::NAN));
        // a quarter-tone flat A is a note too
        assert!(OboeVoice.note(&mut noise, 68.5, 0.7, 0.3).iter().all(|v| v.is_finite()));
    }
}
