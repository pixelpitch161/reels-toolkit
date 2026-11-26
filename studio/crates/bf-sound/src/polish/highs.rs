//! M3: the sting taken out of a piercing instrument. The band the ear is most sensitive in (about 1.25 to
//! 6.5 kHz) is turned down while the instrument is loud there, more the louder it gets, and by 13 dB at the most;
//! what lies above 7.5 kHz is rolled off. Quiet notes keep their brightness but for a small fixed cut.

use bf_core::dmath;

use crate::SRF;
use crate::dsp::{self, FilterKind, butter_sos};
use crate::heard;

/// The fixed cut of the band, the most it is ever cut by, both in dB, and how hard the cut follows the level.
const CUT: f64 = -4.0;
const MOST: f64 = -13.0;
const SLOPE: f64 = 0.7;

/// `x` (a piercing fighter's bus) softened where it stings.
pub fn treat(x: &[[f64; 2]]) -> Vec<[f64; 2]> {
    if x.is_empty() {
        return Vec::new();
    }
    let band = dsp::sosfilt_stereo(&butter_sos(FilterKind::Bp, &[1250.0, 6500.0], 2, SRF), x);
    let power: Vec<f64> = band.iter().map(|f| (f[0] * f[0] + f[1] * f[1]) / 2.0).collect();
    let env: Vec<f64> = dsp::lfilter(&[0.004], &[1.0, -0.996], &power)
        .iter()
        .map(|&p| (p.max(0.0) + 1e-12).sqrt())
        .collect();
    // The level the cut starts at: the quietest quarter of where the instrument sounds stays as it is.
    let top = env.iter().copied().fold(0.0, f64::max);
    let loud: Vec<f64> = env.iter().copied().filter(|&e| e > top * 0.05).collect();
    let thr = if loud.is_empty() {
        top
    } else {
        dsp::percentile(&loud, 25.0)
    };
    let (cut, most) = (heard::gain(CUT), heard::gain(MOST));
    let y: Vec<[f64; 2]> = x
        .iter()
        .zip(&band)
        .zip(&env)
        .map(|((f, b), &e)| {
            let follow = if e > thr { dmath::pow(thr / e, SLOPE) } else { 1.0 };
            let g = (follow * cut).max(most);
            [f[0] - b[0] + b[0] * g, f[1] - b[1] + b[1] * g]
        })
        .collect();
    dsp::sosfilt_stereo(&butter_sos(FilterKind::Lp, &[7500.0], 1, SRF), &y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn tone(hz: f64, seconds: f64) -> Vec<[f64; 2]> {
        (0..(seconds * SRF) as usize)
            .map(|i| {
                let v = 0.3 * dmath::sin(2.0 * PI * hz * i as f64 / SRF);
                [v, v]
            })
            .collect()
    }

    fn rms_db(x: &[[f64; 2]]) -> f64 {
        let ms = x.iter().map(|f| f[0] * f[0] + f[1] * f[1]).sum::<f64>() / (2 * x.len()) as f64;
        heard::db(ms.sqrt())
    }

    /// A 3 kHz tone comes down by 3 dB or more; a 300 Hz tone stays within half a dB.
    #[test]
    fn the_piercing_band_comes_down_and_the_rest_stays() {
        let high = tone(3000.0, 2.0);
        let cut = rms_db(&treat(&high)[24_000..]) - rms_db(&high[24_000..]);
        assert!((-13.5..=-3.0).contains(&cut), "3 kHz moved by {cut:.2} dB");
        let low = tone(300.0, 2.0);
        let kept = rms_db(&treat(&low)[24_000..]) - rms_db(&low[24_000..]);
        assert!(kept.abs() < 0.5, "300 Hz moved by {kept:.2} dB");
        assert!(treat(&[]).is_empty());
    }

    /// The louder the band, the more it is cut: a burst over a quiet tone loses more than the quiet tone does.
    #[test]
    fn a_loud_passage_is_cut_more_than_a_quiet_one() {
        let mut x = tone(3000.0, 3.0);
        for f in &mut x[..96_000] {
            f[0] *= 0.1;
            f[1] *= 0.1;
        }
        let y = treat(&x);
        let quiet = rms_db(&y[48_000..90_000]) - rms_db(&x[48_000..90_000]);
        let loud = rms_db(&y[120_000..]) - rms_db(&x[120_000..]);
        assert!(loud < quiet - 3.0, "quiet {quiet:.2} dB, loud {loud:.2} dB");
    }
}
