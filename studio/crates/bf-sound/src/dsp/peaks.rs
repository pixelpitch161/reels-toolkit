//! `scipy.signal.find_peaks` as the levels check calls it: local maxima at least `height` high and at least
//! `distance` samples apart.
//!
//! Ported from scipy 1.18.1: `find_peaks` (`signal/_peak_finding.py`) with its `height` and `distance` conditions,
//! and `_local_maxima_1d` and `_select_by_peak_distance` (`signal/_peak_finding_utils.pyx`). Whole numbers and
//! comparisons only: where scipy's answer is fixed by its code, this is that answer. The one place where it is not
//! (two equal peaks too close to each other) is described at [`find_peaks`].

/// scipy's `_local_maxima_1d`: the index of every local maximum of `x`, in ascending order. A maximum is one
/// sample, or a run of equal samples (a flat top), with a smaller sample on each side; a flat top counts once,
/// at its middle (rounded down). The first and the last sample are never maxima.
pub fn local_maxima(x: &[f64]) -> Vec<usize> {
    let mut midpoints = Vec::new();
    if x.len() < 3 {
        return midpoints;
    }
    let i_max = x.len() - 1;
    let mut i = 1;
    while i < i_max {
        if x[i - 1] < x[i] {
            // The next sample that differs from x[i].
            let mut i_ahead = i + 1;
            while i_ahead < i_max && x[i_ahead] == x[i] {
                i_ahead += 1;
            }
            if x[i_ahead] < x[i] {
                let (left, right) = (i, i_ahead - 1);
                midpoints.push((left + right) / 2);
                i = i_ahead;
            }
        }
        i += 1;
    }
    midpoints
}

/// scipy's `_select_by_peak_distance`, with the order of the peaks' priorities given: which of the `peaks`
/// (sample indices, ascending) stay when no two may be closer than `distance` samples. `priority_order` lists
/// positions in `peaks` from the lowest priority to the highest (scipy's `np.argsort(priority)`); the peaks are
/// visited from the highest down, and each one still standing removes every peak closer than `distance` to it.
///
/// # Panics
/// If `priority_order` is not as long as `peaks`.
pub fn select_by_peak_distance(peaks: &[usize], priority_order: &[usize], distance: usize) -> Vec<bool> {
    assert!(priority_order.len() == peaks.len(), "one priority for each peak");
    let mut keep = vec![true; peaks.len()];
    for &j in priority_order.iter().rev() {
        if !keep[j] {
            continue;
        }
        let mut k = j;
        while k > 0 && peaks[j] - peaks[k - 1] < distance {
            keep[k - 1] = false;
            k -= 1;
        }
        let mut k = j + 1;
        while k < peaks.len() && peaks[k] - peaks[j] < distance {
            keep[k] = false;
            k += 1;
        }
    }
    keep
}

/// `scipy.signal.find_peaks(x, height=height, distance=distance)[0]`: the indices of the local maxima of `x`
/// (see [`local_maxima`]) that are at least `height` high and at least `distance` samples apart. Where two are
/// closer, the higher one stays. (The heights scipy also returns are `x` at these indices.)
///
/// Two equally high peaks closer than `distance`: scipy visits the peaks in the order of `np.argsort(heights)`,
/// numpy's default sort, which does not keep equal values in order and whose order of equals depends on the
/// sorting code numpy picks for the CPU. So which of the two scipy keeps is not fixed by scipy's code. Here the
/// later one is visited first (a stable sort), on every PC. When no two equal peaks are closer than `distance`
/// the result does not depend on that order and is scipy's.
///
/// # Panics
/// Where scipy raises: `distance` is 0.
pub fn find_peaks(x: &[f64], height: f64, distance: usize) -> Vec<usize> {
    assert!(distance >= 1, "`distance` must be greater or equal to 1");
    let peaks: Vec<usize> = local_maxima(x).into_iter().filter(|&i| height <= x[i]).collect();
    let mut order: Vec<usize> = (0..peaks.len()).collect();
    order.sort_by(|&a, &b| x[peaks[a]].partial_cmp(&x[peaks[b]]).expect("a peak is a number"));
    let keep = select_by_peak_distance(&peaks, &order, distance);
    peaks
        .into_iter()
        .zip(keep)
        .filter(|&(_, k)| k)
        .map(|(p, _)| p)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peaks_by_height_and_distance() {
        // What scipy 1.18.1 gives for this signal (every peak has its own height, so the answer is scipy's own).
        let x = [
            0.0, 1.0, 0.0, 2.0, 2.0, 2.0, 0.0, 5.0, 4.0, 6.0, 6.0, 1.0, 3.0, 3.5, 3.0, 9.0, 9.0, 9.0, 9.0, 10.0, 0.0,
            7.0, 0.0,
        ];
        // A flat top counts once, at its middle (rounded down); a shelf on the way up (the 9s) is no maximum.
        assert_eq!(local_maxima(&x), [1, 4, 7, 9, 13, 19, 21]);
        assert_eq!(find_peaks(&x, 2.0, 1), [4, 7, 9, 13, 19, 21]);
        assert_eq!(find_peaks(&x, 2.0, 2), [4, 7, 9, 13, 19, 21]);
        assert_eq!(find_peaks(&x, 2.0, 3), [4, 9, 13, 19]);
        assert_eq!(find_peaks(&x, 0.0, 4), [4, 9, 13, 19]);
        assert_eq!(find_peaks(&x, 3.5, 6), [9, 19]);
        assert!(find_peaks(&x, 100.0, 2).is_empty());
        // The ends are never peaks, a flat end is not one either, and a NaN is neither a peak nor a slope.
        assert!(find_peaks(&[], 0.0, 1).is_empty());
        assert!(find_peaks(&[1.0, 2.0], 0.0, 1).is_empty());
        assert_eq!(find_peaks(&[1.0, 3.0, 1.0], 0.0, 1), [1]);
        assert!(find_peaks(&[1.0, 3.0, 3.0], 0.0, 1).is_empty());
        assert_eq!(find_peaks(&[1.0, f64::NAN, 1.0, 2.0, 1.0], 0.0, 1), [3]);
    }

    #[test]
    fn of_two_equal_peaks_too_close_the_later_stays() {
        // Peaks of height 5 at 1 and 3, two samples apart.
        let x = [0.0, 5.0, 0.0, 5.0, 0.0, 1.0, 0.0];
        assert_eq!(find_peaks(&x, 0.0, 2), [1, 3, 5]);
        assert_eq!(find_peaks(&x, 0.0, 3), [3]);
        // The order of the priorities decides: visiting the first peak first keeps it (and then the small one).
        assert_eq!(select_by_peak_distance(&[1, 3, 5], &[2, 0, 1], 3), [false, true, false]);
        assert_eq!(select_by_peak_distance(&[1, 3, 5], &[2, 1, 0], 3), [true, false, true]);
        assert!(select_by_peak_distance(&[], &[], 3).is_empty());
    }

    #[test]
    #[should_panic(expected = "`distance` must be greater or equal to 1")]
    fn find_peaks_wants_a_distance() {
        find_peaks(&[0.0, 1.0, 0.0], 0.0, 0);
    }
}
