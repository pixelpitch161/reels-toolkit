//! The checks the music helpers share: numpy's `maximum`, `int()` for a length, what the filters take, the bands of
//! a sweep. (Python's `min` and `max` are `bf_core::py::{py_min, py_max}`; numpy's `clip` and the seeded noise are
//! `crate::synth::{clip, noise}`; the sample rate as a float is `crate::SRF`.)

use super::error::MusicError;
use crate::SRF;

/// The longest note or effect a helper makes, in seconds (ten minutes, about ten soundtracks). Python has no such
/// bound: it tries to allocate whatever is asked and raises `MemoryError` when the PC cannot hold it.
pub const MAX_SECONDS: f64 = 600.0;

/// `np.maximum(a, b)`: the greater, or NaN if either is.
pub(super) fn np_maximum(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if b > a {
        b
    } else {
        a
    }
}

/// Python's `int(v)` for a number of samples: cut toward zero. An error where Python raises (`v` is not a
/// number) and past [`MAX_SECONDS`].
pub(super) fn int(helper: &'static str, what: &str, v: f64) -> Result<i64, MusicError> {
    if !v.is_finite() {
        return Err(MusicError::new(helper, format!("{what} is {v}, not a number")));
    }
    if v.abs() > MAX_SECONDS * SRF {
        return Err(MusicError::new(
            helper,
            format!("{what} of {} s is longer than {MAX_SECONDS} s", v / SRF),
        ));
    }
    Ok(v as i64)
}

/// `int(seconds * SR)` as the length of a new sound. An error where numpy refuses the length (it is below zero),
/// where Python's `int` raises, and past [`MAX_SECONDS`].
pub(super) fn samples(helper: &'static str, seconds: f64) -> Result<usize, MusicError> {
    let n = int(helper, "a length", seconds * SRF)?;
    usize::try_from(n).map_err(|_| MusicError::new(helper, format!("a length of {seconds} s is below zero")))
}

/// A cutoff the filters take: strictly between 0 and half the sample rate (scipy raises outside; a NaN, which
/// scipy lets through into a filter of NaNs, is refused too).
pub(super) fn check_cut(helper: &'static str, f: f64) -> Result<(), MusicError> {
    let w = f / (SRF / 2.0);
    if w > 0.0 && w < 1.0 {
        Ok(())
    } else {
        Err(MusicError::new(
            helper,
            format!("a cutoff of {f} Hz is not between 0 and {} Hz", SRF / 2.0),
        ))
    }
}

/// Band edges the filters take: `0 < lo < hi < SR / 2` (scipy raises otherwise).
pub(super) fn check_band(helper: &'static str, lo: f64, hi: f64) -> Result<(), MusicError> {
    let (a, b) = (lo / (SRF / 2.0), hi / (SRF / 2.0));
    if a > 0.0 && a < b && b < 1.0 {
        Ok(())
    } else {
        Err(MusicError::new(
            helper,
            format!(
                "a band from {lo} to {hi} Hz does not rise between 0 and {} Hz",
                SRF / 2.0
            ),
        ))
    }
}

/// A sound that goes through a filter needs a sample (scipy raises on an empty signal).
pub(super) fn check_some(helper: &'static str, n: usize, seconds: f64) -> Result<(), MusicError> {
    if n > 0 {
        Ok(())
    } else {
        Err(MusicError::new(
            helper,
            format!("{seconds} s is under one sample: there is nothing to filter"),
        ))
    }
}

/// The edges of band `k` of `bands` over `n` samples: `int(k * n / bands)`, `int((k + 1) * n / bands)`.
pub(super) fn band_span(k: usize, n: usize, bands: usize) -> (usize, usize) {
    (
        ((k * n) as f64 / bands as f64) as usize,
        (((k + 1) * n) as f64 / bands as f64) as usize,
    )
}
