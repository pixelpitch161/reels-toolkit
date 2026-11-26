//! Ability kind `offbeat`, BARI SAX's SKANK: its roster settings. `fight.py` `trigger` 2911 to 2914; its cycles in
//! `wreck_tick` (3666 to 3684): for `dur` (or `dur_beats`) a swing starts every bar on the weapon's `skank_off16`.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `offbeat`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["dur", "dur_beats"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("offbeat+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "dur", "dur_beats"];

/// What an ability of kind `offbeat` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OffbeatCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<OffbeatCfg, RosterError> {
    Ok(OffbeatCfg {})
}

/// The kind's own checks on an entry's numbers: none (any number runs, as in Python).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
