//! Ability kind `sustain`, PIANO's SUSTAIN: its roster settings. `fight.py` `trigger` 2941 to 2943; the roll reads the
//! rest in `fire_row` (5236 to 5242).
//!
//! For `dur` seconds or `dur_beats` beats the roll's rows hang like mines: a shot flies for `out16` 16ths (plus
//! `step16` for each hole before it in its row), stops for `hang_beats` beats, then turns to the enemy and flies on.
//! `fire_row` reads those three from the fighter's `unlock1`, each with a default (2, 0 and 1).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `sustain`. While false the roster reads it as
/// `AbilityKind::Unsupported("sustain")`, which the engine refuses in words.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["dur", "dur_beats"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("sustain+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "dur",
    "dur_beats",
    "out16",
    "step16",
    "hang_beats",
];

/// What an ability of kind `sustain` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SustainCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<SustainCfg, RosterError> {
    Ok(SustainCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`: none (every number
/// runs as Python runs it).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
