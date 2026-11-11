//! Shaping a note: `fit`, the pitch warps (`gliss`, `bend`, `vibrato`), the plunger `wah`, and `colour`.

use bf_core::dmath::{self, PI};

use super::error::{MusicError, Samples};
use super::num::{band_span, check_band, check_cut, int, np_maximum};
use crate::SRF;
use crate::dsp;
use crate::synth::clip;
use crate::synth::{self, FILTER_O};
use bf_core::py::{py_max, py_min};

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

/// Why `fit` refuses a note of `len` samples with a fade of `fade` seconds.
pub(super) fn no_room(len: usize, fade: f64) -> String {
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
/// applied), and `lead` is `(own - other) / 100`. [`MusicCtx::colour`](super::MusicCtx::colour) reads the two HP
/// values from the fight.
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
