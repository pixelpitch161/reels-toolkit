//! Ability kind `derby`, CORNET's DERBY HAT (a from-scratch fighter, 2026-11-16; no Python kit reference): its roster
//! settings. The engine side is in `bf-sim/src/kinds/plunger.rs`.
//!
//! A bowler hat is lobbed at where the enemy is heading and lands `tele` beats later. An enemy within `radius` of
//! where it lands takes `dmg` and is muffled for `dur` (or `dur_beats`): it keeps moving but starts no attack.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `derby`.
pub const PORTED: bool = true;

/// The numbers the engine reads as `ab[name]` for this kind.
pub const REQUIRED: &[&[&str]] = &[&["tele"], &["radius"], &["dmg"], &["dur", "dur_beats"]];

/// Every key the engine reads from an ability of this kind.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "tele", "radius", "dmg", "dur", "dur_beats"];

/// What an ability of kind `derby` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DerbyCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<DerbyCfg, RosterError> {
    Ok(DerbyCfg {})
}

/// The kind's own checks on an entry's numbers: none.
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
