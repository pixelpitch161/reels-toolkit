//! Ability kind `stampede`, CONTRABASS's THE ELEPHANT: its roster settings. `fight.py` `trigger` 2953 to 2954; 4302 to
//! 4368 (`start_stampede`, `stampede_step`).
//!
//! A stampede holds only numbers: `beats` (its length), `speed` (x the shared speed) and `turn` (rad/s) of the
//! charge, the trample's `dmg`, `knock` and `cool` (seconds between two), and the stomps: one every `stomp16` 16ths, a
//! ring `stomp_r` beyond the ball's surface spreading at `stomp_speed` (`ab.get('stomp_speed', 9.0)`) with
//! `stomp_dmg` and `stomp_knock`.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `stampede`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[
    &["beats"],
    &["speed"],
    &["turn"],
    &["dmg"],
    &["knock"],
    &["cool"],
    &["stomp16"],
    &["stomp_r"],
    &["stomp_dmg"],
    &["stomp_knock"],
];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("stampede+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "beats",
    "speed",
    "turn",
    "dmg",
    "knock",
    "cool",
    "stomp16",
    "stomp_r",
    "stomp_speed",
    "stomp_dmg",
    "stomp_knock",
];

/// What an ability of kind `stampede` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StampedeCfg {}

/// Reads what is not a number from an ability entry of this kind, after [`REQUIRED`] and [`ALLOWED`] are checked:
/// nothing. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<StampedeCfg, RosterError> {
    Ok(StampedeCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`: none (any number
/// runs, as in Python: a `stomp16` of 0 or less stomps on every substep of the charge).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
