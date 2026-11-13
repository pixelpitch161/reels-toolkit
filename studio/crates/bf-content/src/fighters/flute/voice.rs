//! The FLUTE's voice: `pp_bridge.voice('FLUTE', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/flute60_lowpoly.py`'s `synth` on the scene its
//! roster selects for the FLUTE, the module's first: the modern flute (key `modern`). That is `flute_tone` with
//! the tone `TONES['modern']` (`bright=.35, vib=.0055, chiff=.3`, every other argument at its default, `seed=0`):
//! a sine with three quieter harmonics, a vibrato that comes in after 0.12 s, a 40 ms attack, a 50 ms release
//! after the held length, and a "chiff" of low-passed breath noise on the attack; then, because the scene is
//! the modern one, two echoes of the whole note, 0.75 of a beat of that video (112 BPM) and twice that later, at
//! 0.3 and 0.12 of its level. The note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! The breath is `drums60_lowpoly.lp(nz(n), 2500)`: `n` normals from the series' shared noise stream, one block
//! per note, drawn after the tone and its envelope are computed (nothing else draws).
//!
//! `flute_tone`'s branches for `scoop`, `wobble` and `buzz` are not ported: the modern tone leaves all three at
//! 0, and Python skips them then.

use bf_core::dmath::{self, PI};
use bf_sound::dsp;
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::{PpNoise, Voice};

/// The FLUTE's register in `pp_bridge`'s roster (`('FLUTE', 'flute60_lowpoly', None, (72, 93), ...)`).
pub const REGISTER: (i32, i32) = (72, 93);

/// `flute60_lowpoly.BPM`: the tempo of the flute's video, which sets the echoes' delay.
const BPM: f64 = 112.0;
/// `TONES['modern']['bright']`: the level of the harmonics.
const BRIGHT: f64 = 0.35;
/// `TONES['modern']['vib']`: the vibrato's depth, as a fraction of the frequency.
const VIB: f64 = 0.0055;
/// `TONES['modern']['chiff']`: the level of the breath on the attack.
const CHIFF: f64 = 0.3;
/// `flute_tone`'s default `attack`, in seconds.
const ATTACK: f64 = 0.04;

/// The FLUTE's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct FluteVoice;

impl Voice for FluteVoice {
    fn note(&self, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(noise, midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "FLUTE"
    }

    /// Down to -0.18 s: the tone is `int((dur + .18) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.18)
    }
}

/// `flute_tone(f, dur, v, bright=BRIGHT, vib=VIB, chiff=CHIFF)`: the tone at `f` Hz held for `dur` seconds at
/// velocity `v`, `int((dur + .18) * SR)` samples at 44,100 Hz.
fn flute_tone(noise: &mut PpNoise, f: f64, dur: f64, v: f64) -> Vec<f64> {
    let n = pp::len(dur + 0.18);
    let t = pp::tt(n);
    // fr = f * (1 + vib * sin(2 pi 5.2 t) * clip((t - .12) / .2, 0, 1))
    let wobble = 2.0 * PI * 5.2;
    let fr: Vec<f64> = t
        .iter()
        .map(|&t| f * (1.0 + VIB * dmath::sin(wobble * t) * ((t - 0.12) / 0.2).clamp(0.0, 1.0)))
        .collect();
    let ph: Vec<f64> = dsp::cumsum(&fr).iter().map(|&c| 2.0 * PI * c / SRF).collect();
    // The one draw of the note, after the tone and before the breath's filter.
    let breath = pp::lp(&noise.nz(n), 2500.0);
    (0..n)
        .map(|i| {
            let (t, ph) = (t[i], ph[i]);
            let x = dmath::sin(ph)
                + BRIGHT * (0.45 * dmath::sin(2.0 * ph) + 0.22 * dmath::sin(3.0 * ph) + 0.1 * dmath::sin(4.0 * ph));
            // np.minimum(1, t / attack) * np.where(t < dur, 1, np.exp(-(t - dur) / .05))
            let rise = if t / ATTACK < 1.0 { t / ATTACK } else { 1.0 };
            let hold = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.05) };
            let ch = breath[i] * dmath::exp(-t / 0.018) * CHIFF * 0.5;
            (x * (rise * hold) + ch) * v * 0.5
        })
        .collect()
}

/// `flute60_lowpoly.synth` for the one note `pp_bridge.voice` asks of the modern scene: the note at 44,100 Hz,
/// before it is resampled. `midi` is a whole number.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or below -0.18 s (a negative number of samples).
pub fn synth(noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    let x = flute_tone(noise, pp::midi_hz(midi), dur, vel);
    // The modern scene's echoes: dl = int(.75 * BEAT_S * SR), then the note again .3 and .12 as loud.
    let dl = pp::len(0.75 * (60.0 / BPM));
    let mut y = vec![0.0; x.len() + 2 * dl];
    // y[:len(x)] += x; y[dl:dl + len(x)] += .3 * x; y[2 * dl:2 * dl + len(x)] += .12 * x
    for (delay, level) in [(0, 1.0), (dl, 0.3), (2 * dl, 0.12)] {
        for (o, &v) in y[delay..].iter_mut().zip(&x) {
            *o += level * v;
        }
    }
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_echoes_are_three_quarters_of_a_beat_apart() {
        let dl = pp::len(0.75 * (60.0 / BPM));
        assert_eq!(dl, 17718);
        let mut noise = PpNoise::start();
        let y = synth(&mut noise, 76.0, 0.8, 0.12);
        // int((0.12 + .18) * 44100) samples of tone, and room for the two echoes.
        assert_eq!(y.len(), 13230 + 2 * dl);
        // The note is over before its first echo starts, so each echo is the note, quieter.
        assert!(y[13230..dl].iter().all(|v| *v == 0.0));
        for i in [0, 1, 500, 13229] {
            assert_eq!(y[dl + i], 0.3 * y[i], "sample {i}");
            assert_eq!(y[2 * dl + i], 0.12 * y[i], "sample {i}");
        }
        // It drew one block of 13,230 normals.
        let mut want = PpNoise::start();
        want.nz(13230);
        assert_eq!(noise.nz(4), want.nz(4));
        // A long note runs into its own echoes: they add up.
        let mut a = PpNoise::start();
        let long = synth(&mut a, 76.0, 0.8, 1.0);
        let n = pp::len(1.18);
        assert_eq!(long.len(), n + 2 * dl);
        let mut b = PpNoise::start();
        let tone = flute_tone(&mut b, pp::midi_hz(76.0), 1.0, 0.8);
        let i = 2 * dl + 100;
        assert_eq!(long[i], tone[i] + 0.3 * tone[i - dl] + 0.12 * tone[i - 2 * dl]);
        assert_eq!(long[n + 5], 0.0 + 0.3 * tone[n + 5 - dl] + 0.12 * tone[n + 5 - 2 * dl]);
    }

    #[test]
    fn the_tone_has_its_pitch_its_attack_and_its_breath() {
        let mut noise = PpNoise::start();
        let x = flute_tone(&mut noise, 440.0, 0.5, 1.0);
        // int((0.5 + .18) * 44100): the sum is a hair under 0.68, and the product is cut to 29,987, not 29,988.
        assert_eq!(x.len(), 29987);
        // The breath starts at once (the first sample is the low-passed noise alone), the tone fades in.
        let mut same = PpNoise::start();
        let breath = pp::lp(&same.nz(x.len()), 2500.0);
        assert_eq!(x[0], breath[0] * 1.0 * CHIFF * 0.5 * 1.0 * 0.5);
        // In the held part the level is about half the velocity.
        let peak = x[8820..17640].iter().fold(0.0f64, |p, v| p.max(v.abs()));
        assert!((0.45..0.55).contains(&peak), "{peak}");
        // 440 Hz: about 220 upward zero crossings in the half second after the attack.
        let ups = x[4410..26460].windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        assert!((218..=222).contains(&ups), "{ups}");
        // After the held length the note dies away within the 0.18 s it has left.
        let end = x[x.len() - 441..].iter().fold(0.0f64, |p, v| p.max(v.abs()));
        assert!(end < 0.03, "{end}");
        // The velocity is a plain factor (from the same noise).
        let mut again = PpNoise::start();
        let half = flute_tone(&mut again, 440.0, 0.5, 0.5);
        assert_eq!(half[3000], x[3000] / 1.0 * 0.5);
    }

    #[test]
    fn the_voice_is_the_synth_at_48k() {
        let (mut a, mut b) = (PpNoise::start(), PpNoise::start());
        let note = FluteVoice.note(&mut a, 84.0, 0.9, 0.25);
        let raw = synth(&mut b, 84.0, 0.9, 0.25);
        assert_eq!(note, pp::to_48k(&raw));
        assert_eq!(note.len(), (raw.len() * 160).div_ceil(147));
        assert_eq!((FluteVoice.register(), FluteVoice.name()), (REGISTER, "FLUTE"));
        // What it can hold is what Python does not raise for (-0.18 s is a tone of no samples: only the room
        // for the echoes is left; at -0.19 s numpy refuses).
        assert!(FluteVoice.can_hold(0.0) && FluteVoice.can_hold(-0.1) && FluteVoice.can_hold(-0.18));
        assert!(!FluteVoice.can_hold(-0.19) && !FluteVoice.can_hold(f64::NAN));
        let mut e = PpNoise::start();
        let none = FluteVoice.note(&mut e, 76.0, 0.8, -0.18);
        assert!(none.len() == 38570 && none.iter().all(|v| *v == 0.0));
        // A note of no held length is its attack and release: 0.18 s of tone, then the echoes.
        let mut c = PpNoise::start();
        assert_eq!(synth(&mut c, 76.0, 0.8, 0.0).len(), 7938 + 2 * 17718);
        // Far above the instrument the sine folds over, but the samples are numbers (Python's are too).
        let mut d = PpNoise::start();
        assert!(FluteVoice.note(&mut d, 200.0, 0.8, 0.1).iter().all(|v| v.is_finite()));
    }

    #[test]
    #[should_panic(expected = "negative dimensions are not allowed")]
    fn a_note_cannot_end_before_it_starts() {
        synth(&mut PpNoise::start(), 76.0, 0.8, -0.2);
    }
}
