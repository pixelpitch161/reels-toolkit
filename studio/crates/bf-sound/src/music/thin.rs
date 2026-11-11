//! The 16th grid and the thinning of a hurt fighter's line: `q16`, `h01`, `Thinner`, `h01_37`, `thin_37`.

use bf_core::dmath;
use bf_core::py::round_half_even;

use super::num::py_min;

/// `q16(t)` on a grid whose sixteenth is `s16` seconds: the nearest 16th, `round(t / S16) * S16` (Python's
/// `round`: halves go to the even 16th). A `t` that is not a number comes back as NaN (Python raises).
pub fn q16(t: f64, s16: f64) -> f64 {
    // Python's round gives an int, which has no minus zero: `+ 0.0` turns -0.0 into 0.0.
    (round_half_even(t / s16) + 0.0) * s16
}

// ---------------------------------------------------------------- thinning (a hurt fighter's line loses notes)

/// `int(round(x * 1000)) & 0xFFFFFFFFFFFFFFFF`: what the two hashes read of each argument. Whole numbers go in as
/// floats, which is exact up to 2**53 / 1000.
///
/// # Panics
/// Where Python raises: `x` is NaN or infinite.
fn milli(x: f64) -> u64 {
    let v = round_half_even(x * 1000.0);
    assert!(v.is_finite(), "cannot convert float {v} to integer");
    if v.abs() >= 1.6e35 {
        // A float this large (anything from 2**116, about 8.3e34, up) is a multiple of 2**64: its low 64 bits
        // are zero.
        0
    } else {
        // Exact (a rounded float this small is a whole number an i128 holds); the cast to u64 keeps the low 64
        // bits in two's complement, which is what Python's `&` does to a negative int.
        (v as i128) as u64
    }
}

/// `h01(*xs)`: a deterministic splitmix hash of the arguments in [0, 1) (#37's fix: the old linear hash fell
/// below the threshold on runs of consecutive slots and silenced whole phrases). Each argument goes in as
/// `int(round(x * 1000))`.
///
/// # Panics
/// Where Python raises: an argument is NaN or infinite.
pub fn h01(xs: &[f64]) -> f64 {
    let mut v: u64 = 0x9E37_79B9_7F4A_7C15;
    for &x in xs {
        v ^= milli(x);
        v = v.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = v;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        v = z ^ (z >> 31);
    }
    (v >> 11) as f64 / 9_007_199_254_740_992.0
}

/// `Thinner(seed)`: drops notes as a fighter's HP falls, with the chance `min(0.30, 0.55 (1 - hp)^1.6)` and
/// never two in a row (#41).
#[derive(Clone, Debug, PartialEq)]
pub struct Thinner {
    seed: f64,
    last: bool,
}

impl Thinner {
    /// A thinner with its own hash `seed` (the FLUTE's is 5, the TROMBONE's 7).
    pub fn new(seed: f64) -> Thinner {
        Thinner { seed, last: false }
    }

    /// `keep(slot, hp01)`: whether the note in `slot` plays, for a fighter at `hp01` of its HP (0 to 1). Call it
    /// for every slot in order: it remembers whether it dropped the one before.
    ///
    /// # Panics
    /// Where Python raises: `hp01` is above 1 (the power of a negative number is complex in Python), or `slot`
    /// is NaN or infinite when it is hashed.
    pub fn keep(&mut self, slot: f64, hp01: f64) -> bool {
        assert!(
            hp01 <= 1.0 || hp01.is_nan(),
            "Thinner.keep: hp01 is {hp01}, above 1 (Python cannot compare the complex power)"
        );
        let p = py_min(0.30, 0.55 * dmath::pow(1.0 - hp01, 1.6));
        let drop = !self.last && h01(&[slot, self.seed]) < p;
        self.last = drop;
        !drop
    }
}

/// `h01_37(*xs)`: #37's splitmix64 mixer in [0, 1] (the reed fighters' thinning, as #37 built it).
///
/// # Panics
/// Where Python raises: an argument is NaN or infinite.
pub fn h01_37(xs: &[f64]) -> f64 {
    let mut z: u64 = 0x9E37_79B9_7F4A_7C15;
    for &x in xs {
        z = (z ^ milli(x)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
    }
    // Python divides the int by float(1 << 64): the int is rounded to the nearest float first, as `as f64` does.
    z as f64 / 18_446_744_073_709_551_616.0
}

/// `thin_37(slot, key, hp, prev_dropped)`: drop this note? (#37: up to 30 % of a hurt fighter's notes, never two
/// in a row.) `hp` is the fighter's HP from 0 to 1.
///
/// # Panics
/// Where Python raises: `hp` is above 1 and the note before was not dropped, or `slot` or `key` is NaN or
/// infinite when it is hashed.
pub fn thin_37(slot: f64, key: f64, hp: f64, prev_dropped: bool) -> bool {
    if prev_dropped {
        return false;
    }
    assert!(
        hp <= 1.0 || hp.is_nan(),
        "thin_37: hp is {hp}, above 1 (Python cannot compare the complex power)"
    );
    h01_37(&[slot, key]) < 0.30 * dmath::pow(1.0 - hp, 1.6)
}
