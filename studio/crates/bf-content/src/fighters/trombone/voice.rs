//! The TROMBONE's voice: `pp_bridge.voice('TROMBONE', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/trombone60_lowpoly.py`'s `synth` on the scene
//! its roster selects for the TROMBONE, the module's first: the modern trombone (key `modern`). That is `brass2`
//! with the tone `TONE['modern']` (`bright=1.0, vib=.003, scoop=.03`, every other argument at its default): up
//! to 33 harmonics (those under 7 kHz) summed twice, with a dark and a bright set of weights, and blended by
//! the note's own level, so a loud note is brighter; a short scoop up into the pitch, a vibrato that comes in
//! after 0.18 s, a 30 ms attack and a 35 ms release after the held length. The note is made at 44,100 Hz and
//! resampled to 48,000 Hz.
//!
//! The voice draws no noise: the shared stream is left where it was.
//!
//! Not ported, because a lone note never takes them: the glide into the note (`gl`: `pp_bridge` plays the note
//! at beat 0 of a scene that lists no glides, so `gl_of` gives 0 and the bend factor is the constant 1), the
//! plunger (`wah`: 0 for the modern tone), and the `0.75` level of a note inside a chord (the lone note's chord
//! size is 1).
//!
//! A pitch whose fundamental is above 7 kHz (MIDI 117 and up) has no harmonic to sum: Python divides zero by
//! zero there and gives a note of NaN, and so does this.

use bf_core::dmath::{self, PI};
use bf_sound::dsp;
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::{PpNoise, Voice};

/// The TROMBONE's register in `pp_bridge`'s roster (`('TROMBONE', 'trombone60_lowpoly', None, (45, 67), ...)`).
/// As a fighter it plays from 40 to 65; `kit.music.inst_gain` measures the voice on the middle of this one.
pub const REGISTER: (i32, i32) = (45, 67);

/// `TONE['modern']['bright']`: how far the bright set of weights reaches up the harmonics.
const BRIGHT: f64 = 1.0;
/// `TONE['modern']['vib']`: the vibrato's depth, as a fraction of the frequency.
const VIB: f64 = 0.003;
/// `TONE['modern']['scoop']`: how far under the pitch the note starts, as a fraction of the frequency.
const SCOOP: f64 = 0.03;
/// `brass2`'s default `tilt`: the roll-off of the harmonics, `1 / k ** tilt`.
const TILT: f64 = 0.8;
/// `brass2`'s default `att`: the attack, in seconds.
const ATT: f64 = 0.03;

/// The TROMBONE's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct TromboneVoice;

impl Voice for TromboneVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "TROMBONE"
    }

    /// Down to -0.14 s: the tone is `int((dur + .14) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.14)
    }
}

/// `brass2(f, dur, vel, bright=BRIGHT, vib=VIB, scoop=SCOOP)` with no glide and no plunger: the tone at `f` Hz
/// held for `dur` seconds at velocity `vel`, `int((dur + .14) * SR)` samples at 44,100 Hz.
fn brass2(f: f64, dur: f64, vel: f64) -> Vec<f64> {
    let n = pp::len(dur + 0.14);
    let t = pp::tt(n);
    // env = minimum(1, t / att) ** 1.5 * where(t < dur, 1, exp(-(t - dur) / .035)) * (.88 + .12 * exp(-t * 3))
    let env: Vec<f64> = t
        .iter()
        .map(|&t| {
            let rise = if t / ATT < 1.0 { t / ATT } else { 1.0 };
            let hold = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.035) };
            dmath::pow(rise, 1.5) * hold * (0.88 + 0.12 * dmath::exp(-t * 3.0))
        })
        .collect();
    // fr = f * bendc * (1 - scoop * exp(-t / .025)) * (1 + vib * sin(2 pi 5.4 t) * clip((t - .18) / .2, 0, 1)),
    // with bendc = 1.
    let wobble = 2.0 * PI * 5.4;
    let base = f * 1.0;
    let fr: Vec<f64> = t
        .iter()
        .map(|&t| {
            base * (1.0 - SCOOP * dmath::exp(-t / 0.025))
                * (1.0 + VIB * dmath::sin(wobble * t) * ((t - 0.18) / 0.2).clamp(0.0, 1.0))
        })
        .collect();
    let ph: Vec<f64> = dsp::cumsum(&fr).iter().map(|&c| 2.0 * PI * c / SRF).collect();
    let (mut dark, mut brt) = (vec![0.0; n], vec![0.0; n]);
    let (mut sd, mut sb) = (0.0, 0.0);
    for k in 1..34 {
        let k = f64::from(k);
        if f * k > 7000.0 {
            break;
        }
        let a1 = dmath::exp(-k / 2.2) / dmath::pow(k, TILT);
        let a2 = dmath::exp(-k / (4.0 + 6.0 * BRIGHT)) / dmath::pow(k, TILT);
        for i in 0..n {
            let s = dmath::sin(k * ph[i]);
            dark[i] += a1 * s;
            brt[i] += a2 * s;
        }
        sd += a1;
        sb += a2;
    }
    (0..n)
        .map(|i| {
            // mb = clip(env * vel, 0, 1) ** 1.3: how much of the bright set
            let mb = dmath::pow((env[i] * vel).clamp(0.0, 1.0), 1.3);
            (dark[i] / sd * (1.0 - mb) + brt[i] / sb * mb) * env[i] * vel
        })
        .collect()
}

/// `trombone60_lowpoly.synth` for the one note `pp_bridge.voice` asks of the modern scene: the note at 44,100 Hz,
/// before it is resampled. `midi` is a whole number.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or so far below -0.14 s that the note has a negative number of
/// samples.
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    brass2(pp::midi_hz(midi), dur, vel)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(x: &[f64]) -> f64 {
        x.iter().fold(0.0f64, |p, v| p.max(v.abs()))
    }

    #[test]
    fn the_note_has_its_length_its_pitch_and_its_shape() {
        // int((dur + .14) * 44100) samples.
        assert_eq!(synth(52.0, 0.8, 0.12).len(), 11466);
        assert_eq!(synth(52.0, 0.8, 1.0).len(), 50274);
        // 441 Hz is a period of 100 samples.
        let x = brass2(441.0, 0.5, 0.9);
        assert_eq!(x.len(), pp::len(0.64));
        // It starts from nothing (the attack is a power of t) and has died away by its end.
        assert_eq!(x[0], 0.0);
        assert!(peak(&x[x.len() - 441..]) < 0.03, "{}", peak(&x[x.len() - 441..]));
        let level = peak(&x[4410..17640]);
        assert!((0.3..1.2).contains(&level), "{level}");
        // Between the scoop (over by 0.1 s) and the vibrato (from 0.18 s) the wave repeats every 100 samples,
        // and not every 50.
        let apart = |lag: usize| (5292..7400).fold(0.0f64, |d, i| d.max((x[i + lag] - x[i]).abs()));
        assert!(apart(100) < 0.03 * level, "{} of {level}", apart(100));
        assert!(apart(50) > 0.5 * level, "{} of {level}", apart(50));
    }

    #[test]
    fn a_loud_note_is_brighter_not_only_louder() {
        // The blend of the dark and the bright harmonics follows the level, so the wave changes shape: the loud
        // note is not the soft one times a constant.
        let (soft, loud) = (brass2(110.0, 0.5, 0.3), brass2(110.0, 0.5, 1.0));
        let ratio: Vec<f64> = (9000..9400).map(|i| loud[i] / soft[i]).collect();
        let (lo, hi) = ratio.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), r| {
            (lo.min(*r), hi.max(*r))
        });
        assert!(hi - lo > 0.5, "{lo} {hi}");
    }

    #[test]
    fn the_voice_draws_no_noise_and_is_the_synth_at_48k() {
        let mut noise = PpNoise::start();
        let note = TromboneVoice.note(&mut noise, 40.0, 0.9, 0.25);
        let raw = synth(40.0, 0.9, 0.25);
        assert_eq!(note, pp::to_48k(&raw));
        assert_eq!((raw.len(), note.len()), (17199, 18720));
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert_eq!((TromboneVoice.register(), TromboneVoice.name()), (REGISTER, "TROMBONE"));
        // What it can hold is what Python does not raise for.
        assert!(TromboneVoice.can_hold(0.0) && TromboneVoice.can_hold(-0.1) && TromboneVoice.can_hold(-0.14));
        assert!(!TromboneVoice.can_hold(-0.2) && !TromboneVoice.can_hold(f64::INFINITY));
        // A held length of -0.1 s leaves 0.04 s of release: 1,764 samples, 1,920 at 48 kHz.
        assert_eq!(TromboneVoice.note(&mut noise, 52.0, 0.8, -0.1).len(), 1920);
        // A note of no held length is its attack cut short and its release.
        assert_eq!(synth(52.0, 0.8, 0.0).len(), 6174);
        // A note that ends 0.14 s before it starts has no samples at all (Python's is empty too).
        assert!(synth(52.0, 0.8, -0.14).is_empty());
        assert!(TromboneVoice.note(&mut noise, 52.0, 0.8, -0.14).is_empty());
    }

    #[test]
    fn a_pitch_with_no_harmonic_under_7_khz_is_not_a_number() {
        // MIDI 116 is 6,645 Hz: one harmonic. MIDI 117 is 7,040 Hz: none, and 0 / 0 (Python's note is NaN too).
        assert!(synth(116.0, 0.8, 0.2).iter().all(|v| v.is_finite()));
        assert!(peak(&synth(116.0, 0.8, 0.2)) > 0.1);
        let none = synth(117.0, 0.8, 0.2);
        assert_eq!(none.len(), 14994);
        assert!(none.iter().all(|v| v.is_nan()));
        // Far below the instrument every harmonic fits and the note is only very low.
        assert!(synth(-500.0, 0.8, 0.2).iter().all(|v| v.is_finite()));
    }

    #[test]
    #[should_panic(expected = "negative dimensions are not allowed")]
    fn a_note_cannot_end_before_it_starts() {
        synth(52.0, 0.8, -0.2);
    }
}
