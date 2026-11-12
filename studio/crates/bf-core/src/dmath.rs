//! Deterministic maths: every transcendental the engine uses, with the same bits on every PC.
//!
//! These are thin wrappers over the `libm` crate (a pure-Rust port of musl's maths library), pinned in the workspace
//! with `default-features = false` (no `arch` feature) and `force-soft-floats`, so no CPU intrinsic and no platform
//! maths library is ever involved. This is the one file the std-maths scan (`tests/no_std_maths.rs`) skips, and the
//! only place that may call `libm`; everything else calls these. `sqrt` is not here: `f64::sqrt` is correctly rounded
//! by IEEE 754 and may be used directly.

/// pi, as Python's `math.pi`.
pub const PI: f64 = std::f64::consts::PI;
/// 2 pi, as Python's `2 * math.pi` (the doubling is exact).
pub const TAU: f64 = std::f64::consts::TAU;

/// Sine (`math.sin`).
pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}

/// Cosine (`math.cos`).
pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}

/// Tangent (`math.tan`).
pub fn tan(x: f64) -> f64 {
    libm::tan(x)
}

/// Arc sine (`math.asin`); NaN outside [-1, 1].
pub fn asin(x: f64) -> f64 {
    libm::asin(x)
}

/// Arc cosine (`math.acos`); NaN outside [-1, 1].
pub fn acos(x: f64) -> f64 {
    libm::acos(x)
}

/// Arc tangent (`math.atan`).
pub fn atan(x: f64) -> f64 {
    libm::atan(x)
}

/// The angle of the point (x, y) (`math.atan2(y, x)`).
pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

/// sqrt(x * x + y * y) without overflow: musl's `hypot`. It is NEITHER of Python's two: on the reference PC it
/// differs in the last bit from `math.hypot` for about 12 % of ordinary pairs and from `np.hypot` for about 9 %
/// (ruling M3-R35). A port of a kit line takes [`np_hypot`] for `np.hypot` and [`py_hypot`] for `math.hypot`; this
/// one stays for the callers no gate has shown to differ.
pub fn hypot(x: f64, y: f64) -> f64 {
    libm::hypot(x, y)
}

/// `np.hypot(x, y)` for `float64` as numpy 2.5.3 computes it on the reference PC, for scalars and for arrays of
/// any length, stride and alignment alike (numpy's loop calls the C runtime's `hypot` once per element): the plain
/// `sqrt(x * x + y * y)`, taken on the two numbers scaled by one power of two when a square would leave the
/// normal range (so nothing overflows and a tiny pair keeps its digits). An infinity wins over a NaN.
///
/// Measured against Python: equal on all 560,018 pairs tried (ordinary, equal, zero, subnormal, huge, any ratio);
/// `tests/hypot_kinds.rs` holds a sample.
pub fn np_hypot(x: f64, y: f64) -> f64 {
    if x.is_infinite() || y.is_infinite() {
        return f64::INFINITY;
    }
    let (xx, yy) = (x * x, y * y);
    let sum = xx + yy;
    // No square was rounded on its way below the normal range and the sum did not overflow: scaling by a power
    // of two would change nothing, so this is the whole function for every distance a fight meets.
    if sum.is_finite() && (xx >= f64::MIN_POSITIVE || x == 0.0) && (yy >= f64::MIN_POSITIVE || y == 0.0) {
        return sum.sqrt();
    }
    if sum.is_nan() {
        return f64::NAN;
    }
    let (_, e) = libm::frexp(if y.abs() > x.abs() { y.abs() } else { x.abs() });
    let (xs, ys) = (libm::scalbn(x, -e), libm::scalbn(y, -e));
    libm::scalbn((xs * xs + ys * ys).sqrt(), e)
}

/// `math.hypot(x, y)` as CPython 3.13 computes it (`vector_norm` in `Modules/mathmodule.c`): the squares are taken
/// without error (a fused multiply-add gives each one's lost part), summed with their lost parts kept, and the root
/// is corrected once. Equal to Python on all 560,018 pairs tried; `tests/hypot_kinds.rs` holds a sample.
pub fn py_hypot(x: f64, y: f64) -> f64 {
    let (a, b) = (x.abs(), y.abs());
    if a.is_infinite() || b.is_infinite() {
        return f64::INFINITY;
    }
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    vector_norm([a, b], if b > a { b } else { a })
}

/// CPython's `vector_norm` for two coordinates, already absolute and finite; `max` is the larger.
fn vector_norm(v: [f64; 2], max: f64) -> f64 {
    // (x * y, the part of the product the rounding lost)
    fn dl_mul(x: f64, y: f64) -> (f64, f64) {
        let z = x * y;
        (z, libm::fma(x, y, -z))
    }
    // (a + b, the part of the sum the rounding lost), for |a| >= |b|
    fn dl_fast_sum(a: f64, b: f64) -> (f64, f64) {
        let x = a + b;
        (x, (a - x) + b)
    }
    if max == 0.0 {
        return max;
    }
    let (_, max_e) = libm::frexp(max);
    if max_e < -1023 {
        // ldexp(1.0, -max_e) would overflow: lift the pair out of the subnormals first (exact)
        let tiny = f64::MIN_POSITIVE;
        return tiny * vector_norm([v[0] / tiny, v[1] / tiny], max / tiny);
    }
    let scale = libm::scalbn(1.0, -max_e);
    let (mut csum, mut frac1, mut frac2) = (1.0f64, 0.0f64, 0.0f64);
    for x in v {
        let x = x * scale;
        let pr = dl_mul(x, x);
        let sm = dl_fast_sum(csum, pr.0);
        csum = sm.0;
        frac1 += pr.1;
        frac2 += sm.1;
    }
    let mut h = (csum - 1.0 + (frac1 + frac2)).sqrt();
    let pr = dl_mul(-h, h);
    let sm = dl_fast_sum(csum, pr.0);
    csum = sm.0;
    frac1 += pr.1;
    frac2 += sm.1;
    let x = csum - 1.0 + (frac1 + frac2);
    h += x / (2.0 * h);
    h / scale
}

/// e to the power x (`math.exp`).
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}

/// Natural logarithm (`math.log` with one argument).
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}

/// Base-10 logarithm (`math.log10`).
pub fn log10(x: f64) -> f64 {
    libm::log10(x)
}

/// Base-2 logarithm (`math.log2`).
pub fn log2(x: f64) -> f64 {
    libm::log2(x)
}

/// x to the power y (`x ** y` on floats, `math.pow`). A whole square `x ** 2` is ported as `x * x`, not this.
pub fn pow(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}

/// Hyperbolic tangent (`math.tanh`, `np.tanh`).
pub fn tanh(x: f64) -> f64 {
    libm::tanh(x)
}

/// Hyperbolic sine (`math.sinh`).
pub fn sinh(x: f64) -> f64 {
    libm::sinh(x)
}

/// C `fmod`: the exact remainder of x / y with the sign of x. `bf_core::py::fmod_py` builds Python's `%` on it.
pub fn fmod(x: f64, y: f64) -> f64 {
    libm::fmod(x, y)
}

/// `x * y + z` rounded once (C `fma`), in software like everything here, so the same bits on every PC. The engine
/// does not fuse its own arithmetic; this is only for a port of a numpy product that OpenBLAS computes with fused
/// multiply-add on the reference PC and whose last bit reaches a pixel (see `bf_picture::sprite`).
pub fn fma(x: f64, y: f64, z: f64) -> f64 {
    libm::fma(x, y, z)
}

/// Single-precision sine (numpy `float32` arrays).
pub fn sinf(x: f32) -> f32 {
    libm::sinf(x)
}

/// Single-precision cosine (numpy `float32` arrays).
pub fn cosf(x: f32) -> f32 {
    libm::cosf(x)
}

/// Single-precision e to the power x (numpy `float32` arrays).
pub fn expf(x: f32) -> f32 {
    libm::expf(x)
}

/// Single-precision x to the power y (numpy `float32` arrays).
pub fn powf(x: f32, y: f32) -> f32 {
    libm::powf(x, y)
}
