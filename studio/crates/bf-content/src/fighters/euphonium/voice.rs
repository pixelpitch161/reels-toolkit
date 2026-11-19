//! The EUPHONIUM's voice: `pp_bridge.voice('EUPHONIUM', midi, vel, dur)`, and the synth its part plays directly.
//!
//! `pp_bridge` plays one note through `series/instrument-history/euphonium_lowpoly.py`'s `synth`, which is
//! `euph(_midi_hz(m), dur, v)` with the tone's defaults ([`euph`]): a conical tenor-tuba bore, every harmonic, a
//! strong fundamental, the upper partials rolling off early, a soft attack, a gentle lip scoop and a slow, singing
//! vibrato; louder is brighter. The note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! The voice draws no noise: the shared stream is left where it was.

use bf_core::dmath::{self, PI};
use bf_sound::dsp;
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::{PpNoise, Voice};

/// The EUPHONIUM's register in `pp_bridge`'s roster (`('EUPHONIUM', 'euphonium_lowpoly', 'ball', (46, 77), 0,
/// 1.0)`). As a fighter it plays from 50 to 74; `kit.music.inst_gain` measures the voice on the middle of this one.
pub const REGISTER: (i32, i32) = (46, 77);

/// The EUPHONIUM's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct EuphoniumVoice;

impl Voice for EuphoniumVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&euph(pp::midi_hz(midi), dur, vel))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "EUPHONIUM"
    }

    /// Down to -0.16 s: the tone is `int((dur + .16) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.16)
    }
}

/// `np.clip(x, 0, 1)` on one value (numpy's rule: a NaN stays NaN).
fn clip01(x: f64) -> f64 {
    if x.is_nan() { x } else { x.clamp(0.0, 1.0) }
}

/// `np.minimum(1, x)` on one value (a NaN stays NaN).
fn min1(x: f64) -> f64 {
    if x.is_nan() || x < 1.0 { x } else { 1.0 }
}

/// `euph(f, dur, v)` with its defaults (`bright=.45, vib=.0045, vrate=5.0, scoop=.018, att=.04`): `int((dur + .16) *
/// SR)` samples at 44,100 Hz of a note at `f` Hz held for `dur` seconds at velocity `v`: up to 35 harmonics (those up
/// to 7 kHz) summed with a dark and a bright set of weights (the bright set with a touch of brass ring near 1.1 kHz),
/// blended by the note's own level; a scoop up into the pitch, a vibrato from 0.22 s, a 40 ms attack and a 60 ms
/// release after the held length.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or so far below -0.16 s that the note has a negative number of
/// samples.
pub fn euph(f: f64, dur: f64, v: f64) -> Vec<f64> {
    let (bright, vib, vrate, scoop, att) = (0.45, 0.0045, 5.0, 0.018, 0.04);
    let n = pp::len(dur + 0.16);
    let t = pp::tt(n);
    // env = minimum(1, t / att) ** 1.4 * where(t < dur, 1, exp(-(t - dur) / .06)) * (.9 + .1 * exp(-t * 3))
    let env: Vec<f64> = t
        .iter()
        .map(|&t| {
            let hold = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.06) };
            dmath::pow(min1(t / att), 1.4) * hold * (0.9 + 0.1 * dmath::exp(-t * 3.0))
        })
        .collect();
    // fr = f * (1 - scoop * exp(-t / .03)) * (1 + vib * sin(2 * pi * vrate * t) * clip((t - .22) / .3, 0, 1))
    let wobble = 2.0 * PI * vrate;
    let fr: Vec<f64> = t
        .iter()
        .map(|&t| {
            f * (1.0 - scoop * dmath::exp(-t / 0.03)) * (1.0 + vib * dmath::sin(wobble * t) * clip01((t - 0.22) / 0.3))
        })
        .collect();
    // ph = 2 * pi * cumsum(fr) / SR
    let ph: Vec<f64> = dsp::cumsum(&fr).iter().map(|&c| 2.0 * PI * c / SRF).collect();
    let (mut dark, mut brt) = (vec![0.0; n], vec![0.0; n]);
    let (mut sd, mut sb) = (0.0, 0.0);
    for k in 1..36 {
        let kf = f64::from(k);
        let fk = f * kf;
        if fk > 7000.0 {
            break;
        }
        // a1 = math.exp(-k / 1.9) / k ** .9 * (1.25 if k == 1 else 1.0)
        let a1 = dmath::exp(-kf / 1.9) / dmath::pow(kf, 0.9) * if k == 1 { 1.25 } else { 1.0 };
        let mut a2 = dmath::exp(-kf / (3.2 + 5.5 * bright)) / dmath::pow(kf, 0.8) * if k == 1 { 1.1 } else { 1.0 };
        // a touch of brass "ring" when pushed: a2 *= 1 + .5 * math.exp(-((fk - 1100) / 500) ** 2)
        a2 *= 1.0 + 0.5 * dmath::exp(-dmath::pow((fk - 1100.0) / 500.0, 2.0));
        let shift = 0.2 * kf;
        for i in 0..n {
            // s_ = np.sin(k * ph + .2 * k)
            let s = dmath::sin(kf * ph[i] + shift);
            dark[i] += a1 * s;
            brt[i] += a2 * s;
        }
        sd += a1;
        sb += a2;
    }
    (0..n)
        .map(|i| {
            // mb = clip(env * v, 0, 1) ** 1.35; x = dark / sd * (1 - mb) + brt / sb * mb; x * env * v
            let mb = dmath::pow(clip01(env[i] * v), 1.35);
            let x = dark[i] / sd * (1.0 - mb) + brt[i] / sb * mb;
            x * env[i] * v
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_voice_draws_no_noise_and_is_the_synth_at_48k() {
        let mut noise = PpNoise::start();
        let note = EuphoniumVoice.note(&mut noise, 62.0, 0.8, 0.25);
        let raw = euph(pp::midi_hz(62.0), 0.25, 0.8);
        assert_eq!(note, pp::to_48k(&raw));
        // int((dur + .16) * 44100) samples
        assert_eq!(raw.len(), 18081);
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert_eq!(
            (EuphoniumVoice.register(), EuphoniumVoice.name()),
            (REGISTER, "EUPHONIUM")
        );
        assert!(EuphoniumVoice.can_hold(0.0) && EuphoniumVoice.can_hold(-0.16));
        assert!(!EuphoniumVoice.can_hold(-0.2) && !EuphoniumVoice.can_hold(f64::NAN));
        assert_eq!(raw[0], 0.0);
        let peak = raw.iter().fold(0.0f64, |p, v| p.max(v.abs()));
        assert!((0.2..1.2).contains(&peak), "{peak}");
    }
}
