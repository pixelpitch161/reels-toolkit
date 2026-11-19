//! Ability kind `runway`, TENOR SAX's WALK THE BAR: its roster settings. `fight.py` `trigger` 2966 to 2968;
//! `start_runway` 4648 to 4664, `strut_step` 4665 to 4691.
//!
//! A runway from the tenor through the enemy to the far ropes: it struts down it for `beats` beats (it cannot be
//! knocked and does not give way in a clash), bumping the enemy aside on contact (`dmg`, `knock`, once per `cool`
//! seconds) and honking on every beat: a ring of kind `honk` from its surface to `honk_r` beyond it (`honk_dmg`,
//! `honk_knock`), spreading at `honk_speed` (`ab.get('honk_speed', 8.0)`). The entry holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `runway`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind. An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[
    &["beats"],
    &["dmg"],
    &["knock"],
    &["cool"],
    &["honk_r"],
    &["honk_dmg"],
    &["honk_knock"],
];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("runway+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "beats",
    "dmg",
    "knock",
    "cool",
    "honk_r",
    "honk_dmg",
    "honk_knock",
    "honk_speed",
];

/// What an ability of kind `runway` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunwayCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<RunwayCfg, RosterError> {
    Ok(RunwayCfg {})
}

/// The kind's own checks on an entry's numbers: none (a strut of no beats ends in its first step, as in Python).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
