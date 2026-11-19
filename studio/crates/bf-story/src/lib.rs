//! Story analysis of a recording, the beat-sheet fit and score, seed search, balance and shortlist filters.
//!
//! [`story`] reads one recorded fight the way `projects/ballfight/kit/story.py` `story()` does (who led, the lead
//! changes, the captions, the desperation moves, the longest stretch without a hit); [`score`] is that file's
//! `score()`: the beat sheet's hard windows and the soft story score the seed search ranks fights by.
//!
//! [`search`] fights seeds upward and keeps a ranked shortlist (`kit/search.py`); [`Filters`] narrow it;
//! [`stats`] pools many fights into the figures `kit/tuning.py` prints, and [`auto_balance`] finds the handicap
//! that makes a pairing even. All three give the same answer for any number of worker threads.

pub mod balance;
pub mod filters;
pub mod score;
pub mod search;
pub mod stats;
pub mod story;

pub use balance::{
    BALANCE_SEED0, BalancePlan, BalanceProgress, BalanceResult, BalanceRound, BalanceStep, auto_balance,
    auto_balance_with, in_band,
};
pub use filters::Filters;
pub use score::{BeatSheet, Verdict, score};
pub use search::{
    Candidate, Halt, KEEP_MISFITS, SearchEnd, SearchOpts, SearchProgress, SearchResult, fight, rank, search, shortlist,
};
pub use stats::{KO_BINS, KO_HIST_FROM, Outcome, Stats, ko_hist, stats, stats_until};
pub use story::{Story, story, story_at};
