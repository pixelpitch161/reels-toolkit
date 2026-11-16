//! Ability kind `hive`, SOPRANO SAX's KICK THE HIVE (a from-scratch fighter, 2026-11-16; no Python kit reference): its roster
//! settings. The engine side is in `bf-sim/src/kinds/swarm.rs`.
//!
//! Every orbiting hornet (at least 3) charges the enemy for `dur` (or `dur_beats`), each stinging once for `dmg`.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `hive`.
pub const PORTED: bool = true;

/// The numbers the engine reads as `ab[name]` for this kind.
pub const REQUIRED: &[&[&str]] = &[&["dmg"], &["dur", "dur_beats"]];

/// Every key the engine reads from an ability of this kind.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "dmg", "dur", "dur_beats"];

/// What an ability of kind `hive` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HiveCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<HiveCfg, RosterError> {
    Ok(HiveCfg {})
}

/// The kind's own checks on an entry's numbers: none.
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
