//! The two panels of the card and what sits on them: the fill (a gradient in the fighter's colours, its pattern,
//! dithered bands) and the fighter's glass ball, big and close.
//!
//! The port of `_panel_rgb`, of the pattern table `PATTERNS` as far as the Studio's fighters need it
//! (`_slide_tubes`, the TROMBONE's) and of `draw_big_ball`. Everything is `float64`, pixel by pixel in the order
//! numpy's whole-array expressions run.
//!
//! One number is not Python's to the bit. The gradient's `** 1.3` is numpy's `power`, which calls the C runtime's
//! `pow` for every element on the reference PC; here it is `bf_core::dmath::pow`, and the two differ in the last
//! bit for 9 % of the frame's pixels (21,097 of 230,400). The gradient before the dither then differs in its
//! last bit on those pixels and on no other (with Python's `k` the rest of the arithmetic is Python's to the bit
//! on every pixel). The dither after it is a `floor`, which gives another band only for a value within about
//! 1e-14 of a band's edge: every pixel of every panel compared (the two of the reference cards and twelve more)
//! is in Python's band.

use bf_core::{dmath, py};

use super::INK;
use super::layout;
use crate::config::{self, H, W};
use crate::img::{Img, RgbF};
use crate::np;

/// A panel pattern: `fn(base) -> base` in Python, working on the panel's `(H, W, 3)` `float64` gradient before it
/// is dithered. Here it changes the picture in place.
pub type Pattern = fn(&mut RgbF);

/// The fighters `title.PATTERNS` has a panel pattern for that is not ported yet: the kit's sixteen but the
/// TROMBONE's. Each comes with its fighter, in [`builtin_pattern`].
pub const UNPORTED_PATTERNS: [&str; 15] = [
    "BAGPIPES",
    "ACCORDION",
    "ORGAN",
    "HARP",
    "HORN",
    "UKULELE",
    "ACOUSTIC",
    "CLARINET",
    "OBOE",
    "BASS CLARINET",
    "EUPHONIUM",
    "BASSOON",
    "BARI SAX",
    "VIOLA",
    "VIOLIN",
];

/// What can keep a title card from being built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TitleError {
    /// The card's own pattern table has a panel pattern for this fighter, and it is not ported yet: without it
    /// the card would quietly show a plain panel where Python shows the pattern.
    PatternNotPorted(String),
}

impl std::fmt::Display for TitleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TitleError::PatternNotPorted(name) => write!(f, "the title pattern {name} is not ported yet"),
        }
    }
}

impl std::error::Error for TitleError {}

/// `PATTERNS.get(name)`: the pattern the card itself has for a fighter, by name, or `None` when it has none (the
/// fighter then gets its look's pattern, or the plain gradient). Of the kit's sixteen only the TROMBONE's is
/// ported: for the other fifteen ([`UNPORTED_PATTERNS`]) this is an error rather than a plain panel.
pub fn builtin_pattern(name: &str) -> Result<Option<Pattern>, TitleError> {
    match name {
        "TROMBONE" => Ok(Some(slide_tubes)),
        _ if UNPORTED_PATTERNS.contains(&name) => Err(TitleError::PatternNotPorted(name.to_string())),
        _ => Ok(None),
    }
}

/// `_slide_tubes(base)`, the TROMBONE's panel: pairs of slide tubes along the divider, every 34 px, each pair a
/// lighter tube either side of a darker gap.
pub fn slide_tubes(base: &mut RgbF) {
    let side = layout::side();
    for (px, &s) in base.data.as_chunks_mut::<3>().0.iter_mut().zip(&side.data) {
        // q = (SIDE / 34.0) % 1
        let q = py::fmod_py(s / 34.0, 1.0);
        let tube = (q - 0.40).abs() < 0.045 || (q - 0.60).abs() < 0.045;
        let gap = (q - 0.5).abs() < 0.06;
        // base * np.where(tube, 1.16, np.where(gap, 0.86, 1.0))
        let gain = if tube {
            1.16
        } else if gap {
            0.86
        } else {
            1.0
        };
        for v in px {
            *v *= gain;
        }
    }
}

/// `_panel_rgb(colour, dark, side, name)`: a panel's fill over the whole frame: a gradient from the fighter's
/// deep shade far from the divider to its colour at the divider, then `pattern` (Python's `PATTERNS[name]`; pass
/// a closure that does nothing for the plain panel), then ten dithered bands. Values are multiples of 25.5.
pub fn panel_rgb(colour: [f64; 3], dark: [f64; 3], pattern: impl FnOnce(&mut RgbF)) -> RgbF {
    let side = layout::side();
    let mut base: RgbF = Img::new(W, H, 3);
    for (px, &s) in base.data.as_chunks_mut::<3>().0.iter_mut().zip(&side.data) {
        // d = np.abs(SIDE) / 400.0;  k = np.clip(1 - d, 0, 1) ** 1.3
        let d = s.abs() / 400.0;
        let k = dmath::pow(np::clip(1.0 - d, 0.0, 1.0), 1.3);
        for c in 0..3 {
            // dark * (1 - k) + colour * (0.40 + 0.25 * k) * k + dark * 0.6 * k
            px[c] = dark[c] * (1.0 - k) + colour[c] * (0.40 + 0.25 * k) * k + dark[c] * 0.6 * k;
        }
    }
    pattern(&mut base);
    let bayer = config::bayer_full();
    for (px, &b) in base.data.as_chunks_mut::<3>().0.iter_mut().zip(&bayer.data) {
        for v in px {
            // q = np.floor((base / 255.0) * 10 + BAYER_FULL * 0.9) / 10 * 255;  np.clip(q, 0, 255)
            let q = ((*v / 255.0) * 10.0 + b * 0.9).floor() / 10.0 * 255.0;
            *v = np::clip(q, 0.0, 255.0);
        }
    }
    base
}

/// The ink of the ball's small highlight, `np.array([255, 252, 244])`.
const SPECULAR: [f64; 3] = [255.0, 252.0, 244.0];

/// `draw_big_ball(layer, alpha, k, cx, cy, rp, glow, col)`: a glass ball as in the ring, at close-up size, with
/// the fighter's instrument inside: the glass (tinted `glass`, shaded in four dithered bands), the sprite `spr`
/// (Python's `big_sprite(k)`, RGBA and square) pasted on its centre and brightened by `glow`, the rim in the
/// fighter's colour `col`, an ink edge and a small highlight. `layer` is the panel's `(H, W, 3)` picture, `alpha`
/// its `(H, W)` opacity, set to 1 over the ball and its edge.
///
/// `cx` is a whole number (the Python passes the int of `BALLS`), `cy` and the radius `rp` are floats.
#[allow(clippy::too_many_arguments)] // the Python signature, with the sprite and the glass tint it looks up
pub fn draw_big_ball(
    layer: &mut RgbF,
    alpha: &mut Img<f64>,
    spr: &Img<u8>,
    glass: [f64; 3],
    cx: i32,
    cy: f64,
    rp: f64,
    glow: f64,
    col: [f64; 3],
) {
    let (w, h) = (layer.w as i32, layer.h as i32);
    // ri = int(rp) + 3;  x0, y0 = int(cx) - ri, int(cy) - ri;  n = 2 * ri + 1
    let ri = np::f64_to_i32(rp) + 3;
    let (x0, y0) = (cx - ri, np::f64_to_i32(cy) - ri);
    let n = 2 * ri + 1;
    let (xa, ya) = (x0.max(0), y0.max(0));
    let (xb, yb) = (w.min(x0 + n), h.min(y0 + n));
    if xb <= xa || yb <= ya {
        return;
    }
    let fcx = f64::from(cx);
    // dx = (xx + x0 + 0.5 - cx) / rp;  dy = (yy + y0 + 0.5 - cy) / rp;  d2 = dx * dx + dy * dy
    let at = |x: i32, y: i32| {
        let dx = (f64::from(x) + 0.5 - fcx) / rp;
        let dy = (f64::from(y) + 0.5 - cy) / rp;
        (dx, dy, dx * dx + dy * dy)
    };
    // (1 - 3.0 / rp) ** 2 and (1 + 2.5 / rp) ** 2: Python floats, squared as products here
    let rim_in = {
        let a = 1.0 - 3.0 / rp;
        a * a
    };
    let edge_out = {
        let a = 1.0 + 2.5 / rp;
        a * a
    };

    // reg[m] = inner[sl][m]: the glass
    let tint = [0, 1, 2].map(|c| col[c] * 0.30 + glass[c] * 0.70);
    for y in ya..yb {
        for x in xa..xb {
            let (dx, dy, d2) = at(x, y);
            if d2 <= 1.0 {
                let nz = np::clip(1.0 - d2, 0.0, 1.0).sqrt();
                let lam = np::clip(-0.25 * dx - 0.55 * dy + nz * 0.6, 0.0, 1.0);
                // band = np.floor(lam * 4 + BAYER_FULL[:n, :n] * 0.9) / 4: the dither starts at the ball's own
                // corner, not the frame's
                let bayer = config::BAYER[((y - y0) % 4) as usize][((x - x0) % 4) as usize];
                let band = (lam * 4.0 + bayer * 0.9).floor() / 4.0;
                for (p, t) in layer.px_mut(x as usize, y as usize).iter_mut().zip(tint) {
                    *p = t * (0.70 + 0.36 * band);
                }
            }
        }
    }

    // the instrument: rg[ms] = np.clip(sub[..., :3].astype(float)[ms] * (1.06 + 0.3 * glow), 0, 255)
    let sn = spr.h as i32;
    let (sx0, sy0) = (cx - sn / 2, py::round_half_even(cy) as i32 - sn / 2);
    let (ax0, ay0) = (sx0.max(0), sy0.max(0));
    let (ax1, ay1) = (w.min(sx0 + sn), h.min(sy0 + sn));
    let lit = 1.06 + 0.3 * glow;
    for y in ay0..ay1 {
        for x in ax0..ax1 {
            let s = spr.px((x - sx0) as usize, (y - sy0) as usize);
            if s[3] > 0 {
                for (p, &v) in layer.px_mut(x as usize, y as usize).iter_mut().zip(&s[..3]) {
                    *p = np::clip(f64::from(v) * lit, 0.0, 255.0);
                }
            }
        }
    }

    // the rim, the ink edge, the highlight, and the ball's opacity
    let rim_col = col.map(|c| np::clip(c * (0.9 + 0.5 * glow) + 50.0 * glow, 0.0, 255.0));
    for y in ya..yb {
        for x in xa..xb {
            let (dx, dy, d2) = at(x, y);
            let inside = d2 <= 1.0;
            let edge = d2 > 1.0 && d2 <= edge_out;
            let px = layer.px_mut(x as usize, y as usize);
            if inside && d2 > rim_in {
                px.copy_from_slice(&rim_col);
            }
            if edge {
                px.copy_from_slice(&INK);
            }
            // spec = inside & ((dx + 0.38) ** 2 + (dy + 0.5) ** 2 < 0.012)
            let (ex, ey) = (dx + 0.38, dy + 0.5);
            if inside && ex * ex + ey * ey < 0.012 {
                px.copy_from_slice(&SPECULAR);
            }
            if inside || edge {
                alpha.px_mut(x as usize, y as usize)[0] = 1.0;
            }
        }
    }
}
