//! Synthesis and mixing: oscillators, filters, envelopes, reverb, buses, master and the voice and theme traits.
//!
//! The kit's soundtrack is synthesised with numpy and scipy; this crate rebuilds it sample for sample.
//!
//! - [`dsp`]: the signal-processing layer (filter design and filtering, resampling, FFT convolution, and the
//!   numpy functions whose result depends on the order of their arithmetic).
//! - [`synth`]: `reelkit/audio.py`'s generators (drums, effects, tonal voices, reverb, tape stop).
//! - [`mix`]: named stereo buses and the placing of sounds on them.
//! - [`wav`]: 16-bit `.wav` output.
//! - [`voice`]: the instrument-voice trait, the Pixel Pitch voices' shared noise stream and what those voices
//!   have in common (the instruments' own synthesisers are in `bf-content`).
//! - [`music`]: `kit/music.py`'s helpers (the cached, level-matched notes, the shaping and the shared sound
//!   effects) and the context a soundtrack is built in.
//! - [`traits`]: what a fighter's part and a theme's bed provide to the score (`Sound`, `Bed`).
//! - [`mod@score`]: the order a soundtrack is built in, `kit/music.py score()`.
//! - [`mod@master`]: the master chain (bus levels, reverb, the fold onto the loop point, compressor, limiter).

pub mod dsp;
pub mod master;
pub mod mix;
pub mod music;
pub mod score;
pub mod synth;
pub mod traits;
pub mod voice;
pub mod wav;

pub use self::dsp::{
    FilterKind, Sos, butter_ba, butter_sos, convolve_same, fftconvolve, interp, interp_lr, lfilter, percentile,
    resample_poly, sosfilt,
};
pub use self::master::{Levels, Soundtrack, master};
pub use self::mix::{Mix, Sig};
pub use self::music::{MusicCtx, MusicError};
pub use self::score::score;
pub use self::traits::{Bed, Sound};
pub use self::voice::{PpNoise, Voice};
pub use self::wav::write_wav16;

/// The sample rate of everything the Studio synthesises, in Hz.
pub const SR: usize = 48_000;

/// [`SR`] as a float, for the sample arithmetic (`t * SR`, `n / SR`). The one home of this constant: the music
/// helpers, the fighters' parts and the themes use it. (The Pixel Pitch voices' own rate is
/// [`voice::pp::SRF`].)
pub const SRF: f64 = SR as f64;

/// The length of a soundtrack in samples: 2,860 AAC frames of 1,024 samples, 61.013 s.
pub const NA: usize = 2_928_640;
