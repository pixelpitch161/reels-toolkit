//! TURNTABLES's voice: `pp_bridge.voice('TURNTABLES', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/turntables60_lowpoly.py`'s `synth` on the scene its
//! roster selects for the TURNTABLES: the first, `modern` (the Technics deck; key `None`). A note there (MIDI 12 and
//! up) is `machine(v_organ(f, dur, v, rel), 'modern')`: the garage organ stab, an additive tone of five harmonics
//! (1, 2, 3, 4, 6 with levels 1, 0.55, 0.35, 0.25, 0.12; a harmonic at or above 15 kHz left out), a click of noise
//! over its first 3 ms (0.4 x normals from the module's own generator, `_RNG`: [`ModuleNoise::turntables`]), an
//! envelope `adsr(t, dur, .002, .22, .4, rel)`, then `tanh(1.2 x) * v * 0.6`. `rel` is 0.06 for the one event
//! `pp_bridge` makes (its start, 0.0, is before the scene's twelfth beat). `machine` passes the `modern` scene's
//! notes through unchanged. The note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! MIDI notes below 12 are the scene's scratch marks (`scratch_audio` through the deck's motions); the kit plays
//! none through the voice (the part makes its own scratches) and they are not ported: [`synth`] panics on one.
//!
//! [`ModuleNoise::turntables`]: bf_sound::voice::ModuleNoise::turntables

use bf_core::dmath::{self, PI};
use bf_sound::voice::pp;
use bf_sound::voice::{PpNoise, Voice};

/// The TURNTABLES's register in `pp_bridge`'s roster (`('TURNTABLES', 'turntables60_lowpoly', None, (50, 74), ...)`).
pub const REGISTER: (i32, i32) = (50, 74);

/// `v_organ`'s release for `pp_bridge`'s one event (`.9 if nt >= 12 * BEAT_S - 1e-6 else .06`, `nt = 0`).
const REL: f64 = 0.06;
/// `v_organ`'s harmonics: (multiple, level).
const HARMONICS: [(f64, f64); 5] = [(1.0, 1.0), (2.0, 0.55), (3.0, 0.35), (4.0, 0.25), (6.0, 0.12)];

/// The TURNTABLES's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct TurntablesVoice;

impl Voice for TurntablesVoice {
    fn note(&self, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(noise, midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "TURNTABLES"
    }

    /// Down to -0.3 s: the note is `int((dur + 5 * rel) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + REL * 5.0)
    }
}

/// `adsr(t, dur, a, d, s, r)` of `turntables60_lowpoly`.
fn adsr(t: f64, dur: f64, a: f64, d: f64, s: f64, r: f64) -> f64 {
    // e = np.where(t < a, t / a, s + (1 - s) * np.exp(-(t - a) / d))
    let e = if t < a {
        t / a
    } else {
        s + (1.0 - s) * dmath::exp(-(t - a) / d)
    };
    // e * np.where(t < dur, 1., np.exp(-(t - dur) / r))
    e * if t < dur { 1.0 } else { dmath::exp(-(t - dur) / r) }
}

/// `v_organ(f, dur, v, rel)`: the garage organ stab, at 44,100 Hz.
fn v_organ(noise: &mut PpNoise, f: f64, dur: f64, v: f64, rel: f64) -> Vec<f64> {
    let n = pp::len(dur + rel * 5.0);
    let t = pp::tt(n);
    // ph = 2 * np.pi * f * t
    let w = 2.0 * PI * f;
    // x = sum(a * np.sin(k * ph) for k, a in ... if k * f < 15000): 0 + the first term, then the rest in order
    let mut x = vec![0.0; n];
    for &(k, a) in &HARMONICS {
        if k * f < 15000.0 {
            for (o, &t) in x.iter_mut().zip(&t) {
                *o += a * dmath::sin(k * (w * t));
            }
        }
    }
    // x[:int(.003 * SR)] += .4 * _RNG.normal(0, 1, int(.003 * SR))
    let k = pp::len(0.003);
    let mut nz = vec![0.0; k];
    noise.1.turntables.fill_normal(0.0, 1.0, &mut nz);
    for (o, z) in x.iter_mut().zip(&nz) {
        *o += 0.4 * z;
    }
    // np.tanh(1.2 * x * adsr(t, dur, .002, .22, .4, rel)) * v * .6
    x.iter()
        .zip(&t)
        .map(|(&x, &t)| dmath::np_tanh(1.2 * x * adsr(t, dur, 0.002, 0.22, 0.4, rel)) * v * 0.6)
        .collect()
}

/// `turntables60_lowpoly.synth` for the one note `pp_bridge.voice` asks of the `modern` scene: the note at 44,100
/// Hz, before it is resampled.
///
/// # Panics
/// On a scratch mark (MIDI below 12, not ported), and where Python raises: `dur` is not a number, or so far below
/// zero that the note has a negative number of samples.
pub fn synth(noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    assert!(
        midi >= 12.0,
        "TURNTABLES: MIDI {midi} is one of the deck's scratch marks, which no part plays through the voice"
    );
    // machine(x, 'modern') is x
    v_organ(noise, pp::midi_hz(midi), dur, vel, REL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_has_its_length_and_draws_its_click() {
        let mut a = PpNoise::start();
        let raw = synth(&mut a, 62.0, 0.8, 0.2);
        // int((0.2 + 0.3) * 44100)
        assert_eq!(raw.len(), 22_050);
        assert!(raw.iter().all(|s| s.abs() <= 0.8 * 0.6));
        // 132 normals from the module's own generator, none from the shared stream
        let mut b = PpNoise::start();
        let mut skip = vec![0.0; 132];
        b.1.turntables.fill_normal(0.0, 1.0, &mut skip);
        assert_eq!(a.1.turntables.normal(0.0, 1.0), b.1.turntables.normal(0.0, 1.0));
        assert_eq!(a.nz(4), PpNoise::start().nz(4));
        // the next note's click is other noise: the same note twice differs only in its first 3 ms
        let again = synth(&mut a, 62.0, 0.8, 0.2);
        assert_ne!(raw[..132], again[..132]);
        assert_eq!(raw[200..], again[200..]);
    }

    #[test]
    fn what_it_can_hold_is_what_python_does_not_raise_for() {
        assert!(TurntablesVoice.can_hold(0.0) && TurntablesVoice.can_hold(-0.3));
        assert!(!TurntablesVoice.can_hold(-0.31) && !TurntablesVoice.can_hold(f64::NAN));
        assert_eq!(
            (TurntablesVoice.register(), TurntablesVoice.name()),
            (REGISTER, "TURNTABLES")
        );
    }
}
