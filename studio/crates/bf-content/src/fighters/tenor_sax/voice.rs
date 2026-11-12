//! TENOR SAX's instrument voice: `pp_bridge.voice('TENOR SAX', midi, vel, dur)`, and the tone its part plays.
//!
//! `pp_bridge` plays one note through `series/instrument-history/tenorsax_lowpoly.py`'s `synth`, which is
//! `tenor_tone(_midi_hz(m), dur, v, honk=.35)`: the tenor's husky honk. Up to 69 harmonics (those up to 9 kHz)
//! rolled off by frequency through the tenor's own colour (a broad low formant near 620 Hz, a bump near 1.5 kHz),
//! on a pitch that scoops up into the note with a lazy vibrato on long notes, a growl, and a breathy, airy noise
//! band that never quite goes away. `honk` (0 to 1) is the rock 'n' roll honk: a bigger scoop, more growl and bark
//! and the reed's slap on the attack. `sub` (0 to 1) is the subtone: fewer harmonics, the breath up, a slower
//! attack. The note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! The breath draws from a generator of its own, seeded from the pitch and the length
//! (`default_rng(int(f * 10 + dur * 1000) % 100003)`), so the shared noise stream is left where it was.
//!
//! The part (`sound.rs`) calls [`tenor_tone`] itself with its own `honk` and `sub`, as the Python part does.

use bf_core::dmath::{self, PI};
use bf_core::py::{self, py_max};
use bf_core::rng::Generator;
use bf_sound::dsp::{self, FilterKind};
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::{PpNoise, Voice};

/// TENOR SAX's register in `pp_bridge`'s roster (`('TENOR SAX', 'tenorsax_lowpoly', 'ball', (44, 76), 0, 1.06)`).
pub const REGISTER: (i32, i32) = (44, 76);

/// `synth`'s `honk`: the honk of the note `pp_bridge.voice` plays.
pub const VOICE_HONK: f64 = 0.35;

/// `FORMANTS`: `(centre Hz, width Hz, gain)`.
const FORMANTS: [(f64, f64, f64); 3] = [(620.0, 340.0, 1.6), (1500.0, 560.0, 0.85), (2800.0, 850.0, 0.25)];

/// `tenor_tone`'s default `bright`.
const BRIGHT: f64 = 0.42;

/// TENOR SAX's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct TenorSaxVoice;

impl Voice for TenorSaxVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "TENOR SAX"
    }

    /// Down to -0.18 s: the tone is `int((dur + .18) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.18)
    }
}

/// `_midi_hz(m)`: the module's own, `440.0 * 2 ** ((m - 69) / 12.0)`.
pub fn midi_hz(m: f64) -> f64 {
    440.0 * dmath::pow(2.0, (m - 69.0) / 12.0)
}

/// `tenorsax_lowpoly.synth` for the one note `pp_bridge.voice` asks: `tenor_tone(_midi_hz(m), dur, v, honk=.35)`,
/// at 44,100 Hz, before it is resampled.
///
/// # Panics
/// As [`tenor_tone`].
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    tenor_tone(midi_hz(midi), dur, vel, VOICE_HONK, 0.0)
}

/// `tenor_tone(f, dur, v, honk=honk, sub=sub, bright=.42)`: the tone at `f` Hz held for `dur` seconds at velocity
/// `v`, `int((dur + .18) * SR)` samples at 44,100 Hz.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or so far below -0.18 s that the note has a negative number of
/// samples; a pitch and length whose seed `int(f * 10 + dur * 1000)` is not a whole number an integer holds.
pub fn tenor_tone(f: f64, dur: f64, v: f64, honk: f64, sub: f64) -> Vec<f64> {
    let n = pp::len(dur + 0.18);
    let t = pp::tt(n);
    let att = 0.014 + 0.05 * sub;
    let rel = if dur < 0.15 { 0.04 } else { 0.07 };
    // fr = f * (1 - (.03 + .06 * honk) * np.exp(-t / (.05 + .03 * honk))): the scoop up into the note
    // fr = fr * (1 + .0045 * np.sin(2 * np.pi * 5.1 * t) * np.clip((t - .22) / .3, 0, 1)): a lazy vibrato
    let (depth, fall) = (0.03 + 0.06 * honk, 0.05 + 0.03 * honk);
    let vib = 2.0 * PI * 5.1;
    let fr: Vec<f64> = t
        .iter()
        .map(|&t| {
            let fr = f * (1.0 - depth * dmath::exp(-t / fall));
            fr * (1.0 + 0.0045 * dmath::sin(vib * t) * ((t - 0.22) / 0.3).clamp(0.0, 1.0))
        })
        .collect();
    // ph = 2 * np.pi * np.cumsum(fr) / SR
    let ph: Vec<f64> = dsp::cumsum(&fr).iter().map(|&c| 2.0 * PI * c / SRF).collect();
    let mut x = vec![0.0; n];
    let mut tot = 0.0;
    let br = BRIGHT + 0.3 * honk - 0.35 * sub;
    for k in 1..70 {
        let k = f64::from(k);
        let fk = f * k;
        if fk > 9000.0 {
            break;
        }
        // a = k ** -.75 * (.3 + sum(g * math.exp(-((fk - c) / w) ** 2) for c, w, g in FORMANTS))
        let mut formants = 0.0;
        for (c, w, g) in FORMANTS {
            formants += g * dmath::exp(-dmath::pow((fk - c) / w, 2.0));
        }
        let mut a = dmath::pow(k, -0.75) * (0.3 + formants);
        // a *= math.exp(-fk / max(500.0, 1100 + 3600 * br * v))
        a *= dmath::exp(-fk / py_max(500.0, 1100.0 + 3600.0 * br * v));
        let shift = 0.37 * k;
        for (x, &ph) in x.iter_mut().zip(&ph) {
            *x += a * dmath::sin(k * ph + shift);
        }
        tot += a;
    }
    let tot = tot + 1e-9;
    // x *= 1 + gr * np.sin(2 * np.pi * 31 * t + 2.6 * np.sin(2 * np.pi * 6.5 * t)): the growl
    let gr = 0.04 + 0.26 * honk;
    let (growl, wobble) = (2.0 * PI * 31.0, 2.0 * PI * 6.5);
    for (x, &t) in x.iter_mut().zip(&t) {
        *x /= tot;
        *x *= 1.0 + gr * dmath::sin(growl * t + 2.6 * dmath::sin(wobble * t));
    }
    let seed = py::imod_py(whole(f * 10.0 + dur * 1000.0), 100_003) as u64;
    let mut rng = Generator::new(seed);
    // rng.standard_normal(n), twice: the first for the husk, the second for the air
    let first: Vec<f64> = (0..n).map(|_| rng.standard_normal()).collect();
    let second: Vec<f64> = (0..n).map(|_| rng.standard_normal()).collect();
    let nz = dsp::sosfilt(&dsp::butter_sos(FilterKind::Bp, &[380.0, 2600.0], 2, SRF), &first);
    let air = dsp::sosfilt(&dsp::butter_sos(FilterKind::Bp, &[2200.0, 7000.0], 2, SRF), &second);
    let husk = 0.05 + 0.06 * sub + 0.03 * (1.0 - v);
    let air_k = 0.012 + 0.012 * sub;
    // x += nz * husk * (.65 + 1.4 * np.exp(-t * 16)) + air * (.012 + .012 * sub)
    for i in 0..n {
        x[i] += nz[i] * husk * (0.65 + 1.4 * dmath::exp(-t[i] * 16.0)) + air[i] * air_k;
    }
    if honk != 0.0 {
        // the reed's slap on a hard attack
        let k = pp::len(0.028).min(n);
        let slap = 2.0 * PI * 190.0;
        for i in 0..k {
            let tt = t[i];
            x[i] +=
                0.45 * honk * (dmath::sin(slap * tt) * dmath::exp(-tt * 110.0) + nz[i] * dmath::exp(-tt * 240.0) * 2.0);
        }
    }
    let bump = 0.18 + 0.2 * honk;
    (0..n)
        .map(|i| {
            let t = t[i];
            // env = np.minimum(1, t / att) ** 1.3 * np.where(t < dur, 1, np.exp(-(t - dur) / rel))
            //       * (1 + (.18 + .2 * honk) * np.exp(-t / .05))
            let rise = if t / att < 1.0 { t / att } else { 1.0 };
            let hold = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / rel) };
            let env = dmath::pow(rise, 1.3) * hold * (1.0 + bump * dmath::exp(-t / 0.05));
            x[i] * env * v
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
        // int((dur + .18) * 44100) samples
        assert_eq!(synth(46.0, 0.8, 0.12).len(), 13230);
        assert_eq!(synth(46.0, 0.8, 1.0).len(), 52038);
        let x = tenor_tone(116.54, 0.5, 0.9, 0.0, 0.0);
        assert_eq!(x.len(), pp::len(0.68));
        // it starts from nothing (the attack is a power of t) and has died away by its end
        assert_eq!(x[0], 0.0);
        assert!(peak(&x[x.len() - 441..]) < 0.05, "{}", peak(&x[x.len() - 441..]));
        let level = peak(&x[4410..17640]);
        assert!((0.2..1.6).contains(&level), "{level}");
        assert!(x.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn a_honk_slaps_and_a_subtone_is_slower() {
        // the same pitch and length draw the same breath, so the difference is the honk or the subtone alone
        let plain = tenor_tone(116.54, 0.5, 0.9, 0.0, 0.0);
        let honk = tenor_tone(116.54, 0.5, 0.9, 1.0, 0.0);
        let sub = tenor_tone(116.54, 0.5, 0.9, 0.0, 1.0);
        assert_eq!((plain.len(), plain.len()), (honk.len(), sub.len()));
        // the reed's slap and the bigger bump: the first 28 ms are louder on a honk
        assert!(peak(&honk[..1234]) > peak(&plain[..1234]));
        // a subtone's attack is slower: the first 10 ms are quieter
        assert!(peak(&sub[..441]) < peak(&plain[..441]));
    }

    #[test]
    fn the_breath_is_seeded_by_pitch_and_length_and_the_shared_noise_stays() {
        let mut noise = PpNoise::start();
        let note = TenorSaxVoice.note(&mut noise, 50.0, 0.9, 0.25);
        let raw = synth(50.0, 0.9, 0.25);
        assert_eq!(note, pp::to_48k(&raw));
        assert_eq!(raw.len(), 18963);
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        // the same note again is the same samples
        assert_eq!(raw, synth(50.0, 0.9, 0.25));
        assert_eq!(
            (TenorSaxVoice.register(), TenorSaxVoice.name()),
            (REGISTER, "TENOR SAX")
        );
        // what it can hold is what Python does not raise for
        assert!(TenorSaxVoice.can_hold(0.0) && TenorSaxVoice.can_hold(-0.18));
        assert!(!TenorSaxVoice.can_hold(-0.25) && !TenorSaxVoice.can_hold(f64::INFINITY));
    }

    #[test]
    fn the_voice_is_the_tone_with_its_honk() {
        assert_eq!(synth(46.0, 0.6, 0.12), tenor_tone(midi_hz(46.0), 0.12, 0.6, 0.35, 0.0));
    }
}
