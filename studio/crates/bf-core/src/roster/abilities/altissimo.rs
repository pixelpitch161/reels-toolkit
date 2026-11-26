//! Ability kind `altissimo`, ALTO SAX's ALTISSIMO: its roster settings. `fight.py` `trigger` 2961 to 2963.
//!
//! For `dur` seconds (or `dur_beats` beats) a new sweep's light is `narrow` x as wide, burns `hot` x as much and its
//! line is `span_k` x as long (`spot_tick` 4443 to 4446, which reads all three as `ab[name]`). It holds only
//! numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `altissimo`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["dur", "dur_beats"], &["span_k"], &["narrow"], &["hot"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("altissimo+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "dur",
    "dur_beats",
    "span_k",
    "narrow",
    "hot",
];

/// What an ability of kind `altissimo` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AltissimoCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<AltissimoCfg, RosterError> {
    Ok(AltissimoCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`.
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
