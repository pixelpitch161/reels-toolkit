//! How loud a sound is to the ear (sound upgrade, 2026-11-25): the measures the Composed arrangement's mix and
//! polish steer by, and the ones its checks read.
//!
//! All of them stand on ITU-R BS.1770 K-weighting at 48 kHz ([`k_weight`]) and on the loudness of 400 ms blocks
//! taken every 100 ms ([`blocks`]):
//!
//! - [`integrated`]: the loudness of a whole stretch, gated as the standard gates it (blocks under -70 LUFS and
//!   those more than 10 LU under the rest do not count), so silence does not pull the figure down.
//! - [`heard`]: the block loudness after a 250 Hz high-pass, a stand-in for small speakers, on which most of a
//!   tuba's energy is not heard at all. [`over_band`] is the fighter's heard level less the band's while the
//!   fighter sounds: the figure the mix holds at 5 dB.
//! - [`phone_loss`]: how much loudness a phone speaker (little under 350 Hz) takes away.
//! - [`share`]: the part of a sound's energy between two frequencies ([`RUMBLE`], [`PIERCING`]).
//! - [`true_peak`]: the highest level between the samples (4 times oversampled).

use bf_core::dmath;

use crate::dsp::fft::Fft;
use crate::dsp::{self, FilterKind, butter_sos};
use crate::{SR, SRF};

/// The first K-weighting filter (the high shelf), `(b, a)` at 48 kHz.
pub const K_SHELF: ([f64; 3], [f64; 3]) = (
    [1.53512485958697, -2.69169618940638, 1.19839281085285],
    [1.0, -1.69065929318241, 0.73248077421585],
);
/// The second K-weighting filter (the high-pass), `(b, a)` at 48 kHz.
pub const K_HIGH_PASS: ([f64; 3], [f64; 3]) = ([1.0, -2.0, 1.0], [1.0, -1.99004745483398, 0.99007225036621]);

/// The length of a loudness block and the step between two, in samples (400 ms and 100 ms).
pub const BLOCK: usize = SR * 2 / 5;
pub const HOP: usize = SR / 10;

/// The frequencies of the rumble (what a phone cannot play) and of the piercing band, in Hz.
pub const RUMBLE: (f64, f64) = (0.0, 200.0);
pub const PIERCING: (f64, f64) = (2000.0, 6000.0);

/// `x` through the two K-weighting filters. Their delays are flushed to zero under the smallest normal number
/// ([`dsp::lfilter_stereo_flushed`]): a bus that falls silent otherwise rings them out into subnormal numbers, about 11
/// times slower to work, while every measure here squares the output, which comes out the same to the bit.
/// A long signal's two sides are filtered on two threads (each side on its own, so the same numbers).
pub fn k_weight(x: &[[f64; 2]]) -> Vec<[f64; 2]> {
    if x.len() < SR {
        let y = dsp::lfilter_stereo_flushed(&K_SHELF.0, &K_SHELF.1, x);
        return dsp::lfilter_stereo_flushed(&K_HIGH_PASS.0, &K_HIGH_PASS.1, &y);
    }
    let side = |c: usize| -> Vec<f64> {
        let ch: Vec<f64> = x.iter().map(|f| f[c]).collect();
        let y = dsp::lfilter_flushed(&K_SHELF.0, &K_SHELF.1, &ch);
        dsp::lfilter_flushed(&K_HIGH_PASS.0, &K_HIGH_PASS.1, &y)
    };
    let (left, right) = std::thread::scope(|s| {
        let right = s.spawn(|| side(1));
        (side(0), right.join().expect("the right side's weighting"))
    });
    left.into_iter().zip(right).map(|(l, r)| [l, r]).collect()
}

/// The loudness of a mean square summed over the channels, in LUFS.
fn lufs_of(mean_square: f64) -> f64 {
    -0.691 + 10.0 * dmath::log10(mean_square + 1e-12)
}

/// The mean square a loudness stands for (the inverse of [`lufs_of`], without its floor).
fn power_of(lufs: f64) -> f64 {
    dmath::pow(10.0, (lufs + 0.691) / 10.0)
}

/// The loudness of each 400 ms block of `x` (one every 100 ms), in LUFS. A sound shorter than a block has none.
pub fn blocks(x: &[[f64; 2]]) -> Vec<f64> {
    if x.len() <= BLOCK {
        return Vec::new();
    }
    let z = k_weight(x);
    let mut sums = Vec::with_capacity(z.len() + 1);
    let mut run = 0.0;
    sums.push(run);
    for f in &z {
        run += f[0] * f[0] + f[1] * f[1];
        sums.push(run);
    }
    (0..z.len() - BLOCK)
        .step_by(HOP)
        .map(|i| lufs_of((sums[i + BLOCK] - sums[i]) / BLOCK as f64))
        .collect()
}

/// The gated loudness of block levels: those above -70 LUFS, and of them those within 10 LU of their mean. -99
/// for a sound with no such block.
pub fn integrated_of(levels: &[f64]) -> f64 {
    let mean = |g: &[f64]| lufs_of(g.iter().map(|&l| power_of(l)).sum::<f64>() / g.len() as f64);
    let loud: Vec<f64> = levels.iter().copied().filter(|&l| l > -70.0).collect();
    if loud.is_empty() {
        return -99.0;
    }
    let gate = mean(&loud) - 10.0;
    let kept: Vec<f64> = loud.iter().copied().filter(|&l| l > gate).collect();
    if kept.is_empty() { -99.0 } else { mean(&kept) }
}

/// The integrated loudness of `x`, in LUFS ([`integrated_of`] its [`blocks`]).
pub fn integrated(x: &[[f64; 2]]) -> f64 {
    integrated_of(&blocks(x))
}

/// The block loudness of `x` as small speakers give it: after a 250 Hz high-pass of the second order.
pub fn heard(x: &[[f64; 2]]) -> Vec<f64> {
    if x.len() <= BLOCK {
        return Vec::new();
    }
    blocks(&dsp::sosfilt_stereo(&butter_sos(FilterKind::Hp, &[250.0], 2, SRF), x))
}

/// The samples of `x` from `from` to `to` seconds.
pub fn span(x: &[[f64; 2]], from: f64, to: f64) -> &[[f64; 2]] {
    let at = |t: f64| ((t.max(0.0) * SRF) as usize).min(x.len());
    let (i0, i1) = (at(from), at(to));
    &x[i0..i1.max(i0)]
}

/// How far the fighter stands over the band between `from` and `to` seconds, in dB: the mean of its [`heard`]
/// levels less the mean of the band's, over the blocks where the fighter sounds (within 20 dB of its loudest
/// block, and above -65). `None` when the fighter never sounds there, or the stretch is shorter than a block.
pub fn over_band(fighter: &[[f64; 2]], band: &[[f64; 2]], from: f64, to: f64) -> Option<f64> {
    let (la, lb) = (heard(span(fighter, from, to)), heard(span(band, from, to)));
    let top = la.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let floor = (top - 20.0).max(-65.0);
    let on: Vec<usize> = (0..la.len().min(lb.len())).filter(|&i| la[i] > floor).collect();
    if on.is_empty() {
        return None;
    }
    let mean = |l: &[f64]| on.iter().map(|&i| l[i]).sum::<f64>() / on.len() as f64;
    Some(mean(&la) - mean(&lb))
}

/// `x` as a phone speaker plays it: a 350 Hz high-pass of the third order.
pub fn phone(x: &[[f64; 2]]) -> Vec<[f64; 2]> {
    if x.is_empty() {
        return Vec::new();
    }
    dsp::sosfilt_stereo(&butter_sos(FilterKind::Hp, &[350.0], 3, SRF), x)
}

/// The loudness `x` loses on a phone speaker, in LU (0 or more for any sound with lows).
pub fn phone_loss(x: &[[f64; 2]]) -> f64 {
    integrated(x) - integrated(&phone(x))
}

/// The part of the energy of `x` (its two channels' mean) between `lo` and `hi` Hz, 0 to 1. 0 for silence.
pub fn share(x: &[[f64; 2]], (lo, hi): (f64, f64)) -> f64 {
    if x.is_empty() {
        return 0.0;
    }
    let n = x.len().next_power_of_two();
    let mut re = vec![0.0; n];
    let mut im = vec![0.0; n];
    for (r, f) in re.iter_mut().zip(x) {
        *r = (f[0] + f[1]) / 2.0;
    }
    Fft::new(n).forward(&mut re, &mut im);
    let (mut all, mut band) = (0.0, 0.0);
    for k in 0..=n / 2 {
        let p = re[k] * re[k] + im[k] * im[k];
        let f = k as f64 * SRF / n as f64;
        all += p;
        if f >= lo && f < hi {
            band += p;
        }
    }
    if all > 0.0 { band / all } else { 0.0 }
}

/// The true peak of `x`: the highest absolute level of the two channels, each 4 times oversampled.
pub fn true_peak(x: &[[f64; 2]]) -> f64 {
    if x.is_empty() {
        return 0.0;
    }
    // each channel on a thread of its own: the largest of the two is the same number either way
    let channel = |c: usize| -> f64 {
        let ch: Vec<f64> = x.iter().map(|f| f[c]).collect();
        let mut peak = 0.0f64;
        for v in dsp::resample_poly(&ch, 4, 1) {
            peak = peak.max(v.abs());
        }
        for v in &ch {
            peak = peak.max(v.abs());
        }
        peak
    };
    let (left, right) = std::thread::scope(|s| {
        let right = s.spawn(|| channel(1));
        (channel(0), right.join().expect("the right channel's peak"))
    });
    left.max(right)
}

/// `20 log10(x)`, with a floor of -200 dB.
pub fn db(x: f64) -> f64 {
    20.0 * dmath::log10(x.max(1e-10))
}

/// The gain of `d` dB.
pub fn gain(d: f64) -> f64 {
    dmath::pow(10.0, d / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(hz: f64, level_db: f64, seconds: f64) -> Vec<[f64; 2]> {
        let a = gain(level_db);
        (0..(seconds * SRF) as usize)
            .map(|i| {
                let v = a * dmath::sin(dmath::TAU * hz * i as f64 / SRF);
                [v, v]
            })
            .collect()
    }

    #[test]
    fn a_full_scale_tone_reads_as_the_standard_says() {
        // BS.1770: a 997 Hz sine at 0 dBFS in both channels reads 0 LKFS: -0.691 + 10 log10(2 * 0.5) plus the
        // K-weighting's +0.69 dB near 1 kHz (the offset is there to cancel it). At -20 dBFS, 20 lower.
        let x = sine(997.0, -20.0, 3.0);
        assert!((integrated(&x) - -20.0).abs() < 0.1, "{}", integrated(&x));
        // Silence before and after does not change a gated figure by much: the silent blocks are gated out, and
        // only the six blocks that straddle the tone's two edges and pass the relative gate pull it down, by 0.41 LU
        // for this 3 s tone (worked out on the block powers).
        let mut padded = vec![[0.0; 2]; SR * 2];
        padded.extend(&x);
        padded.extend(vec![[0.0; 2]; SR * 2]);
        let moved = integrated(&padded) - integrated(&x);
        assert!((moved - -0.41).abs() < 0.05, "{moved}");
        assert_eq!(integrated(&[[0.0; 2]; SR * 2]), -99.0);
        assert!(blocks(&x[..BLOCK]).is_empty());
        assert_eq!(blocks(&x).len(), (x.len() - BLOCK).div_ceil(HOP));
    }

    #[test]
    fn rumble_is_lost_on_a_phone_and_a_high_tone_is_not() {
        let low = sine(60.0, -12.0, 3.0);
        assert!(share(&low, RUMBLE) > 0.99 && share(&low, PIERCING) < 0.001);
        assert!(phone_loss(&low) > 20.0, "{}", phone_loss(&low));
        let high = sine(3000.0, -12.0, 3.0);
        assert!(share(&high, PIERCING) > 0.99 && share(&high, RUMBLE) < 0.001);
        assert!(phone_loss(&high).abs() < 0.5);
        assert_eq!(share(&[], RUMBLE), 0.0);
        assert_eq!(share(&[[0.0; 2]; 64], RUMBLE), 0.0);
    }

    #[test]
    fn over_band_is_the_difference_while_the_fighter_sounds() {
        let band = sine(880.0, -18.0, 6.0);
        let fighter: Vec<[f64; 2]> = band.iter().map(|f| [f[0] * 2.0, f[1] * 2.0]).collect();
        let d = over_band(&fighter, &band, 1.0, 5.0).unwrap();
        assert!((d - 6.02).abs() < 0.1, "{d}");
        // A fighter that plays only in the middle is measured there only.
        let mut part = vec![[0.0; 2]; fighter.len()];
        part[SR * 2..SR * 4].copy_from_slice(&fighter[SR * 2..SR * 4]);
        let d = over_band(&part, &band, 0.0, 6.0).unwrap();
        assert!((d - 6.0).abs() < 1.5, "{d}");
        // A silent fighter, an empty stretch, a stretch past the end.
        assert_eq!(over_band(&vec![[0.0; 2]; band.len()], &band, 1.0, 5.0), None);
        assert_eq!(over_band(&fighter, &band, 3.0, 3.0), None);
        assert_eq!(over_band(&fighter, &band, 50.0, 60.0), None);
        assert_eq!(over_band(&fighter, &band, 5.0, 1.0), None);
    }

    #[test]
    fn the_true_peak_is_at_least_the_sample_peak() {
        // A tone at a quarter of the sample rate, sampled 45 degrees off its crests: every sample is 0.707, the
        // wave between them reaches 1.
        let x: Vec<[f64; 2]> = (0..4800)
            .map(|i| {
                let v = dmath::sin(dmath::TAU * (i as f64 / 4.0 + 0.125));
                [v, v]
            })
            .collect();
        let sample = x.iter().flatten().fold(0.0f64, |m, v| m.max(v.abs()));
        assert!((sample - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-3);
        let peak = true_peak(&x);
        assert!(peak > 0.97 && peak < 1.05, "{peak}");
        assert_eq!(true_peak(&[]), 0.0);
        assert!((db(gain(-7.5)) - -7.5).abs() < 1e-9);
    }
}
