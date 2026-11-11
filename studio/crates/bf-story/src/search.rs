//! The seed search: honest fights of one pairing, seed after seed, ranked against the beat sheet. The Studio's
//! `projects/ballfight/kit/search.py`.
//!
//! The kit fights the seeds `a..b` in a process pool, keeps every run, prints the fits as they come and stops once
//! `stop` runs fit. Its pool hands results back in whatever order the workers finish (`imap_unordered`), so which
//! runs it has seen when it stops depends on timing. The Studio's rule is that no result depends on the number of
//! workers: here the workers still take seeds in order, but results are **looked at in seed order**, and the search
//! stops at the seed whose fight is the `stop_fits`-th fit. The seeds checked are then always `from` up to that
//! seed, each one once, whatever the workers were doing.
//!
//! A seed's fight is the Studio's own (`bf_sim::run`): the Rust engine agrees with the kit's fight of the same seed
//! for 25 s or more and may then drift, so a Studio seed is not expected to be the kit's seed.
//!
//! Three things can end a run over seeds before its seeds are used up, and each is said, never guessed:
//!
//! - it has what it was looking for;
//! - the caller's stop flag: tested before every fight, so a search stops within one fight per worker;
//! - a fight that could not be fought: a set-up the engine refuses (`bf_sim::run` returns the reason in words) or a
//!   fight that broke off (a panic, caught in its worker). That ends the run **at that seed** with the reason; it is
//!   never counted as a fight that did not fit, and the seeds after it are not tried (rules that are wrong for one
//!   seed are wrong for every seed).

use std::any::Any;
use std::collections::BTreeMap;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use bf_sim::{Setup, SimError};

use crate::score::{BeatSheet, Verdict, score};
use crate::story::{Story, story_at};

/// One seed's fight, read and judged: a row of the shortlist.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    /// The seed of the fight.
    pub seed: u64,
    /// What happened in it.
    pub story: Story,
    /// Whether it fits the beat sheet, its score, and why it does not fit.
    pub verdict: Verdict,
}

/// How far a search goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchOpts {
    /// The first seed (the Studio searches from 1; balance owns the seeds from `1 << 40`).
    pub from: u64,
    /// Stop at the seed whose fight is this many-th fit (the Studio's shortlist is full at 200).
    pub stop_fits: usize,
    /// Check at most this many seeds; `None`: go on until `stop_fits`, the stop flag, or the last seed there is.
    /// Every seed checked stays in the result, so a search without a limit should have a stop flag someone sets.
    pub max_seeds: Option<u64>,
    /// Worker threads (0 counts as 1). The result does not depend on it.
    pub workers: usize,
}

impl Default for SearchOpts {
    /// From seed 1 until 200 fits, on every core.
    fn default() -> SearchOpts {
        SearchOpts {
            from: 1,
            stop_fits: 200,
            max_seeds: None,
            workers: std::thread::available_parallelism().map_or(1, |n| n.get()),
        }
    }
}

/// How far a search has got. Given to the progress callback once per seed, in seed order, from a worker thread
/// that holds the search's lock: the callback should be quick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchProgress {
    /// Seeds checked so far: `from` to `from + checked - 1`, every one of them.
    pub checked: u64,
    /// How many of them fit.
    pub fits: usize,
}

/// Why a search ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchEnd {
    /// It found `stop_fits` fits; the last seed checked is the last of them.
    Full,
    /// It checked `max_seeds` seeds (or came to the largest seed there is) without finding `stop_fits` fits.
    OutOfSeeds,
    /// The stop flag was set.
    Stopped,
    /// A fight could not be fought; the words name the seed and say why (one line). The candidates are those of
    /// the seeds before it.
    Failed(String),
}

/// What a search found.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchResult {
    /// Every seed checked, as [`rank`] orders them: the fits first, then the rest; each by score downward, then by
    /// seed upward.
    pub candidates: Vec<Candidate>,
    /// How many seeds were checked: `from` to `from + checked - 1`, each one once. A search that goes on starts at
    /// `from + checked`.
    pub checked: u64,
    /// Why it ended.
    pub end: SearchEnd,
}

/// Why a statistics or balance run gave no result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Halt {
    /// The stop flag was set.
    Cancelled,
    /// A fight could not be fought (or the request made no sense); the words say why, in one line.
    Failed(String),
}

impl fmt::Display for Halt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Halt::Cancelled => f.write_str("stopped"),
            Halt::Failed(words) => f.write_str(words),
        }
    }
}

impl std::error::Error for Halt {}

/// One fight, read and judged: `kit/search.py` `one(seed)` (`story.run_seed(seed, record=False)`, `story.story`,
/// `story.score`). The kit rounds the times of the story for its files there; a candidate keeps them whole.
pub fn fight(setup: &Setup, sheet: &BeatSheet) -> Result<Candidate, SimError> {
    let rec = bf_sim::run(setup, false)?;
    let lows = [setup.fighters[0].low.name.as_str(), setup.fighters[1].low.name.as_str()];
    let story = story_at(&rec, setup.rules.t_fight, sheet, lows);
    let verdict = score(&story, sheet);
    Ok(Candidate {
        seed: setup.seed,
        story,
        verdict,
    })
}

/// The shortlist's order: the fits first, then the rest; each by score downward (the kit's
/// `fits.sort(key=lambda r: -r['score'])`), equal scores by seed upward.
pub fn rank(candidates: &mut [Candidate]) {
    candidates.sort_by(|a, b| {
        b.verdict
            .fits
            .cmp(&a.verdict.fits)
            .then(b.verdict.score.total_cmp(&a.verdict.score))
            .then(a.seed.cmp(&b.seed))
    });
}

/// Fights the seeds from `opts.from` upward and returns every seed checked, ranked, with how far it got and why it
/// ended. `setup_for` gives the fight of a seed (the seed handed out is the seed fought, whatever seed the set-up
/// comes back with). `progress` hears of every seed, in seed order. `cancel` is tested before every fight: set it
/// from any thread and the search returns what it has, which is whole (every seed from `from` to the last one
/// checked).
///
/// The result is the same for any number of workers: it is decided by the seeds alone, read in order (see the
/// module's words). Only a stop by `cancel` depends on when it was set.
///
/// A panic of `progress` is not caught: it ends the search and goes on to the caller.
pub fn search<'a>(
    setup_for: &(dyn Fn(u64) -> Setup<'a> + Sync),
    sheet: &BeatSheet,
    opts: &SearchOpts,
    progress: &(dyn Fn(SearchProgress) + Sync),
    cancel: &AtomicBool,
) -> SearchResult {
    let mut candidates = Vec::new();
    let mut fits = 0usize;
    let end = if opts.stop_fits == 0 {
        End::Enough
    } else {
        run_seeds(
            opts.from,
            opts.max_seeds,
            opts.workers,
            &|seed| fight_of(setup_for, sheet, seed),
            &mut |_, c: Candidate| {
                fits += usize::from(c.verdict.fits);
                candidates.push(c);
                progress(SearchProgress {
                    checked: candidates.len() as u64,
                    fits,
                });
                fits >= opts.stop_fits
            },
            cancel,
        )
    };
    let checked = candidates.len() as u64;
    rank(&mut candidates);
    SearchResult {
        candidates,
        checked,
        end: match end {
            End::Enough => SearchEnd::Full,
            End::Ran => SearchEnd::OutOfSeeds,
            End::Cancelled => SearchEnd::Stopped,
            End::Failed(words) => SearchEnd::Failed(words),
        },
    }
}

/// The fight of one seed, or why there is none, in words.
pub(crate) fn fight_of<'a>(
    setup_for: &(dyn Fn(u64) -> Setup<'a> + Sync),
    sheet: &BeatSheet,
    seed: u64,
) -> Result<Candidate, String> {
    let mut setup = setup_for(seed);
    setup.seed = seed;
    fight(&setup, sheet).map_err(|e| e.to_string())
}

/// How a run over seeds ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum End {
    /// Every seed was looked at.
    Ran,
    /// `take` said it had enough.
    Enough,
    /// The stop flag was set before that.
    Cancelled,
    /// A seed's job failed: the seed and the reason, in one line.
    Failed(String),
}

/// The most worker threads a run starts.
const MAX_WORKERS: usize = 256;

/// What the workers of one run share behind its lock: the results that are waiting for an earlier seed, and how
/// far the reading in seed order has got.
struct Line<'t, T> {
    /// Results by index (seed minus `from`) that cannot be read yet.
    waiting: BTreeMap<u64, Result<T, String>>,
    /// The index the reading waits for: every index below it has been given to `take`.
    next: u64,
    take: &'t mut (dyn FnMut(u64, T) -> bool + Send),
    end: Option<End>,
}

/// Runs `one(seed)` for the seeds `from`, `from + 1`, ... (`count` of them; `None`: up to the largest `u64`) on
/// `workers` threads and gives the results to `take` **in seed order**, one at a time. The run ends when `take`
/// answers true (it has enough), at the first seed, in seed order, whose job returned an error or panicked, when
/// the seeds are used up, or when `cancel` is set (tested before every job). Jobs past the end that were already
/// running are finished and thrown away, so what `take` saw is decided by the seeds alone, not by the number of
/// workers or by timing.
pub(crate) fn run_seeds<T: Send>(
    from: u64,
    count: Option<u64>,
    workers: usize,
    one: &(dyn Fn(u64) -> Result<T, String> + Sync),
    take: &mut (dyn FnMut(u64, T) -> bool + Send),
    cancel: &AtomicBool,
) -> End {
    // the seeds are `from + i` for `i` below `limit` (there are 2^64 - from seeds from `from` on)
    let limit = count.unwrap_or(u64::MAX).min((u64::MAX - from).saturating_add(1));
    if limit == 0 {
        return End::Ran;
    }
    let workers = usize::try_from(limit)
        .map_or(workers, |n| workers.min(n))
        .clamp(1, MAX_WORKERS);
    let pool = match rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .thread_name(|i| format!("bf-story-{i}"))
        .build()
    {
        Ok(pool) => pool,
        Err(e) => return End::Failed(format!("could not start {workers} worker threads: {e}")),
    };
    let hand = AtomicU64::new(0);
    let halt = AtomicBool::new(false);
    let line = Mutex::new(Line {
        waiting: BTreeMap::new(),
        next: 0,
        take,
        end: None,
    });
    // Every worker thread runs this loop. A panic in it (only `take` can panic: a job's own are caught) poisons the
    // lock, the other workers stop there, and the panic goes on to the caller.
    pool.broadcast(|_| {
        loop {
            if halt.load(Ordering::SeqCst) || cancel.load(Ordering::SeqCst) {
                return;
            }
            let i = hand.fetch_add(1, Ordering::SeqCst);
            if i >= limit {
                return;
            }
            let seed = from + i;
            let out = catch_unwind(AssertUnwindSafe(|| one(seed)))
                .unwrap_or_else(|panic| Err(format!("the fight broke off ({})", panic_words(panic.as_ref()))))
                .map_err(|why| format!("seed {seed}: {why}"));
            let Ok(mut line) = line.lock() else {
                halt.store(true, Ordering::SeqCst);
                return;
            };
            line.waiting.insert(i, out);
            while line.end.is_none() {
                let next = line.next;
                let Some(out) = line.waiting.remove(&next) else {
                    break;
                };
                match out {
                    Ok(value) => {
                        line.next = next + 1;
                        if (line.take)(from + next, value) {
                            line.end = Some(End::Enough);
                        } else if line.next == limit {
                            line.end = Some(End::Ran);
                        }
                    }
                    Err(words) => line.end = Some(End::Failed(words)),
                }
            }
            if line.end.is_some() {
                halt.store(true, Ordering::SeqCst);
                return;
            }
        }
    });
    let line = line.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner);
    // no end was reached in seed order: only the stop flag sends the workers home before one is
    line.end.unwrap_or(End::Cancelled)
}

/// What a panic said, as one line of text.
fn panic_words(panic: &(dyn Any + Send)) -> String {
    let words = panic
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "an internal error".to_string());
    words.lines().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(from: u64, count: Option<u64>, workers: usize, stop_at: Option<u64>) -> (Vec<u64>, End) {
        let mut seen = Vec::new();
        let end = run_seeds(
            from,
            count,
            workers,
            &|seed| {
                // some seeds take longer, so later seeds finish before earlier ones
                std::thread::sleep(std::time::Duration::from_micros(200 * (seed % 5)));
                Ok(seed.wrapping_mul(10))
            },
            &mut |seed, v| {
                assert_eq!(v, seed.wrapping_mul(10));
                seen.push(seed);
                Some(seed) == stop_at
            },
            &AtomicBool::new(false),
        );
        (seen, end)
    }

    #[test]
    fn results_are_read_in_seed_order_whatever_the_workers_do() {
        for workers in [0, 1, 3, 16, 1000] {
            let (seen, end) = collect(7, Some(60), workers, None);
            assert_eq!(seen, (7..67).collect::<Vec<_>>(), "{workers} workers");
            assert_eq!(end, End::Ran);
            let (seen, end) = collect(7, None, workers, Some(31));
            assert_eq!(seen, (7..=31).collect::<Vec<_>>(), "{workers} workers");
            assert_eq!(end, End::Enough);
        }
    }

    #[test]
    fn the_last_seed_can_be_enough() {
        let (seen, end) = collect(1, Some(9), 4, Some(9));
        assert_eq!((seen.len(), end), (9, End::Enough));
    }

    #[test]
    fn no_seeds_is_a_run_that_ran() {
        let (seen, end) = collect(5, Some(0), 4, None);
        assert_eq!((seen.len(), end), (0, End::Ran));
    }

    #[test]
    fn the_seeds_end_at_the_largest_there_is() {
        let (seen, end) = collect(u64::MAX - 2, None, 4, None);
        assert_eq!(seen, [u64::MAX - 2, u64::MAX - 1, u64::MAX]);
        assert_eq!(end, End::Ran);
        let (seen, end) = collect(u64::MAX, Some(u64::MAX), 4, None);
        assert_eq!((seen, end), (vec![u64::MAX], End::Ran));
        let (seen, end) = collect(0, Some(3), 4, None);
        assert_eq!((seen, end), (vec![0, 1, 2], End::Ran));
    }

    #[test]
    fn the_first_failure_in_seed_order_ends_the_run() {
        for workers in [1, 2, 7] {
            let mut seen = Vec::new();
            let end = run_seeds(
                1,
                None,
                workers,
                &|seed| match seed {
                    // seed 12 fails at once, seed 9 after a while: seed 9 is the one that is told
                    9 => {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        Err("nine is no good".to_string())
                    }
                    12 => Err("twelve neither".to_string()),
                    _ => Ok(seed),
                },
                &mut |seed, _| {
                    seen.push(seed);
                    false
                },
                &AtomicBool::new(false),
            );
            assert_eq!(seen, (1..9).collect::<Vec<_>>(), "{workers} workers");
            assert_eq!(end, End::Failed("seed 9: nine is no good".to_string()));
        }
    }

    #[test]
    fn a_panic_in_a_job_is_a_failure_in_words() {
        for workers in [1, 5] {
            let mut seen = 0;
            let end = run_seeds(
                1,
                Some(50),
                workers,
                &|seed| {
                    if seed == 3 {
                        panic!("three\nbroke");
                    }
                    if seed == 6 {
                        std::panic::panic_any(6_u8);
                    }
                    Ok(())
                },
                &mut |_, ()| {
                    seen += 1;
                    false
                },
                &AtomicBool::new(false),
            );
            assert_eq!(seen, 2);
            assert_eq!(
                end,
                End::Failed("seed 3: the fight broke off (three broke)".to_string())
            );
        }
        let end = run_seeds(
            6,
            Some(1),
            1,
            &|_| -> Result<(), String> { std::panic::panic_any(6_u8) },
            &mut |_, ()| false,
            &AtomicBool::new(false),
        );
        assert_eq!(
            end,
            End::Failed("seed 6: the fight broke off (an internal error)".to_string())
        );
    }

    #[test]
    fn the_stop_flag_is_seen_before_the_next_job() {
        let cancel = AtomicBool::new(false);
        let started = AtomicU64::new(0);
        let mut seen = Vec::new();
        let end = run_seeds(
            1,
            None,
            4,
            &|seed| {
                if started.fetch_add(1, Ordering::SeqCst) == 10 {
                    cancel.store(true, Ordering::SeqCst);
                }
                Ok(seed)
            },
            &mut |seed, _| {
                seen.push(seed);
                false
            },
            &cancel,
        );
        assert_eq!(end, End::Cancelled);
        let started = started.load(Ordering::SeqCst);
        assert!((11..=11 + 4).contains(&started), "{started} jobs were started");
        // what was read is a run of seeds from the first one, with no hole
        assert_eq!(seen, (1..=seen.len() as u64).collect::<Vec<_>>());
        assert!(seen.len() as u64 <= started);
    }

    #[test]
    fn a_panic_of_the_reader_reaches_the_caller_and_hangs_nothing() {
        let caught = catch_unwind(AssertUnwindSafe(|| {
            run_seeds(
                1,
                None,
                6,
                &|seed| Ok(seed),
                &mut |seed, _| {
                    assert!(seed < 20, "the reader gave up");
                    false
                },
                &AtomicBool::new(false),
            )
        }));
        let panic = caught.expect_err("the reader's panic comes out of the run");
        assert_eq!(panic_words(panic.as_ref()), "the reader gave up");
    }

    #[test]
    fn rank_puts_fits_first_then_score_then_seed() {
        let c = |seed, fits, score| Candidate {
            seed,
            story: Story {
                seed,
                winner: None,
                t_ko: None,
                lead_changes: 0,
                lc_t: vec![],
                winner_hp: 100.0,
                firsts: vec![],
                caps: vec![],
                max_gap: 99.0,
                lows: vec![],
                lead: vec![],
            },
            verdict: Verdict {
                fits,
                score,
                misses: vec![],
            },
        };
        let mut list = vec![
            c(1, false, 9.0),
            c(2, true, -1.0),
            c(3, true, 4.5),
            c(4, false, 9.0),
            c(5, true, 4.5),
            c(6, false, -30.0),
            c(7, true, 7.0),
        ];
        rank(&mut list);
        assert_eq!(list.iter().map(|c| c.seed).collect::<Vec<_>>(), [7, 3, 5, 2, 1, 4, 6]);
    }
}
