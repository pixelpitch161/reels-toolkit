//! BASS CLARINET's instrument voice: `pp_bridge.voice('BASS CLARINET', midi, vel, dur)`, and the reed synth behind
//! it, which its part plays directly.
//!
//! `pp_bridge` plays one note through `series/instrument-history/bassclarinet_lowpoly.py`'s `synth`: `bass_reed` at
//! the note's pitch with every tone argument at its default, and a growl of 0.25 below midi 45 (the low chalumeau).
//! The note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! `bass_reed` is a sum of up to 47 harmonics (below 7.5 kHz) of a slightly scooped, slowly vibrating pitch: the
//! even ones held down (`odd`: a bore closed at the reed), two formant bumps (560 Hz and 1,450 Hz), and two spectra,
//! a dark one and a bright one, mixed by the note's own loudness; a breath of band-passed noise (700 to 3,200 Hz)
//! from a generator of the note's own, seeded by its pitch; and, with a `growl`, a rasp: a subharmonic and that
//! noise riding the pitch. It draws nothing from the shared noise stream.

use bf_core::dmath::{self, PI};
use bf_core::rng::Generator;
use bf_sound::dsp::{self, FilterKind};
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::{PpNoise, Voice};

/// The BASS CLARINET's register in `pp_bridge`'s roster (`('BASS CLARINET', 'bassclarinet_lowpoly', 'ball', (34,
/// 70), 0, 1.0)`).
pub const REGISTER: (i32, i32) = (34, 70);

/// `FORMANTS`: `(centre Hz, width Hz, gain)` of the two formant bumps.
const FORMANTS: [(f64, f64, f64); 2] = [(560.0, 260.0, 1.4), (1450.0, 500.0, 0.35)];

/// `bass_reed`'s `bright` (its default: no caller changes it).
const BRIGHT: f64 = 0.45;
/// `bass_reed`'s `vib`.
const VIB: f64 = 0.0025;
/// `bass_reed`'s `vrate`.
const VRATE: f64 = 5.0;
/// `bass_reed`'s `odd`.
const ODD: f64 = 0.92;
/// `bass_reed`'s `att`.
const ATT: f64 = 0.035;
/// `bass_reed`'s `breath`.
const BREATH: f64 = 0.010;

/// The BASS CLARINET's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct BassClarinetVoice;

impl Voice for BassClarinetVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "BASS CLARINET"
    }

    /// Down to -0.16 s: the tone is `int((dur + .16) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.16)
    }
}

/// `bassclarinet_lowpoly.synth` for the one note `pp_bridge.voice` asks: `bass_reed(_midi_hz(m), dur, v, growl=.25
/// if m < 45 else 0.0)`, at 44,100 Hz, before it is resampled.
///
/// # Panics
/// As [`bass_reed`].
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    let growl = if midi < 45.0 { 0.25 } else { 0.0 };
    bass_reed(pp::midi_hz(midi), dur, vel, growl)
}

/// `bass_reed(f, dur, v, growl=growl)`: the chalumeau tone at `f` Hz held for `dur` seconds at velocity `v`:
/// `int((dur + .16) * SR)` samples at 44,100 Hz.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or below -0.16 s (a negative number of samples); `f * 10` is not a
/// number a generator can be seeded with.
pub fn bass_reed(f: f64, dur: f64, v: f64, growl: f64) -> Vec<f64> {
    let n = pp::len(dur + 0.16);
    let t = pp::tt(n);
    // env = np.minimum(1, t / att) ** 1.3 * np.where(t < dur, 1, np.exp(-(t - dur) / .05)) * (.9 + .1 * np.exp(-t * 4))
    let env: Vec<f64> = t
        .iter()
        .map(|&t| {
            let rise = if t / ATT < 1.0 { t / ATT } else { 1.0 };
            let hold = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.05) };
            dmath::pow(rise, 1.3) * hold * (0.9 + 0.1 * dmath::exp(-t * 4.0))
        })
        .collect();
    // fr = f * (1 - .015 * np.exp(-t / .04)) * (1 + vib * np.sin(2 * np.pi * vrate * t) * np.clip((t - .18) / .25, 0, 1))
    let wobble = 2.0 * PI * VRATE;
    let fr: Vec<f64> = t
        .iter()
        .map(|&t| {
            let scooped = f * (1.0 - 0.015 * dmath::exp(-t / 0.04));
            scooped * (1.0 + VIB * dmath::sin(wobble * t) * ((t - 0.18) / 0.25).clamp(0.0, 1.0))
        })
        .collect();
    // ph = 2 * np.pi * np.cumsum(fr) / SR
    let ph: Vec<f64> = dsp::cumsum(&fr).iter().map(|&c| 2.0 * PI * c / SRF).collect();
    let (mut dark, mut brt) = (vec![0.0; n], vec![0.0; n]);
    let (mut sd, mut sb) = (0.0, 0.0);
    for k in 1..48 {
        let kf = f64::from(k);
        let fk = f * kf;
        if fk > 7500.0 {
            break;
        }
        // base = (1 / k ** 1.05) * ((1 - odd) if k % 2 == 0 else 1) * (.45 + sum(g * math.exp(-((fk - c) / w) ** 2)
        //                                                                  for c, w, g in FORMANTS))
        let even = if k % 2 == 0 { 1.0 - ODD } else { 1.0 };
        let bumps = FORMANTS.iter().fold(0.0, |acc, &(c, w, g)| {
            let q = (fk - c) / w;
            acc + g * dmath::exp(-(q * q))
        });
        let base = (1.0 / dmath::pow(kf, 1.05)) * even * (0.45 + bumps);
        let a1 = base * dmath::exp(-kf / 3.0);
        let a2 = base * dmath::exp(-kf / (4.0 + 9.0 * BRIGHT));
        // s_ = np.sin(k * ph + .3 * k)
        let off = 0.3 * kf;
        for i in 0..n {
            let s = dmath::sin(kf * ph[i] + off);
            dark[i] += a1 * s;
            brt[i] += a2 * s;
        }
        sd += a1;
        sb += a2;
    }
    // rng = np.random.default_rng(int(f * 10) % 100003); nz = sosfilt(butter(2, [700, 3200], 'band', fs=SR,
    // output='sos'), rng.standard_normal(n))
    let tenth = f * 10.0;
    assert!(
        tenth.is_finite() && tenth > -1.0,
        "cannot seed a generator with {tenth}"
    );
    let mut rng = Generator::new((tenth as u64) % 100_003);
    let white: Vec<f64> = (0..n).map(|_| rng.standard_normal()).collect();
    let sos = dsp::butter_sos(FilterKind::Bp, &[700.0, 3200.0], 2, SRF);
    let nz = dsp::sosfilt(&sos, &white);
    (0..n)
        .map(|i| {
            // mb = np.clip(env * v, 0, 1) ** 1.2; x = dark / sd * (1 - mb) + brt / sb * mb
            let mb = dmath::pow((env[i] * v).clamp(0.0, 1.0), 1.2);
            let mut x = dark[i] / sd * (1.0 - mb) + brt[i] / sb * mb;
            // x = x + nz * breath * 3.0
            x += nz[i] * BREATH * 3.0;
            if growl > 0.0 {
                // x = x * (1 + growl * .55 * np.sin(ph * .5)) + growl * .25 * nz * np.abs(np.sin(ph))
                x = x * (1.0 + growl * 0.55 * dmath::sin(ph[i] * 0.5)) + growl * 0.25 * nz[i] * dmath::sin(ph[i]).abs();
            }
            x * env[i] * v
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tone_has_its_length_its_attack_and_its_noise() {
        let x = bass_reed(110.0, 0.5, 0.9, 0.0);
        assert_eq!(x.len(), pp::len(0.66));
        // it starts from silence (the attack)
        assert_eq!(x[0], 0.0);
        // its breath is a generator of its own, seeded by the pitch: the same note twice is the same samples
        assert_eq!(x, bass_reed(110.0, 0.5, 0.9, 0.0));
        assert_ne!(x, bass_reed(110.5, 0.5, 0.9, 0.0));
        // the growl changes the tone
        assert_ne!(x, bass_reed(110.0, 0.5, 0.9, 0.3));
        let peak = |a: &[f64]| a.iter().fold(0.0f64, |p, v| p.max(v.abs()));
        assert!(peak(&x[x.len() - 441..]) < 0.1 * peak(&x[8820..17640]));
    }

    #[test]
    fn the_voice_growls_below_midi_45_and_is_the_synth_at_48k() {
        assert_eq!(synth(44.0, 0.8, 0.2), bass_reed(pp::midi_hz(44.0), 0.2, 0.8, 0.25));
        assert_eq!(synth(45.0, 0.8, 0.2), bass_reed(pp::midi_hz(45.0), 0.2, 0.8, 0.0));
        let mut noise = PpNoise::start();
        let note = BassClarinetVoice.note(&mut noise, 50.0, 0.9, 0.25);
        assert_eq!(note, pp::to_48k(&synth(50.0, 0.9, 0.25)));
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert_eq!(
            (BassClarinetVoice.register(), BassClarinetVoice.name()),
            (REGISTER, "BASS CLARINET")
        );
        assert!(BassClarinetVoice.can_hold(-0.16) && !BassClarinetVoice.can_hold(-0.17));
    }
}
