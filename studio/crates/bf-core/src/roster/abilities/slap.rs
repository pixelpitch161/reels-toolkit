//! Ability kind `slap`, CONTRABASS's SLAP BASS: its roster settings. `fight.py` `trigger` 2946 to 2947; 4202 to 4219
//! (`slap_ready`, `start_slap`); the gate in `abilities_step` 3043 to 3045.
//!
//! A slap holds only numbers: `ready` (it waits until the enemy is within this beyond contact, or the fighter is
//! held), and its ring's `speed`, `radius`, `dmg` and `knock`.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `slap`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["ready"], &["speed"], &["radius"], &["dmg"], &["knock"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("slap+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "ready", "speed", "radius", "dmg", "knock"];

/// What an ability of kind `slap` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SlapCfg {}

/// Reads what is not a number from an ability entry of this kind, after [`REQUIRED`] and [`ALLOWED`] are checked:
/// nothing. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<SlapCfg, RosterError> {
    Ok(SlapCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`: none (any number
/// runs, as in Python).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
