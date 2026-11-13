//! Convolutions: `scipy.signal.fftconvolve` (through the Studio's own FFT) and `np.convolve(a, v, mode)` in numpy's
//! order of addition, with the BLAS dot product under it ([`blas_dot`]).

use bf_core::dmath;

use super::fft::Fft;

/// `scipy.signal.fftconvolve(a, b)` (mode `'full'`): the convolution of two real signals, `len(a) + len(b) - 1`
/// samples, by multiplying their spectra. scipy pads to the next 5-smooth size (`next_fast_len`); this pads to
/// the next power of two, so the samples agree with scipy's to rounding (about 1e-13 of the output's scale),
/// not to the bit. The order of the operands does not change the result. Empty if either operand is empty.
pub fn fftconvolve(a: &[f64], b: &[f64]) -> Vec<f64> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let n = a.len() + b.len() - 1;
    let size = n.next_power_of_two();
    let fft = Fft::new(size);
    let spectrum = |x: &[f64]| -> (Vec<f64>, Vec<f64>) {
        let mut re = vec![0.0; size];
        re[..x.len()].copy_from_slice(x);
        let mut im = vec![0.0; size];
        fft.forward(&mut re, &mut im);
        (re, im)
    };
    let (mut re, mut im) = spectrum(a);
    {
        let (b_re, b_im) = spectrum(b);
        for (((ar, ai), br), bi) in re.iter_mut().zip(&mut im).zip(&b_re).zip(&b_im) {
            (*ar, *ai) = (*ar * br - *ai * bi, *ar * bi + *ai * br);
        }
    }
    fft.inverse(&mut re, &mut im);
    re.truncate(n);
    re
}

/// numpy's `np.convolve(a, v, 'same')`: the middle `max(len(a), len(v))` samples of the full convolution (it
/// starts `(len(shorter) - 1) / 2` samples in, rounded down), added in numpy's order ([`correlate`]).
///
/// # Panics
/// Where numpy raises: `a` or `v` is empty.
pub fn convolve_same(a: &[f64], v: &[f64]) -> Vec<f64> {
    convolve(a, v, true)
}

/// numpy's `np.convolve(a, v)` (mode `'full'`): `len(a) + len(v) - 1` samples, added in numpy's order
/// ([`correlate`]). scipy's `lfilter` takes this road when `a` has one coefficient.
///
/// # Panics
/// Where numpy raises: `a` or `v` is empty.
pub fn convolve_full(a: &[f64], v: &[f64]) -> Vec<f64> {
    convolve(a, v, false)
}

/// `np.convolve` (numpy 2.5.3 `_core/numeric.py`): the longer operand first (the first one when they are as long),
/// then `multiarray.correlate(a, v[::-1], mode)`, which copies the reversed window into a contiguous array.
fn convolve(a: &[f64], v: &[f64], same: bool) -> Vec<f64> {
    assert!(!a.is_empty(), "a cannot be empty");
    assert!(!v.is_empty(), "v cannot be empty");
    let (a, v) = if v.len() > a.len() { (v, a) } else { (a, v) };
    let k: Vec<f64> = v.iter().rev().copied().collect();
    let n = k.len();
    let (left, right) = if same { (n / 2, n - n / 2 - 1) } else { (n - 1, n - 1) };
    correlate(a, &k, left, right)
}

/// `_pyarray_correlate` (numpy 2.5.3 `_core/src/multiarray/multiarraymodule.c`) for `float64`, `a` at least as
/// long as `k`, with `left` and `right` outputs where `k` hangs over the ends of `a`. The outputs where the
/// window hangs over an end are each `0.0 +` a [`blas_dot`] of the overlap (`DOUBLE_dot` in `arraytypes.c.src`
/// adds the library's result to a zero). Those where it lies wholly inside `a` are `small_correlate`'s plain sum
/// from zero, product by product in order, for a window of at most 11 samples, and a [`blas_dot`] otherwise.
fn correlate(a: &[f64], k: &[f64], left: usize, right: usize) -> Vec<f64> {
    let (n1, n2) = (a.len(), k.len());
    let mut out = Vec::with_capacity(left + n1 - n2 + 1 + right);
    for i in 0..left {
        // the window's last `n2 - left + i` samples over the start of `a`
        let m = n2 - left + i;
        out.push(0.0 + blas_dot(&a[..m], &k[left - i..]));
    }
    for i in 0..=n1 - n2 {
        let d = &a[i..i + n2];
        out.push(if n2 <= 11 {
            let mut s = 0.0;
            for (x, y) in d.iter().zip(k) {
                s += x * y;
            }
            s
        } else {
            0.0 + blas_dot(d, k)
        });
    }
    for i in 1..=right {
        // the window's first `n2 - i` samples over the end of `a`
        let m = n2 - i;
        out.push(0.0 + blas_dot(&a[n1 - m..], &k[..m]));
    }
    out
}

/// `cblas_ddot(n, x, 1, y, 1)` as numpy 2.5.3's OpenBLAS (0.3.34, `DYNAMIC_ARCH`) computes it on the reference PC,
/// whose processor it runs with its SkylakeX kernels. Written from the kernel's structure and checked against
/// the library's results bit for bit (`tests/dsp_vectors.rs`, `blas_dot`): every product is fused into its
/// accumulator ([`dmath::fma`]). The first `n & !15` products go through vector accumulators: 32 at a time into
/// four of eight lanes (lane `l` of accumulator `k` takes product `32 i + 8 k + l`), folded into four of four lanes
/// (lane `l` plus lane `l + 4`); then 16 at a time into those four (lane `l` of accumulator `k` takes product
/// `16 i + 4 k + l`). The four are added in order, `((a0 + a1) + a2) + a3` lane by lane, and the four lanes as
/// `(l0 + l2) + (l1 + l3)`. The last `n % 16` products are fused one by one onto that, in order.
///
/// Above 10,000 products the library splits the sum among the PC's threads, which depends on the PC; that is
/// not repeated here (no caller comes near: the longest window is 20 ms of sound).
///
/// # Panics
/// If `x` and `y` differ in length.
pub fn blas_dot(x: &[f64], y: &[f64]) -> f64 {
    assert_eq!(x.len(), y.len(), "blas_dot: the operands differ in length");
    let n = x.len();
    let n16 = n & !15;
    let mut dot = 0.0;
    if n16 > 0 {
        let n32 = n & !31;
        let mut acc = [[0.0f64; 4]; 4];
        if n32 > 0 {
            let mut wide = [[0.0f64; 8]; 4];
            for (xs, ys) in x[..n32].as_chunks::<32>().0.iter().zip(y[..n32].as_chunks::<32>().0) {
                for (k, w) in wide.iter_mut().enumerate() {
                    for (l, s) in w.iter_mut().enumerate() {
                        *s = dmath::fma(xs[8 * k + l], ys[8 * k + l], *s);
                    }
                }
            }
            for (a, w) in acc.iter_mut().zip(&wide) {
                for l in 0..4 {
                    a[l] = w[l] + w[l + 4];
                }
            }
        }
        for (xs, ys) in x[n32..n16]
            .as_chunks::<16>()
            .0
            .iter()
            .zip(y[n32..n16].as_chunks::<16>().0)
        {
            for (k, a) in acc.iter_mut().enumerate() {
                for (l, s) in a.iter_mut().enumerate() {
                    *s = dmath::fma(xs[4 * k + l], ys[4 * k + l], *s);
                }
            }
        }
        let lane = |l: usize| ((acc[0][l] + acc[1][l]) + acc[2][l]) + acc[3][l];
        dot = (lane(0) + lane(2)) + (lane(1) + lane(3));
    }
    for (a, b) in x[n16..].iter().zip(&y[n16..]) {
        dot = dmath::fma(*b, *a, dot);
    }
    dot
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The full convolution by its definition.
    fn direct(a: &[f64], b: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; a.len() + b.len() - 1];
        for (i, x) in a.iter().enumerate() {
            for (j, y) in b.iter().enumerate() {
                out[i + j] += x * y;
            }
        }
        out
    }

    #[test]
    fn fftconvolve_agrees_with_the_definition() {
        let a: Vec<f64> = (0..37).map(|i| ((i * 29 + 3) % 17) as f64 / 8.0 - 1.0).collect();
        let b: Vec<f64> = (0..12).map(|i| ((i * 13 + 5) % 11) as f64 / 5.0 - 1.0).collect();
        let want = direct(&a, &b);
        let got = fftconvolve(&a, &b);
        assert_eq!(got.len(), 48);
        for (g, w) in got.iter().zip(&want) {
            assert!((g - w).abs() < 1e-13, "{g} against {w}");
        }
        assert_eq!(fftconvolve(&b, &a), got);
        assert_eq!(fftconvolve(&[2.0], &[3.0]), [6.0]);
        assert!(fftconvolve(&[], &[1.0]).is_empty());
        assert!(fftconvolve(&[1.0], &[]).is_empty());
    }

    #[test]
    fn convolve_same_is_the_middle_of_the_full_convolution() {
        // numpy's docstring: np.convolve([1, 2, 3], [0, 1, 0.5], 'same') is [1, 2.5, 4].
        assert_eq!(convolve_same(&[1.0, 2.0, 3.0], &[0.0, 1.0, 0.5]), [1.0, 2.5, 4.0]);
        // An even window sits one sample early: the full convolution from index (4 - 1) / 2 = 1.
        let a = [1.0, 2.0, 4.0, 8.0, 16.0];
        let full = direct(&a, &[1.0; 4]);
        assert_eq!(convolve_same(&a, &[1.0; 4]), full[1..6]);
        assert_eq!(convolve_same(&a, &[1.0; 3]), direct(&a, &[1.0; 3])[1..6]);
        // The shorter operand is the window, whichever comes first.
        assert_eq!(convolve_same(&[1.0; 4], &a), full[1..6]);
        assert_eq!(convolve_same(&[3.0], &[2.0]), [6.0]);
    }

    #[test]
    #[should_panic(expected = "v cannot be empty")]
    fn convolve_same_wants_a_window() {
        convolve_same(&[1.0], &[]);
    }
}
