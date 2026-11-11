//! The captions of `reelkit/ui.py` in the `impact` style: each line drops a few pixels and lands with weight (a
//! small punch in size, a shake of the text alone, a puff of pixel sand under the letters), the lines of a caption
//! one after another.
//!
//! A caption is `(t0, t1, lines)`; in a line `*word*` takes the highlight colour and `~word~` the second one. A
//! caption is drawn while `t0 <= t < t1`, so one whose window is empty (`t1 <= t0`: the kit makes one for a late
//! K.O.) is never drawn and lands nowhere.
//!
//! Numbers: everything is `f64` in Python's order of operations, with Python's `round` (ties to even) and
//! Python's `sum` (see [`hash01_phase`]). Three results come from the C runtime in Python and from elsewhere
//! here, and may differ from Python's in their last bit: the two sines (`bf_core::dmath::sin`), in the shake and
//! in [`hash01`], which scatters the sand; and the punch's square, Python's `** 2` (the C runtime's `pow`, not
//! always the product), a plain product here. Each is rounded to whole pixels or mixed into bytes afterwards, so
//! a picture can only differ where a value lies within about 1e-9 (the sand), 1e-13 (the punch) or 1e-15 (the
//! shake) of a rounding step. On the reference vectors the nearest is 5e-6, and every picture is Python's.

use super::{DrawLog, Kind, py_max, py_min, round_i32};
use crate::config::{SAFE, SAFE_CX, ss};
use crate::font::{self, ADV};
use crate::img::{Img, Rgb8};
use crate::np;
use bf_core::{dmath, py, pyhash};

/// `CAP_Y0`: the top of the first caption line, 4 px below the top of the safe box.
pub const CAP_Y0: i32 = SAFE.1 + 4;
/// `CAP_BIG_MAX`: the characters a line may have at the big size (scale 3) and still fit the safe box.
pub const CAP_BIG_MAX: usize = 14;
/// `CAP_SMALL_MAX`: the same at the small size (scale 2). Nothing enforces it: a longer line is drawn wider than
/// the safe box (and the safe-zone check reports it).
pub const CAP_SMALL_MAX: usize = 21;
/// `IMPACT_DROP`: the pixels a line falls before it lands.
pub const IMPACT_DROP: f64 = 4.0;
/// `IMPACT_FALL`: the seconds of the fall.
pub const IMPACT_FALL: f64 = 0.10;
/// `IMPACT_STAGGER`: the seconds between one line starting to fall and the next.
pub const IMPACT_STAGGER: f64 = 0.22;
/// `IMPACT_PUNCH`: the scale on landing, settling to 1 over [`IMPACT_PUNCH_T`].
pub const IMPACT_PUNCH: f64 = 1.10;
/// `IMPACT_PUNCH_T`: the seconds of the punch.
pub const IMPACT_PUNCH_T: f64 = 0.10;
/// `IMPACT_SHAKE`: the pixels of the shake on landing, decaying over [`IMPACT_SHAKE_T`].
pub const IMPACT_SHAKE: f64 = 2.0;
/// `IMPACT_SHAKE_T`: the seconds of the shake.
pub const IMPACT_SHAKE_T: f64 = 0.18;
/// `PUFF_T`: the seconds a sand puff lives.
pub const PUFF_T: f64 = 0.45;
/// `draw_captions`' default `puff`: the colour of the sand.
pub const PUFF: [u8; 3] = [236, 204, 150];
/// The multipliers of `_hash01`'s arguments.
pub const HASH_K: [f64; 4] = [12.9898, 78.233, 37.719, 4.581];

/// The outline under every caption letter: `_stamp_rgb`'s default `(10, 20, 26)`.
const OUTLINE: [f64; 3] = [10.0, 20.0, 26.0];

/// One caption: the lines shown from `t0` (inclusive) to `t1` (exclusive).
#[derive(Clone, Debug, PartialEq)]
pub struct Caption {
    pub t0: f64,
    pub t1: f64,
    pub lines: Vec<String>,
}

/// The keywords of `draw_captions` the `impact` style reads. `Default` is Python's: the top centre of the safe
/// box, [`super::CREAM`] text with [`super::AMBER`] and [`super::SKYB`] highlights, sand in [`PUFF`].
#[derive(Clone, Debug, PartialEq)]
pub struct CaptionStyle {
    /// `y0`: the top of the first line.
    pub y0: i32,
    /// `cx`: the column the lines are centred on.
    pub cx: i32,
    /// `base`: the colour of plain text.
    pub base: [u8; 3],
    /// `hi`: the colour of `*words*`.
    pub hi: [u8; 3],
    /// `alt`: the colour of `~words~`.
    pub alt: [u8; 3],
    /// `puff`: the colour of the sand; `None` for no sand.
    pub puff: Option<[u8; 3]>,
}

impl Default for CaptionStyle {
    fn default() -> Self {
        CaptionStyle {
            y0: CAP_Y0,
            cx: SAFE_CX,
            base: super::CREAM,
            hi: super::AMBER,
            alt: super::SKYB,
            puff: Some(PUFF),
        }
    }
}

/// One row of `impact_landings`: when a caption line lands, which line of its caption it is (from 0) and how many
/// characters it has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Landing {
    pub t: f64,
    pub line: usize,
    pub chars: usize,
}

/// `_plain(line)`: the line without its `*` and `~` marks.
fn plain(line: &str) -> String {
    line.chars().filter(|&c| c != '*' && c != '~').collect()
}

/// `caption_scale(lines)`: 3 (big) if every line has at most [`CAP_BIG_MAX`] characters without its marks, else 2
/// (small). Characters are Python's (Unicode code points). Without lines Python raises; this returns 3.
pub fn caption_scale<S: AsRef<str>>(lines: &[S]) -> u32 {
    let longest = lines
        .iter()
        .map(|l| plain(l.as_ref()).chars().count())
        .max()
        .unwrap_or(0);
    if longest <= CAP_BIG_MAX { 3 } else { 2 }
}

/// `impact_landings(caps, stagger=IMPACT_STAGGER, fall=IMPACT_FALL)`: every caption line that lands before its
/// caption ends, in the order of `caps` (the soundtrack puts a soft thud on each).
pub fn impact_landings(caps: &[Caption], stagger: f64, fall: f64) -> Vec<Landing> {
    let mut out = Vec::new();
    for cap in caps {
        for (k, line) in cap.lines.iter().enumerate() {
            let tl = cap.t0 + k as f64 * stagger + fall;
            if tl < cap.t1 {
                out.push(Landing {
                    t: tl,
                    line: k,
                    chars: plain(line).chars().count(),
                });
            }
        }
    }
    out
}

/// `_line_ink(line, scale, ...)`: the line's mask at an integer scale and, for every column of it, which colour
/// its ink takes: 0 base, 1 highlight, 2 second highlight. (Python builds a whole RGB array; it is constant down
/// each column.)
fn line_ink(line: &str, scale: u32) -> (Img<u8>, Vec<u8>) {
    let (mut text, mut modes) = (String::new(), Vec::new());
    let mut mode = 0u8;
    for ch in line.chars() {
        match ch {
            '*' => mode = if mode == 1 { 0 } else { 1 },
            '~' => mode = if mode == 2 { 0 } else { 2 },
            _ => {
                text.push(ch);
                modes.push(mode);
            }
        }
    }
    let m = font::text_mask(&text, scale);
    // ci = np.repeat(cols, ADV * scale)[:width]; a mask wider than that (only the 1 px mask of an empty line) is
    // padded with the last value, 0 if there is none.
    let step = (ADV as usize * scale as usize).max(1);
    let last = modes.last().copied().unwrap_or(0);
    let cols = (0..m.w).map(|x| modes.get(x / step).copied().unwrap_or(last)).collect();
    (m, cols)
}

/// `_resample`'s index list for one axis: `np.minimum((np.arange(n) * size / n).astype(int), size - 1)`, the
/// source index of each of the `n` new rows or columns (nearest neighbour, by truncation).
fn resample_index(n: usize, size: usize) -> Vec<usize> {
    (0..n)
        .map(|i| {
            let src = np::f64_to_i64((i * size) as f64 / n as f64).max(0) as usize;
            src.min(size.saturating_sub(1))
        })
        .collect()
}

/// `_resample(mask, hh, ww)` and `_resample(rgb, hh, ww)`: the mask and its column colours at another size.
fn resample(m: &Img<u8>, cols: &[u8], hh: usize, ww: usize) -> (Img<u8>, Vec<u8>) {
    let (ys, xs) = (resample_index(hh, m.h), resample_index(ww, m.w));
    let mut out: Img<u8> = Img::new(ww, hh, 1);
    for (y, &sy) in ys.iter().enumerate() {
        let src = m.row(sy);
        for (o, &sx) in out.row_mut(y).iter_mut().zip(&xs) {
            *o = src[sx];
        }
    }
    (out, xs.iter().map(|&sx| cols[sx]).collect())
}

/// One layer of `_stamp_rgb`: the set pixels of `m`, placed with its top left at `(mx, my)`, take the colour
/// `colour(column of m)`: replaced when `alpha >= 0.999`, else mixed in. Pixels outside the frame are dropped.
fn stamp_layer(frame: &mut Rgb8, m: &Img<u8>, mx: i32, my: i32, alpha: f64, colour: impl Fn(usize) -> [f64; 3]) {
    let (fw, fh) = (frame.w as i64, frame.h as i64);
    let (mx, my) = (i64::from(mx), i64::from(my));
    let (x0, y0) = (mx.max(0), my.max(0));
    let (x1, y1) = (fw.min(mx + m.w as i64), fh.min(my + m.h as i64));
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    for y in y0..y1 {
        let mrow = m.row((y - my) as usize);
        for x in x0..x1 {
            let mcol = (x - mx) as usize;
            if mrow[mcol] == 0 {
                continue;
            }
            let c = colour(mcol);
            let px = frame.px_mut(x as usize, y as usize);
            for (p, c) in px.iter_mut().zip(c) {
                *p = if alpha >= 0.999 {
                    // reg[sub] = np.clip(c, 0, 255).astype(np.uint8)
                    np::clip_u8(c)
                } else {
                    // reg[sub] = np.clip(reg[sub] * (1 - alpha) + c * alpha, 0, 255).astype(np.uint8)
                    np::clip_u8(f64::from(*p) * (1.0 - alpha) + c * alpha)
                };
            }
        }
    }
}

/// `_stamp_rgb(frame, mask, rgb, x, y, alpha=alpha)`: like [`font::stamp`], but each ink pixel has its own colour
/// (`pal[cols[column]]`): the outline's drop shadow, the 1 px outline, then the ink.
fn stamp_rgb(frame: &mut Rgb8, mask: &Img<u8>, cols: &[u8], pal: &[[f64; 3]; 3], x: i32, y: i32, alpha: f64) {
    let o = font::dilate(mask, 1);
    stamp_layer(frame, &o, x - 1, y + 1, alpha, |_| OUTLINE);
    stamp_layer(frame, &o, x - 1, y - 1, alpha, |_| OUTLINE);
    stamp_layer(frame, mask, x, y, alpha, |mcol| pal[usize::from(cols[mcol])]);
}

/// The number `_hash01` takes the sine of: `sum(x * k for x, k in zip(xs, HASH_K)) + 0.5`. Arguments past the
/// fourth are not used, as in Python.
///
/// Python's `sum()` does not add floats left to right: from Python 3.12 it carries a correction term (Neumaier's
/// compensated sum) and adds it at the end, which changes the last bit of this sum for about one argument triple
/// in five. This is that algorithm, as `builtin_sum` of CPython 3.13 runs it: the first product is added to the
/// integer 0, the others with the correction.
pub fn hash01_phase(xs: &[i64]) -> f64 {
    let mut items = xs.iter().zip(HASH_K).map(|(&x, k)| x as f64 * k);
    let Some(first) = items.next() else {
        return 0.5; // sum of nothing is the integer 0
    };
    // the integer 0 plus the first product (which makes a -0.0 a 0.0)
    let mut total = 0.0 + first;
    let mut carry = 0.0;
    for x in items {
        let t = total + x;
        if total.abs() >= x.abs() {
            carry += (total - t) + x;
        } else {
            carry += (x - t) + total;
        }
        total = t;
    }
    if carry != 0.0 && carry.is_finite() {
        total += carry;
    }
    total + 0.5
}

/// `_hash01(*xs)`: a number from 0 to 1 that looks random and depends only on its arguments: the fraction of
/// `sin(phase) * 43758.5453` with [`hash01_phase`]. The sand puff calls it as `_hash01(key, grain, draw)`.
///
/// Python's sine here is the C runtime's of the PC it runs on, the Studio's is `bf_core::dmath::sin`; where the
/// two differ in the last bit this differs from Python by up to 1.2e-11. On the reference vectors 67 of 5,134
/// values differ, by 7.3e-12 at most (`tests/kit_ui_vectors.rs` prints the count).
pub fn hash01(xs: &[i64]) -> f64 {
    let v = dmath::sin(hash01_phase(xs)) * 43758.5453;
    v - v.floor()
}

/// `_sand_puff(img, t, tl, x0, x1, ybot, col, key)`: the puff of pixel sand kicked out from under a line that
/// landed at `tl`, at time `t`: `12 + (x1 - x0) / 7` grains start along the line's foot (`x0..x1` at row
/// `ybot + 1`), fly outwards and up, fall back and fade over [`PUFF_T`] seconds. Each grain is one pixel, or a
/// 2 x 2 block for the heavier ones in the first quarter second, mixed into the picture; grains outside the safe
/// box are not drawn. `key` makes the puff the same on every frame; `col` `None` draws nothing.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn sand_puff(img: &mut Rgb8, t: f64, tl: f64, x0: i32, x1: i32, ybot: i32, col: Option<[u8; 3]>, key: i64) {
    assert_eq!(
        img.c, 3,
        "sand_puff wants an RGB picture, this one has {} channels",
        img.c
    );
    let u = t - tl;
    let Some(col) = col else {
        return;
    };
    if !(0.0..PUFF_T).contains(&u) {
        return; // also a NaN
    }
    let (x0, x1) = (i64::from(x0), i64::from(x1));
    let span = (x1 - x0) as f64;
    let n = 12 + np::f64_to_i64(span / 7.0);
    let fade = 1.0 - u / PUFF_T;
    let middle = (x0 + x1) as f64 / 2.0;
    for k in 0..n {
        let [r1, r2, r3, r4] = [1, 2, 3, 4].map(|j| hash01(&[key, k, j]));
        let px0 = x0 as f64 + span * r1;
        let side = if px0 < middle { -1.0 } else { 1.0 };
        let vx = side * (14.0 + 46.0 * r2);
        let vy = -(10.0 + 34.0 * r3);
        let px = px0 + vx * u;
        let py = f64::from(ybot + 1) + vy * u + 150.0 * u * u;
        let (xi, yi) = (round_i32(px), round_i32(py));
        if !(SAFE.0 <= xi && xi < SAFE.2 - 1 && SAFE.1 <= yi && yi < SAFE.3 - 1) {
            continue;
        }
        let a = py_min(1.0, fade * (0.75 + 0.45 * r4));
        let gain = 0.85 + 0.25 * r4;
        let sz = if r4 > 0.55 && u < 0.25 { 2 } else { 1 };
        // reg = img[yi:yi + sz, xi:xi + sz]; reg[:] = np.clip(reg * (1 - a) + c * a, 0, 255).astype(np.uint8)
        let (xi, yi) = (xi as usize, yi as usize);
        for y in yi..(yi + sz).min(img.h) {
            for x in xi..(xi + sz).min(img.w) {
                for (p, &c) in img.px_mut(x, y).iter_mut().zip(&col) {
                    *p = np::clip_u8(f64::from(*p) * (1.0 - a) + f64::from(c) * gain * a);
                }
            }
        }
    }
}

/// `draw_captions(img, t, caps, style='impact', ...)`: draws the captions that are open at `t` into the picture
/// and notes every line drawn as a caption (its box as drawn: punched and shaken).
///
/// A line starts to fall `k * `[`IMPACT_STAGGER`] seconds after its caption opens (`k` its index) and is not
/// drawn before; it falls [`IMPACT_DROP`] pixels in [`IMPACT_FALL`] seconds, fading in, lands, and for a moment
/// is drawn up to 10 % larger (never wider than the safe box less 4 px) and shaken by up to 2 px. The whole
/// caption fades out over its last 0.12 s. Every line is as big as the caption's longest line allows
/// ([`caption_scale`]). No line is drawn above the top of the safe box. A caption without lines draws nothing
/// (Python raises).
///
/// Only the `impact` style is ported: the fight uses no other.
pub fn draw_captions(img: &mut Rgb8, log: &mut DrawLog, t: f64, caps: &[Caption], style: &CaptionStyle) {
    assert_eq!(
        img.c, 3,
        "draw_captions wants an RGB picture, this one has {} channels",
        img.c
    );
    let pal = [style.base, style.hi, style.alt].map(|c| c.map(f64::from));
    for cap in caps {
        if !(cap.t0 <= t && t < cap.t1) || cap.lines.is_empty() {
            continue;
        }
        let fade = 1.0 - ss(cap.t1 - 0.12, cap.t1, t);
        let scale = caption_scale(&cap.lines);
        let lh = 7 * scale as i32;
        let mut y = style.y0;
        for (k, line) in cap.lines.iter().enumerate() {
            let pl = plain(line);
            let ts = cap.t0 + k as f64 * IMPACT_STAGGER; // this line starts falling
            let tl = ts + IMPACT_FALL; // ... and lands
            if t >= ts {
                let (mut m, mut cols) = line_ink(line, scale);
                let (h, w) = (m.h, m.w);
                let (dy, a, p, sx, sy);
                if t < tl {
                    // falling: an accelerating drop, fading in
                    let u = (t - ts) / IMPACT_FALL;
                    dy = -IMPACT_DROP * (1.0 - u * u);
                    a = py_min(1.0, 0.25 + u) * fade;
                    (p, sx, sy) = (1.0, 0, 0);
                } else {
                    let u = t - tl;
                    (dy, a) = (0.0, fade);
                    let settle = py_max(0.0, 1.0 - u / IMPACT_PUNCH_T);
                    let punch = 1.0 + (IMPACT_PUNCH - 1.0) * (settle * settle);
                    // the punch never leaves the safe box
                    let widest = f64::from(SAFE.2 - SAFE.0 - 4) / w.max(1) as f64;
                    p = py_max(1.0, py_min(punch, widest));
                    let amp = IMPACT_SHAKE * py_max(0.0, 1.0 - u / IMPACT_SHAKE_T);
                    let k = k as f64;
                    sx = round_i32(amp * dmath::sin(u * 2.0 * dmath::PI * 31.0 + k));
                    sy = round_i32(amp * 0.6 * dmath::sin(u * 2.0 * dmath::PI * 23.0 + 1.7 + k));
                }
                let (mut hh, mut ww) = (h, w);
                if p > 1.0 {
                    hh = round_i32(h as f64 * p).max(0) as usize;
                    ww = round_i32(w as f64 * p).max(0) as usize;
                    (m, cols) = resample(&m, &cols, hh, ww);
                }
                let x = round_i32(f64::from(style.cx) - ww as f64 / 2.0) + sx;
                let yy = round_i32(f64::from(y) + f64::from(lh) / 2.0 - hh as f64 / 2.0 + dy) + sy;
                let yy = yy.max(SAFE.1);
                stamp_rgb(img, &m, &cols, &pal, x, yy, a);
                log.note(x, yy, x + ww as i32, yy + hh as i32, Kind::Caption, &pl);
                // the sand under the line as it stands when settled, keyed by its text, its place and its caption
                let chars = pl.chars().count() as i32;
                let w0 = chars * ADV * scale as i32 - scale as i32;
                let xl = round_i32(f64::from(style.cx) - f64::from(w0) / 2.0);
                let key = py::imod_py(pyhash::str_hash(&pl), 997) + 13 * k as i64 + np::f64_to_i64(cap.t0 * 10.0);
                sand_puff(img, t, tl, xl, xl + w0, y + lh, style.puff, key);
            }
            y += lh + 7;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(t0: f64, t1: f64, lines: &[&str]) -> Caption {
        Caption {
            t0,
            t1,
            lines: lines.iter().map(|l| l.to_string()).collect(),
        }
    }

    fn grey() -> Rgb8 {
        Img::filled(360, 240, &[128, 128, 128])
    }

    #[test]
    fn marks_are_not_characters() {
        assert_eq!(plain("*K.O.*"), "K.O.");
        assert_eq!(plain("~a~*b*c"), "abc");
        assert_eq!(caption_scale(&["FLUTTER TONGUE"]), 3);
        assert_eq!(caption_scale(&["*NEXT MATCH-UP*", "COMMENT THE"]), 3);
        assert_eq!(caption_scale(&["15 CHARS HERE!!"]), 2);
        // Characters, not bytes: fifteen two-byte letters are fifteen.
        assert_eq!(caption_scale(&["\u{e9}".repeat(14)]), 3);
        assert_eq!(caption_scale(&["\u{e9}".repeat(15)]), 2);
        assert_eq!(caption_scale::<&str>(&[]), 3);
    }

    #[test]
    fn ink_columns_follow_the_marks() {
        let (m, cols) = line_ink("a*b*~c~", 2);
        assert_eq!((m.w, m.h), (34, 14));
        assert_eq!(cols.len(), 34);
        assert!(cols[..12].iter().all(|&c| c == 0));
        assert!(cols[12..24].iter().all(|&c| c == 1));
        assert!(cols[24..].iter().all(|&c| c == 2));
        // An empty line is a mask one font pixel wide, in the base colour; an unclosed mark runs to the end.
        let (m, cols) = line_ink("**", 3);
        assert_eq!((m.w, m.h, cols), (3, 21, vec![0, 0, 0]));
        assert_eq!(line_ink("*ab", 2).1, vec![1; 22]);
    }

    #[test]
    fn resampling_is_nearest_by_truncation() {
        assert_eq!(resample_index(4, 2), [0, 0, 1, 1]);
        assert_eq!(resample_index(3, 2), [0, 0, 1]);
        assert_eq!(resample_index(5, 3), [0, 0, 1, 1, 2]);
        assert_eq!(resample_index(2, 5), [0, 2]);
        let m = Img::from_vec(2, 1, 1, vec![1u8, 0]);
        let (big, cols) = resample(&m, &[2, 1], 2, 3);
        assert_eq!((big.w, big.h), (3, 2));
        assert_eq!(big.data, [1, 1, 0, 1, 1, 0]);
        assert_eq!(cols, [2, 2, 1]);
    }

    #[test]
    fn the_hash_sums_as_python_does() {
        // Python: sum(x * k for x, k in zip((1, 17, 2), K)) + 0.5 is 1418.8888; added left to right the sum would
        // be 1418.3888000000002 and the phase one bit higher.
        assert_eq!(hash01_phase(&[1, 17, 2]), 1418.8888);
        let naive = 1.0 * HASH_K[0] + 17.0 * HASH_K[1] + 2.0 * HASH_K[2];
        assert_eq!(naive, 1418.3888000000002);
        assert_ne!(hash01_phase(&[1, 17, 2]), naive + 0.5);
        assert_eq!(hash01_phase(&[3, 17, 3]), 1482.5874000000001);
        assert_eq!(hash01_phase(&[]), 0.5);
        assert_eq!(hash01_phase(&[2]), 2.0 * 12.9898 + 0.5);
        // A fifth argument is not used.
        assert_eq!(hash01_phase(&[1, 2, 3, 4, 99]), hash01_phase(&[1, 2, 3, 4]));
        // Python: _hash01(191, 0, 1) is 0.6393859047038859, _hash01() is 0.9641489889145305, _hash01(3) is
        // 0.6999709724987042 (to the last bit of the sine).
        assert!((hash01(&[191, 0, 1]) - 0.6393859047038859).abs() < 1.25e-11);
        assert!((hash01(&[]) - 0.9641489889145305).abs() < 1.25e-11);
        assert!((hash01(&[3]) - 0.6999709724987042).abs() < 1.25e-11);
        for key in 0..200 {
            let r = hash01(&[key, 3, 2]);
            assert!((0.0..1.0).contains(&r), "{key}: {r}");
        }
    }

    #[test]
    fn landings_follow_the_stagger() {
        let caps = [cap(1.0, 3.0, &["A", "*B*c"]), cap(5.0, 5.05, &["TOO", "SHORT"])];
        let got = impact_landings(&caps, IMPACT_STAGGER, IMPACT_FALL);
        // Python: [(1.1, 0, 1), (1.32, 1, 2)]
        assert_eq!(
            got,
            [
                Landing {
                    t: 1.1,
                    line: 0,
                    chars: 1
                },
                Landing {
                    t: 1.32,
                    line: 1,
                    chars: 2
                }
            ]
        );
    }

    #[test]
    fn a_caption_whose_window_never_opens_is_never_drawn() {
        // The kit makes such a caption for a late K.O.: its end is at or before its start.
        for (t0, t1) in [(50.0, 50.0), (50.0, 49.2), (50.0, f64::NAN)] {
            let caps = [cap(t0, t1, &["*K.O.*"])];
            for t in [49.0, 49.2, 49.9, 50.0, 50.05, 50.1, 50.3, 51.0] {
                let (mut img, mut log) = (grey(), DrawLog::new());
                draw_captions(&mut img, &mut log, t, &caps, &CaptionStyle::default());
                assert_eq!(img, grey(), "window {t0}..{t1} at {t}");
                assert!(log.boxes.is_empty());
            }
            assert!(impact_landings(&caps, IMPACT_STAGGER, IMPACT_FALL).is_empty());
        }
        // A caption without lines draws nothing either.
        let (mut img, mut log) = (grey(), DrawLog::new());
        draw_captions(&mut img, &mut log, 1.5, &[cap(1.0, 2.0, &[])], &CaptionStyle::default());
        assert_eq!(img, grey());
        assert!(log.boxes.is_empty());
    }

    #[test]
    fn a_settled_line_sits_at_the_top_of_the_safe_box() {
        let caps = [cap(10.0, 12.4, &["*K.O.*", "ok"])];
        let (mut img, mut log) = (grey(), DrawLog::new());
        draw_captions(&mut img, &mut log, 11.0, &caps, &CaptionStyle::default());
        // Scale 3: K.O. is (4 * 6 - 1) * 3 = 69 wide and 21 high, centred on column 180; the next line 28 lower.
        let boxes: Vec<_> = log
            .boxes
            .iter()
            .map(|b| (b.x0, b.y0, b.x1, b.y1, b.kind, b.info.as_str()))
            .collect();
        assert_eq!(
            boxes,
            [
                (146, 100, 215, 121, Kind::Caption, "K.O."),
                (164, 128, 197, 149, Kind::Caption, "ok")
            ]
        );
        // The K's stem in the highlight colour, the O's left side (its second font row) in the base colour, the
        // outline around both.
        assert_eq!(img.px(146, 100), &super::super::AMBER);
        assert_eq!(img.px(164, 131), &super::super::CREAM);
        assert_eq!(img.px(145, 100), &[10, 20, 26]);
        // Before the second line starts to fall only the first is drawn.
        let (mut img, mut log) = (grey(), DrawLog::new());
        draw_captions(&mut img, &mut log, 10.2, &caps, &CaptionStyle::default());
        assert_eq!(log.boxes.len(), 1);
        assert_eq!(img.px(164, 131), &[128, 128, 128]);
    }

    #[test]
    fn sand_stays_in_the_safe_box_and_ends() {
        let before = grey();
        let mut img = grey();
        sand_puff(&mut img, 5.1, 5.0, 60, 300, 121, Some(PUFF), 77);
        assert_ne!(img, before);
        for y in 0..img.h {
            for x in 0..img.w {
                if img.px(x, y) != before.px(x, y) {
                    assert!((20..340).contains(&x) && (96..500).contains(&y), "({x}, {y})");
                }
            }
        }
        for (t, col) in [
            (5.45, Some(PUFF)),
            (4.99, Some(PUFF)),
            (5.1, None),
            (f64::NAN, Some(PUFF)),
        ] {
            let mut img = grey();
            sand_puff(&mut img, t, 5.0, 60, 300, 121, col, 77);
            assert_eq!(img, before, "{t} {col:?}");
        }
    }
}
