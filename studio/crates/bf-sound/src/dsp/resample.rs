//! `scipy.signal.resample_poly`: polyphase resampling by a whole-number ratio, with scipy's default filter.
//!
//! Ported from scipy 1.18.1: `resample_poly` (`_signaltools.py`), the low-pass it designs (`firwin` in
//! `_fir_filter_design.py`, with `np.sinc` and the Kaiser window of `windows/_windows.py`), and the polyphase
//! filter itself (`upfirdn`: `_pad_h` in `_upfirdn.py`, `_apply_impl` in `_upfirdn_apply.pyx`), in zero-padding
//! mode. The Kaiser window needs the Bessel function `i0`: scipy takes it from its `xsf` library (Cephes `i0`),
//! whose coefficients and Chebyshev evaluation are the ones numpy 2.5.3 carries in `lib/_function_base_impl.py`
//! (`_i0A`, `_i0B`, `_chbevl`), ported here.

use bf_core::dmath;

use super::npstat::sum;

/// Chebyshev coefficients for `exp(-x) I0(x)` on `[0, 8]` (Cephes `i0.c`, array `A`), digit for digit.
#[allow(clippy::excessive_precision)]
const I0_A: [f64; 30] = [
    -4.41534164647933937950E-18,
    3.33079451882223809783E-17,
    -2.43127984654795469359E-16,
    1.71539128555513303061E-15,
    -1.16853328779934516808E-14,
    7.67618549860493561688E-14,
    -4.85644678311192946090E-13,
    2.95505266312963983461E-12,
    -1.72682629144155570723E-11,
    9.67580903537323691224E-11,
    -5.18979560163526290666E-10,
    2.65982372468238665035E-9,
    -1.30002500998624804212E-8,
    6.04699502254191894932E-8,
    -2.67079385394061173391E-7,
    1.11738753912010371815E-6,
    -4.41673835845875056359E-6,
    1.64484480707288970893E-5,
    -5.75419501008210370398E-5,
    1.88502885095841655729E-4,
    -5.76375574538582365885E-4,
    1.63947561694133579842E-3,
    -4.32430999505057594430E-3,
    1.05464603945949983183E-2,
    -2.37374148058994688156E-2,
    4.93052842396707084878E-2,
    -9.49010970480476444210E-2,
    1.71620901522208775349E-1,
    -3.04682672343198398683E-1,
    6.76795274409476084995E-1,
];

/// Chebyshev coefficients for `exp(-x) sqrt(x) I0(x)` on `[8, infinity)` (Cephes `i0.c`, array `B`), digit for
/// digit.
#[allow(clippy::excessive_precision)]
const I0_B: [f64; 25] = [
    -7.23318048787475395456E-18,
    -4.83050448594418207126E-18,
    4.46562142029675999901E-17,
    3.46122286769746109310E-17,
    -2.82762398051658348494E-16,
    -3.42548561967721913462E-16,
    1.77256013305652638360E-15,
    3.81168066935262242075E-15,
    -9.55484669882830764870E-15,
    -4.15056934728722208663E-14,
    1.54008621752140982691E-14,
    3.85277838274214270114E-13,
    7.18012445138366623367E-13,
    -1.79417853150680611778E-12,
    -1.32158118404477131188E-11,
    -3.14991652796324136454E-11,
    1.18891471078464383424E-11,
    4.94060238822496958910E-10,
    3.39623202570838634515E-9,
    2.26666899049817806459E-8,
    2.04891858946906374183E-7,
    2.89137052083475648297E-6,
    6.88975834691682398426E-5,
    3.36911647825569408990E-3,
    8.04490411014108831608E-1,
];

/// Cephes `chbevl`: a Chebyshev series by Clenshaw's recurrence.
fn chbevl(x: f64, coefficients: &[f64]) -> f64 {
    let mut b0 = coefficients[0];
    let mut b1 = 0.0;
    let mut b2 = 0.0;
    for &c in &coefficients[1..] {
        b2 = b1;
        b1 = b0;
        b0 = x * b1 - b2 + c;
    }
    0.5 * (b0 - b2)
}

/// `scipy.special.i0`: the modified Bessel function of the first kind, order 0 (Cephes `i0`).
fn i0(x: f64) -> f64 {
    let x = x.abs();
    if x <= 8.0 {
        dmath::exp(x) * chbevl(x / 2.0 - 2.0, &I0_A)
    } else {
        dmath::exp(x) * chbevl(32.0 / x - 2.0, &I0_B) / x.sqrt()
    }
}

/// `scipy.signal.windows.kaiser(m, beta)` (symmetric).
fn kaiser(m: usize, beta: f64) -> Vec<f64> {
    if m <= 1 {
        return vec![1.0; m];
    }
    let alpha = (m - 1) as f64 / 2.0;
    let i0_beta = i0(beta);
    (0..m)
        .map(|n| {
            let r = (n as f64 - alpha) / alpha;
            i0(beta * (1.0 - r * r).sqrt()) / i0_beta
        })
        .collect()
}

/// `np.sinc`: `sin(pi x) / (pi x)`, with numpy's stand-in for a zero argument.
fn sinc(x: f64) -> f64 {
    let x = dmath::PI * x;
    let y = if x != 0.0 { x } else { f64::EPSILON };
    dmath::sin(y) / y
}

/// `scipy.signal.firwin(numtaps, cutoff, window=('kaiser', beta))`: a low-pass by the window method, `cutoff`
/// as a fraction of the Nyquist frequency, scaled to a gain of exactly 1 at 0 Hz (the taps are divided by their
/// sum, which numpy adds pairwise).
///
/// # Panics
/// Where scipy raises: `cutoff` is not between 0 and 1.
pub fn firwin_kaiser(numtaps: usize, cutoff: f64, beta: f64) -> Vec<f64> {
    assert!(
        cutoff > 0.0 && cutoff < 1.0,
        "Invalid cutoff frequency: frequencies must be greater than 0 and less than fs/2."
    );
    // One pass band, from `left` = 0 to `right` = the cutoff.
    let (left, right) = (0.0, cutoff);
    let alpha = 0.5 * (numtaps as f64 - 1.0);
    let window = kaiser(numtaps, beta);
    // The pass band starts at 0 Hz, so the gain is measured there: the cosine below is cos(0) for every tap.
    let scale_frequency = 0.0;
    let mut h = Vec::with_capacity(numtaps);
    let mut weighed = Vec::with_capacity(numtaps);
    for (i, w) in window.iter().enumerate() {
        let m = i as f64 - alpha;
        let mut tap = right * sinc(right * m);
        tap -= left * sinc(left * m);
        tap *= w;
        h.push(tap);
        weighed.push(tap * dmath::cos(dmath::PI * m * scale_frequency));
    }
    let s = sum(&weighed);
    for tap in &mut h {
        *tap /= s;
    }
    h
}

/// `_output_len`: how many samples `upfirdn` returns for a filter of `len_h` taps and `in_len` input samples.
fn output_len(len_h: usize, in_len: usize, up: usize, down: usize) -> usize {
    (((in_len - 1) * up + len_h) - 1) / down + 1
}

/// `scipy.signal.upfirdn(h, x, up, down)`: put `up - 1` zeros after every sample, filter with `h`, keep every
/// `down`-th sample; values beyond the ends of `x` count as zero. Done phase by phase, in scipy's order of
/// additions, so on the same filter the output has scipy's bits. An empty `x` gives an empty output.
///
/// # Panics
/// Where scipy raises: `h` is empty, or `up` or `down` is 0.
pub fn upfirdn(h: &[f64], x: &[f64], up: usize, down: usize) -> Vec<f64> {
    assert!(!h.is_empty(), "h must be 1-D with non-zero length");
    assert!(up >= 1 && down >= 1, "Both up and down must be >= 1");
    if x.is_empty() {
        return Vec::new();
    }
    // `_pad_h`: the taps of each phase, last first; a phase that runs out of taps is padded with zeros.
    let h_per_phase = h.len().div_ceil(up);
    let mut h_trans_flip = vec![0.0; h_per_phase * up];
    for phase in 0..up {
        for j in 0..h_per_phase {
            if let Some(&tap) = h.get(phase + (h_per_phase - 1 - j) * up) {
                h_trans_flip[phase * h_per_phase + j] = tap;
            }
        }
    }

    let len_x = x.len();
    let len_out = output_len(h.len(), len_x, up, down);
    let mut out = vec![0.0; len_out];
    let padded_len = len_x + h_per_phase - 1;
    let (mut x_idx, mut y_idx, mut t) = (0usize, 0usize, 0usize);
    // While the newest sample under the filter is inside `x`.
    while x_idx < len_x {
        let taps = &h_trans_flip[t * h_per_phase..(t + 1) * h_per_phase];
        // Where the filter hangs over the start, the taps that meet the zeros before `x` are skipped.
        let skipped = h_per_phase.saturating_sub(x_idx + 1);
        let first = (x_idx + 1).saturating_sub(h_per_phase);
        for (&v, &tap) in x[first..=x_idx].iter().zip(&taps[skipped..]) {
            out[y_idx] += v * tap;
        }
        y_idx += 1;
        if y_idx >= len_out {
            return out;
        }
        t += down;
        x_idx += t / up;
        t %= up;
    }
    // The tail: the filter hangs over the end, where the input counts as zero.
    while x_idx < padded_len {
        let taps = &h_trans_flip[t * h_per_phase..(t + 1) * h_per_phase];
        for (j, &tap) in taps.iter().enumerate() {
            // Tap j meets sample x_idx + 1 - h_per_phase + j.
            let v = (x_idx + 1 + j)
                .checked_sub(h_per_phase)
                .and_then(|i| x.get(i))
                .copied()
                .unwrap_or(0.0);
            out[y_idx] += v * tap;
        }
        y_idx += 1;
        if y_idx >= len_out {
            return out;
        }
        t += down;
        x_idx += t / up;
        t %= up;
    }
    out
}

/// `scipy.signal.resample_poly(x, up, down)` with scipy's defaults: the sample rate becomes `up / down` times
/// what it was, through a low-pass `firwin(2 * 10 * max(up, down) + 1, 1 / max(up, down), window=('kaiser',
/// 5.0))` scaled by `up`. The filter is centred (no delay) and the output has `ceil(len(x) * up / down)` samples.
///
/// # Panics
/// Where scipy raises: `up` or `down` is 0.
pub fn resample_poly(x: &[f64], up: usize, down: usize) -> Vec<f64> {
    assert!(up >= 1 && down >= 1, "up and down must be >= 1");
    let g = gcd(up, down);
    let (up, down) = (up / g, down / g);
    if up == 1 && down == 1 {
        return x.to_vec();
    }
    let n_in = x.len();
    if n_in == 0 {
        return Vec::new();
    }
    let n_out = (n_in * up).div_ceil(down);

    let max_rate = up.max(down);
    let half_len = 10 * max_rate;
    let taps = firwin_kaiser(2 * half_len + 1, 1.0 / max_rate as f64, 5.0);

    // Zeros in front put the output samples at the centre of the filter.
    let n_pre_pad = down - half_len % down;
    let n_pre_remove = (half_len + n_pre_pad) / down;
    let mut n_post_pad = 0;
    while output_len(taps.len() + n_pre_pad + n_post_pad, n_in, up, down) < n_out + n_pre_remove {
        n_post_pad += 1;
    }
    let mut h = vec![0.0; n_pre_pad];
    h.extend(taps.iter().map(|tap| tap * up as f64));
    h.resize(h.len() + n_post_pad, 0.0);

    let y = upfirdn(&h, x, up, down);
    y[n_pre_remove..n_pre_remove + n_out].to_vec()
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd(b, a % b) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i0_at_known_points() {
        // scipy.special.i0 at 1, 5 and 10 (the second Chebyshev series takes over above 8).
        assert_eq!(i0(0.0), 1.0);
        for (x, want) in [
            (1.0, 1.2660658777520082),
            (5.0, 27.239871823604442),
            (10.0, 2815.716628466254),
        ] {
            assert!((i0(x) / want - 1.0).abs() < 5e-16, "i0({x}) = {}", i0(x));
            assert_eq!(i0(-x), i0(x));
        }
    }

    #[test]
    fn kaiser_is_symmetric_with_a_peak_of_one() {
        let w = kaiser(51, 5.0);
        assert_eq!(w[25], 1.0);
        for i in 0..51 {
            assert_eq!(w[i], w[50 - i], "tap {i}");
        }
        assert!((w[0] - 1.0 / i0(5.0)).abs() < 1e-17);
        assert_eq!(kaiser(1, 5.0), [1.0]);
        assert!(kaiser(0, 5.0).is_empty());
    }

    #[test]
    fn firwin_has_unit_gain_at_0_hz() {
        let h = firwin_kaiser(101, 0.25, 5.0);
        assert!((sum(&h) - 1.0).abs() < 1e-14);
        for i in 0..101 {
            assert_eq!(h[i], h[100 - i], "tap {i}");
        }
    }

    #[test]
    fn upfirdn_matches_scipys_examples() {
        // From scipy's docstring: FIR filter, upsampling with zeros, linear interpolation, decimation.
        assert_eq!(
            upfirdn(&[1.0, 1.0, 1.0], &[1.0, 1.0, 1.0], 1, 1),
            [1.0, 2.0, 3.0, 2.0, 1.0]
        );
        assert_eq!(
            upfirdn(&[1.0], &[1.0, 2.0, 3.0], 3, 1),
            [1.0, 0.0, 0.0, 2.0, 0.0, 0.0, 3.0]
        );
        assert_eq!(
            upfirdn(&[1.0, 1.0, 1.0], &[1.0, 2.0, 3.0], 3, 1),
            [1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 3.0, 3.0, 3.0]
        );
        assert_eq!(
            upfirdn(&[0.5, 1.0, 0.5], &[1.0, 1.0, 1.0], 2, 1),
            [0.5, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5]
        );
        let ramp: Vec<f64> = (0..10).map(f64::from).collect();
        assert_eq!(upfirdn(&[1.0], &ramp, 1, 3), [0.0, 3.0, 6.0, 9.0]);
        assert_eq!(
            upfirdn(&[0.5, 1.0, 0.5], &ramp, 2, 3),
            [0.0, 1.0, 2.5, 4.0, 5.5, 7.0, 8.5]
        );
    }

    #[test]
    fn resample_keeps_a_constant_and_the_length_rule() {
        // Away from the ends a constant stays that constant; the length is ceil(n * up / down).
        let x = vec![1.0; 2000];
        let y = resample_poly(&x, 160, 147);
        assert_eq!(y.len(), (2000usize * 160).div_ceil(147));
        for (i, v) in y.iter().enumerate().skip(100).take(y.len() - 200) {
            assert!((v - 1.0).abs() < 1e-3, "sample {i}: {v}");
        }
        // The ratio is reduced first: 320 / 294 is 160 / 147, and 3 / 3 copies.
        assert_eq!(resample_poly(&x, 320, 294), y);
        assert_eq!(resample_poly(&[1.0, 2.0, 3.0], 3, 3), [1.0, 2.0, 3.0]);
        assert_eq!(resample_poly(&[1.0; 7], 1, 2).len(), 4);
        assert_eq!(resample_poly(&[1.0; 7], 2, 1).len(), 14);
    }
}
