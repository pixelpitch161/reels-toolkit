//! Ability kind `octave`, ALTO SAX's OCTAVE KEY: its roster settings. `fight.py` `trigger` 2958 to 2960.
//!
//! For `dur` seconds (or `dur_beats` beats) every sweep brings a second light, square to the first through the same
//! point, sweeping there and back at twice the speed (`spot_tick` 4453 to 4459). It holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `octave`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["dur", "dur_beats"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("octave+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "dur", "dur_beats"];

/// What an ability of kind `octave` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OctaveCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<OctaveCfg, RosterError> {
    Ok(OctaveCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`.
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
