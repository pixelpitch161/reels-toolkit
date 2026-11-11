//! Many fights pooled into a few figures: who wins how often, when the K.O. comes, how often the lead flips, how
//! many fights fit the beat sheet. The figures `projects/ballfight/kit/tuning.py` prints for a pairing, defined as
//! the reference export `tools/studio_export/stats.py` writes them (`pooled`), numpy's median and percentile
//! included.
//!
//! The figures are of the Studio's own fights. A Studio seed's fight may end otherwise than the kit's fight of
//! that seed (the engines agree for 25 s or more of a fight, then drift in the last bits), so the two are compared
//! as distributions over many seeds, never seed by seed: `tests/stats_parity.rs`.

use std::ops::Range;
use std::sync::atomic::AtomicBool;

use bf_sim::Setup;

use crate::score::BeatSheet;
use crate::search::{Candidate, End, Halt, fight_of, run_seeds};

/// What a fight with no K.O. counts as in the K.O. times, in seconds (`r['t_ko'] or 99` in `kit/tuning.py`, as in
/// `kit/story.py` `score`).
pub const NO_KO_T: f64 = 99.0;

/// The pooled figures of `n` fights.
#[derive(Clone, Debug, PartialEq)]
pub struct Stats {
    /// How many fights.
    pub n: usize,
    /// How many of them ended by K.O.
    pub decided: usize,
    /// The left fighter's share of the decided fights, 0 to 1 (fights with no K.O. left out); 0 when none was
    /// decided, as the kit prints it.
    pub win_share: f64,
    /// The median K.O. time over all `n` fights, a fight with no K.O. counting as [`NO_KO_T`] (numpy's median).
    pub ko_median: f64,
    /// The 10th percentile of the same times (numpy's `percentile`, linear).
    pub ko_p10: f64,
    /// Their 90th percentile.
    pub ko_p90: f64,
    /// The share of fights with no K.O., 0 to 1.
    pub no_ko_share: f64,
    /// The mean number of lead changes over all `n` fights.
    pub lead_changes_mean: f64,
    /// The share of fights that fit the beat sheet, 0 to 1 (over all `n`; a fight with no K.O. never fits the
    /// series sheet).
    pub fit_rate: f64,
}

/// What the figures need of one fight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outcome {
    /// The fighter left standing; `None` without a K.O.
    pub winner: Option<usize>,
    /// The K.O. time; `None` without a K.O.
    pub t_ko: Option<f64>,
    /// How often the lead flipped.
    pub lead_changes: u32,
    /// Whether the fight fits the beat sheet.
    pub fits: bool,
}

impl Outcome {
    /// The outcome of a judged fight.
    pub fn of(c: &Candidate) -> Outcome {
        Outcome {
            winner: c.story.winner,
            t_ko: c.story.t_ko,
            lead_changes: c.story.lead_changes,
            fits: c.verdict.fits,
        }
    }
}

/// numpy's `_lerp(a, b, t)`: `a + (b - a) * t`, and `b - (b - a) * (1 - t)` from the middle on.
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    let diff = b - a;
    if t >= 0.5 { b - diff * (1.0 - t) } else { a + diff * t }
}

/// `np.percentile(a, q)` (method `linear`) of an ascending list that is not empty: the value at the virtual index
/// `(n - 1) * (q / 100)`, between its two neighbours by [`lerp`].
fn percentile(sorted: &[f64], q: f64) -> f64 {
    let n = sorted.len();
    let virt = (n - 1) as f64 * (q / 100.0);
    if virt >= (n - 1) as f64 {
        return sorted[n - 1];
    }
    let prev = virt.floor();
    let i = prev as usize;
    lerp(sorted[i], sorted[i + 1], virt - prev)
}

/// `np.median(a)` of an ascending list that is not empty: the middle value, or the mean of the two in the middle.
fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

impl Stats {
    /// The pooled figures of these fights, each defined as `tools/studio_export/stats.py` `pooled` has it; `None`
    /// for no fights at all (a median of nothing is not a number).
    ///
    /// The order of the rows does not matter: every figure is a count, a sum of whole numbers or read off the
    /// sorted K.O. times.
    pub fn of(rows: &[Outcome]) -> Option<Stats> {
        let n = rows.len();
        if n == 0 {
            return None;
        }
        // dec = [r for r in rows if r['winner'] is not None]
        let decided = rows.iter().filter(|r| r.winner.is_some()).count();
        let left = rows.iter().filter(|r| r.winner == Some(0)).count();
        // tko = [r['t_ko'] or 99 ...]: None and 0.0 are both false in Python
        let mut tko: Vec<f64> = rows
            .iter()
            .map(|r| r.t_ko.filter(|&t| t != 0.0).unwrap_or(NO_KO_T))
            .collect();
        tko.sort_by(f64::total_cmp);
        let lead_changes: u64 = rows.iter().map(|r| u64::from(r.lead_changes)).sum();
        let fits = rows.iter().filter(|r| r.fits).count();
        Some(Stats {
            n,
            decided,
            // kit/tuning.py: `np.mean([x[2] == 0 for x in dec]) if dec else 0.0`
            win_share: if decided == 0 {
                0.0
            } else {
                left as f64 / decided as f64
            },
            ko_median: median(&tko),
            ko_p10: percentile(&tko, 10.0),
            ko_p90: percentile(&tko, 90.0),
            no_ko_share: (n - decided) as f64 / n as f64,
            lead_changes_mean: lead_changes as f64 / n as f64,
            fit_rate: fits as f64 / n as f64,
        })
    }
}

/// The pooled figures of the fights of `seeds` on `workers` threads (0 counts as 1; the figures do not depend on
/// it). `setup_for` gives the fight of a seed, as for [`crate::search`].
///
/// A fight that cannot be fought (a set-up the engine refuses, a fight that broke off) is `Halt::Failed` with the
/// seed and the reason, said once: the figures of the other fights are not given, since they would not be the
/// figures of `seeds`. So is an empty range.
pub fn stats<'a>(
    setup_for: &(dyn Fn(u64) -> Setup<'a> + Sync),
    sheet: &BeatSheet,
    seeds: Range<u64>,
    workers: usize,
) -> Result<Stats, Halt> {
    stats_until(setup_for, sheet, seeds, workers, &AtomicBool::new(false))
}

/// [`stats`] with a stop flag, tested before every fight: `Halt::Cancelled` when it was set before the last fight
/// was fought.
pub fn stats_until<'a>(
    setup_for: &(dyn Fn(u64) -> Setup<'a> + Sync),
    sheet: &BeatSheet,
    seeds: Range<u64>,
    workers: usize,
    cancel: &AtomicBool,
) -> Result<Stats, Halt> {
    let count = seeds.end.saturating_sub(seeds.start);
    if count == 0 {
        return Err(Halt::Failed(format!(
            "no fights to count: the seeds {} to {} are none",
            seeds.start, seeds.end
        )));
    }
    let mut rows = Vec::new();
    let end = run_seeds(
        seeds.start,
        Some(count),
        workers,
        &|seed| fight_of(setup_for, sheet, seed).map(|c| Outcome::of(&c)),
        &mut |_, row| {
            rows.push(row);
            false
        },
        cancel,
    );
    match end {
        End::Ran | End::Enough => Stats::of(&rows).ok_or_else(|| Halt::Failed("no fights to count".to_string())),
        End::Cancelled => Err(Halt::Cancelled),
        End::Failed(words) => Err(Halt::Failed(words)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(winner: Option<usize>, t_ko: Option<f64>, lead_changes: u32, fits: bool) -> Outcome {
        Outcome {
            winner,
            t_ko,
            lead_changes,
            fits,
        }
    }

    #[test]
    fn figures_of_a_handful_of_fights() {
        // numpy: tko = [40, 50, 52, 99, 99] -> median 52, p10 = 44, p90 = 99
        let rows = [
            row(Some(0), Some(50.0), 2, true),
            row(Some(1), Some(40.0), 0, false),
            row(None, None, 1, false),
            row(Some(0), Some(52.0), 3, false),
            row(None, None, 0, false),
        ];
        let s = Stats::of(&rows).unwrap();
        assert_eq!((s.n, s.decided), (5, 3));
        assert_eq!(s.win_share, 2.0 / 3.0);
        assert_eq!((s.ko_median, s.ko_p10, s.ko_p90), (52.0, 44.0, 99.0));
        assert_eq!(s.no_ko_share, 0.4);
        assert_eq!(s.lead_changes_mean, 1.2);
        assert_eq!(s.fit_rate, 0.2);
        // the order of the fights does not matter
        let mut back = rows;
        back.reverse();
        assert_eq!(Stats::of(&back).unwrap(), s);
    }

    #[test]
    fn nobody_decided_is_a_win_share_of_zero() {
        let s = Stats::of(&[row(None, None, 0, false), row(None, None, 0, false)]).unwrap();
        assert_eq!((s.decided, s.win_share, s.no_ko_share), (0, 0.0, 1.0));
        assert_eq!((s.ko_median, s.ko_p10, s.ko_p90), (NO_KO_T, NO_KO_T, NO_KO_T));
    }

    #[test]
    fn one_fight_is_its_own_median_and_percentiles() {
        let s = Stats::of(&[row(Some(1), Some(47.5), 1, true)]).unwrap();
        assert_eq!((s.ko_median, s.ko_p10, s.ko_p90), (47.5, 47.5, 47.5));
        assert_eq!((s.win_share, s.fit_rate, s.lead_changes_mean), (0.0, 1.0, 1.0));
    }

    #[test]
    fn the_percentile_is_numpys() {
        // np.percentile([1.0, 2.0, 4.0, 8.0], q) for q = 0, 10, 50, 90, 100 on the reference PC
        let a = [1.0, 2.0, 4.0, 8.0];
        assert_eq!(percentile(&a, 0.0), 1.0);
        assert_eq!(percentile(&a, 10.0), 1.3);
        assert_eq!(percentile(&a, 50.0), 3.0);
        assert_eq!(percentile(&a, 90.0), 6.800000000000001);
        assert_eq!(percentile(&a, 100.0), 8.0);
        assert_eq!(median(&a), 3.0);
    }
}
