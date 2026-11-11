//! The numpy calls of the sound code whose result is more than one rounding: sums and means (numpy adds a 1-D
//! array pairwise), running sums, `linspace`, `interp` and `percentile`.
//!
//! Ported from numpy 2.5.3: `DOUBLE_pairwise_sum` (`_core/src/umath/loops_utils.h.src`), `linspace`
//! (`_core/function_base.py`), `arr_interp` with `binary_search_with_guess`
//! (`_core/src/multiarray/compiled_base.c`), and `percentile` in its default `'linear'` method
//! (`lib/_function_base_impl.py`: `_quantile`, `_get_indexes`, `_get_gamma`, `_lerp`). All of it is plain IEEE
//! arithmetic in numpy's order, so the results have numpy's bits.

/// The block numpy sums with eight running totals before it starts halving (`PW_BLOCKSIZE`).
const PW_BLOCKSIZE: usize = 128;

/// `x.sum()` of a 1-D `float64` array (and of any contiguous array summed over all its axes: numpy reads it flat):
/// numpy's pairwise sum, added to the reduction's starting value 0 (so a sum of `-0.0` values is `0.0`, as in
/// numpy).
pub fn sum(a: &[f64]) -> f64 {
    0.0 + pairwise_sum(a)
}

/// `DOUBLE_pairwise_sum`. Fewer than 8 values are added in order; up to 128 go into eight running totals that are
/// then added as `((r0 + r1) + (r2 + r3)) + ((r4 + r5) + (r6 + r7))`; more are split in two (the first half
/// rounded down to a multiple of 8) and each half summed this way.
fn pairwise_sum(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 8 {
        // From -0, as numpy's loop starts (`sum` then adds the reduction's own +0).
        let mut res = -0.0;
        for &v in a {
            res += v;
        }
        res
    } else if n <= PW_BLOCKSIZE {
        let mut r = [a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]];
        let whole = n - n % 8;
        for block in a[8..whole].as_chunks::<8>().0 {
            for (total, v) in r.iter_mut().zip(block) {
                *total += v;
            }
        }
        let mut res = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
        for &v in &a[whole..] {
            res += v;
        }
        res
    } else {
        let mut n2 = n / 2;
        n2 -= n2 % 8;
        pairwise_sum(&a[..n2]) + pairwise_sum(&a[n2..])
    }
}

/// `x.mean()` of a 1-D array (or of a whole contiguous array, read flat): the pairwise [`sum`] divided by the
/// number of values. NaN for an empty array.
pub fn mean(a: &[f64]) -> f64 {
    sum(a) / a.len() as f64
}

/// `x.sum(0)` of a stereo array: the sum of each column. numpy runs down the rows here, so each column is added
/// in order, one row after the other (not pairwise).
pub fn sum_axis0(x: &[[f64; 2]]) -> [f64; 2] {
    let mut s = [0.0; 2];
    for row in x {
        s[0] += row[0];
        s[1] += row[1];
    }
    s
}

/// `np.cumsum(x)`: the running sum, one addition per value.
pub fn cumsum(a: &[f64]) -> Vec<f64> {
    let mut out = Vec::with_capacity(a.len());
    let mut total = 0.0;
    for (i, &v) in a.iter().enumerate() {
        total = if i == 0 { v } else { total + v };
        out.push(total);
    }
    out
}

/// `np.linspace(start, stop, num)`: `num` values from `start` to `stop`, both ends included. Each is
/// `i * ((stop - start) / (num - 1)) + start`, and the last is `stop` itself.
pub fn linspace(start: f64, stop: f64, num: usize) -> Vec<f64> {
    let delta = stop - start;
    let mut y: Vec<f64> = (0..num).map(|i| i as f64).collect();
    if num > 1 {
        let div = (num - 1) as f64;
        let step = delta / div;
        for v in &mut y {
            if step == 0.0 {
                // numpy's care for a step too small to hold: divide first.
                *v /= div;
                *v *= delta;
            } else {
                *v *= step;
            }
        }
    } else {
        for v in &mut y {
            *v *= delta;
        }
    }
    for v in &mut y {
        *v += start;
    }
    if num > 1 {
        y[num - 1] = stop;
    }
    y
}

/// How far around the last index `interp` looks before it bisects (`LIKELY_IN_CACHE_SIZE`).
const LIKELY_IN_CACHE_SIZE: isize = 8;

/// numpy's `binary_search_with_guess`: the index `i` with `arr[i] <= key < arr[i + 1]`, trying `guess` and its
/// neighbours first; -1 for a key below `arr[0]`, `len` for one above the last, `len - 1` for the last itself.
fn binary_search_with_guess(key: f64, arr: &[f64], guess: isize) -> isize {
    let len = arr.len() as isize;
    let at = |i: isize| arr[i as usize];
    let (mut imin, mut imax) = (0, len);
    if key > at(len - 1) {
        return len;
    } else if key < at(0) {
        return -1;
    }
    // Up to four points: a linear search (the key is at or above arr[0] here).
    if len <= 4 {
        let mut i = 1;
        while i < len && key >= at(i) {
            i += 1;
        }
        return i - 1;
    }
    let guess = guess.min(len - 3).max(1);
    // The most likely places: guess - 1, guess, guess + 1.
    if key < at(guess) {
        if key < at(guess - 1) {
            imax = guess - 1;
            if guess > LIKELY_IN_CACHE_SIZE && key >= at(guess - LIKELY_IN_CACHE_SIZE) {
                imin = guess - LIKELY_IN_CACHE_SIZE;
            }
        } else {
            return guess - 1;
        }
    } else if key < at(guess + 1) {
        return guess;
    } else if key < at(guess + 2) {
        return guess + 1;
    } else {
        imin = guess + 2;
        if guess < len - LIKELY_IN_CACHE_SIZE - 1 && key < at(guess + LIKELY_IN_CACHE_SIZE) {
            imax = guess + LIKELY_IN_CACHE_SIZE;
        }
    }
    while imin < imax {
        let imid = imin + ((imax - imin) >> 1);
        if key >= at(imid) {
            imin = imid + 1;
        } else {
            imax = imid;
        }
    }
    imin - 1
}

/// `np.interp(x, xp, fp)`: the line through the points `(xp, fp)` read at each `x`; `xp` must ascend. Below
/// `xp[0]` the result is `fp[0]`, above the last point it is the last `fp`.
///
/// # Panics
/// As [`interp_lr`].
pub fn interp(x: &[f64], xp: &[f64], fp: &[f64]) -> Vec<f64> {
    interp_lr(x, xp, fp, None, None)
}

/// `np.interp(x, xp, fp, left=left, right=right)`: as [`interp`], with `left` given for `x` below `xp[0]` and
/// `right` for `x` above the last point (`None` keeps the end value of `fp`). Between two points the value is
/// `slope * (x - xp[j]) + fp[j]`; on a point it is that point's `fp`.
///
/// # Panics
/// Where numpy raises: `xp` is empty while `x` is not, or `fp` and `xp` differ in length.
pub fn interp_lr(x: &[f64], xp: &[f64], fp: &[f64], left: Option<f64>, right: Option<f64>) -> Vec<f64> {
    assert!(!(xp.is_empty() && !x.is_empty()), "array of sample points is empty");
    assert!(fp.len() == xp.len(), "fp and xp are not of the same length.");
    if x.is_empty() {
        return Vec::new();
    }
    let lenxp = xp.len();
    let lval = left.unwrap_or(fp[0]);
    let rval = right.unwrap_or(fp[lenxp - 1]);
    if lenxp == 1 {
        return x
            .iter()
            .map(|&v| {
                if v < xp[0] {
                    lval
                } else if v > xp[0] {
                    rval
                } else {
                    fp[0]
                }
            })
            .collect();
    }
    let mut j: isize = 0;
    x.iter()
        .map(|&x_val| {
            if x_val.is_nan() {
                return x_val;
            }
            j = binary_search_with_guess(x_val, xp, j);
            if j == -1 {
                return lval;
            }
            let i = j as usize;
            if i == lenxp {
                rval
            } else if i == lenxp - 1 || xp[i] == x_val {
                fp[i]
            } else {
                let slope = (fp[i + 1] - fp[i]) / (xp[i + 1] - xp[i]);
                let mut res = slope * (x_val - xp[i]) + fp[i];
                // A NaN from one side (an infinite fp): try the other.
                if res.is_nan() {
                    res = slope * (x_val - xp[i + 1]) + fp[i + 1];
                    if res.is_nan() && fp[i] == fp[i + 1] {
                        res = fp[i];
                    }
                }
                res
            }
        })
        .collect()
}

/// `np.percentile(x, q)` (the default `'linear'` method) of all the values of `x`: the value at the position
/// `(n - 1) * (q / 100)` of the sorted values, read between its two neighbours `a <= b` as `a + (b - a) * t`,
/// or as `b - (b - a) * (1 - t)` once `t` reaches one half. NaN if any value is NaN.
///
/// # Panics
/// Where numpy raises: `q` is not in `[0, 100]`; `x` is empty.
pub fn percentile(x: &[f64], q: f64) -> f64 {
    let q = q / 100.0;
    assert!((0.0..=1.0).contains(&q), "Percentiles must be in the range [0, 100]");
    let n = x.len();
    assert!(n > 0, "percentile: x is empty");
    if x.iter().any(|v| v.is_nan()) {
        return f64::NAN;
    }
    let virtual_index = (n - 1) as f64 * q;
    // The two neighbours; at the top both are the largest value.
    let (previous, next) = if virtual_index >= (n - 1) as f64 {
        (-1.0, -1.0)
    } else {
        (virtual_index.floor(), virtual_index.floor() + 1.0)
    };
    let gamma = virtual_index - previous;
    let index = |i: f64| if i < 0.0 { n - 1 } else { i as usize };
    let (lo, hi) = (index(previous), index(next));
    // Only the two order statistics are needed, not the whole sort.
    let mut v = x.to_vec();
    let (_, &mut a, above) = v.select_nth_unstable_by(lo, |p, q| p.partial_cmp(q).expect("no NaN is left"));
    let b = if hi == lo {
        a
    } else {
        above.iter().copied().fold(f64::INFINITY, f64::min)
    };
    lerp(a, b, gamma)
}

/// numpy's `_lerp`.
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    let diff_b_a = b - a;
    if t >= 0.5 {
        b - diff_b_a * (1.0 - t)
    } else {
        a + diff_b_a * t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairwise_sum_follows_numpys_blocks() {
        // Fewer than 8: in order. An empty sum and a sum of minus zeros are plus zero, as numpy's.
        assert!(sum(&[]) == 0.0 && sum(&[]).is_sign_positive());
        assert!(sum(&[-0.0, -0.0]).is_sign_positive() && sum(&[-0.0; 200]).is_sign_positive());
        assert!(pairwise_sum(&[-0.0, -0.0]).is_sign_negative());
        assert_eq!(sum(&[1e16, 1.0, -1e16, 1.0]), 1.0);
        // Eight totals: with 16 values, each total takes values i and i + 8, then the totals pair up.
        let mut a = [0.0; 16];
        (a[0], a[8], a[1], a[9]) = (1e16, 1.0, -1e16, 1.0);
        // In order this would be 1e16 + 1 (lost) - 1e16 + ... = 1; in numpy's order r0 = 1e16 + 1 = 1e16 (lost),
        // r1 = -1e16 + 1 = -1e16 (lost), r0 + r1 = 0.
        assert_eq!(sum(&a), 0.0);
        // More than 128: split at a multiple of 8 and added half by half.
        let big: Vec<f64> = (0..1000).map(|i| 0.1 * f64::from(i)).collect();
        let halves = pairwise_sum(&big[..496]) + pairwise_sum(&big[496..]);
        assert_eq!(sum(&big), halves);
        assert!((sum(&big) - 49950.0).abs() < 1e-9);
        assert_eq!(mean(&[1.0, 2.0, 6.0]), 3.0);
        assert!(mean(&[]).is_nan());
    }

    #[test]
    fn sums_along_the_rows_and_running_sums() {
        assert_eq!(sum_axis0(&[[1.0, 10.0], [2.0, 20.0], [3.0, 30.0]]), [6.0, 60.0]);
        assert_eq!(sum_axis0(&[]), [0.0, 0.0]);
        assert_eq!(cumsum(&[1.0, 2.0, 3.5]), [1.0, 3.0, 6.5]);
        assert!(cumsum(&[]).is_empty());
        // The first value is kept as it is, minus zero included.
        assert!(cumsum(&[-0.0])[0].is_sign_negative());
    }

    #[test]
    fn linspace_ends_and_short_cases() {
        assert_eq!(linspace(0.0, 1.0, 5), [0.0, 0.25, 0.5, 0.75, 1.0]);
        assert_eq!(linspace(1.0, 0.0, 3), [1.0, 0.5, 0.0]);
        assert_eq!(linspace(1.0, 0.0, 1), [1.0]);
        assert_eq!(linspace(2.0, 2.0, 3), [2.0, 2.0, 2.0]);
        assert!(linspace(0.0, 1.0, 0).is_empty());
        // The last value is `stop` itself, not a rounded sum.
        assert_eq!(*linspace(0.0, 0.3, 4).last().unwrap(), 0.3);
    }

    #[test]
    fn interp_ends_points_and_between() {
        let (xp, fp) = ([1.0, 2.0, 4.0], [10.0, 20.0, 0.0]);
        let x = [0.0, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0, f64::NAN];
        let y = interp(&x, &xp, &fp);
        assert_eq!(y[..7], [10.0, 10.0, 15.0, 20.0, 10.0, 0.0, 0.0]);
        assert!(y[7].is_nan());
        let y = interp_lr(&x[..7], &xp, &fp, Some(-1.0), Some(-2.0));
        assert_eq!(y, [-1.0, 10.0, 15.0, 20.0, 10.0, 0.0, -2.0]);
        // One point: left of it, on it, right of it.
        assert_eq!(
            interp_lr(&[0.0, 1.0, 2.0], &[1.0], &[5.0], Some(4.0), Some(6.0)),
            [4.0, 5.0, 6.0]
        );
        assert!(interp(&[], &[], &[]).is_empty());
        // A long table, read forwards, backwards and at random: the guess never changes the answer.
        let xp: Vec<f64> = (0..100).map(f64::from).collect();
        let fp: Vec<f64> = xp.iter().map(|v| v * v).collect();
        for q in [0.5, 98.5, 3.25, 50.0, 49.75, 99.0, 0.0, 12.5, 97.0, 96.5] {
            let i = f64::floor(q);
            let want = if q == i {
                i * i
            } else {
                ((i + 1.0) * (i + 1.0) - i * i) * (q - i) + i * i
            };
            assert_eq!(interp(&[70.0, q], &xp, &fp)[1], want, "{q}");
        }
    }

    #[test]
    #[should_panic(expected = "fp and xp are not of the same length.")]
    fn interp_wants_matching_tables() {
        interp(&[1.0], &[1.0, 2.0], &[1.0]);
    }

    #[test]
    fn percentile_reads_between_neighbours() {
        let x = [4.0, 1.0, 3.0, 2.0, 5.0];
        assert_eq!(percentile(&x, 0.0), 1.0);
        assert_eq!(percentile(&x, 100.0), 5.0);
        assert_eq!(percentile(&x, 50.0), 3.0);
        assert_eq!(percentile(&x, 25.0), 2.0);
        assert_eq!(percentile(&x, 10.0), 1.4);
        // 90 %: position 3.6 between 4 and 5, read from the upper neighbour: 5 - 1 * (1 - 0.6).
        assert_eq!(percentile(&x, 90.0), 5.0 - (1.0 - (4.0 * 0.9 - 3.0)));
        assert_eq!(percentile(&[7.0], 99.97), 7.0);
        assert!(percentile(&[1.0, f64::NAN], 50.0).is_nan());
    }

    #[test]
    #[should_panic(expected = "percentile: x is empty")]
    fn percentile_wants_values() {
        percentile(&[], 50.0);
    }

    #[test]
    #[should_panic(expected = "Percentiles must be in the range [0, 100]")]
    fn percentile_wants_a_percentage() {
        percentile(&[1.0], 101.0);
    }
}
