//! Python and numpy number semantics, ported operation for operation.
//!
//! Python's `round`, `%` and `//` do not behave like Rust's `f64::round`, `%` and `/`: `round` ties to even,
//! `round(x, n)` rounds the exact decimal value of `x`, `%` takes the sign of the divisor and `//` floors. numpy's
//! `np.round(x, n)` is a different algorithm again (scale, round half to even, unscale). Each function here follows
//! the C code CPython 3.13 or numpy 2.5 runs, and `studio/reference/py/vectors.json` (what Python gave on this PC)
//! checks them bit for bit, minus zero included.

use crate::dmath;

/// Python `round(x)` (as a float) and `np.rint(x)`: the nearest whole number, ties to even. Keeps the sign, so
/// `round_half_even(-0.3)` is `-0.0` (as `np.rint`; Python's `round` returns an int, which has no sign of zero).
/// Infinities and NaN come back unchanged.
pub fn round_half_even(x: f64) -> f64 {
    x.round_ties_even()
}

/// Python `round(x, n)` for a float: the decimal with `n` digits after the point (before it, for negative `n`) that
/// is nearest to the exact binary value of `x`, ties to even, read back as the nearest float. This is CPython's
/// `double_round` (`_Py_dg_dtoa` mode 3, then `_Py_dg_strtod`). So `round_nd(2.675, 2)` is 2.67 (2.675 is really
/// 2.67499999...), and `round_nd(0.125, 2)` is 0.12 (an exact tie). The sign survives: `round_nd(-0.001, 2)` is
/// `-0.0`. Infinities and NaN return unchanged; `n` above 323 returns `x` and below -308 returns `0.0 * x`, as
/// CPython does. Where Python raises `OverflowError` (rounding the largest floats up to the next power of ten) this
/// returns an infinity.
pub fn round_nd(x: f64, n: i32) -> f64 {
    // CPython's NDIGITS_MAX and NDIGITS_MIN: (DBL_MANT_DIG - DBL_MIN_EXP) * 0.30103 and -(DBL_MAX_EXP + 1) * 0.30103.
    const NDIGITS_MAX: i32 = 323;
    const NDIGITS_MIN: i32 = -308;
    if !x.is_finite() || n > NDIGITS_MAX {
        return x;
    }
    if n < NDIGITS_MIN {
        return 0.0 * x;
    }
    let text = if n >= 0 {
        // Rust's fixed-precision formatting is exact: it rounds the exact decimal expansion, ties to even.
        format!("{:.*}", n as usize, x)
    } else {
        round_to_tens(x, n.unsigned_abs() as usize)
    };
    text.parse().expect("a formatted float parses back")
}

/// `x` rounded to a multiple of `10^m` (m >= 1), ties to even, as decimal text with its sign.
fn round_to_tens(x: f64, m: usize) -> String {
    let whole = x.trunc();
    // Exact: `x - trunc(x)` loses nothing, and only whether it is zero matters below.
    let has_fraction = x != whole;
    // An integer-valued float formats with every digit exact.
    let digits = format!("{:.0}", whole.abs());
    let digits = if digits.len() < m + 1 {
        format!("{digits:0>width$}", width = m + 1)
    } else {
        digits
    };
    let (keep, rest) = digits.split_at(digits.len() - m);
    let half = format!("5{}", "0".repeat(m - 1));
    let up = match rest.cmp(half.as_str()) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => has_fraction || keep.as_bytes()[keep.len() - 1] % 2 == 1,
    };
    let mut keep: Vec<u8> = keep.bytes().collect();
    if up {
        let mut k = keep.len();
        loop {
            if k == 0 {
                keep.insert(0, b'1');
                break;
            }
            k -= 1;
            if keep[k] == b'9' {
                keep[k] = b'0';
            } else {
                keep[k] += 1;
                break;
            }
        }
    }
    let sign = if x.is_sign_negative() { "-" } else { "" };
    format!("{sign}{}e{m}", String::from_utf8(keep).expect("ASCII digits"))
}

/// numpy's `power_of_ten` (`multiarray/calculation.c`): a table up to 1e8, then repeated multiplication by 10.
fn np_power_of_ten(n: u32) -> f64 {
    const P10: [f64; 9] = [1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8];
    if n < 9 {
        P10[n as usize]
    } else {
        let mut ret = 1e9;
        for _ in 9..n {
            ret *= 10.0;
        }
        ret
    }
}

/// `np.round(x, n)` on a float64 (`PyArray_Round`): `n == 0` is `np.rint(x)`; `n > 0` is `rint(x * 10^n) / 10^n`;
/// `n < 0` is `rint(x / 10^-n) * 10^-n`. Not correctly rounded: `np_round_nd(2.675, 2)` is 2.68 where Python's
/// `round` gives 2.67.
pub fn np_round_nd(x: f64, n: i32) -> f64 {
    if n == 0 {
        return round_half_even(x);
    }
    let f = np_power_of_ten(n.unsigned_abs());
    if n > 0 {
        round_half_even(x * f) / f
    } else {
        round_half_even(x / f) * f
    }
}

/// Python `a % b` on floats (CPython `float_rem`): the remainder with the sign of `b`; a zero remainder is a zero
/// with the sign of `b`. Python raises `ZeroDivisionError` for `b == 0`; this returns NaN.
pub fn fmod_py(a: f64, b: f64) -> f64 {
    let m = dmath::fmod(a, b);
    if m != 0.0 {
        if (b < 0.0) != (m < 0.0) { m + b } else { m }
    } else {
        0.0f64.copysign(b)
    }
}

/// Python `a // b` on floats (CPython `_float_div_mod`): `(a - fmod(a, b)) / b`, moved down one when the remainder
/// changes sign, then snapped to the nearest whole number; a zero quotient takes the sign of `a / b`. Python raises
/// `ZeroDivisionError` for `b == 0`; this returns NaN or an infinity.
pub fn floor_div(a: f64, b: f64) -> f64 {
    let m = dmath::fmod(a, b);
    let mut div = (a - m) / b;
    if m != 0.0 && (b < 0.0) != (m < 0.0) {
        div -= 1.0;
    }
    if div != 0.0 {
        let mut floordiv = div.floor();
        if div - floordiv > 0.5 {
            floordiv += 1.0;
        }
        floordiv
    } else {
        0.0f64.copysign(a / b)
    }
}

/// Python `a % b` on ints: the remainder with the sign of `b`. Panics for `b == 0` (Python raises).
pub fn imod_py(a: i64, b: i64) -> i64 {
    let r = a % b;
    if r != 0 && (r < 0) != (b < 0) { r + b } else { r }
}

/// Python `a // b` on ints: the quotient rounded down. Panics for `b == 0` (Python raises).
pub fn ifloor_div(a: i64, b: i64) -> i64 {
    let q = a / b;
    if a % b != 0 && (a < 0) != (b < 0) { q - 1 } else { q }
}

/// fight.py `_wrap(a)`: `(a + math.pi) % (2 * math.pi) - math.pi`, an angle brought into [-pi, pi).
pub fn wrap_pi(a: f64) -> f64 {
    fmod_py(a + dmath::PI, 2.0 * dmath::PI) - dmath::PI
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::collections::BTreeSet;

    /// Every case `studio/reference/py/vectors.json` may hold. A name in the file that is not here fails the test.
    const KNOWN: [&str; 12] = [
        "round",
        "round_1",
        "round_2",
        "round_3",
        "round_5",
        "np_round_2",
        "mod",
        "floordiv",
        "rint",
        "wrap_pi",
        "imod",
        "ifloordiv",
    ];

    /// Parses a Python `float.hex()` string exactly: `[-]0x1.<13 hex>p<exp>` (normal), `[-]0x0.<13 hex>p-1022`
    /// (subnormal), `[-]0x0.0p+0` (zero), and `inf`, `-inf`, `nan`.
    fn hex_f64(s: &str) -> f64 {
        match s {
            "inf" => return f64::INFINITY,
            "-inf" => return f64::NEG_INFINITY,
            "nan" => return f64::NAN,
            _ => {}
        }
        let (neg, body) = match s.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, s),
        };
        let body = body
            .strip_prefix("0x")
            .unwrap_or_else(|| panic!("hex float without 0x: {s}"));
        let (mant, exp) = body
            .split_once('p')
            .unwrap_or_else(|| panic!("hex float without p: {s}"));
        let (lead, frac) = mant
            .split_once('.')
            .unwrap_or_else(|| panic!("hex float without '.': {s}"));
        let exp: i64 = exp.parse().unwrap_or_else(|_| panic!("bad exponent: {s}"));
        let bits = match (lead, frac) {
            ("0", "0") => {
                assert_eq!(exp, 0, "zero with an exponent: {s}");
                0u64
            }
            ("1", f) | ("0", f) => {
                assert_eq!(f.len(), 13, "expected 13 fraction digits: {s}");
                let f = u64::from_str_radix(f, 16).unwrap_or_else(|_| panic!("bad fraction: {s}"));
                if lead == "1" {
                    assert!((-1022..=1023).contains(&exp), "exponent out of range: {s}");
                    (((exp + 1023) as u64) << 52) | f
                } else {
                    assert_eq!(exp, -1022, "subnormal with exponent other than -1022: {s}");
                    assert_ne!(f, 0, "subnormal zero: {s}");
                    f
                }
            }
            _ => panic!("bad hex float: {s}"),
        };
        f64::from_bits(bits | if neg { 1 << 63 } else { 0 })
    }

    fn f(v: &Value) -> f64 {
        hex_f64(
            v.as_str()
                .unwrap_or_else(|| panic!("expected a hex float string, got {v}")),
        )
    }

    fn i(v: &Value) -> i64 {
        v.as_i64().unwrap_or_else(|| panic!("expected an integer, got {v}"))
    }

    fn show(x: f64) -> String {
        format!("{x:e} ({:#018x})", x.to_bits())
    }

    #[test]
    fn hex_parser_is_exact() {
        assert_eq!(hex_f64("0x1.999999999999ap-4"), 0.1);
        assert_eq!(hex_f64("-0x0.0p+0").to_bits(), (-0.0f64).to_bits());
        assert_eq!(hex_f64("0x0.0p+0").to_bits(), 0);
        assert_eq!(hex_f64("0x0.0000000000001p-1022"), 5e-324);
        assert_eq!(hex_f64("0x0.fffffffffffffp-1022").to_bits(), 0x000f_ffff_ffff_ffff);
        assert_eq!(hex_f64("0x1.0000000000000p-1022"), f64::MIN_POSITIVE);
        assert_eq!(hex_f64("0x1.fffffffffffffp+1023"), f64::MAX);
        assert_eq!(hex_f64("-0x1.8000000000000p+1"), -3.0);
    }

    /// Cases the vectors do not reach (negative and extreme `n`, overflow), with the values Python 3.13.18 and
    /// numpy 2.5.3 gave on the reference PC.
    #[test]
    fn edge_cases_match_python() {
        let round_cases = [
            (1250.4, -2, "0x1.4500000000000p+10"),
            (1250.0, -2, "0x1.2c00000000000p+10"),
            (1350.0, -2, "0x1.5e00000000000p+10"),
            (-50.0, -2, "-0x0.0p+0"),
            (950.0, -2, "0x1.f400000000000p+9"),
            (-1234.5, -1, "-0x1.3380000000000p+10"),
            (1e300, -300, "0x1.7e43c8800759cp+996"),
            (5e299, -300, "0x1.7e43c8800759cp+996"),
            (1.5e300, -300, "0x1.7e43c8800759cp+997"),
            (123.456, 400, "0x1.edd2f1a9fbe77p+6"),
            (123.456, -400, "0x0.0p+0"),
            (-123.456, -400, "-0x0.0p+0"),
            (0.125, 2, "0x1.eb851eb851eb8p-4"),
            (2.675, 2, "0x1.55c28f5c28f5cp+1"),
            (1.000005, 5, "0x1.0000a7c5ac472p+0"),
        ];
        for (x, n, want) in round_cases {
            assert_eq!(round_nd(x, n).to_bits(), hex_f64(want).to_bits(), "round({x}, {n})");
        }
        let np_cases = [
            (2.675, 2, "0x1.570a3d70a3d71p+1"),
            (1234.5, -1, "0x1.3380000000000p+10"),
            (1e300, 12, "inf"),
            (-0.004, 2, "-0x0.0p+0"),
            (1.23456789e-3, 10, "0x1.43a2730550e4ep-10"),
        ];
        for (x, n, want) in np_cases {
            assert_eq!(
                np_round_nd(x, n).to_bits(),
                hex_f64(want).to_bits(),
                "np.round({x}, {n})"
            );
        }
        // Infinities and NaN come back unchanged and nothing panics.
        for n in [-400, -5, 0, 2, 400] {
            for x in [f64::INFINITY, f64::NEG_INFINITY] {
                assert_eq!(round_nd(x, n), x);
                if n.abs() < 300 {
                    assert_eq!(np_round_nd(x, n), x);
                }
            }
            assert!(round_nd(f64::NAN, n).is_nan());
            assert!(np_round_nd(f64::NAN, n).is_nan());
        }
        // numpy's 10^n overflows to inf for huge n, and inf / inf (or 0 * inf) is NaN: numpy 2.5.3 gives nan for
        // all of these, so the port does too.
        for (x, n) in [
            (f64::INFINITY, -400),
            (1.0, -400),
            (-1.0, -400),
            (1.0, 400),
            (f64::INFINITY, 400),
        ] {
            assert!(np_round_nd(x, n).is_nan(), "np.round({x}, {n})");
        }
        assert_eq!(round_half_even(f64::INFINITY), f64::INFINITY);
        assert!(round_half_even(f64::NAN).is_nan());
        assert_eq!(round_nd(f64::MAX, 2), f64::MAX);
    }

    #[test]
    fn matches_python_vectors() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../reference/py/vectors.json");
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
        let doc: Value = serde_json::from_str(&text).expect("vectors.json is JSON");
        let cases = doc["cases"].as_object().expect("vectors.json has a cases object");
        let mut seen = BTreeSet::new();
        let mut failures = Vec::new();
        let mut checked = 0usize;
        let mut minus_zero = 0usize;
        for (name, case) in cases {
            assert!(
                KNOWN.contains(&name.as_str()),
                "vectors.json has a case this test does not know: {name}"
            );
            seen.insert(name.as_str());
            let rows = case["rows"].as_array().unwrap_or_else(|| panic!("{name}: no rows"));
            let arity = case["inputs"]
                .as_array()
                .unwrap_or_else(|| panic!("{name}: no inputs"))
                .len();
            assert!(!rows.is_empty(), "{name}: empty");
            for (k, row) in rows.iter().enumerate() {
                let row = row.as_array().unwrap_or_else(|| panic!("{name} row {k}: not a list"));
                assert_eq!(row.len(), arity + 1, "{name} row {k}: wrong length");
                // (inputs as text, got, want, equal)
                let one = |g: &dyn Fn(f64) -> f64| {
                    let (x, want) = (f(&row[0]), f(&row[1]));
                    let got = g(x);
                    (show(x), show(got), show(want), got.to_bits() == want.to_bits(), want)
                };
                let two = |g: &dyn Fn(f64, f64) -> f64| {
                    let (a, b, want) = (f(&row[0]), f(&row[1]), f(&row[2]));
                    let got = g(a, b);
                    (
                        format!("{}, {}", show(a), show(b)),
                        show(got),
                        show(want),
                        got.to_bits() == want.to_bits(),
                        want,
                    )
                };
                let int2 = |g: &dyn Fn(i64, i64) -> i64| {
                    let (a, b, want) = (i(&row[0]), i(&row[1]), i(&row[2]));
                    let got = g(a, b);
                    (format!("{a}, {b}"), got.to_string(), want.to_string(), got == want, 1.0)
                };
                let (inputs, got, want, ok, want_f) = match name.as_str() {
                    "round" => {
                        let (x, want) = (f(&row[0]), i(&row[1]));
                        let got = round_half_even(x);
                        // Python's round(x) is an int: compare the value (an int has no minus zero).
                        (
                            show(x),
                            show(got),
                            want.to_string(),
                            got == want as f64 && want.abs() <= 1 << 53,
                            1.0,
                        )
                    }
                    "round_1" => one(&|x| round_nd(x, 1)),
                    "round_2" => one(&|x| round_nd(x, 2)),
                    "round_3" => one(&|x| round_nd(x, 3)),
                    "round_5" => one(&|x| round_nd(x, 5)),
                    "np_round_2" => one(&|x| np_round_nd(x, 2)),
                    "rint" => one(&round_half_even),
                    "wrap_pi" => one(&wrap_pi),
                    "mod" => two(&fmod_py),
                    "floordiv" => two(&floor_div),
                    "imod" => int2(&imod_py),
                    "ifloordiv" => int2(&ifloor_div),
                    other => panic!("vectors.json case {other} has no check in this test"),
                };
                checked += 1;
                if want_f == 0.0 && want_f.is_sign_negative() {
                    minus_zero += 1;
                }
                if !ok {
                    failures.push(format!("{name}({inputs}): got {got}, Python gave {want}"));
                }
            }
        }
        let missing: Vec<&str> = KNOWN.iter().copied().filter(|k| !seen.contains(k)).collect();
        assert!(missing.is_empty(), "vectors.json lacks the cases {missing:?}");
        assert!(
            minus_zero > 0,
            "no minus-zero outputs seen: the hex parser or the file is wrong"
        );
        assert!(
            failures.is_empty(),
            "{} of {checked} rows differ from Python (first 30):\n{}",
            failures.len(),
            failures.iter().take(30).cloned().collect::<Vec<_>>().join("\n")
        );
    }
}
