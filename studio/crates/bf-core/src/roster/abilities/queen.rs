//! Ability kind `queen`, SOPRANO SAX's QUEEN (a from-scratch fighter, 2026-11-16; no Python kit reference): its roster
//! settings. The engine side is in `bf-sim/src/kinds/swarm.rs`.
//!
//! A queen hornet chases the enemy at `speed` for `dur` (or `dur_beats`), stinging for `dmg` with `knock` at most once every `gap` beats.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `queen`.
pub const PORTED: bool = true;

/// The numbers the engine reads as `ab[name]` for this kind.
pub const REQUIRED: &[&[&str]] = &[&["speed"], &["dmg"], &["knock"], &["gap"], &["dur", "dur_beats"]];

/// Every key the engine reads from an ability of this kind.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "speed",
    "dmg",
    "knock",
    "gap",
    "dur",
    "dur_beats",
];

/// What an ability of kind `queen` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct QueenCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<QueenCfg, RosterError> {
    Ok(QueenCfg {})
}

/// The kind's own checks on an entry's numbers: none.
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
