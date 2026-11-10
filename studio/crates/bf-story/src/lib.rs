//! Story analysis of a recording, the beat-sheet fit and score, seed search, balance and shortlist filters.
//!
//! [`story`] reads one recorded fight the way `projects/ballfight/kit/story.py` `story()` does (who led, the lead
//! changes, the captions, the desperation moves, the longest stretch without a hit); [`score`] is that file's
//! `score()`: the beat sheet's hard windows and the soft story score the seed search ranks fights by.

pub mod score;
pub mod story;

pub use score::{BeatSheet, Verdict, score};
pub use story::{Story, story};
