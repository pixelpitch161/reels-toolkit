//! Ability kind `swan`, CELLO's SWAN SONG: its roster settings. `fight.py` `trigger` 2951 to 2952; 4244 to 4301
//! (`start_swan`, `swan_step`).
//!
//! The fighter rises for `rise_beats`, glides for `glide_beats` along a curve that bows `bow` toward the ring's
//! centre to the enemy's spot predicted `lead` (default 1.0) x 0.6 of its motion, dives for `dive_beats` and crashes:
//! an enemy within `radius` (+0.6 R) takes `dmg` and `knock`. `height` (default 1.5) is only recorded in its event,
//! for the picture. It holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `swan`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind. An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[
    &["rise_beats"],
    &["glide_beats"],
    &["dive_beats"],
    &["bow"],
    &["radius"],
    &["dmg"],
    &["knock"],
];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("swan+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "rise_beats",
    "glide_beats",
    "dive_beats",
    "bow",
    "lead",
    "height",
    "radius",
    "dmg",
    "knock",
];

/// What an ability of kind `swan` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SwanCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<SwanCfg, RosterError> {
    Ok(SwanCfg {})
}

/// The kind's own checks on an entry's numbers: none (every number runs as Python runs it).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
