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

/// sqrt(x * x + y * y) without overflow (`math.hypot`).
pub fn hypot(x: f64, y: f64) -> f64 {
    libm::hypot(x, y)
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
