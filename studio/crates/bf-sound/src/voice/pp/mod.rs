//! What the Pixel Pitch voices share: `pp_framework`'s sample rate and pitch, `drums60_lowpoly`'s filters, and
//! `pp_bridge`'s resampling; and the synthesis blocks several kit voices call ([`brass`], [`pluck`], [`sax`]).

use bf_core::dmath;

use crate::dsp::{self, FilterKind};

pub mod brass;
pub mod pluck;
pub mod sax;

/// The series' sample rate (`pp_framework.SR`, `pp_bridge.PP_SR`), in Hz.
pub const SR: usize = 44_100;

/// [`SR`] as a float (the voices in `bf-content` use it too).
pub const SRF: f64 = SR as f64;

/// `int(seconds * SR)`: a length in the series' samples, cut toward zero.
///
/// # Panics
/// Where Python raises: a length that is not a number, or below zero (numpy refuses a negative length).
pub fn len(seconds: f64) -> usize {
    let n = seconds * SRF;
    assert!(n.is_finite(), "cannot convert float {n} to integer");
    assert!(n > -1.0, "negative dimensions are not allowed ({seconds} s)");
    n as usize
}

/// Whether [`len`] takes `seconds`: a number, and not a negative count of samples (a fraction of a sample
/// below zero is cut to none, as Python's `int` cuts it).
pub fn has_len(seconds: f64) -> bool {
    let n = seconds * SRF;
    n.is_finite() && n > -1.0
}

/// `_t(n)`: the time of each of `n` samples, in seconds (`np.arange(n) / SR`).
pub fn tt(n: usize) -> Vec<f64> {
    (0..n).map(|i| i as f64 / SRF).collect()
}

/// `midi_hz(m)`: the frequency of MIDI note `m`, `440 * 2 ** ((m - 69) / 12)`.
pub fn midi_hz(m: f64) -> f64 {
    440.0 * dmath::pow(2.0, (m - 69.0) / 12.0)
}

/// `drums60_lowpoly.lp(x, f)`: `x` through `butter(2, f / (SR / 2), "low")` with `lfilter` (the series
/// designs its filters as `b`, `a`, not as second-order sections). An empty `x` gives an empty result.
///
/// # Panics
/// Where scipy raises: `f` is not between 0 and half the sample rate.
pub fn lp(x: &[f64], f: f64) -> Vec<f64> {
    // Python passes no `fs`: the cutoff is a fraction of the Nyquist frequency.
    let (b, a) = dsp::butter_ba(FilterKind::Lp, &[f / (SRF / 2.0)], 2, 2.0);
    dsp::lfilter(&b, &a, x)
}

/// `drums60_lowpoly.bp(x, lo, hi)`: `x` through `butter(2, [lo / (SR / 2), hi / (SR / 2)], "band")` with
/// `lfilter` (a band-pass of order 2 has four poles). An empty `x` gives an empty result.
///
/// # Panics
/// Where scipy raises: the edges are not `0 < lo < hi < SR / 2`.
pub fn bp(x: &[f64], lo: f64, hi: f64) -> Vec<f64> {
    let (b, a) = dsp::butter_ba(FilterKind::Bp, &[lo / (SRF / 2.0), hi / (SRF / 2.0)], 2, 2.0);
    dsp::lfilter(&b, &a, x)
}

/// `pp_bridge._resample(x)`: from 44,100 Hz to 48,000 Hz, `scipy.signal.resample_poly(x, 160, 147)`.
pub fn to_48k(x: &[f64]) -> Vec<f64> {
    dsp::resample_poly(x, 160, 147)
}

/// `np.clip(x, 0, 1)`: numpy's rule (a NaN stays NaN).
pub(crate) fn clip01(x: f64) -> f64 {
    crate::synth::clip(x, 0.0, 1.0)
}

/// `np.minimum(1, x)`: 1 where `x` is larger, `x` otherwise (a NaN stays NaN).
pub(crate) fn min1(x: f64) -> f64 {
    if x.is_nan() || x < 1.0 { x } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_and_times() {
        assert_eq!(len(1.0), 44100);
        // int((0.12 + .18) * SR) and int(.75 * (60 / 112) * SR): cut, not rounded.
        assert_eq!(len(0.12 + 0.18), 13230);
        assert_eq!(len(0.75 * (60.0 / 112.0)), 17718);
        assert_eq!(len(0.0), 0);
        // int() cuts toward zero: a length a fraction of a sample below zero is none.
        assert_eq!(len(-0.5 / 44100.0), 0);
        // has_len says where len would refuse.
        assert!(has_len(0.0) && has_len(1.0) && has_len(-0.5 / 44100.0));
        assert!(!has_len(-0.2) && !has_len(-1.5 / 44100.0) && !has_len(f64::NAN) && !has_len(f64::INFINITY));
        assert_eq!(tt(3), [0.0, 1.0 / 44100.0, 2.0 / 44100.0]);
        assert!(tt(0).is_empty());
    }

    #[test]
    #[should_panic(expected = "negative dimensions are not allowed (-0.2 s)")]
    fn a_negative_length_is_refused() {
        len(-0.2);
    }

    #[test]
    #[should_panic(expected = "cannot convert float NaN to integer")]
    fn a_length_must_be_a_number() {
        len(f64::NAN);
    }

    #[test]
    fn pitch() {
        assert_eq!(midi_hz(69.0), 440.0);
        assert_eq!(midi_hz(81.0), 880.0);
        assert_eq!(midi_hz(57.0), 220.0);
        assert!((midi_hz(60.0) - 261.6255653005986).abs() < 1e-12);
    }

    #[test]
    fn the_low_pass_and_the_resampler() {
        // A constant comes through the low-pass at its own level, and nothing through is nothing.
        let y = lp(&vec![1.0; 4000], 2500.0);
        assert_eq!(y.len(), 4000);
        assert!((y[3999] - 1.0).abs() < 1e-9, "{}", y[3999]);
        assert!(y[0] > 0.0 && y[0] < 0.05);
        assert!(lp(&[], 2500.0).is_empty());
        // 147 samples at 44,100 Hz are 160 at 48,000 Hz; any other length is rounded up.
        assert_eq!(to_48k(&[0.0; 147]).len(), 160);
        assert_eq!(to_48k(&[0.0; 148]).len(), 162);
        assert!(to_48k(&[]).is_empty());
        let flat = to_48k(&vec![1.0; 4410]);
        assert!((flat[2400] - 1.0).abs() < 1e-3, "{}", flat[2400]);
    }

    #[test]
    fn the_band_pass_lets_its_band_through() {
        // A constant (0 Hz) dies away; a tone in the band comes through at about its level.
        let y = bp(&vec![1.0; 20000], 1600.0, 6500.0);
        assert!(y[19999].abs() < 1e-6, "{}", y[19999]);
        let tone: Vec<f64> = tt(20000)
            .iter()
            .map(|t| dmath::sin(2.0 * dmath::PI * 3200.0 * t))
            .collect();
        let peak = bp(&tone, 1600.0, 6500.0)[10000..]
            .iter()
            .fold(0.0f64, |p, v| p.max(v.abs()));
        assert!((0.8..1.1).contains(&peak), "{peak}");
        assert!(bp(&[], 800.0, 4000.0).is_empty());
        // min1 and clip01 keep numpy's NaN.
        assert!(min1(f64::NAN).is_nan() && clip01(f64::NAN).is_nan());
        assert_eq!(
            (min1(2.0), min1(0.25), clip01(-1.0), clip01(3.0)),
            (1.0, 0.25, 0.0, 1.0)
        );
    }
}
