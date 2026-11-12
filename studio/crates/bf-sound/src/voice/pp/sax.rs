//! `altosax60_lowpoly.sax_tone`: the Pixel Pitch alto saxophone's tone. The ALTO SAX's voice
//! (`altosax_lowpoly.alto_tone`, the same recipe a shade brighter) and the alto voice the mambo bed plays
//! (`altosax60_lowpoly.synth` with its scene's `TONES` entry) both call it; their tone tables and what they add
//! belong to their own ports.
//!
//! Its breath and slap noise come from the module's own generator, `_RN = default_rng(23)`, which both callers
//! share: [`crate::voice::ModuleNoise::sax`].

use bf_core::dmath::{self, PI};
use bf_core::rng::Generator;

use super::{SRF, bp, clip01, len, min1, tt};
use crate::dsp;

/// `sax_tone`'s tone arguments with their Python defaults ([`Sax::default`]). The ones Python tests for truth
/// (`if gl:`, `if growl:`, `if breath:`, `if slap:`) are off at `None` or 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sax {
    /// `bright=.6`
    pub bright: f64,
    /// `vib=.005`: the vibrato's depth, a fraction of the frequency.
    pub vib: f64,
    /// `vrate=5.2`: the vibrato's rate, Hz.
    pub vrate: f64,
    /// `odd=.12`: how much quieter the even harmonics are.
    pub odd: f64,
    /// `scoop=.03`
    pub scoop: f64,
    /// `att=.025`: the attack, seconds.
    pub att: f64,
    /// `formant=1150.`: the body's resonance, Hz.
    pub formant: f64,
    /// `fboost=.8`: how strong it is.
    pub fboost: f64,
    /// `breath=.05`: breath noise.
    pub breath: f64,
    /// `growl=0.`
    pub growl: f64,
    /// `slap=0.`: a slap-tongue attack.
    pub slap: f64,
    /// `gl=None`: a glide into the note, `(semitones, seconds)`.
    pub gl: Option<(f64, f64)>,
}

impl Default for Sax {
    fn default() -> Sax {
        Sax {
            bright: 0.6,
            vib: 0.005,
            vrate: 5.2,
            odd: 0.12,
            scoop: 0.03,
            att: 0.025,
            formant: 1150.0,
            fboost: 0.8,
            breath: 0.05,
            growl: 0.0,
            slap: 0.0,
            gl: None,
        }
    }
}

/// `sax_tone(f, dur, v, **tone)`: `int((dur + .16) * SR)` samples at 44,100 Hz of the alto at `f` Hz held for `dur`
/// seconds at velocity `v`: up to 47 harmonics (those up to 9 kHz) shaped by the formant and blended dark to bright
/// by the level, then the growl, the breath (`bp(_RN.normal(0, 1, n), 1600, 6500)`) and the slap
/// (`bp(_RN.normal(0, 1, int(.03 * SR)), 800, 4000)` on the first 30 ms), drawing from `rn` in that order.
///
/// # Panics
/// Where Python raises: `dur` is not a number or gives a negative number of samples, or a slap on a note shorter
/// than 30 ms (`dur` below -0.13 s: numpy cannot add the 1,323 slap samples to fewer).
pub fn sax_tone(rn: &mut Generator, f: f64, dur: f64, v: f64, tone: &Sax) -> Vec<f64> {
    let n = len(dur + 0.16);
    let t = tt(n);
    // env = minimum(1, t / att) ** 1.4 * where(t < dur, 1, exp(-(t - dur) / .05)) * (.86 + .14 * exp(-t * 5))
    let env: Vec<f64> = t
        .iter()
        .map(|&t| {
            let hold = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.05) };
            dmath::pow(min1(t / tone.att), 1.4) * hold * (0.86 + 0.14 * dmath::exp(-t * 5.0))
        })
        .collect();
    // fr = f * (1 - scoop * exp(-t / .05)) * (1 + vib * sin(2 pi vrate t) * clip((t - .16) / .3, 0, 1))
    let wobble = 2.0 * PI * tone.vrate;
    let mut fr: Vec<f64> = t
        .iter()
        .map(|&t| {
            f * (1.0 - tone.scoop * dmath::exp(-t / 0.05))
                * (1.0 + tone.vib * dmath::sin(wobble * t) * clip01((t - 0.16) / 0.3))
        })
        .collect();
    if let Some((semis, secs)) = tone.gl {
        // u = clip(t / secs, 0, 1); fr = fr * 2 ** (semis * (1 - u * u * (3 - 2 * u)) / 12)
        for (fr, &t) in fr.iter_mut().zip(&t) {
            let u = clip01(t / secs);
            *fr *= dmath::pow(2.0, semis * (1.0 - u * u * (3.0 - 2.0 * u)) / 12.0);
        }
    }
    let ph: Vec<f64> = dsp::cumsum(&fr).iter().map(|&c| 2.0 * PI * c / SRF).collect();
    let (mut dark, mut brt) = (vec![0.0; n], vec![0.0; n]);
    let (mut sd, mut sb) = (0.0, 0.0);
    for k in 1..48i32 {
        let kf = f64::from(k);
        let fk = f * kf;
        if fk > 9000.0 {
            break;
        }
        // base = (1 / k ** .85) * ((1 - odd) if k % 2 == 0 else 1)
        let mut base = (1.0 / dmath::pow(kf, 0.85)) * if k % 2 == 0 { 1.0 - tone.odd } else { 1.0 };
        // base *= 1 + fboost * exp(-((fk - formant) / 650) ** 2) + .6 * bright * exp(-((fk - 2.4 * formant) / 900) ** 2)
        let q1 = (fk - tone.formant) / 650.0;
        let q2 = (fk - 2.4 * tone.formant) / 900.0;
        base *= 1.0 + tone.fboost * dmath::exp(-(q1 * q1)) + 0.6 * tone.bright * dmath::exp(-(q2 * q2));
        let a1 = base * dmath::exp(-kf / 3.0);
        let a2 = base * dmath::exp(-kf / (5.0 + 11.0 * tone.bright));
        let off = 0.4 * kf;
        for i in 0..n {
            let s = dmath::sin(kf * ph[i] + off);
            dark[i] += a1 * s;
            brt[i] += a2 * s;
        }
        sd += a1;
        sb += a2;
    }
    let mut x: Vec<f64> = (0..n)
        .map(|i| {
            // mb = clip(env * v, 0, 1) ** 1.2
            let mb = dmath::pow(clip01(env[i] * v), 1.2);
            (dark[i] / sd * (1.0 - mb) + brt[i] / sb * mb) * env[i]
        })
        .collect();
    if tone.growl != 0.0 {
        // x = x * (1 + growl * sin(2 pi 29 t + 3 sin(2 pi 7 t)))
        let (w29, w7) = (2.0 * PI * 29.0, 2.0 * PI * 7.0);
        for (x, &t) in x.iter_mut().zip(&t) {
            *x *= 1.0 + tone.growl * dmath::sin(w29 * t + 3.0 * dmath::sin(w7 * t));
        }
    }
    if tone.breath != 0.0 {
        // nz = D.bp(_RN.normal(0, 1, n), 1600, 6500); x = x + breath * nz * env * (.45 + 1.6 * exp(-t * 18))
        let mut noise = vec![0.0; n];
        rn.fill_normal(0.0, 1.0, &mut noise);
        let nz = bp(&noise, 1600.0, 6500.0);
        for i in 0..n {
            x[i] += tone.breath * nz[i] * env[i] * (0.45 + 1.6 * dmath::exp(-t[i] * 18.0));
        }
    }
    if tone.slap != 0.0 {
        // k = int(.03 * SR); x[:k] += slap * (sin(2 pi 170 tt) * exp(-tt * 110)
        //                                     + D.bp(_RN.normal(0, 1, k), 800, 4000) * exp(-tt * 300) * .6)
        let k = len(0.03);
        let mut noise = vec![0.0; k];
        rn.fill_normal(0.0, 1.0, &mut noise);
        assert!(
            n >= k,
            "operands could not be broadcast together with shapes ({n},) ({k},) (a slap on a note of {n} samples)"
        );
        let nz = bp(&noise, 800.0, 4000.0);
        let w = 2.0 * PI * 170.0;
        for i in 0..k {
            let tt = t[i];
            x[i] += tone.slap * (dmath::sin(w * tt) * dmath::exp(-tt * 110.0) + nz[i] * dmath::exp(-tt * 300.0) * 0.6);
        }
    }
    for x in &mut x {
        *x *= v;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tone_draws_its_noise_from_its_own_generator() {
        let tone = Sax::default();
        let (mut a, mut b) = (Generator::new(23), Generator::new(23));
        let x = sax_tone(&mut a, 440.0, 0.2, 0.8, &tone);
        assert_eq!(x.len(), 15876);
        // the breath drew n normals; nothing else
        let mut skip = vec![0.0; 15876];
        b.fill_normal(0.0, 1.0, &mut skip);
        assert_eq!(a.random().to_bits(), b.random().to_bits());
        // no breath, no slap: no draw at all
        let mut c = Generator::new(23);
        let dry = sax_tone(&mut c, 440.0, 0.2, 0.8, &Sax { breath: 0.0, ..tone });
        assert_eq!(c.random().to_bits(), Generator::new(23).random().to_bits());
        assert!(x.iter().zip(&dry).any(|(p, q)| p != q));
        // a slap draws 1,323 more, after the breath's
        let (mut d, mut e) = (Generator::new(23), Generator::new(23));
        sax_tone(&mut d, 440.0, 0.2, 0.8, &Sax { slap: 0.5, ..tone });
        let mut skip = vec![0.0; 15876 + 1323];
        e.fill_normal(0.0, 1.0, &mut skip);
        assert_eq!(d.random().to_bits(), e.random().to_bits());
    }

    #[test]
    fn the_glide_and_the_growl_change_the_tone() {
        let tone = Sax {
            breath: 0.0,
            ..Sax::default()
        };
        let mut g = Generator::new(23);
        let plain = sax_tone(&mut g, 330.0, 0.3, 0.9, &tone);
        let glide = sax_tone(
            &mut g,
            330.0,
            0.3,
            0.9,
            &Sax {
                gl: Some((-2.0, 0.08)),
                ..tone
            },
        );
        let growl = sax_tone(&mut g, 330.0, 0.3, 0.9, &Sax { growl: 0.3, ..tone });
        assert!(plain.iter().zip(&glide).any(|(p, q)| (p - q).abs() > 1e-3));
        assert!(plain.iter().zip(&growl).any(|(p, q)| (p - q).abs() > 1e-3));
        // past the glide's end the glided note runs on at the same pitch
        assert_eq!(plain.len(), glide.len());
    }

    #[test]
    #[should_panic(expected = "operands could not be broadcast together")]
    fn a_slap_on_a_note_shorter_than_the_slap_is_refused() {
        sax_tone(
            &mut Generator::new(23),
            440.0,
            -0.14,
            0.8,
            &Sax {
                slap: 0.5,
                ..Sax::default()
            },
        );
    }
}
