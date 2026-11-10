//! Every fighter and every theme, plus the registry and the pair-rule table.
//!
//! Milestone 1 holds the registry (the kit roster from `data/roster.json`, the fighters the app offers, which
//! fighters' music feeds their fight) and the pair-rule table.

mod pair_rules;
mod registry;

pub use pair_rules::pair_rules;
pub use registry::{PORTED, RETIRED, available, music_feeds_fight, name_status, roster};
