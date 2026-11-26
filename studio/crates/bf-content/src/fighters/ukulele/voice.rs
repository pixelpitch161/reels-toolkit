//! UKULELE's instrument voice: `pp_bridge.voice('UKULELE', midi, vel, dur)`, the one note `ukulele60_lowpoly.synth`
//! makes on its default scene (key `modern`): a Karplus-Strong string with a fractional-delay loop filter (bright
//! 0.75, decay 0.9982, ringing `min(2.4, dur + 0.9)` s), its body resonance (`+ 0.66 * bp(x, 266, 532)`), a low-pass
//! at 6,500 Hz and a 60 ms release, times the velocity. Made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! Close to the Pixel Pitch voice by ear (ruling M4-R16), not bit for bit: the string's noise is the shared
//! generator seeded 0, as `ks(.., seed=idx)` is for the note's index 0.

use bf_core::rng::Generator;
use bf_sound::voice::pp;
use bf_sound::voice::real::{Correction, Fill, Rework};
use bf_sound::voice::{PpNoise, Voice};

/// The UKULELE's register in `pp_bridge`'s roster (`('UKULELE', 'ukulele60_lowpoly', None, (60, 81), 0, 1.0)`).
pub const REGISTER: (i32, i32) = (60, 81);

/// The UKULELE's added overtones: the harmonics its own note barely has, laid in at a real nylon-string guitar's
/// level (FluidR3 has no ukulele; dB under the fundamental, `NONE` for none), note by note. Written with the
/// [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (60, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (61, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (62, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (63, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (64, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (65, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (66, [-99.0, -99.0, 4.2, -1.9, -99.0, -99.0, -18.3, -99.0]),
    (67, [-99.0, -99.0, 4.4, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (68, [-99.0, -99.0, -99.0, -99.0, -99.0, -12.7, -22.9, -37.3]),
    (69, [-99.0, -99.0, -99.0, -99.0, -99.0, -12.8, -23.1, -37.6]),
    (70, [-99.0, -99.0, -99.0, -99.0, -99.0, -13.0, -23.3, -37.9]),
    (71, [-99.0, -99.0, -99.0, -99.0, -13.3, -99.0, -41.3, -27.6]),
    (72, [-99.0, -99.0, -99.0, -99.0, -13.5, -99.0, -42.4, -28.2]),
    (73, [-99.0, -99.0, -99.0, -99.0, -13.7, -99.0, -99.0, -29.0]),
    (74, [-99.0, -99.0, -99.0, -18.4, -13.9, -99.0, -99.0, -29.8]),
    (75, [-99.0, 7.0, -99.0, -18.6, -14.2, -99.0, -99.0, -30.7]),
    (76, [-99.0, 5.9, -9.8, -4.3, -6.9, -20.0, -24.7, -33.7]),
    (77, [-99.0, 5.7, -10.0, -4.9, -7.9, -21.0, -26.0, -35.8]),
    (78, [-99.0, 5.5, -99.0, -5.6, -8.8, -22.0, -27.4, -37.9]),
    (79, [-99.0, 5.3, -10.4, -6.2, -9.7, -23.1, -28.9, -40.2]),
    (80, [-99.0, 5.2, -10.5, -6.9, -10.6, -24.3, -30.4, -42.5]),
    (81, [-99.0, 5.1, -10.6, -7.6, -11.4, -25.6, -32.1, -99.0]),
];

/// The UKULELE's overtone correction toward a real nylon-string guitar (FluidR3 has no ukulele): gains in dB for
/// its first eight harmonics, note by note (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "UKULELE"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (60, [5.4, -17.1, -16.3, -1.8, -14.7, -10.9, -23.6, -30.0]),
    (61, [1.5, -11.2, -13.3, 10.5, -9.4, -2.3, -22.5, -12.0]),
    (62, [1.1, -12.6, -13.9, 9.2, -6.4, -7.1, -22.3, -19.8]),
    (63, [-2.9, -12.5, 3.4, 4.1, 2.9, -9.2, -1.0, -9.8]),
    (64, [-5.4, -13.2, 7.5, 6.2, 5.4, -9.4, 5.9, -1.5]),
    (65, [-5.9, -12.8, 9.1, 6.8, -2.6, -5.9, 4.8, -0.4]),
    (66, [-0.5, -5.4, 0.0, -0.6, 3.2, 9.8, 0.3, 12.7]),
    (67, [0.9, -5.1, 0.0, 15.9, -3.5, 3.9, 8.6, 1.0]),
    (68, [0.0, 3.2, 9.9, -1.6, 6.4, -0.4, 0.1, -0.4]),
    (69, [0.0, 4.1, 10.9, -6.0, 12.1, -0.1, 0.1, 0.1]),
    (70, [0.0, 2.1, 10.0, -12.4, 11.7, -0.6, -0.6, 0.3]),
    (71, [-7.7, 1.4, -5.8, -7.2, -8.4, -7.0, -7.5, -7.7]),
    (72, [-7.6, 1.2, -3.5, -5.9, -8.0, -1.2, -7.4, -7.5]),
    (73, [-7.4, 0.5, -6.4, -8.3, -6.0, -7.3, 0.0, -7.6]),
    (74, [-8.2, 5.9, 1.1, -9.6, -8.4, 0.0, 0.0, -8.3]),
    (75, [-0.9, 0.0, 7.8, -1.1, -1.1, 0.0, 0.0, -1.0]),
    (76, [-1.0, 0.0, -0.3, -1.0, -1.0, -1.0, -1.0, -1.0]),
    (77, [-0.8, 0.0, -0.3, -0.8, -0.8, -0.7, -0.7, -0.8]),
    (78, [-0.7, 0.0, 10.2, -1.0, -0.8, -0.7, -0.7, -0.6]),
    (79, [-0.4, 0.0, -0.3, -0.4, -0.4, -0.4, -0.4, -0.4]),
    (80, [0.5, 0.0, 0.3, 0.5, 0.5, 0.5, 0.4, 0.4]),
    (81, [0.7, 0.0, 0.5, 0.6, 0.6, 0.6, 0.7, 0.0]),
];

/// The Composed arrangement's UKULELE (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added, and a quicker stop (a 150 ms fade after its length), nearer the real
/// one's; it keeps its own pluck's snap.
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.0,
    stop: 0.15,
    keep_shape: true,
    tail: &[],
};

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

    fn rework(&self) -> Option<&'static Rework> {
        Some(&REWORK)
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
