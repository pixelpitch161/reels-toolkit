//! Every fighter and every theme, plus the registry and the pair-rule table.
//!
//! Milestone 1 holds the registry (the kit roster from `data/roster.json`, the fighters the app offers, which
//! fighters' music feeds their fight), the pair-rule table, the fighters themselves ([`fighters`]: the
//! instrument voices, the parts in the score and the looks of the two ported ones: [`look_for`]), the themes
//! ([`themes`]: the big top), the soundtrack of a fight ([`build_soundtrack`]) and its picture
//! ([`build_picture`]). Milestone 2 adds what the interface shows about them ([`card`], [`theme_card`],
//! [`shown_order`]) and the fight a new match-up begins from ([`starter`], [`draft_captions`]).

mod cards;
mod error;
pub mod fighters;
mod pair_rules;
mod picture;
mod registry;
mod soundtrack;
mod starter;
pub mod themes;

pub use cards::{FighterCard, Section, ThemeCard, card, note_name, pair_rule_text, section, shown_order, theme_card};
pub use error::ContentError;
pub use fighters::look_for;
pub use pair_rules::pair_rules;
pub use picture::build_picture;
pub use registry::{PORTED, RETIRED, available, music_feeds_fight, name_status, roster};
pub use soundtrack::{SoundBuild, build_soundtrack, validate};
pub use starter::{draft_captions, starter};
pub use themes::theme_names;
