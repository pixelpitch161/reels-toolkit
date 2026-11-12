//! The deterministic hash in [0, 1) four kit parts keep a copy of, under their episode's number: ACOUSTIC's
//! `h01_33` (its up-strokes drop out when hurt), HORN's `_h01_34`, ORGAN's `h01_35`, TRUMPET's `h01_29` (its ghost
//! notes thin out). The code is the same in all four:
//!
//! ```python
//! v = 0
//! for x in xs:
//!     v = (v * 1000003 + int(x * 1000) + 7) % 2147483647
//! return ((v * 48271) % 2147483647) / 2147483647.0
//! ```
//!
//! It is not `bf_sound::music::h01` (`kit/music.py`'s, a different function).

/// `2147483647`, the modulus.
const M: i64 = 2_147_483_647;

/// `h01_29(*xs)` (= `h01_33`, `_h01_34`, `h01_35`). A whole number argument is the same as its float.
///
/// Python's integers do not overflow: `int(x * 1000)` may be any size. Only its remainder by the modulus matters,
/// and for a product of 2**53 or more (a whole number already) that remainder is taken exactly in floating point.
///
/// # Panics
/// Where Python raises: an argument whose product with 1000 is not a finite number.
pub fn h01(xs: &[f64]) -> f64 {
    let mut v: i64 = 0;
    for &x in xs {
        let p = x * 1000.0;
        assert!(p.is_finite(), "cannot convert float {p} to integer");
        // int(p) cuts toward zero; its remainder by M, as Python's % gives it (never negative)
        let t = p.trunc();
        let r = if t.abs() < 9.0e15 {
            (t as i64).rem_euclid(M)
        } else {
            // a whole float: fmod is exact; then into 0..M
            let r = t % (M as f64);
            (if r < 0.0 { r + M as f64 } else { r }) as i64
        };
        v = (v * 1_000_003 + r + 7).rem_euclid(M);
    }
    ((v * 48_271) % M) as f64 / M as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_of_nothing_and_of_a_few_numbers() {
        assert_eq!(h01(&[]), 0.0);
        // v = 7 for 0.0: (7 * 48271 % M) / M
        assert_eq!(h01(&[0.0]), (7.0 * 48271.0) / 2147483647.0);
        // int() cuts toward zero: -0.0019 * 1000 = -1.9 is -1
        let v = (-1 + 7) as f64;
        assert_eq!(h01(&[-0.0019]), (v * 48271.0) / 2147483647.0);
        assert!((0.0..1.0).contains(&h01(&[12.345, 4.0])));
        // a huge argument is reduced exactly
        assert!((0.0..1.0).contains(&h01(&[1e300])));
    }
}
