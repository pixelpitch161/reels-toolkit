//! Convolutions: `scipy.signal.fftconvolve` (through the Studio's own FFT) and `np.convolve(a, v, 'same')`.

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

/// `np.convolve(a, v, 'same')`: the middle `max(len(a), len(v))` samples of the full convolution (it starts
/// `(len(shorter) - 1) / 2` samples in, rounded down). Each sample is a sum of products added from zero in the
/// order of the longer operand; numpy adds the same products with its dot product, so the last bits can differ.
///
/// # Panics
/// Where numpy raises: `a` or `v` is empty.
pub fn convolve_same(a: &[f64], v: &[f64]) -> Vec<f64> {
    assert!(!a.is_empty(), "a cannot be empty");
    assert!(!v.is_empty(), "v cannot be empty");
    let (a, v) = if v.len() > a.len() { (v, a) } else { (a, v) };
    let (n1, n2) = (a.len(), v.len());
    let first = n2 - n2 / 2 - 1;
    (first..first + n1)
        .map(|k| {
            let mut sum = 0.0;
            for j in k.saturating_sub(n2 - 1)..=k.min(n1 - 1) {
                sum += a[j] * v[k - j];
            }
            sum
        })
        .collect()
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
