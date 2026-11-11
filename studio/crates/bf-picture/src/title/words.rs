//! The words of the card: a pixel-font word at a whole-number scale, resized by the slam's size factor, with an
//! ink outline, a drop shadow and a pale highlight on its top two font rows; for the middle word (the kit's
//! `MIDDLE`) turned about its centre first.
//!
//! The port of `rotate_mask`, `word_masks` and `stamp_word`. A word is first stamped on a layer of its own
//! (Python's `wl = np.zeros_like(layer)` and `wa = np.zeros_like(alpha)`), which the card then lays over a panel;
//! `VS` is laid over both panels, each keeping its half. [`WordLayer`] is that layer, held only over the rectangle
//! the stamps can reach: everywhere else Python's two arrays stay zero and take no part. A name on two lines is
//! two stamps on one layer ([`stamp_words`]).

use bf_core::{dmath, py};

use super::layout::radians;
use super::{INK, over};
use crate::font;
use crate::img::Img;
use crate::np;
use crate::np::index::resample_index;

/// `word_masks(text, scale, k)`: the masks of a word and the thickness of its outline.
#[derive(Clone, Debug)]
pub struct WordMasks {
    /// `m`: the letters.
    pub fill: Img<u8>,
    /// `o`: the letters grown by the outline's thickness in every direction, on a canvas that much wider and
    /// higher on each side.
    pub outline: Img<u8>,
    /// `hi`: the top two font rows of the letters.
    pub hi: Img<u8>,
    /// `r`: `max(2, int(round(scale * 0.6 * k)))`, the offset the outline is stamped at; 0 for a turned word,
    /// whose masks share one frame (Python returns 0 there).
    pub r: i32,
}

/// Python's `max(1, int(round(x)))` for a size in pixels.
fn size_of(x: f64) -> usize {
    np::f64_to_i64(py::round_half_even(x)).max(1) as usize
}

/// `rotate_mask(m, deg, size=None)`: a mask turned `deg` degrees anticlockwise about its centre, nearest
/// neighbour (crisp pixel letters). `size` (height, width) fixes the output frame so the masks of one word line
/// up; `None` makes it just big enough, plus 2.
pub fn rotate_mask(m: &Img<u8>, deg: f64, size: Option<(usize, usize)>) -> Img<u8> {
    let a = radians(deg);
    let (ca, sa) = (dmath::cos(a), dmath::sin(a));
    let (h, w) = (m.h as f64, m.w as f64);
    // (int(math.ceil(abs(h * ca) + abs(w * sa))) + 2, int(math.ceil(abs(w * ca) + abs(h * sa))) + 2)
    let (ho, wo) = size.unwrap_or_else(|| {
        (
            np::f64_to_i64(((h * ca).abs() + (w * sa).abs()).ceil()) as usize + 2,
            np::f64_to_i64(((w * ca).abs() + (h * sa).abs()).ceil()) as usize + 2,
        )
    });
    let mut out: Img<u8> = Img::new(wo, ho, 1);
    let (cx, cy) = ((wo as f64 - 1.0) / 2.0, (ho as f64 - 1.0) / 2.0);
    let (mx, my) = ((w - 1.0) / 2.0, (h - 1.0) / 2.0);
    for yy in 0..ho {
        let dy = yy as f64 - cy;
        for xx in 0..wo {
            let dx = xx as f64 - cx;
            // u = dx * ca - dy * sa + (w - 1) / 2;  v = dx * sa + dy * ca + (h - 1) / 2: back into the word's frame
            let u = dx * ca - dy * sa + mx;
            let v = dx * sa + dy * ca + my;
            // ui_, vi = np.round(u).astype(int), np.round(v).astype(int)
            let (ui, vi) = (
                np::f64_to_i64(py::round_half_even(u)),
                np::f64_to_i64(py::round_half_even(v)),
            );
            if (0..m.w as i64).contains(&ui) && (0..m.h as i64).contains(&vi) {
                out.data[yy * wo + xx] = m.data[vi as usize * m.w + ui as usize];
            }
        }
    }
    out
}

/// `word_masks(text, scale, k)`: the word in the pixel font at `scale`, resized by the factor `k` (nearest
/// neighbour; left as it is when `k` is within a thousandth of 1), its outline and its highlight.
pub fn word_masks(text: &str, scale: u32, k: f64) -> WordMasks {
    word_masks_at(text, scale, k, 0.0)
}

/// `word_masks(text, scale, k, ang)`: [`word_masks`], and for an angle other than 0 (Python's `if ang:`) the
/// word turned `ang` degrees anticlockwise: the letters and the highlight are turned into one frame padded by
/// `2 r + 2` on every side ([`rotate_mask`], nearest neighbour, before the outline, so the outline stays even),
/// the outline grown from the turned letters, and `r` is 0.
pub fn word_masks_at(text: &str, scale: u32, k: f64, ang: f64) -> WordMasks {
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
    if ang != 0.0 {
        let pad = 2 * r as usize + 2;
        let mr = rotate_mask(&m, ang, None);
        let size = (mr.h + 2 * pad, mr.w + 2 * pad);
        let (m, hi) = (rotate_mask(&m, ang, Some(size)), rotate_mask(&hi, ang, Some(size)));
        let outline = font::dilate(&m, r as u32);
        // (outline already inside the frame)
        return WordMasks {
            fill: m,
            outline,
            hi,
            r: 0,
        };
    }
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

/// What `stamp_word` knows of the words it stamped: the layer, and the boxes it files for the safe-zone check
/// when the word is settled. An upright word files one box, `(x0 - r, y0 - r, x0 + wm + r, y0 + hm + r)` (the
/// letters with their outline); a turned word files narrow strips, one per 12 columns of its outline.
#[derive(Clone, Debug)]
pub struct Stamped {
    pub layer: WordLayer,
    pub noted: Vec<(i32, i32, i32, i32)>,
}

/// One word placed on the frame: its masks and the top left of its letters, `x0, y0 = int(round(cx - wm / 2)),
/// int(round(cy - hm / 2))`.
struct Placed {
    masks: WordMasks,
    x0: i32,
    y0: i32,
}

impl Placed {
    fn new(text: &str, scale: u32, k: f64, cx: f64, cy: f64, ang: f64) -> Placed {
        let masks = word_masks_at(text, scale, k, ang);
        let (hm, wm) = (masks.fill.h as i32, masks.fill.w as i32);
        let x0 = py::round_half_even(cx - f64::from(wm) / 2.0) as i32;
        let y0 = py::round_half_even(cy - f64::from(hm) / 2.0) as i32;
        Placed { masks, x0, y0 }
    }

    /// The rectangle the four stamps can reach on a `fw x fh` frame: the outline at `(x0 - r, y0 - r)` and its
    /// shadow 3 right and 4 down, as `(x0, y0, x1, y1)` clipped to the frame.
    fn bounds(&self, fw: i32, fh: i32) -> (i32, i32, i32, i32) {
        let r = self.masks.r;
        let (ow, oh) = (self.masks.outline.w as i32, self.masks.outline.h as i32);
        let (bx0, by0) = ((self.x0 - r).clamp(0, fw), (self.y0 - r).clamp(0, fh));
        let (bx1, by1) = (
            (self.x0 - r + 3 + ow).clamp(bx0, fw),
            (self.y0 - r + 4 + oh).clamp(by0, fh),
        );
        (bx0, by0, bx1, by1)
    }

    /// The boxes `stamp_word` notes for the word (`note=True`).
    fn noted(&self, turned: bool) -> Vec<(i32, i32, i32, i32)> {
        let m = &self.masks;
        let (x0, y0, r) = (self.x0, self.y0, m.r);
        let (hm, wm) = (m.fill.h as i32, m.fill.w as i32);
        if !turned {
            return vec![(x0 - r, y0 - r, x0 + wm + r, y0 + hm + r)];
        }
        // #54: the turned word as narrow strips: cols = np.nonzero(o.any(0))[0]
        let o = &m.outline;
        let col_on = |c: usize| (0..o.h).any(|y| o.data[y * o.w + c] != 0);
        let cols: Vec<usize> = (0..o.w).filter(|&c| col_on(c)).collect();
        let (Some(&first), Some(&last)) = (cols.first(), cols.last()) else {
            return Vec::new(); // (Python fails on an empty outline; a fitted middle word is never empty)
        };
        let mut out = Vec::new();
        for c0 in (first..=last).step_by(12) {
            // ys = np.nonzero(o[:, c0:c0 + 12].any(1))[0]
            let c1 = (c0 + 12).min(o.w);
            let ys: Vec<usize> = (0..o.h)
                .filter(|&y| o.data[y * o.w + c0..y * o.w + c1].iter().any(|&v| v != 0))
                .collect();
            if let (Some(&ya), Some(&yb)) = (ys.first(), ys.last()) {
                let c0i = c0 as i32;
                out.push((x0 + c0i, y0 + ya as i32, x0 + (c0i + 12).min(wm), y0 + yb as i32 + 1));
            }
        }
        out
    }
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
    stamp_words(frame_w, frame_h, &[(text, cx, cy)], scale, k, col, a, 0.0)
}

/// `stamp_word` once for each of `words` (`(text, cx, cy)`), in order, on one empty layer: a name on two lines
/// (`TITLE_LINES`: one stamp a line, the second mixed over the first), or one word, turned `ang` degrees when
/// `ang` is not 0 (the middle word). The noted boxes are each stamp's, in order.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn stamp_words(
    frame_w: usize,
    frame_h: usize,
    words: &[(&str, f64, f64)],
    scale: u32,
    k: f64,
    col: [f64; 3],
    a: f64,
    ang: f64,
) -> Stamped {
    let (fw, fh) = (frame_w as i32, frame_h as i32);
    let placed: Vec<Placed> = words
        .iter()
        .map(|&(text, cx, cy)| Placed::new(text, scale, k, cx, cy, ang))
        .collect();
    // one rectangle that every stamp can reach
    let mut b = (fw, fh, 0, 0);
    for p in &placed {
        let (x0, y0, x1, y1) = p.bounds(fw, fh);
        if x1 > x0 && y1 > y0 {
            b = (b.0.min(x0), b.1.min(y0), b.2.max(x1), b.3.max(y1));
        }
    }
    if b.2 <= b.0 || b.3 <= b.1 {
        b = (0, 0, 0, 0);
    }
    let (bw, bh) = ((b.2 - b.0) as usize, (b.3 - b.1) as usize);
    let mut layer = WordLayer {
        x: b.0 as usize,
        y: b.1 as usize,
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
    let mut noted = Vec::new();
    for p in &placed {
        let (m, r, x0, y0) = (&p.masks, p.masks.r, p.x0, p.y0);
        layer.stamp(frame, &m.outline, (x0 - r + 3, y0 - r + 4), shadow, a);
        layer.stamp(frame, &m.outline, (x0 - r, y0 - r), INK, a);
        layer.stamp(frame, &m.fill, (x0, y0), col, a);
        layer.stamp(frame, &m.hi, (x0, y0), pale, a);
        noted.extend(p.noted(ang != 0.0));
    }
    Stamped { layer, noted }
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
