//! Recursive filters: `scipy.signal.sosfilt` and `scipy.signal.lfilter`.
//!
//! Both are the transposed direct form II, with the additions and multiplications in scipy's order, so on the same
//! coefficients they give the same bits as scipy 1.18.1 (`_sosfilt.pyx` `_sosfilt_float`, `_lfilter.cc` `filt`).

/// Second-order sections as scipy orders them: each row is `b0 b1 b2 a0 a1 a2`.
pub type Sos = Vec<[f64; 6]>;

/// The state of a cascade: two delays per section, zero at the start (`zi=None`).
fn sos_run(sos: &[[f64; 6]], zi: &mut [[f64; 2]], x: f64) -> f64 {
    let mut x_cur = x;
    for (s, z) in sos.iter().zip(zi) {
        // `a0` (s[3]) is not read: scipy's loop assumes it is 1, which is what `butter` returns.
        let x_new = s[0] * x_cur + z[0];
        z[0] = s[1] * x_cur - s[4] * x_new + z[1];
        z[1] = s[2] * x_cur - s[5] * x_new;
        x_cur = x_new;
    }
    x_cur
}

/// `scipy.signal.sosfilt(sos, x)` on a mono signal, from rest.
pub fn sosfilt(sos: &Sos, x: &[f64]) -> Vec<f64> {
    let mut zi = vec![[0.0; 2]; sos.len()];
    x.iter().map(|&v| sos_run(sos, &mut zi, v)).collect()
}

/// `scipy.signal.sosfilt(sos, x, axis=0)` on a stereo signal: each channel filtered on its own, from rest.
pub fn sosfilt_stereo(sos: &Sos, x: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut zl = vec![[0.0; 2]; sos.len()];
    let mut zr = vec![[0.0; 2]; sos.len()];
    x.iter()
        .map(|&[l, r]| [sos_run(sos, &mut zl, l), sos_run(sos, &mut zr, r)])
        .collect()
}

/// `b` and `a` as scipy's kernel uses them: both as long as the longer one (padded with zeros) and divided by
/// `a[0]`.
///
/// # Panics
/// Where scipy raises: `b` or `a` is empty. (`a[0] == 0` gives infinities or NaN, as in scipy.)
fn normalised(b: &[f64], a: &[f64]) -> (Vec<f64>, Vec<f64>) {
    assert!(!b.is_empty(), "lfilter: b is empty");
    assert!(!a.is_empty(), "lfilter: a is empty");
    let n = b.len().max(a.len());
    let a0 = a[0];
    let pad = |c: &[f64]| -> Vec<f64> { (0..n).map(|i| c.get(i).copied().unwrap_or(0.0) / a0).collect() };
    (pad(b), pad(a))
}

/// One sample through the transposed direct form II of `_lfilter.cc`; `z` holds `b.len() - 1` delays.
fn lfilter_run(b: &[f64], a: &[f64], z: &mut [f64], x: f64) -> f64 {
    let n = b.len();
    if n == 1 {
        return x * b[0];
    }
    let y = z[0] + b[0] * x;
    for k in 1..n - 1 {
        z[k - 1] = z[k] + x * b[k] - y * a[k];
    }
    z[n - 2] = x * b[n - 1] - y * a[n - 1];
    y
}

/// `scipy.signal.lfilter(b, a, x)` on a mono signal, from rest.
///
/// scipy takes another road when `a` has one coefficient (`np.convolve(b / a[0], x)`, cut to `len(x)`): the same
/// sum of the same products, but added in the order of numpy's dot product, so there the last bits can differ.
///
/// # Panics
/// If `b` or `a` is empty.
pub fn lfilter(b: &[f64], a: &[f64], x: &[f64]) -> Vec<f64> {
    let (b, a) = normalised(b, a);
    let mut z = vec![0.0; b.len() - 1];
    x.iter().map(|&v| lfilter_run(&b, &a, &mut z, v)).collect()
}

/// `scipy.signal.lfilter(b, a, x, axis=0)` on a stereo signal: each channel filtered on its own, from rest.
///
/// # Panics
/// If `b` or `a` is empty.
pub fn lfilter_stereo(b: &[f64], a: &[f64], x: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let (b, a) = normalised(b, a);
    let mut zl = vec![0.0; b.len() - 1];
    let mut zr = vec![0.0; b.len() - 1];
    x.iter()
        .map(|&[l, r]| [lfilter_run(&b, &a, &mut zl, l), lfilter_run(&b, &a, &mut zr, r)])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_is_the_difference_equation() {
        // y[n] = b0 x[n] + b1 x[n-1] + b2 x[n-2] - a1 y[n-1] - a2 y[n-2], on an impulse.
        let sos: Sos = vec![[0.5, 0.25, 0.125, 1.0, -0.5, 0.25]];
        let y = sosfilt(&sos, &[1.0, 0.0, 0.0, 0.0]);
        assert_eq!(y, [0.5, 0.5, 0.25, 0.0]);
        // lfilter on the same coefficients agrees.
        assert_eq!(lfilter(&sos[0][..3], &sos[0][3..], &[1.0, 0.0, 0.0, 0.0]), y);
    }

    #[test]
    fn lfilter_divides_by_a0_and_pads_the_shorter_side() {
        // (2 x[n]) / 4 with one pole: y[n] = 0.5 x[n] + 0.5 y[n-1].
        assert_eq!(lfilter(&[2.0], &[4.0, -2.0], &[1.0, 0.0, 0.0]), [0.5, 0.25, 0.125]);
        // A pure gain, and a two-tap average with no feedback.
        assert_eq!(lfilter(&[3.0], &[2.0], &[1.0, -2.0]), [1.5, -3.0]);
        assert_eq!(lfilter(&[0.5, 0.5], &[1.0], &[1.0, 3.0, 5.0]), [0.5, 2.0, 4.0]);
        assert!(lfilter(&[1.0], &[1.0], &[]).is_empty());
    }

    #[test]
    fn stereo_filters_each_channel_on_its_own() {
        let sos: Sos = vec![[0.2, 0.3, 0.1, 1.0, -0.4, 0.2], [1.0, -1.0, 0.0, 1.0, 0.1, 0.0]];
        let l = [1.0, -0.5, 0.25, 2.0, 0.0];
        let r = [0.0, 3.0, -1.0, 0.5, 0.5];
        let x: Vec<[f64; 2]> = l.iter().zip(&r).map(|(&a, &b)| [a, b]).collect();
        let (yl, yr) = (sosfilt(&sos, &l), sosfilt(&sos, &r));
        let y = sosfilt_stereo(&sos, &x);
        assert_eq!(y.iter().map(|f| f[0]).collect::<Vec<_>>(), yl);
        assert_eq!(y.iter().map(|f| f[1]).collect::<Vec<_>>(), yr);
        let (b, a) = ([0.3, 0.2, 0.1], [1.0, -0.5, 0.1]);
        let y = lfilter_stereo(&b, &a, &x);
        assert_eq!(y.iter().map(|f| f[0]).collect::<Vec<_>>(), lfilter(&b, &a, &l));
        assert_eq!(y.iter().map(|f| f[1]).collect::<Vec<_>>(), lfilter(&b, &a, &r));
    }

    #[test]
    #[should_panic(expected = "lfilter: a is empty")]
    fn lfilter_wants_coefficients() {
        lfilter(&[1.0], &[], &[1.0]);
    }
}
