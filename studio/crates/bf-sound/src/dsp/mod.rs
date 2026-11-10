//! The signal-processing layer under the synthesiser: what the kit takes from `scipy.signal` and the numpy
//! functions whose result depends on the order of their arithmetic, ported from the libraries' own sources
//! (scipy 1.18.1, numpy 2.5.3).
//!
//! - [`butter`]: Butterworth designs (`scipy.signal.butter`), as second-order sections or `b`, `a`.
//! - [`iir`]: `sosfilt` and `lfilter`.
//! - [`resample`]: `resample_poly` with its Kaiser-windowed low-pass and `upfirdn`.
//! - [`fft`]: the Studio's own radix-2 FFT; [`conv`]: `fftconvolve` on it, and `np.convolve(..., 'same')`.
//! - [`npstat`]: pairwise sums and means, `cumsum`, `linspace`, `interp`, `percentile`, and `find_peaks`.
//!
//! Filtering, resampling on a given filter, the sums, `linspace`, `interp` and `percentile` are plain IEEE
//! arithmetic in the libraries' order and give their bits. Filter design, the resampler's own filter and the
//! FFT go through `dmath` where numpy and scipy use the platform's maths library, and agree to rounding
//! (`tests/dsp_vectors.rs` holds the measured bars).

pub mod butter;
pub mod conv;
pub mod fft;
pub mod iir;
pub mod npstat;
pub mod resample;

pub use self::butter::{FilterKind, butter_ba, butter_sos};
pub use self::conv::{convolve_same, fftconvolve};
pub use self::iir::{Sos, lfilter, lfilter_stereo, sosfilt, sosfilt_stereo};
pub use self::npstat::{
    cumsum, find_peaks, interp, interp_lr, linspace, local_maxima, mean, percentile, select_by_peak_distance, sum,
    sum_axis0,
};
pub use self::resample::{firwin_kaiser, resample_poly, upfirdn};
