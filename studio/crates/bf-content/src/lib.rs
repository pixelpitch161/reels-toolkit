//! Every fighter and every theme, plus the registry and the pair-rule table.
//!
//! Milestone 1 holds the registry (the kit roster from `data/roster.json`, the fighters the app offers, which
//! fighters' music feeds their fight), the pair-rule table, the fighters themselves ([`fighters`]: so far the
//! instrument voices and the parts in the score of the two ported ones), the themes ([`themes`]: the big top)
//! and the soundtrack of a fight ([`build_soundtrack`]).

mod error;
pub mod fighters;
mod pair_rules;
mod registry;
mod soundtrack;
pub mod themes;

pub use error::ContentError;
pub use pair_rules::pair_rules;
pub use registry::{PORTED, RETIRED, available, music_feeds_fight, name_status, roster};
pub use soundtrack::{SoundBuild, build_soundtrack, validate};
pub use themes::theme_names;
