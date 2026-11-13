//! Auto-balance: the handicap that makes a pairing even.
//!
//! The kit has no such search. `projects/ballfight/kit/tuning.py` fights a grid of handicaps a person typed and
//! prints each cell's figures ("Aim: 40-60 % wins, ~50 s median K.O."), and the person picks a cell. This does the
//! picking: the figures are the kit's ([`crate::stats`]), the aim is the kit's, the search is the Studio's.
//!
//! The two handicaps are written as a scale `s` and a ratio `r`: `(s * sqrt(r), s / sqrt(r))`. The ratio decides
//! who wins (the left fighter hits harder as `r` grows), the scale decides how long a fight lasts (everybody hits
//! harder as `s` grows). Each is found by halving an interval of its logarithm:
//!
//! - round A: `ln r` in `[ln 1/8, ln 8]`, for a win share of 50 %;
//! - round B: `ln s` in `[ln 1/4, ln 4]`, for a median K.O. at 50.5 s (a fight with no K.O. counts as 99 s).
//!
//! A, B, A, B, seven steps each; a round starts from its whole interval again, so a poor first round (the scale it
//! ran at was a guess) is put right by the second. Every step fights the same 400 seeds, so two handicaps are told
//! apart by the fights and not by the luck of their seeds. The result is rounded to 2 decimals and measured once
//! more on 3,000 seeds that no step saw.
//!
//! Balance seeds start at `1 << 40`: the seed search counts up from 1 and never comes near them, so a handicap is
//! never tuned on the fight that is then shown.

use std::sync::atomic::AtomicBool;

use bf_core::dmath;
use bf_core::py::round_nd;
use bf_sim::Setup;

use crate::score::BeatSheet;
use crate::search::Halt;
use crate::stats::{Stats, stats_until};

/// The first balance seed.
pub const BALANCE_SEED0: u64 = 1 << 40;
/// The confirmation's first seed is this far above [`BALANCE_SEED0`].
pub const CONFIRM_OFFSET: u64 = 1_000_000;
/// Round A aims at this share of wins for the left fighter.
pub const TARGET_WIN_SHARE: f64 = 0.5;
/// Round B aims at this median K.O. time, in seconds on the video clock.
pub const TARGET_KO: f64 = 50.5;
/// The ratio between the two handicaps is looked for between 1 / this and this.
pub const RATIO_SPAN: f64 = 8.0;
/// The scale of the two handicaps is looked for between 1 / this and this.
pub const SCALE_SPAN: f64 = 4.0;
/// A, B, A, B.
pub const ROUNDS: usize = 4;
/// In the band: the left fighter wins this share of the decided fights ...
pub const BAND_WIN_SHARE: (f64, f64) = (0.40, 0.60);
/// ... and the median K.O. comes in this window, in seconds.
pub const BAND_KO: (f64, f64) = (49.0, 52.0);

/// How much work a balance run does. The default is the Studio's; tests of the method use a smaller one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BalancePlan {
    /// Fights per step: the seeds `BALANCE_SEED0..BALANCE_SEED0 + eval_seeds`, the same for every step (400).
    pub eval_seeds: u64,
    /// Halvings per round (7: the ratio is then known to 3 %, the scale to 2 %).
    pub steps: usize,
    /// Fights of the confirmation: the seeds from `BALANCE_SEED0 + CONFIRM_OFFSET` (3,000).
    pub confirm_seeds: u64,
}

impl Default for BalancePlan {
    fn default() -> BalancePlan {
        BalancePlan {
            eval_seeds: 400,
            steps: 7,
            confirm_seeds: 3_000,
        }
    }
}

/// What a round looks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BalanceRound {
    /// Round A: the ratio between the handicaps, for the win share.
    Ratio,
    /// Round B: their common scale, for the median K.O. time.
    Scale,
}

/// One step of a round: a handicap that was tried and the figures of its fights.
#[derive(Clone, Debug, PartialEq)]
pub struct BalanceStep {
    /// The round, from 0: A, B, A, B.
    pub round: usize,
    /// What the round looks for.
    pub what: BalanceRound,
    /// The handicap that was tried (not rounded).
    pub handicap: [f64; 2],
    /// The figures of its fights on the evaluation seeds.
    pub stats: Stats,
}

/// How far a balance run has got: `done` of `total` measurements (every step, then the confirmation: 29 for the
/// Studio's plan). Given to the progress callback after each one, on the caller's thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BalanceProgress {
    pub done: usize,
    pub total: usize,
}

/// What a balance run found.
#[derive(Clone, Debug, PartialEq)]
pub struct BalanceResult {
    /// The handicap, left fighter first, rounded to 2 decimals.
    pub handicap: [f64; 2],
    /// The figures of that handicap on the confirmation seeds.
    pub confirm: Stats,
    /// Whether the confirmation is in the band ([`in_band`]).
    pub in_band: bool,
    /// Every step, in the order it was made.
    pub steps: Vec<BalanceStep>,
}

/// Whether a pairing is balanced: the left fighter wins 40 to 60 % of the decided fights and the median K.O. comes
/// between 49 and 52 s (both ends included).
pub fn in_band(stats: &Stats) -> bool {
    (BAND_WIN_SHARE.0 <= stats.win_share && stats.win_share <= BAND_WIN_SHARE.1)
        && (BAND_KO.0 <= stats.ko_median && stats.ko_median <= BAND_KO.1)
}

/// The handicap of a ratio and a scale, both as logarithms: `(s * sqrt(r), s / sqrt(r))`.
fn handicap(ln_r: f64, ln_s: f64) -> [f64; 2] {
    let root = dmath::exp(ln_r).sqrt();
    let s = dmath::exp(ln_s);
    [s * root, s / root]
}

/// Finds the handicap that balances a pairing, starting from `start`, with the Studio's plan (28 steps of 400
/// fights and a confirmation of 3,000: 14,200 fights).
///
/// `setup_for` gives the fight of a seed under a handicap (left fighter first). The handicap is the ONLY thing it
/// may change from one call to the next: it is the rules' damage handicap (`rules.dmg_side`), the one lever a
/// pairing has. The fighters, their roster numbers and every other rule stay as they are; a fight is never edited
/// to make it even. `progress` hears of every
/// measurement. `cancel` is tested before every fight; a run that was stopped is `Halt::Cancelled` and gives
/// nothing. A handicap or a set-up the engine refuses, or a fight that broke off, is `Halt::Failed` with the seed
/// and the reason, said once (the first fight of the first step already says it).
///
/// The result does not depend on `workers` (0 counts as 1).
pub fn auto_balance<'a>(
    setup_for: &(dyn Fn([f64; 2], u64) -> Setup<'a> + Sync),
    sheet: &BeatSheet,
    start: [f64; 2],
    workers: usize,
    progress: &(dyn Fn(BalanceProgress) + Sync),
    cancel: &AtomicBool,
) -> Result<BalanceResult, Halt> {
    auto_balance_with(
        &BalancePlan::default(),
        setup_for,
        sheet,
        start,
        workers,
        progress,
        cancel,
    )
}

/// [`auto_balance`] with another plan.
///
/// Of `start` only the scale is used (the square root of the two handicaps' product): round A looks for the ratio
/// over its whole interval at that scale, wherever the ratio started.
pub fn auto_balance_with<'a>(
    plan: &BalancePlan,
    setup_for: &(dyn Fn([f64; 2], u64) -> Setup<'a> + Sync),
    sheet: &BeatSheet,
    start: [f64; 2],
    workers: usize,
    progress: &(dyn Fn(BalanceProgress) + Sync),
    cancel: &AtomicBool,
) -> Result<BalanceResult, Halt> {
    if !start.iter().all(|h| h.is_finite() && *h > 0.0) {
        return Err(Halt::Failed(format!(
            "handicap: {start:?} cannot be balanced from (each fighter's handicap is a number above 0)"
        )));
    }
    let total = ROUNDS * plan.steps + 1;
    let mut done = 0;
    let mut measure = |h: [f64; 2], from: u64, n: u64| -> Result<Stats, Halt> {
        let got = stats_until(&|seed| setup_for(h, seed), sheet, from..from + n, workers, cancel)?;
        done += 1;
        progress(BalanceProgress { done, total });
        Ok(got)
    };

    // where the search stands: the logarithms of the ratio and of the scale (round A finds its own ratio first)
    let (mut ln_r, mut ln_s) = (0.0, dmath::ln((start[0] * start[1]).sqrt()));
    let mut steps = Vec::with_capacity(ROUNDS * plan.steps);
    for round in 0..ROUNDS {
        let what = if round % 2 == 0 {
            BalanceRound::Ratio
        } else {
            BalanceRound::Scale
        };
        let span = match what {
            BalanceRound::Ratio => RATIO_SPAN,
            BalanceRound::Scale => SCALE_SPAN,
        };
        let (mut lo, mut hi) = (dmath::ln(1.0 / span), dmath::ln(span));
        for _ in 0..plan.steps {
            let mid = (lo + hi) / 2.0;
            let tried = match what {
                BalanceRound::Ratio => handicap(mid, ln_s),
                BalanceRound::Scale => handicap(ln_r, mid),
            };
            let got = measure(tried, BALANCE_SEED0, plan.eval_seeds)?;
            // which half the answer is in: a larger ratio gives the left fighter more wins, a larger scale gives
            // earlier K.O.s
            let too_far = match what {
                BalanceRound::Ratio => got.win_share > TARGET_WIN_SHARE,
                BalanceRound::Scale => got.ko_median < TARGET_KO,
            };
            if too_far {
                hi = mid;
            } else {
                lo = mid;
            }
            steps.push(BalanceStep {
                round,
                what,
                handicap: tried,
                stats: got,
            });
        }
        let found = (lo + hi) / 2.0;
        match what {
            BalanceRound::Ratio => ln_r = found,
            BalanceRound::Scale => ln_s = found,
        }
    }
    let exact = handicap(ln_r, ln_s);
    let rounded = [round_nd(exact[0], 2), round_nd(exact[1], 2)];
    let confirm = measure(rounded, BALANCE_SEED0 + CONFIRM_OFFSET, plan.confirm_seeds)?;
    Ok(BalanceResult {
        handicap: rounded,
        in_band: in_band(&confirm),
        confirm,
        steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn figures(win_share: f64, ko_median: f64) -> Stats {
        Stats {
            n: 3_000,
            decided: 2_900,
            win_share,
            ko_median,
            ko_p10: 40.0,
            ko_p90: 58.0,
            no_ko_share: 0.03,
            lead_changes_mean: 0.6,
            fit_rate: 0.01,
            ko_hist: [0; crate::stats::KO_BINS],
        }
    }

    #[test]
    fn the_band_is_40_to_60_percent_and_49_to_52_s() {
        assert!(in_band(&figures(0.5, 50.5)));
        assert!(
            in_band(&figures(0.40, 49.0)) && in_band(&figures(0.60, 52.0)),
            "the ends are in"
        );
        assert!(!in_band(&figures(0.399, 50.5)) && !in_band(&figures(0.601, 50.5)));
        assert!(!in_band(&figures(0.5, 48.99)) && !in_band(&figures(0.5, 52.01)));
        assert!(!in_band(&figures(f64::NAN, 50.5)));
    }

    #[test]
    fn a_handicap_is_its_scale_times_and_over_the_root_of_its_ratio() {
        // the kit's hand-tuned 2.0 / 0.40: ratio 5, scale sqrt(0.8)
        let h = handicap(dmath::ln(5.0), dmath::ln(0.8_f64.sqrt()));
        assert!((h[0] - 2.0).abs() < 1e-12 && (h[1] - 0.4).abs() < 1e-12, "{h:?}");
        assert_eq!(handicap(0.0, 0.0), [1.0, 1.0]);
        // the ends of the two intervals: no handicap below 0.08 or above 11.4 is ever tried
        let low = handicap(dmath::ln(1.0 / RATIO_SPAN), dmath::ln(1.0 / SCALE_SPAN));
        assert!((low[0] - 0.25 / 8.0_f64.sqrt()).abs() < 1e-12 && (low[1] - 0.25 * 8.0_f64.sqrt()).abs() < 1e-12);
    }
}
