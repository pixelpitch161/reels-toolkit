//! F3: the room of the Composed arrangement. Today's room is decaying noise with no high-pass, and four fifths of
//! its energy lies under 200 Hz. This one starts about 20 ms after the sound, with seven early reflections and a
//! tail of about 1.25 s whose highs fade faster than its lows, and holds nothing under 220 Hz or above 7 kHz.

use bf_core::dmath;
use bf_core::rng::Generator;

use crate::SRF;
use crate::dsp::{self, FilterKind, butter_sos};

/// The seed of the room's noise and of where its reflections fall.
pub const SEED: u64 = 7;
/// The length of the room's response and how long its middle takes to fade by 60 dB, in seconds.
const LENGTH: f64 = 2.4;
const FADE: f64 = 1.25;
/// The early reflections: when (seconds, each moved by up to 3 ms either way) and how strong.
const EARLY: [(f64, f64); 7] = [
    (0.020, 0.9),
    (0.026, 0.75),
    (0.034, 0.7),
    (0.044, 0.55),
    (0.056, 0.5),
    (0.070, 0.4),
    (0.082, 0.3),
];
const JITTER: f64 = 0.003;
/// The early reflections against the tail (each of unit energy before).
const EARLY_LEVEL: f64 = 0.55;
/// The edges of the room's sound, in Hz.
pub const EDGES: (f64, f64) = (220.0, 7000.0);
/// How far the room lies under the dry mix, in LU.
pub const UNDER: f64 = 18.0;

/// What a bus sends to the room, by the kind of instrument: a low one next to nothing (a room full of bass is
/// mud), a piercing one less than the rest.
pub fn send(low: bool, piercing: bool) -> f64 {
    if low {
        0.04
    } else if piercing {
        0.2
    } else {
        0.3
    }
}

/// The room's response to one click, left and right (they differ, which is what makes it wide).
pub fn ir(seed: u64) -> Vec<[f64; 2]> {
    let n = (LENGTH * SRF) as usize;
    let mut r = Generator::new(seed);
    let sos = |kind, hz: f64| butter_sos(kind, &[hz], 2, SRF);
    let mut out = vec![[0.0; 2]; n];
    for c in 0..2 {
        let mut z = vec![0.0; n];
        r.fill_normal(0.0, 1.0, &mut z);
        let lo = dsp::sosfilt(&sos(FilterKind::Lp, 500.0), &z);
        let hi = dsp::sosfilt(&sos(FilterKind::Hp, 3000.0), &z);
        // each range fades at its own speed: the highs in under half the time
        let mut tail: Vec<f64> = (0..n)
            .map(|i| {
                let t = i as f64 / SRF;
                let fall = |by: f64| dmath::exp(-6.9 * t / (FADE * by));
                let grow = ((t - 0.033) / 0.05).clamp(0.0, 1.0);
                (lo[i] * fall(0.9) + (z[i] - lo[i] - hi[i]) * fall(1.0) + hi[i] * fall(0.45)) * grow
            })
            .collect();
        unit(&mut tail);
        let mut early = vec![0.0; n];
        for (at, level) in EARLY {
            let t = at + r.uniform(-JITTER, JITTER);
            let sign = if r.random() < 0.5 { -1.0 } else { 1.0 };
            early[((t * SRF) as usize).min(n - 1)] += level * sign;
        }
        unit(&mut early);
        let both: Vec<f64> = (0..n).map(|i| EARLY_LEVEL * early[i] + tail[i]).collect();
        let both = dsp::sosfilt(&sos(FilterKind::Hp, EDGES.0), &both);
        let both = dsp::sosfilt(&sos(FilterKind::Lp, EDGES.1), &both);
        for (o, v) in out.iter_mut().zip(both) {
            o[c] = v;
        }
    }
    out
}

/// `x` scaled to unit energy (silence stays silence).
fn unit(x: &mut [f64]) {
    let e = x.iter().map(|v| v * v).sum::<f64>().sqrt();
    if e > 0.0 {
        for v in x {
            *v /= e;
        }
    }
}

/// What the room gives back of `send`: `send` through [`ir`], as long as `send`.
pub fn wet(send: &[[f64; 2]], seed: u64) -> Vec<[f64; 2]> {
    if send.is_empty() {
        return Vec::new();
    }
    let ir = ir(seed);
    let channel = |c: usize| -> Vec<f64> {
        let dry: Vec<f64> = send.iter().map(|f| f[c]).collect();
        let room: Vec<f64> = ir.iter().map(|f| f[c]).collect();
        dsp::fftconvolve(&dry, &room)
    };
    let (l, r) = (channel(0), channel(1));
    (0..send.len()).map(|i| [l[i], r[i]]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SR, heard};

    /// The room starts 17 ms after the sound at the earliest, holds next to no rumble, and its two sides differ.
    #[test]
    fn the_room_starts_late_and_holds_no_rumble() {
        let ir = ir(SEED);
        assert_eq!(ir.len(), (LENGTH * SRF) as usize);
        let before = (0.017 * SRF) as usize;
        assert!(ir[..before].iter().all(|f| f[0] == 0.0 && f[1] == 0.0));
        assert!(
            heard::share(&ir, heard::RUMBLE) < 0.10,
            "{}",
            heard::share(&ir, heard::RUMBLE)
        );
        assert!(ir.iter().any(|f| f[0] != f[1]));
        assert_eq!(ir, super::ir(SEED));
        // the highs fade faster: the second half second holds a smaller part above 3 kHz than the first
        let half = SR / 2;
        let highs = |x: &[[f64; 2]]| heard::share(x, (3000.0, 24_000.0));
        assert!(highs(&ir[half..2 * half]) < highs(&ir[..half]));
    }

    /// A click comes back as the room's response; the sends follow the kind of instrument.
    #[test]
    fn a_click_gives_the_response() {
        let mut click = vec![[0.0; 2]; SR];
        click[0] = [1.0, 1.0];
        let w = wet(&click, SEED);
        let ir = ir(SEED);
        assert_eq!(w.len(), click.len());
        for i in (0..SR).step_by(997) {
            assert!(
                (w[i][0] - ir[i][0]).abs() < 1e-9 && (w[i][1] - ir[i][1]).abs() < 1e-9,
                "at {i}"
            );
        }
        assert!(wet(&[], SEED).is_empty());
        assert_eq!(
            (
                send(true, false),
                send(true, true),
                send(false, true),
                send(false, false)
            ),
            (0.04, 0.04, 0.2, 0.3)
        );
    }
}
