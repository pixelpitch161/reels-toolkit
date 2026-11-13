//! Ability kind `snap`, BARI SAX's CABLE SNAP: its roster settings. `fight.py` `trigger` 2919 to 2920; 3787 to 3856
//! (`start_snap`, `snap_step`).
//!
//! A snap holds only numbers: `tele16` (16ths from the start to the crash), `hoist` (the share of it spent hoisting,
//! `.get('hoist', 0.6)`), `height`, `radius` (the crash hits within this beyond contact), `dmg`, `knock`, `lead`
//! (`.get('lead', 1.0)`) and `air_edge` (`.get('air_edge', <the weapon's or 0>)`).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `snap`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind. An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["tele16"], &["height"], &["radius"], &["dmg"], &["knock"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("snap+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "tele16",
    "hoist",
    "height",
    "radius",
    "dmg",
    "knock",
    "lead",
    "air_edge",
];

/// What an ability of kind `snap` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SnapCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<SnapCfg, RosterError> {
    Ok(SnapCfg {})
}

/// The kind's own checks on an entry's numbers: none (any number runs, as in Python).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
