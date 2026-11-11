//! The FFT convolution under the lens blur: `scipy.signal.fftconvolve(a, kernel, mode='same')` for a `float32`
//! plane and a `float64` kernel, as `reelkit.lens.dof` calls it.
//!
//! scipy 1.18.1 computes it as `irfftn(rfftn(a, s) * rfftn(kernel, s), s)`, cropped, with `s` the next sizes that
//! are products of 2, 3 and 5. Its FFT is the `ducc0` library (`scipy/subprojects/duccfft`), and **the transform
//! of the `float32` plane runs in `float32`**: its rounding noise is about 1e-7 of the picture's values, a
//! thousand times above the bar the lens blur is held to. So this module is not "an FFT" but that FFT: the same
//! plans, the same passes, the same order of operations in the same precision, ported from `fft1d_impl.h`,
//! `fftnd_impl.h` and `unity_roots.h`. Against scipy (`studio/reference/picture_kit/fftconv.npz`) the `float32`
//! transform is equal to the bit in every one of the 55,194 reference values, and a whole convolution agrees to
//! about 1e-15 of the plane's largest value.
//!
//! What is ported, and its limits:
//!
//! - Lengths up to 10,000 that are products of 2, 3 and 5 only (what `scipy.fft.next_fast_len(n, real=True)`
//!   gives, so every length `fftconvolve` uses). The radix 7 and 11 passes, the generic pass and Bluestein's
//!   algorithm are not here, nor the routes `ducc0` takes above 10,000 samples; [`rfftn_f32`] and its siblings
//!   panic for such a length.
//! - Two-dimensional arrays, transformed as `ducc0` does: the rows first (real to half-complex), then the columns
//!   (complex). `ducc0` transforms columns four at a time (two in `float64`) with one plan. In `float32` the
//!   columns left over, when the length is above 300 and a multiple of 4, get a differently built plan
//!   (`cfftp_vecpass`), and the same is done here, since the two plans round differently. In `float64` there is
//!   no such plan on the reference build (it needs a SIMD type of four values, and the build is SSE2): a
//!   left-over column is transformed like every other.
//! - Where the port cannot be bit-identical: the twiddle factors come from `sin` and `cos` (`bf_core::dmath` here,
//!   the C library there; a twiddle can differ in the last bit of its double, which a `float32` twiddle almost
//!   never sees), and numpy multiplies the two spectra with fused operations. Both are far below the bar. What
//!   it means for each precision: the `float32` forward transform is equal to scipy's to the bit on every
//!   reference case. A `float64` transform, forward or backward, agrees to about 1e-16 of the array's largest
//!   value and comes out with no difference at all only where the twiddles happen to coincide (of the
//!   reference cases, 9 of the 15 `irfftn` ones). The backward passes run only in `float64` (`irfftn` of the
//!   product of the spectra), so those `irfftn` cases are the only place a backward pass is checked exactly;
//!   the `fftconvolve` and `dof` cases hold them to the bar, not to the bit.
//! - Speed: every call builds its plans again (the factors and twiddle tables of both lengths). Only the
//!   kernel's spectrum is kept between calls ([`Convolver`]). A later speed task may keep the plans per length.

use crate::img::Img;
use bf_core::dmath;
use std::ops::{Add, Mul, Neg, Sub};

/// A complex number as `[real, imaginary]`, the form the public functions take and return.
pub type Cpx<T> = [T; 2];

/// The longest transform this module follows `ducc0` for. Above 10,000 samples `ducc0` splits a plan's passes
/// into packets and transforms long columns one at a time, which is not ported: a longer transform panics.
pub const MAX_LEN: usize = 10_000;

/// The float type of a transform: `f32` or `f64`.
trait Real:
    Copy + Default + PartialEq + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Neg<Output = Self>
{
    /// How many columns `ducc0` transforms at a time in this precision (its SIMD width on the reference build).
    const LANES: usize;
    /// Whether the reference build has a SIMD type of four values in this precision (`simd_exists<Tfs, 4>`). It is
    /// an SSE2 build: four `float`s fit a register, four `double`s do not. Only where this holds does `ducc0`
    /// build the split plan for a single transform ([`CPlan::split`]).
    const FOUR_LANES: bool;
    /// A double constant or twiddle in this precision (`Tfs(x)`).
    fn of(x: f64) -> Self;
}

impl Real for f32 {
    const LANES: usize = 4;
    const FOUR_LANES: bool = true;
    fn of(x: f64) -> f32 {
        x as f32
    }
}

impl Real for f64 {
    const LANES: usize = 2;
    const FOUR_LANES: bool = false;
    fn of(x: f64) -> f64 {
        x
    }
}

/// `Cmplx<T>` of `ducc0/math/cmplx.h`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct C<T> {
    r: T,
    i: T,
}

impl<T: Real> C<T> {
    fn new(r: T, i: T) -> C<T> {
        C { r, i }
    }

    /// `special_mul<fwd>`: times the conjugate of `w` going forward, times `w` going back.
    fn tw(self, w: C<T>, fwd: bool) -> C<T> {
        if fwd {
            C::new(self.r * w.r + self.i * w.i, self.i * w.r - self.r * w.i)
        } else {
            C::new(self.r * w.r - self.i * w.i, self.r * w.i + self.i * w.r)
        }
    }

    /// `ROTX90<fwd>`: a quarter turn, clockwise going forward.
    fn rot90(self, fwd: bool) -> C<T> {
        if fwd {
            C::new(self.i, -self.r)
        } else {
            C::new(-self.i, self.r)
        }
    }

    /// `ROTX45<fwd>` of the radix 8 pass.
    fn rot45(self, fwd: bool, hsqt2: T) -> C<T> {
        if fwd {
            C::new(hsqt2 * (self.r + self.i), hsqt2 * (self.i - self.r))
        } else {
            C::new(hsqt2 * (self.r - self.i), hsqt2 * (self.i + self.r))
        }
    }

    /// `ROTX135<fwd>` of the radix 8 pass.
    fn rot135(self, fwd: bool, hsqt2: T) -> C<T> {
        if fwd {
            C::new(hsqt2 * (self.i - self.r), hsqt2 * (-self.r - self.i))
        } else {
            C::new(hsqt2 * (-self.r - self.i), hsqt2 * (self.r - self.i))
        }
    }

    fn scale(self, k: T) -> C<T> {
        C::new(self.r * k, self.i * k)
    }
}

impl<T: Real> Add for C<T> {
    type Output = C<T>;
    fn add(self, o: C<T>) -> C<T> {
        C::new(self.r + o.r, self.i + o.i)
    }
}

impl<T: Real> Sub for C<T> {
    type Output = C<T>;
    fn sub(self, o: C<T>) -> C<T> {
        C::new(self.r - o.r, self.i - o.i)
    }
}

/// `PM(a, b, c, d)`: `(c + d, c - d)`.
fn pm<V: Add<Output = V> + Sub<Output = V> + Copy>(c: V, d: V) -> (V, V) {
    (c + d, c - d)
}

/// `MULPM(a, b, c, d, e, f)`: `(c e + d f, c f - d e)`.
fn mulpm<T: Real>(c: T, d: T, e: T, f: T) -> (T, T) {
    (c * e + d * f, c * f - d * e)
}

// The constants of the passes, as the C++ source spells them (long double literals rounded to the float type; for
// each of them the rounding via 80 bits and the direct one agree).
const SQRT3_HALF: f64 = 0.866_025_403_784_438_6;
const COS_2PI_5: f64 = 0.309_016_994_374_947_45;
const SIN_2PI_5: f64 = 0.951_056_516_295_153_5;
const COS_4PI_5: f64 = -0.809_016_994_374_947_5;
const SIN_4PI_5: f64 = 0.587_785_252_292_473_1;
const HALF_SQRT2: f64 = std::f64::consts::FRAC_1_SQRT_2;
const SQRT2: f64 = std::f64::consts::SQRT_2;

// ---------------------------------------------------------------- roots of unity
/// `UnityRoots<T, Cmplx<T>>(n)`: the `n`-th roots of unity, each the product of an entry of two short tables of
/// doubles. The split into tables is part of the value: the product is rounded, so the same root from a table
/// built for another `n` can differ in its last bit.
struct Roots {
    n: usize,
    mask: usize,
    shift: u32,
    v1: Vec<(f64, f64)>,
    v2: Vec<(f64, f64)>,
}

/// `Thigh(0.25L * pi / n)`: a quarter of pi over `n`. The C++ divides the 80-bit pi by `n` in the x87 unit, which
/// in a Python process on Windows rounds to 53 bits: one rounding of the exact quotient of the 80-bit constant.
/// Done here in whole numbers.
fn quarter_pi_over(n: usize) -> f64 {
    const PI_80: u128 = 0xC90F_DAA2_2168_C235; // pi = PI_80 * 2^-62
    let (num, n) = (PI_80 << 64, n as u128);
    // pi / 4 / n = (num / n) * 2^-128
    let (q, rest) = (num / n, num % n);
    let drop = (128 - q.leading_zeros()) - 53;
    let mut mant = q >> drop;
    let (low, half) = (q & ((1u128 << drop) - 1), 1u128 << (drop - 1));
    if low > half || (low == half && (rest != 0 || mant & 1 == 1)) {
        mant += 1;
    }
    // mant * 2^(drop - 128): a 53-bit whole number times a power of two, exact.
    let power = f64::from_bits(((1023 + drop as i64 - 128) as u64) << 52);
    mant as f64 * power
}

impl Roots {
    fn new(n: usize) -> Roots {
        let ang = quarter_pi_over(n);
        let nval = (n + 2) / 2;
        let mut shift = 1u32;
        while (1usize << shift) * (1usize << shift) < nval {
            shift += 1;
        }
        let mask = (1usize << shift) - 1;
        let mut v1 = vec![(1.0, 0.0); mask + 1];
        for (i, v) in v1.iter_mut().enumerate().skip(1) {
            *v = Roots::calc(i, n, ang);
        }
        let mut v2 = vec![(1.0, 0.0); (nval + mask) / (mask + 1)];
        for (i, v) in v2.iter_mut().enumerate().skip(1) {
            *v = Roots::calc(i * (mask + 1), n, ang);
        }
        Roots { n, mask, shift, v1, v2 }
    }

    /// `calc(x, n, ang)`: `exp(2 pi i x / n)` from an angle folded into the first octant.
    fn calc(x: usize, n: usize, ang: f64) -> (f64, f64) {
        let (cos, sin) = (
            |k: usize| dmath::cos(k as f64 * ang),
            |k: usize| dmath::sin(k as f64 * ang),
        );
        let mut x = x << 3;
        if x < 4 * n {
            if x < 2 * n {
                if x < n {
                    return (cos(x), sin(x));
                }
                return (sin(2 * n - x), cos(2 * n - x));
            }
            x -= 2 * n;
            if x < n {
                return (-sin(x), cos(x));
            }
            (-cos(2 * n - x), sin(2 * n - x))
        } else {
            x = 8 * n - x;
            if x < 2 * n {
                if x < n {
                    return (cos(x), -sin(x));
                }
                return (sin(2 * n - x), -cos(2 * n - x));
            }
            x -= 2 * n;
            if x < n {
                return (-sin(x), -cos(x));
            }
            (-cos(2 * n - x), -sin(2 * n - x))
        }
    }

    /// `(*roots)[idx]` in the transform's precision.
    fn get<T: Real>(&self, idx: usize) -> C<T> {
        let part = |idx: usize| {
            let (x1, x2) = (self.v1[idx & self.mask], self.v2[idx >> self.shift]);
            (x1.0 * x2.0 - x1.1 * x2.1, x1.0 * x2.1 + x1.1 * x2.0)
        };
        if 2 * idx <= self.n {
            let (r, i) = part(idx);
            C::new(T::of(r), T::of(i))
        } else {
            let (r, i) = part(self.n - idx);
            C::new(T::of(r), -T::of(i))
        }
    }
}

fn unsupported(factor: usize) -> ! {
    panic!(
        "fft: a transform length with the factor {factor} is not ported (lengths are products of 2, 3 and 5, as \
         scipy.fft.next_fast_len(n, real=True) gives)"
    )
}

// ---------------------------------------------------------------- complex passes
/// One radix pass of a complex transform (`cfftp2`, `cfftp3`, `cfftp4`, `cfftp5`, `cfftp8`): `ip` the radix,
/// `l1` the product of the radices before it, `ido` the product of those after it.
struct CPass<T> {
    ip: usize,
    l1: usize,
    ido: usize,
    wa: Vec<C<T>>,
}

impl<T: Real> CPass<T> {
    fn new(l1: usize, ido: usize, ip: usize, roots: &Roots) -> CPass<T> {
        if ![2, 3, 4, 5, 8].contains(&ip) {
            unsupported(ip);
        }
        let n = ip * l1 * ido;
        let rfct = roots.n / n;
        assert_eq!(roots.n, n * rfct, "fft: the roots do not fit the pass");
        let mut wa = vec![C::default(); (ip - 1) * (ido - 1)];
        for i in 1..ido {
            for j in 1..ip {
                wa[(j - 1) + (i - 1) * (ip - 1)] = roots.get(rfct * j * l1 * i);
            }
        }
        CPass { ip, l1, ido, wa }
    }

    /// `WA(x, i)`.
    fn wa(&self, x: usize, i: usize) -> C<T> {
        self.wa[x + (i - 1) * (self.ip - 1)]
    }

    /// Runs the pass on `cc`. Returns true if the result is in `ch`, false if it was done in place.
    fn exec(&self, fwd: bool, cc: &mut [C<T>], ch: &mut [C<T>]) -> bool {
        match self.ip {
            2 => self.pass2(fwd, cc, ch),
            3 => self.pass3(fwd, cc, ch),
            4 => self.pass4(fwd, cc, ch),
            5 => self.pass5(fwd, cc, ch),
            _ => return self.pass8(fwd, cc, ch),
        }
        true
    }

    /// The index of `CC(i, m, k)`: input element `i` of sub-transform `m` in block `k`.
    fn cc(&self, i: usize, m: usize, k: usize) -> usize {
        i + self.ido * (m + self.ip * k)
    }

    /// The index of `CH(i, k, m)`.
    fn ch(&self, i: usize, k: usize, m: usize) -> usize {
        i + self.ido * (k + self.l1 * m)
    }

    fn pass2(&self, fwd: bool, cc: &[C<T>], ch: &mut [C<T>]) {
        for k in 0..self.l1 {
            for i in 0..self.ido {
                let (a, b) = (cc[self.cc(i, 0, k)], cc[self.cc(i, 1, k)]);
                ch[self.ch(i, k, 0)] = a + b;
                ch[self.ch(i, k, 1)] = if i == 0 { a - b } else { (a - b).tw(self.wa(0, i), fwd) };
            }
        }
    }

    fn pass3(&self, fwd: bool, cc: &[C<T>], ch: &mut [C<T>]) {
        let tw1r = T::of(-0.5);
        let tw1i = if fwd { -T::of(SQRT3_HALF) } else { T::of(SQRT3_HALF) };
        for k in 0..self.l1 {
            for i in 0..self.ido {
                let t0 = cc[self.cc(i, 0, k)];
                let (t1, t2) = pm(cc[self.cc(i, 1, k)], cc[self.cc(i, 2, k)]);
                ch[self.ch(i, k, 0)] = t0 + t1;
                let ca = t0 + t1.scale(tw1r);
                let cb = C::new(-t2.i * tw1i, t2.r * tw1i);
                let (u1, u2) = pm(ca, cb);
                if i == 0 {
                    ch[self.ch(0, k, 1)] = u1;
                    ch[self.ch(0, k, 2)] = u2;
                } else {
                    ch[self.ch(i, k, 1)] = u1.tw(self.wa(0, i), fwd);
                    ch[self.ch(i, k, 2)] = u2.tw(self.wa(1, i), fwd);
                }
            }
        }
    }

    fn pass4(&self, fwd: bool, cc: &[C<T>], ch: &mut [C<T>]) {
        for k in 0..self.l1 {
            for i in 0..self.ido {
                let (t2, t1) = pm(cc[self.cc(i, 0, k)], cc[self.cc(i, 2, k)]);
                let (t3, t4) = pm(cc[self.cc(i, 1, k)], cc[self.cc(i, 3, k)]);
                let t4 = t4.rot90(fwd);
                if i == 0 {
                    let (a, b) = pm(t2, t3);
                    ch[self.ch(0, k, 0)] = a;
                    ch[self.ch(0, k, 2)] = b;
                    let (a, b) = pm(t1, t4);
                    ch[self.ch(0, k, 1)] = a;
                    ch[self.ch(0, k, 3)] = b;
                } else {
                    ch[self.ch(i, k, 0)] = t2 + t3;
                    ch[self.ch(i, k, 1)] = (t1 + t4).tw(self.wa(0, i), fwd);
                    ch[self.ch(i, k, 2)] = (t2 - t3).tw(self.wa(1, i), fwd);
                    ch[self.ch(i, k, 3)] = (t1 - t4).tw(self.wa(2, i), fwd);
                }
            }
        }
    }

    fn pass5(&self, fwd: bool, cc: &[C<T>], ch: &mut [C<T>]) {
        let sign = |x: f64| if fwd { -T::of(x) } else { T::of(x) };
        let (tw1r, tw1i) = (T::of(COS_2PI_5), sign(SIN_2PI_5));
        let (tw2r, tw2i) = (T::of(COS_4PI_5), sign(SIN_4PI_5));
        for k in 0..self.l1 {
            for i in 0..self.ido {
                let t0 = cc[self.cc(i, 0, k)];
                let (t1, t4) = pm(cc[self.cc(i, 1, k)], cc[self.cc(i, 4, k)]);
                let (t2, t3) = pm(cc[self.cc(i, 2, k)], cc[self.cc(i, 3, k)]);
                ch[self.ch(i, k, 0)] = C::new(t0.r + t1.r + t2.r, t0.i + t1.i + t2.i);
                // POCKETFFT_PARTSTEP5(u1, u2, twar, twbr, twai, twbi), with twbi's sign written out
                let step = |twar: T, twbr: T, twai: T, twbi: T| {
                    let ca = C::new(t0.r + twar * t1.r + twbr * t2.r, t0.i + twar * t1.i + twbr * t2.i);
                    let cb = C::new(-(twai * t4.i + twbi * t3.i), twai * t4.r + twbi * t3.r);
                    pm(ca, cb)
                };
                let (a1, a4) = step(tw1r, tw2r, tw1i, tw2i);
                // (2, 3, tw2r, tw1r, +tw2i, -tw1i): the C++ subtracts tw1i * t3; x + (-y) z and x - y z are the
                // same number.
                let (a2, a3) = step(tw2r, tw1r, tw2i, -tw1i);
                if i == 0 {
                    ch[self.ch(0, k, 1)] = a1;
                    ch[self.ch(0, k, 4)] = a4;
                    ch[self.ch(0, k, 2)] = a2;
                    ch[self.ch(0, k, 3)] = a3;
                } else {
                    ch[self.ch(i, k, 1)] = a1.tw(self.wa(0, i), fwd);
                    ch[self.ch(i, k, 4)] = a4.tw(self.wa(3, i), fwd);
                    ch[self.ch(i, k, 2)] = a2.tw(self.wa(1, i), fwd);
                    ch[self.ch(i, k, 3)] = a3.tw(self.wa(2, i), fwd);
                }
            }
        }
    }

    /// `cfftp8`. With `l1 == 1` (the first pass of a length that is a multiple of 8) the C++ works in place.
    fn pass8(&self, fwd: bool, cc: &mut [C<T>], ch: &mut [C<T>]) -> bool {
        let hsqt2 = T::of(HALF_SQRT2);
        let in_place = self.l1 == 1;
        for k in 0..self.l1 {
            for i in 0..self.ido {
                let at = |m: usize| cc[self.cc(i, m, k)];
                let (a1, a5) = pm(at(1), at(5));
                let (a3, a7) = pm(at(3), at(7));
                let (a1, a3) = pm(a1, a3);
                let a3 = a3.rot90(fwd);
                let a7 = a7.rot90(fwd);
                let (a5, a7) = pm(a5, a7);
                let a5 = a5.rot45(fwd, hsqt2);
                let a7 = a7.rot135(fwd, hsqt2);
                let (a0, a4) = pm(at(0), at(4));
                let (a2, a6) = pm(at(2), at(6));
                let (a0, a2) = pm(a0, a2);
                let a6 = a6.rot90(fwd);
                let (a4, a6) = pm(a4, a6);
                // What goes to output m, before its twiddle.
                let outs = [a0 + a1, a4 + a5, a2 + a3, a6 + a7, a0 - a1, a4 - a5, a2 - a3, a6 - a7];
                for (m, &v) in outs.iter().enumerate() {
                    let v = if i == 0 || m == 0 {
                        v
                    } else {
                        v.tw(self.wa(m - 1, i), fwd)
                    };
                    if in_place {
                        cc[self.cc(i, m, 0)] = v;
                    } else {
                        ch[self.ch(i, k, m)] = v;
                    }
                }
            }
        }
        !in_place
    }
}

/// `cfftpass<Tfs>::factorize`: eights, then fours, then a two (moved to the front), then the odd primes.
fn factorize_complex(mut n: usize) -> Vec<usize> {
    let mut factors = Vec::new();
    while n & 7 == 0 {
        factors.push(8);
        n >>= 3;
    }
    while n & 3 == 0 {
        factors.push(4);
        n >>= 2;
    }
    if n & 1 == 0 {
        n >>= 1;
        factors.push(2);
        let last = factors.len() - 1;
        factors.swap(0, last);
    }
    odd_factors(n, &mut factors);
    factors
}

fn odd_factors(mut n: usize, factors: &mut Vec<usize>) {
    let mut divisor = 3;
    while divisor * divisor <= n {
        while n.is_multiple_of(divisor) {
            factors.push(divisor);
            n /= divisor;
        }
        divisor += 2;
    }
    if n > 1 {
        factors.push(n);
    }
}

/// A complex transform of one length: `pocketfft_c<T>`.
enum CPlan<T> {
    /// `cfft_multipass` (or a single pass, or nothing for length 1): the radix passes one after another.
    Passes(Vec<CPass<T>>),
    /// `cfftp_vecpass<4>`: one radix 4 pass over the whole array, then the four quarters transformed each on its
    /// own and woven together. What `ducc0` uses for a left-over `float32` column whose length is above 300 and a
    /// multiple of 4 ([`CPlan::split`]).
    Quarters { first: CPass<T>, rest: Box<CPlan<T>> },
}

impl<T: Real> CPlan<T> {
    /// Whether the plan for one transform on its own (`vectorize`) is the split one: `cfftpass::make_pass` takes
    /// `cfftp_vecpass<4>` for a length above 300 (and up to 100,000) that is a multiple of 4, and only
    /// `if constexpr (simd_exists<Tfs, 4>)`. So never in `float64` on the reference build: there a column on its
    /// own is transformed exactly like a column in a group (in scipy a 1-D `complex128` transform equals the
    /// same column of a 2-D one to the bit; a `complex64` one of such a length does not).
    fn split(n: usize) -> bool {
        T::FOUR_LANES && n > 300 && n <= 100_000 && n & 3 == 0
    }

    /// `pocketfft_c<T>(n, vectorize)`.
    fn new(n: usize, vectorize: bool) -> CPlan<T> {
        // Above this ducc0 splits the passes into two packets, and transforms long columns one at a time.
        assert!(
            n <= MAX_LEN,
            "fft: complex transforms above 10,000 samples are not ported"
        );
        let roots = Roots::new(n);
        if vectorize && Self::split(n) {
            // The sub-plans take their twiddles from the roots of the whole length.
            return CPlan::Quarters {
                first: CPass::new(1, n / 4, 4, &roots),
                rest: Box::new(CPlan::passes(n / 4, &roots)),
            };
        }
        CPlan::passes(n, &roots)
    }

    fn passes(n: usize, roots: &Roots) -> CPlan<T> {
        let mut passes = Vec::new();
        let mut l1 = 1;
        if n > 1 {
            for f in factorize_complex(n) {
                passes.push(CPass::new(l1, n / (f * l1), f, roots));
                l1 *= f;
            }
        }
        CPlan::Passes(passes)
    }

    /// Transforms `data` in place; `scratch` is as long.
    fn exec(&self, fwd: bool, data: &mut [C<T>], scratch: &mut [C<T>]) {
        match self {
            CPlan::Passes(passes) => {
                let mut in_data = true;
                for p in passes {
                    let moved = if in_data {
                        p.exec(fwd, data, scratch)
                    } else {
                        p.exec(fwd, scratch, data)
                    };
                    in_data ^= moved;
                }
                if !in_data {
                    data.copy_from_slice(scratch);
                }
            }
            CPlan::Quarters { first, rest } => {
                first.exec(fwd, data, scratch);
                let q = data.len() / 4;
                let mut tmp = vec![C::default(); q];
                for (j, quarter) in scratch.chunks_exact_mut(q).enumerate() {
                    rest.exec(fwd, quarter, &mut tmp);
                    for (i, &v) in quarter.iter().enumerate() {
                        data[i * 4 + j] = v;
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------- real passes
/// One radix pass of a real transform (`rfftp2`, `rfftp3`, `rfftp4`, `rfftp5`), on half-complex data in FFTPACK
/// order (`r0, r1, i1, r2, i2, ...`).
struct RPass<T> {
    ip: usize,
    l1: usize,
    ido: usize,
    wa: Vec<T>,
}

impl<T: Real> RPass<T> {
    fn new(l1: usize, ido: usize, ip: usize, roots: &Roots) -> RPass<T> {
        if ![2, 3, 4, 5].contains(&ip) {
            unsupported(ip);
        }
        let n = ip * l1 * ido;
        let rfct = roots.n / n;
        assert_eq!(roots.n, n * rfct, "fft: the roots do not fit the pass");
        let mut wa = vec![T::default(); (ip - 1) * (ido - 1)];
        for j in 1..ip {
            for i in 1..=(ido - 1) / 2 {
                let val: C<T> = roots.get(rfct * j * l1 * i);
                wa[(j - 1) * (ido - 1) + 2 * i - 2] = val.r;
                wa[(j - 1) * (ido - 1) + 2 * i - 1] = val.i;
            }
        }
        RPass { ip, l1, ido, wa }
    }

    /// `WA(x, i)`.
    fn wa(&self, x: usize, i: usize) -> T {
        self.wa[i + x * (self.ido - 1)]
    }

    /// The index with the radix as the middle dimension: `[a + ido * (b + ip * c)]`.
    fn by_radix(&self, a: usize, b: usize, c: usize) -> usize {
        a + self.ido * (b + self.ip * c)
    }

    /// The index with the blocks as the middle dimension: `[a + ido * (b + l1 * c)]`.
    fn by_block(&self, a: usize, b: usize, c: usize) -> usize {
        a + self.ido * (b + self.l1 * c)
    }

    /// Runs the pass from `cc` into `ch`.
    fn exec(&self, fwd: bool, cc: &[T], ch: &mut [T]) {
        match (self.ip, fwd) {
            (2, true) => self.forward2(cc, ch),
            (2, false) => self.backward2(cc, ch),
            (3, true) => self.forward3(cc, ch),
            (3, false) => self.backward3(cc, ch),
            (4, true) => self.forward4(cc, ch),
            (4, false) => self.backward4(cc, ch),
            (_, true) => self.forward5(cc, ch),
            (_, false) => self.backward5(cc, ch),
        }
    }

    fn forward2(&self, cc: &[T], ch: &mut [T]) {
        let (l1, ido) = (self.l1, self.ido);
        let ccx = |a: usize, b: usize, c: usize| cc[self.by_block(a, b, c)];
        let chx = |a: usize, b: usize, c: usize| self.by_radix(a, b, c);
        for k in 0..l1 {
            (ch[chx(0, 0, k)], ch[chx(ido - 1, 1, k)]) = pm(ccx(0, k, 0), ccx(0, k, 1));
        }
        if ido & 1 == 0 {
            for k in 0..l1 {
                ch[chx(0, 1, k)] = -ccx(ido - 1, k, 1);
                ch[chx(ido - 1, 0, k)] = ccx(ido - 1, k, 0);
            }
        }
        if ido <= 2 {
            return;
        }
        for k in 0..l1 {
            for i in (2..ido).step_by(2) {
                let ic = ido - i;
                let (tr2, ti2) = mulpm(self.wa(0, i - 2), self.wa(0, i - 1), ccx(i - 1, k, 1), ccx(i, k, 1));
                (ch[chx(i - 1, 0, k)], ch[chx(ic - 1, 1, k)]) = pm(ccx(i - 1, k, 0), tr2);
                (ch[chx(i, 0, k)], ch[chx(ic, 1, k)]) = pm(ti2, ccx(i, k, 0));
            }
        }
    }

    fn backward2(&self, cc: &[T], ch: &mut [T]) {
        let (l1, ido) = (self.l1, self.ido);
        let ccx = |a: usize, b: usize, c: usize| cc[self.by_radix(a, b, c)];
        let chx = |a: usize, b: usize, c: usize| self.by_block(a, b, c);
        let two = T::of(2.0);
        for k in 0..l1 {
            (ch[chx(0, k, 0)], ch[chx(0, k, 1)]) = pm(ccx(0, 0, k), ccx(ido - 1, 1, k));
        }
        if ido & 1 == 0 {
            for k in 0..l1 {
                ch[chx(ido - 1, k, 0)] = two * ccx(ido - 1, 0, k);
                ch[chx(ido - 1, k, 1)] = -two * ccx(0, 1, k);
            }
        }
        if ido <= 2 {
            return;
        }
        for k in 0..l1 {
            for i in (2..ido).step_by(2) {
                let ic = ido - i;
                let (a, tr2) = pm(ccx(i - 1, 0, k), ccx(ic - 1, 1, k));
                ch[chx(i - 1, k, 0)] = a;
                let (ti2, b) = pm(ccx(i, 0, k), ccx(ic, 1, k));
                ch[chx(i, k, 0)] = b;
                (ch[chx(i, k, 1)], ch[chx(i - 1, k, 1)]) = mulpm(self.wa(0, i - 2), self.wa(0, i - 1), ti2, tr2);
            }
        }
    }

    fn forward3(&self, cc: &[T], ch: &mut [T]) {
        let (l1, ido) = (self.l1, self.ido);
        let (taur, taui) = (T::of(-0.5), T::of(SQRT3_HALF));
        let ccx = |a: usize, b: usize, c: usize| cc[self.by_block(a, b, c)];
        let chx = |a: usize, b: usize, c: usize| self.by_radix(a, b, c);
        for k in 0..l1 {
            let cr2 = ccx(0, k, 1) + ccx(0, k, 2);
            ch[chx(0, 0, k)] = ccx(0, k, 0) + cr2;
            ch[chx(0, 2, k)] = taui * (ccx(0, k, 2) - ccx(0, k, 1));
            ch[chx(ido - 1, 1, k)] = ccx(0, k, 0) + taur * cr2;
        }
        if ido == 1 {
            return;
        }
        for k in 0..l1 {
            for i in (2..ido).step_by(2) {
                let ic = ido - i;
                let (dr2, di2) = mulpm(self.wa(0, i - 2), self.wa(0, i - 1), ccx(i - 1, k, 1), ccx(i, k, 1));
                let (dr3, di3) = mulpm(self.wa(1, i - 2), self.wa(1, i - 1), ccx(i - 1, k, 2), ccx(i, k, 2));
                // POCKETFFT_REARRANGE(dr2, di2, dr3, di3)
                let (dr2, di2, dr3, di3) = (dr2 + dr3, di2 + di3, di2 - di3, dr3 - dr2);
                ch[chx(i - 1, 0, k)] = ccx(i - 1, k, 0) + dr2;
                ch[chx(i, 0, k)] = ccx(i, k, 0) + di2;
                let tr2 = ccx(i - 1, k, 0) + taur * dr2;
                let ti2 = ccx(i, k, 0) + taur * di2;
                let (tr3, ti3) = (taui * dr3, taui * di3);
                (ch[chx(i - 1, 2, k)], ch[chx(ic - 1, 1, k)]) = pm(tr2, tr3);
                (ch[chx(i, 2, k)], ch[chx(ic, 1, k)]) = pm(ti3, ti2);
            }
        }
    }

    fn backward3(&self, cc: &[T], ch: &mut [T]) {
        let (l1, ido) = (self.l1, self.ido);
        let (taur, taui) = (T::of(-0.5), T::of(SQRT3_HALF));
        let two = T::of(2.0);
        let ccx = |a: usize, b: usize, c: usize| cc[self.by_radix(a, b, c)];
        let chx = |a: usize, b: usize, c: usize| self.by_block(a, b, c);
        for k in 0..l1 {
            let tr2 = two * ccx(ido - 1, 1, k);
            let cr2 = ccx(0, 0, k) + taur * tr2;
            ch[chx(0, k, 0)] = ccx(0, 0, k) + tr2;
            let ci3 = two * taui * ccx(0, 2, k);
            (ch[chx(0, k, 2)], ch[chx(0, k, 1)]) = pm(cr2, ci3);
        }
        if ido == 1 {
            return;
        }
        for k in 0..l1 {
            for i in (2..ido).step_by(2) {
                let ic = ido - i;
                let tr2 = ccx(i - 1, 2, k) + ccx(ic - 1, 1, k);
                let ti2 = ccx(i, 2, k) - ccx(ic, 1, k);
                let cr2 = ccx(i - 1, 0, k) + taur * tr2;
                let ci2 = ccx(i, 0, k) + taur * ti2;
                ch[chx(i - 1, k, 0)] = ccx(i - 1, 0, k) + tr2;
                ch[chx(i, k, 0)] = ccx(i, 0, k) + ti2;
                let cr3 = taui * (ccx(i - 1, 2, k) - ccx(ic - 1, 1, k));
                let ci3 = taui * (ccx(i, 2, k) + ccx(ic, 1, k));
                let (dr3, dr2) = pm(cr2, ci3);
                let (di2, di3) = pm(ci2, cr3);
                (ch[chx(i, k, 1)], ch[chx(i - 1, k, 1)]) = mulpm(self.wa(0, i - 2), self.wa(0, i - 1), di2, dr2);
                (ch[chx(i, k, 2)], ch[chx(i - 1, k, 2)]) = mulpm(self.wa(1, i - 2), self.wa(1, i - 1), di3, dr3);
            }
        }
    }

    fn forward4(&self, cc: &[T], ch: &mut [T]) {
        let (l1, ido) = (self.l1, self.ido);
        let hsqt2 = T::of(HALF_SQRT2);
        let ccx = |a: usize, b: usize, c: usize| cc[self.by_block(a, b, c)];
        let chx = |a: usize, b: usize, c: usize| self.by_radix(a, b, c);
        for k in 0..l1 {
            let (tr1, a) = pm(ccx(0, k, 3), ccx(0, k, 1));
            ch[chx(0, 2, k)] = a;
            let (tr2, b) = pm(ccx(0, k, 0), ccx(0, k, 2));
            ch[chx(ido - 1, 1, k)] = b;
            (ch[chx(0, 0, k)], ch[chx(ido - 1, 3, k)]) = pm(tr2, tr1);
        }
        if ido & 1 == 0 {
            for k in 0..l1 {
                let ti1 = -hsqt2 * (ccx(ido - 1, k, 1) + ccx(ido - 1, k, 3));
                let tr1 = hsqt2 * (ccx(ido - 1, k, 1) - ccx(ido - 1, k, 3));
                (ch[chx(ido - 1, 0, k)], ch[chx(ido - 1, 2, k)]) = pm(ccx(ido - 1, k, 0), tr1);
                (ch[chx(0, 3, k)], ch[chx(0, 1, k)]) = pm(ti1, ccx(ido - 1, k, 2));
            }
        }
        if ido <= 2 {
            return;
        }
        for k in 0..l1 {
            for i in (2..ido).step_by(2) {
                let ic = ido - i;
                let (cr2, ci2) = mulpm(self.wa(0, i - 2), self.wa(0, i - 1), ccx(i - 1, k, 1), ccx(i, k, 1));
                let (cr3, ci3) = mulpm(self.wa(1, i - 2), self.wa(1, i - 1), ccx(i - 1, k, 2), ccx(i, k, 2));
                let (cr4, ci4) = mulpm(self.wa(2, i - 2), self.wa(2, i - 1), ccx(i - 1, k, 3), ccx(i, k, 3));
                let (tr1, tr4) = pm(cr4, cr2);
                let (ti1, ti4) = pm(ci2, ci4);
                let (tr2, tr3) = pm(ccx(i - 1, k, 0), cr3);
                let (ti2, ti3) = pm(ccx(i, k, 0), ci3);
                (ch[chx(i - 1, 0, k)], ch[chx(ic - 1, 3, k)]) = pm(tr2, tr1);
                (ch[chx(i, 0, k)], ch[chx(ic, 3, k)]) = pm(ti1, ti2);
                (ch[chx(i - 1, 2, k)], ch[chx(ic - 1, 1, k)]) = pm(tr3, ti4);
                (ch[chx(i, 2, k)], ch[chx(ic, 1, k)]) = pm(tr4, ti3);
            }
        }
    }

    fn backward4(&self, cc: &[T], ch: &mut [T]) {
        let (l1, ido) = (self.l1, self.ido);
        let (sqrt2, two) = (T::of(SQRT2), T::of(2.0));
        let ccx = |a: usize, b: usize, c: usize| cc[self.by_radix(a, b, c)];
        let chx = |a: usize, b: usize, c: usize| self.by_block(a, b, c);
        for k in 0..l1 {
            let (tr2, tr1) = pm(ccx(0, 0, k), ccx(ido - 1, 3, k));
            let tr3 = two * ccx(ido - 1, 1, k);
            let tr4 = two * ccx(0, 2, k);
            (ch[chx(0, k, 0)], ch[chx(0, k, 2)]) = pm(tr2, tr3);
            (ch[chx(0, k, 3)], ch[chx(0, k, 1)]) = pm(tr1, tr4);
        }
        if ido & 1 == 0 {
            for k in 0..l1 {
                let (ti1, ti2) = pm(ccx(0, 3, k), ccx(0, 1, k));
                let (tr2, tr1) = pm(ccx(ido - 1, 0, k), ccx(ido - 1, 2, k));
                ch[chx(ido - 1, k, 0)] = tr2 + tr2;
                ch[chx(ido - 1, k, 1)] = sqrt2 * (tr1 - ti1);
                ch[chx(ido - 1, k, 2)] = ti2 + ti2;
                ch[chx(ido - 1, k, 3)] = -sqrt2 * (tr1 + ti1);
            }
        }
        if ido <= 2 {
            return;
        }
        for k in 0..l1 {
            for i in (2..ido).step_by(2) {
                let ic = ido - i;
                let (tr2, tr1) = pm(ccx(i - 1, 0, k), ccx(ic - 1, 3, k));
                let (ti1, ti2) = pm(ccx(i, 0, k), ccx(ic, 3, k));
                let (tr4, ti3) = pm(ccx(i, 2, k), ccx(ic, 1, k));
                let (tr3, ti4) = pm(ccx(i - 1, 2, k), ccx(ic - 1, 1, k));
                let (a, cr3) = pm(tr2, tr3);
                ch[chx(i - 1, k, 0)] = a;
                let (b, ci3) = pm(ti2, ti3);
                ch[chx(i, k, 0)] = b;
                let (cr4, cr2) = pm(tr1, tr4);
                let (ci2, ci4) = pm(ti1, ti4);
                (ch[chx(i, k, 1)], ch[chx(i - 1, k, 1)]) = mulpm(self.wa(0, i - 2), self.wa(0, i - 1), ci2, cr2);
                (ch[chx(i, k, 2)], ch[chx(i - 1, k, 2)]) = mulpm(self.wa(1, i - 2), self.wa(1, i - 1), ci3, cr3);
                (ch[chx(i, k, 3)], ch[chx(i - 1, k, 3)]) = mulpm(self.wa(2, i - 2), self.wa(2, i - 1), ci4, cr4);
            }
        }
    }

    fn forward5(&self, cc: &[T], ch: &mut [T]) {
        let (l1, ido) = (self.l1, self.ido);
        let (tr11, ti11) = (T::of(COS_2PI_5), T::of(SIN_2PI_5));
        let (tr12, ti12) = (T::of(COS_4PI_5), T::of(SIN_4PI_5));
        let ccx = |a: usize, b: usize, c: usize| cc[self.by_block(a, b, c)];
        let chx = |a: usize, b: usize, c: usize| self.by_radix(a, b, c);
        for k in 0..l1 {
            let (cr2, ci5) = pm(ccx(0, k, 4), ccx(0, k, 1));
            let (cr3, ci4) = pm(ccx(0, k, 3), ccx(0, k, 2));
            ch[chx(0, 0, k)] = ccx(0, k, 0) + cr2 + cr3;
            ch[chx(ido - 1, 1, k)] = ccx(0, k, 0) + tr11 * cr2 + tr12 * cr3;
            ch[chx(0, 2, k)] = ti11 * ci5 + ti12 * ci4;
            ch[chx(ido - 1, 3, k)] = ccx(0, k, 0) + tr12 * cr2 + tr11 * cr3;
            ch[chx(0, 4, k)] = ti12 * ci5 - ti11 * ci4;
        }
        if ido == 1 {
            return;
        }
        for k in 0..l1 {
            for i in (2..ido).step_by(2) {
                let ic = ido - i;
                let (dr2, di2) = mulpm(self.wa(0, i - 2), self.wa(0, i - 1), ccx(i - 1, k, 1), ccx(i, k, 1));
                let (dr3, di3) = mulpm(self.wa(1, i - 2), self.wa(1, i - 1), ccx(i - 1, k, 2), ccx(i, k, 2));
                let (dr4, di4) = mulpm(self.wa(2, i - 2), self.wa(2, i - 1), ccx(i - 1, k, 3), ccx(i, k, 3));
                let (dr5, di5) = mulpm(self.wa(3, i - 2), self.wa(3, i - 1), ccx(i - 1, k, 4), ccx(i, k, 4));
                // POCKETFFT_REARRANGE(dr2, di2, dr5, di5) and (dr3, di3, dr4, di4)
                let (dr2, di2, dr5, di5) = (dr2 + dr5, di2 + di5, di2 - di5, dr5 - dr2);
                let (dr3, di3, dr4, di4) = (dr3 + dr4, di3 + di4, di3 - di4, dr4 - dr3);
                ch[chx(i - 1, 0, k)] = ccx(i - 1, k, 0) + dr2 + dr3;
                ch[chx(i, 0, k)] = ccx(i, k, 0) + di2 + di3;
                let tr2 = ccx(i - 1, k, 0) + tr11 * dr2 + tr12 * dr3;
                let ti2 = ccx(i, k, 0) + tr11 * di2 + tr12 * di3;
                let tr3 = ccx(i - 1, k, 0) + tr12 * dr2 + tr11 * dr3;
                let ti3 = ccx(i, k, 0) + tr12 * di2 + tr11 * di3;
                let tr5 = ti11 * dr5 + ti12 * dr4;
                let ti5 = ti11 * di5 + ti12 * di4;
                let tr4 = ti12 * dr5 - ti11 * dr4;
                let ti4 = ti12 * di5 - ti11 * di4;
                (ch[chx(i - 1, 2, k)], ch[chx(ic - 1, 1, k)]) = pm(tr2, tr5);
                (ch[chx(i, 2, k)], ch[chx(ic, 1, k)]) = pm(ti5, ti2);
                (ch[chx(i - 1, 4, k)], ch[chx(ic - 1, 3, k)]) = pm(tr3, tr4);
                (ch[chx(i, 4, k)], ch[chx(ic, 3, k)]) = pm(ti4, ti3);
            }
        }
    }

    fn backward5(&self, cc: &[T], ch: &mut [T]) {
        let (l1, ido) = (self.l1, self.ido);
        let (tr11, ti11) = (T::of(COS_2PI_5), T::of(SIN_2PI_5));
        let (tr12, ti12) = (T::of(COS_4PI_5), T::of(SIN_4PI_5));
        let ccx = |a: usize, b: usize, c: usize| cc[self.by_radix(a, b, c)];
        let chx = |a: usize, b: usize, c: usize| self.by_block(a, b, c);
        for k in 0..l1 {
            let ti5 = ccx(0, 2, k) + ccx(0, 2, k);
            let ti4 = ccx(0, 4, k) + ccx(0, 4, k);
            let tr2 = ccx(ido - 1, 1, k) + ccx(ido - 1, 1, k);
            let tr3 = ccx(ido - 1, 3, k) + ccx(ido - 1, 3, k);
            ch[chx(0, k, 0)] = ccx(0, 0, k) + tr2 + tr3;
            let cr2 = ccx(0, 0, k) + tr11 * tr2 + tr12 * tr3;
            let cr3 = ccx(0, 0, k) + tr12 * tr2 + tr11 * tr3;
            let (ci5, ci4) = mulpm(ti5, ti4, ti11, ti12);
            (ch[chx(0, k, 4)], ch[chx(0, k, 1)]) = pm(cr2, ci5);
            (ch[chx(0, k, 3)], ch[chx(0, k, 2)]) = pm(cr3, ci4);
        }
        if ido == 1 {
            return;
        }
        for k in 0..l1 {
            for i in (2..ido).step_by(2) {
                let ic = ido - i;
                let (tr2, tr5) = pm(ccx(i - 1, 2, k), ccx(ic - 1, 1, k));
                let (ti5, ti2) = pm(ccx(i, 2, k), ccx(ic, 1, k));
                let (tr3, tr4) = pm(ccx(i - 1, 4, k), ccx(ic - 1, 3, k));
                let (ti4, ti3) = pm(ccx(i, 4, k), ccx(ic, 3, k));
                ch[chx(i - 1, k, 0)] = ccx(i - 1, 0, k) + tr2 + tr3;
                ch[chx(i, k, 0)] = ccx(i, 0, k) + ti2 + ti3;
                let cr2 = ccx(i - 1, 0, k) + tr11 * tr2 + tr12 * tr3;
                let ci2 = ccx(i, 0, k) + tr11 * ti2 + tr12 * ti3;
                let cr3 = ccx(i - 1, 0, k) + tr12 * tr2 + tr11 * tr3;
                let ci3 = ccx(i, 0, k) + tr12 * ti2 + tr11 * ti3;
                let (cr5, cr4) = mulpm(tr5, tr4, ti11, ti12);
                let (ci5, ci4) = mulpm(ti5, ti4, ti11, ti12);
                let (dr4, dr3) = pm(cr3, ci4);
                let (di3, di4) = pm(ci3, cr4);
                let (dr5, dr2) = pm(cr2, ci5);
                let (di2, di5) = pm(ci2, cr5);
                (ch[chx(i, k, 1)], ch[chx(i - 1, k, 1)]) = mulpm(self.wa(0, i - 2), self.wa(0, i - 1), di2, dr2);
                (ch[chx(i, k, 2)], ch[chx(i - 1, k, 2)]) = mulpm(self.wa(1, i - 2), self.wa(1, i - 1), di3, dr3);
                (ch[chx(i, k, 3)], ch[chx(i - 1, k, 3)]) = mulpm(self.wa(2, i - 2), self.wa(2, i - 1), di4, dr4);
                (ch[chx(i, k, 4)], ch[chx(i - 1, k, 4)]) = mulpm(self.wa(3, i - 2), self.wa(3, i - 1), di5, dr5);
            }
        }
    }
}

/// `rfftpass<Tfs>::factorize`: fours, then a two (moved to the front), then the odd primes.
fn factorize_real(mut n: usize) -> Vec<usize> {
    let mut factors = Vec::new();
    while n & 3 == 0 {
        factors.push(4);
        n >>= 2;
    }
    if n & 1 == 0 {
        n >>= 1;
        factors.push(2);
        let last = factors.len() - 1;
        factors.swap(0, last);
    }
    odd_factors(n, &mut factors);
    factors
}

/// A real transform of one length: `pocketfft_r<T>` (`rfft_multipass`, or a single pass).
struct RPlan<T> {
    passes: Vec<RPass<T>>,
}

impl<T: Real> RPlan<T> {
    fn new(n: usize) -> RPlan<T> {
        assert!(
            n <= MAX_LEN || n & 1 == 1,
            "fft: real transforms of an even length above 10,000 are not ported"
        );
        let roots = Roots::new(n);
        let mut passes = Vec::new();
        let mut l1 = 1;
        if n > 1 {
            for f in factorize_real(n) {
                passes.push(RPass::new(l1, n / (f * l1), f, &roots));
                l1 *= f;
            }
        }
        RPlan { passes }
    }

    /// Transforms `data` in place (half-complex after a forward transform, before a backward one); `scratch` is
    /// as long. Going forward the passes run last to first.
    fn exec(&self, fwd: bool, data: &mut [T], scratch: &mut [T]) {
        let mut in_data = true;
        let mut step = |p: &RPass<T>| {
            if in_data {
                p.exec(fwd, data, scratch);
            } else {
                p.exec(fwd, scratch, data);
            }
            in_data = !in_data;
        };
        if fwd {
            self.passes.iter().rev().for_each(&mut step);
        } else {
            self.passes.iter().for_each(&mut step);
        }
        if !in_data {
            data.copy_from_slice(scratch);
        }
    }
}

// ---------------------------------------------------------------- two dimensions
/// `scipy.fft.next_fast_len(n, real=True)`: the smallest product of 2, 3 and 5 that is at least `n`
/// (`util1d::good_size_real`).
pub fn good_size_real(n: usize) -> usize {
    if n <= 6 {
        return n;
    }
    let mut best = 2 * n;
    let mut f5 = 1;
    while f5 < best {
        let mut x = f5;
        while x < n {
            x *= 2;
        }
        loop {
            if x < n {
                x *= 3;
            } else if x > n {
                if x < best {
                    best = x;
                }
                if x & 1 == 1 {
                    break;
                }
                x >>= 1;
            } else {
                return n;
            }
        }
        f5 *= 5;
    }
    best
}

/// The complex transform down every column of a `(rows, cols)` array, in place: `ducc0::c2c` along axis 0. The
/// columns are taken `T::LANES` at a time with the plain plan; the columns left over get the plan `ducc0` builds
/// for single transforms, which is another one only in `float32`, when the length is above 300 and a multiple of
/// 4 ([`CPlan::split`]).
fn columns<T: Real>(a: &mut [C<T>], rows: usize, cols: usize, fwd: bool) {
    let plan: CPlan<T> = CPlan::new(rows, false);
    let odd_plan: Option<CPlan<T>> = CPlan::<T>::split(rows).then(|| CPlan::new(rows, true));
    let together = cols - cols % T::LANES;
    let (mut col, mut scratch) = (vec![C::default(); rows], vec![C::default(); rows]);
    for x in 0..cols {
        for (y, v) in col.iter_mut().enumerate() {
            *v = a[y * cols + x];
        }
        let p = if x < together {
            &plan
        } else {
            odd_plan.as_ref().unwrap_or(&plan)
        };
        p.exec(fwd, &mut col, &mut scratch);
        for (y, &v) in col.iter().enumerate() {
            a[y * cols + x] = v;
        }
    }
}

/// `scipy.fft.rfftn(x, s)` for a 2-D real array: `x` (`shape.0` rows of `shape.1`) cut or padded with zeros to
/// `s`, transformed along the rows (real to complex, `s.1 / 2 + 1` values a row), then down the columns.
fn rfftn<T: Real>(x: &[T], shape: (usize, usize), s: (usize, usize)) -> Vec<C<T>> {
    assert_eq!(x.len(), shape.0 * shape.1, "rfftn: the array does not have its shape");
    assert!(s.0 >= 1 && s.1 >= 1, "rfftn: an empty transform");
    let (rows, len) = s;
    let cols = len / 2 + 1;
    let plan: RPlan<T> = RPlan::new(len);
    let mut out = vec![C::default(); rows * cols];
    let (mut row, mut scratch) = (vec![T::default(); len], vec![T::default(); len]);
    for y in 0..rows {
        row.fill(T::default());
        if y < shape.0 {
            let n = shape.1.min(len);
            row[..n].copy_from_slice(&x[y * shape.1..y * shape.1 + n]);
        }
        plan.exec(true, &mut row, &mut scratch);
        // FFTPACK's half-complex row as complex numbers.
        let o = &mut out[y * cols..(y + 1) * cols];
        o[0] = C::new(row[0], T::default());
        let mut i = 1;
        let mut ii = 1;
        while i + 1 < len {
            o[ii] = C::new(row[i], row[i + 1]);
            i += 2;
            ii += 1;
        }
        if i < len {
            o[ii] = C::new(row[i], T::default());
        }
    }
    columns(&mut out, rows, cols, true);
    out
}

/// `scipy.fft.irfftn(X, s)` for a 2-D array of `s.0` rows of `s.1 / 2 + 1` complex values: back down the columns,
/// then each row from half-complex to `s.1` real values, times `1 / (s.0 * s.1)`.
fn irfftn<T: Real>(x: &[C<T>], s: (usize, usize), fct: T) -> Vec<T> {
    let (rows, len) = s;
    let cols = len / 2 + 1;
    assert_eq!(x.len(), rows * cols, "irfftn: the array does not have its shape");
    let mut tmp = x.to_vec();
    columns(&mut tmp, rows, cols, false);
    let plan: RPlan<T> = RPlan::new(len);
    let mut out = vec![T::default(); rows * len];
    let mut scratch = vec![T::default(); len];
    for y in 0..rows {
        let (src, row) = (&tmp[y * cols..(y + 1) * cols], &mut out[y * len..(y + 1) * len]);
        row[0] = src[0].r;
        let mut i = 1;
        let mut ii = 1;
        while i + 1 < len {
            row[i] = src[ii].r;
            row[i + 1] = src[ii].i;
            i += 2;
            ii += 1;
        }
        if i < len {
            row[i] = src[ii].r;
        }
        plan.exec(false, row, &mut scratch);
        for v in row.iter_mut() {
            *v = *v * fct;
        }
    }
    out
}

fn pairs<T: Real>(a: Vec<C<T>>) -> Vec<Cpx<T>> {
    a.into_iter().map(|c| [c.r, c.i]).collect()
}

/// `scipy.fft.rfftn(x, s)` for a 2-D `float32` array, row-major, of `shape` (rows, columns), with `s` the
/// (rows, columns) to cut or zero-pad to: `s.0` rows of `s.1 / 2 + 1` complex values (`complex64`). Panics unless
/// both lengths of `s` are products of 2, 3 and 5.
pub fn rfftn_f32(x: &[f32], shape: (usize, usize), s: (usize, usize)) -> Vec<Cpx<f32>> {
    pairs(rfftn(x, shape, s))
}

/// [`rfftn_f32`] for a `float64` array (`complex128`).
pub fn rfftn_f64(x: &[f64], shape: (usize, usize), s: (usize, usize)) -> Vec<Cpx<f64>> {
    pairs(rfftn(x, shape, s))
}

/// `scipy.fft.irfftn(X, s)` for a 2-D `complex128` array of `s.0` rows of `s.1 / 2 + 1` values: `s.0` rows of
/// `s.1` real values.
pub fn irfftn_f64(x: &[Cpx<f64>], s: (usize, usize)) -> Vec<f64> {
    let x: Vec<C<f64>> = x.iter().map(|c| C::new(c[0], c[1])).collect();
    // norm_fct: 1 / N, one rounding
    irfftn(&x, s, 1.0 / (s.0 * s.1) as f64)
}

/// A kernel ready to be convolved with planes of one size: its spectrum at the padded size, which
/// `fftconvolve` works out again on every call and [`crate::lens::dof`] needs four times per blur level.
pub struct Convolver {
    /// The plane's (rows, columns).
    plane: (usize, usize),
    /// The kernel's (rows, columns).
    kernel: (usize, usize),
    /// The padded (rows, columns) of the transforms.
    s: (usize, usize),
    spectrum: Vec<C<f64>>,
}

impl Convolver {
    /// The kernel (`float64`, one channel) for planes of `w` x `h`.
    pub fn new(kern: &Img<f64>, w: usize, h: usize) -> Convolver {
        assert_eq!(kern.c, 1, "fftconvolve: the kernel has one channel");
        // fftconvolve leaves an axis of length 1 out of the transform: not what the lens does, not ported.
        assert!(
            w > 1 && h > 1 && kern.w > 1 && kern.h > 1,
            "fftconvolve: the plane and the kernel must be more than one pixel each way"
        );
        let (plane, kernel) = ((h, w), (kern.h, kern.w));
        let s = (good_size_real(h + kern.h - 1), good_size_real(w + kern.w - 1));
        Convolver {
            plane,
            kernel,
            s,
            spectrum: rfftn(&kern.data, kernel, s),
        }
    }

    /// `fftconvolve(a, kern, mode='same')` for a `float32` plane of the size given to [`Convolver::new`],
    /// row-major: the `float64` plane of the same size.
    pub fn convolve(&self, a: &[f32]) -> Vec<f64> {
        let ((h, w), s) = (self.plane, self.s);
        // complex64 times complex128, in complex128. (numpy's product is fused on the reference PC.)
        let product: Vec<C<f64>> = rfftn(a, self.plane, s)
            .iter()
            .zip(&self.spectrum)
            .map(|(p, q)| {
                let (pr, pi) = (f64::from(p.r), f64::from(p.i));
                C::new(pr * q.r - pi * q.i, pr * q.i + pi * q.r)
            })
            .collect();
        let full = irfftn(&product, s, 1.0 / (s.0 * s.1) as f64);
        // The full convolution is the top-left (h + kh - 1, w + kw - 1); 'same' is its centre.
        let (y0, x0) = ((self.kernel.0 - 1) / 2, (self.kernel.1 - 1) / 2);
        let mut out = Vec::with_capacity(h * w);
        for y in 0..h {
            let at = (y + y0) * s.1 + x0;
            out.extend_from_slice(&full[at..at + w]);
        }
        out
    }
}

/// `scipy.signal.fftconvolve(a, kern, mode='same')` for a `float32` plane and a `float64` kernel (both one
/// channel, both more than one pixel each way): a `float64` plane the size of `a`.
pub fn fftconvolve_same(a: &Img<f32>, kern: &Img<f64>) -> Img<f64> {
    assert_eq!(a.c, 1, "fftconvolve: the plane has one channel");
    Img::from_vec(a.w, a.h, 1, Convolver::new(kern, a.w, a.h).convolve(&a.data))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The transform by its definition, in double.
    fn dft(x: &[(f64, f64)], sign: f64) -> Vec<(f64, f64)> {
        let n = x.len();
        (0..n)
            .map(|k| {
                let mut s = (0.0, 0.0);
                for (j, &(re, im)) in x.iter().enumerate() {
                    let a = sign * 2.0 * dmath::PI * ((j * k) % n) as f64 / n as f64;
                    let (c, si) = (dmath::cos(a), dmath::sin(a));
                    s.0 += re * c - im * si;
                    s.1 += re * si + im * c;
                }
                s
            })
            .collect()
    }

    fn noise(n: usize, seed: u64) -> Vec<f64> {
        let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        (0..n)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state >> 11) as f64 / (1u64 << 53) as f64 - 0.5
            })
            .collect()
    }

    #[test]
    fn a_quarter_of_pi_over_n() {
        assert_eq!(quarter_pi_over(1), dmath::PI / 4.0);
        assert_eq!(quarter_pi_over(2), dmath::PI / 8.0);
        assert_eq!(quarter_pi_over(64), dmath::PI / 256.0);
        for n in [3usize, 5, 375, 648, 675] {
            let plain = dmath::PI / 4.0 / n as f64;
            assert!((quarter_pi_over(n) - plain).abs() <= plain * 2.3e-16, "{n}");
        }
    }

    #[test]
    fn roots_are_on_the_circle() {
        for n in [1usize, 2, 8, 12, 375, 648] {
            let roots = Roots::new(n);
            for k in 0..n {
                let got: C<f64> = roots.get(k);
                let a = 2.0 * dmath::PI * k as f64 / n as f64;
                assert!(
                    (got.r - dmath::cos(a)).abs() < 1e-15 && (got.i - dmath::sin(a)).abs() < 1e-15,
                    "{n} {k}"
                );
            }
        }
        let quarter: C<f64> = Roots::new(8).get(2);
        assert_eq!((quarter.r, quarter.i), (0.0, 1.0));
    }

    #[test]
    fn factors_come_in_ducc_order() {
        assert_eq!(factorize_complex(648), [8, 3, 3, 3, 3]);
        assert_eq!(factorize_complex(675), [3, 3, 3, 5, 5]);
        assert_eq!(factorize_complex(96), [8, 4, 3]);
        assert_eq!(factorize_complex(48), [2, 8, 3]);
        assert_eq!(factorize_complex(300), [4, 3, 5, 5]);
        assert_eq!(factorize_real(384), [2, 4, 4, 4, 3]);
        assert_eq!(factorize_real(375), [3, 5, 5, 5]);
        assert_eq!(factorize_real(100), [4, 5, 5]);
        assert_eq!(factorize_real(6), [2, 3]);
    }

    #[test]
    fn good_sizes_are_products_of_2_3_and_5() {
        let sizes = [
            (1, 1),
            (6, 6),
            (7, 8),
            (11, 12),
            (362, 375),
            (382, 384),
            (642, 648),
            (662, 675),
            (98, 100),
        ];
        for (n, want) in sizes {
            assert_eq!(good_size_real(n), want, "{n}");
        }
    }

    #[test]
    fn complex_plans_match_the_definition() {
        for n in [
            1usize, 2, 3, 4, 5, 6, 8, 9, 10, 12, 16, 20, 24, 25, 30, 36, 40, 60, 64, 72, 100, 120, 320, 360,
        ] {
            let (re, im) = (noise(n, 3 * n as u64), noise(n, 7 * n as u64 + 1));
            let x: Vec<(f64, f64)> = re.iter().zip(&im).map(|(&a, &b)| (a, b)).collect();
            for (fwd, sign) in [(true, -1.0), (false, 1.0)] {
                let want = dft(&x, sign);
                let plan: CPlan<f64> = CPlan::new(n, false);
                let mut data: Vec<C<f64>> = x.iter().map(|&(r, i)| C::new(r, i)).collect();
                plan.exec(fwd, &mut data, &mut vec![C::default(); n]);
                for (k, (g, w)) in data.iter().zip(&want).enumerate() {
                    assert!(
                        (g.r - w.0).abs() < 1e-12 && (g.i - w.1).abs() < 1e-12,
                        "n {n} fwd {fwd} k {k}: {g:?} {w:?}"
                    );
                }
                // In float32, with the plan for a transform on its own (the split one for 320 and 360).
                for vectorize in [false, true] {
                    let plan: CPlan<f32> = CPlan::new(n, vectorize);
                    assert_eq!(matches!(plan, CPlan::Quarters { .. }), vectorize && n > 300);
                    let mut data: Vec<C<f32>> = x.iter().map(|&(r, i)| C::new(r as f32, i as f32)).collect();
                    plan.exec(fwd, &mut data, &mut vec![C::default(); n]);
                    for (k, (g, w)) in data.iter().zip(&want).enumerate() {
                        assert!(
                            (f64::from(g.r) - w.0).abs() < 1e-4 && (f64::from(g.i) - w.1).abs() < 1e-4,
                            "n {n} fwd {fwd} vectorize {vectorize} k {k}: {g:?} {w:?}"
                        );
                    }
                }
            }
        }
    }

    /// The same noise down the first and the last of `cols` columns of `rows`; the two columns after `columns`.
    fn first_and_last_column<T: Real>(rows: usize, cols: usize, fwd: bool) -> (Vec<C<T>>, Vec<C<T>>) {
        let (re, im) = (
            noise(rows * cols, rows as u64),
            noise(rows * cols, 31 * rows as u64 + 7),
        );
        let mut a: Vec<C<T>> = re.iter().zip(&im).map(|(&r, &i)| C::new(T::of(r), T::of(i))).collect();
        for y in 0..rows {
            a[y * cols + cols - 1] = a[y * cols];
        }
        columns(&mut a, rows, cols, fwd);
        let column = |x: usize| (0..rows).map(|y| a[y * cols + x]).collect::<Vec<_>>();
        (column(0), column(cols - 1))
    }

    #[test]
    fn a_float64_column_left_over_is_transformed_like_one_in_a_group() {
        // Three columns in float64: two together, one left over. scipy gives the left-over one the same bits.
        assert_eq!((<f64 as Real>::FOUR_LANES, <f64 as Real>::LANES), (false, 2));
        for rows in [320usize, 360, 400, 648, 300, 96] {
            assert!(matches!(CPlan::<f64>::new(rows, true), CPlan::Passes(_)), "{rows}");
            for fwd in [true, false] {
                let (grouped, alone) = first_and_last_column::<f64>(rows, 3, fwd);
                let bits = |c: &[C<f64>]| c.iter().map(|v| (v.r.to_bits(), v.i.to_bits())).collect::<Vec<_>>();
                assert_eq!(bits(&grouped), bits(&alone), "rows {rows} fwd {fwd}");
            }
        }
    }

    #[test]
    fn a_float32_column_left_over_takes_the_split_plan_above_300() {
        // Five columns in float32: four together, one left over.
        for (rows, split) in [
            (320usize, true),
            (360, true),
            (648, true),
            (300, false),
            (288, false),
            (375, false),
        ] {
            assert_eq!(CPlan::<f32>::split(rows), split, "{rows}");
            for fwd in [true, false] {
                let (grouped, alone) = first_and_last_column::<f32>(rows, 5, fwd);
                // The split plan rounds differently (scipy: not one such column in 60 keeps its bits).
                assert_eq!(grouped != alone, split, "rows {rows} fwd {fwd}");
                for (g, a) in grouped.iter().zip(&alone) {
                    assert!((g.r - a.r).abs() < 1e-3 && (g.i - a.i).abs() < 1e-3, "rows {rows}");
                }
            }
        }
    }

    #[test]
    fn real_plans_match_the_definition_and_come_back() {
        for n in [
            1usize, 2, 3, 4, 5, 6, 8, 9, 10, 12, 15, 16, 20, 25, 30, 32, 45, 50, 60, 100, 125, 375, 384,
        ] {
            let x = noise(n, n as u64);
            let want = dft(&x.iter().map(|&v| (v, 0.0)).collect::<Vec<_>>(), -1.0);
            let plan: RPlan<f64> = RPlan::new(n);
            let mut data = x.clone();
            let mut scratch = vec![0.0; n];
            plan.exec(true, &mut data, &mut scratch);
            // r0, r1, i1, r2, i2, ...
            assert!((data[0] - want[0].0).abs() < 1e-12, "n {n}");
            for k in 1..=(n - 1) / 2 {
                assert!((data[2 * k - 1] - want[k].0).abs() < 1e-12, "n {n} k {k}");
                assert!((data[2 * k] - want[k].1).abs() < 1e-12, "n {n} k {k}");
            }
            if n % 2 == 0 && n > 1 {
                assert!((data[n - 1] - want[n / 2].0).abs() < 1e-12, "n {n}");
            }
            plan.exec(false, &mut data, &mut scratch);
            for (g, w) in data.iter().zip(&x) {
                assert!((g / n as f64 - w).abs() < 1e-13, "n {n}");
            }
        }
    }

    #[test]
    #[should_panic(expected = "the factor 7")]
    fn other_factors_are_refused() {
        let _: RPlan<f64> = RPlan::new(14);
    }

    #[test]
    fn convolving_with_a_box_sums_the_neighbourhood() {
        let (w, h) = (9, 7);
        let a = Img::from_vec(w, h, 1, (0..w * h).map(|i| ((i * 37) % 11) as f32).collect());
        let kern = Img::from_vec(3, 3, 1, vec![1.0; 9]);
        let got = fftconvolve_same(&a, &kern);
        assert_eq!((got.w, got.h, got.c), (w, h, 1));
        for y in 0..h as i64 {
            for x in 0..w as i64 {
                let mut want = 0.0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (yy, xx) = (y + dy, x + dx);
                        if (0..h as i64).contains(&yy) && (0..w as i64).contains(&xx) {
                            want += f64::from(a.px(xx as usize, yy as usize)[0]);
                        }
                    }
                }
                let g = got.px(x as usize, y as usize)[0];
                assert!((g - want).abs() < 1e-4, "({x}, {y}): {g} against {want}");
            }
        }
    }

    #[test]
    fn a_round_trip_through_both_transforms_is_the_identity() {
        let (rows, cols) = (12, 10);
        let x = noise(rows * cols, 5);
        let spectrum = rfftn_f64(&x, (rows, cols), (rows, cols));
        assert_eq!(spectrum.len(), rows * (cols / 2 + 1));
        let back = irfftn_f64(&spectrum, (rows, cols));
        for (g, w) in back.iter().zip(&x) {
            assert!((g - w).abs() < 1e-14);
        }
        // Padding: the transform of the padded array.
        let padded = rfftn_f64(&x[..6], (2, 3), (4, 5));
        let mut big = vec![0.0; 20];
        big[..3].copy_from_slice(&x[..3]);
        big[5..8].copy_from_slice(&x[3..6]);
        assert_eq!(padded, rfftn_f64(&big, (4, 5), (4, 5)));
    }
}
