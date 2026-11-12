//! `scipy.ndimage.gaussian_filter` as TRUMPET's look calls it (`gaussian_filter(mask, sigma)` on a `float32` mask,
//! `kit/fighters/trumpet.py` `draw_beam`): every default (`order=0`, `mode='reflect'`, `cval=0.0`,
//! `truncate=4.0`), the result in the input's dtype. Ported from scipy 1.18.1, `ndimage/_filters.py`
//! (`gaussian_filter`, `gaussian_filter1d`, `_gaussian_kernel1d`, `correlate1d`) and the C it calls,
//! `ndimage/src/ni_filters.c` `NI_Correlate1D` and `ni_support.c` `NI_ExtendLine` / the line buffers:
//!
//! - a sigma at or below `1e-15` (zero, a negative one, NaN) is a copy;
//! - otherwise one pass per axis, the first axis (rows) first, each written back into the input's dtype before
//!   the next pass reads it (a `float32` picture is rounded to `float32` between the two passes);
//! - the kernel is `exp(-0.5 / sigma ** 2 * x ** 2)` for `x` in `-r..=r`, `r = int(4.0 * sigma + 0.5)`, divided by
//!   its numpy (pairwise) sum;
//! - each line is copied into a `float64` buffer extended at both ends by `r` samples in the `reflect` way
//!   (`d c b a | a b c d | d c b a`, going on round the line when `r` is longer than it), and each output is
//!   `x[0] * w[0]` plus `(x[-j] + x[j]) * w[j]` for `j` from `r` down to 1, in `float64` (the symmetric branch:
//!   the kernel is exactly symmetric).
//!
//! The kernel's `exp` is `bf_core::dmath::exp`; numpy's gives the C library's bits, and the two differ in the last
//! bit now and then: of the twelve sigmas the vectors hold, at 2.4 (4 of 21 weights) and 6 (2 of 49). 2.4 is the
//! sigma of TRUMPET's halo, a literal of the kit (`trumpet.py` `draw_beam`), so its kernel is a constant of the
//! kit and is held here as scipy's exact weights ([`KIT_KERNELS`], as `title::layout::DIVIDER_DEG` holds the
//! kit's angle); every other sigma computes its kernel. The correlation itself is scipy's to the bit: with
//! scipy's weights every value of every case is equal.

use bf_core::dmath;

use crate::img::Img;
use crate::np;

/// `truncate=4.0`: the kernel reaches this many sigmas each way.
pub const TRUNCATE: f64 = 4.0;

/// A sample type `gaussian_filter` keeps: `float32` or `float64`, read into and written back from the `float64`
/// line buffer as C casts them.
pub trait Sample: Copy {
    fn to_f64(self) -> f64;
    fn from_f64(x: f64) -> Self;
}

impl Sample for f32 {
    fn to_f64(self) -> f64 {
        f64::from(self)
    }
    fn from_f64(x: f64) -> f32 {
        x as f32
    }
}

impl Sample for f64 {
    fn to_f64(self) -> f64 {
        self
    }
    fn from_f64(x: f64) -> f64 {
        x
    }
}

/// `int(truncate * float(sigma) + 0.5)`: the kernel's radius.
///
/// # Panics
/// Where Python raises: the radius is negative (`Radius must be a nonnegative integer.`) or not a number.
pub fn radius(sigma: f64, truncate: f64) -> usize {
    let lw = truncate * sigma + 0.5;
    assert!(lw.is_finite(), "cannot convert float {lw} to integer");
    // int() cuts toward zero, so -0.5 < lw < 1 is radius 0
    assert!(lw > -1.0, "Radius must be a nonnegative integer.");
    lw as usize
}

/// The half kernels (centre last) scipy makes for the sigmas a kit look passes where `dmath::exp` would give a
/// weight one bit off: sigma 2.4, radius 10 (TRUMPET's halo). Bits of `_gaussian_kernel1d(2.4, 0, 10)[::-1]`,
/// scipy 1.18.1 on the reference PC (`studio/reference/shared/ndimage.npz` holds them too).
pub const KIT_KERNELS: [(u64, usize, [u64; 11]); 1] = [(
    0x4003_3333_3333_3333, // 2.4
    10,
    [
        0x3efd_9b3e_7bc5_0724,
        0x3f23_41b2_609c_808a,
        0x3f45_0eb6_6685_b2d0,
        0x3f63_5b57_ff6d_8e9d,
        0x3f7d_ea4f_d80f_ac2b,
        0x3f93_6eaa_9e96_1e56,
        0x3fa5_38d5_d50c_afcb,
        0x3fb3_7b97_141f_dd31,
        0x3fbe_1227_928b_e957,
        0x3fc3_8210_87a8_edd9,
        0x3fc5_46f3_0075_011f,
    ],
)];

/// `_gaussian_kernel1d(sigma, 0, radius)[::-1]`: the `2 * radius + 1` weights (symmetric, so the reversal
/// changes nothing). A sigma and radius of [`KIT_KERNELS`] give scipy's weights from there.
pub fn gaussian_kernel1d(sigma: f64, radius: usize) -> Vec<f64> {
    if let Some((_, _, half)) = KIT_KERNELS
        .iter()
        .find(|(s, r, _)| *s == sigma.to_bits() && *r == radius)
    {
        let mut w: Vec<f64> = half.iter().map(|&b| f64::from_bits(b)).collect();
        w.extend(half[..radius].iter().rev().map(|&b| f64::from_bits(b)));
        return w;
    }
    computed_kernel1d(sigma, radius)
}

/// [`gaussian_kernel1d`] computed, never from [`KIT_KERNELS`] (`dmath::exp`; a test compares the two).
pub fn computed_kernel1d(sigma: f64, radius: usize) -> Vec<f64> {
    let sigma2 = sigma * sigma;
    let c = -0.5 / sigma2;
    let r = radius as i64;
    // phi_x = np.exp(-0.5 / sigma2 * x ** 2): the integer square, then the float product
    let phi: Vec<f64> = (-r..=r).map(|x| dmath::exp(c * (x * x) as f64)).collect();
    let s = np::sum(&phi);
    let mut w: Vec<f64> = phi.iter().map(|&p| p / s).collect();
    w.reverse();
    w
}

/// `NI_ExtendLine(buffer, len, before, after, NI_EXTEND_REFLECT)`: `buf` holds the line at `before..before + len`;
/// fills both ends exactly as the C walks them (past a whole reflection it reads what it has already written).
fn extend_reflect(buf: &mut [f64], len: usize, before: usize, after: usize) {
    let first = before as isize;
    let last = first + len as isize;
    let at = |i: isize| i as usize;
    // src = first; dst = first - 1; while size_before && src < last: *dst-- = *src++
    let (mut src, mut dst) = (first, first - 1);
    let mut size_before = before;
    while size_before > 0 && src < last {
        buf[at(dst)] = buf[at(src)];
        dst -= 1;
        src += 1;
        size_before -= 1;
    }
    // src = last - 1; while size_before--: *dst-- = *src--
    src = last - 1;
    while size_before > 0 {
        buf[at(dst)] = buf[at(src)];
        dst -= 1;
        src -= 1;
        size_before -= 1;
    }
    // src = last - 1; dst = last; while size_after && src >= first: *dst++ = *src--
    let (mut src, mut dst) = (last - 1, last);
    let mut size_after = after;
    while size_after > 0 && src >= first {
        buf[at(dst)] = buf[at(src)];
        dst += 1;
        src -= 1;
        size_after -= 1;
    }
    // src = first; while size_after--: *dst++ = *src++
    src = first;
    while size_after > 0 {
        buf[at(dst)] = buf[at(src)];
        dst += 1;
        src += 1;
        size_after -= 1;
    }
}

/// How `NI_Correlate1D` reads the weights: symmetric, antisymmetric (to `DBL_EPSILON`) or neither.
fn symmetry(w: &[f64]) -> i32 {
    let size = w.len();
    let s1 = size / 2;
    if size & 1 == 0 {
        return 0;
    }
    if (1..=s1).all(|i| (w[i + s1] - w[s1 - i]).abs() <= f64::EPSILON) {
        return 1;
    }
    if (1..=s1).all(|i| (w[s1 + i] + w[s1 - i]).abs() <= f64::EPSILON) {
        return -1;
    }
    0
}

/// One line through the weights (`NI_Correlate1D`'s inner loops, origin 0, mode `reflect`): `line` in, the
/// correlated line out, in `float64`.
fn correlate_line(line: &[f64], w: &[f64], sym: i32, buf: &mut Vec<f64>, out: &mut Vec<f64>) {
    let size = w.len();
    let size1 = size / 2;
    let size2 = size - size1 - 1;
    let len = line.len();
    buf.clear();
    buf.resize(size1 + len + size2, 0.0);
    buf[size1..size1 + len].copy_from_slice(line);
    if size1 + size2 > 0 {
        extend_reflect(buf, len, size1, size2);
    }
    out.clear();
    // fw += size1: fw[j] is w[size1 + j]; iline[j] is buf[size1 + l + j]
    for l in 0..len {
        let c = size1 + l;
        let o = match sym {
            1 => {
                let mut o = buf[c] * w[size1];
                for j in (1..=size1).rev() {
                    o += (buf[c - j] + buf[c + j]) * w[size1 - j];
                }
                o
            }
            -1 => {
                let mut o = buf[c] * w[size1];
                for j in (1..=size1).rev() {
                    o += (buf[c - j] - buf[c + j]) * w[size1 - j];
                }
                o
            }
            _ => {
                // oline = iline[size2] * fw[size2]; for jj in -size1..size2: oline += iline[jj] * fw[jj]
                let mut o = buf[c + size2] * w[size1 + size2];
                for jj in -(size1 as isize)..size2 as isize {
                    o += buf[(c as isize + jj) as usize] * w[(size1 as isize + jj) as usize];
                }
                o
            }
        };
        out.push(o);
    }
}

/// `scipy.ndimage.correlate1d(img, weights, axis, output=img's dtype, mode='reflect', cval=0.0, origin=0)` for a
/// single-channel picture: axis 0 runs down each column (numpy's rows axis), axis 1 along each row.
///
/// # Panics
/// If `img` has more than one channel, `axis` is not 0 or 1, or there are no weights (`no filter weights given`).
pub fn correlate1d<T: Sample>(img: &Img<T>, weights: &[f64], axis: usize) -> Img<T> {
    assert_eq!(img.c, 1, "correlate1d: a picture of one channel, not {}", img.c);
    assert!(axis < 2, "correlate1d: axis {axis} of a two-dimensional picture");
    assert!(!weights.is_empty(), "no filter weights given");
    let mut out = img.clone();
    let (w, h) = (img.w, img.h);
    if w == 0 || h == 0 {
        return out;
    }
    let sym = symmetry(weights);
    let (mut buf, mut res) = (Vec::new(), Vec::new());
    if axis == 0 {
        let mut line = vec![0.0; h];
        for x in 0..w {
            for (y, v) in line.iter_mut().enumerate() {
                *v = img.data[y * w + x].to_f64();
            }
            correlate_line(&line, weights, sym, &mut buf, &mut res);
            for (y, &v) in res.iter().enumerate() {
                out.data[y * w + x] = T::from_f64(v);
            }
        }
    } else {
        let mut line = vec![0.0; w];
        for y in 0..h {
            for (x, v) in line.iter_mut().enumerate() {
                *v = img.data[y * w + x].to_f64();
            }
            correlate_line(&line, weights, sym, &mut buf, &mut res);
            for (x, &v) in res.iter().enumerate() {
                out.data[y * w + x] = T::from_f64(v);
            }
        }
    }
    out
}

/// `scipy.ndimage.gaussian_filter(img, sigma)`: both axes, rows first, the result in `img`'s dtype.
///
/// # Panics
/// If `img` has more than one channel, or where Python raises (a sigma whose radius is not a number).
pub fn gaussian_filter<T: Sample>(img: &Img<T>, sigma: f64) -> Img<T> {
    assert_eq!(img.c, 1, "gaussian_filter: a picture of one channel, not {}", img.c);
    // axes = [... for ii in range(num_axes) if sigmas[ii] > 1e-15]; none left: output[...] = input[...]
    if sigma.is_nan() || sigma <= 1e-15 {
        return img.clone();
    }
    let weights = gaussian_kernel1d(sigma, radius(sigma, TRUNCATE));
    let rows = correlate1d(img, &weights, 0);
    correlate1d(&rows, &weights, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_img(v: &[f64]) -> Img<f64> {
        Img::from_vec(v.len(), 1, 1, v.to_vec())
    }

    #[test]
    fn reflect_goes_on_round_the_line() {
        // a b | a b | b a ... as the C writes it: "abcddcba|abcd|dcbaabcd", and further round
        let mut buf = vec![0.0; 5 + 2 + 6];
        buf[5] = 1.0;
        buf[6] = 2.0;
        extend_reflect(&mut buf, 2, 5, 6);
        assert_eq!(buf, [1.0, 1.0, 2.0, 2.0, 1.0, 1.0, 2.0, 2.0, 1.0, 1.0, 2.0, 2.0, 1.0]);
        let mut one = vec![0.0; 3 + 1 + 3];
        one[3] = 5.0;
        extend_reflect(&mut one, 1, 3, 3);
        assert_eq!(one, [5.0; 7]);
    }

    #[test]
    fn the_kernel_and_the_radius() {
        assert_eq!(radius(2.4, TRUNCATE), 10);
        assert_eq!(radius(0.12, TRUNCATE), 0);
        assert_eq!(radius(0.125, TRUNCATE), 1);
        assert_eq!(radius(-0.1, TRUNCATE), 0);
        let w = gaussian_kernel1d(1.0, 4);
        assert_eq!(w.len(), 9);
        assert_eq!(w[0].to_bits(), w[8].to_bits());
        assert!((w.iter().sum::<f64>() - 1.0).abs() < 1e-15);
        assert_eq!(gaussian_kernel1d(0.01, 0), [1.0]);
        assert_eq!(symmetry(&w), 1);
        assert_eq!(symmetry(&[1.0, 0.0, -1.0]), -1);
        assert_eq!(symmetry(&[1.0, 3.0]), 0);
    }

    #[test]
    fn a_tiny_or_no_sigma_is_a_copy_and_a_constant_stays_constant() {
        let img = line_img(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(gaussian_filter(&img, 0.0), img);
        assert_eq!(gaussian_filter(&img, 1e-16), img);
        assert_eq!(gaussian_filter(&img, -1.0), img);
        assert_eq!(gaussian_filter(&img, f64::NAN), img);
        let flat: Img<f32> = Img::from_vec(5, 4, 1, vec![0.75; 20]);
        let out = gaussian_filter(&flat, 3.0);
        assert!(out.data.iter().all(|&v| (v - 0.75).abs() < 1e-6), "{:?}", out.data);
        // the general branch of correlate1d: an odd kernel that is neither symmetric nor antisymmetric
        let g = correlate1d(&line_img(&[1.0, 2.0, 3.0]), &[0.0, 1.0, 2.0], 1);
        // reflect: [1 | 1 2 3 | 3]: 1*1 + 2*2, 2*1 + 3*2, 3*1 + 3*2
        assert_eq!(g.data, [5.0, 8.0, 9.0]);
    }
}
