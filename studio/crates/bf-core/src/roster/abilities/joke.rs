//! Ability kind `joke`, VIOLA's VIOLA JOKE: its roster settings. `fight.py` `trigger` 2921 to 2922; 5118 to 5136
//! (`start_joke`).
//!
//! The enemy freezes for `stun_beats` beats (not while shielded or intangible) while the biggest whole note (`size` x
//! the current note radius, `dmg`, `knock`) drops on its spot, landing `tele` beats later. It holds only numbers, so
//! [`JokeCfg`] is empty.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `joke`. While false the roster reads it as
/// `AbilityKind::Unsupported("joke")`, which the engine refuses in words.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["stun_beats"], &["size"], &["dmg"], &["knock"], &["tele"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("joke+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "stun_beats", "size", "dmg", "knock", "tele"];

/// What an ability of kind `joke` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JokeCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<JokeCfg, RosterError> {
    Ok(JokeCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`: none (any number
/// runs, as in Python).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
