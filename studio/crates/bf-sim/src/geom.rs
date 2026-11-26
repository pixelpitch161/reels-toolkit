//! Plane geometry as `fight.py` does it: the helpers above `class Ball` (`_wrap`, `_swept`, `_seg_pt`, `_seg_seg`,
//! lines 167 to 213) and the numpy operations on 2-vectors the engine uses, each with numpy's order of operations.
//!
//! A position or a velocity is a [`V2`], Python's `np.array([x, y])`. numpy arithmetic on such an array is element by
//! element, so `v / n * s` is `[(v[0] / n) * s, (v[1] / n) * s]`; the functions here spell that out once.

use bf_core::{dmath, py};

/// A 2-vector: `np.array([x, y])`.
pub type V2 = [f64; 2];

/// `a + b`.
pub fn add(a: V2, b: V2) -> V2 {
    [a[0] + b[0], a[1] + b[1]]
}

/// `a - b`.
pub fn sub(a: V2, b: V2) -> V2 {
    [a[0] - b[0], a[1] - b[1]]
}

/// `a * s` (also `s * a`).
pub fn scale(a: V2, s: f64) -> V2 {
    [a[0] * s, a[1] * s]
}

/// `a / s`.
pub fn div(a: V2, s: f64) -> V2 {
    [a[0] / s, a[1] / s]
}

/// `float(a @ b)` and `np.dot(a, b)` for two 2-vectors, as numpy 2.5.3 computes them on the reference PC: its
/// BLAS adds the second product to the first in one fused step, `fma(a[1], b[1], a[0] * b[0])` (200,000 of 200,000
/// pairs; the plain `a[0] * b[0] + a[1] * b[1]` is numpy's on only 74 %). Ruling M3-R39. This is for the kit's `@`
/// and `np.dot` on whole vectors, rows and columns alike; a line the kit writes out with scalars
/// (`a[0] * b[0] + a[1] * b[1]`), `(a * b).sum()`, `np.einsum` and a vector seen backwards (`a[::-1] @ b`) are the
/// plain sum, and a matrix times a vector is [`dot_mv`].
pub fn dot(a: V2, b: V2) -> f64 {
    dmath::fma(a[1], b[1], a[0] * b[0])
}

/// One row of `M @ b` for an array `M` of `rows` 2-vectors (the keys, the balls) and a 2-vector `b`, as numpy
/// computes it on the reference PC: with two rows or more its BLAS fuses the other way round,
/// `fma(m[0], b[0], m[1] * b[1])`; a single row is the plain [`dot`]. (`b @ M` for a `(2, n)` array mixes both
/// orders along its length past two columns: no fight line does that, and a port that meets one measures it first.)
pub fn dot_mv(m: V2, b: V2, rows: usize) -> f64 {
    if rows >= 2 {
        dmath::fma(m[0], b[0], m[1] * b[1])
    } else {
        dot(m, b)
    }
}

/// `np.linalg.norm(a)` for a 2-vector: the root of numpy's own dot product, `sqrt(fma(a[1], a[1], a[0] * a[0]))`
/// on the reference PC (ruling M3-R39). It is not a `hypot` (see [`hypot`]), and `np.linalg.norm(M, axis=1)` is
/// not this either: along an axis numpy takes the plain `sqrt(x * x + y * y)`.
pub fn norm(a: V2) -> f64 {
    dot(a, a).sqrt()
}

/// `float(np.hypot(*a))`, and one element of `np.hypot(*arr.T)`: numpy's `hypot` (`dmath::np_hypot`), which for
/// every distance of a fight is the plain `sqrt(a[0] * a[0] + a[1] * a[1])`. It is not `math.hypot` (CPython's own,
/// `dmath::py_hypot`), which `fight.py` never calls, and not `np.linalg.norm` ([`norm`]): the three differ in the
/// last bit, and the kit's line decides which one a port takes (ruling M3-R35).
pub fn hypot(a: V2) -> f64 {
    dmath::np_hypot(a[0], a[1])
}

/// `math.radians(deg)`: `deg * (pi / 180.0)`, the constant formed first, as CPython does.
pub fn radians(deg: f64) -> f64 {
    deg * (dmath::PI / 180.0)
}

/// Python's `max(a, b)` and `min(a, b)` on floats (one home: `bf_core::py`).
pub use bf_core::py::{py_max, py_min};

/// `np.clip(x, lo, hi)` on a float with plain numbers as bounds: numpy's loop is `if x < lo: x = lo`, then
/// `if x > hi: x = hi`, so `hi` wins when the bounds cross and NaN stays NaN.
pub fn np_clip(x: f64, lo: f64, hi: f64) -> f64 {
    let x = if x < lo { lo } else { x };
    if x > hi { hi } else { x }
}

/// `np.sign(x)`: -1.0, 0.0 or 1.0, and NaN for NaN.
pub fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else if x == 0.0 {
        0.0
    } else {
        f64::NAN
    }
}

/// `_wrap(a)`: `(a + math.pi) % (2 * math.pi) - math.pi`, an angle brought into [-pi, pi).
pub fn wrap(a: f64) -> f64 {
    py::wrap_pi(a)
}

/// `_swept(th0, dth, phi, half)`: does a blade sweeping from `th0` by `dth` (signed) cross the angular interval
/// `phi +- half`?
pub fn swept(th0: f64, dth: f64, phi: f64, half: f64) -> bool {
    let (th0, dth) = if dth < 0.0 { (th0 + dth, -dth) } else { (th0, dth) };
    let d = py::fmod_py(phi - half - th0, 2.0 * dmath::PI);
    d <= dth || d >= 2.0 * dmath::PI - 2.0 * half
}

/// `_seg_pt(p, a, b)`: the closest point to `p` on the segment `a`-`b`.
pub fn seg_pt(p: V2, a: V2, b: V2) -> V2 {
    let ab = sub(b, a);
    let n = dot(ab, ab);
    let s = if n < 1e-12 {
        0.0
    } else {
        py_min(1.0, py_max(0.0, dot(sub(p, a), ab) / n))
    };
    add(a, scale(ab, s))
}

/// `_seg_seg(p0, p1, q0, q1)`: (distance, point on `p0`-`p1` nearest `q0`-`q1`) between two segments: 0 where they
/// cross, else the closest of the four endpoint-to-segment distances (the first of equals, in Python's order).
pub fn seg_seg(p0: V2, p1: V2, q0: V2, q1: V2) -> (f64, V2) {
    let (r, s) = (sub(p1, p0), sub(q1, q0));
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() > 1e-12 {
        let qp = sub(q0, p0);
        let u = (qp[0] * s[1] - qp[1] * s[0]) / den;
        let v = (qp[0] * r[1] - qp[1] * r[0]) / den;
        if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
            return (0.0, add(p0, scale(r, u)));
        }
    }
    let mut best: Option<(f64, V2)> = None;
    for (p, on_p) in [(p0, true), (p1, true), (q0, false), (q1, false)] {
        let (dd, cp) = if on_p {
            let c = seg_pt(p, q0, q1);
            (hypot(sub(p, c)), p)
        } else {
            let c = seg_pt(p, p0, p1);
            (hypot(sub(p, c)), c)
        };
        if best.is_none_or(|(d, _)| dd < d) {
            best = Some((dd, cp));
        }
    }
    best.expect("four candidates were tried")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every expected value below is what fight.py gave on the reference PC (CPython 3.13, numpy 2.5).

    /// numpy's products of 2-vectors on the reference PC (ruling M3-R39), as bits:
    /// `a[0], a[1], b[0], b[1], float(a @ b), float(np.linalg.norm(a)), (np.array([a, b]) @ b)[0],
    /// (np.array([a]) @ b)[0]`. The first nine rows are pairs where the plain sum is not numpy's in at least one
    /// of them; a matrix of two rows or more fuses the other way round than a vector does.
    #[test]
    fn products_are_numpys() {
        #[rustfmt::skip]
        const ROWS: [[u64; 8]; 12] = [
        [0xbfd837b238b8e958, 0x3fe345603944b3f0, 0xc00729ea2d7019ff, 0xbfe523b740b56778, 0x3fe65477fb05f966, 0x3fe6c26d66effe5f, 0x3fe65477fb05f967, 0x3fe65477fb05f966],
        [0xc08eaf7678118be5, 0xc060108563bb0e50, 0xc0043c25b00071b3, 0x3fff8ac5a6ce664c, 0x40a16ca1b6a8d23f, 0x408ef27559f69875, 0x40a16ca1b6a8d240, 0x40a16ca1b6a8d23f],
        [0x3fef135d16c5a982, 0xbfdfd2ff613e3c10, 0xc003e60502960b34, 0xbffb7d1160d94f7e, 0xbff8fa8a24406dff, 0x3ff174d07208cb2b, 0xbff8fa8a24406dff, 0xbff8fa8a24406dff],
        [0x4094335f422ee9b6, 0x4094a906ef4698ea, 0xbff10968931adfbf, 0x4005aa4ef944ddbc, 0x40a13887ebd618d8, 0x409ce516499578e3, 0x40a13887ebd618d8, 0x40a13887ebd618d8],
        [0x3fec549d29dc116c, 0x3fb8fc912922bad0, 0xbfe1054807521598, 0x3ffd2ab14a78dcf8, 0xbfd2c02439d51768, 0x3fec808e35b0d89b, 0xbfd2c02439d51769, 0xbfd2c02439d51768],
        [0x40933a1966970956, 0xc08e5f0cad7c7f51, 0x3ffe7d7e18bcd488, 0xbff13d0e323776db, 0x40aa801739d12931, 0x409880274bb70b00, 0x40aa801739d12930, 0x40aa801739d12931],
        [0x3fe422ddf1fe8d0a, 0xbfcd21a9ef026388, 0x3fd7053da3d155f0, 0x3ff2346488c5b498, 0xbfa0b23071152ed7, 0x3fe569ab63715430, 0xbfa0b23071152ed5, 0xbfa0b23071152ed7],
        [0x408cef0060c04f10, 0xc082e0bee82c6724, 0xc00438655677d200, 0x4007792be106f310, 0xc0b010b18494066e, 0x40914614046c9a10, 0xc0b010b18494066f, 0xc0b010b18494066e],
        [0xbfc98ec390f717a8, 0x3fe250dac1dad758, 0x3ff5d65fc1be8c28, 0x40051751e4b93fc4, 0x3ff3c89734aa3c6c, 0x3fe365f7e620e2a0, 0x3ff3c89734aa3c6b, 0x3ff3c89734aa3c6c],
        [0xc0705b2a1b3e3564, 0x409018640c1d2fbc, 0x3f8d364fc8cc6a00, 0x3ff9d200712134d8, 0x4099ea6e8512d326, 0x40909b47fe839be5, 0x4099ea6e8512d326, 0x4099ea6e8512d326],
        [0x3fef0de6552665aa, 0xbfe184d4ec4b6204, 0xbfd3df61d5c0aed0, 0x3ff5fa95b77e7d54, 0xbff0da9d95cc4bda, 0x3ff1d3d798923865, 0xbff0da9d95cc4bda, 0xbff0da9d95cc4bda],
        [0x409310882a3b320e, 0x40920072add89b7c, 0x3ff1b8674965a5ec, 0x3ffd1a5fe9efe8d4, 0x40aaede32dfb0ee4, 0x409a387ad00bbefa, 0x40aaede32dfb0ee5, 0x40aaede32dfb0ee4],
        ];
        let (mut plain_differs, mut orders_differ) = (0, 0);
        for row in ROWS {
            let v = row.map(f64::from_bits);
            let (a, b) = ([v[0], v[1]], [v[2], v[3]]);
            assert_eq!(dot(a, b).to_bits(), row[4], "{a:?} @ {b:?}");
            assert_eq!(dot(b, a).to_bits(), row[4], "{b:?} @ {a:?}");
            assert_eq!(norm(a).to_bits(), row[5], "norm({a:?})");
            assert_eq!(dot_mv(a, b, 2).to_bits(), row[6], "a row of two: {a:?} @ {b:?}");
            assert_eq!(dot_mv(a, b, 7).to_bits(), row[6], "a row of seven: {a:?} @ {b:?}");
            assert_eq!(dot_mv(a, b, 1).to_bits(), row[7], "the only row: {a:?} @ {b:?}");
            plain_differs += usize::from((a[0] * b[0] + a[1] * b[1]).to_bits() != row[4]);
            orders_differ += usize::from(row[6] != row[4]);
        }
        assert!(
            plain_differs >= 3 && orders_differ >= 3,
            "{plain_differs} {orders_differ}"
        );
    }

    #[test]
    fn wrap_matches_python() {
        for (a, want) in [
            (0.0, 0.0),
            (3.5, -2.7831853071795862),
            (-3.5, 2.7831853071795862),
            (dmath::PI, -dmath::PI),
            (-dmath::PI, -dmath::PI),
            (7.0, 0.7168146928204138),
            (-7.0, -0.7168146928204138),
            (100.0, -0.5309649148733762),
        ] {
            assert_eq!(wrap(a).to_bits(), f64::to_bits(want), "_wrap({a})");
        }
    }

    /// `np.clip` with plain numbers as bounds: -0.0 is not below 0, so it passes; the upper bound wins when the
    /// bounds cross; NaN stays NaN.
    #[test]
    fn np_clip_matches_numpy() {
        let xs = [-1.0, 0.5, 2.0, f64::NAN, -0.0, 0.0, 1.0, -0.2, -0.3];
        for (lo, hi, want) in [
            (0.0, 1.0, [0.0, 0.5, 1.0, f64::NAN, -0.0, 0.0, 1.0, 0.0, 0.0]),
            (0.0, -0.2, [-0.2, -0.2, -0.2, f64::NAN, -0.2, -0.2, -0.2, -0.2, -0.2]),
            (0.0, 0.0, [0.0, 0.0, 0.0, f64::NAN, -0.0, 0.0, 0.0, 0.0, 0.0]),
        ] {
            for (x, want) in xs.into_iter().zip(want) {
                let got = np_clip(x, lo, hi);
                if want.is_nan() {
                    assert!(got.is_nan(), "np.clip({x}, {lo}, {hi}) = {got}");
                } else {
                    assert_eq!(got.to_bits(), want.to_bits(), "np.clip({x}, {lo}, {hi}) = {got}");
                }
            }
        }
    }

    #[test]
    fn swept_matches_python() {
        for ((th0, dth, phi, half), want) in [
            ((0.1, 0.2, 0.25, 0.05), true),
            ((0.1, -0.2, 6.2, 0.05), true),
            ((6.2, 0.2, 0.05, 0.02), true),
            ((1.0, 0.01, 2.0, 0.3), false),
            ((1.0, 0.5, 1.2, 0.3), true),
            ((0.0, -0.3, 6.0, 0.01), true),
            ((3.0, 0.1, 3.2, 0.05), false),
            ((3.0, 0.1, 3.13, 0.05), true),
        ] {
            assert_eq!(swept(th0, dth, phi, half), want, "_swept({th0}, {dth}, {phi}, {half})");
        }
    }

    #[test]
    fn seg_pt_matches_python() {
        for (p, a, b, want) in [
            ([0.5, 1.0], [0.0, 0.0], [2.0, 0.0], [0.5, 0.0]),
            ([-1.0, 1.0], [0.0, 0.0], [2.0, 0.0], [0.0, 0.0]),
            ([3.0, 1.0], [0.0, 0.0], [2.0, 0.0], [2.0, 0.0]),
            (
                [0.3, 0.7],
                [0.1, 0.2],
                [0.9, 1.3],
                [0.407027027027027, 0.6221621621621621],
            ),
            // a segment with no length is its own point
            ([0.3, 0.7], [0.1, 0.2], [0.1, 0.2], [0.1, 0.2]),
        ] {
            let got = seg_pt(p, a, b);
            assert!(
                (got[0] - want[0]).abs() <= 1e-15 && (got[1] - want[1]).abs() <= 1e-15,
                "_seg_pt({p:?}, {a:?}, {b:?}) = {got:?}, Python gave {want:?}"
            );
        }
    }

    #[test]
    fn seg_seg_matches_python() {
        let cases: [(V2, V2, V2, V2, f64, V2); 6] = [
            // crossing: distance 0 at the crossing point
            ([0.0, 0.0], [2.0, 0.0], [1.0, -1.0], [1.0, 1.0], 0.0, [1.0, 0.0]),
            // q's end is nearest: the point returned is on p
            ([0.0, 0.0], [2.0, 0.0], [1.0, 0.5], [1.0, 1.5], 0.5, [1.0, 0.0]),
            // p's end is nearest: that end
            (
                [0.0, 0.0],
                [2.0, 0.0],
                [3.0, 1.0],
                [4.0, 2.0],
                std::f64::consts::SQRT_2, // 1.4142135623730951
                [2.0, 0.0],
            ),
            // parallel: the first of the equal candidates, in Python's order
            ([0.0, 0.0], [2.0, 0.0], [0.5, 0.3], [1.5, 0.3], 0.3, [0.5, 0.0]),
            // p is a point
            (
                [0.3, 0.3],
                [0.3, 0.3],
                [0.0, 0.0],
                [1.0, 0.1],
                0.2686600413566971,
                [0.3, 0.3],
            ),
            (
                [0.1, 0.2],
                [1.3, 0.9],
                [0.4, 1.1],
                [1.2, 0.1],
                0.0,
                [0.7954545454545454, 0.6056818181818182],
            ),
        ];
        for (p0, p1, q0, q1, want_d, want_c) in cases {
            let (d, c) = seg_seg(p0, p1, q0, q1);
            assert!(
                (d - want_d).abs() <= 1e-15 && (c[0] - want_c[0]).abs() <= 1e-15 && (c[1] - want_c[1]).abs() <= 1e-15,
                "_seg_seg({p0:?}, {p1:?}, {q0:?}, {q1:?}) = ({d}, {c:?}), Python gave ({want_d}, {want_c:?})"
            );
        }
    }

    #[test]
    fn small_helpers_follow_python() {
        // math.radians(38.0) and math.radians(-5.0)
        assert_eq!(radians(38.0).to_bits(), 0.6632251157578453f64.to_bits());
        assert_eq!(radians(-5.0).to_bits(), (-0.08726646259971647f64).to_bits());
        // max and min keep the first argument unless the second beats it
        assert_eq!(py_max(0.0, -1.0), 0.0);
        assert_eq!(py_max(0.0, 2.0), 2.0);
        assert_eq!(py_max(0.0, f64::NAN), 0.0);
        assert_eq!(py_min(1.0, 0.5), 0.5);
        assert_eq!(py_min(1.0, 3.0), 1.0);
        assert_eq!((sign(-3.0), sign(0.0), sign(-0.0), sign(2.0)), (-1.0, 0.0, 0.0, 1.0));
        assert!(sign(f64::NAN).is_nan());
        // np.linalg.norm([3, 4]) and np.hypot(3, 4)
        assert_eq!((norm([3.0, 4.0]), hypot([3.0, 4.0])), (5.0, 5.0));
        // np.hypot(x, y) on the reference PC, where math.hypot(x, y) is one step away (the second number)
        for (x, y, np_hypot, math_hypot) in [
            (-1.613, -1.996, 0x400487bcfcce3c1fu64, 0x400487bcfcce3c20u64),
            (1.252, -1.349, 0x3ffd7289999e0598, 0x3ffd7289999e0597),
        ] {
            assert_eq!(hypot([x, y]).to_bits(), np_hypot, "np.hypot({x}, {y})");
            assert_ne!(dmath::py_hypot(x, y).to_bits(), np_hypot);
            assert_eq!(dmath::py_hypot(x, y).to_bits(), math_hypot, "math.hypot({x}, {y})");
        }
        assert_eq!(dot([1.0, 2.0], [3.0, 4.0]), 11.0);
        assert_eq!(div(scale(add([1.0, 2.0], [3.0, 4.0]), 2.0), 4.0), [2.0, 3.0]);
    }
}
