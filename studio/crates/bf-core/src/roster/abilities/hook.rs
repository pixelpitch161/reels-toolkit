//! Ability kind `hook`, BASSOON's BOCAL HOOK: its roster settings. `fight.py` `trigger` 2915 to 2916; 3430 to 3476
//! (`hook_ready`, `start_hook`, `hook_step`); the gate in `abilities_step` 3040 to 3042.
//!
//! A hook holds only numbers: `ready` (it waits until the enemy is within this beyond contact), `reach` and `half`
//! (the hit: within `reach` beyond contact and `half` degrees of the aim), `open16` (16ths it winds up), `track`
//! (rad/s the aim turns to the enemy meanwhile), `dmg` and `knock`.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `hook`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[
    &["ready"],
    &["reach"],
    &["half"],
    &["open16"],
    &["track"],
    &["dmg"],
    &["knock"],
];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("hook+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "ready",
    "reach",
    "half",
    "open16",
    "track",
    "dmg",
    "knock",
];

/// What an ability of kind `hook` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HookCfg {}

/// Reads what is not a number from an ability entry of this kind, after [`REQUIRED`] and [`ALLOWED`] are checked:
/// nothing. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<HookCfg, RosterError> {
    Ok(HookCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`: none (any number
/// runs, as in Python).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
