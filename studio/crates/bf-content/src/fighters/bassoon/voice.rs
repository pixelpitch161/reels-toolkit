//! BASSOON's voice: `pp_bridge.voice('BASSOON', midi, vel, dur)`, and the tone its part calls directly.
//!
//! `pp_bridge` plays one note through `series/instrument-history/bassoon_lowpoly.py`'s `synth`, which is
//! `bassoon_reed(_midi_hz(m), dur, v)`: the double reed. A stack of harmonics up to 9 kHz shaped by four formants
//! (the bassoon's nasal "aw"), louder notes brighter, a weak fundamental down low; the pitch a touch flat for the
//! first instants (the tongue) and a slow vibrato on notes held past 0.3 s; an optional growl (a 31 Hz tremolo);
//! the reed's buzz and breath as band-passed noise; an 18 ms attack with the tongue's bump and a short release.
//! The note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! The noise is the note's own: `np.random.default_rng(int(f * 10) % 100003)`, a generator per pitch, `n` normals.
//! The series' shared noise stream is not touched.
//!
//! BASSOON's part (`kit/fighters/bassoon.py` `Sound.bsn`) does not go through `pp_bridge.voice`: it calls
//! [`bassoon_reed`] itself (with a growl for its blats) and fits and levels the note its own way.

use bf_core::dmath::{self, PI};
use bf_core::py::{py_max, py_min};
use bf_core::rng::Generator;
use bf_sound::dsp::{self, FilterKind};
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::{PpNoise, Voice};

/// The BASSOON's register in `pp_bridge`'s roster (`('BASSOON', 'bassoon_lowpoly', 'ball', (34, 75), ...)`).
pub const REGISTER: (i32, i32) = (34, 75);

/// `FORMANTS`: `(Hz, width, gain)`.
const FORMANTS: [(f64, f64, f64); 4] = [
    (480.0, 170.0, 2.3),
    (1150.0, 320.0, 1.25),
    (2500.0, 600.0, 0.35),
    (3600.0, 800.0, 0.15),
];
/// `bassoon_reed`'s default `bright`.
const BRIGHT: f64 = 0.55;
/// Its default `vib`: the vibrato's depth.
const VIB: f64 = 0.004;
/// Its default `vrate`: the vibrato's rate in Hz.
const VRATE: f64 = 5.0;
/// Its default `att`: the attack in seconds.
const ATT: f64 = 0.018;
/// Its default `buzz`.
const BUZZ: f64 = 0.08;
/// Its default `breath`.
const BREATH: f64 = 0.010;
/// The seconds a note rings past its held length: `n = int((dur + .14) * SR)`.
const TAIL: f64 = 0.14;

/// The BASSOON's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct BassoonVoice;

impl Voice for BassoonVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&bassoon_reed(pp::midi_hz(midi), dur, vel, 0.0))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "BASSOON"
    }

    /// Down to -0.14 s: the note is `int((dur + .14) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + TAIL)
    }
}

/// `bassoon_reed(f, dur, v, growl=growl)` with the module's other defaults: one note of the double reed at `f` Hz,
/// held `dur` seconds at velocity `v`, at 44,100 Hz. `growl` (0 for none) is the depth of a 31 Hz tremolo.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or below -0.14 s (a negative number of samples); `f` is not a
/// finite number (its seed).
pub fn bassoon_reed(f: f64, dur: f64, v: f64, growl: f64) -> Vec<f64> {
    let n = pp::len(dur + TAIL);
    let t = pp::tt(n);
    let rel = if dur < 0.15 { 0.028 } else { 0.05 };
    // env = np.minimum(1, t / att) ** 1.3 * np.where(t < dur, 1, np.exp(-(t - dur) / rel))
    // env *= 1 + .18 * np.exp(-t / .035)                                       (the tongue's bump)
    let env: Vec<f64> = t
        .iter()
        .map(|&t| {
            let rise = if t / ATT < 1.0 { t / ATT } else { 1.0 };
            let hold = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / rel) };
            dmath::pow(rise, 1.3) * hold * (1.0 + 0.18 * dmath::exp(-t / 0.035))
        })
        .collect();
    // held = np.clip((t - .2) / .25, 0, 1) if dur > .3 else 0.0
    // fr = f * (1 - .015 * np.exp(-t / .025)) * (1 + vib * np.sin(2 * np.pi * vrate * t) * held)
    let wobble = 2.0 * PI * VRATE;
    let fr: Vec<f64> = t
        .iter()
        .map(|&t| {
            let held = if dur > 0.3 {
                bf_sound::synth::clip((t - 0.2) / 0.25, 0.0, 1.0)
            } else {
                0.0
            };
            f * (1.0 - 0.015 * dmath::exp(-t / 0.025)) * (1.0 + VIB * dmath::sin(wobble * t) * held)
        })
        .collect();
    // ph = 2 * np.pi * np.cumsum(fr) / SR
    let ph: Vec<f64> = dsp::cumsum(&fr).iter().map(|&c| 2.0 * PI * c / SRF).collect();
    let mut x = vec![0.0; n];
    let mut tot = 0.0;
    for k in 1..60 {
        let kf = f64::from(k);
        let fk = f * kf;
        if fk > 9000.0 {
            break;
        }
        // a = k ** -.55 * (.22 + sum(g * math.exp(-((fk - c) / w) ** 2) for c, w, g in FORMANTS))
        let mut formants = 0.0;
        for (c, w, g) in FORMANTS {
            let d = (fk - c) / w;
            formants += g * dmath::exp(-(d * d));
        }
        let mut a = dmath::pow(kf, -0.55) * (0.22 + formants);
        // louder = brighter
        a *= dmath::exp(-fk / (1800.0 + 3600.0 * BRIGHT * v));
        if k == 1 {
            // a weak fundamental down low
            a *= py_min(1.0, py_max(0.3, (f - 60.0) / 180.0));
        }
        // x += a * np.sin(k * ph + .9 * k)
        let shift = 0.9 * kf;
        for (o, &p) in x.iter_mut().zip(&ph) {
            *o += a * dmath::sin(kf * p + shift);
        }
        tot += a;
    }
    let by = tot + 1e-9;
    for o in &mut x {
        *o /= by;
    }
    if growl != 0.0 {
        // x *= 1 + growl * np.sin(2 * np.pi * 31 * t)
        let w = 2.0 * PI * 31.0;
        for (o, &t) in x.iter_mut().zip(&t) {
            *o *= 1.0 + growl * dmath::sin(w * t);
        }
    }
    // rng = np.random.default_rng(int(f * 10) % 100003): deterministic noise per pitch
    let tenth = f * 10.0;
    assert!(tenth.is_finite(), "cannot convert float {tenth} to integer");
    let mut rng = Generator::new((tenth as i64).rem_euclid(100_003) as u64);
    let nz: Vec<f64> = (0..n).map(|_| rng.standard_normal()).collect();
    // nb = sosfilt(butter(2, [700, 2600], 'band', fs=SR, output='sos'), nz)
    let nb = dsp::sosfilt(&dsp::butter_sos(FilterKind::Bp, &[700.0, 2600.0], 2, SRF), &nz);
    // x += nb * (buzz * np.exp(-t / .02) + breath) * .7;  return x * env * v
    (0..n)
        .map(|i| {
            let s = x[i] + nb[i] * (BUZZ * dmath::exp(-t[i] / 0.02) + BREATH) * 0.7;
            s * env[i] * v
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_rings_a_seventh_of_a_second_past_its_length() {
        let x = bassoon_reed(pp::midi_hz(52.0), 0.2, 0.8, 0.0);
        assert_eq!(x.len(), ((0.2 + 0.14) * SRF) as usize);
        assert_eq!(x[0], 0.0, "the attack starts from silence");
        assert_eq!(
            x,
            bassoon_reed(pp::midi_hz(52.0), 0.2, 0.8, 0.0),
            "the noise is the pitch's own"
        );
        let peak = x.iter().fold(0.0f64, |p, v| p.max(v.abs()));
        assert!(peak > 0.05 && peak < 1.5, "{peak}");
        let end = x[x.len() - 200..].iter().fold(0.0f64, |p, v| p.max(v.abs()));
        assert!(end < 0.1 * peak, "{end} against {peak}");
    }

    #[test]
    fn a_growl_is_a_tremolo_on_the_tone() {
        let (plain, growl) = (bassoon_reed(110.0, 0.3, 1.0, 0.0), bassoon_reed(110.0, 0.3, 1.0, 0.4));
        assert_eq!(plain.len(), growl.len());
        assert_ne!(plain, growl);
    }

    #[test]
    fn the_voice_is_the_reed_at_48_khz() {
        let v = BassoonVoice;
        assert_eq!((v.name(), v.register()), ("BASSOON", (34, 75)));
        assert!(v.can_hold(-0.14) && !v.can_hold(-0.15) && !v.can_hold(f64::NAN));
        let mut noise = PpNoise::start();
        let before = format!("{noise:?}");
        let x = v.note(&mut noise, 54.0, 0.8, 0.12);
        assert_eq!(format!("{noise:?}"), before, "the shared noise is not drawn from");
        assert_eq!(x, pp::to_48k(&bassoon_reed(pp::midi_hz(54.0), 0.12, 0.8, 0.0)));
    }
}
