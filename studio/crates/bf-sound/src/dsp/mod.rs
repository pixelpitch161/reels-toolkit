//! The signal-processing layer under the synthesiser: what the kit takes from `scipy.signal` and the numpy
//! functions whose result depends on the order of their arithmetic, ported from the libraries' own sources
//! (scipy 1.18.1, numpy 2.5.3).
//!
//! - [`butter`]: Butterworth designs (`scipy.signal.butter`), as second-order sections or `b`, `a`.
//! - [`iir`]: `sosfilt` and `lfilter`.
//! - [`resample`]: `resample_poly` with its Kaiser-windowed low-pass and `upfirdn`.
//! - [`fft`]: the Studio's own radix-2 FFT; [`conv`]: `fftconvolve` on it, and `np.convolve` with the BLAS dot
//!   product under it.
//! - [`npstat`]: pairwise sums and means, `cumsum`, `linspace`, `interp`, `percentile`.
//! - [`peaks`]: `scipy.signal.find_peaks` by height and distance.
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
pub mod peaks;
pub mod resample;

pub use self::butter::{FilterKind, butter_ba, butter_sos};
pub use self::conv::{blas_dot, convolve_full, convolve_same, fftconvolve};
pub use self::iir::{Sos, lfilter, lfilter_stereo, sosfilt, sosfilt_stereo};
pub use self::npstat::{cumsum, interp, interp_lr, linspace, mean, percentile, sum, sum_axis0};
pub use self::peaks::{find_peaks, local_maxima, select_by_peak_distance};
pub use self::resample::{firwin_kaiser, resample_poly, upfirdn};
