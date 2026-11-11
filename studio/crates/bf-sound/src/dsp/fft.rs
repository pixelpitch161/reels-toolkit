//! A radix-2 fast Fourier transform, written out in scalar code with one fixed order of operations.
//!
//! No SIMD library, no runtime dispatch and no threads: a transform is the same sequence of `f64` additions and
//! multiplications on every PC, so its output has the same bits everywhere. The twiddle factors come from
//! `dmath`. This is the Studio's own transform (scipy uses pocketfft at a mixed-radix size); results agree with
//! scipy's to rounding, not to the bit.

use bf_core::dmath;

/// A transform of one power-of-two size, with its table of twiddle factors.
pub struct Fft {
    n: usize,
    /// `cos(2 pi k / n)` for `k < n / 2`.
    cos: Vec<f64>,
    /// `sin(2 pi k / n)` for `k < n / 2`.
    sin: Vec<f64>,
}

impl Fft {
    /// A transform of `n` points.
    ///
    /// # Panics
    /// If `n` is not a power of two.
    pub fn new(n: usize) -> Fft {
        assert!(n.is_power_of_two(), "the transform size {n} is not a power of two");
        let angle = |k: usize| dmath::TAU * k as f64 / n as f64;
        Fft {
            n,
            cos: (0..n / 2).map(|k| dmath::cos(angle(k))).collect(),
            sin: (0..n / 2).map(|k| dmath::sin(angle(k))).collect(),
        }
    }

    /// The number of points.
    pub fn len(&self) -> usize {
        self.n
    }

    /// Whether the transform has no points (never: the smallest size is 1).
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// The forward transform, in place: `X[k] = sum over j of x[j] exp(-2 pi i j k / n)`.
    ///
    /// # Panics
    /// If `re` or `im` is not `n` long.
    pub fn forward(&self, re: &mut [f64], im: &mut [f64]) {
        self.transform(re, im, -1.0);
    }

    /// The inverse transform, in place, divided by `n`: `inverse(forward(x))` is `x` up to rounding.
    ///
    /// # Panics
    /// If `re` or `im` is not `n` long.
    pub fn inverse(&self, re: &mut [f64], im: &mut [f64]) {
        self.transform(re, im, 1.0);
        let n = self.n as f64;
        for v in re.iter_mut().chain(im.iter_mut()) {
            *v /= n;
        }
    }

    /// Bit reversal, then the butterflies stage by stage (blocks of 2, 4, 8, ... points). `sign` is the sign of
    /// the exponent.
    fn transform(&self, re: &mut [f64], im: &mut [f64], sign: f64) {
        let n = self.n;
        assert!(re.len() == n && im.len() == n, "the transform takes {n} points");
        if n == 1 {
            return;
        }
        let shift = usize::BITS - n.trailing_zeros();
        for i in 0..n {
            let j = i.reverse_bits() >> shift;
            if i < j {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        // Blocks of two: the twiddle factor is 1.
        for (r, i) in re.as_chunks_mut::<2>().0.iter_mut().zip(im.as_chunks_mut::<2>().0) {
            (r[0], r[1]) = (r[0] + r[1], r[0] - r[1]);
            (i[0], i[1]) = (i[0] + i[1], i[0] - i[1]);
        }
        let mut len = 4;
        while len <= n {
            let half = len / 2;
            let step = n / len;
            for (block_re, block_im) in re.chunks_exact_mut(len).zip(im.chunks_exact_mut(len)) {
                let (re_a, re_b) = block_re.split_at_mut(half);
                let (im_a, im_b) = block_im.split_at_mut(half);
                let twiddles = self.cos.iter().step_by(step).zip(self.sin.iter().step_by(step));
                for ((((ra, rb), ia), ib), (&wr, &ws)) in re_a.iter_mut().zip(re_b).zip(im_a).zip(im_b).zip(twiddles) {
                    let wi = sign * ws;
                    let tr = *rb * wr - *ib * wi;
                    let ti = *rb * wi + *ib * wr;
                    *rb = *ra - tr;
                    *ib = *ia - ti;
                    *ra += tr;
                    *ia += ti;
                }
            }
            len *= 2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The transform by its definition.
    fn dft(re: &[f64], im: &[f64], sign: f64) -> (Vec<f64>, Vec<f64>) {
        let n = re.len();
        let (mut out_re, mut out_im) = (vec![0.0; n], vec![0.0; n]);
        for k in 0..n {
            for j in 0..n {
                let a = sign * dmath::TAU * ((j * k) % n) as f64 / n as f64;
                let (c, s) = (dmath::cos(a), dmath::sin(a));
                out_re[k] += re[j] * c - im[j] * s;
                out_im[k] += re[j] * s + im[j] * c;
            }
        }
        (out_re, out_im)
    }

    #[test]
    fn agrees_with_the_definition() {
        for n in [1usize, 2, 4, 8, 64, 256] {
            // A fixed, patternless input.
            let re: Vec<f64> = (0..n).map(|i| ((i * 37 + 11) % 101) as f64 / 50.0 - 1.0).collect();
            let im: Vec<f64> = (0..n).map(|i| ((i * 53 + 7) % 89) as f64 / 44.0 - 1.0).collect();
            let (want_re, want_im) = dft(&re, &im, -1.0);
            let fft = Fft::new(n);
            assert_eq!(fft.len(), n);
            let (mut got_re, mut got_im) = (re.clone(), im.clone());
            fft.forward(&mut got_re, &mut got_im);
            for k in 0..n {
                assert!(
                    (got_re[k] - want_re[k]).abs() < 1e-11 && (got_im[k] - want_im[k]).abs() < 1e-11,
                    "n = {n}, k = {k}"
                );
            }
            fft.inverse(&mut got_re, &mut got_im);
            for j in 0..n {
                assert!(
                    (got_re[j] - re[j]).abs() < 1e-13 && (got_im[j] - im[j]).abs() < 1e-13,
                    "n = {n}, j = {j}"
                );
            }
        }
    }

    #[test]
    fn an_impulse_has_a_flat_spectrum() {
        let fft = Fft::new(16);
        let (mut re, mut im) = (vec![0.0; 16], vec![0.0; 16]);
        re[0] = 1.0;
        fft.forward(&mut re, &mut im);
        assert_eq!(re, [1.0; 16]);
        assert_eq!(im, [0.0; 16]);
    }

    #[test]
    #[should_panic(expected = "the transform size 12 is not a power of two")]
    fn other_sizes_are_refused() {
        Fft::new(12);
    }
}
