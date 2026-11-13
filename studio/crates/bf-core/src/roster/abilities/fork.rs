//! Ability kind `fork`, ELECTRIC's DISTORTION: its roster settings. `fight.py` `trigger` 2933 to 2935.
//!
//! For `dur` s or `dur_beats` beats every bolt of the LIGHTNING RIFF forks: the headstock's reach and the posts'
//! (`reach`, `post_r`, and the reach of a miss) are times `reach_k` (`fork_ab.get('reach_k', 1.0)`). It holds only
//! numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `fork`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["dur", "dur_beats"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("fork+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "dur", "dur_beats", "reach_k"];

/// What an ability of kind `fork` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ForkCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<ForkCfg, RosterError> {
    Ok(ForkCfg {})
}

/// The kind's own checks on an entry's numbers: none (every number runs in Python).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
