//! The words of the card: a pixel-font word at a whole-number scale, resized by the slam's size factor, with an
//! ink outline, a drop shadow and a pale highlight on its top two font rows.
//!
//! The port of `word_masks` and `stamp_word`. A word is first stamped on a layer of its own (Python's
//! `wl = np.zeros_like(layer)` and `wa = np.zeros_like(alpha)`), which the card then lays over a panel; `VS` is
//! laid over both panels, each keeping its half. [`WordLayer`] is that layer, held only over the rectangle the
//! stamp can reach: everywhere else Python's two arrays stay zero and take no part.

use bf_core::py;

use super::{INK, over};
use crate::font;
use crate::img::Img;
use crate::np;

/// `word_masks(text, scale, k)`: the masks of a word and the thickness of its outline.
#[derive(Clone, Debug)]
pub struct WordMasks {
    /// `m`: the letters.
    pub fill: Img<u8>,
    /// `o`: the letters grown by `r` in every direction, on a canvas `2 r` wider and higher.
    pub outline: Img<u8>,
    /// `hi`: the top two font rows of the letters.
    pub hi: Img<u8>,
    /// `r`: `max(2, int(round(scale * 0.6 * k)))`.
    pub r: i32,
}

/// The source index of each of `n` new rows or columns of an axis `size` long:
/// `np.minimum((np.arange(n) * size / n).astype(int), size - 1)`, nearest neighbour by truncation.
fn resample_index(n: usize, size: usize) -> Vec<usize> {
    (0..n)
        .map(|i| {
            let src = np::f64_to_i64((i * size) as f64 / n as f64).max(0) as usize;
            src.min(size.saturating_sub(1))
        })
        .collect()
}

/// Python's `max(1, int(round(x)))` for a size in pixels.
fn size_of(x: f64) -> usize {
    np::f64_to_i64(py::round_half_even(x)).max(1) as usize
}

/// `word_masks(text, scale, k)`: the word in the pixel font at `scale`, resized by the factor `k` (nearest
/// neighbour; left as it is when `k` is within a thousandth of 1), its outline and its highlight.
pub fn word_masks(text: &str, scale: u32, k: f64) -> WordMasks {
    let base = font::text_mask(text, scale);
    let m = if (k - 1.0).abs() > 1e-3 {
        let (h0, w0) = (base.h, base.w);
        let (hh, ww) = (size_of(h0 as f64 * k), size_of(w0 as f64 * k));
        let (ys, xs) = (resample_index(hh, h0), resample_index(ww, w0));
        // m = base[ys][:, xs]
        let mut m: Img<u8> = Img::new(ww, hh, 1);
        for (y, &sy) in ys.iter().enumerate() {
            let src = base.row(sy);
            for (o, &sx) in m.row_mut(y).iter_mut().zip(&xs) {
                *o = src[sx];
            }
        }
        m
    } else {
        base
    };
    let r = (py::round_half_even(f64::from(scale) * 0.6 * k) as i32).max(2);
    // gh = max(1, m.shape[0] // 7 * 2);  hi[:gh] = m[:gh]
    let gh = (m.h / 7 * 2).max(1).min(m.h);
    let mut hi: Img<u8> = Img::new(m.w, m.h, 1);
    hi.data[..gh * m.w].copy_from_slice(&m.data[..gh * m.w]);
    let outline = font::dilate(&m, r as u32);
    WordMasks {
        fill: m,
        outline,
        hi,
        r,
    }
}

/// A word on its own transparent layer: a `w x h` rectangle of the frame with its top left at `(x, y)`, holding
/// Python's `wl` (three colour samples a pixel) and `wa` (the opacity). Outside it both are zero.
#[derive(Clone, Debug)]
pub struct WordLayer {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub rgb: Vec<f64>,
    pub alpha: Vec<f64>,
}

/// What `stamp_word` knows of the word it stamped: the layer, and the box it files for the safe-zone check when
/// the word is settled, `(x0 - r, y0 - r, x0 + wm + r, y0 + hm + r)`: the letters with their outline.
#[derive(Clone, Debug)]
pub struct Stamped {
    pub layer: WordLayer,
    pub noted: (i32, i32, i32, i32),
}

/// `stamp_word(wl, wa, text, scale, k, cx, cy, col, a)` on an empty layer of a `frame_w x frame_h` frame: the word
/// at font scale `scale` and size factor `k`, centred on `(cx, cy)`, in the colour `col` at opacity `a`: the
/// outline's drop shadow (half ink, 3 px right and 4 px down), the ink outline, the letters, the highlight.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn stamp_word(
    frame_w: usize,
    frame_h: usize,
    text: &str,
    scale: u32,
    k: f64,
    cx: f64,
    cy: f64,
    col: [f64; 3],
    a: f64,
) -> Stamped {
    let masks = word_masks(text, scale, k);
    let r = masks.r;
    let (hm, wm) = (masks.fill.h as i32, masks.fill.w as i32);
    // x0, y0 = int(round(cx - wm / 2)), int(round(cy - hm / 2))
    let x0 = py::round_half_even(cx - f64::from(wm) / 2.0) as i32;
    let y0 = py::round_half_even(cy - f64::from(hm) / 2.0) as i32;
    let (fw, fh) = (frame_w as i32, frame_h as i32);

    // The rectangle the four stamps can reach: the outline at (x0 - r, y0 - r) and its shadow 3 right and 4 down.
    let (ow, oh) = (masks.outline.w as i32, masks.outline.h as i32);
    let (bx0, by0) = ((x0 - r).clamp(0, fw), (y0 - r).clamp(0, fh));
    let (bx1, by1) = ((x0 - r + 3 + ow).clamp(bx0, fw), (y0 - r + 4 + oh).clamp(by0, fh));
    let (bw, bh) = ((bx1 - bx0) as usize, (by1 - by0) as usize);
    let mut layer = WordLayer {
        x: bx0 as usize,
        y: by0 as usize,
        w: bw,
        h: bh,
        rgb: vec![0.0; bw * bh * 3],
        alpha: vec![0.0; bw * bh],
    };

    // The four stamps of Python's loop, `(mask, c, ox, oy, rr)` placed at `(x0 - rr + ox, y0 - rr + oy)`:
    // (o, INK * 0.5, 3, 4, r), (o, INK, 0, 0, r), (m, col, 0, 0, 0), (hi, np.clip(col * 0.55 + 255 * 0.45, 0, 255), 0, 0, 0)
    let shadow = INK.map(|c| c * 0.5);
    let pale = col.map(|c| np::clip(c * 0.55 + 255.0 * 0.45, 0.0, 255.0));
    let frame = (fw, fh);
    layer.stamp(frame, &masks.outline, (x0 - r + 3, y0 - r + 4), shadow, a);
    layer.stamp(frame, &masks.outline, (x0 - r, y0 - r), INK, a);
    layer.stamp(frame, &masks.fill, (x0, y0), col, a);
    layer.stamp(frame, &masks.hi, (x0, y0), pale, a);
    Stamped {
        layer,
        noted: (x0 - r, y0 - r, x0 + wm + r, y0 + hm + r),
    }
}

impl WordLayer {
    /// One stamp of `stamp_word`: the set pixels of `mask`, placed with its top left at `at` on a frame of size
    /// `frame`, are mixed with the colour `c` at opacity `a` (`reg[sub] = reg[sub] * (1 - a) + c * a`) and are
    /// at least `a` opaque (`ar[sub] = np.maximum(ar[sub], a)`). Pixels off the frame are dropped; the layer must
    /// hold every pixel of the frame the mask can reach.
    fn stamp(&mut self, frame: (i32, i32), mask: &Img<u8>, at: (i32, i32), c: [f64; 3], a: f64) {
        let (mh, mw) = (mask.h as i32, mask.w as i32);
        let (xa, ya) = (at.0.max(0), at.1.max(0));
        let (xb, yb) = (frame.0.min(at.0 + mw), frame.1.min(at.1 + mh));
        if xb <= xa || yb <= ya {
            return;
        }
        for y in ya..yb {
            let mrow = mask.row((y - at.1) as usize);
            for x in xa..xb {
                if mrow[(x - at.0) as usize] == 0 {
                    continue;
                }
                let i = (y as usize - self.y) * self.w + (x as usize - self.x);
                for (p, cc) in self.rgb[i * 3..i * 3 + 3].iter_mut().zip(c) {
                    *p = over(*p, cc, a);
                }
                self.alpha[i] = np::maximum(self.alpha[i], a);
            }
        }
    }
}
