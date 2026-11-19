//! `kit/checks.py` `loudness()`: perceived loudness (ITU-R BS.1770 K-weighting) of the intro against the fight.
//!
//! RMS weighs every frequency alike; the ear hears bright sounds louder. The master goes through the two
//! K-weighting filters (the 48 kHz coefficients), and the loudness of a stretch is
//! `-0.691 + 10 log10(sum over the channels of the mean square + 1e-12)`. The intro is everything before FIGHT!
//! (the title card and the count-in), the fight runs from FIGHT! to the K.O.
//!
//! The kit prints the difference and sets no limit, so this check passes whenever it could measure.

use bf_core::dmath;
use bf_core::show::Show;
use bf_sound::dsp;

use super::{CheckResult, sample_at, unusable};

const NAME: &str = "loudness";
const TITLE: &str = "Perceived loudness";

/// The first K-weighting stage (the high shelf), `b1, a1`.
const SHELF: ([f64; 3], [f64; 3]) = (
    [1.53512485958697, -2.69169618940638, 1.19839281085285],
    [1.0, -1.69065929318241, 0.73248077421585],
);
/// The second stage (the high-pass), `b2, a2`.
const HIGH_PASS: ([f64; 3], [f64; 3]) = ([1.0, -2.0, 1.0], [1.0, -1.99004745483398, 0.99007225036621]);

/// `lufs(z)`: `-0.691 + 10 * np.log10((z ** 2).mean(0).sum() + 1e-12)`. numpy's mean down the rows adds each
/// channel in order. NaN for no samples.
fn lufs(z: &[[f64; 2]]) -> f64 {
    let squares: Vec<[f64; 2]> = z.iter().map(|f| [f[0] * f[0], f[1] * f[1]]).collect();
    let [l, r] = dsp::sum_axis0(&squares);
    let n = z.len() as f64;
    -0.691 + 10.0 * dmath::log10((l / n + r / n) + 1e-12)
}

/// The loudness check on a master as its 16-bit file holds it (see the module notes of [`super`]).
///
/// Numbers: `intro_lufs`, `fight_lufs` and `intro_minus_fight` (LU; negative when the intro is the quieter).
pub fn loudness(master: &[[f64; 2]], show: &Show) -> CheckResult {
    if let Some(r) = unusable(NAME, TITLE, master) {
        return r;
    }
    if master.iter().all(|f| f[0] == 0.0 && f[1] == 0.0) {
        return CheckResult::failed(
            NAME,
            format!(
                "{TITLE}: the soundtrack is silent from start to end, so there is no loudness to measure; check \
                 that the fight file's music plays."
            ),
        );
    }
    // fight = lufs(y[int(T.T_FIGHT * sr):int(M.T_KO * sr)]); intro = lufs(y[:int(T.T_FIGHT * sr)])
    let (i0, i1) = (
        sample_at(show.grid.t_fight, master.len()),
        sample_at(show.t_ko, master.len()),
    );
    if i0 == 0 || i1 <= i0 {
        let (fight, ko) = (show.grid.t_fight, show.t_ko);
        let text = if fight.is_finite() && ko.is_finite() {
            format!(
                "the soundtrack is {:.1} s long and has no intro before {fight:.1} s followed by a fight up to \
                 the knockout at {ko:.1} s, so the two cannot be compared; a video's soundtrack is 61 s",
                master.len() as f64 / super::SRF
            )
        } else {
            "the fight has no time for its start or for its knockout, so the intro and the fight cannot be \
             compared; pick a fight that ends in a knockout"
                .to_string()
        };
        return CheckResult::failed(NAME, format!("{TITLE}: {text}."));
    }
    // y = lfilter(b2, a2, lfilter(b1, a1, x, axis=0), axis=0)
    let y = dsp::lfilter_stereo(&SHELF.0, &SHELF.1, master);
    let y = dsp::lfilter_stereo(&HIGH_PASS.0, &HIGH_PASS.1, &y);
    let fight = lufs(&y[i0..i1]);
    let intro = lufs(&y[..i0]);
    let diff = intro - fight;
    let how = if diff < 0.0 {
        format!("{:.1} loudness units (LU) quieter than", -diff)
    } else {
        format!("{diff:.1} loudness units (LU) louder than")
    };
    CheckResult::new(
        NAME,
        true,
        format!(
            "{TITLE}: to the ear the intro (title card and count-in) is {how} the fight ({intro:.1} against \
             {fight:.1} LUFS), which is shown for your judgement because the series sets no limit for it."
        ),
        &[
            ("intro_lufs", intro),
            ("fight_lufs", fight),
            ("intro_minus_fight", diff),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_measures_the_floor() {
        // -0.691 + 10 log10(1e-12) = -120.691
        assert_eq!(lufs(&[[0.0, 0.0]; 4]), -0.691 + 10.0 * dmath::log10(1e-12));
        assert!(lufs(&[]).is_nan());
    }
}
