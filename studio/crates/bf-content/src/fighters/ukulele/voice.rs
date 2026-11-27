//! UKULELE's instrument voice: `pp_bridge.voice('UKULELE', midi, vel, dur)`, the one note `ukulele60_lowpoly.synth`
//! makes on its default scene (key `modern`): a Karplus-Strong string with a fractional-delay loop filter (bright
//! 0.75, decay 0.9982, ringing `min(2.4, dur + 0.9)` s), its body resonance (`+ 0.66 * bp(x, 266, 532)`), a low-pass
//! at 6,500 Hz and a 60 ms release, times the velocity. Made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! Close to the Pixel Pitch voice by ear (ruling M4-R16), not bit for bit: the string's noise is the shared
//! generator seeded 0, as `ks(.., seed=idx)` is for the note's index 0.

use bf_core::rng::Generator;
use bf_sound::voice::engine2::plucked::PluckedParts;
use bf_sound::voice::engine2::space::{Home, Space};
use bf_sound::voice::engine2::{Instrument, Parts};
use bf_sound::voice::pp;
use bf_sound::voice::{PpNoise, Voice};

/// The UKULELE's register in `pp_bridge`'s roster (`('UKULELE', 'ukulele60_lowpoly', None, (60, 81), 0, 1.0)`).
pub const REGISTER: (i32, i32) = (60, 81);

/// The Composed arrangement's UKULELE (Voice Engine 2): a soprano ukulele built from how it works, on the
/// plucked-string engine (Aedan's pick U6: new fluorocarbon strings, the fingers, through its small box, a light
/// studio polish), with every technique a ukulele has, in a small room of its own
/// (`docs/studio/voice-engine-2-instruments.md`, "Ukulele"). Classic keeps the note below.
pub static ENGINE2: Instrument = Instrument::built(
    "UKULELE",
    Parts::Plucked(PluckedParts::U6),
    Space {
        width: 0.25,
        tilt: 0.0,
        home: Some(Home {
            length: 2.4,
            under: 15.0,
            hp: 120.0,
        }),
        amp: None,
    },
);

/// The UKULELE's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct UkuleleVoice;

impl Voice for UkuleleVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn engine2(&self) -> Option<&'static Instrument> {
        Some(&ENGINE2)
    }

    fn name(&self) -> &str {
        "UKULELE"
    }
}

/// `ks(f, dur, bright, decay, seed)` of `ukulele60_lowpoly`: Karplus-Strong with a fractional-delay loop filter (in
/// tune at high pitches).
fn ks(f: f64, dur: f64, bright: f64, decay: f64, seed: u64) -> Vec<f64> {
    let pd = pp::SRF / f;
    let big_n = (pd - 0.5).max(1.0) as usize;
    let a = pd - 0.5 - big_n as f64;
    let n = pp::len(dur);
    let mut r = Generator::new(seed);
    let mut buf = vec![0.0; big_n + 2];
    r.fill_uniform(-1.0, 1.0, &mut buf);
    for _ in 0..((1.0 - bright) * 4.0).max(0.0) as usize {
        let prev = buf.clone();
        let m = prev.len();
        for i in 0..m {
            buf[i] = 0.5 * (prev[i] + prev[(i + m - 1) % m]);
        }
    }
    let mean = buf.iter().sum::<f64>() / buf.len() as f64;
    buf.iter_mut().for_each(|v| *v -= mean);
    let mut out = vec![0.0; n.max(big_n + 3) + big_n + 3];
    out[..big_n + 2].copy_from_slice(&buf);
    let (c0, c1, c2) = (0.5 * (1.0 - a), 0.5, 0.5 * a);
    for i in big_n + 2..n {
        out[i] = decay * (c0 * out[i - big_n] + c1 * out[i - big_n - 1] + c2 * out[i - big_n - 2]);
    }
    out.truncate(n);
    out
}

/// `ukulele60_lowpoly.synth` for the one note `pp_bridge.voice` asks of the `modern` scene, at 44,100 Hz.
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    let f = pp::midi_hz(midi);
    let ring = if dur > 2.0 { 3.2 } else { (dur + 0.9).min(2.4) };
    let decay = if dur > 2.0 { 0.9989 } else { 0.9982 };
    let x = ks(f, ring, 0.75, decay, 0);
    let body = pp::bp(&x, 380.0 * 0.7, 380.0 * 1.4);
    let x: Vec<f64> = x.iter().zip(&body).map(|(a, b)| a + 0.22 * 3.0 * b).collect();
    let x = pp::lp(&x, 6500.0);
    x.iter()
        .zip(pp::tt(x.len()))
        .map(|(v, t)| v * ((ring - t) / 0.06).min(1.0) * vel)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_rings_and_fades_out() {
        let x = synth(69.0, 0.9, 0.3);
        assert_eq!(x.len(), pp::len(1.2));
        let peak = x.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        assert!(peak > 0.1 && peak < 3.0, "{peak}");
        assert!(x[x.len() - 1].abs() < 1e-2);
        assert_eq!(UkuleleVoice.register(), REGISTER);
    }
}
