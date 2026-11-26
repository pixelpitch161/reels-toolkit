//! A few gentle tone controls (the audio cookbook's shelves and peaks), for an instrument's body and its studio
//! polish: each band lifts or cuts around its frequency, the rest left as it is.

use bf_core::dmath;

use crate::SRF;
use crate::dsp;

/// One band: what it does, at `hz`, by `db` (up when positive), `q` its width (0.7 a broad shelf, 1 a peak an
/// octave and a half wide).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Band {
    /// Everything under `hz` moved by `db`.
    Low { hz: f64, db: f64, q: f64 },
    /// A bump (or a dip) around `hz`.
    Peak { hz: f64, db: f64, q: f64 },
    /// Everything over `hz` moved by `db`.
    High { hz: f64, db: f64, q: f64 },
}

impl Band {
    /// Its filter's `(b, a)`, at 48 kHz.
    pub fn coeffs(&self) -> ([f64; 3], [f64; 3]) {
        let (hz, db, q) = match *self {
            Band::Low { hz, db, q } | Band::Peak { hz, db, q } | Band::High { hz, db, q } => (hz, db, q),
        };
        let a = dmath::pow(10.0, db / 40.0);
        let w = dmath::TAU * hz / SRF;
        let (c, s) = (dmath::cos(w), dmath::sin(w));
        let al = s / (2.0 * q.max(1e-3));
        let sq = 2.0 * a.sqrt() * al;
        let (b, a) = match self {
            Band::Peak { .. } => (
                [1.0 + al * a, -2.0 * c, 1.0 - al * a],
                [1.0 + al / a, -2.0 * c, 1.0 - al / a],
            ),
            Band::Low { .. } => (
                [
                    a * ((a + 1.0) - (a - 1.0) * c + sq),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * c),
                    a * ((a + 1.0) - (a - 1.0) * c - sq),
                ],
                [
                    (a + 1.0) + (a - 1.0) * c + sq,
                    -2.0 * ((a - 1.0) + (a + 1.0) * c),
                    (a + 1.0) + (a - 1.0) * c - sq,
                ],
            ),
            Band::High { .. } => (
                [
                    a * ((a + 1.0) + (a - 1.0) * c + sq),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * c),
                    a * ((a + 1.0) + (a - 1.0) * c - sq),
                ],
                [
                    (a + 1.0) - (a - 1.0) * c + sq,
                    2.0 * ((a - 1.0) - (a + 1.0) * c),
                    (a + 1.0) - (a - 1.0) * c - sq,
                ],
            ),
        };
        let a0 = a[0];
        ([b[0] / a0, b[1] / a0, b[2] / a0], [1.0, a[1] / a0, a[2] / a0])
    }
}

/// `x` through `bands`, one after another.
pub fn apply(x: &[f64], bands: &[Band]) -> Vec<f64> {
    let mut y = x.to_vec();
    for band in bands {
        if y.is_empty() {
            break;
        }
        let (b, a) = band.coeffs();
        y = dsp::lfilter(&b, &a, &y);
    }
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The level of a steady sine at `hz` after `bands`, dB against the sine.
    fn gain_at(bands: &[Band], hz: f64) -> f64 {
        let n = 48_000;
        let x: Vec<f64> = (0..n).map(|i| dmath::sin(dmath::TAU * hz * i as f64 / SRF)).collect();
        let y = apply(&x, bands);
        let rms = |v: &[f64]| (v[n / 2..].iter().map(|s| s * s).sum::<f64>() / (n / 2) as f64).sqrt();
        20.0 * dmath::log10(rms(&y) / rms(&x))
    }

    #[test]
    fn each_band_moves_its_own_part_and_leaves_the_rest() {
        let low = [Band::Low {
            hz: 200.0,
            db: -2.5,
            q: 0.7,
        }];
        assert!((gain_at(&low, 40.0) + 2.5).abs() < 0.3);
        assert!(gain_at(&low, 4000.0).abs() < 0.1);
        let peak = [Band::Peak {
            hz: 1500.0,
            db: -3.0,
            q: 1.0,
        }];
        assert!((gain_at(&peak, 1500.0) + 3.0).abs() < 0.05);
        assert!(gain_at(&peak, 100.0).abs() < 0.1 && gain_at(&peak, 15_000.0).abs() < 0.2);
        let high = [Band::High {
            hz: 3000.0,
            db: 4.0,
            q: 0.7,
        }];
        assert!((gain_at(&high, 16_000.0) - 4.0).abs() < 0.3);
        assert!(gain_at(&high, 100.0).abs() < 0.1);
        assert!(apply(&[], &high).is_empty());
    }
}
