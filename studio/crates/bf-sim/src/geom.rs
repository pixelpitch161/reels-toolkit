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

/// `float(a @ b)`: the dot product, `a[0] * b[0] + a[1] * b[1]`.
pub fn dot(a: V2, b: V2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

/// `np.linalg.norm(a)`: `sqrt(a @ a)` (numpy takes the root of the dot product; it is not `hypot`).
pub fn norm(a: V2) -> f64 {
    dot(a, a).sqrt()
}

/// `float(np.hypot(*a))`.
pub fn hypot(a: V2) -> f64 {
    dmath::hypot(a[0], a[1])
}

/// `math.radians(deg)`: `deg * (pi / 180.0)`, the constant formed first, as CPython does.
pub fn radians(deg: f64) -> f64 {
    deg * (dmath::PI / 180.0)
}

/// Python's `max(a, b)` on floats: `a` unless `b` is greater.
pub fn py_max(a: f64, b: f64) -> f64 {
    if b > a { b } else { a }
}

/// Python's `min(a, b)` on floats: `a` unless `b` is smaller.
pub fn py_min(a: f64, b: f64) -> f64 {
    if b < a { b } else { a }
}

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
        assert_eq!(dot([1.0, 2.0], [3.0, 4.0]), 11.0);
        assert_eq!(div(scale(add([1.0, 2.0], [3.0, 4.0]), 2.0), 4.0), [2.0, 3.0]);
    }
}
