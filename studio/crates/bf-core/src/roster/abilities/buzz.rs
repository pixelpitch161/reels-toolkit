//! Ability kind `buzz`, SOPRANO SAX's BUZZ OFF (a from-scratch fighter, 2026-11-16; no Python kit reference): its roster
//! settings. The engine side is in `bf-sim/src/kinds/swarm.rs`.
//!
//! For `dur` (or `dur_beats`) the swarm wraps the fighter: hits on it are blocked (the shield timer), and an enemy that touches it is stung for `dmg` with `knock`, at most twice a beat.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `buzz`.
pub const PORTED: bool = true;

/// The numbers the engine reads as `ab[name]` for this kind.
pub const REQUIRED: &[&[&str]] = &[&["dmg"], &["knock"], &["dur", "dur_beats"]];

/// Every key the engine reads from an ability of this kind.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "dmg", "knock", "dur", "dur_beats"];

/// What an ability of kind `buzz` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuzzCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<BuzzCfg, RosterError> {
    Ok(BuzzCfg {})
}

/// The kind's own checks on an entry's numbers: none.
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
