//! Ability kind `vibrato`, CELLO's VIBRATO: its roster settings. `fight.py` `trigger` 2955 to 2957; the shakes in
//! `grapple_tick` (4074 to 4076).
//!
//! For `dur` seconds or `dur_beats` beats (`ab_dur`) an enemy its fighter holds in THE EMBRACE shakes and takes `dmg`
//! on every 16th of the hold. It holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `vibrato`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["dur", "dur_beats"], &["dmg"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("vibrato+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "dur", "dur_beats", "dmg"];

/// What an ability of kind `vibrato` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VibratoCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<VibratoCfg, RosterError> {
    Ok(VibratoCfg {})
}

/// The kind's own checks on an entry's numbers: none (every number runs as Python runs it).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
