//! numpy's slices and fancy indices on a picture, as the kit's drawing code uses them: a slice or an index that
//! leaves the picture does not fail in Python. A slice is cut to the picture, and a negative bound or index counts
//! from the far edge ([`py_slice`], [`py_index`]); only an index at or beyond the far edge, where numpy raises,
//! panics. Also the slice assignment of a colour ([`fill`]), the mix of a picture with an earlier copy of itself
//! ([`mix_over`]) and the nearest-neighbour resample by index lists ([`resample_index`], [`resample`]).

use std::ops::Range;

use super::{f64_to_i64, f64_to_u8};
use crate::img::{Img, Rgb8};

/// The indices Python's `a[lo:hi]` selects on an axis of `n` elements: a negative bound counts from the end, and
/// both bounds are then cut to the axis. Empty when the bounds cross.
pub(crate) fn py_slice(lo: i32, hi: i32, n: usize) -> Range<usize> {
    let n = n as i64;
    let bound = |b: i32| {
        let b = i64::from(b);
        (if b < 0 { b + n } else { b }).clamp(0, n) as usize
    };
    let (lo, hi) = (bound(lo), bound(hi));
    lo..hi.max(lo)
}

/// The element Python's `a[i]` reads on an axis of `n` elements: a negative index counts from the end. Panics
/// when the index is outside the axis, where numpy raises `IndexError`.
pub(crate) fn py_index(i: i32, n: usize) -> usize {
    let wrapped = if i < 0 { i64::from(i) + n as i64 } else { i64::from(i) };
    assert!(
        (0..n as i64).contains(&wrapped),
        "index {i} is out of bounds for an axis of {n} pixels"
    );
    wrapped as usize
}

/// `img[ys.0:ys.1, xs.0:xs.1] = col`: a filled rectangle, as numpy's slice assignment paints it.
pub(crate) fn fill(img: &mut Rgb8, ys: (i32, i32), xs: (i32, i32), col: [u8; 3]) {
    let cols = py_slice(xs.0, xs.1, img.w);
    for y in py_slice(ys.0, ys.1, img.h) {
        for x in cols.clone() {
            img.px_mut(x, y).copy_from_slice(&col);
        }
    }
}

/// `img[:] = (before * (1 - k) + img * k).astype(np.uint8)`: the whole picture mixed with an earlier copy of
/// itself, every pixel (also the ones that did not change: `v * (1 - k) + v * k` is not always `v`).
pub(crate) fn mix_over(img: &mut Rgb8, before: &Rgb8, k: f64) {
    let back = 1.0 - k;
    for (p, &b) in img.data.iter_mut().zip(&before.data) {
        *p = f64_to_u8(f64::from(b) * back + f64::from(*p) * k);
    }
}

/// The source index of each of `n` new rows or columns of an axis `size` long:
/// `np.minimum((np.arange(n) * size / n).astype(int), size - 1)`, nearest neighbour by truncation (`reelkit.ui`'s
/// `_resample` and `title.word_masks`).
pub(crate) fn resample_index(n: usize, size: usize) -> Vec<usize> {
    (0..n)
        .map(|i| {
            let src = f64_to_i64((i * size) as f64 / n as f64).max(0) as usize;
            src.min(size.saturating_sub(1))
        })
        .collect()
}

/// `_resample(mask, hh, ww)` and `_resample(rgb, hh, ww)` of `reelkit.ui`: a mask and its column colours at
/// another size, `m[ys][:, xs]` and `cols[xs]` with the index lists of [`resample_index`].
pub(crate) fn resample(m: &Img<u8>, cols: &[u8], hh: usize, ww: usize) -> (Img<u8>, Vec<u8>) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slices_count_from_the_end_and_are_cut() {
        assert_eq!(py_slice(2, 5, 10), 2..5);
        assert_eq!(py_slice(-3, 20, 10), 7..10);
        assert_eq!(py_slice(-30, 4, 10), 0..4);
        assert_eq!(py_slice(4, -2, 10), 4..8);
        assert!(py_slice(6, 3, 10).is_empty());
        assert!(py_slice(12, 15, 10).is_empty());
        assert!(py_slice(-2, 3, 10).is_empty());
        assert_eq!((py_index(3, 10), py_index(-1, 10), py_index(-10, 10)), (3, 9, 0));
    }

    #[test]
    #[should_panic(expected = "index 10 is out of bounds")]
    fn an_index_past_the_axis_panics() {
        py_index(10, 10);
    }

    #[test]
    fn fill_and_mix() {
        let mut img: Rgb8 = Img::filled(4, 3, &[100, 100, 100]);
        fill(&mut img, (1, 9), (-2, 4), [1, 2, 3]);
        assert_eq!(img.px(1, 1), &[100, 100, 100]);
        assert_eq!(img.px(2, 1), &[1, 2, 3]);
        assert_eq!(img.px(3, 2), &[1, 2, 3]);
        assert_eq!(img.px(3, 0), &[100, 100, 100]);
        let before: Rgb8 = Img::filled(4, 3, &[200, 200, 200]);
        mix_over(&mut img, &before, 0.25);
        // 200 * 0.75 + 100 * 0.25 = 175; 200 * 0.75 + 1 * 0.25 = 150.25
        assert_eq!(img.px(0, 0), &[175, 175, 175]);
        assert_eq!(img.px(2, 1), &[150, 150, 150]);
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
}
