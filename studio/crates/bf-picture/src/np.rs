//! numpy's number semantics, for the expressions the renderer is made of.
//!
//! The kit computes its pictures with numpy 2.5.3. Each function here does to one element what numpy does to every
//! element of an array, with the same result on the reference PC; `studio/reference/picture_kit/np.npz` (written by
//! `tools/studio_export/kit_vectors.py`) checks them bit for bit.
//!
//! What a port has to keep in mind besides these helpers:
//!
//! - **dtypes.** A numpy expression keeps `float32` as long as every array in it is `float32` (a Python scalar
//!   does not widen it); one `float64` array widens the rest. Do each step in the type numpy would use
//!   (`f32` arithmetic for `float32`), then convert.
//! - **Casts truncate.** `a.astype(np.uint8)` drops the fraction (toward zero) and does not clamp: use
//!   [`f64_to_u8`]. The kit usually clips first (`np.clip(a, 0, 255).astype(np.uint8)`: [`clip_u8`]).
//! - **`np.dot`, `@` and `np.linalg.norm` are not plain loops.** They go through OpenBLAS, whose kernels use
//!   fused multiply-add on the reference PC: `v.dot(v)` for three elements is `fma(c, c, fma(b, b, a * a))`, which
//!   differs from `a * a + b * b + c * c` in the last bit about once in ten. Nothing here covers that; a port of
//!   such an expression matches to rounding, not to the bit.

use crate::img::Img;

pub(crate) mod index;

/// `np.hypot(x, y)` for `float64`: numpy's `hypot` is the C runtime's, which on the reference PC is the plain
/// `sqrt(x * x + y * y)` (equal on 100,000 of 100,000 random pairs). Python's `math.hypot` is another, more careful
/// algorithm (as is `bf_core::dmath::hypot`) and differs from numpy's in the last bit for about 17 % of pairs: a
/// port of `np.hypot` whose result meets a threshold takes this one.
pub fn hypot(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

/// `np.clip(x, lo, hi)` with both bounds given, for `float64`: numpy's loop is
/// `if (x < lo) x = lo; if (x > hi) x = hi;`. So a NaN `x` stays NaN, and an `x` equal to a bound stays itself
/// (`clip(-0.0, 0.0, 1.0)` is `-0.0`). If a bound is NaN the result is that NaN, as numpy gives.
pub fn clip(x: f64, lo: f64, hi: f64) -> f64 {
    if lo.is_nan() {
        return lo;
    }
    if hi.is_nan() {
        return hi;
    }
    let mut x = x;
    if x < lo {
        x = lo;
    }
    if x > hi {
        x = hi;
    }
    x
}

/// [`clip`] for `float32` arrays (`np.clip(a32, lo, hi)` stays `float32`; the bounds are converted to `float32`
/// first, as numpy converts Python scalars).
pub fn clip_f32(x: f32, lo: f32, hi: f32) -> f32 {
    if lo.is_nan() {
        return lo;
    }
    if hi.is_nan() {
        return hi;
    }
    let mut x = x;
    if x < lo {
        x = lo;
    }
    if x > hi {
        x = hi;
    }
    x
}

/// `np.maximum(a, b)`, which is also `np.clip(a, b, None)`: the larger value; NaN if either is NaN.
///
/// One thing is not pinned down: for `-0.0` against `0.0` numpy answers with `b` in the vectorised part of an
/// array and with `a` in the last few elements (its scalar tail), so the sign of a zero result depends on where
/// the element sits. This returns `b`, as the vectorised part does. No picture depends on the sign of a zero.
pub fn maximum(a: f64, b: f64) -> f64 {
    if a.is_nan() || a > b { a } else { b }
}

/// `np.minimum(a, b)`, which is also `np.clip(a, None, b)`: the smaller value; NaN if either is NaN. Zeros of
/// different sign: see [`maximum`].
pub fn minimum(a: f64, b: f64) -> f64 {
    if a.is_nan() || a < b { a } else { b }
}

/// [`maximum`] for `float32`. `np.maximum(a32, x)` with a Python float `x` stays `float32`: convert `x` to `f32`
/// first, as numpy does.
pub fn maximum_f32(a: f32, b: f32) -> f32 {
    if a.is_nan() || a > b { a } else { b }
}

/// `a.astype(np.int32)` for one `float64`: the fraction is dropped (toward zero). NaN, the infinities and every
/// value whose whole part does not fit give `i32::MIN`: numpy leaves such casts to the processor, and that is what
/// the x86 conversion (`cvttsd2si`) returns. C's `(int)x` in Pillow is the same instruction.
pub fn f64_to_i32(x: f64) -> i32 {
    if x > -2_147_483_649.0 && x < 2_147_483_648.0 {
        x as i32
    } else {
        i32::MIN
    }
}

/// `a.astype(np.int64)` (also `a.astype(int)`) for one `float64`: toward zero; NaN, the infinities and values
/// outside the type give `i64::MIN`, as on x86.
pub fn f64_to_i64(x: f64) -> i64 {
    if (-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&x) {
        x as i64
    } else {
        i64::MIN
    }
}

/// `a.astype(np.uint8)` for one `float64`: the fraction is dropped, then the low 8 bits of the 32-bit whole number
/// are kept, so the value wraps instead of clamping (`-1.5` is 255, `256.7` is 0, `300.5` is 44). NaN, the
/// infinities and values beyond 32 bits give 0. This is what numpy does on the reference PC; the kit only relies
/// on it between 0 and 255.
pub fn f64_to_u8(x: f64) -> u8 {
    f64_to_i32(x) as u8
}

/// `a.astype(np.uint8)` for one `float32`: as [`f64_to_u8`] (widening a `float32` loses nothing).
pub fn f32_to_u8(x: f32) -> u8 {
    f64_to_u8(f64::from(x))
}

/// `np.clip(x, 0, 255).astype(np.uint8)` for one `float64`: the kit's usual way from a float picture to bytes.
/// NaN becomes 0 (the clip keeps the NaN, the cast turns it into 0).
pub fn clip_u8(x: f64) -> u8 {
    f64_to_u8(clip(x, 0.0, 255.0))
}

/// `np.clip(x, 0, 255).astype(np.uint8)` for one `float32`.
pub fn clip_u8_f32(x: f32) -> u8 {
    f32_to_u8(clip_f32(x, 0.0, 255.0))
}

/// The kit's banding dither `np.floor(v * levels + bayer * gain) / levels`: `v` cut into `levels` steps, with the
/// Bayer value (0 to 15/16) deciding which pixels take the next step. `gain` is the kit's 0.999 or 0.9. All
/// `float64`; if `v` is `float32` in the Python, convert it only where numpy would (when it meets the `float64`
/// Bayer array, that is after `v * levels`).
pub fn floor_dither(v: f64, levels: f64, bayer: f64, gain: f64) -> f64 {
    (v * levels + bayer * gain).floor() / levels
}

/// The kit's quantising dither `np.floor(x / step + bayer) * step`: `x` snapped down to multiples of `step`, the
/// Bayer value deciding which pixels take the next multiple. Where the Python has `BAYER_FULL * 0.999`, pass
/// `bayer * 0.999`. All `float64` (see [`floor_dither`] about `float32` pictures: there `x / step` is a `float32`
/// division).
pub fn floor_dither_step(x: f64, step: f64, bayer: f64) -> f64 {
    (x / step + bayer).floor() * step
}

/// `np.roll(a, shift, axis)` for an image: axis 0 moves rows down by `shift` (up if negative), axis 1 moves
/// columns right; what leaves one side comes back on the other. Pixels move whole (all channels together).
/// Panics on any other axis.
pub fn roll<T: Clone>(a: &Img<T>, shift: i64, axis: usize) -> Img<T> {
    assert!(axis < 2, "roll: axis {axis} (0 is rows, 1 is columns)");
    let (w, h, c) = (a.w, a.h, a.c);
    let mut data = Vec::with_capacity(a.data.len());
    let n = if axis == 0 { h } else { w };
    if n == 0 {
        return a.clone();
    }
    // Element i of the result is element (i - shift) mod n of the source.
    let back = shift.rem_euclid(n as i64) as usize;
    let src = |i: usize| (i + n - back) % n;
    if axis == 0 {
        for y in 0..h {
            data.extend_from_slice(a.row(src(y)));
        }
    } else {
        for y in 0..h {
            let row = a.row(y);
            for x in 0..w {
                let s = src(x) * c;
                data.extend_from_slice(&row[s..s + c]);
            }
        }
    }
    Img { w, h, c, data }
}

/// `np.roll(a, shift)` for a 1-D array.
pub fn roll_1d<T: Clone>(a: &[T], shift: i64) -> Vec<T> {
    let n = a.len();
    if n == 0 {
        return Vec::new();
    }
    let back = shift.rem_euclid(n as i64) as usize;
    (0..n).map(|i| a[(i + n - back) % n].clone()).collect()
}

/// `np.nonzero(m)` for a 2-D mask (a one-channel image): the `(y, x)` of every non-zero pixel, row by row from
/// the top and left to right within a row. numpy returns the two as separate arrays `(ys, xs)`, in this order.
pub fn nonzero(m: &Img<u8>) -> Vec<(usize, usize)> {
    assert_eq!(
        m.c, 1,
        "nonzero wants a one-channel mask, this image has {} channels",
        m.c
    );
    let mut out = Vec::new();
    for y in 0..m.h {
        for (x, &v) in m.row(y).iter().enumerate() {
            if v != 0 {
                out.push((y, x));
            }
        }
    }
    out
}

/// `np.nonzero(v)[0]` for a 1-D mask: the indices of the true elements, ascending.
pub fn nonzero_1d(v: &[bool]) -> Vec<usize> {
    v.iter().enumerate().filter_map(|(i, &on)| on.then_some(i)).collect()
}

/// `a.sum()` (`np.add.reduce`) for a 1-D `float64` array, in numpy's order, which is not left to right: fewer
/// than 8 elements are added in order; up to 128 go into eight running sums (element `i` into sum `i % 8`) that
/// are then added as `((r0 + r1) + (r2 + r3)) + ((r4 + r5) + (r6 + r7))`, with the last `n % 8` elements added
/// one by one after that; a longer array is cut in two (the first half rounded down to a multiple of 8) and the
/// halves are summed this way and added. (`pairwise_sum` in numpy's `loops_utils.h.src`; checked against numpy
/// for every length from 1 to 299.)
///
/// This is the sum along the array's only axis. A sum down the rows of a 2-D array (`a.sum(0)`, `a.mean(0)`)
/// adds row after row, in order.
pub fn sum(a: &[f64]) -> f64 {
    // The reduction starts from add's identity.
    0.0 + pairwise_sum(a)
}

fn pairwise_sum(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 8 {
        let mut res = 0.0;
        for &v in a {
            res += v;
        }
        res
    } else if n <= 128 {
        let mut r = [0.0; 8];
        r.copy_from_slice(&a[..8]);
        let whole = n - n % 8;
        for block in a[8..whole].as_chunks::<8>().0 {
            for (acc, &v) in r.iter_mut().zip(block) {
                *acc += v;
            }
        }
        let mut res = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
        for &v in &a[whole..] {
            res += v;
        }
        res
    } else {
        let mut half = n / 2;
        half -= half % 8;
        pairwise_sum(&a[..half]) + pairwise_sum(&a[half..])
    }
}

/// `a.mean()` for a 1-D `float64` array: [`sum`] divided by the number of elements (NaN for an empty array).
pub fn mean(a: &[f64]) -> f64 {
    sum(a) / a.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use bf_testdata::parse_hex_f64;

    #[test]
    fn hypot_is_numpys() {
        assert_eq!(hypot(3.0, 4.0), 5.0);
        assert_eq!(hypot(-0.0, 0.0), 0.0);
        // Python: np.hypot(x, y) and math.hypot(x, y) for two pairs where they differ (numpy's is the first)
        for (x, y, np_hypot, math_hypot) in [
            (
                "-0x1.69489ace217b4p+3",
                "-0x1.266d2ae7f6f18p+2",
                "0x1.86200567116c7p+3",
                "0x1.86200567116c8p+3",
            ),
            (
                "-0x1.79ddad711213cp+3",
                "-0x1.6522de9444730p+1",
                "0x1.84455a822ef09p+3",
                "0x1.84455a822ef0ap+3",
            ),
        ] {
            let (x, y) = (parse_hex_f64(x), parse_hex_f64(y));
            assert_eq!(hypot(x, y).to_bits(), parse_hex_f64(np_hypot).to_bits());
            assert_ne!(hypot(x, y).to_bits(), parse_hex_f64(math_hypot).to_bits());
        }
    }

    #[test]
    fn roll_moves_rows_and_columns() {
        // [[0, 1, 2], [3, 4, 5]]
        let a = Img::from_vec(3, 2, 1, vec![0u8, 1, 2, 3, 4, 5]);
        assert_eq!(roll(&a, 1, 0).data, [3, 4, 5, 0, 1, 2]);
        assert_eq!(roll(&a, 1, 1).data, [2, 0, 1, 5, 3, 4]);
        assert_eq!(roll(&a, -1, 1).data, [1, 2, 0, 4, 5, 3]);
        assert_eq!(roll(&a, 3, 1).data, a.data);
        assert_eq!(roll(&a, -7, 1).data, roll(&a, 2, 1).data);
        assert_eq!(roll_1d(&[1, 2, 3, 4], 1), [4, 1, 2, 3]);
        assert_eq!(roll_1d(&[1, 2, 3, 4], -5), [2, 3, 4, 1]);
        assert!(roll_1d::<u8>(&[], 3).is_empty());
    }

    #[test]
    fn sums_follow_numpys_order() {
        // a[i] = (-1000 if i % 3 == 0 else 1) / (i + 1) + i * 1e-7, and what numpy 2.5.3 gives for it.
        let a = |n: usize| -> Vec<f64> {
            (0..n)
                .map(|i| (if i % 3 == 0 { -1000.0 } else { 1.0 }) / (i as f64 + 1.0) + i as f64 * 1e-7)
                .collect()
        };
        for (n, total, average) in [
            (1, 0xc08f400000000000, 0xc08f400000000000),
            (5, 0xc09383dddd9ac200, 0xc06f3962fc2ad000),
            (8, 0xc095be20e952b948, 0xc065be20e952b948),
            (21, 0xc09a4bce29aab8fc, 0xc05408fe99a6a552),
            (128, 0xc0a1eaa3f1239ab6, 0xc031eaa3f1239ab6),
            (129, 0xc0a1ea9ff7663b4e, 0xc031c711d3bebdd2),
            (300, 0xc0a41d9c05425b58, 0xc0212a5559d23977),
        ] {
            assert_eq!(sum(&a(n)).to_bits(), total, "sum of {n}");
            assert_eq!(mean(&a(n)).to_bits(), average, "mean of {n}");
        }
        assert_eq!(sum(&[]), 0.0);
        assert!(mean(&[]).is_nan());
        // Left to right would give another last bit for the long ones.
        let plain: f64 = a(300).iter().sum();
        assert_ne!(plain.to_bits(), sum(&a(300)).to_bits());
    }

    #[test]
    fn nonzero_is_row_major() {
        let m = Img::from_vec(3, 2, 1, vec![0u8, 1, 1, 9, 0, 1]);
        assert_eq!(nonzero(&m), [(0, 1), (0, 2), (1, 0), (1, 2)]);
        assert_eq!(nonzero_1d(&[false, true, true, false]), [1, 2]);
    }

    #[test]
    fn casts_truncate_and_wrap() {
        assert_eq!(f64_to_u8(255.99), 255);
        assert_eq!(f64_to_u8(-0.7), 0);
        assert_eq!(f64_to_u8(-1.5), 255);
        assert_eq!(f64_to_u8(256.7), 0);
        assert_eq!(f64_to_u8(300.5), 44);
        assert_eq!(f64_to_u8(f64::NAN), 0);
        assert_eq!(f64_to_i32(2_147_483_647.9), i32::MAX);
        assert_eq!(f64_to_i32(2_147_483_648.0), i32::MIN);
        assert_eq!(f64_to_i32(-2_147_483_648.9), i32::MIN);
        assert_eq!(f64_to_i32(-2_147_483_649.0), i32::MIN);
        assert_eq!(f64_to_i32(f64::INFINITY), i32::MIN);
        assert_eq!(f64_to_i64(1e19), i64::MIN);
        assert_eq!(f64_to_i64(-9.2e18), -9_200_000_000_000_000_000);
        assert_eq!(clip_u8(300.0), 255);
        assert_eq!(clip_u8(-3.0), 0);
        assert_eq!(clip_u8(127.999), 127);
    }

    #[test]
    fn clip_keeps_nan_and_equal_zeros() {
        assert!(clip(f64::NAN, 0.0, 1.0).is_nan());
        assert!(clip(-0.0, 0.0, 1.0).is_sign_negative());
        assert!(clip(0.0, -0.0, 1.0).is_sign_positive());
        assert!(clip(0.5, f64::NAN, 1.0).is_nan());
        assert_eq!(clip(7.0, 0.0, 1.0), 1.0);
        assert_eq!(clip_f32(-7.0, 0.0, 1.0), 0.0);
        assert!(maximum(f64::NAN, 1.0).is_nan());
        assert!(maximum(1.0, f64::NAN).is_nan());
        assert!(minimum(1.0, f64::NAN).is_nan());
        assert_eq!((maximum(2.0, 3.0), minimum(2.0, 3.0)), (3.0, 2.0));
        assert_eq!((maximum_f32(2.0, 3.0), maximum_f32(3.0, 2.0)), (3.0, 3.0));
        assert!(maximum_f32(f32::NAN, 1.0).is_nan() && maximum_f32(1.0, f32::NAN).is_nan());
        // The floor of a float32 depth: the Python float 1e-3 as a float32.
        assert_eq!(maximum_f32(1e-9, 1e-3).to_bits(), 1e-3f32.to_bits());
    }
}
