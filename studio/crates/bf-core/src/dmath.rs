//! Deterministic maths: every transcendental the engine uses, with the bits Python gives on the reference PC
//! (ruling M3-R38).
//!
//! Python's `math.sin`, `math.atan2`, `x ** y` and the rest call the Windows C runtime (`ucrtbase.dll`), and so do
//! numpy's `float64` `np.sin`, `np.exp`, `np.arctan2`, `np.log10`, `np.arcsin` and `np.power` there (numpy 2.5.3 has
//! its own code only for `np.tanh`, see [`np_tanh`], and for `np.hypot` it calls a different runtime function than
//! `math.hypot` does, see [`np_hypot`] and [`py_hypot`]). The runtime's functions are not correctly rounded and no
//! library the engine may copy gives their bits (musl's differ in the last place on 0.2 % to 17 % of ordinary
//! arguments), so on Windows `dmath` calls the runtime itself (`dmath/crt.rs`): the system's `ucrtbase.dll`, loaded
//! once at run time. That copies nothing, and gives Python's bits wherever the runtime is the reference PC's and
//! takes the same path. [`check`] tests that at start-up against pinned results and says so in plain words when it
//! is not so (a processor without fused multiply-add, a Windows with a different runtime). Elsewhere (no engine
//! target, only tools and tests on other systems) the same functions are musl's, which [`check`] reports too.
//!
//! The `libm` crate (a pure-Rust port of musl, pinned with `default-features = false` and `force-soft-floats`)
//! serves `fmod`, `fma`, `frexp`, `scalbn`, the old [`hypot`], the four single-precision functions, and the
//! fallback away from Windows. This file is the one file the std-maths scan (`tests/no_std_maths.rs`, its
//! `ALLOWED`) lets call `libm` or the std methods; the files in its `dmath/` folder are scanned like every other and
//! call neither. Everything else calls these. `sqrt` is not here: `f64::sqrt` is correctly rounded by IEEE 754 and
//! may be used directly.
//!
//! # PROVENANCE
//!
//! - `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `exp`, `ln`, `log10`, `log2`, `pow`, `tanh`, `sinh`:
//!   not implemented here. On Windows each is the system C runtime's own function, called in `ucrtbase.dll`
//!   (`dmath/crt.rs` holds only the function names); no constant, table or step of it is in this repository. Away
//!   from Windows each is the `libm` crate's (MIT or Apache-2.0; musl, FreeBSD msun).
//! - [`np_tanh`]: numpy 2.5.3's `simd_tanh_f64`, ported in `dmath/np_tanh.rs` with its table `lut18x16` copied from
//!   numpy's source (`numpy/_core/src/umath/loops_hyperbolic.dispatch.cpp.src`); numpy's BSD-3-Clause notice is at
//!   the top of that file.
//! - [`py_hypot`]: CPython 3.13's `vector_norm` (`Modules/mathmodule.c`), rewritten from its description; no
//!   constants. [`np_hypot`]: plain `sqrt(x * x + y * y)` with power-of-two scaling, measured; no constants.
//! - `fmod`, `fma`, [`hypot`], `sinf`, `cosf`, `expf`, `powf`: the `libm` crate (MIT or Apache-2.0). [`fma`] also
//!   uses the processor's instruction, which gives the same bits.
//! - [`check`]'s pins: arguments from the audit's samples with the results Python gave on the reference PC.

#[cfg(windows)]
mod crt;

/// numpy's `np.tanh` with a software fused multiply-add: any processor.
mod soft {
    #[inline(always)]
    fn fma(x: f64, y: f64, z: f64) -> f64 {
        libm::fma(x, y, z)
    }
    include!("dmath/np_tanh.rs");
}

/// The same with the processor's fused multiply-add: the same bits, several times faster.
#[cfg(target_arch = "x86_64")]
mod hard {
    use std::arch::x86_64::{_mm_cvtsd_f64, _mm_fmadd_sd, _mm_set_sd};

    #[target_feature(enable = "fma")]
    #[inline]
    pub(super) fn fma_cpu(x: f64, y: f64, z: f64) -> f64 {
        _mm_cvtsd_f64(_mm_fmadd_sd(_mm_set_sd(x), _mm_set_sd(y), _mm_set_sd(z)))
    }

    #[inline(always)]
    fn fma(x: f64, y: f64, z: f64) -> f64 {
        // SAFETY: this module is entered only through `entry`, which `super::has_fma` guards.
        unsafe { fma_cpu(x, y, z) }
    }
    include!("dmath/np_tanh.rs");

    /// The door into this module: compiled with the instruction switched on.
    pub(super) mod entry {
        #[target_feature(enable = "fma")]
        pub(in super::super) fn np_tanh(x: f64) -> f64 {
            super::np_tanh(x)
        }
    }
}

/// Whether this processor has the fused multiply-add instruction (asked once, then remembered).
#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn has_fma() -> bool {
    std::is_x86_feature_detected!("fma")
}

/// One C runtime function: the system's on Windows, the `libm` crate's elsewhere.
macro_rules! system {
    ($name:ident, $fallback:path, $($arg:ident),+) => {{
        #[cfg(windows)]
        {
            // SAFETY: a C maths function of plain numbers has no precondition; `crt` loaded it with its signature.
            unsafe { (crt::crt().$name)($($arg),+) }
        }
        #[cfg(not(windows))]
        {
            $fallback($($arg),+)
        }
    }};
}

/// pi, as Python's `math.pi`.
pub const PI: f64 = std::f64::consts::PI;
/// 2 pi, as Python's `2 * math.pi` (the doubling is exact).
pub const TAU: f64 = std::f64::consts::TAU;

/// Sine (`math.sin`, `np.sin` on `float64`).
pub fn sin(x: f64) -> f64 {
    system!(sin, libm::sin, x)
}

/// Cosine (`math.cos`, `np.cos` on `float64`).
pub fn cos(x: f64) -> f64 {
    system!(cos, libm::cos, x)
}

/// Tangent (`math.tan`, `np.tan` on `float64`).
pub fn tan(x: f64) -> f64 {
    system!(tan, libm::tan, x)
}

/// Arc sine (`math.asin`, `np.arcsin` on `float64`); NaN outside [-1, 1].
pub fn asin(x: f64) -> f64 {
    system!(asin, libm::asin, x)
}

/// Arc cosine (`math.acos`, `np.arccos` on `float64`); NaN outside [-1, 1].
pub fn acos(x: f64) -> f64 {
    system!(acos, libm::acos, x)
}

/// Arc tangent (`math.atan`, `np.arctan` on `float64`).
pub fn atan(x: f64) -> f64 {
    system!(atan, libm::atan, x)
}

/// The angle of the point (x, y) (`math.atan2(y, x)`, `np.arctan2(y, x)` on `float64`).
pub fn atan2(y: f64, x: f64) -> f64 {
    system!(atan2, libm::atan2, y, x)
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

/// e to the power x (`math.exp`, `np.exp` on `float64`).
pub fn exp(x: f64) -> f64 {
    system!(exp, libm::exp, x)
}

/// Natural logarithm (`math.log` with one argument, `np.log` on `float64`).
pub fn ln(x: f64) -> f64 {
    system!(log, libm::log, x)
}

/// Base-10 logarithm (`math.log10`, `np.log10` on `float64`).
pub fn log10(x: f64) -> f64 {
    system!(log10, libm::log10, x)
}

/// Base-2 logarithm (`math.log2`, `np.log2` on `float64`).
pub fn log2(x: f64) -> f64 {
    system!(log2, libm::log2, x)
}

/// x to the power y: `x ** y` on Python floats and on numpy `float64` scalars, `math.pow`, `np.power`, and a numpy
/// array `a ** y`, EXCEPT that numpy takes a short cut when an ARRAY meets one of these scalar exponents:
/// `a ** 2` is `a * a`, `a ** 0.5` is `sqrt(a)`, `a ** -1` is `1 / a`. A Python float or a numpy scalar squared
/// (`x ** 2`, `np.float64(x) ** 2`) takes no short cut: it is `pow(x, 2.0)`, which is one step from `x * x` for
/// about 1 argument in 2,000 (and `x ** 3`, `x ** 4` differ from the products for a quarter and a half of them).
/// Where Python raises (`0.0 ** -1`, a negative number to a fraction with `math.pow`) this gives the C result
/// (infinity, NaN).
pub fn pow(x: f64, y: f64) -> f64 {
    system!(pow, libm::pow, x, y)
}

/// Hyperbolic tangent as `math.tanh` gives it (the C runtime's). NOT `np.tanh`: that is [`np_tanh`], which
/// differs in the last place for about 1 argument in 9.
pub fn tanh(x: f64) -> f64 {
    system!(tanh, libm::tanh, x)
}

/// `np.tanh` for `float64` as numpy 2.5.3 computes it on the reference PC, for arrays of any length and stride and
/// for numpy scalars alike: numpy's own code (16 intervals of |x|, a polynomial of degree 16 on each, fused), not
/// the C runtime's, ported from numpy's source (BSD-3-Clause, notice in `dmath/np_tanh.rs`). Plain arithmetic, so
/// the same bits on every PC. Equal to numpy on all 3,000,000 arguments tried.
pub fn np_tanh(x: f64) -> f64 {
    #[cfg(target_arch = "x86_64")]
    if has_fma() {
        // SAFETY: the processor has just said it has the instruction this door is compiled with.
        return unsafe { hard::entry::np_tanh(x) };
    }
    soft::np_tanh(x)
}

/// Hyperbolic sine (`math.sinh`, `np.sinh` on `float64`).
pub fn sinh(x: f64) -> f64 {
    system!(sinh, libm::sinh, x)
}

/// C `fmod`: the exact remainder of x / y with the sign of x. `bf_core::py::fmod_py` builds Python's `%` on it.
pub fn fmod(x: f64, y: f64) -> f64 {
    libm::fmod(x, y)
}

/// `x * y + z` rounded once (C `fma`): exact by definition, so the same bits on every PC whether the processor
/// does it (where it has the instruction) or the `libm` crate's software does. For a port of a numpy product that
/// the BLAS computes with fused multiply-add on the reference PC: the fight's 2-vector products (`bf_sim::geom`,
/// ruling M3-R39) and the sprite camera's (`bf_picture::sprite`, ruling M3-R30). The engine fuses nothing else.
pub fn fma(x: f64, y: f64, z: f64) -> f64 {
    #[cfg(target_arch = "x86_64")]
    if has_fma() {
        // SAFETY: the processor has just said it has the instruction.
        return unsafe { hard::fma_cpu(x, y, z) };
    }
    libm::fma(x, y, z)
}

/// Single-precision sine: musl's. NOT numpy's `float32` `np.sin` (numpy has its own code for `float32` sin, cos, exp,
/// log and tanh, which is neither this nor the `float64` result rounded): no engine code calls the four
/// single-precision functions, and a port that needs one measures and ports numpy's first.
pub fn sinf(x: f32) -> f32 {
    libm::sinf(x)
}

/// Single-precision cosine: musl's (see [`sinf`]).
pub fn cosf(x: f32) -> f32 {
    libm::cosf(x)
}

/// Single-precision e to the power x: musl's (see [`sinf`]).
pub fn expf(x: f32) -> f32 {
    libm::expf(x)
}

/// Single-precision x to the power y: musl's (see [`sinf`]).
pub fn powf(x: f32, y: f32) -> f32 {
    libm::powf(x, y)
}

/// What [`check`] found: whether this PC's maths gives the reference PC's bits, and a line saying so in plain words
/// (for the start log and fault reports).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    /// Every pinned result came out bit for bit as on the reference PC.
    pub same: bool,
    /// One plain line: what the maths is here and whether it matches.
    pub text: String,
}

/// Results Python gave on the reference PC, `(function, x, y, result)` as bits (`y` unused by the one-argument
/// functions). Most are arguments where the C runtime's two versions (with and without fused multiply-add) differ,
/// so a runtime that took the other version, or a different runtime, fails here.
#[rustfmt::skip]
const PINS: [(&str, u64, u64, u64); 28] = [
    ("sin", 0x400f09539daf302c, 0, 0xbfe5876c1b7501e8), ("sin", 0xc005e2bfd7b81d3c, 0, 0xbfd944c7a51114f0),
    ("cos", 0xc0112dccbdd9618e, 0, 0xbfd9f5c314b84902), ("cos", 0x3fe750ef22cd2d78, 0, 0x3fe7dff2fc80046a),
    ("tan", 0x4014b6fb43f1150e, 0, 0xbfffc9e2f7ee97cf), ("tan", 0xbfe0874b87748e40, 0, 0xbfe22c995b93c30d),
    ("asin", 0xbfe2502b98e8a32a, 0, 0xbfe37f506c2baec8), ("asin", 0xbfeb91d0a2c69410, 0, 0xbff09cef7bdfafe3),
    ("acos", 0x3fdcc9cd70f0f60c, 0, 0x3ff1aaf33484d715), ("acos", 0x3fe6f24484c0d562, 0, 0x3fe8adaef0c8e6ed),
    ("atan", 0xc00d86e6b709eadc, 0, 0xbff4e63c14d548d0), ("atan", 0x3fda43658ba53e80, 0, 0x3fd8ec115d2ea881),
    ("exp", 0xc01420ede1fa36a8, 0, 0x3f7ab9a9d130cdb6), ("exp", 0xc0305ea1c2177a78, 0, 0x3e74df890db60b80),
    ("ln", 0x40b788aa67653f6d, 0, 0x40216840858d5af3), ("ln", 0x40c35903ff781bfb, 0, 0x402266dc3e0dec21),
    ("log10", 0x40b6f2f4521d9dbf, 0, 0x400e26ebd49a2d5a), ("log10", 0x40b4c12e2847572d, 0, 0x400dcd86b037a3b1),
    ("log2", 0x40c1fcbcb8a12137, 0, 0x402a567a77b6ca92), ("log2", 0x40a703f86e436b8d, 0, 0x40270c8fc8615c86),
    ("tanh", 0xbfef51d993ff5188, 0, 0xbfe814a604c20096), ("tanh", 0x3fed665ffbc97c70, 0, 0x3fe735b0ea996a96),
    ("sinh", 0xc00f78f27f03ea82, 0, 0xc0398c12dc2d931e), ("sinh", 0x4002a4e9bb4431d0, 0, 0x40145f4648f819d3),
    ("atan2", 0xc0945336fda93422, 0x4096339832db575a, 0xbfe7b8bea083e665),
    ("atan2", 0x407387cadf1b8d7c, 0x408a315172ea688c, 0x3fd6d6cdf1c9d449),
    ("pow", 0x4052745862316541, 0xc011bbf6d43f2bd4, 0x3e3667d15e536152),
    ("pow", 0x40559bdca247bd55, 0x400c6849c3735220, 0x415cbda99780ac02),
];

/// Tests this PC's maths against [`PINS`] (a few microseconds; the first call also loads the runtime). Call it
/// once at start-up and write its line where the person and a fault report will see it.
pub fn check() -> Check {
    let mut wrong: Vec<&str> = Vec::new();
    for (name, x, y, want) in PINS {
        let (x, y) = (f64::from_bits(x), f64::from_bits(y));
        let got = match name {
            "sin" => sin(x),
            "cos" => cos(x),
            "tan" => tan(x),
            "asin" => asin(x),
            "acos" => acos(x),
            "atan" => atan(x),
            "exp" => exp(x),
            "ln" => ln(x),
            "log10" => log10(x),
            "log2" => log2(x),
            "tanh" => tanh(x),
            "sinh" => sinh(x),
            "atan2" => atan2(x, y),
            _ => pow(x, y),
        };
        if got.to_bits() != want && !wrong.contains(&name) {
            wrong.push(name);
        }
    }
    let fused = fused_runtime();
    let same = wrong.is_empty();
    let text = match (cfg!(windows), same) {
        (true, true) => "Maths: the Windows C runtime (ucrtbase.dll); every pinned result equals the reference PC's, \
                         so fights compute as they do there."
            .to_string(),
        (true, false) => format!(
            "Maths warning: this PC's Windows C runtime (ucrtbase.dll) gives other results than the reference PC \
             for {} ({}). Fights here can differ slightly from the reference PC's; the replay checks may fail.",
            wrong.join(", "),
            match fused {
                Some(false) =>
                    "it is not using its fused multiply-add version: this processor may lack the instruction",
                _ => "its version differs from the reference PC's",
            }
        ),
        (false, _) => "Maths warning: not on Windows, so the maths is musl's, not the Windows C runtime's; fights \
                       here can differ slightly from the reference PC's."
            .to_string(),
    };
    Check {
        same: same && cfg!(windows),
        text,
    }
}

/// Whether the C runtime took its fused multiply-add versions (`None` where it cannot say or is not used).
fn fused_runtime() -> Option<bool> {
    #[cfg(windows)]
    {
        // SAFETY: `_get_FMA3_enable` takes nothing and returns an int.
        crt::crt().fma3.map(|f| unsafe { f() } != 0)
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed stream of argument pairs: every magnitude, both signs, plenty near 1 and across many turns.
    fn args() -> impl Iterator<Item = (f64, f64)> {
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        (0..60_000).map(move |k| {
            let (a, b) = (next(), next());
            let unit = |v: u64| (v >> 11) as f64 / (1u64 << 53) as f64;
            match k % 6 {
                0 => (f64::from_bits(a), f64::from_bits(b)),
                1 => (unit(a) * 14.0 - 7.0, unit(b) * 14.0 - 7.0),
                2 => (unit(a) * 2.0 - 1.0, unit(b) * 2.0 - 1.0),
                3 => (unit(a) * 3000.0 - 1500.0, unit(b) * 3000.0 - 1500.0),
                4 => (unit(a) * 1.5e3 - 750.0, unit(b) * 60.0),
                _ => (0.9 + unit(a) * 0.2, unit(b) * 1e9 - 5e8),
            }
        })
    }

    fn same(a: f64, b: f64) -> bool {
        a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
    }

    /// The processor's fused multiply-add and the software one give the same bits, alone and through numpy's tanh
    /// (the one function here built on them), so a PC without the instruction computes what the reference PC does.
    #[test]
    #[cfg(target_arch = "x86_64")]
    fn the_processors_fused_multiply_add_changes_no_bit() {
        if !has_fma() {
            return;
        }
        for (x, y) in args() {
            for z in [x * 0.37 - y, -(x * y), y, 1.0] {
                // SAFETY: `has_fma` was checked above.
                let cpu = unsafe { hard::fma_cpu(x, y, z) };
                assert!(same(libm::fma(x, y, z), cpu), "fma({x:e}, {y:e}, {z:e})");
            }
            for v in [x, y] {
                // SAFETY: as above.
                let (s, h) = (soft::np_tanh(v), unsafe { hard::entry::np_tanh(v) });
                assert!(same(s, h), "np_tanh of {v:e}: {s:e} and {h:e}");
            }
        }
    }

    /// On the reference PC the system's runtime gives every pinned result.
    #[test]
    #[cfg(windows)]
    fn the_system_runtime_passes_its_check() {
        let found = check();
        assert!(found.same, "{}", found.text);
    }
}
