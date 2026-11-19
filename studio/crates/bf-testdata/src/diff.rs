//! How far a result is from its reference: sample differences in dB, relative differences, and pixel counts.

use std::fmt;

/// `20 log10(max |a - b|)`, the largest sample difference in dBFS; `f64::NEG_INFINITY` when no pair differs
/// (`-0.0` against `0.0` and NaN against NaN count as equal). A NaN on one side only, or an infinity against
/// anything else, counts as an infinite difference (`+inf`). Panics if the lengths differ.
pub fn db_max_diff(a: &[f64], b: &[f64]) -> f64 {
    let max = max_by(a, b, |d, _| d);
    if max == 0.0 {
        f64::NEG_INFINITY
    } else if max == f64::INFINITY {
        f64::INFINITY
    } else {
        20.0 * bf_core::dmath::log10(max)
    }
}

/// `max |a - b| / max(|b|, f64::MIN_POSITIVE)` over the pairs (`b` is the reference); 0 when no pair differs,
/// `+inf` under the same rules as [`db_max_diff`]. Panics if the lengths differ.
pub fn max_rel_diff(a: &[f64], b: &[f64]) -> f64 {
    max_by(a, b, |d, y| d / y.abs().max(f64::MIN_POSITIVE))
}

/// The largest `score(|x - y|, y)` over the pairs; a pair with the same bits scores 0, a pair whose difference
/// is NaN or infinite scores `+inf`.
fn max_by(a: &[f64], b: &[f64], score: impl Fn(f64, f64) -> f64) -> f64 {
    assert_eq!(a.len(), b.len(), "lengths differ: {} and {}", a.len(), b.len());
    let mut max = 0.0f64;
    for (&x, &y) in a.iter().zip(b) {
        if x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan()) {
            continue;
        }
        let d = (x - y).abs();
        let s = if d.is_finite() { score(d, y) } else { f64::INFINITY };
        if s > max {
            max = s;
        }
    }
    max
}

/// The difference of two 8-bit images: how many pixels differ (a pixel differs if any channel does), the
/// largest channel difference, the box around the differing pixels as `(x0, y0, x1, y1)` with exclusive ends
/// (`None` when none differ), and the number of pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImgDiff {
    pub differing: usize,
    pub max_diff: u8,
    pub bbox: Option<(usize, usize, usize, usize)>,
    pub total: usize,
}

/// Compares two `w x h` images of `c` channels, rows top to bottom. Panics if either buffer is not `w * h * c`
/// bytes.
pub fn diff_u8(a: &[u8], b: &[u8], w: usize, h: usize, c: usize) -> ImgDiff {
    let want = w * h * c;
    assert!(
        a.len() == want && b.len() == want,
        "{w} x {h} x {c} = {want} bytes wanted, got {} and {}",
        a.len(),
        b.len()
    );
    let mut d = ImgDiff {
        differing: 0,
        max_diff: 0,
        bbox: None,
        total: w * h,
    };
    if c == 0 {
        return d;
    }
    for (i, (pa, pb)) in a.chunks_exact(c).zip(b.chunks_exact(c)).enumerate() {
        let most = pa.iter().zip(pb).map(|(&x, &y)| x.abs_diff(y)).max().unwrap_or(0);
        if most == 0 {
            continue;
        }
        let (x, y) = (i % w, i / w);
        d.differing += 1;
        d.max_diff = d.max_diff.max(most);
        d.bbox = Some(match d.bbox {
            None => (x, y, x + 1, y + 1),
            Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1)),
        });
    }
    d
}

impl fmt::Display for ImgDiff {
    /// `12 px differ (0.005 %), max 1 level, box x 40..52 y 300..301`; `0 px differ (0.000 %)` when the images
    /// are equal.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pct = if self.total == 0 {
            0.0
        } else {
            100.0 * self.differing as f64 / self.total as f64
        };
        write!(f, "{} px differ ({pct:.3} %)", self.differing)?;
        if let Some((x0, y0, x1, y1)) = self.bbox {
            let unit = if self.max_diff == 1 { "level" } else { "levels" };
            write!(f, ", max {} {unit}, box x {x0}..{x1} y {y0}..{y1}", self.max_diff)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_of_the_largest_difference() {
        assert_eq!(db_max_diff(&[1.0, 2.0], &[1.0, 2.0]), f64::NEG_INFINITY);
        assert_eq!(db_max_diff(&[], &[]), f64::NEG_INFINITY);
        assert_eq!(db_max_diff(&[0.0, 0.5], &[0.1, 0.4]), db_max_diff(&[0.1], &[0.0]));
        assert_eq!(db_max_diff(&[1.0, 0.0], &[1.0, 0.1]), -20.0);
        assert_eq!(db_max_diff(&[0.0, 1e-6], &[0.0, 0.0]), -120.0);
        assert_eq!(db_max_diff(&[2.0], &[1.0]), 0.0);
        assert_eq!(db_max_diff(&[f64::NAN], &[f64::NAN]), f64::NEG_INFINITY);
        assert_eq!(db_max_diff(&[f64::NAN], &[0.0]), f64::INFINITY);
        assert_eq!(db_max_diff(&[f64::INFINITY], &[f64::INFINITY]), f64::NEG_INFINITY);
        assert_eq!(db_max_diff(&[f64::INFINITY], &[1.0]), f64::INFINITY);
        // -0.0 and 0.0 differ in bits but not in value: no difference.
        assert_eq!(db_max_diff(&[-0.0], &[0.0]), f64::NEG_INFINITY);
    }

    #[test]
    #[should_panic(expected = "lengths differ: 2 and 1")]
    fn db_wants_equal_lengths() {
        db_max_diff(&[0.0, 0.0], &[0.0]);
    }

    #[test]
    fn relative_difference() {
        assert_eq!(max_rel_diff(&[1.0, 2.0], &[1.0, 2.0]), 0.0);
        assert_eq!(max_rel_diff(&[1.5, 9.0], &[1.0, 10.0]), 0.5);
        assert_eq!(max_rel_diff(&[-3.0], &[-2.0]), 0.5);
        assert_eq!(max_rel_diff(&[f64::MIN_POSITIVE], &[0.0]), 1.0);
        assert_eq!(max_rel_diff(&[f64::NAN], &[1.0]), f64::INFINITY);
    }

    #[test]
    fn image_difference() {
        // 4 x 3, RGB.
        let a = vec![10u8; 4 * 3 * 3];
        let same = diff_u8(&a, &a, 4, 3, 3);
        assert_eq!(
            same,
            ImgDiff {
                differing: 0,
                max_diff: 0,
                bbox: None,
                total: 12
            }
        );
        assert_eq!(same.to_string(), "0 px differ (0.000 %)");
        let mut b = a.clone();
        b[(4 + 1) * 3 + 2] = 11; // (x 1, y 1), blue +1
        b[(2 * 4 + 2) * 3] = 7; // (x 2, y 2), red -3
        b[(2 * 4 + 2) * 3 + 1] = 12; // same pixel, green +2
        let d = diff_u8(&a, &b, 4, 3, 3);
        assert_eq!(
            d,
            ImgDiff {
                differing: 2,
                max_diff: 3,
                bbox: Some((1, 1, 3, 3)),
                total: 12
            }
        );
        assert_eq!(d.to_string(), "2 px differ (16.667 %), max 3 levels, box x 1..3 y 1..3");
        let one = diff_u8(&[0, 0], &[0, 1], 2, 1, 1);
        assert_eq!(
            one.to_string(),
            "1 px differ (50.000 %), max 1 level, box x 1..2 y 0..1"
        );
        let full = diff_u8(&[0], &[255], 1, 1, 1);
        assert_eq!(full.max_diff, 255);
    }

    #[test]
    #[should_panic(expected = "4 x 3 x 3 = 36 bytes")]
    fn image_sizes_must_match() {
        diff_u8(&[0; 36], &[0; 35], 4, 3, 3);
    }
}
