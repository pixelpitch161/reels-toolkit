//! `scipy.signal.butter`: digital Butterworth designs, as second-order sections or as `b`, `a`.
//!
//! Ported from scipy 1.18.1 `scipy/signal/_filter_design.py`, step for step: `iirfilter` (the cutoff as a fraction
//! of the Nyquist frequency, pre-warped with `4 tan(pi Wn / 2)`), `buttap` (the analog prototype), `lp2lp_zpk`,
//! `lp2hp_zpk` or `lp2bp_zpk`, `bilinear_zpk` at `fs = 2`, then `zpk2sos` (through `_cplxreal`, with its
//! `'nearest'` pairing and its section order) or `zpk2tf`. Complex numbers follow numpy 2.5.3's loops: the
//! product is `(ar br - ai bi, ar bi + ai br)`, the quotient is Smith's method as `loops.c.src` writes it (a real
//! divisor `d` therefore multiplies by `1 / d`), the square root is `npy_csqrt`'s (Algorithm 312).
//!
//! numpy takes `tan`, `sin`, `cos`, `hypot` and `pow` from the platform here and the Studio takes them from
//! `dmath`, so a coefficient can differ from scipy's in its last bits; the reference designs agree to a relative
//! 1e-12 or better. Exact zeros (a band-pass section's middle `b`) come out exact.

use std::ops::{Add, Div, Mul, Neg, Sub};

use bf_core::dmath;

use super::iir::Sos;

/// The kind of filter: scipy's `btype` `'lowpass'` (`'lp'`, `'low'`), `'highpass'` (`'hp'`, `'high'`) and
/// `'bandpass'` (`'bp'`, `'band'`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterKind {
    Lp,
    Hp,
    Bp,
}

/// A complex number with numpy's `complex128` arithmetic.
#[derive(Clone, Copy, Debug, PartialEq)]
struct C {
    re: f64,
    im: f64,
}

impl C {
    const ZERO: C = C { re: 0.0, im: 0.0 };

    const fn new(re: f64, im: f64) -> C {
        C { re, im }
    }

    /// A real number as numpy promotes it: `x + 0j`.
    const fn real(re: f64) -> C {
        C { re, im: 0.0 }
    }

    /// `np.isreal`: the imaginary part is zero.
    fn is_real(self) -> bool {
        self.im == 0.0
    }

    fn conj(self) -> C {
        C::new(self.re, -self.im)
    }

    /// `np.abs`: `hypot(re, im)`.
    fn abs(self) -> f64 {
        dmath::hypot(self.re, self.im)
    }

    /// `np.square` on `complex128`.
    fn square(self) -> C {
        C::new(
            self.re * self.re - self.im * self.im,
            self.re * self.im + self.im * self.re,
        )
    }

    /// `np.sqrt` on `complex128` for a finite argument (`npy_csqrt`, Algorithm 312 of CACM; its overflow scaling
    /// and non-finite cases are left out: a design's poles are finite and far from the largest float).
    fn sqrt(self) -> C {
        let (a, b) = (self.re, self.im);
        if a == 0.0 && b == 0.0 {
            return C::new(0.0, b);
        }
        if a >= 0.0 {
            let t = ((a + dmath::hypot(a, b)) * 0.5).sqrt();
            C::new(t, b / (2.0 * t))
        } else {
            let t = ((-a + dmath::hypot(a, b)) * 0.5).sqrt();
            C::new(b.abs() / (2.0 * t), t.copysign(b))
        }
    }
}

impl Add for C {
    type Output = C;
    fn add(self, o: C) -> C {
        C::new(self.re + o.re, self.im + o.im)
    }
}

impl Sub for C {
    type Output = C;
    fn sub(self, o: C) -> C {
        C::new(self.re - o.re, self.im - o.im)
    }
}

impl Neg for C {
    type Output = C;
    fn neg(self) -> C {
        C::new(-self.re, -self.im)
    }
}

impl Mul for C {
    type Output = C;
    fn mul(self, o: C) -> C {
        C::new(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }
}

impl Div for C {
    type Output = C;
    /// numpy's `complex128` division (`loops.c.src`, Smith's method).
    fn div(self, o: C) -> C {
        let (in1r, in1i, in2r, in2i) = (self.re, self.im, o.re, o.im);
        let (in2r_abs, in2i_abs) = (in2r.abs(), in2i.abs());
        if in2r_abs >= in2i_abs {
            if in2r_abs == 0.0 && in2i_abs == 0.0 {
                // Division by zero gives a complex infinity or NaN.
                return C::new(in1r / in2r_abs, in1i / in2r_abs);
            }
            let rat = in2i / in2r;
            let scl = 1.0 / (in2r + in2i * rat);
            C::new((in1r + in1i * rat) * scl, (in1i - in1r * rat) * scl)
        } else {
            let rat = in2r / in2i;
            let scl = 1.0 / (in2i + in2r * rat);
            C::new((in1r * rat + in1i) * scl, (in1i * rat - in1r) * scl)
        }
    }
}

/// `np.prod` of a complex array: from one, left to right.
fn prod(v: impl IntoIterator<Item = C>) -> C {
    v.into_iter().fold(C::real(1.0), |acc, c| acc * c)
}

/// Zeros, poles and gain.
type Zpk = (Vec<C>, Vec<C>, f64);

/// `buttap(N)`: the poles of the analog prototype, `-exp(1j * pi * m / (2 * N))` for `m = -N+1, -N+3, .., N-1`.
/// It has no zeros and its gain is 1.
fn buttap(n: usize) -> Vec<C> {
    (0..n)
        .map(|i| {
            let m = (2 * i) as f64 - (n as f64 - 1.0);
            // `1j * pi` is `(0, pi)`; numpy multiplies it by `m + 0j` and divides by `2N + 0j`.
            let w = C::new(0.0, dmath::PI) * C::real(m) / C::real((2 * n) as f64);
            let e = dmath::exp(w.re);
            -C::new(e * dmath::cos(w.im), e * dmath::sin(w.im))
        })
        .collect()
}

/// `lp2lp_zpk`: every point scaled by `wo`.
fn lp2lp_zpk((z, p, k): Zpk, wo: f64) -> Zpk {
    let degree = relative_degree(&z, &p);
    let scale = |v: Vec<C>| -> Vec<C> { v.into_iter().map(|c| C::real(wo) * c).collect() };
    (scale(z), scale(p), k * dmath::pow(wo, degree as f64))
}

/// `lp2hp_zpk`: every point turned inside out about `wo`; the zeros at infinity land on the origin.
fn lp2hp_zpk((z, p, k): Zpk, wo: f64) -> Zpk {
    let degree = relative_degree(&z, &p);
    let mut z_hp: Vec<C> = z.iter().map(|&c| C::real(wo) / c).collect();
    let p_hp: Vec<C> = p.iter().map(|&c| C::real(wo) / c).collect();
    z_hp.extend(std::iter::repeat_n(C::ZERO, degree));
    let k_hp = k * (prod(z.iter().map(|&c| -c)) / prod(p.iter().map(|&c| -c))).re;
    (z_hp, p_hp, k_hp)
}

/// `lp2bp_zpk`: every point scaled to the bandwidth, then split to `+wo` and `-wo`; half the zeros at infinity
/// land on the origin.
fn lp2bp_zpk((z, p, k): Zpk, wo: f64, bw: f64) -> Zpk {
    let degree = relative_degree(&z, &p);
    let split = |v: &[C]| -> Vec<C> {
        let lp: Vec<C> = v.iter().map(|&c| c * C::real(bw) / C::real(2.0)).collect();
        let root = |c: C| (c.square() - C::real(wo * wo)).sqrt();
        lp.iter()
            .map(|&c| c + root(c))
            .chain(lp.iter().map(|&c| c - root(c)))
            .collect()
    };
    let mut z_bp = split(&z);
    let p_bp = split(&p);
    z_bp.extend(std::iter::repeat_n(C::ZERO, degree));
    (z_bp, p_bp, k * dmath::pow(bw, degree as f64))
}

/// `bilinear_zpk` at the sample rate `fs` (Tustin's method); the zeros at infinity land on the Nyquist frequency.
fn bilinear_zpk((z, p, k): Zpk, fs: f64) -> Zpk {
    let degree = relative_degree(&z, &p);
    let fs2 = C::real(2.0 * fs);
    let mut z_z: Vec<C> = z.iter().map(|&c| (fs2 + c) / (fs2 - c)).collect();
    let p_z: Vec<C> = p.iter().map(|&c| (fs2 + c) / (fs2 - c)).collect();
    z_z.extend(std::iter::repeat_n(C::real(-1.0), degree));
    let k_z = k * (prod(z.iter().map(|&c| fs2 - c)) / prod(p.iter().map(|&c| fs2 - c))).re;
    (z_z, p_z, k_z)
}

/// `_relative_degree`: poles minus zeros.
///
/// # Panics
/// Where scipy raises: more zeros than poles.
fn relative_degree(z: &[C], p: &[C]) -> usize {
    assert!(
        p.len() >= z.len(),
        "Improper transfer function. Must have at least as many poles as zeros."
    );
    p.len() - z.len()
}

/// `iirfilter(N, Wn, btype, ftype='butter', fs=fs)` up to the digital zeros, poles and gain.
fn butter_zpk(kind: FilterKind, freqs: &[f64], order: usize, fs: f64) -> Zpk {
    match kind {
        FilterKind::Lp | FilterKind::Hp => assert!(
            freqs.len() == 1,
            "Must specify a single critical frequency Wn for lowpass or highpass filter"
        ),
        FilterKind::Bp => assert!(
            freqs.len() == 2,
            "Wn must specify start and stop frequencies for bandpass or bandstop filter"
        ),
    }
    let wn: Vec<f64> = freqs.iter().map(|f| f / (fs / 2.0)).collect();
    assert!(
        !wn.iter().any(|&w| w <= 0.0),
        "filter critical frequencies must be greater than 0"
    );
    assert!(wn.len() == 1 || wn[0] < wn[1], "Wn[0] must be less than Wn[1]");
    let prototype: Zpk = (Vec::new(), buttap(order), 1.0);
    assert!(
        !wn.iter().any(|&w| w >= 1.0),
        "Digital filter critical frequencies must be 0 < Wn < fs/2 (fs={fs} -> fs/2={})",
        fs / 2.0
    );
    // Pre-warp for the bilinear transform, which scipy runs at fs = 2.
    let fs = 2.0;
    let warped: Vec<f64> = wn.iter().map(|&w| 2.0 * fs * dmath::tan(dmath::PI * w / fs)).collect();
    let analog = match kind {
        FilterKind::Lp => lp2lp_zpk(prototype, warped[0]),
        FilterKind::Hp => lp2hp_zpk(prototype, warped[0]),
        FilterKind::Bp => {
            let bw = warped[1] - warped[0];
            let wo = (warped[0] * warped[1]).sqrt();
            lp2bp_zpk(prototype, wo, bw)
        }
    };
    bilinear_zpk(analog, fs)
}

/// `np.convolve(a, v)` (mode `'full'`) on complex arrays: each output is a dot product added from zero, in the
/// order of the longer operand.
fn convolve(a: &[C], v: &[C]) -> Vec<C> {
    let (a, v) = if v.len() > a.len() { (v, a) } else { (a, v) };
    let (n1, n2) = (a.len(), v.len());
    (0..n1 + n2 - 1)
        .map(|k| {
            let mut sum = C::ZERO;
            for j in k.saturating_sub(n2 - 1)..=k.min(n1 - 1) {
                sum = sum + a[j] * v[k - j];
            }
            sum
        })
        .collect()
}

/// `np.poly`: the coefficients of the polynomial with these roots, highest power first. scipy keeps the real
/// part when the roots come in conjugate pairs.
///
/// # Panics
/// If they do not (the polynomial would have complex coefficients, which no filter here has).
fn poly(roots: &[C]) -> Vec<f64> {
    let mut a = vec![C::real(1.0)];
    for &root in roots {
        a = convolve(&a, &[C::real(1.0), -root]);
    }
    let sorted = |sign: f64| -> Vec<f64> {
        let mut im: Vec<f64> = roots.iter().map(|c| sign * c.im).collect();
        im.sort_by(|x, y| x.partial_cmp(y).expect("a root is NaN"));
        im
    };
    assert!(sorted(1.0) == sorted(-1.0), "the roots are not in conjugate pairs");
    a.iter().map(|c| c.re).collect()
}

/// `zpk2tf`: `b = k * poly(z)`, `a = poly(p)`.
fn zpk2tf(z: &[C], p: &[C], k: f64) -> (Vec<f64>, Vec<f64>) {
    (poly(z).iter().map(|c| k * c).collect(), poly(p))
}

/// `_cplxreal`: the complex members (one of each conjugate pair, the one with the positive imaginary part,
/// ordered by real part) and the real members (ascending).
///
/// # Panics
/// Where scipy raises: a complex value has no matching conjugate.
fn cplxreal(z: &[C]) -> (Vec<C>, Vec<f64>) {
    if z.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let tol = 100.0 * f64::EPSILON;
    let order = |x: f64, y: f64| x.partial_cmp(&y).expect("a pole or zero is NaN");
    // np.lexsort((abs(z.imag), z.real)): by real part, then by the size of the imaginary part; stable.
    let mut z = z.to_vec();
    z.sort_by(|a, b| order(a.re, b.re).then(order(a.im.abs(), b.im.abs())));
    let is_real = |c: &C| c.im.abs() <= tol * c.abs();
    let zr: Vec<f64> = z.iter().filter(|c| is_real(c)).map(|c| c.re).collect();
    if zr.len() == z.len() {
        return (Vec::new(), zr);
    }
    let complex: Vec<C> = z.iter().copied().filter(|c| !is_real(c)).collect();
    let mut zp: Vec<C> = complex.iter().copied().filter(|c| c.im > 0.0).collect();
    let mut zn: Vec<C> = complex.iter().copied().filter(|c| c.im < 0.0).collect();
    const NO_MATCH: &str = "Array contains complex value with no matching conjugate.";
    assert!(zp.len() == zn.len(), "{NO_MATCH}");
    // Runs of (nearly) the same real part are put in order of their imaginary parts.
    let same_real: Vec<bool> = zp.windows(2).map(|w| w[1].re - w[0].re <= tol * w[0].abs()).collect();
    let mut start = 0;
    while start < same_real.len() {
        if !same_real[start] {
            start += 1;
            continue;
        }
        let mut stop = start;
        while stop < same_real.len() && same_real[stop] {
            stop += 1;
        }
        // The run covers the members start..=stop.
        for chunk in [&mut zp[start..=stop], &mut zn[start..=stop]] {
            chunk.sort_by(|a, b| order(a.im.abs(), b.im.abs()));
        }
        start = stop + 1;
    }
    assert!(
        !zp.iter().zip(&zn).any(|(&p, &n)| (p - n.conj()).abs() > tol * n.abs()),
        "{NO_MATCH}"
    );
    // The two halves of a pair, averaged.
    let zc = zp
        .iter()
        .zip(&zn)
        .map(|(&p, &n)| (p + n.conj()) / C::real(2.0))
        .collect();
    (zc, zr)
}

/// Which members `nearest` may pick.
#[derive(Clone, Copy, PartialEq)]
enum Which {
    Real,
    Complex,
    Any,
}

/// `_nearest_real_complex_idx`: the index of the member of `from` closest to `to`, among the real ones, the
/// complex ones or all of them. Of two at the same distance the earlier wins (numpy's sort may pick either; in a
/// Butterworth design such a tie is between equal zeros).
fn nearest(from: &[C], to: C, which: Which) -> usize {
    let distance: Vec<f64> = from.iter().map(|&c| (c - to).abs()).collect();
    let mut order: Vec<usize> = (0..from.len()).collect();
    order.sort_by(|&i, &j| distance[i].partial_cmp(&distance[j]).expect("a distance is NaN"));
    order
        .into_iter()
        .find(|&i| match which {
            Which::Any => true,
            Which::Real => from[i].is_real(),
            Which::Complex => !from[i].is_real(),
        })
        .expect("no zero of the wanted kind is left to pair with the pole")
}

/// The digital `idx_worst`: the index of the pole closest to the unit circle (the first of equals).
fn idx_worst(p: &[C]) -> usize {
    let mut best = 0;
    for i in 1..p.len() {
        if (1.0 - p[i].abs()).abs() < (1.0 - p[best].abs()).abs() {
            best = i;
        }
    }
    best
}

/// `_single_zpksos`: one section from up to two zeros and two poles, with gain 1.
fn single_zpksos(z: &[C], p: &[C]) -> [f64; 6] {
    let (b, a) = zpk2tf(z, p, 1.0);
    let mut sos = [0.0; 6];
    sos[3 - b.len()..3].copy_from_slice(&b);
    sos[6 - a.len()..6].copy_from_slice(&a);
    sos
}

/// `zpk2sos(z, p, k)` for a digital filter (pairing `'nearest'`): the pole closest to the unit circle goes into
/// the last section with the zeros closest to it, and so on back to the first section, which takes the gain.
fn zpk2sos(z: Vec<C>, p: Vec<C>, k: f64) -> Sos {
    if z.is_empty() && p.is_empty() {
        return vec![[k, 0.0, 0.0, 1.0, 0.0, 0.0]];
    }
    let (mut z, mut p) = (z, p);
    // The same number of poles and zeros, and an even number of each.
    while p.len() < z.len() {
        p.push(C::ZERO);
    }
    while z.len() < p.len() {
        z.push(C::ZERO);
    }
    let n_sections = p.len().div_ceil(2);
    if p.len() % 2 == 1 {
        p.push(C::ZERO);
        z.push(C::ZERO);
    }
    // One member of each conjugate pair, then the real ones.
    let join = |(complex, real): (Vec<C>, Vec<f64>)| -> Vec<C> {
        complex.into_iter().chain(real.into_iter().map(C::real)).collect()
    };
    let mut z = join(cplxreal(&z));
    let mut p = join(cplxreal(&p));
    let reals = |v: &[C]| v.iter().filter(|c| c.is_real()).count();

    let mut sos = vec![[0.0; 6]; n_sections];
    for section in sos.iter_mut().rev() {
        // The next "worst" pole.
        let p1 = p.remove(idx_worst(&p));
        if p1.is_real() && reals(&p) == 0 {
            // The last real pole left: a first-order section, with the nearest real zero.
            let z1 = z.remove(nearest(&z, p1, Which::Real));
            *section = single_zpksos(&[z1, C::ZERO], &[p1, C::ZERO]);
        } else if p.len() + 1 == z.len() && !p1.is_real() && reals(&p) == 1 && reals(&z) == 1 {
            // One real pole and one real zero are left for each other: this pole must take a complex zero.
            let z1 = z.remove(nearest(&z, p1, Which::Complex));
            *section = single_zpksos(&[z1, z1.conj()], &[p1, p1.conj()]);
        } else {
            let p2 = if p1.is_real() {
                let real_at: Vec<usize> = (0..p.len()).filter(|&i| p[i].is_real()).collect();
                let real: Vec<C> = real_at.iter().map(|&i| p[i]).collect();
                p.remove(real_at[idx_worst(&real)])
            } else {
                p1.conj()
            };
            if z.is_empty() {
                *section = single_zpksos(&[], &[p1, p2]);
            } else {
                let z1 = z.remove(nearest(&z, p1, Which::Any));
                *section = if !z1.is_real() {
                    single_zpksos(&[z1, z1.conj()], &[p1, p2])
                } else if z.is_empty() {
                    single_zpksos(&[z1], &[p1, p2])
                } else {
                    let z2 = z.remove(nearest(&z, p1, Which::Real));
                    single_zpksos(&[z1, z2], &[p1, p2])
                };
            }
        }
    }
    assert!(p.is_empty() && z.is_empty(), "zpk2sos left a pole or a zero unpaired");
    // The gain goes into the first section.
    for b in &mut sos[0][..3] {
        *b *= k;
    }
    sos
}

/// `scipy.signal.butter(order, freqs, kind, fs=fs, output='sos')`: a digital Butterworth filter as second-order
/// sections. `freqs` holds one cutoff for a low- or high-pass and two for a band-pass, in the unit of `fs`; pass
/// `fs = 2.0` for cutoffs given as fractions of the Nyquist frequency (Python's `fs=None`). A band-pass has
/// `order` sections, the others `(order + 1) / 2`.
///
/// # Panics
/// Where scipy raises: the wrong number of cutoffs, a cutoff at or below 0 or at or above `fs / 2`, band edges
/// not ascending.
pub fn butter_sos(kind: FilterKind, freqs: &[f64], order: usize, fs: f64) -> Sos {
    let (z, p, k) = butter_zpk(kind, freqs, order, fs);
    zpk2sos(z, p, k)
}

/// `scipy.signal.butter(order, freqs, kind, fs=fs)` in its default output form: the transfer function's
/// numerator `b` and denominator `a`, highest power first. Arguments and panics as [`butter_sos`].
pub fn butter_ba(kind: FilterKind, freqs: &[f64], order: usize, fs: f64) -> (Vec<f64>, Vec<f64>) {
    let (z, p, k) = butter_zpk(kind, freqs, order, fs);
    zpk2tf(&z, &p, k)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-14 * b.abs().max(1.0)
    }

    #[test]
    fn complex_division_is_numpys() {
        // A real divisor multiplies by its reciprocal: 1 / 6 is not exact, so this is not `x / 6`.
        let q = C::new(1.0, 5.0) / C::real(6.0);
        assert_eq!(q, C::new(1.0 * (1.0 / 6.0), 5.0 * (1.0 / 6.0)));
        // Both branches of Smith's method on (1 + 2j) / (3 + 4j) and (1 + 2j) / (4 + 3j) = 0.4 + 0.2j.
        let q = C::new(1.0, 2.0) / C::new(3.0, 4.0);
        assert!(close(q.re, 0.44) && close(q.im, 0.08), "{q:?}");
        let q = C::new(1.0, 2.0) / C::new(4.0, 3.0);
        assert!(close(q.re, 0.4) && close(q.im, 0.2), "{q:?}");
        let s = C::new(-3.0, -4.0).sqrt();
        assert!(close(s.re, 1.0) && close(s.im, -2.0), "{s:?}");
        let s = C::new(3.0, 4.0).sqrt();
        assert!(close(s.re, 2.0) && close(s.im, 1.0), "{s:?}");
    }

    #[test]
    fn the_prototype_has_conjugate_poles_on_the_unit_circle() {
        for n in 1..=8 {
            let p = buttap(n);
            assert_eq!(p.len(), n);
            for (i, c) in p.iter().enumerate() {
                assert!(close(c.abs(), 1.0) && c.re < 0.0, "{n}: {c:?}");
                assert_eq!(*c, p[n - 1 - i].conj(), "order {n}, pole {i}");
            }
            if n % 2 == 1 {
                assert_eq!(
                    p[n / 2],
                    C::new(-1.0, -0.0),
                    "the middle pole of an odd order is exactly real"
                );
            }
        }
    }

    #[test]
    fn section_counts_and_exact_zeros() {
        let fs = 48000.0;
        let lp = butter_sos(FilterKind::Lp, &[1000.0], 2, fs);
        assert_eq!(lp.len(), 1);
        // Two zeros at -1: b is b0 * (1, 2, 1).
        assert_eq!((lp[0][1], lp[0][2], lp[0][3]), (2.0 * lp[0][0], lp[0][0], 1.0));
        let hp = butter_sos(FilterKind::Hp, &[1000.0], 2, fs);
        assert_eq!((hp[0][1], hp[0][2], hp[0][3]), (-2.0 * hp[0][0], hp[0][0], 1.0));
        let bp = butter_sos(FilterKind::Bp, &[300.0, 3000.0], 2, fs);
        assert_eq!(bp.len(), 2);
        assert_eq!(butter_sos(FilterKind::Bp, &[300.0, 3000.0], 4, fs).len(), 4);
        assert_eq!(butter_sos(FilterKind::Lp, &[1000.0], 5, fs).len(), 3);
        // Order 0: a gain of one.
        assert_eq!(
            butter_sos(FilterKind::Lp, &[1000.0], 0, fs),
            vec![[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]]
        );
    }

    #[test]
    fn gain_is_one_in_the_pass_band() {
        // A low-pass passes 0 Hz: sum(b) / sum(a) of the cascade is 1. A high-pass passes the Nyquist frequency.
        let at = |sos: &Sos, z_inv: f64| -> f64 {
            sos.iter()
                .map(|s| (s[0] + s[1] * z_inv + s[2]) / (s[3] + s[4] * z_inv + s[5]))
                .product()
        };
        for order in 1..=6 {
            let lp = butter_sos(FilterKind::Lp, &[2500.0], order, 48000.0);
            assert!(
                (at(&lp, 1.0) - 1.0).abs() < 1e-9,
                "low-pass order {order}: {}",
                at(&lp, 1.0)
            );
            let hp = butter_sos(FilterKind::Hp, &[2500.0], order, 48000.0);
            assert!(
                (at(&hp, -1.0) - 1.0).abs() < 1e-9,
                "high-pass order {order}: {}",
                at(&hp, -1.0)
            );
            assert!(
                at(&hp, 1.0).abs() < 1e-9 && at(&lp, -1.0).abs() < 1e-9,
                "order {order}: the stop band"
            );
        }
        let (b, a) = butter_ba(FilterKind::Lp, &[0.25], 3, 2.0);
        assert_eq!((b.len(), a.len()), (4, 4));
        assert!((b.iter().sum::<f64>() / a.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }

    #[test]
    #[should_panic(expected = "Digital filter critical frequencies must be 0 < Wn < fs/2 (fs=48000 -> fs/2=24000)")]
    fn a_cutoff_at_the_nyquist_frequency_is_refused() {
        butter_sos(FilterKind::Lp, &[24000.0], 2, 48000.0);
    }

    #[test]
    #[should_panic(expected = "Wn[0] must be less than Wn[1]")]
    fn band_edges_must_ascend() {
        butter_sos(FilterKind::Bp, &[3000.0, 300.0], 2, 48000.0);
    }

    #[test]
    #[should_panic(expected = "Must specify a single critical frequency")]
    fn a_low_pass_takes_one_cutoff() {
        butter_sos(FilterKind::Lp, &[300.0, 3000.0], 2, 48000.0);
    }
}
