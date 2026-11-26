//! M2: a low instrument a phone can play. Most of a tuba's energy lies under 200 Hz, where a phone speaker gives
//! nothing, so on a phone its notes all but vanish. The treatment adds overtones one and two octaves above the
//! note, made from the note's own lows so they follow its pitch and loudness, and takes away the rumble nobody
//! hears. The loudness of the whole stays what it was: only where the sound sits changes.
//!
//! This is the quick, shared version on the fighter's bus. A voice that has been rebuilt to carry its own
//! overtones (stage 3) is taken off it by its saved rumble share falling under a half.

use bf_core::dmath;

use crate::SRF;
use crate::dsp::{self, FilterKind, butter_sos};
use crate::heard::{self, BLOCK, HOP};

/// What a treated low instrument may lose on a phone, in LU (the bar is 9; this leaves a little room).
pub const PHONE_LOSS: f64 = 8.0;

/// The most the overtones are added by, against their own level.
const MOST: f64 = 8.0;

/// The squares of two signals summed block by block, for the loudness of `a + k * b` at any `k` without
/// filtering again: its mean square is `aa + 2 k ab + k^2 bb`.
struct Blocks {
    aa: Vec<f64>,
    ab: Vec<f64>,
    bb: Vec<f64>,
}

impl Blocks {
    /// Of two K-weighted signals of one length.
    fn new(a: &[[f64; 2]], b: &[[f64; 2]]) -> Blocks {
        let sums = |f: &dyn Fn(usize) -> f64| -> Vec<f64> {
            let mut run = vec![0.0; a.len() + 1];
            for i in 0..a.len() {
                run[i + 1] = run[i] + f(i);
            }
            (0..a.len().saturating_sub(BLOCK))
                .step_by(HOP)
                .map(|i| (run[i + BLOCK] - run[i]) / BLOCK as f64)
                .collect()
        };
        Blocks {
            aa: sums(&|i| a[i][0] * a[i][0] + a[i][1] * a[i][1]),
            ab: sums(&|i| a[i][0] * b[i][0] + a[i][1] * b[i][1]),
            bb: sums(&|i| b[i][0] * b[i][0] + b[i][1] * b[i][1]),
        }
    }

    /// The integrated loudness of `a + k * b`.
    fn integrated(&self, k: f64) -> f64 {
        let levels: Vec<f64> = (0..self.aa.len())
            .map(|i| {
                let ms = self.aa[i] + 2.0 * k * self.ab[i] + k * k * self.bb[i];
                -0.691 + 10.0 * dmath::log10(ms.max(0.0) + 1e-12)
            })
            .collect();
        heard::integrated_of(&levels)
    }
}

/// `x` (a low fighter's bus) with overtones a phone can play and without its rumble, as loud as before between
/// `from` and `to` seconds (the fight). A bus that is silent there comes back as it was.
pub fn treat(x: &[[f64; 2]], from: f64, to: f64) -> Vec<[f64; 2]> {
    if x.is_empty() || heard::integrated(heard::span(x, from, to)) <= -70.0 {
        return x.to_vec();
    }
    let sos = |kind, freqs: &[f64], order| butter_sos(kind, freqs, order, SRF);
    // The overtones: the lows, levelled by their own envelope, bent (odd and even overtones), and given the
    // envelope back, so a soft note stays soft.
    let low = dsp::sosfilt_stereo(&sos(FilterKind::Lp, &[300.0], 4), x);
    let power: Vec<f64> = low.iter().map(|f| (f[0] * f[0] + f[1] * f[1]) / 2.0).collect();
    let env: Vec<f64> = dsp::lfilter(&[0.002], &[1.0, -0.998], &power)
        .iter()
        .map(|&p| (p.max(0.0) + 1e-12).sqrt())
        .collect();
    let shaped: Vec<[f64; 2]> = low
        .iter()
        .zip(&env)
        .map(|(f, &e)| {
            let bend = |v: f64| {
                let y = v / (e * 2.2 + 1e-4);
                (dmath::tanh(4.0 * y) * 0.7 + dmath::tanh(3.0 * y).abs() * 0.6) * e
            };
            [bend(f[0]), bend(f[1])]
        })
        .collect();
    drop(low);
    let harm = dsp::sosfilt_stereo(&sos(FilterKind::Bp, &[260.0, 2400.0], 2), &shaped);
    let harm = dsp::sosfilt_stereo(&sos(FilterKind::Lp, &[1400.0], 1), &harm);
    // The rumble out: a 38 Hz high-pass and a shelf down under about 110 Hz.
    let mut trimmed = dsp::sosfilt_stereo(&sos(FilterKind::Hp, &[38.0], 2), x);
    let lowest = dsp::sosfilt_stereo(&sos(FilterKind::Lp, &[110.0], 2), &trimmed);
    for (t, l) in trimmed.iter_mut().zip(&lowest) {
        t[0] -= 0.4 * l[0];
        t[1] -= 0.4 * l[1];
    }
    drop(lowest);
    // How much of the overtones: the least that brings the loss on a phone to PHONE_LOSS, found by halving.
    let (t_span, h_span) = (heard::span(&trimmed, from, to), heard::span(&harm, from, to));
    let whole = Blocks::new(&heard::k_weight(t_span), &heard::k_weight(h_span));
    let phone = Blocks::new(
        &heard::k_weight(&heard::phone(t_span)),
        &heard::k_weight(&heard::phone(h_span)),
    );
    let loss = |k: f64| whole.integrated(k) - phone.integrated(k);
    let (mut lo, mut hi) = (0.0, MOST);
    if loss(0.0) <= PHONE_LOSS {
        hi = 0.0;
    } else {
        for _ in 0..9 {
            let mid = (lo + hi) / 2.0;
            if loss(mid) > PHONE_LOSS {
                lo = mid;
            } else {
                hi = mid;
            }
        }
    }
    // As loud as before, over the fight.
    let before = heard::integrated(heard::span(x, from, to));
    let g = heard::gain(before - whole.integrated(hi));
    trimmed
        .iter()
        .zip(&harm)
        .map(|(t, h)| [(t[0] + hi * h[0]) * g, (t[1] + hi * h[1]) * g])
        .collect()
}

/// Where the band and the hits make room for a bass fighter (C6), in Hz, and by how much, in dB.
pub const ROOM_FOR_BASS: (f64, f64) = (150.0, -6.0);

/// How long the room for the bass takes to open and to close, in seconds.
const ROOM_RAMP: f64 = 0.05;

/// `x` (a bus of the band or the hits) with its lows under [`ROOM_FOR_BASS`] turned down between `from` and `to`
/// seconds (the fight), opening and closing over [`ROOM_RAMP`]: when a bass fighter holds the bottom (C6), the
/// band's kick and the hits' thumps leave it the lows instead of covering them. Outside the fight `x` is as it
/// was.
pub fn room_for_bass(x: &[[f64; 2]], from: f64, to: f64) -> Vec<[f64; 2]> {
    let (hz, cut) = ROOM_FOR_BASS;
    if x.is_empty() || to <= from {
        return x.to_vec();
    }
    // the lows, forward and backward (no shift in time), taken out down to the cut's level
    let sos = butter_sos(FilterKind::Lp, &[hz], 2, SRF);
    let mut low = dsp::sosfilt_stereo(&sos, x);
    low.reverse();
    let mut low = dsp::sosfilt_stereo(&sos, &low);
    low.reverse();
    let away = 1.0 - heard::gain(cut);
    x.iter()
        .zip(&low)
        .enumerate()
        .map(|(i, (f, l))| {
            let t = i as f64 / SRF;
            let open = ((t - from) / ROOM_RAMP).min((to - t) / ROOM_RAMP).clamp(0.0, 1.0);
            let k = away * open;
            [f[0] - k * l[0], f[1] - k * l[1]]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    /// Four seconds of low A (55 Hz) in notes of half a second, with the first overtones a tuba has.
    fn low_notes() -> Vec<[f64; 2]> {
        (0..4 * crate::SR)
            .map(|i| {
                let t = i as f64 / SRF;
                let fade = dmath::exp(-3.0 * (t % 0.5));
                let v: f64 = [(1.0, 1.0), (2.0, 0.5), (3.0, 0.25), (4.0, 0.12)]
                    .iter()
                    .map(|&(h, a)| 0.3 * a * dmath::sin(2.0 * PI * 55.0 * h * t))
                    .sum();
                [v * fade, v * fade]
            })
            .collect()
    }

    /// Low notes all but vanish on a phone (they lose 26 LU); treated, they lose at most 9, are as loud as they
    /// were, and hold less rumble.
    #[test]
    fn low_notes_are_heard_on_a_phone() {
        let x = low_notes();
        assert!(heard::phone_loss(&x) > 20.0, "{}", heard::phone_loss(&x));
        let y = treat(&x, 0.0, 4.0);
        assert_eq!(y.len(), x.len());
        let loss = heard::phone_loss(&y);
        assert!(loss <= 9.0, "loses {loss:.1} LU on a phone");
        let moved = heard::integrated(&y) - heard::integrated(&x);
        assert!(moved.abs() < 0.5, "moved by {moved:.2} LU");
        let (before, after) = (heard::share(&x, heard::RUMBLE), heard::share(&y, heard::RUMBLE));
        assert!(
            after < 0.8 && after < before - 0.15,
            "rumble {before:.2} before, {after:.2} after"
        );
    }

    /// Silence, and nothing at all, come back as they were.
    #[test]
    fn silence_is_left_alone() {
        let quiet = vec![[0.0; 2]; crate::SR];
        assert_eq!(treat(&quiet, 0.0, 1.0), quiet);
        assert!(treat(&[], 0.0, 1.0).is_empty());
    }

    /// The room for a bass fighter: a 60 Hz tone is down by the cut in the fight and as it was outside it, and a
    /// 1 kHz tone is left alone.
    #[test]
    fn the_band_makes_room_for_a_bass_fighter() {
        let n = 3 * 48_000;
        let tone = |hz: f64| -> Vec<[f64; 2]> {
            (0..n)
                .map(|i| {
                    let v = 0.3 * dmath::sin(2.0 * std::f64::consts::PI * hz * i as f64 / SRF);
                    [v, v]
                })
                .collect()
        };
        let rms = |x: &[[f64; 2]]| (x.iter().map(|f| f[0] * f[0]).sum::<f64>() / x.len() as f64).sqrt();
        let (low, high) = (tone(60.0), tone(1000.0));
        let (l, h) = (room_for_bass(&low, 1.0, 2.0), room_for_bass(&high, 1.0, 2.0));
        let inside = 52_800..91_200;
        let moved =
            |y: &[[f64; 2]], x: &[[f64; 2]], r: std::ops::Range<usize>| heard::db(rms(&y[r.clone()]) / rms(&x[r]));
        assert!(
            (moved(&l, &low, inside.clone()) - ROOM_FOR_BASS.1).abs() < 1.0,
            "{}",
            moved(&l, &low, inside.clone())
        );
        assert!(moved(&l, &low, 9_600..40_000).abs() < 0.2, "before the fight");
        assert!(moved(&l, &low, 110_000..134_000).abs() < 0.2, "after the fight");
        assert!(moved(&h, &high, inside).abs() < 0.3, "the middle stays");
        assert!(room_for_bass(&[], 0.0, 1.0).is_empty());
    }
}
