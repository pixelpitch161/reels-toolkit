//! Synthesis and mixing: oscillators, filters, envelopes, reverb, buses, master and the voice and theme traits.
//!
//! The kit's soundtrack is synthesised with numpy and scipy; this crate rebuilds it sample for sample.
//!
//! - [`dsp`]: the signal-processing layer (filter design and filtering, resampling, FFT convolution, and the
//!   numpy functions whose result depends on the order of their arithmetic).
//! - [`synth`]: `reelkit/audio.py`'s generators (drums, effects, tonal voices, reverb, tape stop).
//! - [`mix`]: named stereo buses and the placing of sounds on them.
//! - [`wav`]: 16-bit `.wav` output.

pub mod dsp;
pub mod mix;
pub mod synth;
pub mod wav;

pub use self::dsp::{
    FilterKind, Sos, butter_ba, butter_sos, convolve_same, fftconvolve, interp, interp_lr, lfilter, percentile,
    resample_poly, sosfilt,
};
pub use self::mix::{Mix, Sig};
pub use self::wav::write_wav16;

/// The sample rate of everything the Studio synthesises, in Hz.
pub const SR: usize = 48_000;

/// The length of a soundtrack in samples: 2,860 AAC frames of 1,024 samples, 61.013 s.
pub const NA: usize = 2_928_640;
