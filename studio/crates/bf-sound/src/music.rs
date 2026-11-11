//! `projects/ballfight/kit/music.py`'s helpers, with the same names: the context a soundtrack is built in, the
//! fighters' cached, level-matched notes, the shaping of a note (slides, bends, vibrato, the plunger "wah", the
//! tone that thins and dirties as a fighter's HP drops), the note-thinning hashes, and the shared sound effects.
//!
//! Python's module-level state (the run, the timeline, the two fighters, the note cache `_VC`, the gains `_GAIN`,
//! the series' noise generator) is one [`MusicCtx`] here, made once per soundtrack build. What needs none of it
//! is a free function.
//!
//! **Notes are made one at a time, in the order they are first asked for.** The Pixel Pitch voices share one
//! noise stream ([`crate::voice::PpNoise`]), so the order of first-time notes is part of the sound.
//! [`MusicCtx::note`] holds the stream and the cache behind one lock, synthesises a note the first time its key
//! is asked for and hands out the same samples afterwards without drawing anything.
//!
//! **Defaults.** Rust has no default arguments: every function takes all of Python's arguments in Python's
//! order, and each default is a constant named `<FUNCTION>_<ARGUMENT>` (`NOTE_CAP`, `RISER_OVER`, ...).
//! `colour`'s keyword arguments are [`ColourArgs`], whose `Default` holds Python's.
//!
//! **Errors.** A helper that makes or shapes samples returns `Err` exactly where Python raises (an empty note
//! handed to a filter, a band of a sweep with no sample of its own, a cutoff outside the filter's range, a length
//! that is not a number), instead of reaching the panics of [`crate::dsp`]. One bound is this module's own: a
//! length over [`MAX_SECONDS`] is refused (Python would try to allocate it). [`MusicCtx::note`] refuses a few
//! values more, listed there. The number helpers (`h01`, `Thinner`, ...) panic where Python raises, as the rest
//! of the crate does. If a voice panics while a note is being made, the build's state is not handed out again:
//! every later call on that context is an error.
//!
//! Each sample is computed with the operations of the numpy expression it ports, in the same order; `sin`, `exp`,
//! `pow` and `tanh` come from `dmath`.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};

use bf_core::dmath::{self, PI};
use bf_core::event::{Event, Value};
use bf_core::fightfile::{FightFile, INTRO_BARS};
use bf_core::harmony::{self, Harmony};
use bf_core::py::{fmod_py, ifloor_div, round_half_even, round_nd};
use bf_core::recording::{Array, Recording};
use bf_core::rng::Generator;
use bf_core::show::Show;
use bf_sim::roster::FighterCfg;

use crate::SR;
use crate::dsp;
use crate::mix::{Mix, Sig};
use crate::synth::{self, FILTER_O};
use crate::traits::Bed;
use crate::voice::{PpNoise, Voice};

/// The sample rate as a float.
const SRF: f64 = SR as f64;

/// `note`'s default held length, in seconds.
pub const NOTE_DUR: f64 = 0.18;
/// `note`'s default `cap`: the length the note is cut or padded to, in seconds.
pub const NOTE_CAP: f64 = 0.6;
/// `note`'s default velocity.
pub const NOTE_VEL: f64 = 0.85;
/// `fit`'s default fade, in seconds.
pub const FIT_FADE: f64 = 0.04;
/// `vibrato`'s default depth, in semitones.
pub const VIBRATO_DEPTH: f64 = 0.35;
/// `vibrato`'s default rate, in Hz.
pub const VIBRATO_RATE: f64 = 5.5;
/// `vibrato`'s default delay, in seconds.
pub const VIBRATO_DELAY: f64 = 0.15;
/// `colour`'s default `drive_k`.
pub const COLOUR_DRIVE_K: f64 = 1.0;
/// `colour`'s default `cut0`: the high-pass cutoff at full HP, in Hz.
pub const COLOUR_CUT0: f64 = 90.0;
/// `colour`'s default `cut_span`: how far the cutoff rises by 0 HP, in Hz.
pub const COLOUR_CUT_SPAN: f64 = 520.0;
/// `colour`'s default `drive`.
pub const COLOUR_DRIVE: f64 = 4.5;
/// `colour`'s default `low`: not a bass line.
pub const COLOUR_LOW: bool = false;
/// `colour`'s default `low_cut`: a bass line's cutoff at full HP and how far it rises, in Hz.
pub const COLOUR_LOW_CUT: (f64, f64) = (30.0, 70.0);
/// `clank`'s default noise seed.
pub const CLANK_SEED: u64 = 0;
/// `clank`'s default length, in seconds.
pub const CLANK_DUR: f64 = 0.4;
/// `clank`'s default partials: (frequency ratio, decay in seconds).
pub const CLANK_DECAYS: [(f64, f64); 3] = [(1.0, 0.12), (2.76, 0.06), (5.4, 0.03)];
/// `hiss`'s default length, in seconds.
pub const HISS_DUR: f64 = 0.09;
/// `hiss`'s default noise seed.
pub const HISS_SEED: u64 = 0;
/// `hiss`'s default lower band edge, in Hz.
pub const HISS_LO: f64 = 3500.0;
/// `hiss`'s default upper band edge, in Hz.
pub const HISS_HI: f64 = 10000.0;
/// `hiss`'s default decay, in seconds.
pub const HISS_DEC: f64 = 0.03;
/// `roar`'s default noise seed.
pub const ROAR_SEED: u64 = 3;
/// `thunder`'s default length, in seconds.
pub const THUNDER_DUR: f64 = 1.6;
/// `thunder`'s default noise seed.
pub const THUNDER_SEED: u64 = 0;
/// `thunder`'s default `crack`: with the crack on top.
pub const THUNDER_CRACK: bool = true;
/// `riser`'s default `over`: how far past its end the riser runs, in seconds.
pub const RISER_OVER: f64 = 0.012;
/// `riser`'s default noise seed.
pub const RISER_SEED: u64 = 11;

/// The longest note or effect a helper makes, in seconds (ten minutes, about ten soundtracks). Python has no such
/// bound: it tries to allocate whatever is asked and raises `MemoryError` when the PC cannot hold it.
pub const MAX_SECONDS: f64 = 600.0;

// ---------------------------------------------------------------- errors

/// Why a music helper could not make its sound: the helper's name and the problem, in plain words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicError {
    helper: &'static str,
    problem: String,
}

impl MusicError {
    /// An error of the helper `helper` (the kit's name for it, such as `note` or `wah`).
    pub fn new(helper: &'static str, problem: impl Into<String>) -> MusicError {
        MusicError {
            helper,
            problem: problem.into(),
        }
    }

    /// The helper that failed.
    pub fn helper(&self) -> &'static str {
        self.helper
    }

    /// What was wrong.
    pub fn problem(&self) -> &str {
        &self.problem
    }
}

impl fmt::Display for MusicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.helper, self.problem)
    }
}

impl std::error::Error for MusicError {}

/// What a helper that makes or shapes samples returns.
pub type Samples = Result<Vec<f64>, MusicError>;

// ---------------------------------------------------------------- Python's numbers

/// Python's `min(a, b)` on floats: `a` unless `b` is smaller.
fn py_min(a: f64, b: f64) -> f64 {
    if b < a { b } else { a }
}

/// Python's `max(a, b)` on floats: `a` unless `b` is greater.
fn py_max(a: f64, b: f64) -> f64 {
    if b > a { b } else { a }
}

/// `np.clip(x, lo, hi)` (a NaN stays a NaN).
fn clip(x: f64, lo: f64, hi: f64) -> f64 {
    x.clamp(lo, hi)
}

/// `np.maximum(a, b)`: the greater, or NaN if either is.
fn np_maximum(a: f64, b: f64) -> f64 {
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
fn int(helper: &'static str, what: &str, v: f64) -> Result<i64, MusicError> {
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
fn samples(helper: &'static str, seconds: f64) -> Result<usize, MusicError> {
    let n = int(helper, "a length", seconds * SRF)?;
    usize::try_from(n).map_err(|_| MusicError::new(helper, format!("a length of {seconds} s is below zero")))
}

/// A cutoff the filters take: strictly between 0 and half the sample rate (scipy raises outside; a NaN, which
/// scipy lets through into a filter of NaNs, is refused too).
fn check_cut(helper: &'static str, f: f64) -> Result<(), MusicError> {
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
fn check_band(helper: &'static str, lo: f64, hi: f64) -> Result<(), MusicError> {
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
fn check_some(helper: &'static str, n: usize, seconds: f64) -> Result<(), MusicError> {
    if n > 0 {
        Ok(())
    } else {
        Err(MusicError::new(
            helper,
            format!("{seconds} s is under one sample: there is nothing to filter"),
        ))
    }
}

/// `np.random.default_rng(seed).normal(0, 1, n)`.
fn noise(seed: u64, n: usize) -> Vec<f64> {
    let mut x = vec![0.0; n];
    Generator::new(seed).fill_normal(0.0, 1.0, &mut x);
    x
}

// ---------------------------------------------------------------- the 16th grid

/// `q16(t)` on a grid whose sixteenth is `s16` seconds: the nearest 16th, `round(t / S16) * S16` (Python's
/// `round`: halves go to the even 16th). A `t` that is not a number comes back as NaN (Python raises).
pub fn q16(t: f64, s16: f64) -> f64 {
    // Python's round gives an int, which has no minus zero: `+ 0.0` turns -0.0 into 0.0.
    (round_half_even(t / s16) + 0.0) * s16
}

// ---------------------------------------------------------------- thinning (a hurt fighter's line loses notes)

/// `int(round(x * 1000)) & 0xFFFFFFFFFFFFFFFF`: what the two hashes read of each argument. Whole numbers go in as
/// floats, which is exact up to 2**53 / 1000.
///
/// # Panics
/// Where Python raises: `x` is NaN or infinite.
fn milli(x: f64) -> u64 {
    let v = round_half_even(x * 1000.0);
    assert!(v.is_finite(), "cannot convert float {v} to integer");
    if v.abs() >= 1.6e35 {
        // A float this large (anything from 2**116, about 8.3e34, up) is a multiple of 2**64: its low 64 bits
        // are zero.
        0
    } else {
        // Exact (a rounded float this small is a whole number an i128 holds); the cast to u64 keeps the low 64
        // bits in two's complement, which is what Python's `&` does to a negative int.
        (v as i128) as u64
    }
}

/// `h01(*xs)`: a deterministic splitmix hash of the arguments in [0, 1) (#37's fix: the old linear hash fell
/// below the threshold on runs of consecutive slots and silenced whole phrases). Each argument goes in as
/// `int(round(x * 1000))`.
///
/// # Panics
/// Where Python raises: an argument is NaN or infinite.
pub fn h01(xs: &[f64]) -> f64 {
    let mut v: u64 = 0x9E37_79B9_7F4A_7C15;
    for &x in xs {
        v ^= milli(x);
        v = v.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = v;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        v = z ^ (z >> 31);
    }
    (v >> 11) as f64 / 9_007_199_254_740_992.0
}

/// `Thinner(seed)`: drops notes as a fighter's HP falls, with the chance `min(0.30, 0.55 (1 - hp)^1.6)` and
/// never two in a row (#41).
#[derive(Clone, Debug, PartialEq)]
pub struct Thinner {
    seed: f64,
    last: bool,
}

impl Thinner {
    /// A thinner with its own hash `seed` (the FLUTE's is 5, the TROMBONE's 7).
    pub fn new(seed: f64) -> Thinner {
        Thinner { seed, last: false }
    }

    /// `keep(slot, hp01)`: whether the note in `slot` plays, for a fighter at `hp01` of its HP (0 to 1). Call it
    /// for every slot in order: it remembers whether it dropped the one before.
    ///
    /// # Panics
    /// Where Python raises: `hp01` is above 1 (the power of a negative number is complex in Python), or `slot`
    /// is NaN or infinite when it is hashed.
    pub fn keep(&mut self, slot: f64, hp01: f64) -> bool {
        assert!(
            hp01 <= 1.0 || hp01.is_nan(),
            "Thinner.keep: hp01 is {hp01}, above 1 (Python cannot compare the complex power)"
        );
        let p = py_min(0.30, 0.55 * dmath::pow(1.0 - hp01, 1.6));
        let drop = !self.last && h01(&[slot, self.seed]) < p;
        self.last = drop;
        !drop
    }
}

/// `h01_37(*xs)`: #37's splitmix64 mixer in [0, 1] (the reed fighters' thinning, as #37 built it).
///
/// # Panics
/// Where Python raises: an argument is NaN or infinite.
pub fn h01_37(xs: &[f64]) -> f64 {
    let mut z: u64 = 0x9E37_79B9_7F4A_7C15;
    for &x in xs {
        z = (z ^ milli(x)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
    }
    // Python divides the int by float(1 << 64): the int is rounded to the nearest float first, as `as f64` does.
    z as f64 / 18_446_744_073_709_551_616.0
}

/// `thin_37(slot, key, hp, prev_dropped)`: drop this note? (#37: up to 30 % of a hurt fighter's notes, never two
/// in a row.) `hp` is the fighter's HP from 0 to 1.
///
/// # Panics
/// Where Python raises: `hp` is above 1 and the note before was not dropped, or `slot` or `key` is NaN or
/// infinite when it is hashed.
pub fn thin_37(slot: f64, key: f64, hp: f64, prev_dropped: bool) -> bool {
    if prev_dropped {
        return false;
    }
    assert!(
        hp <= 1.0 || hp.is_nan(),
        "thin_37: hp is {hp}, above 1 (Python cannot compare the complex power)"
    );
    h01_37(&[slot, key]) < 0.30 * dmath::pow(1.0 - hp, 1.6)
}

// ---------------------------------------------------------------- shaping

/// Why `fit` refuses a note of `len` samples with a fade of `fade` seconds.
fn no_room(len: usize, fade: f64) -> String {
    format!(
        "{len} sample(s) with a fade of {fade} s leave no room for a fade (3 samples and a fade of one sample are \
         the least)"
    )
}

/// `fit(x, cap, fade)`: `x` cut or padded with silence to `int(cap * SR)` samples, its end faded out over
/// `fade` seconds (over a third of it, if that is shorter). A `cap` below zero cuts that much off the end
/// instead, as Python's slice does.
///
/// # Errors
/// Where Python raises: the result would have 1 or 2 samples, or `fade` is under one sample (there is no room
/// for a fade), `fade` is below zero, or `cap` or `fade` is not a number.
pub fn fit(x: &[f64], cap: f64, fade: f64) -> Samples {
    const H: &str = "fit";
    let n = int(H, "cap", cap * SRF)?;
    let fade_n = int(H, "fade", fade * SRF)?;
    // x[:n]
    let len = x.len() as i64;
    let keep = if n >= 0 { n.min(len) } else { (len + n).max(0) };
    let mut y = x[..keep as usize].to_vec();
    if (y.len() as i64) < n {
        y.resize(n as usize, 0.0);
    }
    let fd = (y.len() as i64 / 3).min(fade_n);
    if fd < 0 {
        return Err(MusicError::new(H, format!("a fade of {fade} s is below zero")));
    }
    if y.is_empty() {
        return Ok(y);
    }
    if fd == 0 {
        // x[-0:] is the whole note, and numpy cannot scale it by a ramp of no samples.
        return Err(MusicError::new(H, no_room(y.len(), fade)));
    }
    let fd = fd as usize;
    let start = y.len() - fd;
    for (v, ramp) in y[start..].iter_mut().zip(dsp::linspace(1.0, 0.0, fd)) {
        *v *= ramp;
    }
    Ok(y)
}

/// The time warp the three pitch shapers share: the note is read at a rate of `2 ** (off / 12)` (`off` in
/// semitones, one per sample), `np.interp(cumsum(rate) - rate[0], arange(n), x, right=0.0)`.
fn warp(helper: &'static str, x: &[f64], off: impl Fn(f64) -> f64) -> Samples {
    if x.is_empty() {
        return Err(MusicError::new(helper, "the note has no samples"));
    }
    let rate: Vec<f64> = synth::tt(x.len())
        .iter()
        .map(|&t| dmath::pow(2.0, off(t) / 12.0))
        .collect();
    let first = rate[0];
    let pos: Vec<f64> = dsp::cumsum(&rate).iter().map(|c| c - first).collect();
    let at: Vec<f64> = (0..x.len()).map(|i| i as f64).collect();
    Ok(dsp::interp_lr(&pos, &at, x, None, Some(0.0)))
}

/// `gliss(x, semis, dur)`: slide into the note: start `semis` below (negative: above) and glide to pitch over
/// `dur` seconds (at least 1 ms). A time warp, so what is read past the end of the note is silence.
///
/// # Errors
/// Where Python raises: `x` is empty.
pub fn gliss(x: &[f64], semis: f64, dur: f64) -> Samples {
    let dur = py_max(dur, 1e-3);
    warp("gliss", x, |t| -semis * dmath::pow(1.0 - clip(t / dur, 0.0, 1.0), 1.5))
}

/// `bend(x, semis, t0, t1)`: bend the pitch by `semis` between `t0` and `t1` seconds into the note (a sag, a
/// falling K.O.).
///
/// # Errors
/// Where Python raises: `x` is empty.
pub fn bend(x: &[f64], semis: f64, t0: f64, t1: f64) -> Samples {
    let span = py_max(1e-3, t1 - t0);
    warp("bend", x, |t| semis * clip((t - t0) / span, 0.0, 1.0))
}

/// `vibrato(x, depth, rate, delay)`: a vibrato of `depth` semitones at `rate` Hz that comes in over 0.2 s from
/// `delay` seconds into the note.
///
/// # Errors
/// Where Python raises: `x` is empty.
pub fn vibrato(x: &[f64], depth: f64, rate: f64, delay: f64) -> Samples {
    let w = 2.0 * PI * rate;
    warp("vibrato", x, |t| {
        depth * dmath::sin(w * t) * clip((t - delay) / 0.2, 0.0, 1.0)
    })
}

/// The bands of [`wah`].
const WAH_BANDS: usize = 48;
/// The bands of [`riser`].
const RISER_BANDS: usize = 20;

/// The edges of band `k` of `bands` over `n` samples: `int(k * n / bands)`, `int((k + 1) * n / bands)`.
fn band_span(k: usize, n: usize, bands: usize) -> (usize, usize) {
    (
        ((k * n) as f64 / bands as f64) as usize,
        (((k + 1) * n) as f64 / bands as f64) as usize,
    )
}

/// `wah(x, opens)`: a plunger mute: a band-pass whose centre opens from 450 to 1,850 Hz over each `(t0, t1)`
/// span of `opens` (and stays open for a quarter of a second after it): the "wah" (#34). The note is filtered
/// in 48 bands in time, each with a run-in of 4,096 samples, and comes out 1.6 times as loud.
///
/// # Errors
/// Where Python raises: `x` has fewer than 48 samples (a band with no sample of its own), or a span of `opens`
/// makes a band's centre something that is not a number (a span that starts at minus infinity does).
pub fn wah(x: &[f64], opens: &[(f64, f64)]) -> Samples {
    const H: &str = "wah";
    let n = x.len();
    if n < WAH_BANDS {
        return Err(MusicError::new(
            H,
            format!("{n} sample(s) are too few for {WAH_BANDS} bands"),
        ));
    }
    let t = synth::tt(n);
    let mut fc = vec![450.0; n];
    for &(t0, t1) in opens {
        let span = py_max(1e-3, t1 - t0);
        for (f, &t) in fc.iter_mut().zip(&t) {
            let u = clip((t - t0) / span, 0.0, 1.0);
            let s = dmath::sin(PI * 0.5 * u);
            let open = if t >= t0 && t < t1 + 0.25 {
                450.0 + 1400.0 * (s * s)
            } else {
                450.0
            };
            *f = np_maximum(*f, open);
        }
    }
    let mut out = vec![0.0; n];
    for k in 0..WAH_BANDS {
        let (a, b) = band_span(k, n, WAH_BANDS);
        let c = fc[(a + b) / 2];
        let hi = py_min(c * 1.8, SRF / 2.0 - 200.0);
        check_band(H, c * 0.55, hi)?;
        let band = synth::bp(&x[a.saturating_sub(4096)..b], c * 0.55, hi, FILTER_O);
        // `[-(b - a):]` of the filtered run.
        out[a..b].copy_from_slice(&band[band.len() - (b - a)..]);
    }
    Ok(out.iter().map(|v| v * 1.6).collect())
}

/// `colour`'s keyword arguments: how a fighter's tone thins and dirties as its HP drops. `Default` holds
/// Python's defaults (and `kit/fighters/base.py`'s `Sound.colour_args`, which are the same three).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColourArgs {
    /// `cut0`: the high-pass cutoff at full HP, in Hz.
    pub cut0: f64,
    /// `cut_span`: how far the cutoff rises by 0 HP, in Hz.
    pub cut_span: f64,
    /// `drive`: how hard the tone is driven at 0 HP.
    pub drive: f64,
    /// `low`: a bass line, which keeps its fundamental (the cutoff follows `low_cut` instead).
    pub low: bool,
    /// `low_cut`: a bass line's cutoff at full HP, and how far it rises by 0 HP, in Hz.
    pub low_cut: (f64, f64),
}

impl Default for ColourArgs {
    fn default() -> ColourArgs {
        ColourArgs {
            cut0: COLOUR_CUT0,
            cut_span: COLOUR_CUT_SPAN,
            drive: COLOUR_DRIVE,
            low: COLOUR_LOW,
            low_cut: COLOUR_LOW_CUT,
        }
    }
}

/// `colour` for a fighter at `own` HP against one at `other` HP (both 0 to 100, as `hp_at` gives them): the
/// signal thinner (high-passed) and dirtier (driven through `tanh`) the lower `own` is, then `(signal, gain,
/// lead)`: the leader plays up to +3.5 dB, the loser down to -4 dB (`gain` is that as a factor; it is not
/// applied), and `lead` is `(own - other) / 100`. [`MusicCtx::colour`] reads the two HP values from the fight.
///
/// # Errors
/// Where Python raises: `x` is empty, `own` is above 100, or the cutoff the arguments give is not between 0 and
/// half the sample rate.
pub fn colour_hp(
    x: &[f64],
    own: f64,
    other: f64,
    drive_k: f64,
    args: &ColourArgs,
) -> Result<(Vec<f64>, f64, f64), MusicError> {
    const H: &str = "colour";
    let hp = own / 100.0;
    let lead = (own - other) / 100.0;
    let down = 1.0 - hp;
    if down < 0.0 {
        return Err(MusicError::new(H, format!("an HP of {own} is above 100")));
    }
    let thin = dmath::pow(down, 1.6);
    let cut = if args.low {
        args.low_cut.0 + args.low_cut.1 * thin
    } else {
        args.cut0 + args.cut_span * thin
    };
    check_cut(H, cut)?;
    if x.is_empty() {
        return Err(MusicError::new(H, "the note has no samples"));
    }
    let dr = 1.0 + args.drive * drive_k * (down * down);
    let full = dmath::tanh(dr * 1.2);
    let level = 1.0 / (1.0 + 0.15 * (dr - 1.0));
    let y = synth::hp(x, cut, FILTER_O)
        .iter()
        .map(|&v| dmath::tanh(dr * v * 1.2) / full * level)
        .collect();
    let g = dmath::pow(10.0, clip(6.0 * lead, -4.0, 3.5) / 20.0);
    Ok((y, g, lead))
}

// ---------------------------------------------------------------- shared sound effects

/// `clank(m, seed, dur, decays)`: metal on canvas: a short metallic ring on MIDI note `m`, a sine for each
/// `(frequency ratio, decay)` of `decays`, with a click of band-passed noise on its front.
///
/// # Errors
/// Where Python raises: `dur` is under one sample or not a number.
pub fn clank(m: f64, seed: u64, dur: f64, decays: &[(f64, f64)]) -> Samples {
    const H: &str = "clank";
    let n = samples(H, dur)?;
    check_some(H, n, dur)?;
    let t = synth::tt(n);
    let f = synth::mtof(m);
    // Python's `sum(...)` over the partials starts from 0.
    let mut x = vec![0.0; n];
    for &(r, d) in decays {
        let w = 2.0 * PI * f * r;
        for (v, &t) in x.iter_mut().zip(&t) {
            *v += dmath::sin(w * t) * dmath::exp(-t / d);
        }
    }
    let click = synth::bp(&noise(seed, n), 2000.0, 8000.0, FILTER_O);
    Ok((0..n)
        .map(|i| x[i] + 0.3 * click[i] * dmath::exp(-t[i] / 0.005))
        .collect())
}

/// `hiss(dur, seed, lo, hi, dec)`: a puff of noise between `lo` and `hi` Hz, decaying over `dec` seconds.
///
/// # Errors
/// Where Python raises: `dur` is under one sample or not a number, or the band does not rise between 0 and half
/// the sample rate.
pub fn hiss(dur: f64, seed: u64, lo: f64, hi: f64, dec: f64) -> Samples {
    const H: &str = "hiss";
    let n = samples(H, dur)?;
    check_some(H, n, dur)?;
    check_band(H, lo, hi)?;
    let t = synth::tt(n);
    let band = synth::bp(&noise(seed, n), lo, hi, FILTER_O);
    Ok((0..n).map(|i| band[i] * dmath::exp(-t[i] / dec)).collect())
}

/// `roar(dur, seed)`: the crowd: two bands of noise that swell over a quarter of a second and die away.
///
/// # Errors
/// Where Python raises: `dur` is under one sample or not a number.
pub fn roar(dur: f64, seed: u64) -> Samples {
    const H: &str = "roar";
    let n = samples(H, dur)?;
    check_some(H, n, dur)?;
    let x = noise(seed, n);
    let low = synth::bp(&x, 300.0, 2600.0, FILTER_O);
    let high = synth::bp(&x, 900.0, 5000.0, FILTER_O);
    let t = synth::tt(n);
    Ok((0..n)
        .map(|i| {
            let since = t[i] - 0.6;
            let late = if since > 0.0 { since } else { 0.0 };
            let env = clip(t[i] / 0.25, 0.0, 1.0) * dmath::exp(-late / (dur * 0.45));
            (low[i] * 0.5 + high[i] * 0.25) * env
        })
        .collect())
}

/// `thunder(dur, seed, crack)`: a rolling thunder rumble, with a crack on top if `crack` (#37). The rumble and
/// the crack take their noise from one generator, the rumble first.
///
/// # Errors
/// Where Python raises: `dur` is under one sample or not a number.
pub fn thunder(dur: f64, seed: u64, crack: bool) -> Samples {
    const H: &str = "thunder";
    let n = samples(H, dur)?;
    check_some(H, n, dur)?;
    let t = synth::tt(n);
    let mut r = Generator::new(seed);
    let mut draw = vec![0.0; n];
    r.fill_normal(0.0, 1.0, &mut draw);
    let low = synth::lp(&draw, 140.0, FILTER_O);
    let phase = seed as f64;
    let mut x: Vec<f64> = (0..n)
        .map(|i| {
            let rumble = low[i] * 3.0 * clip(t[i] / 0.03, 0.0, 1.0) * dmath::exp(-t[i] / (dur * 0.35));
            rumble * (0.7 + 0.3 * dmath::sin(2.0 * PI * 3.1 * t[i] + phase))
        })
        .collect();
    if crack {
        r.fill_normal(0.0, 1.0, &mut draw);
        let high = synth::bp(&draw, 1500.0, 9000.0, FILTER_O);
        for i in 0..n {
            x[i] += high[i] * dmath::exp(-t[i] / 0.03) * 0.9;
        }
    }
    Ok(x)
}

/// `riser(dur, over, seed)`: a noise and saw riser that builds right up to (and `over` seconds past) its end:
/// no fade, so the loop seam stays loud. The noise is filtered in 20 bands in time that climb from 300 Hz to
/// 6 kHz, each with a run-in of 2,048 samples.
///
/// # Errors
/// Where Python raises: `dur + over` is under 20 samples (a band with no sample of its own) or not a number, or
/// a band's centre is not a number (a `dur` of 0 with fewer than 40 samples: the first sample divides 0 by 0).
/// With a `dur` of 0 and more samples than that Python gives NaN for every sample, and so does this.
pub fn riser(dur: f64, over: f64, seed: u64) -> Samples {
    const H: &str = "riser";
    let n = samples(H, dur + over)?;
    if n < RISER_BANDS {
        return Err(MusicError::new(
            H,
            format!("{n} sample(s) are too few for {RISER_BANDS} bands"),
        ));
    }
    let t = synth::tt(n);
    let u: Vec<f64> = t.iter().map(|&t| clip(t / dur, 0.0, 1.05)).collect();
    let x = noise(seed, n);
    let f: Vec<f64> = u.iter().map(|&u| 300.0 * dmath::pow(20.0, u)).collect();
    let rate: Vec<f64> = u.iter().map(|&u| 55.0 * dmath::pow(2.0, u * 2.5)).collect();
    let sweep: Vec<f64> = dsp::cumsum(&rate)
        .iter()
        .map(|&c| {
            let ph = 2.0 * PI * c / SRF;
            2.0 * fmod_py(ph / (2.0 * PI), 1.0) - 1.0
        })
        .collect();
    let mut out = vec![0.0; n];
    for k in 0..RISER_BANDS {
        let (a, b) = band_span(k, n, RISER_BANDS);
        let fc = f[(n - 1).min((a + b) / 2)];
        let hi = py_min(fc * 1.4, SRF / 2.0 - 200.0);
        check_band(H, fc * 0.7, hi)?;
        let band = synth::bp(&x[a.saturating_sub(2048)..b], fc * 0.7, hi, FILTER_O);
        out[a..b].copy_from_slice(&band[band.len() - (b - a)..]);
    }
    let low = synth::lp(&sweep, 2500.0, FILTER_O);
    Ok((0..n).map(|i| (out[i] + 0.35 * low[i]) * (u[i] * u[i])).collect())
}

// ---------------------------------------------------------------- the context

/// One first-time note: the arguments `note` was called with when it synthesised it.
#[derive(Clone, Debug, PartialEq)]
pub struct NoteCall {
    /// The voice's roster name.
    pub name: String,
    /// The MIDI note, `int(midi)`.
    pub midi: i64,
    /// The held length, as passed.
    pub dur: f64,
    /// The length the note was cut or padded to, as passed.
    pub cap: f64,
    /// The velocity, as passed.
    pub vel: f64,
}

/// One window registered with [`MusicCtx::line_override`]: from `t0` to `t1` the fighter's line plays `notes`
/// instead, `(beat from tz, midi note or a rest, beats)`.
#[derive(Clone, Debug, PartialEq)]
pub struct LineOverride {
    pub t0: f64,
    pub t1: f64,
    pub tz: f64,
    pub notes: Vec<(f64, Option<i32>, f64)>,
}

/// The key `note` caches by: `(name, int(midi), round(dur, 3), cap, round(vel, 2))`, the floats by their bits
/// (`-0.0` as `0.0`, which Python's dict takes for the same key).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct NoteKey {
    name: String,
    midi: i64,
    dur: u64,
    cap: u64,
    vel: u64,
}

fn key_bits(x: f64) -> u64 {
    if x == 0.0 { 0 } else { x.to_bits() }
}

/// What a soundtrack build changes as it goes: Python's `drums60_lowpoly._rng`, `_VC`, `_GAIN` and
/// `T.LINE_OVERRIDE`.
struct State {
    noise: PpNoise,
    notes: BTreeMap<NoteKey, Arc<[f64]>>,
    gains: BTreeMap<String, f64>,
    first: Vec<NoteCall>,
    line_overrides: BTreeMap<String, Vec<LineOverride>>,
}

impl State {
    fn new(noise: PpNoise) -> State {
        State {
            noise,
            notes: BTreeMap::new(),
            gains: BTreeMap::new(),
            first: Vec::new(),
            line_overrides: BTreeMap::new(),
        }
    }
}

/// What `kit/music.py` knows while it builds a soundtrack: the show (`T`, `T_KO`, `WIN`, ...), the recording
/// (`RUN`, `EV`), the fight file, the harmony, the two fighters' roster entries (`CFG`) and voices, and, behind
/// one lock, the note cache, the instruments' gains and the Pixel Pitch noise stream.
///
/// Make one per soundtrack build. It can be shared between threads, but the notes of a build must be asked for
/// from one thread, in the score's order: the order of first-time notes is part of the sound.
pub struct MusicCtx<'a> {
    /// The show: the grid (`T`), `NAMES`, `WIN`, `LOSE`, `T_KO`, `T_HS`, `T_SOLO`, the captions.
    pub show: &'a Show,
    /// The recording: `RUN` and its events (`EV`).
    pub rec: &'a Recording,
    /// The fight file: the music settings (roles, lines, solos, title hits).
    pub ff: &'a FightFile,
    /// The timeline's harmony: the key, the scale, one chord per bar, the loop.
    pub harmony: Harmony,
    /// `CFG`: the two fighters' roster entries, left corner first.
    pub cfg: [&'a FighterCfg; 2],
    /// The two fighters' voices, in the same order. `note` finds a voice by its fighter's `pp` name.
    pub voices: [Arc<dyn Voice>; 2],
    /// `RUN['HP']`: both fighters' HP at every recorded frame, frame after frame.
    hp: &'a [f64],
    state: Mutex<State>,
}

impl fmt::Debug for MusicCtx<'_> {
    /// The fighters and their voices. What sits behind the lock is not printed: formatting a context must not
    /// wait for a note that is being made.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MusicCtx")
            .field("names", &self.show.names)
            .field("voices", &[self.voices[0].name(), self.voices[1].name()])
            .finish_non_exhaustive()
    }
}

/// `reelkit.config.ease(x)`: the quintic ease-in-out on [0, 1].
fn ease(x: f64) -> f64 {
    let x = py_min(1.0, py_max(0.0, x));
    x * x * x * (x * (6.0 * x - 15.0) + 10.0)
}

impl<'a> MusicCtx<'a> {
    /// The context of one soundtrack build: the harmony of the fight file (its key and scale, the intro chords,
    /// then the chord loop to the last bar), the noise stream at its start, nothing cached. `cfg` and `voices`
    /// are the two fighters' roster entries and voices, left corner first.
    ///
    /// # Errors
    /// The fight file's key, scale or chords do not parse; the recording has no `HP` array of two fighters with
    /// at least one frame; or `voices[k]` is not the voice fighter `k`'s roster entry names (its `pp`): swapped
    /// voices would play each part on the other's instrument.
    pub fn new(
        show: &'a Show,
        rec: &'a Recording,
        ff: &'a FightFile,
        cfg: [&'a FighterCfg; 2],
        voices: [Arc<dyn Voice>; 2],
    ) -> Result<MusicCtx<'a>, MusicError> {
        const H: &str = "music";
        let m = &ff.music;
        let bad = |e: harmony::HarmonyError| MusicError::new(H, e.to_string());
        let chords = harmony::chords_for(&m.chords_intro, &m.chord_loop, show.grid.n_bars);
        let harmony = Harmony::new(
            &show.grid,
            &m.key,
            harmony::scale(&m.scale).map_err(bad)?,
            chords,
            Some(m.chord_loop.clone()),
            INTRO_BARS,
        )
        .map_err(bad)?;
        let hp = match rec.arrays.get("HP") {
            Some(Array::F64 { shape, data }) if shape.len() == 2 && shape[1] == 2 && !data.is_empty() => {
                if data.len() != shape[0] * 2 {
                    return Err(MusicError::new(
                        H,
                        "the recording's HP does not have the length it states",
                    ));
                }
                data.as_slice()
            }
            _ => {
                return Err(MusicError::new(
                    H,
                    "the recording has no HP of two fighters at every frame",
                ));
            }
        };
        for (k, (voice, cfg)) in voices.iter().zip(cfg).enumerate() {
            if voice.name() != cfg.pp {
                return Err(MusicError::new(
                    H,
                    format!("voice {k} is {:?}, but {} plays {:?}", voice.name(), cfg.name, cfg.pp),
                ));
            }
        }
        Ok(MusicCtx {
            show,
            rec,
            ff,
            harmony,
            cfg,
            voices,
            hp,
            state: Mutex::new(State::new(PpNoise::start())),
        })
    }

    /// The lock on the build's state. If a voice panicked while a note was being made, the noise stream and the
    /// cache may be half-advanced, and later notes would be silently wrong: the state is not handed out again.
    ///
    /// # Errors
    /// The lock is poisoned.
    fn lock(&self) -> Result<MutexGuard<'_, State>, MusicError> {
        self.state.lock().map_err(|_| {
            MusicError::new(
                "music",
                "a voice panicked while a note was being made: the noise stream and the notes of this build cannot \
                 be trusted any more",
            )
        })
    }

    /// Forgets every cached note, every gain and the log of first-time notes, and puts the noise stream at
    /// `noise`: a new build in the same context (line overrides stay).
    ///
    /// # Errors
    /// A voice panicked earlier (as every method that reads or changes the build's state).
    pub fn restart(&self, noise: PpNoise) -> Result<(), MusicError> {
        let mut st = self.lock()?;
        let line_overrides = std::mem::take(&mut st.line_overrides);
        *st = State::new(noise);
        st.line_overrides = line_overrides;
        Ok(())
    }

    /// A copy of the noise stream where it stands now.
    pub fn noise(&self) -> Result<PpNoise, MusicError> {
        Ok(self.lock()?.noise.clone())
    }

    /// Every first-time note so far, in the order `note` synthesised them.
    pub fn first_notes(&self) -> Result<Vec<NoteCall>, MusicError> {
        Ok(self.lock()?.first.clone())
    }

    /// `q16(t)`: the nearest 16th of the episode's grid, see [`q16`].
    pub fn q16(&self, t: f64) -> f64 {
        q16(t, self.show.grid.s16)
    }

    /// The number of recorded frames, `NSIM`.
    fn nsim(&self) -> usize {
        self.hp.len() / 2
    }

    /// `P.sim_i(t)`: the recorded frame at `t`, `round((t - T_FIGHT) * FPS)` kept inside the recording. A `t`
    /// that is not a number gives frame 0 (Python raises).
    pub fn sim_i(&self, t: f64) -> usize {
        let grid = &self.show.grid;
        let frame = round_half_even((t - grid.t_fight) * f64::from(grid.fps));
        if frame > 0.0 {
            (frame as usize).min(self.nsim() - 1)
        } else {
            0
        }
    }

    /// `hp_at(k, t)`: fighter `k`'s HP at `t`, 0 to 100: 100 before FIGHT!, the recorded HP up to the K.O.,
    /// then the HP it ended on (0 for the loser), refilled over the glide to the corners at the end.
    ///
    /// # Panics
    /// When `k` is not 0 or 1.
    pub fn hp_at(&self, k: usize, t: f64) -> f64 {
        assert!(k < 2, "hp_at: fighter {k} (there are two, 0 and 1)");
        let grid = &self.show.grid;
        if t < grid.t_fight {
            return 100.0;
        }
        if t < self.show.t_ko {
            return self.hp[self.sim_i(t) * 2 + k];
        }
        let end = if k == self.show.win {
            self.hp[(self.nsim() - 1) * 2 + k]
        } else {
            0.0
        };
        end + (100.0 - end) * ease((t - grid.t_glide) / (grid.t_rev - grid.t_glide))
    }

    /// The fighter whose voice has the roster name `name` (`CFG[k]['pp']`).
    fn voice_of(&self, helper: &'static str, name: &str) -> Result<usize, MusicError> {
        (0..2).find(|&k| self.cfg[k].pp == name).ok_or_else(|| {
            MusicError::new(
                helper,
                format!(
                    "{name:?} is not the voice of a fighter of this fight ({:?} and {:?} are)",
                    self.cfg[0].pp, self.cfg[1].pp
                ),
            )
        })
    }

    /// `inst_gain(name)`: the factor that brings the voice `name` to the series' level, measured once: the note
    /// in the middle of the voice's register (`(lo + hi) // 2`, velocity 0.8, 0.2 s), its first 0.3 s high-passed
    /// at 120 Hz, `min(4.0, 0.16 / (rms + 1e-6))`. Measuring it synthesises that note, which draws from the
    /// noise stream if the voice has noise: `note` therefore measures it AFTER the first note's own voice call,
    /// as Python does, and calling this first gives a different soundtrack.
    ///
    /// # Errors
    /// `name` is not the voice of a fighter of this fight, or the voice gives no samples, or samples that are
    /// not numbers, for that note.
    pub fn inst_gain(&self, name: &str) -> Result<f64, MusicError> {
        self.gain(&mut *self.lock()?, "inst_gain", name)
    }

    fn gain(&self, st: &mut State, helper: &'static str, name: &str) -> Result<f64, MusicError> {
        if let Some(&g) = st.gains.get(name) {
            return Ok(g);
        }
        let k = self.voice_of(helper, name)?;
        let (lo, hi) = self.voices[k].register();
        let middle = ifloor_div(i64::from(lo) + i64::from(hi), 2) as f64;
        let x = self.voices[k].note(&mut st.noise, middle, 0.8, 0.2);
        let x = &x[..x.len().min((0.3 * SRF) as usize)];
        if x.is_empty() || !x.iter().all(|v| v.is_finite()) {
            return Err(MusicError::new(
                helper,
                format!("{name} gives no sound for MIDI note {middle}, the middle of its register"),
            ));
        }
        let squares: Vec<f64> = synth::hp(x, 120.0, FILTER_O).iter().map(|v| v * v).collect();
        let g = py_min(4.0, 0.16 / (dsp::mean(&squares).sqrt() + 1e-6));
        st.gains.insert(name.to_string(), g);
        Ok(g)
    }

    /// `note(name, midi, dur, cap, vel)`: one note in the Pixel Pitch voice `name` (a fighter's `pp` roster
    /// name), level-matched across instruments: `fit(voice(name, int(midi), vel, dur), cap) * inst_gain(name)`,
    /// `int(cap * SR)` samples long.
    ///
    /// A note is synthesised once and cached by `(name, int(midi), round(dur, 3), cap, round(vel, 2))`; later
    /// calls with the same key get the same samples and draw no noise. So two calls whose `dur` or `vel` differ
    /// only past the rounding share the first one's note, as in the kit. The first note of a voice also measures
    /// the voice's gain, after its own voice call (see [`MusicCtx::inst_gain`]).
    ///
    /// A `dur` a little below zero is a note that is its release alone, and a `cap` below zero cuts that much
    /// off the end of the note instead of setting its length (Python's slice), both as in the kit.
    ///
    /// # Errors
    /// Checked before anything is synthesised: an argument that is not a number (Python raises for `midi`,
    /// `dur` and `cap`, and plays NaN for `vel`); a `dur` the voice cannot hold (so far below zero that the note
    /// has a negative number of samples: Python raises) or over [`MAX_SECONDS`]; a `cap` of 1 or 2 samples
    /// (Python raises: no room for the fade); a `name` that is not the voice of a fighter of this fight (Python
    /// has every instrument of the series). And after the voice call: samples that are not numbers, which is
    /// what a MIDI note far outside what the voice can make gives (Python plays the NaN); and a negative `cap`
    /// that leaves 1 or 2 samples of the note (Python raises there too, after the voice has drawn its noise).
    pub fn note(&self, name: &str, midi: f64, dur: f64, cap: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        const H: &str = "note";
        for (what, v) in [("midi", midi), ("dur", dur), ("cap", cap), ("vel", vel)] {
            if !v.is_finite() {
                return Err(MusicError::new(H, format!("{what} is {v}, not a number")));
            }
        }
        if dur > MAX_SECONDS {
            return Err(MusicError::new(
                H,
                format!("a held length (dur) of {dur} s is longer than {MAX_SECONDS} s"),
            ));
        }
        let midi = midi.trunc();
        let key = NoteKey {
            name: name.to_string(),
            midi: midi as i64,
            dur: key_bits(round_nd(dur, 3)),
            cap: key_bits(cap),
            vel: key_bits(round_nd(vel, 2)),
        };
        let mut st = self.lock()?;
        if let Some(x) = st.notes.get(&key) {
            return Ok(Arc::clone(x));
        }
        let k = self.voice_of(H, name)?;
        // What fit refuses for a cap from zero up depends on the cap alone (the note is then `int(cap * SR)`
        // samples long whatever the voice gives): find out before the voice draws any noise.
        let n = int(H, "cap", cap * SRF)?;
        if n == 1 || n == 2 {
            return Err(MusicError::new(H, no_room(n as usize, FIT_FADE)));
        }
        if !self.voices[k].can_hold(dur) {
            return Err(MusicError::new(
                H,
                format!("{name} has no note for a held length (dur) of {dur} s: it would end before it starts"),
            ));
        }
        let x = self.voices[k].note(&mut st.noise, midi, vel, dur);
        if !x.iter().all(|v| v.is_finite()) {
            return Err(MusicError::new(
                H,
                format!(
                    "{name} gives samples that are not numbers for MIDI note {midi} (velocity {vel}, {dur} s): \
                     the note is outside what the voice can make"
                ),
            ));
        }
        let fitted = fit(&x, cap, FIT_FADE).map_err(|e| MusicError::new(H, e.problem))?;
        let gain = self.gain(&mut st, H, name)?;
        let out: Arc<[f64]> = fitted.iter().map(|v| v * gain).collect();
        st.notes.insert(key, Arc::clone(&out));
        st.first.push(NoteCall {
            name: name.to_string(),
            midi: midi as i64,
            dur,
            cap,
            vel,
        });
        Ok(out)
    }

    /// `colour(x, k, t, drive_k, ...)`: fighter `k`'s tone at time `t`: thinner (high-passed) and dirtier
    /// (driven) as its HP drops; returns `(signal, gain, lead)`: the leader plays up to +3.5 dB, the loser down
    /// to -4 dB. A bass line (`args.low`) keeps its fundamental. See [`colour_hp`].
    ///
    /// # Errors
    /// As [`colour_hp`], and when `k` is not 0 or 1.
    pub fn colour(
        &self,
        x: &[f64],
        k: usize,
        t: f64,
        drive_k: f64,
        args: &ColourArgs,
    ) -> Result<(Vec<f64>, f64, f64), MusicError> {
        if k > 1 {
            return Err(MusicError::new(
                "colour",
                format!("fighter {k} (there are two, 0 and 1)"),
            ));
        }
        colour_hp(x, self.hp_at(k, t), self.hp_at(1 - k, t), drive_k, args)
    }

    /// `line_override(k, t0, t1, tz, notes)` (#53, opt-in): from `t0` to `t1`, fighter `k`'s line plays `notes`
    /// instead: `(beat from tz, midi note or a rest, beats)`. A fighter registers it in its `prepare`; whoever
    /// reads the fighter's line (`T.line_at`) looks the windows up with [`MusicCtx::line_overrides`].
    ///
    /// # Errors
    /// `k` is not 0 or 1, or a voice panicked earlier.
    pub fn line_override(
        &self,
        k: usize,
        t0: f64,
        t1: f64,
        tz: f64,
        notes: &[(f64, Option<i32>, f64)],
    ) -> Result<(), MusicError> {
        let Some(name) = self.show.names.get(k) else {
            return Err(MusicError::new(
                "line_override",
                format!("fighter {k} (there are two, 0 and 1)"),
            ));
        };
        self.lock()?
            .line_overrides
            .entry(name.clone())
            .or_default()
            .push(LineOverride {
                t0,
                t1,
                tz,
                notes: notes.to_vec(),
            });
        Ok(())
    }

    /// `T.LINE_OVERRIDE.get(name, ())`: the windows registered for the fighter `name`, in the order they were
    /// registered.
    ///
    /// # Errors
    /// A voice panicked earlier.
    pub fn line_overrides(&self, name: &str) -> Result<Vec<LineOverride>, MusicError> {
        Ok(self.lock()?.line_overrides.get(name).cloned().unwrap_or_default())
    }
}

// ---------------------------------------------------------------- the ring's own sounds

/// Element `i` of an event (Python's `e[i]`, `i >= 2`) as a number.
fn number(e: &Event, i: usize) -> Result<f64, MusicError> {
    match e.fields.get(i - 2) {
        Some(Value::Float(x)) => Ok(*x),
        Some(Value::Int(n)) => Ok(*n as f64),
        other => Err(MusicError::new(
            "shared_events",
            format!(
                "the {} event at {} s has no number as element {i} (it has {other:?})",
                e.kind, e.t
            ),
        )),
    }
}

/// Python's `int(v)` as a noise seed: numpy refuses a seed below zero.
fn seed(v: f64) -> Result<u64, MusicError> {
    if v.is_finite() && v > -1.0 {
        Ok(v as u64)
    } else {
        Err(MusicError::new("shared_events", format!("{v} is not a noise seed")))
    }
}

/// `shared_events(mix)`: the ring's own sounds from the run, up to the K.O.: a soft thud for every rope bounce
/// (louder the faster the ball hit, panned to where), the bed's own sound for a clash of the two balls, a
/// metallic clank for a blocked hit (on the 16th) and for a parry.
///
/// # Errors
/// An event of one of those kinds lacks a number the kit reads, or the bed's `clash` fails.
pub fn shared_events(ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Result<(), MusicError> {
    for e in &ctx.rec.events {
        let t = e.t;
        if t > ctx.show.t_ko + 0.01 {
            break;
        }
        match e.kind.as_str() {
            "rope" => {
                let thud = synth::thud(0.22, 95.0, 55.0, seed(t * 10.0)?);
                let gain = 0.05 * py_min(1.5, number(e, 7)? / 2.6);
                let pan = clip(number(e, 5)? / 3.5, -0.6, 0.6);
                mix.place("fx", &Sig::Mono(thud), t, gain, pan);
            }
            "clash" => bed.clash(ctx, mix, t)?,
            "block" => {
                let ring = clank(84.0, seed(t * 5.0)?, CLANK_DUR, &CLANK_DECAYS)?;
                mix.place("fx", &Sig::Mono(ring), ctx.q16(t), 0.06, 0.0);
            }
            "parry" => {
                let ring = clank(88.0, seed(t * 9.0)?, CLANK_DUR, &CLANK_DECAYS)?;
                mix.place("fx", &Sig::Mono(ring), t, 0.04, 0.0);
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod testkit {
    //! A recording, a fight file, a small voice and a bed that plays nothing, for the unit tests of this crate.

    use std::path::PathBuf;
    use std::sync::Mutex;

    use bf_sim::roster::parse_roster;

    use super::*;
    use crate::traits::Played;

    fn studio() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// The example episode: its recording (seed 392), fight file, show and the two fighters' roster entries.
    pub(crate) struct Episode {
        pub(crate) rec: Recording,
        pub(crate) ff: FightFile,
        pub(crate) show: Show,
        pub(crate) cfg: Vec<FighterCfg>,
    }

    pub(crate) fn episode() -> Episode {
        let rec = Recording::read(&studio().join("reference/fights/flute_vs_trombone/ep392.bfrec"))
            .expect("the recording reads");
        let read = |rel: &str| std::fs::read_to_string(studio().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let ff = FightFile::parse(&read("fights/flute_vs_trombone.toml")).expect("the fight file parses");
        let show = Show::new(&rec, &ff).expect("the episode has a K.O.");
        let roster = parse_roster(&read("crates/bf-content/data/roster.json")).expect("the roster reads");
        let cfg = show
            .names
            .iter()
            .map(|n| roster.iter().find(|f| &f.name == n).expect("in the roster").clone())
            .collect();
        Episode { rec, ff, show, cfg }
    }

    /// A voice for tests, under the roster name `name`: a sine on the note's pitch for `dur + 0.1` s (so it can
    /// hold a `dur` down to -0.1 s), plus `draws` normals from the noise stream at a hundredth of the level. It
    /// remembers every call.
    pub(crate) struct TestVoice {
        pub(crate) name: &'static str,
        pub(crate) draws: usize,
        pub(crate) calls: Mutex<Vec<(f64, f64, f64)>>,
    }

    impl TestVoice {
        pub(crate) fn new(name: &'static str, draws: usize) -> Arc<TestVoice> {
            Arc::new(TestVoice {
                name,
                draws,
                calls: Mutex::new(Vec::new()),
            })
        }

        pub(crate) fn calls(&self) -> Vec<(f64, f64, f64)> {
            self.calls.lock().expect("the call list").clone()
        }
    }

    impl Voice for TestVoice {
        fn note(&self, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
            self.calls.lock().expect("the call list").push((midi, vel, dur));
            let n = ((dur + 0.1) * SRF) as usize;
            let breath = noise.nz(self.draws);
            let f = synth::mtof(midi);
            (0..n)
                .map(|i| {
                    let hiss = breath.get(i).map_or(0.0, |b| 0.01 * b);
                    vel * (dmath::sin(2.0 * PI * f * (i as f64 / SRF)) + hiss)
                })
                .collect()
        }

        fn register(&self) -> (i32, i32) {
            (60, 73)
        }

        fn name(&self) -> &str {
            self.name
        }

        fn can_hold(&self, dur: f64) -> bool {
            dur >= -0.1
        }
    }

    /// A bed for tests: two buses, a room, a compressor, and sections that play nothing. It remembers the time
    /// of every clash it is asked for, and marks each with one sample of 1.0 on the bus `clash`.
    pub(crate) struct TestBed {
        pub(crate) reverb: (f64, f64, f64),
        pub(crate) clashes: Mutex<Vec<f64>>,
    }

    impl TestBed {
        pub(crate) fn new(reverb: (f64, f64, f64)) -> TestBed {
            TestBed {
                reverb,
                clashes: Mutex::new(Vec::new()),
            }
        }
    }

    impl Bed for TestBed {
        fn buses(&self) -> &[(&'static str, (f64, f64))] {
            &[("drums", (1.0, 0.15)), ("fx", (0.5, 0.1))]
        }
        fn reverb(&self) -> (f64, f64, f64) {
            self.reverb
        }
        fn comp(&self) -> (f64, f64) {
            (0.15, 0.62)
        }
        fn roar_gain(&self) -> f64 {
            0.15
        }
        fn doors_gain(&self) -> f64 {
            0.4
        }
        fn landing(&self) -> (f64, f64) {
            (0.07, 0.05)
        }
        fn pop(&self) -> (f64, f64, f64) {
            (95.0, 100.0, 0.1)
        }
        fn boom(&self, _: &MusicCtx, _: &mut Mix, _: f64, _: f64) -> Played {
            Ok(())
        }
        fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
            if t < 0.0 {
                return Err(MusicError::new("clash", format!("a clash at {t} s")));
            }
            self.clashes.lock().expect("the clash list").push(t);
            mix.place("clash", &Sig::Mono(vec![1.0]), t, 1.0, 0.0);
            Ok(())
        }
        fn title(&self, _: &MusicCtx, _: &mut Mix, _: [f64; 3]) -> Played {
            Ok(())
        }
        fn vs(&self, _: &MusicCtx, _: &mut Mix, _: f64) -> Played {
            Ok(())
        }
        fn title_rest(&self, _: &MusicCtx, _: &mut Mix) -> Played {
            Ok(())
        }
        fn count_in(&self, _: &MusicCtx, _: &mut Mix) -> Played {
            Ok(())
        }
        fn fight_start(&self, _: &MusicCtx, _: &mut Mix) -> Played {
            Ok(())
        }
        fn fight(&self, _: &MusicCtx, _: &mut Mix, _: usize) -> Played {
            Ok(())
        }
        fn ko(&self, _: &MusicCtx, _: &mut Mix) -> Played {
            Ok(())
        }
        fn stinger(&self, _: &MusicCtx, _: &mut Mix, _: f64) -> Played {
            Ok(())
        }
        fn after_ko(&self, _: &MusicCtx, _: &mut Mix, _: usize, _: f64, _: f64) -> Played {
            Ok(())
        }
        fn victory(&self, _: &MusicCtx, _: &mut Mix, _: usize) -> Played {
            Ok(())
        }
        fn last_bars(&self, _: &MusicCtx, _: &mut Mix) -> Played {
            Ok(())
        }
    }

    /// The episode's context with two test voices: the FLUTE's draws 100 normals a note, the TROMBONE's none.
    pub(crate) fn context(ep: &Episode) -> (MusicCtx<'_>, [Arc<TestVoice>; 2]) {
        let voices = [TestVoice::new("FLUTE", 100), TestVoice::new("TROMBONE", 0)];
        let ctx = MusicCtx::new(
            &ep.show,
            &ep.rec,
            &ep.ff,
            [&ep.cfg[0], &ep.cfg[1]],
            [voices[0].clone(), voices[1].clone()],
        )
        .expect("the context builds");
        (ctx, voices)
    }
}

#[cfg(test)]
mod tests {
    use super::testkit::{TestBed, TestVoice, context, episode};
    use super::*;

    /// A float as Python's `float.hex()` prints it.
    fn hex(text: &str) -> f64 {
        bf_testdata::parse_hex_f64(text)
    }

    /// The message of an error.
    fn refused<T: fmt::Debug>(r: Result<T, MusicError>) -> String {
        r.expect_err("it was not refused").to_string()
    }

    #[test]
    fn hashes_are_pythons_on_odd_arguments() {
        // Printed by the kit on the reference PC: h01(-1), h01(1e30), h01(-2.5, 7), h01(0.0005), h01(0.0015),
        // h01(-1e30, 2). A negative int wraps as Python's `&` wraps it; a float past 2**64 keeps its low bits;
        // 0.5 and 1.5 thousandths round to the even one (0 and 2).
        assert_eq!(h01(&[-1.0]), hex("0x1.6ca1cc5aa2763p-1"));
        assert_eq!(h01(&[1e30]), hex("0x1.ecfe83b5e5a48p-4"));
        assert_eq!(h01(&[-2.5, 7.0]), hex("0x1.b2ed4e3c999d8p-1"));
        assert_eq!(h01(&[0.0005]), h01(&[0.0]));
        assert_eq!(h01(&[0.0005]), hex("0x1.b9e279aa86e58p-2"));
        assert_eq!(h01(&[0.0015]), hex("0x1.7f908c2017f83p-1"));
        assert_eq!(h01(&[-1e30, 2.0]), hex("0x1.b6035718dd38ap-2"));
        // h01_37(-7, 3), h01_37(0.0005), h01_37(1e30), h01_37(3, -1.5).
        assert_eq!(h01_37(&[-7.0, 3.0]), hex("0x1.62bc478bc8489p-1"));
        assert_eq!(h01_37(&[0.0005]), hex("0x1.918a9f3158192p-1"));
        assert_eq!(h01_37(&[1e30]), hex("0x1.1c5b67e767eacp-5"));
        assert_eq!(h01_37(&[3.0, -1.5]), hex("0x1.e66a76ae8c971p-1"));
        // No argument: the seed constant itself.
        assert_eq!(
            h01(&[]),
            (0x9E37_79B9_7F4A_7C15u64 >> 11) as f64 / 9_007_199_254_740_992.0
        );
        // A float beyond 2**117 has no low bits left: it hashes as 0 does.
        assert_eq!(h01(&[1e40]), h01(&[0.0]));
        assert_eq!(h01(&[-1e300]), h01(&[0.0]));
    }

    #[test]
    #[should_panic(expected = "cannot convert float NaN to integer")]
    fn a_hash_argument_must_be_a_number() {
        h01(&[1.0, f64::NAN]);
    }

    #[test]
    #[should_panic(expected = "cannot convert float inf to integer")]
    fn a_hash_argument_must_be_finite() {
        h01_37(&[f64::INFINITY]);
    }

    #[test]
    fn thinning_follows_the_hp() {
        // At full HP nothing is dropped; at none, some notes are, never two in a row.
        let mut th = Thinner::new(5.0);
        assert!((0..400).all(|s| th.keep(f64::from(s), 1.0)));
        let mut th = Thinner::new(5.0);
        let kept: Vec<bool> = (0..400).map(|s| th.keep(f64::from(s), 0.0)).collect();
        let dropped = kept.iter().filter(|k| !**k).count();
        assert!((60..=140).contains(&dropped), "{dropped} of 400 dropped at 0 HP");
        assert!(!kept.windows(2).any(|w| w == [false, false]));
        // Printed by the kit: Thinner(5).keep(3, 0.5) is False; with a NaN HP the chance is the full 0.30
        // (Python's min keeps its first argument), slots 0..12.
        assert!(!Thinner::new(5.0).keep(3.0, 0.5));
        let mut th = Thinner::new(5.0);
        let nan: Vec<bool> = (0..12).map(|s| th.keep(f64::from(s), f64::NAN)).collect();
        assert_eq!(
            nan,
            [
                true, true, false, true, false, true, false, true, true, false, true, false
            ]
        );
        // thin_37: nothing at full HP, nothing right after a drop, nothing for a NaN HP.
        assert!((0..400).all(|s| !thin_37(f64::from(s), 3.0, 1.0, false)));
        assert!((0..400).all(|s| !thin_37(f64::from(s), 3.0, 0.0, true)));
        assert!((0..400).any(|s| thin_37(f64::from(s), 3.0, 0.0, false)));
        assert!(!thin_37(0.0, 3.0, f64::NAN, false));
        // Above full HP Python raises only if it gets as far as the comparison.
        assert!(!thin_37(0.0, 3.0, 1.5, true));
    }

    #[test]
    #[should_panic(expected = "Thinner.keep: hp01 is 1.5, above 1")]
    fn a_thinner_refuses_more_than_full_hp() {
        Thinner::new(5.0).keep(0.0, 1.5);
    }

    #[test]
    #[should_panic(expected = "thin_37: hp is 1.5, above 1")]
    fn thin_37_refuses_more_than_full_hp() {
        thin_37(0.0, 3.0, 1.5, false);
    }

    #[test]
    fn q16_snaps_to_the_even_sixteenth() {
        let s16 = hex("0x1.cb4b4b4b4b4b4p-4");
        // Printed by the kit: q16(-0.03), q16(0.5 * S16), q16(1.5 * S16), q16(2.5 * S16).
        assert_eq!(q16(-0.03, s16).to_bits(), 0.0f64.to_bits());
        assert_eq!(q16(0.5 * s16, s16).to_bits(), 0.0f64.to_bits());
        assert_eq!(q16(1.5 * s16, s16), 2.0 * s16);
        assert_eq!(q16(2.5 * s16, s16), 2.0 * s16);
        assert_eq!(q16(1.0, 0.25), 1.0);
        assert!(q16(f64::NAN, s16).is_nan());
    }

    #[test]
    fn fit_cuts_pads_and_fades() {
        let ones = vec![1.0; 5000];
        // Cut to 2,400 samples; the last 800 (a third) fade to nothing.
        let y = fit(&ones, 0.05, FIT_FADE).unwrap();
        assert_eq!(y.len(), 2400);
        assert_eq!((y[1599], y[1600], y[2399]), (1.0, 1.0, 0.0));
        assert_eq!(y[1601], dsp::linspace(1.0, 0.0, 800)[1]);
        // Padded with silence: a short note keeps its samples up to the fade.
        let y = fit(&[1.0; 10], 0.05, FIT_FADE).unwrap();
        assert_eq!(y.len(), 2400);
        assert_eq!((y[9], y[10], y[2399]), (1.0, 0.0, 0.0));
        // A short fade: 240 samples.
        let y = fit(&ones, 0.05, 0.005).unwrap();
        assert_eq!((y[2159], y[2160], y[2399]), (1.0, 1.0, 0.0));
        assert!(y[2161] < 1.0);
        // What Python gives at the edges (run on the reference PC): no samples at all for a cap of 0; three
        // samples untouched (a fade of one sample is a factor of 1); a negative cap cuts the end off.
        assert!(fit(&ones, 0.0, FIT_FADE).unwrap().is_empty());
        assert_eq!(fit(&ones, 3.0 / 48000.0, FIT_FADE).unwrap(), [1.0, 1.0, 1.0]);
        let y = fit(&ones, -0.01, FIT_FADE).unwrap();
        assert_eq!(y.len(), 4520);
        assert!((y[4518] - 0.00066445).abs() < 1e-8 && y[4519] == 0.0);
        assert!(fit(&ones, -0.2, FIT_FADE).unwrap().is_empty());
        assert_eq!(fit(&ones, -4997.0 / 48000.0, FIT_FADE).unwrap(), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn fit_refuses_what_python_refuses() {
        let ones = vec![1.0; 5000];
        let no_room = "leave no room for a fade";
        assert!(refused(fit(&ones, 1.0 / 48000.0, FIT_FADE)).contains(no_room));
        assert!(refused(fit(&ones, 2.5 / 48000.0, FIT_FADE)).contains(no_room));
        assert!(refused(fit(&ones, -4999.0 / 48000.0, FIT_FADE)).contains(no_room));
        assert!(refused(fit(&ones, 0.05, 0.0)).contains(no_room));
        assert!(refused(fit(&ones, 0.05, 1e-6)).contains(no_room));
        assert_eq!(refused(fit(&ones, 0.05, -0.01)), "fit: a fade of -0.01 s is below zero");
        assert_eq!(refused(fit(&ones, f64::NAN, FIT_FADE)), "fit: cap is NaN, not a number");
        assert_eq!(
            refused(fit(&ones, 0.05, f64::INFINITY)),
            "fit: fade is inf, not a number"
        );
        assert_eq!(
            refused(fit(&ones, 1e9, FIT_FADE)),
            "fit: cap of 1000000000 s is longer than 600 s"
        );
    }

    #[test]
    fn a_warp_of_nothing_is_the_note() {
        let x: Vec<f64> = (0..2000).map(|i| dmath::sin(f64::from(i) * 0.05)).collect();
        // No slide, no bend, no vibrato: the read position is every sample in turn.
        assert_eq!(gliss(&x, 0.0, 0.05).unwrap(), x);
        assert_eq!(bend(&x, 0.0, 0.0, 0.2).unwrap(), x);
        assert_eq!(vibrato(&x, 0.0, VIBRATO_RATE, VIBRATO_DELAY).unwrap(), x);
        // Sliding up into the note reads slower at first: the end of the note is not reached, and nothing past
        // it is read. Falling away (a negative slide) reads faster: the note runs out and silence follows.
        let up = gliss(&x, 12.0, 0.02).unwrap();
        assert_eq!((up.len(), up[0]), (2000, x[0]));
        assert!(up[1999] != 0.0 && up[1999] != x[1999]);
        let down = gliss(&x, -12.0, 0.02).unwrap();
        assert_eq!(down[0], x[0]);
        assert!(down[1900..].iter().all(|v| *v == 0.0) && down[1000] != 0.0);
        // A bend starts at t0: before it the note is untouched.
        let bent = bend(&x, 2.0, 0.01, 0.03).unwrap();
        assert_eq!(bent[..480], x[..480]);
        assert!(bent[481..].iter().zip(&x[481..]).any(|(a, b)| a != b));
        // The vibrato comes in after its delay.
        let wobbly = vibrato(&x, 0.5, 5.0, 0.01).unwrap();
        assert_eq!(wobbly[..481], x[..481]);
        assert!(wobbly[481..] != x[481..]);
        for r in [
            gliss(&[], 2.0, 0.035),
            bend(&[], 2.0, 0.0, 0.2),
            vibrato(&[], 0.35, 5.5, 0.15),
        ] {
            assert!(refused(r).ends_with(": the note has no samples"));
        }
    }

    #[test]
    fn wah_needs_a_sample_per_band() {
        let x: Vec<f64> = (0..4800).map(|i| dmath::sin(f64::from(i) * 0.1)).collect();
        let y = wah(&x, &[(0.0, 0.05)]).unwrap();
        assert_eq!(y.len(), 4800);
        assert!(y.iter().all(|v| v.is_finite()) && y.iter().any(|v| v.abs() > 0.1));
        // No span at all: the band stays at 450 Hz.
        assert_eq!(wah(&x, &[]).unwrap().len(), 4800);
        assert_eq!(wah(&x[..48], &[]).unwrap().len(), 48);
        assert_eq!(
            refused(wah(&x[..47], &[])),
            "wah: 47 sample(s) are too few for 48 bands"
        );
        assert_eq!(refused(wah(&[], &[])), "wah: 0 sample(s) are too few for 48 bands");
        // A span from a time that is not a number never opens (no comparison with it holds), as in Python; one
        // from minus infinity makes the centre NaN, which no filter takes.
        assert_eq!(wah(&x, &[(f64::NAN, 0.1)]).unwrap(), wah(&x, &[]).unwrap());
        assert!(refused(wah(&x, &[(f64::NEG_INFINITY, 0.1)])).starts_with("wah: a band from NaN to NaN Hz"));
    }

    #[test]
    fn colour_thins_and_drives_with_the_hp() {
        let x: Vec<f64> = (0..4800).map(|i| 0.5 * dmath::sin(f64::from(i) * 0.05)).collect();
        let args = ColourArgs::default();
        // At full HP against full HP: a 90 Hz high-pass, no drive (tanh(1.2 y) / tanh(1.2)), no gain.
        let (y, g, lead) = colour_hp(&x, 100.0, 100.0, COLOUR_DRIVE_K, &args).unwrap();
        assert_eq!((g, lead), (1.0, 0.0));
        let plain = synth::hp(&x, 90.0, FILTER_O);
        let want: Vec<f64> = plain
            .iter()
            .map(|&v| dmath::tanh(1.0 * v * 1.2) / dmath::tanh(1.2) * 1.0)
            .collect();
        assert_eq!(y, want);
        // The gain follows the lead: 6 dB for a full bar, kept between -4 and +3.5 dB.
        let gain = |own: f64, other: f64| colour_hp(&x, own, other, 1.0, &args).unwrap().1;
        assert_eq!(gain(100.0, 0.0), dmath::pow(10.0, 3.5 / 20.0));
        assert_eq!(gain(0.0, 100.0), dmath::pow(10.0, -4.0 / 20.0));
        assert_eq!(gain(60.0, 50.0), dmath::pow(10.0, 6.0 * ((60.0 - 50.0) / 100.0) / 20.0));
        // A bass line keeps its fundamental: its cutoff starts at 30 Hz, not 90.
        let low = ColourArgs { low: true, ..args };
        assert_eq!(
            colour_hp(&x, 100.0, 100.0, 1.0, &low).unwrap().0,
            synth::hp(&x, 30.0, FILTER_O)
                .iter()
                .map(|&v| dmath::tanh(v * 1.2) / dmath::tanh(1.2))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            refused(colour_hp(&[], 50.0, 50.0, 1.0, &args)),
            "colour: the note has no samples"
        );
        assert_eq!(
            refused(colour_hp(&x, 100.5, 50.0, 1.0, &args)),
            "colour: an HP of 100.5 is above 100"
        );
        let wild = ColourArgs { cut0: 30000.0, ..args };
        assert_eq!(
            refused(colour_hp(&x, 100.0, 50.0, 1.0, &wild)),
            "colour: a cutoff of 30000 Hz is not between 0 and 24000 Hz"
        );
        let none = ColourArgs { cut0: 0.0, ..args };
        assert!(refused(colour_hp(&x, 100.0, 50.0, 1.0, &none)).contains("a cutoff of 0 Hz"));
    }

    #[test]
    fn effects_have_pythons_lengths() {
        assert_eq!(clank(84.0, CLANK_SEED, CLANK_DUR, &CLANK_DECAYS).unwrap().len(), 19200);
        assert_eq!(
            hiss(HISS_DUR, HISS_SEED, HISS_LO, HISS_HI, HISS_DEC).unwrap().len(),
            4320
        );
        assert_eq!(roar(0.5, ROAR_SEED).unwrap().len(), 24000);
        assert_eq!(thunder(0.25, THUNDER_SEED, THUNDER_CRACK).unwrap().len(), 12000);
        assert_eq!(riser(0.5, RISER_OVER, RISER_SEED).unwrap().len(), 24576);
        // No partials: Python's sum is the int 0, and the click alone is left.
        let click = clank(84.0, 3, 0.1, &[]).unwrap();
        let noise = synth::bp(&noise(3, 4800), 2000.0, 8000.0, FILTER_O);
        assert_eq!(click[100], 0.3 * noise[100] * dmath::exp(-(100.0 / 48000.0) / 0.005));
        // The crack is the generator's second draw: without it the rumble is the same.
        let with = thunder(0.25, 4, true).unwrap();
        let without = thunder(0.25, 4, false).unwrap();
        assert!(with != without);
        let mut r = Generator::new(4);
        let mut first = vec![0.0; 12000];
        r.fill_normal(0.0, 1.0, &mut first);
        let low = synth::lp(&first, 140.0, FILTER_O);
        let t = 6000.0 / 48000.0;
        let rumble = low[6000] * 3.0 * 1.0 * dmath::exp(-t / (0.25 * 0.35));
        assert_eq!(
            without[6000],
            rumble * (0.7 + 0.3 * dmath::sin(2.0 * PI * 3.1 * t + 4.0))
        );
    }

    #[test]
    fn effects_refuse_what_python_refuses() {
        let nothing = "is under one sample: there is nothing to filter";
        assert!(refused(clank(84.0, 0, 0.0, &CLANK_DECAYS)).contains(nothing));
        assert!(refused(hiss(1e-6, 0, HISS_LO, HISS_HI, HISS_DEC)).contains(nothing));
        assert!(refused(roar(0.0, ROAR_SEED)).contains(nothing));
        assert!(refused(thunder(0.0, 0, true)).contains(nothing));
        assert_eq!(
            refused(riser(19.5 / 48000.0, 0.0, RISER_SEED)),
            "riser: 19 sample(s) are too few for 20 bands"
        );
        assert_eq!(riser(20.5 / 48000.0, 0.0, RISER_SEED).unwrap().len(), 20);
        assert_eq!(refused(roar(-1.0, ROAR_SEED)), "roar: a length of -1 s is below zero");
        assert_eq!(refused(roar(f64::NAN, 3)), "roar: a length is NaN, not a number");
        assert_eq!(
            refused(thunder(601.0, 0, true)),
            "thunder: a length of 601 s is longer than 600 s"
        );
        assert_eq!(
            refused(hiss(0.1, 0, 9000.0, 3000.0, 0.03)),
            "hiss: a band from 9000 to 3000 Hz does not rise between 0 and 24000 Hz"
        );
        assert!(refused(hiss(0.1, 0, 0.0, 3000.0, 0.03)).starts_with("hiss: a band from 0 to 3000 Hz"));
        assert!(refused(hiss(0.1, 0, 3000.0, 24000.0, 0.03)).starts_with("hiss: a band from 3000 to 24000 Hz"));
        assert!(refused(hiss(0.1, 0, f64::NAN, 3000.0, 0.03)).starts_with("hiss: a band from NaN"));
        // A riser that rises over no time divides 0 by 0 at its first sample. With under 40 samples that sample
        // is the first band's centre, and no filter takes it; with more, every sample comes out NaN (Python's
        // too: the saw's phase is a running sum from that first NaN).
        assert!(refused(riser(0.0, 25.5 / 48000.0, RISER_SEED)).starts_with("riser: a band from NaN"));
        assert!(riser(0.0, 0.01, RISER_SEED).unwrap().iter().all(|v| v.is_nan()));
    }

    #[test]
    fn hp_follows_the_fight() {
        let ep = episode();
        let (ctx, _) = context(&ep);
        let grid = &ep.show.grid;
        assert_eq!((ep.show.win, ctx.nsim()), (1, 1394));
        // Printed by the kit on the reference PC (`M.hp_at(k, t)` as float.hex(), k = 0 and 1).
        let full = "0x1.9000000000000p+6";
        let python: [(f64, [&str; 2]); 11] = [
            (0.0, [full, full]),
            (grid.t_fight, [full, full]),
            (3.6, [full, full]),
            (12.0, ["0x1.6e66666666664p+6", "0x1.53851eb851ebap+6"]),
            (ep.show.t_ko - 0.001, ["0x0.0p+0", "0x1.051eb851eb8c8p+2"]),
            (ep.show.t_ko, ["0x0.0p+0", "0x1.051eb851eb8c8p+2"]),
            (ep.show.t_ko + 0.5, ["0x0.0p+0", "0x1.051eb851eb8c8p+2"]),
            (grid.t_glide + 0.3, ["0x1.cf0b8f96f633fp+3", "0x1.1f5b40df3c895p+4"]),
            (grid.t_glide + 0.5, ["0x1.6c56f471782b8p+5", "0x1.7e1d5703929a8p+5"]),
            (grid.t_rev, [full, full]),
            (60.9, [full, full]),
        ];
        for (t, want) in python {
            for (k, want) in want.iter().enumerate() {
                assert_eq!(ctx.hp_at(k, t).to_bits(), hex(want).to_bits(), "hp_at({k}, {t})");
            }
        }
        // sim_i: frame 0 up to FIGHT!, the last frame from the K.O. on.
        assert_eq!((ctx.sim_i(0.0), ctx.sim_i(grid.t_fight), ctx.sim_i(12.0)), (0, 0, 252));
        assert_eq!((ctx.sim_i(ep.show.t_ko), ctx.sim_i(1e9)), (1393, 1393));
        assert_eq!((ctx.sim_i(f64::NAN), ctx.sim_i(-1e9)), (0, 0));
        // A time that is not a number is past the K.O. for Python's comparisons: the HP it ended on.
        assert_eq!(ctx.hp_at(0, f64::NAN), 0.0);
        assert_eq!(ctx.q16(1.0), q16(1.0, grid.s16));
    }

    #[test]
    #[should_panic(expected = "hp_at: fighter 2")]
    fn hp_is_of_one_of_the_two() {
        let ep = episode();
        context(&ep).0.hp_at(2, 12.0);
    }

    #[test]
    fn the_context_reads_the_fight_file() {
        let ep = episode();
        let (ctx, _) = context(&ep);
        // The harmony the kit builds for the episode: E harmonic minor, ['Em', 'B7'] + LOOP * 4.
        assert_eq!((ctx.harmony.key_pc, ctx.harmony.loop_start), (4, 2));
        assert_eq!(ctx.harmony.chords.len(), 34);
        assert_eq!(ctx.harmony.chords[..4], ["Em", "B7", "Em", "Am"]);
        assert_eq!(ctx.harmony.loop_, ep.ff.music.chord_loop);
        assert!(format!("{ctx:?}").contains("FLUTE"));
        // A scale the kit does not know, and a recording without HP.
        let mut ff = ep.ff.clone();
        ff.music.scale = "wholetone".into();
        let voices = || -> [Arc<dyn Voice>; 2] { [TestVoice::new("FLUTE", 0), TestVoice::new("TROMBONE", 0)] };
        let cfg = [&ep.cfg[0], &ep.cfg[1]];
        let e = refused(MusicCtx::new(&ep.show, &ep.rec, &ff, cfg, voices()));
        assert!(e.starts_with("music: ") && e.contains("wholetone"), "{e}");
        let mut rec = ep.rec.clone();
        rec.arrays.remove("HP");
        let no_hp = "music: the recording has no HP of two fighters at every frame";
        assert_eq!(refused(MusicCtx::new(&ep.show, &rec, &ep.ff, cfg, voices())), no_hp);
        // An HP of another shape, of no frames, or shorter than it says.
        let mut with = |shape: Vec<usize>, len: usize| {
            let data = vec![50.0; len];
            rec.arrays.insert("HP".into(), Array::F64 { shape, data });
            refused(MusicCtx::new(&ep.show, &rec, &ep.ff, cfg, voices()))
        };
        assert_eq!(with(vec![4, 3], 12), no_hp);
        assert_eq!(with(vec![8], 8), no_hp);
        assert_eq!(with(vec![0, 2], 0), no_hp);
        assert_eq!(
            with(vec![5, 2], 7),
            "music: the recording's HP does not have the length it states"
        );
    }

    #[test]
    fn a_note_is_made_once_and_its_gain_after_it() {
        let ep = episode();
        let (ctx, voices) = context(&ep);
        let start = ctx.noise().unwrap();
        let x = ctx.note("FLUTE", 76.9, 0.2, 0.3, 0.8).unwrap();
        // int(midi), then the level-matching note: the middle of the register, velocity 0.8, 0.2 s.
        assert_eq!(voices[0].calls(), [(76.0, 0.8, 0.2), (66.0, 0.8, 0.2)]);
        assert!(voices[1].calls().is_empty());
        assert_eq!(x.len(), 14400);
        // fit(voice, cap) * gain, with the noise the note drew FIRST and the gain note's after it.
        let mut noise = start.clone();
        let own = voices[0].note(&mut noise, 76.0, 0.8, 0.2);
        let level = voices[0].note(&mut noise, 66.0, 0.8, 0.2);
        let squares: Vec<f64> = synth::hp(&level[..14400], 120.0, FILTER_O)
            .iter()
            .map(|v| v * v)
            .collect();
        let gain = 0.16 / (dsp::mean(&squares).sqrt() + 1e-6);
        assert_eq!(ctx.inst_gain("FLUTE").unwrap(), gain);
        let want: Vec<f64> = fit(&own, 0.3, FIT_FADE).unwrap().iter().map(|v| v * gain).collect();
        assert_eq!(x.to_vec(), want);
        assert_eq!(ctx.noise().unwrap().0.clone().random(), noise.0.random());

        // The same key again: the same samples, no voice call, no noise.
        let calls = voices[0].calls().len();
        let before = ctx.noise().unwrap().0.random();
        for (midi, dur, vel) in [(76.9, 0.2, 0.8), (76.0, 0.2004, 0.8), (76.2, 0.1996, 0.804)] {
            let again = ctx.note("FLUTE", midi, dur, 0.3, vel).unwrap();
            assert!(Arc::ptr_eq(&again, &x), "midi {midi}, dur {dur}, vel {vel}");
        }
        assert_eq!(voices[0].calls().len(), calls);
        assert_eq!(ctx.noise().unwrap().0.random(), before);
        // Another key (the cap, the rounded dur, the rounded vel, the note): another note, with the arguments
        // as they were passed, and no second gain note.
        ctx.note("FLUTE", 76.0, 0.2, 0.31, 0.8).unwrap();
        ctx.note("FLUTE", 76.0, 0.2006, 0.3, 0.8).unwrap();
        ctx.note("FLUTE", 76.0, 0.2, 0.3, 0.806).unwrap();
        ctx.note("FLUTE", 77.0, 0.2, 0.3, 0.8).unwrap();
        assert_eq!(
            voices[0].calls()[calls..],
            [
                (76.0, 0.8, 0.2),
                (76.0, 0.8, 0.2006),
                (76.0, 0.806, 0.2),
                (77.0, 0.8, 0.2)
            ]
        );
        // The other voice has its own gain, measured on its own first note.
        ctx.note("TROMBONE", 52.0, 0.2, 0.3, 0.8).unwrap();
        assert_eq!(voices[1].calls(), [(52.0, 0.8, 0.2), (66.0, 0.8, 0.2)]);
        let log = ctx.first_notes().unwrap();
        assert_eq!(log.len(), 6);
        assert_eq!(
            log[0],
            NoteCall {
                name: "FLUTE".into(),
                midi: 76,
                dur: 0.2,
                cap: 0.3,
                vel: 0.8
            }
        );
        assert_eq!(
            (log[2].dur, log[3].vel, log[5].name.as_str()),
            (0.2006, 0.806, "TROMBONE")
        );

        // A new build: nothing cached, the stream where it is put.
        ctx.restart(start.clone()).unwrap();
        assert!(ctx.first_notes().unwrap().is_empty());
        let again = ctx.note("FLUTE", 76.9, 0.2, 0.3, 0.8).unwrap();
        assert!(!Arc::ptr_eq(&again, &x));
        assert_eq!(again, x);
        // Measuring the gain before the first note puts other noise under the note.
        ctx.restart(start).unwrap();
        ctx.inst_gain("FLUTE").unwrap();
        assert!(ctx.note("FLUTE", 76.9, 0.2, 0.3, 0.8).unwrap() != x);
    }

    #[test]
    fn a_note_refuses_what_it_cannot_play() {
        let ep = episode();
        let (ctx, voices) = context(&ep);
        let start = ctx.noise().unwrap().0.random();
        let note = |midi: f64, dur: f64, cap: f64, vel: f64| refused(ctx.note("FLUTE", midi, dur, cap, vel));
        assert_eq!(note(f64::NAN, 0.2, 0.3, 0.8), "note: midi is NaN, not a number");
        assert_eq!(note(76.0, f64::INFINITY, 0.3, 0.8), "note: dur is inf, not a number");
        assert_eq!(note(76.0, 0.2, f64::NAN, 0.8), "note: cap is NaN, not a number");
        assert_eq!(note(76.0, 0.2, 0.3, f64::NAN), "note: vel is NaN, not a number");
        assert_eq!(
            note(76.0, 700.0, 0.3, 0.8),
            "note: a held length (dur) of 700 s is longer than 600 s"
        );
        assert_eq!(
            note(76.0, 0.2, 1.0 / 48000.0, 0.8),
            "note: 1 sample(s) with a fade of 0.04 s leave no room for a fade (3 samples and a fade of one sample \
             are the least)"
        );
        assert_eq!(
            note(76.0, 0.2, 1e9, 0.8),
            "note: cap of 1000000000 s is longer than 600 s"
        );
        assert_eq!(
            refused(ctx.note("TUBA", 40.0, 0.2, 0.3, 0.8)),
            "note: \"TUBA\" is not the voice of a fighter of this fight (\"FLUTE\" and \"TROMBONE\" are)"
        );
        assert!(refused(ctx.inst_gain("TUBA")).starts_with("inst_gain: \"TUBA\" is not the voice"));
        // Nothing was synthesised, nothing drawn, nothing logged.
        assert!(voices[0].calls().is_empty() && ctx.first_notes().unwrap().is_empty());
        assert_eq!(ctx.noise().unwrap().0.random(), start);
        // A note of no held length and a note of no length are notes (Python plays both).
        assert_eq!(ctx.note("FLUTE", 76.0, 0.0, 0.3, 0.8).unwrap().len(), 14400);
        assert!(ctx.note("FLUTE", 76.0, 0.2, 0.0, 0.8).unwrap().is_empty());
        // A pitch the voice cannot make: the sine of an infinite frequency is not a number.
        let e = note(1e6, 0.2, 0.3, 0.8);
        assert!(
            e.starts_with("note: FLUTE gives samples that are not numbers for MIDI note 1000000"),
            "{e}"
        );
    }

    /// A voice that plays nothing at all.
    struct Mute;

    impl Voice for Mute {
        fn note(&self, _: &mut PpNoise, _: f64, _: f64, _: f64) -> Vec<f64> {
            Vec::new()
        }

        fn register(&self) -> (i32, i32) {
            (-3, 0)
        }

        fn name(&self) -> &str {
            "FLUTE"
        }
    }

    #[test]
    fn a_voice_without_sound_has_no_gain() {
        let ep = episode();
        let ctx = MusicCtx::new(
            &ep.show,
            &ep.rec,
            &ep.ff,
            [&ep.cfg[0], &ep.cfg[1]],
            [Arc::new(Mute), TestVoice::new("TROMBONE", 0)],
        )
        .unwrap();
        // (lo + hi) // 2 floors: (-3 + 0) // 2 is -2.
        assert_eq!(
            refused(ctx.inst_gain("FLUTE")),
            "inst_gain: FLUTE gives no sound for MIDI note -2, the middle of its register"
        );
        assert!(refused(ctx.note("FLUTE", 76.0, 0.2, 0.3, 0.8)).starts_with("note: FLUTE gives no sound"));
    }

    #[test]
    fn line_overrides_are_kept_by_fighter() {
        let ep = episode();
        let (ctx, _) = context(&ep);
        assert!(ctx.line_overrides("TROMBONE").unwrap().is_empty());
        ctx.line_override(1, 10.0, 12.0, 10.0, &[(0.0, Some(52), 1.0), (1.0, None, 1.0)])
            .unwrap();
        ctx.line_override(1, 20.0, 22.0, 20.5, &[]).unwrap();
        let got = ctx.line_overrides("TROMBONE").unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(
            got[0],
            LineOverride {
                t0: 10.0,
                t1: 12.0,
                tz: 10.0,
                notes: vec![(0.0, Some(52), 1.0), (1.0, None, 1.0)]
            }
        );
        assert_eq!((got[1].t0, got[1].tz), (20.0, 20.5));
        assert!(ctx.line_overrides("FLUTE").unwrap().is_empty());
        // They belong to the fight, not to one pass over its notes.
        ctx.restart(PpNoise::start()).unwrap();
        assert_eq!(ctx.line_overrides("TROMBONE").unwrap().len(), 2);
    }

    /// One event of a recording.
    fn event(t: f64, kind: &str, fields: Vec<Value>) -> Event {
        Event {
            t,
            kind: kind.into(),
            fields,
        }
    }

    #[test]
    fn shared_events_place_the_rings_sounds() {
        let mut ep = episode();
        let t_ko = ep.show.t_ko;
        let (f, i) = (Value::Float, Value::Int);
        ep.rec.events = vec![
            // [t, 'rope', who, ., ., e[5], e[6], e[7]]: e[5] pans the thud, e[7] is how hard the ball hit.
            event(5.0, "rope", vec![i(1), i(1), i(-1), f(-0.7), f(-2.4), f(2.6)]),
            event(6.0, "hit", vec![i(1), i(0), f(1.4)]),
            event(7.0, "clash", vec![i(-1), f(-0.6), f(-0.7)]),
            event(8.01, "block", vec![i(0)]),
            event(9.0, "parry", vec![i(1)]),
            // Far to the right and hard: the pan stops at 0.6, the level at 1.5 times; a whole number counts.
            event(10.0, "rope", vec![i(0), i(0), i(1), f(5.0), f(0.0), i(8)]),
            // Up to 10 ms past the K.O. an event still sounds; the first one later ends the list, whatever
            // follows it (the kit's loop breaks there).
            event(t_ko + 0.005, "parry", vec![]),
            event(t_ko + 0.02, "rope", vec![]),
            event(11.0, "parry", vec![]),
        ];
        let (ctx, _) = context(&ep);
        let bed = TestBed::new((0.05, 0.02, 0.2));
        let mut mix = Mix::new(52.0);
        shared_events(&ctx, &bed, &mut mix).unwrap();
        assert_eq!(mix.bus_names(), ["fx", "clash"]);
        assert_eq!(*bed.clashes.lock().unwrap(), [7.0]);

        // The same placements, written out as the kit writes them.
        let mut want = Mix::new(52.0);
        let ring = |m: f64, seed: u64| Sig::Mono(clank(m, seed, CLANK_DUR, &CLANK_DECAYS).unwrap());
        let thud = |seed: u64| Sig::Mono(synth::thud(0.22, 95.0, 55.0, seed));
        want.place("fx", &thud(50), 5.0, 0.05 * (2.6 / 2.6), -0.7 / 3.5);
        want.place("fx", &ring(84.0, 40), ctx.q16(8.01), 0.06, 0.0);
        want.place("fx", &ring(88.0, 81), 9.0, 0.04, 0.0);
        want.place("fx", &thud(100), 10.0, 0.05 * 1.5, 0.6);
        want.place(
            "fx",
            &ring(88.0, ((t_ko + 0.005) * 9.0) as u64),
            t_ko + 0.005,
            0.04,
            0.0,
        );
        assert!(mix.bus("fx") == want.bus("fx"));
        // The block rings on the 16th, not where it happened.
        assert!(ctx.q16(8.01) != 8.01);
        let at = (ctx.q16(8.01) * SRF).round() as usize;
        let fx = mix.bus("fx").unwrap();
        assert!(fx[at] != [0.0, 0.0] && fx[at - 1] == [0.0, 0.0]);
    }

    #[test]
    fn shared_events_refuse_an_event_they_cannot_read() {
        let mut ep = episode();
        let (f, i) = (Value::Float, Value::Int);
        let bed = TestBed::new((0.05, 0.02, 0.2));
        let mut run = |events: Vec<Event>| -> Result<(), MusicError> {
            ep.rec.events = events;
            let (ctx, _) = context(&ep);
            shared_events(&ctx, &bed, &mut Mix::new(1.0))
        };
        // A rope bounce without its speed, and one whose pan is a word.
        assert_eq!(
            refused(run(vec![event(5.0, "rope", vec![i(1), i(1), i(-1), f(-0.7), f(-2.4)])])),
            "shared_events: the rope event at 5 s has no number as element 7 (it has None)"
        );
        let e = refused(run(vec![event(
            5.0,
            "rope",
            vec![i(1), i(1), i(-1), Value::Str("left".into()), f(-2.4), f(2.6)],
        )]));
        assert!(
            e.contains("has no number as element 5 (it has Some(Str(\"left\")))"),
            "{e}"
        );
        // A time before the start is no noise seed (numpy refuses a seed below zero).
        assert_eq!(
            refused(run(vec![event(-2.0, "parry", vec![])])),
            "shared_events: -18 is not a noise seed"
        );
        // What the bed refuses comes back as it is.
        assert_eq!(
            refused(run(vec![event(-1.0, "clash", vec![])])),
            "clash: a clash at -1 s"
        );
        // Nothing to read, nothing wrong: other kinds are not this function's.
        assert_eq!(run(vec![event(5.0, "hit", vec![]), event(6.0, "ko", vec![])]), Ok(()));
        assert_eq!(run(Vec::new()), Ok(()));
    }

    #[test]
    fn a_note_below_zero_follows_python() {
        let ep = episode();
        let (ctx, voices) = context(&ep);
        // A held length a little below zero is a note: what the voice rings on past it (the test voice 0.1 s,
        // so 0.05 s are left here), padded to the cap.
        let x = ctx.note("FLUTE", 76.0, -0.05, 0.3, 0.8).unwrap();
        assert_eq!(x.len(), 14400);
        assert_eq!(voices[0].calls()[0], (76.0, 0.8, -0.05));
        assert!(x[2399] != 0.0 && x[2400..].iter().all(|v| *v == 0.0));
        // A held length the voice cannot hold is refused before anything is drawn (Python raises there).
        let before = ctx.noise().unwrap().0.random();
        assert_eq!(
            refused(ctx.note("FLUTE", 77.0, -0.2, 0.3, 0.8)),
            "note: FLUTE has no note for a held length (dur) of -0.2 s: it would end before it starts"
        );
        assert_eq!(ctx.noise().unwrap().0.random(), before);
        assert_eq!(voices[0].calls().len(), 2);

        // A cap below zero cuts that much off the end of what the voice gave (Python's slice `x[:n]`), then
        // fades what is left: here 14,400 samples less 4,800.
        let whole = ctx.note("TROMBONE", 52.0, 0.2, 0.3, 0.8).unwrap();
        let cut = ctx.note("TROMBONE", 52.0, 0.2, -0.1, 0.8).unwrap();
        assert_eq!((whole.len(), cut.len()), (14400, 9600));
        assert_eq!(cut[..7680], whole[..7680]);
        assert!(cut[7681] != whole[7681] && cut[9599] == 0.0);
        // More than there is: an empty note.
        assert!(ctx.note("TROMBONE", 52.0, 0.2, -1.0, 0.8).unwrap().is_empty());
        // 1 or 2 samples left have no room for the fade: found once the voice has given its note.
        let calls = voices[1].calls().len();
        let e = refused(ctx.note("TROMBONE", 53.0, 0.2, -14398.5 / 48000.0, 0.8));
        assert_eq!(
            e,
            "note: 2 sample(s) with a fade of 0.04 s leave no room for a fade (3 samples and a fade of one sample \
             are the least)"
        );
        assert_eq!(voices[1].calls().len(), calls + 1);
    }

    #[test]
    fn the_voices_must_be_the_fighters() {
        let ep = episode();
        let cfg = [&ep.cfg[0], &ep.cfg[1]];
        let swapped: [Arc<dyn Voice>; 2] = [TestVoice::new("TROMBONE", 0), TestVoice::new("FLUTE", 0)];
        assert_eq!(
            refused(MusicCtx::new(&ep.show, &ep.rec, &ep.ff, cfg, swapped)),
            "music: voice 0 is \"TROMBONE\", but FLUTE plays \"FLUTE\""
        );
        let other: [Arc<dyn Voice>; 2] = [TestVoice::new("FLUTE", 0), TestVoice::new("TUBA", 0)];
        assert_eq!(
            refused(MusicCtx::new(&ep.show, &ep.rec, &ep.ff, cfg, other)),
            "music: voice 1 is \"TUBA\", but TROMBONE plays \"TROMBONE\""
        );
    }

    /// A voice that breaks on MIDI note 13, after it has drawn its noise.
    struct Shaky;

    impl Voice for Shaky {
        fn note(&self, noise: &mut PpNoise, midi: f64, _: f64, _: f64) -> Vec<f64> {
            let x = noise.nz(10);
            assert!(midi != 13.0, "the voice broke");
            x
        }

        fn register(&self) -> (i32, i32) {
            (60, 72)
        }

        fn name(&self) -> &str {
            "FLUTE"
        }
    }

    #[test]
    fn a_voice_that_panicked_ends_the_build() {
        let ep = episode();
        let ctx = MusicCtx::new(
            &ep.show,
            &ep.rec,
            &ep.ff,
            [&ep.cfg[0], &ep.cfg[1]],
            [Arc::new(Shaky), TestVoice::new("TROMBONE", 0)],
        )
        .unwrap();
        assert!(ctx.note("FLUTE", 76.0, 0.2, 0.3, 0.8).is_ok());
        // Formatting a context does not wait for the lock: it works while a note is being made.
        {
            let _making_a_note = ctx.state.lock().unwrap();
            assert!(format!("{ctx:?}").contains("[\"FLUTE\", \"TROMBONE\"]"));
        }
        let broke = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ctx.note("FLUTE", 13.0, 0.2, 0.3, 0.8)));
        assert!(broke.is_err());
        // The voice had drawn noise before it broke, so every later note would be silently another note: nothing
        // of this build is handed out any more, not even a note that was cached.
        let dead = "music: a voice panicked while a note was being made: the noise stream and the notes of this \
                    build cannot be trusted any more";
        assert_eq!(refused(ctx.note("FLUTE", 76.0, 0.2, 0.3, 0.8)), dead);
        assert_eq!(refused(ctx.note("TROMBONE", 52.0, 0.2, 0.3, 0.8)), dead);
        assert_eq!(refused(ctx.inst_gain("FLUTE")), dead);
        assert_eq!(refused(ctx.noise()), dead);
        assert_eq!(refused(ctx.first_notes()), dead);
        assert_eq!(refused(ctx.restart(PpNoise::start())), dead);
        assert_eq!(refused(ctx.line_override(0, 1.0, 2.0, 1.0, &[])), dead);
        assert_eq!(refused(ctx.line_overrides("FLUTE")), dead);
        assert!(format!("{ctx:?}").contains("FLUTE"));
    }

    #[test]
    fn a_line_override_is_for_one_of_the_two() {
        let ep = episode();
        let (ctx, _) = context(&ep);
        assert_eq!(
            refused(ctx.line_override(2, 1.0, 2.0, 1.0, &[])),
            "line_override: fighter 2 (there are two, 0 and 1)"
        );
        assert!(ctx.line_overrides("FLUTE").unwrap().is_empty());
    }
}
