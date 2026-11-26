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
use bf_sound::voice::real::{Correction, Fill, Rework, Tail};
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

/// The BASS CLARINET's added overtones: the harmonics its own note barely has, laid in at a real clarinet's level (dB
/// under the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (34, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 0.1, -99.0]),
    (35, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 0.1, -99.0]),
    (36, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 0.1, -99.0]),
    (37, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 0.1, -99.0]),
    (38, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.0, -99.0]),
    (39, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.2, -99.0]),
    (40, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.4, -27.5]),
    (41, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.6, -26.1]),
    (42, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.9, -24.6]),
    (43, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.2, -23.2]),
    (44, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.5, -21.8]),
    (45, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.7, -20.7]),
    (46, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.8, -19.7]),
    (47, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.8, -18.8]),
    (48, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.7, -18.1]),
    (49, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.5, -17.4]),
    (50, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -1.2, -17.0]),
    (51, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.8, -16.6]),
    (52, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -0.3, -16.3]),
    (53, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 0.1, -16.0]),
    (54, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 1.5, -10.4]),
    (55, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 1.5, -10.0]),
    (56, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, 1.5, -9.8]),
    (57, [-99.0, -99.0, -99.0, -99.0, 0.1, -16.2, -7.1, -28.2]),
    (58, [-99.0, -99.0, -99.0, -99.0, 0.2, -15.9, -7.0, -28.0]),
    (59, [-99.0, -99.0, -99.0, -99.0, 0.4, -15.7, -6.9, -27.7]),
    (60, [-99.0, -99.0, -99.0, -99.0, 5.8, -20.1, 3.5, -9.2]),
    (61, [-99.0, -99.0, -99.0, -99.0, 5.8, -19.9, 3.5, -9.2]),
    (62, [-99.0, -99.0, -99.0, -99.0, 5.9, -19.7, 3.5, -9.2]),
    (63, [-99.0, -99.0, -0.7, -7.2, -99.0, -24.1, -0.4, -10.5]),
    (64, [-99.0, -99.0, -0.7, -7.2, -8.3, -24.1, -0.4, -10.5]),
    (65, [-99.0, -99.0, -0.7, -7.2, -8.2, -24.1, -0.4, -10.5]),
    (66, [-99.0, -99.0, -99.0, -18.7, 2.8, -0.7, -4.6, -2.8]),
    (67, [-99.0, -99.0, -99.0, -18.8, 2.8, -0.7, -4.5, -2.7]),
    (68, [-99.0, -99.0, -99.0, -18.9, 2.8, -0.7, -4.5, -2.6]),
    (69, [-99.0, -99.0, -0.6, -13.2, -4.2, -18.7, -12.4, -22.5]),
    (70, [-99.0, -99.0, -0.6, -13.2, -4.2, -18.7, -12.4, -22.4]),
];

/// The BASS CLARINET's overtone correction toward a real clarinet (FluidR3 has no bass clarinet): gains in dB for its first eight harmonics, note by
/// note (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "BASS CLARINET"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (34, [0.5, -6.9, 10.1, -11.6, 12.2, 2.5, 0.3, 15.0]),
    (35, [0.5, -6.5, 9.8, -11.8, 11.7, 1.8, 0.2, 14.3]),
    (36, [0.6, -6.3, 8.8, -11.9, 10.7, 1.0, 0.1, 11.1]),
    (37, [0.6, -5.8, 8.7, -11.5, 10.4, 0.5, 0.1, 11.3]),
    (38, [0.8, -5.2, 9.1, -11.4, 10.7, 0.3, 0.5, 13.5]),
    (39, [0.7, -5.0, 8.8, -11.3, 10.2, -0.3, 0.4, 13.8]),
    (40, [0.6, -5.8, 6.3, -11.7, 7.7, -2.7, -0.4, 0.6]),
    (41, [0.3, -6.0, 5.7, -11.6, 7.0, -3.6, -0.6, -0.0]),
    (42, [0.1, -6.3, 5.1, -11.5, 6.6, -4.5, -0.9, -0.0]),
    (43, [0.0, -6.5, 4.6, -11.4, 6.3, -4.9, -0.8, 0.0]),
    (44, [0.0, -6.7, 4.2, -10.9, 6.2, -4.8, -0.8, -0.1]),
    (45, [0.0, -6.8, 3.8, -11.0, 6.3, -4.7, -0.6, 0.2]),
    (46, [0.0, -7.2, 3.4, -10.7, 6.6, -4.3, -0.5, 0.1]),
    (47, [0.0, -7.6, 3.0, -10.4, 7.1, -2.8, -0.5, 0.1]),
    (48, [0.0, -8.0, 2.7, -10.0, 7.8, -2.3, -0.4, 0.1]),
    (49, [0.0, -8.4, 2.3, -9.8, 8.8, 0.3, -0.5, 0.0]),
    (50, [0.0, -8.9, 2.1, -9.3, 10.1, 2.9, -0.5, 0.1]),
    (51, [0.0, -9.2, 1.9, -8.4, 11.6, 3.1, -0.5, -0.0]),
    (52, [0.0, -9.5, 1.9, -7.6, 13.2, 4.9, -0.5, 0.1]),
    (53, [0.6, -9.3, 2.6, -6.0, 15.5, 4.0, 0.0, 0.6]),
    (54, [0.8, -9.8, -4.4, 0.4, 10.6, 12.9, 0.0, 0.8]),
    (55, [0.9, -10.3, -4.1, 2.6, 11.7, 14.5, 0.0, 0.8]),
    (56, [0.7, -11.0, -3.9, 4.1, 12.0, 10.7, 0.0, 0.7]),
    (57, [-0.3, -8.1, -21.8, -6.5, 0.4, 0.3, -0.7, 0.1]),
    (58, [-0.5, -8.0, -21.2, -5.5, 0.2, 0.2, -0.8, -0.2]),
    (59, [-0.6, -7.6, -20.2, -4.6, 0.0, 0.1, -0.8, -0.1]),
    (60, [0.2, -14.1, 6.7, 6.7, 0.0, 0.5, 0.2, 0.5]),
    (61, [0.2, -13.8, 8.7, 6.2, 0.0, 0.4, 0.2, 0.5]),
    (62, [0.2, -13.3, 10.6, 5.9, 0.0, 0.5, 0.3, 0.6]),
    (63, [0.2, -13.5, -0.3, -0.2, 15.1, 0.2, 0.3, 0.7]),
    (64, [0.1, -12.4, -0.3, 0.1, 0.2, 0.6, 0.4, 0.7]),
    (65, [0.0, -10.9, -0.3, 0.1, 0.2, 0.8, 0.5, 0.8]),
    (66, [-0.3, 1.3, 12.8, -1.2, 0.0, 0.3, 0.3, 0.7]),
    (67, [-0.4, 3.3, 13.1, -1.0, 0.0, 0.3, 0.4, 0.8]),
    (68, [-0.5, 5.3, 13.2, -1.1, 0.0, 0.3, 0.4, 0.9]),
    (69, [0.0, -13.6, 0.1, 0.2, 0.5, 0.8, 0.6, 1.2]),
    (70, [0.0, -11.4, 0.0, 0.3, 0.6, 0.8, 0.8, 1.5]),
];

/// Its real instrument above the eighth harmonic (sound upgrade, stage 3, S1): the 9th to 12th harmonics beside the
/// 5th to 8th, which [`REWORK`] carries its notes on with (`references -- tail`).
#[rustfmt::skip]
pub const TAIL: &Tail = &[
    (34, [-6.3, -22.2, -5.7, -14.7]),
    (35, [-6.3, -22.3, -5.5, -14.2]),
    (36, [-6.2, -22.4, -5.3, -13.8]),
    (37, [-6.2, -22.4, -5.1, -13.4]),
    (38, [-6.1, -22.4, -4.9, -13.0]),
    (39, [-6.1, -22.5, -4.8, -12.8]),
    (40, [-6.1, -22.7, -4.8, -12.6]),
    (41, [-6.1, -23.0, -4.8, -12.5]),
    (42, [-6.1, -23.4, -4.9, -12.5]),
    (43, [-6.2, -23.7, -5.1, -12.6]),
    (44, [-6.4, -24.0, -5.4, -12.9]),
    (45, [-6.6, -24.2, -5.7, -13.3]),
    (46, [-6.8, -24.3, -6.1, -13.7]),
    (47, [-7.1, -24.4, -6.6, -14.1]),
    (48, [-7.5, -24.3, -7.1, -14.6]),
    (49, [-7.9, -24.2, -7.6, -15.1]),
    (50, [-8.2, -24.0, -8.1, -15.6]),
    (51, [-8.6, -23.9, -8.5, -16.1]),
    (52, [-8.9, -23.8, -8.9, -16.6]),
    (53, [-9.2, -23.8, -9.3, -17.0]),
    (54, [-2.6, -12.2, -4.4, -22.0]),
    (55, [-2.8, -11.9, -4.6, -21.9]),
    (56, [-3.0, -11.7, -4.7, -21.9]),
    (57, [-3.1, -5.6, -18.1, -36.4]),
    (58, [-3.1, -5.8, -18.2, -35.0]),
    (59, [-3.1, -6.0, -18.2, -33.3]),
    (60, [-10.0, -15.2, -10.5, -29.3]),
    (61, [-10.0, -15.2, -10.5, -29.3]),
    (62, [-10.1, -15.2, -10.5, -29.4]),
    (63, [-6.3, -18.6, -16.3, -15.6]),
    (64, [-6.3, -18.6, -16.3, -15.6]),
    (65, [-6.3, -18.6, -16.3, -15.6]),
    (66, [-19.3, -24.7, -30.3, -50.8]),
    (67, [-19.2, -24.7, -30.2, -50.9]),
    (68, [-19.2, -24.6, -30.2, -50.9]),
    (69, [-29.0, -27.9, -44.2, -46.6]),
    (70, [-29.0, -27.9, -44.2, -46.6]),
];

/// The Composed arrangement's BASS CLARINET (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added, a slower start (60 ms to full level) and a quicker stop (a 50 ms fade after its length), nearer the real one's.
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.06,
    stop: 0.05,
    keep_shape: false,
    tail: TAIL,
};

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

    fn rework(&self) -> Option<&'static Rework> {
        Some(&REWORK)
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
