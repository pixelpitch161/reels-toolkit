//! Ability kind `endless`, OBOE's ENDLESS NOTE: its roster settings. `fight.py` `trigger` 2884 to 2888; the held
//! beam's bite in `tune_tick` (2459 to 2475) and its aim in `tune_step` (2505).
//!
//! The A beam is held for `dur` seconds or `dur_beats` beats (the normal shots pause), `len` long and `width` wide,
//! its aim turning to the enemy at `track` radians a second; on every 8th note the enemy touches it: `dmg`, and out
//! of tune again.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `endless`. While false the roster reads it as
/// `AbilityKind::Unsupported("endless")`, which the engine refuses in words.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["dur", "dur_beats"], &["len"], &["width"], &["track"], &["dmg"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("endless+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "dur",
    "dur_beats",
    "len",
    "width",
    "track",
    "dmg",
];

/// What an ability of kind `endless` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EndlessCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<EndlessCfg, RosterError> {
    Ok(EndlessCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`: none.
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
