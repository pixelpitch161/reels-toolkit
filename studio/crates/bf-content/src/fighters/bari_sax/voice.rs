//! BARI SAX's voice: `pp_bridge.voice('BARI SAX', midi, vel, dur)`, and the tone its part calls directly.
//!
//! `pp_bridge` plays one note through `series/instrument-history/barisax_lowpoly.py`'s `synth`, which is
//! `bari_tone(_midi_hz(m), dur, v)`: harmonics up to 8 kHz rolled off by frequency through the bari's own colour (a
//! low, wide formant near 560 Hz, a second near 1.35 kHz, a third near 2.6 kHz), louder notes brighter; the pitch a
//! scoop flat for the first instants and a slow vibrato on held notes; a rough flutter (29 Hz, wobbling at 7 Hz);
//! band-passed breath, strongest on the attack; a 16 ms attack with a bite and a short release. `honk` (0 to 1) adds
//! the growl, the scoop and the slap of a hard-blown low note. The note is made at 44,100 Hz and resampled to
//! 48,000 Hz.
//!
//! The noise is the note's own: `np.random.default_rng(int(f * 10) % 100003)`, a generator per pitch, `n` normals.
//! The series' shared noise stream is not touched.
//!
//! BARI SAX's part (`older/fighters/bari_sax.py` `Sound.bari`) calls [`bari_tone`] itself (with a honk) and fits and
//! levels the note its own way.

use bf_core::dmath::{self, PI};
use bf_core::rng::Generator;
use bf_sound::dsp::{self, FilterKind};
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::{PpNoise, Voice};

/// The BARI SAX's register in `pp_bridge`'s roster (`('BARI SAX', 'barisax_lowpoly', 'ball', (36, 69), ...)`).
pub const REGISTER: (i32, i32) = (36, 69);

/// `FORMANTS`: `(Hz, width, gain)`.
const FORMANTS: [(f64, f64, f64); 3] = [(560.0, 300.0, 1.7), (1350.0, 520.0, 0.8), (2600.0, 800.0, 0.2)];
/// `bari_tone`'s default `bright`.
const BRIGHT: f64 = 0.6;
/// The seconds a note rings past its held length: `n = int((dur + .16) * SR)`.
const TAIL: f64 = 0.16;
/// The widest scoop of a steady note ([`bari_tone_scooped`]): 1.2 % flat, about a fifth of a semitone, as a real
/// bari sax's (sound upgrade, "Bari sax: no slide").
pub const STEADY_SCOOP: f64 = 0.012;

/// The BARI SAX's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct BariSaxVoice;

impl Voice for BariSaxVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&bari_tone(pp::midi_hz(midi), dur, vel, 0.0))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "BARI SAX"
    }

    /// Down to -0.16 s: the note is `int((dur + .16) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + TAIL)
    }
}

/// `bari_tone(f, dur, v, honk=honk)` with the module's other defaults: one note of the bari at `f` Hz, held `dur`
/// seconds at velocity `v`, at 44,100 Hz.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or below -0.16 s (a negative number of samples); `f` is not a
/// finite number (its seed).
pub fn bari_tone(f: f64, dur: f64, v: f64, honk: f64) -> Vec<f64> {
    bari_tone_scooped(f, dur, v, honk, None)
}

/// [`bari_tone`] with its scoop held to `max_scoop` (a share of the pitch; `None` leaves the module's own scoop,
/// which is [`bari_tone`] to the sample). The Pixel Pitch tone starts each note 3.5 % flat, and a full honk 7.5 %,
/// over a semitone: heard on every long honk that is a slide. BARI SAX's part asks for [`STEADY_SCOOP`] under the
/// Composed arrangement (sound upgrade, "Bari sax: no slide"); Classic and the instrument's voice ask for none.
///
/// # Panics
/// As [`bari_tone`].
pub fn bari_tone_scooped(f: f64, dur: f64, v: f64, honk: f64, max_scoop: Option<f64>) -> Vec<f64> {
    let n = pp::len(dur + TAIL);
    let t = pp::tt(n);
    let rel = if dur < 0.15 { 0.035 } else { 0.06 };
    // env = np.minimum(1, t / .016) ** 1.3 * np.where(t < dur, 1, np.exp(-(t - dur) / rel)) * (1 + .2 * exp(-t / .04))
    let env: Vec<f64> = t
        .iter()
        .map(|&t| {
            let rise = if t / 0.016 < 1.0 { t / 0.016 } else { 1.0 };
            let hold = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / rel) };
            dmath::pow(rise, 1.3) * hold * (1.0 + 0.2 * dmath::exp(-t / 0.04))
        })
        .collect();
    // fr = f * (1 - (.035 + .04 * honk) * np.exp(-t / .045)) *
    //      (1 + .003 * (1 - honk) * np.sin(2 * np.pi * 4.8 * t) * np.clip((t - .2) / .3, 0, 1))
    let wobble = 2.0 * PI * 4.8;
    let scoop = 0.035 + 0.04 * honk;
    let scoop = match max_scoop {
        Some(most) if scoop > most => most,
        _ => scoop,
    };
    let vib = 0.003 * (1.0 - honk);
    let fr: Vec<f64> = t
        .iter()
        .map(|&t| {
            let held = bf_sound::synth::clip((t - 0.2) / 0.3, 0.0, 1.0);
            f * (1.0 - scoop * dmath::exp(-t / 0.045)) * (1.0 + vib * dmath::sin(wobble * t) * held)
        })
        .collect();
    // ph = 2 * np.pi * np.cumsum(fr) / SR
    let ph: Vec<f64> = dsp::cumsum(&fr).iter().map(|&c| 2.0 * PI * c / SRF).collect();
    let mut x = vec![0.0; n];
    let mut tot = 0.0;
    let br = BRIGHT + 0.25 * honk;
    for k in 1..90 {
        let kf = f64::from(k);
        let fk = f * kf;
        if fk > 8000.0 {
            break;
        }
        // a = k ** -.7 * (.3 + sum(g * math.exp(-((fk - c) / w) ** 2) for c, w, g in FORMANTS))
        let mut formants = 0.0;
        for (c, w, g) in FORMANTS {
            let d = (fk - c) / w;
            formants += g * dmath::exp(-(d * d));
        }
        let mut a = dmath::pow(kf, -0.7) * (0.3 + formants);
        a *= dmath::exp(-fk / (1300.0 + 3200.0 * br * v));
        // x += a * np.sin(k * ph + .4 * k)
        let shift = 0.4 * kf;
        for (o, &p) in x.iter_mut().zip(&ph) {
            *o += a * dmath::sin(kf * p + shift);
        }
        tot += a;
    }
    let by = tot + 1e-9;
    for o in &mut x {
        *o /= by;
    }
    // growl: a rough flutter: x *= 1 + gr * np.sin(2 * np.pi * 29 * t + 3 * np.sin(2 * np.pi * 7 * t))
    let gr = 0.05 + 0.22 * honk;
    let (w29, w7) = (2.0 * PI * 29.0, 2.0 * PI * 7.0);
    for (o, &t) in x.iter_mut().zip(&t) {
        *o *= 1.0 + gr * dmath::sin(w29 * t + 3.0 * dmath::sin(w7 * t));
    }
    // rng = np.random.default_rng(int(f * 10) % 100003)
    let tenth = f * 10.0;
    assert!(tenth.is_finite(), "cannot convert float {tenth} to integer");
    let mut rng = Generator::new((tenth as i64).rem_euclid(100_003) as u64);
    let raw: Vec<f64> = (0..n).map(|_| rng.standard_normal()).collect();
    // nz = sosfilt(butter(2, [500, 3200], 'band', fs=SR, output='sos'), rng.standard_normal(n))
    let nz = dsp::sosfilt(&dsp::butter_sos(FilterKind::Bp, &[500.0, 3200.0], 2, SRF), &raw);
    // x += nz * (.05 + .05 * honk) * (.5 + 1.6 * np.exp(-t * 18))
    let breath = 0.05 + 0.05 * honk;
    for i in 0..n {
        x[i] += nz[i] * breath * (0.5 + 1.6 * dmath::exp(-t[i] * 18.0));
    }
    if honk != 0.0 {
        // the slap of the reed on a hard attack: k = int(.03 * SR); tt = t[:k]
        // x[:k] += .5 * honk * (np.sin(2 * np.pi * 150 * tt) * np.exp(-tt * 100) + nz[:k] * np.exp(-tt * 260) * 2)
        let k = ((0.03 * SRF) as usize).min(n);
        let w150 = 2.0 * PI * 150.0;
        let g = 0.5 * honk;
        for i in 0..k {
            let tt = t[i];
            x[i] += g * (dmath::sin(w150 * tt) * dmath::exp(-tt * 100.0) + nz[i] * dmath::exp(-tt * 260.0) * 2.0);
        }
    }
    // return x * env * v
    (0..n).map(|i| x[i] * env[i] * v).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_rings_past_its_length_and_honks() {
        let x = bari_tone(pp::midi_hz(40.0), 0.3, 0.9, 0.0);
        assert_eq!(x.len(), ((0.3 + 0.16) * SRF) as usize);
        assert_eq!(x[0], 0.0, "the attack starts from silence");
        assert_eq!(
            x,
            bari_tone(pp::midi_hz(40.0), 0.3, 0.9, 0.0),
            "the noise is the pitch's own"
        );
        assert_ne!(x, bari_tone(pp::midi_hz(40.0), 0.3, 0.9, 1.0));
    }

    #[test]
    fn the_voice_is_the_tone_at_48_khz() {
        let v = BariSaxVoice;
        assert_eq!((v.name(), v.register()), ("BARI SAX", (36, 69)));
        assert!(v.can_hold(-0.16) && !v.can_hold(-0.17) && !v.can_hold(f64::NAN));
        let mut noise = PpNoise::start();
        let before = format!("{noise:?}");
        let x = v.note(&mut noise, 45.0, 0.8, 0.2);
        assert_eq!(format!("{noise:?}"), before, "the shared noise is not drawn from");
        assert_eq!(x, pp::to_48k(&bari_tone(pp::midi_hz(45.0), 0.2, 0.8, 0.0)));
    }
}
